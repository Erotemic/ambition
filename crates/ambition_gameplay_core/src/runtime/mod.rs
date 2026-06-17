//! Sandbox SESSION lifecycle: startup setup ([`setup`]), full
//! reset/respawn ([`reset`]), the coarse [`game_mode::GameMode`] state
//! machine that gates input + cutscene flow, RON data manifests
//! ([`data`]), and camera layering ([`camera_layers`]).
//!
//! Name overlap warning: this is the in-crate session runtime, NOT the
//! separate `ambition_platformer_primitives` crate (re-exported here as
//! `crate::platformer_runtime`), which holds content-free engine
//! primitives. This module is sim-side session glue that `app/` (the
//! schedule wiring) calls into.

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
