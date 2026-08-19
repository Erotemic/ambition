//! Projectile schedule seams owned by the runtime composition tier.
//!
//! The projectile MODEL and the one authoritative spawn-request/materialization
//! road live in `ambition_projectiles`. The victim-routing and charge-input
//! steppers still live in the actor sim heart because they touch un-carved
//! actor/player/boss/world state. Callers outside the runtime should schedule
//! against these runtime names rather than reaching through those owners; this
//! keeps the residual glue enumerable while the remaining actor-side steppers
//! are split.

pub use ambition_platformer2d_actor_monolith::projectile::{
    charge_projectile_input, stamp_new_projectile_allegiance, step_projectiles,
    ProjectileAllegiance, ProjectileStepSet,
};
pub use ambition_projectiles::collision_world::ProjectileCollisionWorld;
pub use ambition_projectiles::materialize_projectiles_for_next_tick;
pub use ambition_projectiles::materialize_projectiles_for_this_tick;
