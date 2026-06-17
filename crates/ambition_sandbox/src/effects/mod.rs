//! The effect vocabulary + executor now live in the reusable, content-free
//! [`ambition_vfx`] crate. Re-exported here so the historical
//! `crate::effects::…` paths are unchanged.
//!
//! The substrate-bound executors stay in the lib, next to what they touch:
//! `apply_summon_effects` (`features::ecs::spawn_actors`, the enemy roster) and
//! `apply_projectile_effects` (`enemy_projectile::systems`, the projectile
//! pool) read this crate's [`Effect`] enum.

pub use ambition_vfx::*;
