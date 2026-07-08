//! Sandbox SESSION lifecycle: startup setup ([`setup`]), full
//! reset/respawn ([`reset`]), RON data manifests ([`data`]), and setup glue.
//! Coarse `GameMode` state and camera layer markers now live in
//! `ambition_platformer_primitives`.
//!
//! Name overlap warning: this is the in-crate session runtime, NOT the
//! separate `ambition_platformer_primitives` crate (re-exported here as
//! `crate::platformer_runtime`), which holds content-free engine
//! primitives. This module is sim-side session glue that `app/` (the
//! schedule wiring) calls into.

pub mod data;
pub mod reset;
pub mod setup;

pub use ambition_world::rooms::RespawnRoomVisualsRequested;
