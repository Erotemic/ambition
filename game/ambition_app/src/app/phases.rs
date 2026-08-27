//! Home body PRESENTATION phase helper.
//!
//! Movement integration and the ledge-platform carry moved DOWN into
//! `ambition_platformer2d::actors::avatar::body_integration` (called by the unified
//! `integrate_sim_bodies` phase). What remains here is the presentation HOOK the
//! app-side `sync_player_presentation` system calls: it reads the
//! [`PlayerBodyFrameOutput`] hand-off and emits screen-facing feedback.

use bevy::prelude::*;

use ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput;
use ambition_platformer2d::actors::features::handle_player_events;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::sfx::{SfxMessage, SfxWriter};
use ambition_platformer2d::vfx::VfxMessage;

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
    combat: &mut ambition_platformer2d::characters::actor::BodyCombat,
    blink_cam: &mut ambition_platformer2d::platformer::camera_ease::PlayerBlinkCameraState,
    anim: &mut ambition_platformer2d::characters::actor::BodyAnimFacts,
    sfx_writer: &mut SfxWriter,
    vfx_writer: &mut MessageWriter<VfxMessage>,
    shake: &mut ambition_platformer2d::platformer::camera_ease::CameraShakeState,
    // The active route's shake ceiling: a landing thump is one of the two things in the game
    // that shakes the screen, and how hard it is allowed to is now the ROUTE's statement rather
    // than a constant every game shares.
    shake_tuning: ambition_platformer2d::platformer::camera_ease::CameraShakeTuning,
    is_primary: bool,
    // A13: the player body's presentation source, so its jump/dash/land cues
    // resolve in ITS character's bank rather than the session provider's.
    source: Option<&ambition_platformer2d::sfx::PresentationSourceId>,
) {
    if frame_out.reset.is_some() {
        return;
    }
    // Hard-fall screen shake consumes the kernel's semantic landing edge.
    // Initialization at a grounded authored pose is not a landing, while an
    // airborne body that touches down during its first tick still carries a
    // real impact speed.
    let shake_amplitude = ambition_platformer2d::platformer::camera_ease::hard_fall_shake_amplitude(
        frame_out.events.ground_contact.landing_impact_speed(),
    );
    // THE HIT SHAKE IS NOT HERE, AND MUST NOT COME BACK HERE (P4.37). It lives in the ENGINE
    // now, reading every body: `features::ecs::hit_camera_shake`, scheduled in `CombatSet::Settle`.
    //
    // The hard-fall thump below stays: a LANDING genuinely is home presentation
    // (it is read off this body's own `PlayerBodyFrameOutput`), and its SFX is
    // emitted for the body the camera is following.
    if is_primary && shake_amplitude > 0.0 {
        shake.kick(shake_amplitude, shake_tuning);
        sfx_writer.write_for_body(
            source,
            SfxMessage::Play {
                id: ambition_platformer2d::sfx::ids::PLAYER_LAND,
                pos: clusters.kinematics.pos,
            },
        );
    }
    handle_player_events(
        sfx_writer,
        vfx_writer,
        clusters,
        combat,
        blink_cam,
        anim,
        frame_out.events.clone(),
        source,
    );
}
