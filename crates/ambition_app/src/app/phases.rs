//! Home body PRESENTATION phase helper.
//!
//! Movement integration and the ledge-platform carry moved DOWN into
//! `ambition_gameplay_core::player::body_integration` (called by the unified
//! `integrate_sim_bodies` phase). What remains here is the presentation HOOK the
//! app-side `sync_player_presentation` system calls: it reads the
//! [`PlayerBodyFrameOutput`] hand-off and emits screen-facing feedback.

use bevy::prelude::*;

use ambition_engine_core as ae;
use ambition_sfx::SfxMessage;
use ambition_gameplay_core::player::{handle_player_events, PlayerBodyFrameOutput};
use ambition_render::fx::VfxMessage;

/// PHASE — sync player presentation. Reads the [`PlayerBodyFrameOutput`] the
/// movement phase wrote and emits the screen-facing feedback: the hard-fall screen
/// shake + landing SFX (primary only) and the per-op anim/SFX/VFX in
/// `handle_player_events`. It moves no body and resolves no physics. A frame the
/// movement phase flagged a reset (`reset`) already had its presentation state reset
/// by the home reset-policy phase, so it is skipped.
#[allow(clippy::too_many_arguments)]
pub(super) fn sync_player_presentation(
    frame_out: &PlayerBodyFrameOutput,
    clusters: &ae::BodyClustersMut<'_>,
    combat: &mut ambition_characters::actor::BodyCombat,
    blink_cam: &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
    anim: &mut ambition_gameplay_core::player::PlayerAnimState,
    sfx_writer: &mut MessageWriter<SfxMessage>,
    vfx_writer: &mut MessageWriter<VfxMessage>,
    shake: &mut ambition_gameplay_core::time::camera_ease::CameraShakeState,
    is_primary: bool,
) {
    if frame_out.reset {
        return;
    }
    let was_grounded = frame_out.was_grounded;
    // Hard-fall screen shake: pure trigger in `time::camera_ease`. Saturates above
    // terminal velocity via `kick()`'s cap. `pre_sim_fall_speed` is the
    // along-gravity fall speed that entered the movement tick.
    let shake_amplitude = ambition_gameplay_core::time::camera_ease::hard_fall_shake_amplitude(
        was_grounded,
        clusters.ground.on_ground,
        frame_out.pre_sim_fall_speed,
    );
    if is_primary && shake_amplitude > 0.0 {
        shake.kick(shake_amplitude);
        sfx_writer.write(SfxMessage::Play {
            id: ambition_sfx::ids::PLAYER_LAND,
            pos: clusters.kinematics.pos,
        });
    }
    handle_player_events(
        sfx_writer,
        vfx_writer,
        clusters,
        combat,
        blink_cam,
        anim,
        frame_out.events.clone(),
        Some(was_grounded),
    );
}
