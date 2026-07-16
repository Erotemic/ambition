//! Sandbox PLAYER-faction projectile glue.
//!
//! The reusable projectile MODEL — shot vocabulary (`ProjectileKind` / the open
//! `ProjectileVisualId` + content-owned visual catalog), the ECS components,
//! `PlayerProjectileState`, the `SpawnProjectile`
//! pool + player-pool spawner, and pure portal transit — now lives in the
//! [`ambition_projectiles`] crate (E2 carve) and is re-exported below so
//! `crate::projectile::*` paths resolve unchanged for every sandbox consumer.
//!
//! What STAYS here is the victim/world/anim-woven sim STEPPERS that cannot leave
//! until the boss/actor/player domains carve (E6/E7) and the world overlay lands
//! in `ambition_world` (W3): the unified [`systems::step_projectiles`] (queries
//! bosses/breakables/actors, emits `HitEvent`, parry-heals the player), the
//! [`systems::charge_projectile_input`] player-input/anim driver, and the
//! `ambition_projectiles::collision_world::ProjectileCollisionWorld` param, which
//! reads the ECS world overlay (it came home in R4).
//! They CONSUME the model crate — the legal sim → model direction.

pub use ambition_projectiles::*;

pub mod systems;
pub use systems::{charge_projectile_input, step_projectiles};

#[cfg(test)]
mod tests;
