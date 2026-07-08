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

pub mod camera_layers;
pub mod data;
pub mod game_mode;
pub mod reset;
pub mod setup;

pub use ambition_world::rooms::RespawnRoomVisualsRequested;
