//! Home/player body HOME-POLICY + PRESENTATION phases.
//!
//! Movement integration for the home body is NO LONGER here. It moved DOWN into
//! the unified `ambition_platformer2d::actors::features::integrate_sim_bodies` phase
//! (`WorldPrep`), which integrates every non-boss sim body — home and actor — in
//! ONE scheduled system through the same engine entry. There is no `player_body_tick`
//! gameplay-movement route anymore. What remains here are the two HOME-specific
//! phases that read the [`ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput`]
//! hand-off the movement phase writes:
//!
//! - [`apply_home_reset_policy`] — HOME RESET POLICY. On a flagged body reset
//!   (drown / hazard / out-of-bounds / death) the primary home body runs the full
//!   sandbox reset (`reset_sandbox`) + a room-feature reset. This is genuine
//!   home policy: an actor owns its own hazard reaction and never teleports to the
//!   player spawn. Moves no body — the movement phase already teleported it.
//! - [`sync_player_presentation`] — HOME PRESENTATION. Emits screen shake / landing
//!   SFX / per-op anim/SFX/VFX from the hand-off. Moves no body, resolves no physics.

use bevy::prelude::*;

use ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput;
use ambition_platformer2d::actors::time::feel::Platformer2dFeelTuningMonolith;
use ambition_platformer2d::combat::{ResetRoomFeaturesEvent, RoomResetReason};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::RoomGeometry;

use super::feedback::GameplayFeedbackWriters;
use super::phases::sync_player_presentation as sync_player_presentation_phase;
use ambition_platformer2d::runtime::reset_sandbox;

use ambition_platformer2d::runtime::room_transition::RoomClock;

/// PHASE — home reset policy. The one thing the actor path does NOT do (an actor
/// owns its own hazard reaction; it never teleports to the player spawn). Reads the
/// [`PlayerBodyFrameOutput`] the movement phase wrote and, on a flagged reset for
/// the PRIMARY home body, runs the full sandbox reset (`reset_sandbox`) and requests
/// a room-feature reset. The body itself was already teleported to spawn by the
/// movement phase; this owns the SANDBOX/ROOM reset, which is home policy.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_home_reset_policy(
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    active_tuning: Res<ae::ActiveMovementTuning>,
    feel_tuning: Res<Platformer2dFeelTuningMonolith>,
    mut event_writers: GameplayFeedbackWriters,
    mut room_clock: RoomClock,
    mut reset_room_features: MessageWriter<ResetRoomFeaturesEvent>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actors::features::MotionModel,
            &mut ambition_platformer2d::actors::actor::BodyAnimFacts,
            &mut ambition_platformer2d::characters::actor::BodyCombat,
            &mut ambition_platformer2d::platformer::camera_ease::PlayerBlinkCameraState,
            &mut ambition_platformer2d::actors::actor::BodyMelee,
            &mut ambition_platformer2d::actors::avatar::PlayerSafetyState,
            &PlayerBodyFrameOutput,
        ),
        (
            With<ambition_platformer2d::actors::actor::PlayerEntity>,
            With<ambition_platformer2d::actors::actor::PrimaryPlayer>,
        ),
    >,
    mut slot_gestures: ResMut<ambition_platformer2d::actors::control::SlotInteractionState>,
) {
    let Ok((
        mut cluster_item,
        mut motion_model,
        mut anim,
        mut combat,
        mut blink_cam,
        mut attack,
        mut safety,
        frame_out,
    )) = player_q.single_mut()
    else {
        return;
    };
    if frame_out.reset.is_none() {
        return;
    }
    let mut clusters = cluster_item.as_clusters_mut();
    let tuning = active_tuning.0;
    reset_sandbox(
        &world.0,
        &mut event_writers.sfx,
        &mut event_writers.vfx,
        &mut motion_model,
        &mut clusters,
        &mut room_clock.sim_state,
        &mut room_clock.clock_resets,
        &mut safety,
        &mut attack.swing,
        &mut anim,
        &mut combat,
        slot_gestures.primary_mut(),
        &mut blink_cam,
        tuning,
        *feel_tuning,
    );
    reset_room_features.write(ResetRoomFeaturesEvent {
        reason: RoomResetReason::PlayerDeath,
    });
}

/// PHASE — sync player presentation. The HOME PRESENTATION half of the body tick.
/// Reads the [`PlayerBodyFrameOutput`] the movement phase (now
/// `integrate_sim_bodies`) wrote and emits the screen-facing feedback: the hard-fall
/// screen shake + landing SFX (primary only) and the per-op anim/SFX/VFX. Moves no
/// body, resolves no physics. A frame the movement phase flagged a reset is skipped
/// (the reset-policy phase already reset the presentation state).
pub fn sync_player_presentation(
    mut event_writers: GameplayFeedbackWriters,
    mut shake: ResMut<ambition_platformer2d::platformer::camera_ease::CameraShakeState>,
    // The active route's shake ceiling (D14), published from its presentation
    // profile. Read once per system rather than per body: it is a fact about the
    // experience, not about a fighter.
    shake_tuning: Res<ambition_platformer2d::platformer::camera_ease::CameraShakeTuning>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actors::actor::BodyAnimFacts,
            &mut ambition_platformer2d::characters::actor::BodyCombat,
            &mut ambition_platformer2d::platformer::camera_ease::PlayerBlinkCameraState,
            &PlayerBodyFrameOutput,
            Option<&ambition_platformer2d::actors::actor::PrimaryPlayer>,
            // A13: whose cues this player body emits.
            Option<&ambition_platformer2d::sfx::BodyPresentationSource>,
        ),
        With<ambition_platformer2d::actors::actor::PlayerEntity>,
    >,
) {
    for (mut cluster_item, mut anim, mut combat, mut blink_cam, frame_out, primary, source) in
        &mut player_q
    {
        let is_primary = primary.is_some();
        let clusters = cluster_item.as_clusters_mut();
        sync_player_presentation_phase(
            frame_out,
            &clusters,
            &mut combat,
            &mut blink_cam,
            &mut anim,
            &mut event_writers.sfx,
            &mut event_writers.vfx,
            &mut shake,
            *shake_tuning,
            is_primary,
            source.map(ambition_platformer2d::sfx::BodyPresentationSource::id),
        );
    }
}
