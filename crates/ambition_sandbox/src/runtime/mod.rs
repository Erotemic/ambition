//! Sandbox session lifecycle: startup setup, full reset/respawn, and the
//! coarse `GameMode` state machine that gates input + cutscene flow.
//!
//! Distinct from `app/` (which owns the Bevy schedule wiring): this is
//! the simulation-side glue that `app` calls into.

pub mod camera_layers;
pub mod data;
pub mod game_mode;
pub mod reset;
pub mod setup;
