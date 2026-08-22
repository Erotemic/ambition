//! Actor spawn/surface state and shared movement integration for brain-driven actors.
//!
//! Grounded, aerial, and adhesive actors all integrate through `ae::step_motion`.

use super::*;

mod integration;
pub use integration::ContactAttack;

/// Spatial baseline restored by same-room actor reset.
/// Composite actors remain separate entities, so only position/body size belong here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorSpawnState {
    /// World position the actor spawned at.
    pub pos: ae::Vec2,
    /// Authored body size.
    pub size: ae::Vec2,
}

// TODO(compat-remove): migrate remaining `crate::features::ActorSurfaceState` callers to
// `ambition_platformer2d_core::ActorSurfaceState`, then delete this re-export.
pub use ambition_platformer2d_core::ActorSurfaceState;

// TODO(compat-remove): migrate the remaining `crate::features::RespawnPolicy` caller to
// `ambition_entity_catalog::placements::RespawnPolicy`, then delete this re-export.
pub use ambition_entity_catalog::placements::RespawnPolicy;

/// Shared suffix for persistent `_dead_until_rest` flags.
pub const ENEMY_DEAD_UNTIL_REST_SUFFIX: &str = "_dead_until_rest";
