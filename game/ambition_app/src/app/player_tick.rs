//! Home/player body HOME-POLICY + PRESENTATION phases.
//!
//! Movement integration for the home body is NO LONGER here. It moved DOWN into
//! the unified `ambition_platformer2d::actors::features::integrate_sim_bodies` phase
//! (`WorldPrep`), which integrates every non-boss sim body — home and actor — in
//! ONE scheduled system through the same engine entry. There is no `player_body_tick`
//! gameplay-movement route anymore. What remains here is the ONE HOME-specific
//! phase that reads the [`ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput`]
//! hand-off the movement phase writes:
//!
//! - [`sync_player_presentation`] — HOME PRESENTATION. Emits screen shake / landing
//!   SFX / per-op anim/SFX/VFX from the hand-off. Moves no body, resolves no physics.
//!
//! ⛔ **its former neighbour `apply_home_reset_policy` is DELETED** (ADR 0033).
//! It turned a flagged body reset into a full sandbox reset plus a room-feature
//! reset, on the frame it was flagged — so a death resolved itself before any
//! ruleset could speak, and a death beat that held the body outside the world
//! re-flagged it every frame of the dwell. A death is a FACT now; the game's
//! authored `DeathRules` owns the consequence.

use bevy::prelude::*;

use ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput;
use ambition_platformer2d::engine_core as ae;

use super::feedback::GameplayFeedbackWriters;
use super::phases::sync_player_presentation as sync_player_presentation_phase;

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
