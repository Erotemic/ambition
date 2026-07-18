//! Home body PRESENTATION phase helper.
//!
//! Movement integration and the ledge-platform carry moved DOWN into
//! `ambition::actors::avatar::body_integration` (called by the unified
//! `integrate_sim_bodies` phase). What remains here is the presentation HOOK the
//! app-side `sync_player_presentation` system calls: it reads the
//! [`PlayerBodyFrameOutput`] hand-off and emits screen-facing feedback.

use bevy::prelude::*;

use ambition::actors::avatar::PlayerBodyFrameOutput;
use ambition::actors::features::handle_player_events;
use ambition::engine_core as ae;
use ambition::sfx::{SfxMessage, SfxWriter};
use ambition::vfx::VfxMessage;

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
    combat: &mut ambition::characters::actor::BodyCombat,
    blink_cam: &mut ambition::actors::avatar::PlayerBlinkCameraState,
    anim: &mut ambition::actors::actor::BodyAnimFacts,
    sfx_writer: &mut SfxWriter,
    vfx_writer: &mut MessageWriter<VfxMessage>,
    shake: &mut ambition::platformer::camera_ease::CameraShakeState,
    is_primary: bool,
) {
    if frame_out.reset {
        return;
    }
    // Hard-fall screen shake consumes the kernel's semantic landing edge.
    // Initialization at a grounded authored pose is not a landing, while an
    // airborne body that touches down during its first tick still carries a
    // real impact speed.
    let shake_amplitude = ambition::platformer::camera_ease::hard_fall_shake_amplitude(
        frame_out.events.ground_contact.landing_impact_speed(),
    );
    if is_primary && shake_amplitude > 0.0 {
        shake.kick(shake_amplitude);
        sfx_writer.write(SfxMessage::Play {
            id: ambition::sfx::ids::PLAYER_LAND,
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
    );
}
