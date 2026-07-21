//! The sandbox reset authority and its room-replay consumer.
//!
//! Moved out of `ambition_app` (2026-07-21, tracks §2.5). It lived there
//! because `reset_sandbox` sat in `app::world_flow::room_flow`, a module that
//! also composes `load_room` and therefore imports `ambition_render` — a
//! dependency this crate must never take. Nothing about the RESET half needed
//! render: it names only `ambition_engine_core`, `ambition_actors`,
//! `ambition_characters`, `ambition_sfx`, and `ambition_vfx`, all of which
//! `ambition_runtime` already depends on. Splitting the reset out of that
//! module is the whole reason it could not move earlier.
//!
//! **Why it had to move.** [`ambition_actors::session::reset::RoomReplayRequested`]
//! is the engine's generic "replay the active room" request, and content emits
//! it from three places today (Mary-O's flag completion and timeout, Sanic's
//! act clear, Ambition's cut-rope "try again"). Its only consumer used to be
//! registered by `ambition_app`, and neither demo app depends on
//! `ambition_app` — that IS the demo gate. So in the shipped standalone
//! Mary-O and Sanic binaries the message was written into a registered channel
//! that nothing drained: the player was not returned to spawn, the room was not
//! rebuilt, and pickups and enemies did not come back. Carrying the consumer in
//! [`crate::PlatformerEnginePlugins`] gives all three hosts one consumer.

use bevy::prelude::*;

use ambition_actors::time::feel::SandboxFeelTuning;
use ambition_actors::time::time_control::{ClockRequester, ClockResetRequest};
use ambition_actors::SandboxSimState;
use ambition_combat::{ResetRoomFeaturesEvent, RoomResetReason};
use ambition_engine_core as ae;
use ambition_engine_core::RoomGeometry;
use ambition_platformer_primitives::schedule::SandboxSet;
use ambition_platformer_primitives::schedule::SimScheduleExt;
use ambition_sfx::{SfxMessage, SfxWriter};
use ambition_vfx::VfxMessage;

/// Return a body to the room's spawn and clear its per-attempt state.
///
/// The one reset authority every host shares: the input-driven reset (a player
/// pressing Reset), the home reset policy (drown / hazard / out-of-bounds), and
/// the content-driven room replay all land here, so all three agree on what
/// "back to spawn" means. Callers own the POLICY of when to reset; this owns
/// what a reset IS.
///
/// Moves the body, refills movement resources and mana, re-anchors the respawn
/// safety point, snaps the sim clock back to 1.0, and clears the melee swing,
/// anim, combat, gesture, and blink-camera state. Emits the reset SFX/VFX pair
/// from the before/after positions.
#[allow(clippy::too_many_arguments)]
pub fn reset_sandbox(
    world: &ae::World,
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    motion_model: &mut ae::MotionModel,
    clusters: &mut ae::BodyClustersMut<'_>,
    sim_state: &mut SandboxSimState,
    clock_resets: &mut MessageWriter<ClockResetRequest>,
    safety: &mut ambition_actors::avatar::PlayerSafetyState,
    attack: &mut Option<ambition_actors::MeleeSwing>,
    anim: &mut ambition_actors::actor::BodyAnimFacts,
    combat: &mut ambition_characters::actor::BodyCombat,
    interaction: &mut ambition_actors::control::SlotGestures,
    blink_cam: &mut ambition_actors::avatar::PlayerBlinkCameraState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    let reset_from = clusters.kinematics.pos;
    ae::reset_body_clusters(motion_model, clusters, world.spawn);
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning.air_jumps,
    );
    clusters.mana.meter.refill_full();
    safety.last_safe_pos = world.spawn;
    clock_resets.write(ClockResetRequest::sim_clock(
        ClockRequester::Engine,
        "sandbox_reset",
    ));
    sim_state.room_transition_cooldown = 0.0;
    *attack = None;
    anim.reset();
    combat.reset();
    combat.hit_flash = feel.reset_flash_time;
    interaction.reset();
    blink_cam.reset();
    let reset_to = clusters.kinematics.pos;
    sfx.write(SfxMessage::Reset { pos: reset_to });
    vfx.write(VfxMessage::ResetEffects {
        from: reset_from,
        to: reset_to,
    });
}

/// Replay the ACTIVE room on a content-emitted
/// [`RoomReplayRequested`](ambition_actors::session::reset::RoomReplayRequested)
/// — a level restart, a death, a "try again" dialogue beat.
///
/// Engine-generic: this returns the primary body to spawn and requests the room
/// feature reset. Any CONTENT-named per-attempt state (a boss's persisted
/// "cleared" record, its music) is reset by content systems in
/// [`ContentRoomReplayResetSet`](ambition_actors::session::reset::ContentRoomReplayResetSet),
/// which [`RoomReplaySchedulePlugin`] anchors before this consumer — so this
/// system names no content.
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
    mut replay_requests: MessageReader<ambition_actors::session::reset::RoomReplayRequested>,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<RoomGeometry>,
    active_tuning: Res<ae::ActiveMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock_resets: MessageWriter<ClockResetRequest>,
    mut reset_room_features: MessageWriter<ResetRoomFeaturesEvent>,
    mut sfx_writer: SfxWriter,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_actors::features::MotionModel,
            &mut ambition_actors::actor::BodyAnimFacts,
            &mut ambition_characters::actor::BodyCombat,
            &mut ambition_actors::avatar::PlayerBlinkCameraState,
            &mut ambition_actors::actor::BodyMelee,
            &mut ambition_actors::avatar::PlayerSafetyState,
        ),
        ambition_actors::actor::PrimaryPlayerOnly,
    >,
    mut slot_gestures: ResMut<ambition_actors::control::SlotInteractionState>,
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
/// [`SandboxSet::PlayerInput`], after the dev-edit sync and before the input
/// timer. A host with its own reset-input system pins itself relative to this
/// one (the Ambition app does).
pub struct RoomReplaySchedulePlugin;

impl Plugin for RoomReplaySchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            apply_room_replay_request_system
                .in_set(SandboxSet::PlayerInput)
                .after(ambition_dev_tools::DevEditApplySet)
                .before(ambition_actors::control::input_timer_system),
        );
        // Content dialogue-followup emitters (e.g. cut-rope "try again") run
        // before the consumer that drains their requests the same frame;
        // content's replay-reset systems run before it too, so a named boss's
        // per-attempt state is cleared the same frame the room replays.
        //
        // `ContentDialogueFollowupSet` gets its PHASE home from
        // `PlayerSchedulePlugin`; this adds the consumer-relative edge. Now
        // that the consumer is engine-side, the engine owns both edges — the
        // host used to supply this one because the consumer was the host's.
        app.configure_sets(
            sim,
            (
                ambition_actors::session::reset::ContentDialogueFollowupSet,
                ambition_actors::session::reset::ContentRoomReplayResetSet,
            )
                .before(apply_room_replay_request_system),
        );
    }
}
