//! Home/player body HOME-POLICY + PRESENTATION phases.
//!
//! Movement integration for the home body is NO LONGER here. It moved DOWN into
//! the unified `ambition::actors::features::integrate_sim_bodies` phase
//! (`WorldPrep`), which integrates every non-boss sim body — home and actor — in
//! ONE scheduled system through the same engine entry. There is no `player_body_tick`
//! gameplay-movement route anymore. What remains here are the two HOME-specific
//! phases that read the [`ambition::actors::player::PlayerBodyFrameOutput`]
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

use ambition::actors::player::PlayerBodyFrameOutput;
use ambition::actors::time::feel::SandboxFeelTuning;
use ambition::combat::{ResetRoomFeaturesEvent, RoomResetReason};
use ambition::dev_tools::dev_tools::EditableMovementTuning;
use ambition::engine_core as ae;
use ambition::engine_core::RoomGeometry;

use super::feedback::SandboxEventWriters;
use super::phases::sync_player_presentation as sync_player_presentation_phase;
use super::world_flow::{reset_sandbox, RoomClock};

/// PHASE — home reset policy. The one thing the actor path does NOT do (an actor
/// owns its own hazard reaction; it never teleports to the player spawn). Reads the
/// [`PlayerBodyFrameOutput`] the movement phase wrote and, on a flagged reset for
/// the PRIMARY home body, runs the full sandbox reset (`reset_sandbox`) and requests
/// a room-feature reset. The body itself was already teleported to spawn by the
/// movement phase; this owns the SANDBOX/ROOM reset, which is home policy.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_home_reset_policy(
    world: Res<RoomGeometry>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    gravity_field: Option<Res<ambition::actors::physics::GravityField>>,
    mut event_writers: SandboxEventWriters,
    mut room_clock: RoomClock,
    mut reset_room_features: MessageWriter<ResetRoomFeaturesEvent>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition::actors::player::BodyAnimFacts,
            &mut ambition::characters::actor::BodyCombat,
            &mut ambition::actors::player::PlayerBlinkCameraState,
            &mut ambition::actors::player::BodyMelee,
            &mut ambition::actors::player::PlayerSafetyState,
            &PlayerBodyFrameOutput,
        ),
        (
            With<ambition::actors::actor::PlayerEntity>,
            With<ambition::actors::actor::PrimaryPlayer>,
        ),
    >,
    mut slot_gestures: ResMut<ambition::actors::player::SlotInteractionState>,
) {
    let Ok((
        mut cluster_item,
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
    if !frame_out.reset {
        return;
    }
    let mut clusters = cluster_item.as_clusters_mut();
    let mut tuning = editable_tuning.as_engine();
    let gdir = ambition::actors::physics::gravity_dir_or_default(gravity_field.as_deref());
    ambition::actors::physics::apply_gravity_dir(&mut tuning, gdir);
    reset_sandbox(
        &world.0,
        &mut event_writers.sfx,
        &mut event_writers.vfx,
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
    mut event_writers: SandboxEventWriters,
    mut shake: ResMut<ambition::platformer::camera_ease::CameraShakeState>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition::actors::player::BodyAnimFacts,
            &mut ambition::characters::actor::BodyCombat,
            &mut ambition::actors::player::PlayerBlinkCameraState,
            &PlayerBodyFrameOutput,
            Option<&ambition::actors::actor::PrimaryPlayer>,
        ),
        With<ambition::actors::actor::PlayerEntity>,
    >,
) {
    for (mut cluster_item, mut anim, mut combat, mut blink_cam, frame_out, primary) in &mut player_q
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
            is_primary,
        );
    }
}
