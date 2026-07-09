//! Enemy-fired projectile glue (pirate volleys etc).
//!
//! The enemy-shot MODEL — the `EnemyProjectile` marker + `EnemyProjectileState`/
//! `EnemyProjectileSpawn` — now lives in [`ambition_projectiles::enemy`] (E2
//! carve) and is re-exported below so `crate::enemy_projectile::*` paths resolve
//! unchanged. The canonical effect-request spawn executor now lives in
//! [`ambition_projectiles::enemy::apply_enemy_projectile_effect_requests`]. This
//! module keeps the legacy system name for actor-internal tests and transitional
//! call sites only; runtime scheduling goes through `ambition_runtime`.

pub use ambition_projectiles::enemy::*;

pub mod systems;
pub use systems::apply_projectile_effects;

#[cfg(test)]
pub(crate) mod test_support;
