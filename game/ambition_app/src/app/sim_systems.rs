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

use bevy::prelude::*;

use ambition_platformer2d::sim::{PlayerSlot, SeatRawFrames};

/// Detect a player-pressed reset (the Reset button / `controls.reset_pressed`)
/// and ASK for a room replay.
///
/// ⛔⛔ IT USED TO PERFORM THE RESET ITSELF — `reset_sandbox` on the primary
/// avatar, then the room-features message — which made the reset button a second
/// authority on what a replay IS, running before anything had decided the replay
/// could happen. It writes the request now; `runtime::admit_room_replay` owns
/// admission and every consequence, so the button, a death, and a content "try
/// again" beat all take one road.
///
/// It still clears the primary seat's `reset_pressed` after reading it, so the
/// engine path inside `update_player_control_with_clusters` does not re-trigger
/// on the same frame.
///
/// Gated by `gameplay_allowed`: paused / dialogue modes don't process reset
/// input.
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
    mut replay: MessageWriter<ambition_platformer2d::actors::session::reset::RoomReplayRequested>,
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
    // Clear the press immediately so the inline engine update in
    // `player_control_phase` doesn't ask twice on the same frame.
    ambition_platformer2d::actors::control::shape_seat_frame(
        latches.as_deref(),
        rollback.as_deref(),
        &mut slots,
        &mut raw,
        PlayerSlot::PRIMARY,
        |frame| frame.reset_pressed = false,
    );
    replay.write(ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual());
}
