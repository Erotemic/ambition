//! Physical projectile vocabulary, construction, and flight services.
//!
//! Combat captures source facts and materializes requests with this model.
//! All controllers use the shared flight primitive.

pub mod collision_world;
/// The `projectile_visuals` schema: a game's projectile looks as pack data.
#[cfg(feature = "content_pack")]
pub mod content_schema;
pub mod diagnostics;
pub mod entity;
pub mod kind;
pub mod ownership;
mod snapshot_impls;
pub mod spawn;
pub mod birth;
pub mod state;
pub mod visual;

#[cfg(test)]
mod engine_tests;

pub use entity::{
    FiredByMoveInstance, LiveProjectile, ProjectileLaunch, ProjectileOwner, ProjectileSeq,
    ProjectileSeqCounter, WeaponShot,
};
pub use kind::{FireballChargeTuning, FireballFlight, ProjectileKind};
pub use spawn::{ProjectileSpawner, SpawnFailure};
pub use birth::{
    build_in_flight_projectile, ProjectileBirth,
};
pub use state::PlayerProjectileState;
pub use visual::{
    ProjectileArt, ProjectileArtSource, ProjectileExpiryBurst, ProjectileRenderSize,
    ProjectileRotation, ProjectileVisualAppExt, ProjectileVisualCatalog, ProjectileVisualId,
};

// Keep all producers on the shared projectile-physics primitive through this facade.
pub use ambition_platformer2d_shared_tangle::projectile::{
    query_world_collision, query_world_collision_in, ProjectileWorldContact, ProjectileWorldResponse, InFlightProjectile, ProjectileBody,
    ProjectileGameplay, ProjectileSpec, Rebound, WorldHitOutcome, WorldHitPolicy,
};
pub use ambition_projectile_spec::ProjectileSpawn;

// Preserve the projectile-facing motion-input facade.
pub use ambition_input::{
    MotionDirection, MotionInputBuffer, MotionSample, MotionTechnique, MotionTechniqueAppExt,
    MotionTechniqueCatalog,
};

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
