//! Proto-runtime facade for reusable platformer systems.
//!
//! The import-clean lifecycle vocabulary, schedule sets, pure portal-map math,
//! and body-transit velocity helper now live in the standalone
//! `ambition_platformer_primitives` crate (Stage 13 / Task K + Stage M1 of
//! `docs/current/state.md`). This module re-exports
//! them unchanged so every
//! `crate::platformer_runtime::{lifecycle, schedule, math, transit}` path keeps
//! resolving for sandbox callers.
//!
//! Stage 16 moved the generic body / world-query / body-kinematics surface into
//! the crate too: `body` and `collision` are now thin facades re-exporting
//! `ambition_platformer_primitives::{body, world_query}`.
//!
//! The not-yet-extracted remainder stays here because it still reaches back into
//! the sandbox and is therefore NOT import-clean:
//! - `orientation` -> `crate::physics`, `crate::player`, `crate::features`, `ambition_time::WorldTime`
//!
//! It moves out once gravity (`crate::physics`) is in-crate (Stage 16 / S4–S5).

// Re-export the extracted crate's modules so existing paths resolve unchanged.
pub use ambition_platformer_primitives::{gravity, lifecycle, math, schedule, transit};

// Facade modules re-exporting extracted runtime surfaces (Stage 16 / S1–S2).
pub mod body;
pub mod collision;
// Still-local: the not-yet-extracted remainder (orientation, until gravity moves).
pub mod orientation;
pub mod prelude;
