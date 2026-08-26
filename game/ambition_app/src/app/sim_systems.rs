//! Host-bound simulation systems that CANNOT move down to a library plugin.
//!
//! The host schedule (`super::plugins::register_player_input_systems`) still owns their ordering +
//! `run_if` gates and references those moved `pub fn`s.
//!
//! What remains is the RESET-INPUT system: Ambition's own "the player pressed
//! Reset" path, which reads `ControlFrame::reset_pressed`. It stays app-side
//! because the button binding is Ambition's, not the engine's.
//!
//! Both called `reset_sandbox`, which was the stated reason they were stuck here — but
//! `reset_sandbox` was only app-local by accident of the module it sat in, and the replay
//! consumer is engine-generic (it names no content and no Ambition input). Leaving it here
//! meant the standalone demo binaries, which depend on `ambition_platformer2d` but never on
//! `ambition_app`, had no consumer at all.
//!
//! This system is a narrow query/resource system registered in the
//! [`Platformer2dSimulationPhaseMonolith::CoreSimulation`] chain configured by
//! [`super::schedule::configure_platformer2d_simulation_phases`]. Cross-set ordering lives in the
//! schedule; intra-set ordering is expressed by `.chain()` where registered.

use ambition_platformer2d::engine_core as ae;
use bevy::prelude::*;

use ambition_platformer2d::time::time_control::ClockResetRequest;
use ambition_platformer2d::platformer::safe_position::RoomTransitionCooldown;
use ambition_platformer2d::combat::feel::Platformer2dFeelTuningMonolith;
use ambition_platformer2d::combat::{ResetRoomFeaturesEvent, RoomResetReason};
use ambition_platformer2d::engine_core::RoomGeometry;
use ambition_platformer2d::sfx::SfxWriter;
use ambition_platformer2d::sim::{PlayerSlot, SeatRawFrames};
use ambition_platformer2d::vfx::VfxMessage;

/// Detect a player-pressed reset (the Reset button / `controls.reset_pressed`)
/// and execute the full sandbox reset before the rest of the gameplay
/// chain runs.
///
/// Handles input-driven resets before the rest of gameplay. Engine-driven resets
/// still finish in their player-control/simulation call sites because those paths
/// have already mutated the player and must complete cleanup immediately.
///
/// This system clears the primary seat's `reset_pressed` after handling it
/// so the engine path inside `update_player_control_with_clusters`
/// does not re-trigger a reset on the same frame. Writes sfx/vfx directly to
/// `MessageWriter`s via local Vec buffers (the engine helper
/// `reset_sandbox` still uses Vec push semantics).
///
/// Gated by `gameplay_allowed`: paused / dialogue modes don't process
/// reset input.
pub fn apply_player_reset_input_system(
    // This read the global `ControlFrame`, where "the primary seat" was never stated — it was
    // what that resource happened to mean. Reset belongs to the SESSION rather than to a seat,
    // so the seat named here is the one whose button the shell listens to, not a limit on who
    // may reset. read and cleared through `shape_seat_frame`, because WHICH table holds this
    // tick's press depends on the host: a rollback session publishes into the slot, a
    // frame-stepped composition assembles the raw row.
    mut raw: ResMut<SeatRawFrames>,
    mut slots: ResMut<ambition_platformer2d::sim::SlotControls>,
    latches: Option<Res<ambition_platformer2d::sim::SlotControlLatches>>,
    rollback: Option<Res<ambition_platformer2d::platformer::schedule::SimulationReplayState>>,
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
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
            &mut ambition_platformer2d::actor::MotionModel,
            &mut ambition_platformer2d::actors::actor::BodyAnimFacts,
            &mut ambition_platformer2d::characters::actor::BodyCombat,
            &mut ambition_platformer2d::platformer::camera_ease::PlayerBlinkCameraState,
            &mut ambition_platformer2d::actors::actor::BodyMelee,
            &mut ambition_platformer2d::platformer::safe_position::PlayerSafetyState,
            // A body put back at spawn comes back ALIVE (ADR 0033).
            Option<&mut ambition_platformer2d::characters::actor::BodyHealth>,
        ),
        ambition_platformer2d::actors::actor::PrimaryPlayerOnly,
    >,
    // Reset zeroes the local controller's slot gestures (reset/save identity is a
    // sanctioned PrimaryPlayer concern).
    mut slot_gestures: ResMut<ambition_platformer2d::characters::control::SlotInteractionState>,
) {
    let pressed = ambition_platformer2d::actors::control::seat_frame_this_tick(
        latches.as_deref(),
        rollback.as_deref(),
        &slots,
        &raw,
        PlayerSlot::PRIMARY,
    )
    .reset_pressed;
    if !pressed {
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
        return;
    };
    // Clear the press immediately so the inline engine update in
    // `player_control_phase` doesn't trigger a redundant `player.reset_to`
    // followed by another sandbox-side reset later this frame.
    ambition_platformer2d::actors::control::shape_seat_frame(
        latches.as_deref(),
        rollback.as_deref(),
        &mut slots,
        &mut raw,
        PlayerSlot::PRIMARY,
        |frame| frame.reset_pressed = false,
    );

    let mut clusters = cluster_item.as_clusters_mut();
    ambition_platformer2d::runtime::reset_sandbox(
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
