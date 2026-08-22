//! Reusable, content-free projectile vocabulary and materialization.
//!
//! All projectile producers share one spawn-request path and the physics
//! primitive re-exported from `ambition_platformer2d_shared_tangle::projectile`,
//! so ownership does not select a separate motion path. Victim-side hit routing
//! and player charge/animation stepping remain in
//! `ambition_platformer2d_actor_monolith` while they still depend on unextracted
//! actor/world state.

pub mod collision_world;
pub mod diagnostics;
pub mod entity;
pub mod kind;
pub mod materialize;
#[cfg(feature = "portal")]
pub mod portal_transit;
mod snapshot_impls;
pub mod spawn;
pub mod spawn_request;
pub mod state;
pub mod visual;

#[cfg(test)]
mod engine_tests;

pub use entity::{LiveProjectile, ProjectileOwner, ProjectileSeq, ProjectileSeqCounter};
pub use kind::{FireballChargeTuning, ProjectileKind};
pub use materialize::{
    materialize_projectiles_for_next_tick, materialize_projectiles_for_this_tick,
};
pub use spawn::{ProjectileSpawner, SpawnFailure};
pub use spawn_request::{
    build_in_flight_projectile, ProjectilePresentation, ProjectileSpawnRequest, ProjectileStart,
};
pub use state::PlayerProjectileState;
pub use visual::{
    ProjectileArt, ProjectileArtSource, ProjectileExpiryBurst, ProjectileRenderSize,
    ProjectileRotation, ProjectileVisualAppExt, ProjectileVisualCatalog, ProjectileVisualId,
};

// Keep all producers on the shared projectile-physics primitive through this facade.
pub use ambition_platformer2d_shared_tangle::projectile::{
    resolve_world_collision, InFlightProjectile, ProjectileBody, ProjectileGameplay,
    ProjectileSolidHit, ProjectileSpec, WorldHitOutcome, WorldHitPolicy,
};
pub use ambition_projectile_spec::ProjectileSpawn;

// Preserve the projectile-facing motion-input facade.
pub use ambition_input::{
    MotionDirection, MotionInputBuffer, MotionSample, MotionTechnique, MotionTechniqueAppExt,
    MotionTechniqueCatalog,
};
#[cfg(feature = "portal")]
pub use portal_transit::try_projectile_portal_transit;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
