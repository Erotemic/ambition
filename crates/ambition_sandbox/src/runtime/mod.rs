//! Sandbox session lifecycle: startup setup, full reset/respawn, and the
//! coarse `GameMode` state machine that gates input + cutscene flow.
//!
//! Distinct from `app/` (which owns the Bevy schedule wiring): this is
//! the simulation-side glue that `app` calls into.

use bevy::prelude::Message;

pub mod camera_layers;
pub mod data;
pub mod game_mode;
pub mod reset;
pub mod setup;

/// Request to (re)spawn the active room's static visuals + parallax layers.
///
/// The sim side (e.g. [`reset::process_sandbox_reset_request`]) emits this after
/// it has flipped the active room; the presentation layer consumes it and calls
/// the render-only `spawn_room_visuals` / `spawn_parallax_layers` helpers, reading
/// the active room from [`crate::rooms::RoomSet`]. This keeps the sim from
/// reaching into the render layer to spawn visual entities — a headless build
/// simply has no consumer, which is correct (it needs no visuals).
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct RespawnRoomVisualsRequested;
