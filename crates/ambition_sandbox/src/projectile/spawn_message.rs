//! `SpawnProjectile` decouples fire sites from projectile storage.
//!
//! Firing code writes a pool-tagged message; per-pool consumers create the
//! runtime projectile representation at the schedule point that preserves
//! first-tick timing for player and enemy projectiles.

use bevy::prelude::*;

use crate::projectile::InFlightProjectile;

/// Which in-flight pool a spawned projectile belongs to. The two pools have
/// different storage (the player pool is a per-entity component, the enemy
/// pool a resource) and different spawn-frame timing, so the consumer routes
/// on this tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectilePool {
    /// The firing player's [`crate::projectile::PlayerProjectileState::bodies`].
    /// `owner` names which player entity's pool receives the body (co-op /
    /// possession safe).
    Player { owner: Entity },
    /// The global [`crate::enemy_projectile::EnemyProjectileState::bodies`]
    /// (pirate volleys, boss bolts, apple rain, wielded ranged attacks).
    Enemy,
}

/// A request to add one in-flight projectile to a pool. Replaces the direct
/// `state.bodies.push(..)` / `EnemyProjectileState::spawn(..)` calls at the
/// fire sites; a per-pool consumer performs the push.
#[derive(Message, Clone, Debug)]
pub struct SpawnProjectile {
    pub pool: ProjectilePool,
    pub projectile: InFlightProjectile,
}

impl SpawnProjectile {
    /// Build an enemy-pool spawn message from a spawn request + faction. The
    /// body-building lives in [`crate::enemy_projectile::EnemyProjectileState::build`]
    /// so the message path and the direct-`spawn` path (tests) stay in sync.
    pub fn enemy(
        request: crate::enemy_projectile::EnemyProjectileSpawn,
        faction: crate::projectile::ProjectileFaction,
    ) -> Self {
        Self {
            pool: ProjectilePool::Enemy,
            projectile: crate::enemy_projectile::EnemyProjectileState::build(request, faction),
        }
    }
}
