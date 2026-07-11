//! Legacy actor-side name for the enemy-projectile effect-request spawn executor.
//!
//! The canonical implementation now lives in
//! [`ambition_projectiles::enemy::apply_enemy_projectile_effect_requests`]. It
//! materializes `ambition_vfx::Effect::Projectiles` requests as enemy-pool
//! projectile entities and does not inspect actor/player/victim state. This
//! module keeps the old `crate::enemy_projectile::apply_projectile_effects` name
//! for actor-internal tests and transitional call sites while runtime scheduling
//! routes through `ambition_runtime::projectile_schedule`.

#[cfg(test)]
use bevy::prelude::*;

#[cfg(test)]
use crate::projectile::ProjectileSeqCounter;

pub use ambition_projectiles::enemy::apply_enemy_projectile_effect_requests as apply_projectile_effects;

#[cfg(test)]
mod tests;
