//! Proto-runtime facade for reusable platformer systems.
//!
//! The import-clean lifecycle vocabulary, schedule sets, pure portal-map math,
//! and body-transit velocity helper now live in the standalone
//! `ambition_platformer_runtime` crate (Stage 13 / Task K + Stage M1 of
//! `docs/planning/plugin_refactor/14_action_plan.md`). This module re-exports
//! them unchanged so every
//! `crate::platformer_runtime::{lifecycle, schedule, math, transit}` path keeps
//! resolving for sandbox callers.
//!
//! The not-yet-extracted remainder stays here because each still reaches back
//! into the sandbox and is therefore NOT import-clean:
//! - `collision` -> `crate::engine_core`
//! - `orientation` -> `crate::physics`, `crate::player`, `crate::features`, `crate::WorldTime`
//!
//! These move out in a later pass once a generic body/gravity/world abstraction
//! decouples them from Ambition content.

// Re-export the extracted crate's modules so existing paths resolve unchanged.
pub use ambition_platformer_runtime::{lifecycle, math, schedule, transit};

// Still-local modules: the not-yet-extracted remainder.
pub mod collision;
pub mod orientation;
pub mod prelude;
