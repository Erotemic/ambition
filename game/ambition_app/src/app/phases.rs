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
    anim: &mut ambition_platformer2d::actors::actor::BodyAnimFacts,
    sfx_writer: &mut SfxWriter,
    vfx_writer: &mut MessageWriter<VfxMessage>,
    shake: &mut ambition_platformer2d::platformer::camera_ease::CameraShakeState,
    // The active route's shake ceiling (D14): a landing thump is one of the two
    // things in the game that shakes the screen, and how hard it is allowed to
    // is now the ROUTE's statement rather than a constant every game shares.
    shake_tuning: ambition_platformer2d::platformer::camera_ease::CameraShakeTuning,
    // **The route's reference hitlag**, against which this frame's freeze is a
    // severity. `None` = no route feel installed = no hit shake; see
    // `camera_ease::hit_shake_amplitude`.
    reference_hitlag_s: Option<f32>,
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
    // **A LANDED HIT SHAKES THE SCREEN, SCALED BY HOW HARD IT LANDED** (P4.37).
    //
    // ⛔ nothing did this. `kick` had two production call sites in the whole
    // workspace — a boss phase change and the hard-fall landing directly above —
    // so a smash that sent a fighter to the blast zone and a jab moved this
    // camera identically, and hitlag alone carried the whole difference.
    //
    // ⭐ **the hitstop IS the severity, already resolved.** `hitlag_duration`
    // wrote it from the knockback the hit actually produced, so reading it back
    // keeps the camera body-generic: no move ids, no per-character table, and a
    // character that authors a heavier launch gets a heavier camera for free.
    //
    // ⚠ **kicked every frame the freeze is live, deliberately, and it is not a
    // repeat.** `kick` is strongest-wins, so re-asserting the same amplitude
    // HOLDS the shake for exactly the freeze and releases into the decay after —
    // which is the beat a platform fighter wants, and it needs no edge-detection
    // state. A `Local` remembering last frame's timer would be cross-frame state
    // in a rollback schedule, for a effect that is already idempotent.
    if is_primary {
        if let Some(reference) = reference_hitlag_s {
            let hit_shake = ambition_platformer2d::platformer::camera_ease::hit_shake_amplitude(
                combat.hitstop_timer,
                reference,
            );
            if hit_shake > 0.0 {
                shake.kick(hit_shake, shake_tuning);
            }
        }
    }
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
