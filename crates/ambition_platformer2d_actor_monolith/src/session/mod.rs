//! Ambition-game session lifecycle: startup setup ([`setup`]), full
//! reset/respawn ([`reset`]), RON data manifests ([`data`]), and setup glue.
//! Coarse `GameMode` state and camera layer markers now live in
//! `ambition_platformer2d_shared_tangle`.
//!
//! Name overlap warning: this is the in-crate session runtime, NOT the
//! separate `ambition_platformer2d_shared_tangle` crate (re-exported here as
//! `crate::platformer_runtime`), which holds content-free engine
//! primitives. This module is sim-side session glue that `app/` (the
//! schedule wiring) calls into.

pub mod data;
/// The death interlude and the roster question that decides a level reset
/// (ADR 0033).
pub mod death;
/// The DURABLE save horizon: occurrence whereabouts, custody and runtime-minted
/// descriptions on disk, and the load that resumes from them.
pub mod durable_horizon;
pub mod lifecycle_commit;
pub mod reset;
pub mod setup;
pub mod teardown;

pub use teardown::{reset_session_scoped_resources_on_retire, SessionTeardownPlugin};
