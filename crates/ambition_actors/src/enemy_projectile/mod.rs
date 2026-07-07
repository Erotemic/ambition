//! Enemy-fired projectile glue (pirate volleys etc).
//!
//! The enemy-shot MODEL — the `EnemyProjectile` marker + `EnemyProjectileState`/
//! `EnemyProjectileSpawn` — now lives in [`ambition_projectiles::enemy`] (E2
//! carve) and is re-exported below so `crate::enemy_projectile::*` paths resolve
//! unchanged. What STAYS here is the victim-side effect stepper
//! ([`systems::apply_projectile_effects`], which damages the player) — sim-heart
//! material that consumes the model crate.

pub use ambition_projectiles::enemy::*;

pub mod systems;
pub use systems::apply_projectile_effects;

#[cfg(test)]
pub(crate) mod test_support;
