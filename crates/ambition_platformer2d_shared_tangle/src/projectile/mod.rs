//! Content-free projectile physics: authored specs, kinematic state, and world collision.
//!
//! Consumers own projectile spawning, damage routing, and named projectile content.

pub mod body;
pub mod collision;
pub mod spec;

pub use body::{
    InFlightProjectile, ProjectileBody, ProjectileGameplay, ProjectileHits, ProjectileSolidHit,
};
pub use collision::{resolve_world_collision, WorldHitOutcome, WorldHitPolicy};
// Authored spawn intent is owned by `ambition_projectile_spec`, below this physics layer.
pub use ambition_projectile_spec::ProjectileSpawn;
pub use spec::ProjectileSpec;
