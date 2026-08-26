//! Actor spawn/surface state and shared movement integration for brain-driven actors.
//!
//! Grounded, aerial, and adhesive actors all integrate through `ae::step_motion`.

use super::*;

mod integration;
pub use integration::ContactAttack;

// TODO(compat-remove): migrate remaining `crate::features::ActorSurfaceState` callers to
// `ambition_platformer2d_core::ActorSurfaceState`, then delete this re-export.
pub use ambition_platformer2d_core::ActorSurfaceState;

// TODO(compat-remove): migrate the remaining `crate::features::RespawnPolicy` caller to
// `ambition_entity_catalog::placements::RespawnPolicy`, then delete this re-export.
pub use ambition_entity_catalog::placements::RespawnPolicy;

/// Shared suffix for persistent `_dead_until_rest` flags.
pub const ENEMY_DEAD_UNTIL_REST_SUFFIX: &str = "_dead_until_rest";
