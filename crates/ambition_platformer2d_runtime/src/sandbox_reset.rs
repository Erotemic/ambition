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
use ambition_combat::{ResetRoomFeaturesEvent, RoomResetReason};
use ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::RoomGeometry;
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
/// The one reset authority owes the whole answer; `health` is `Option` only because a scratch
/// body without a meter is a valid thing to reset.
#[allow(clippy::too_many_arguments)]
pub fn reset_sandbox(
    world: &ae::World,
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    motion_model: &mut ae::MotionModel,
    clusters: &mut ae::BodyClustersMut<'_>,
    sim_state: &mut RoomTransitionCooldown,
    clock_resets: &mut MessageWriter<ClockResetRequest>,
    safety: &mut ambition_platformer2d_shared_tangle::safe_position::PlayerSafetyState,
    attack: &mut Option<ambition_platformer2d_actor_monolith::MeleeSwing>,
    anim: &mut ambition_platformer2d_actor_monolith::actor::BodyAnimFacts,
    combat: &mut ambition_characters::actor::BodyCombat,
    health: Option<&mut ambition_characters::actor::BodyHealth>,
    interaction: &mut ambition_characters::control::SlotGestures,
    blink_cam: &mut ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState,
    tuning: ae::MovementTuning,
    feel: Platformer2dFeelTuningMonolith,
) {
    let reset_from = clusters.kinematics.pos;
    ae::reset_body_clusters(motion_model, clusters, world.spawn, tuning.air_jumps);
    clusters.mana.meter.refill_full();
    safety.last_safe_pos = world.spawn;
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
    blink_cam.reset_to_spawn(ambition_platformer2d_actor_monolith::ROOM_DOOR_CAMERA_SNAP_TIME);
    let reset_to = clusters.kinematics.pos;
    sfx.write(SfxMessage::Reset { pos: reset_to });
    vfx.write(VfxMessage::ResetEffects {
        from: reset_from,
        to: reset_to,
    });
}

/// The set [`apply_room_replay_request_system`] runs in.
///
/// A room replay rebuilds the room; a reset input that must be seen by that
/// rebuild lands before it. ONE member — applying the request IS the step.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RoomReplayApplied;

/// Replay the ACTIVE room on a content-emitted
/// [`RoomReplayRequested`](ambition_platformer2d_actor_monolith::session::reset::RoomReplayRequested)
/// — a level restart, a death, a "try again" dialogue beat.
///
/// Engine-generic: this returns the primary body to spawn and requests the room feature reset.
///
/// This intentionally mirrors the host's reset-input system instead of driving
/// `ControlFrame::reset_pressed`: the request can arrive while gameplay input
/// is suspended by dialogue, so relying on the input frame would make the reset
/// timing depend on UI/game-mode scheduling.
///
/// The room-feature reset is requested even when no primary body matches the
/// query, so a replay still rebuilds the room in a host that has no home avatar
/// at that instant.
#[allow(clippy::too_many_arguments)]
pub fn apply_room_replay_request_system(
    mut replay_requests: MessageReader<
        ambition_platformer2d_actor_monolith::session::reset::RoomReplayRequested,
    >,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomGeometry>,
    active_tuning: Res<ae::ActiveMovementTuning>,
    feel_tuning: Res<Platformer2dFeelTuningMonolith>,
    mut sim_state: ResMut<RoomTransitionCooldown>,
    mut clock_resets: MessageWriter<ClockResetRequest>,
    mut reset_room_features: MessageWriter<ResetRoomFeaturesEvent>,
    mut sfx_writer: SfxWriter,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d_core::movement::MotionModel,
            &mut ambition_platformer2d_actor_monolith::actor::BodyAnimFacts,
            &mut ambition_characters::actor::BodyCombat,
            &mut ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState,
            &mut ambition_platformer2d_actor_monolith::actor::BodyMelee,
            &mut ambition_platformer2d_shared_tangle::safe_position::PlayerSafetyState,
            // A body put back at spawn comes back ALIVE (ADR 0033). `Option`
            // because a scratch body without a meter is a valid thing to reset.
            Option<&mut ambition_characters::actor::BodyHealth>,
        ),
        ambition_platformer2d_actor_monolith::actor::PrimaryPlayerOnly,
    >,
    mut slot_gestures: ResMut<ambition_characters::control::SlotInteractionState>,
) {
    if replay_requests.read().count() == 0 {
        return;
    }

    let Ok((
        mut cluster_item,
        mut motion_model,
        mut anim,
        mut combat,
        mut blink_cam,
        mut attack,
        mut safety,
        health,
    )) = player_q.single_mut()
    else {
        reset_room_features.write(ResetRoomFeaturesEvent {
            reason: RoomResetReason::Manual,
        });
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
        &mut safety,
        &mut attack.swing,
        &mut anim,
        &mut combat,
        health.map(|h| h.into_inner()),
        slot_gestures.primary_mut(),
        &mut blink_cam,
        active_tuning.0,
        *feel_tuning,
    );
    reset_room_features.write(ResetRoomFeaturesEvent {
        reason: RoomResetReason::Manual,
    });
}

/// Registers the one [`apply_room_replay_request_system`] consumer and anchors
/// the two content slots that must run before it. Part of
/// [`crate::PlatformerEnginePlugins`], so every host — the Ambition app, the
/// standalone demo binaries, and the shell-hosted demos — drains the replay
/// request through the same system.
///
/// The consumer holds the position the app's copy held: in
/// [`Platformer2dSimulationPhaseMonolith::PlayerInput`], after the dev-edit sync and before the input
/// timer. A host with its own reset-input system pins itself relative to this
/// one (the Ambition app does).
pub struct RoomReplaySchedulePlugin;

impl Plugin for RoomReplaySchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            apply_room_replay_request_system
                .in_set(RoomReplayApplied)
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput)
                .after(ambition_dev_tools::DevEditApplySet)
                // EXACTLY equivalent to the `.before(InputTimersAdvanced)` this
                // replaces, not merely stricter: that system is the FIRST element
                // of the tuple that gets `.chain().in_set(PlayerInputSet::Device)`,
                // so being before it is being before all of Device.
                .before(ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::Device),
        );
        // The replay transaction, in the order its meaning requires:
        //
        //   emit the request  →  clear the per-attempt state it invalidates
        //                     →  rebuild the room
        //
        // So `ContentRoomReplayResetSet` could run FIRST, read no request (the dialogue followup
        // had not written it yet), and do nothing; the followup then emitted; and the generic
        // consumer, correctly after both, rebuilt the room from state that was still persisted as
        // cleared. The reset sees the message on a later frame, when the reconstruction it was
        // supposed to precede has already happened.
        //
        //  the Smirking Behemoth's "press reset and start again" rebuilt the
        // room with the boss still recorded defeated. `.chain()` is the whole
        // fix; the sets were always the right vocabulary.
        //
        // `ContentDialogueFollowupSet` gets its PHASE home from
        // `PlayerSchedulePlugin`; this owns the semantic order between the three.
        app.configure_sets(
            sim,
            (
                ambition_platformer2d_actor_monolith::session::reset::ContentDialogueFollowupSet,
                ambition_platformer2d_actor_monolith::session::reset::ContentRoomReplayResetSet,
                RoomReplayApplied,
            )
                .chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

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
        let mut sim_state = ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown::default();
        let mut safety = ambition_platformer2d_shared_tangle::safe_position::PlayerSafetyState::default();
        let mut attack: Option<ambition_platformer2d_actor_monolith::MeleeSwing> = None;
        let mut anim = ambition_platformer2d_actor_monolith::actor::BodyAnimFacts::default();
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
                &mut safety,
                &mut attack,
                &mut anim,
                &mut combat,
                None,
                &mut gestures,
                &mut blink_cam,
                ae::DEFAULT_TUNING,
                ambition_combat::feel::Platformer2dFeelTuningMonolith::default(),
            );
        }
        probe.snapped = blink_cam.camera_snap_timer;
        probe.blink_cam = blink_cam;
    }
}
