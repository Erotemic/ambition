//! The sandbox reset authority and its room-replay consumer.
//!
//! It lived there because `reset_sandbox` sat in `app::world_flow::room_flow`, a module that also
//! composes `load_room` and therefore imports `ambition_render` — a dependency this crate must
//! never take. Nothing about the RESET half needed render: it names only
//! `ambition_platformer2d_core`, `ambition_platformer2d_actor_monolith`, `ambition_characters`,
//! `ambition_sfx`, and `ambition_vfx`, all of which `ambition_platformer2d_runtime` already depends
//! on. Splitting the reset out of that module is the whole reason it could not move earlier.
//!
//! Why it had to move.
//! [`ambition_platformer2d_actor_monolith::session::reset::RoomReplayRequested`] is the engine's
//! generic "replay the active room" request, and content emits it from three places today (Mary-O's
//! flag completion and timeout, Sanic's act clear, Ambition's cut-rope "try again"). So in the
//! shipped standalone Mary-O and Sanic binaries the message was written into a registered channel
//! that nothing drained: the player was not returned to spawn, the room was not rebuilt, and
//! pickups and enemies did not come back. Carrying the consumer in
//! [`crate::PlatformerEnginePlugins`] gives all three hosts one consumer.

use bevy::prelude::*;

use ambition_combat::feel::Platformer2dFeelTuningMonolith;
use ambition_combat::RoomReplayAdmitted;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown;
use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_sfx::{SfxMessage, SfxWriter};
use ambition_time::time_control::{ClockRequester, ClockResetRequest};
use ambition_vfx::VfxMessage;

/// Return a body to the room's spawn and clear its per-attempt state.
///
/// The one reset authority every host shares: the input-driven reset (a player
/// pressing Reset), the home reset policy (drown / hazard / out-of-bounds), and
/// the content-driven room replay all land here, so all three agree on what
/// "back to spawn" means. Callers own the POLICY of when to reset; this owns
/// what a reset IS.
///
/// Moves the body, refills movement resources, mana and health, re-anchors
/// the respawn safety point, snaps the sim clock back to 1.0, and clears the
/// melee swing, anim, combat, gesture, and blink-camera state. Emits the reset
/// SFX/VFX pair from the before/after positions.
///
/// ⭐ THE HOME-AVATAR HALVES ARE `Option`, for the same reason `health` is: a
/// reset is for whatever body is PLAYING the room, and a possessed enemy carries
/// no `PlayerSafetyState` and no `PlayerBlinkCameraState`. Requiring them made
/// the whole reset a no-op for a driven non-player body — measured: the replay
/// silently left a possessed actor at the last attempt's health, because the
/// query it was fetched through simply did not match.
#[allow(clippy::too_many_arguments)]
pub fn reset_sandbox(
    world: &ae::World,
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    motion_model: &mut ae::MotionModel,
    clusters: &mut ae::BodyClustersMut<'_>,
    sim_state: &mut RoomTransitionCooldown,
    clock_resets: &mut MessageWriter<ClockResetRequest>,
    safety: Option<&mut ambition_platformer2d_shared_tangle::safe_position::PlayerSafetyState>,
    attack: &mut Option<ambition_combat::components::MeleeSwing>,
    anim: &mut ambition_characters::actor::BodyAnimFacts,
    combat: &mut ambition_characters::actor::BodyCombat,
    health: Option<&mut ambition_characters::actor::BodyHealth>,
    interaction: &mut ambition_characters::control::SlotGestures,
    blink_cam: Option<
        &mut ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState,
    >,
    tuning: ae::MovementTuning,
    feel: Platformer2dFeelTuningMonolith,
) {
    let reset_from = clusters.kinematics.pos;
    ae::reset_body_clusters(motion_model, clusters, world.spawn, tuning.air_jumps);
    clusters.mana.meter.refill_full();
    if let Some(safety) = safety {
        safety.last_safe_pos = world.spawn;
    }
    clock_resets.write(ClockResetRequest::sim_clock(
        ClockRequester::Engine,
        "sandbox_reset",
    ));
    sim_state.remaining = 0.0;
    *attack = None;
    anim.reset();
    combat.reset();
    if let Some(health) = health {
        health.reset();
    }
    // The grace a body is owed for coming back somewhere it did not choose to
    // be — inherited from the deleted `death_respawn_player`, which is the only
    // thing that used to grant it.
    combat.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    combat.hit_flash = feel.reset_flash_time;
    interaction.reset();
    // ONE CALL: clears the blink AND keeps the snap. Writing the two separately
    // is what produced the defect — `reset()` clears the snap, so a placer that
    // forgot the second line eased the camera across the whole room.
    if let Some(blink_cam) = blink_cam {
        blink_cam.reset_to_spawn(ambition_platformer2d_actor_monolith::ROOM_DOOR_CAMERA_SNAP_TIME);
    }
    let reset_to = clusters.kinematics.pos;
    sfx.write(SfxMessage::Reset { pos: reset_to });
    vfx.write(VfxMessage::ResetEffects {
        from: reset_from,
        to: reset_to,
    });
}

/// ⭐ WHERE A REPLAY IS ADMITTED. One member: deciding whether the replay
/// happens IS the step, and everything that reacts to a replay is ordered
/// AFTER it.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RoomReplayAdmission;

/// Where the admitted replay's consequences run: the body back at spawn, the
/// attempt's residue retired, content's per-attempt state cleared.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RoomReplayConsequences;

/// Decide whether a requested replay HAPPENS, and say so once.
///
/// ```text
/// RoomReplayRequested          the ask — anybody may write it, it may be refused
///        |
///        v   resolve the controlled body, name the active room,
///            take the one pending-lifecycle slot
///        |
/// RoomReplayAdmitted           the fact — one writer, and only on acceptance
/// ```
///
/// ⛔⛔ NOTHING AUTHORITATIVE MAY CHANGE BEFORE THE SLOT IS TAKEN.
/// [`PendingLifecycleCommit::record`] is earliest-sticky: another lifecycle
/// operation can already own it, and then this replay does not happen at all.
/// The previous shape reset the avatar, wrote the reset message (which reset
/// gravity, cleared pending hits, advanced a boss arena's heavy-object cycle and
/// despawned the attempt's dropped loot), and only afterwards tried to record
/// the intent — so a refused replay left a half-reset room standing.
///
/// ⛔ AND IT NAMES THE CONTROLLED BODY, NOT THE HOME AVATAR.
/// `RoomReplayRequested`'s own contract says "the controlled player", and the
/// implementation queried `PrimaryPlayerOnly` — so while possessing an actor,
/// the replay reset the body the player was NOT driving and named it as the
/// subject of the rebuild, while the possessed body carried the previous
/// attempt's state through custody.
///
/// A composition with no controlled body still replays: the room is rebuilt with
/// nobody in it, `subject` is `None`, and the transition road is not asked for a
/// crossing it cannot describe.
pub fn admit_room_replay(
    mut requests: MessageReader<
        ambition_platformer2d_actor_monolith::session::reset::RoomReplayRequested,
    >,
    controlled: Option<Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>>,
    identities: Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
    >,
    mut pending: ResMut<
        ambition_platformer2d_actor_monolith::session::lifecycle_commit::PendingLifecycleCommit,
    >,
    boundary: Option<Res<ae::ConfirmedFrameBoundary>>,
    mut admitted: MessageWriter<RoomReplayAdmitted>,
) {
    use ambition_platformer2d_actor_monolith::session::lifecycle_commit::{
        LifecycleIntent, RoomTransitionIntent,
    };

    // Drained unconditionally: a request seen while no world exists must not be
    // re-read several frames later against a different one.
    let Some(reason) = requests.read().map(|request| request.reason).next() else {
        return;
    };
    let Some(room_set) = room_set.as_deref() else {
        return;
    };
    let active = room_set.active_spec();

    // The body the player is actually playing the room with.
    let subject = controlled
        .as_deref()
        .and_then(|controlled| controlled.0)
        .and_then(|entity| identities.get(entity).ok())
        .cloned();

    let admission = match subject.clone() {
        Some(subject) => pending.record(
            boundary.map_or(0, |boundary| boundary.current),
            LifecycleIntent::Transition(RoomTransitionIntent {
                subject,
                target_room: active.id.clone(),
                arrival: active.world.spawn,
                // A replay is not a walk off the side of a room.
                edge_exit: false,
                // Silent on purpose: nobody opened a door.
                zone_sfx: None,
            }),
        ),
        // ⚠ NO BODY, NO CROSSING TO DESCRIBE. The room is not rebuilt here —
        // `RoomTransitionIntent` requires a subject by contract — but the
        // consequences that are not about a body still run, and the log says
        // which half happened rather than leaving a silent partial.
        None => {
            bevy::log::info!(
                target: "ambition_platformer2d::room_reset",
                "room replay ({reason:?}) admitted with no controlled body: \
                 clearing the attempt, not rebuilding `{}`",
                active.id,
            );
            ambition_platformer2d_actor_monolith::session::lifecycle_commit::Admission::Admitted
        }
    };

    if !admission.admitted() {
        bevy::log::info!(
            target: "ambition_platformer2d::room_reset",
            "room replay ({reason:?}) REFUSED: another lifecycle operation \
             already owns the pending slot. Nothing was reset."
        );
        return;
    }
    ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
        "room-replay admitted reason={reason:?} room={}",
        active.id
    ));
    admitted.write(RoomReplayAdmitted { reason, subject });
}

/// Put the admitted replay's subject back at the room spawn.
///
/// Reads the SUBJECT the admission resolved rather than re-deriving it: control
/// can move, end, or the body can die between the two, and a reset aimed at a
/// different body than the rebuild carries is the fork this seam exists to
/// remove.
#[allow(clippy::too_many_arguments)]
pub fn return_the_replay_subject_to_spawn(
    mut admitted: MessageReader<RoomReplayAdmitted>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomGeometry>,
    active_tuning: Res<ae::ActiveMovementTuning>,
    feel_tuning: Res<Platformer2dFeelTuningMonolith>,
    mut sim_state: ResMut<RoomTransitionCooldown>,
    mut clock_resets: MessageWriter<ClockResetRequest>,
    mut sfx_writer: SfxWriter,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut bodies: Query<(
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d_core::movement::MotionModel,
        &mut ambition_characters::actor::BodyAnimFacts,
        &mut ambition_characters::actor::BodyCombat,
        // ⛔ `Option`: THE SUBJECT MAY NOT BE A PLAYER BODY. A possessed enemy is
        // the body playing the room and carries neither of these; requiring them
        // made the reset silently skip it entirely.
        Option<&mut ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState>,
        &mut ambition_combat::BodyMelee,
        Option<&mut ambition_platformer2d_shared_tangle::safe_position::PlayerSafetyState>,
        // A body put back at spawn comes back ALIVE (ADR 0033). `Option`
        // because a scratch body without a meter is a valid thing to reset.
        Option<&mut ambition_characters::actor::BodyHealth>,
    )>,
    mut slot_gestures: ResMut<ambition_characters::control::SlotInteractionState>,
) {
    let Some(subject) = admitted
        .read()
        .filter_map(|admitted| admitted.subject.clone())
        .next()
    else {
        return;
    };
    let Some((
        _,
        mut cluster_item,
        mut motion_model,
        mut anim,
        mut combat,
        blink_cam,
        mut attack,
        safety,
        health,
    )) = bodies.iter_mut().find(|(id, ..)| **id == subject)
    else {
        return;
    };

    let mut clusters = cluster_item.as_clusters_mut();
    reset_sandbox(
        &world.0,
        &mut sfx_writer,
        &mut vfx_writer,
        &mut motion_model,
        &mut clusters,
        &mut sim_state,
        &mut clock_resets,
        safety.map(|s| s.into_inner()),
        &mut attack.swing,
        &mut anim,
        &mut combat,
        health.map(|h| h.into_inner()),
        slot_gestures.primary_mut(),
        blink_cam.map(|c| c.into_inner()),
        active_tuning.0,
        *feel_tuning,
    );
}

/// Registers the replay TRANSACTION — admission, then consequences — and
/// anchors the two content slots around it. Part of
/// [`crate::PlatformerEnginePlugins`], so every host (the Ambition app, the
/// standalone demo binaries, the shell-hosted demos) admits a replay through
/// the same system.
pub struct RoomReplaySchedulePlugin;

impl Plugin for RoomReplaySchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            (
                admit_room_replay.in_set(RoomReplayAdmission),
                return_the_replay_subject_to_spawn.in_set(RoomReplayConsequences),
            )
                .chain()
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput)
                .after(ambition_dev_tools::DevEditApplySet)
                // EXACTLY equivalent to the `.before(InputTimersAdvanced)` this
                // replaces, not merely stricter: that system is the FIRST element
                // of the tuple that gets `.chain().in_set(PlayerInputSet::Device)`,
                // so being before it is being before all of Device.
                .before(ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::Device),
        );
        // ⭐ THE ORDER THE TRANSACTION'S MEANING REQUIRES, AND IT INVERTED.
        //
        //   emit the request  →  ADMIT it  →  clear what the admitted replay
        //                                     invalidates  →  rebuild the room
        //
        // Content's per-attempt reset used to run BEFORE the consumer, on the
        // REQUEST — which is how a boss arena advanced its heavy-object cycle
        // and a persisted "cleared" record got retracted for a replay that had
        // not been admitted and might never happen. It now runs after admission
        // and reads `RoomReplayAdmitted`.
        //
        // `ContentDialogueFollowupSet` gets its PHASE home from
        // `PlayerSchedulePlugin`; this owns the semantic order between the four.
        app.configure_sets(
            sim,
            (
                ambition_platformer2d_actor_monolith::session::reset::ContentDialogueFollowupSet,
                RoomReplayAdmission,
                ambition_platformer2d_actor_monolith::session::reset::ContentRoomReplayResetSet,
                RoomReplayConsequences,
            )
                .chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐⭐ THE WIRING, not the primitive.
    ///
    /// `camera_ease`'s own test proves `snap_after_placement` sets the timer;
    /// it stays GREEN with this call site deleted, which is a test that pins the
    /// FUNCTION and not the WIRING. This one drives the real `reset_sandbox`
    /// and asks what the CAMERA was left holding.
    ///
    /// ⛔ AND THE ASSERTION IS ORDERED AGAINST `blink_cam.reset()`. The defect
    /// was never a missing feature — the snap mechanism has existed since door
    /// transitions — it was that the reset cleared it on the one teleport that
    /// most needed it. A test that only checked "the timer is positive" without
    /// the reset in the path would pass against the bug.
    #[test]
    fn a_sandbox_reset_leaves_the_camera_asking_to_snap() {
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        app.add_message::<ambition_time::time_control::ClockResetRequest>();
        app.init_resource::<ambition_sfx::SfxEmissionContext>();

        // The camera state a body carries, with a blink running so the reset has
        // something real to clear.
        let mut blink_cam =
            ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState::default();
        blink_cam.blink_in_timer = 0.4;
        app.insert_resource(Probe {
            blink_cam,
            snapped: 0.0,
        });

        app.add_systems(Update, drive_one_reset);
        app.update();

        let probe = app.world().resource::<Probe>();
        assert_eq!(
            probe.blink_cam.blink_in_timer, 0.0,
            "the reset did not clear the blink, so it is not the reset this test \
             thinks it is driving"
        );
        assert!(
            probe.snapped > 0.0,
            "a body put back at spawn left the camera with no snap request, so it \
             EASES to a position that teleported — Jon measured 440px over about \
             forty ticks"
        );
    }

    #[derive(Resource)]
    struct Probe {
        blink_cam: ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState,
        snapped: f32,
    }

    /// One real `reset_sandbox` call, with the writers a world supplies.
    fn drive_one_reset(
        mut probe: ResMut<Probe>,
        mut sfx: ambition_sfx::SfxWriter,
        mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
        mut clock_resets: MessageWriter<ambition_time::time_control::ClockResetRequest>,
    ) {
        // A room with one floor and a spawn away from where the body starts, so
        // the reset is a real teleport rather than a no-op.
        let world = ae::World {
            name: "reset wiring".to_string(),
            size: ae::Vec2::new(1600.0, 900.0),
            spawn: ae::Vec2::new(120.0, 800.0),
            blocks: vec![ae::world::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 848.0),
                ae::Vec2::new(1600.0, 48.0),
            )],
            water_regions: Vec::new(),
            climbable_regions: Vec::new(),
            chains: Vec::new(),
            edges: Default::default(),
        };
        let mut scratch = ae::BodyClusterScratch::new_with_abilities(
            ae::Vec2::new(400.0, 400.0),
            ae::AbilitySet::sandbox_all(),
        );
        let mut model = ae::MotionModel::default();
        let mut sim_state =
            ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown::default();
        let mut safety =
            ambition_platformer2d_shared_tangle::safe_position::PlayerSafetyState::default();
        let mut attack: Option<ambition_combat::components::MeleeSwing> = None;
        let mut anim = ambition_characters::actor::BodyAnimFacts::default();
        let mut combat = ambition_characters::actor::BodyCombat::default();
        let mut gestures = ambition_characters::control::SlotGestures::default();
        let mut blink_cam = probe.blink_cam;
        {
            let mut clusters = scratch.as_mut();
            super::reset_sandbox(
                &world,
                &mut sfx,
                &mut vfx,
                &mut model,
                &mut clusters,
                &mut sim_state,
                &mut clock_resets,
                Some(&mut safety),
                &mut attack,
                &mut anim,
                &mut combat,
                None,
                &mut gestures,
                Some(&mut blink_cam),
                ae::DEFAULT_TUNING,
                ambition_combat::feel::Platformer2dFeelTuningMonolith::default(),
            );
        }
        probe.snapped = blink_cam.camera_snap_timer;
        probe.blink_cam = blink_cam;
    }
}
