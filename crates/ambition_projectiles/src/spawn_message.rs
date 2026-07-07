//! `SpawnProjectile` decouples fire sites from projectile storage.
//!
//! Firing code writes a pool-tagged message; per-pool consumers create the
//! runtime projectile representation at the schedule point that preserves
//! first-tick timing for player and enemy projectiles.

use bevy::prelude::*;

use crate::{InFlightProjectile, ProjectileKind};

/// Which in-flight pool a spawned projectile belongs to. The two pools have
/// different storage (the player pool is a per-entity component, the enemy
/// pool a resource) and different spawn-frame timing, so the consumer routes
/// on this tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectilePool {
    /// The firing player's [`crate::PlayerProjectileState::bodies`].
    /// `owner` names which player entity's pool receives the body (co-op /
    /// possession safe).
    Player { owner: Entity },
    /// The global [`crate::enemy::EnemyProjectileState::bodies`]
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
    /// Named projectile kind for player-fired shots — attached to the entity as
    /// a `ProjectileKind` component so combat attribution / trace / render can
    /// read the named identity. `None` for kind-less (enemy) shots.
    pub kind: Option<ProjectileKind>,
}

// `SpawnProjectile::enemy` was removed: enemy-pool projectiles are now emitted
// as `ambition_vfx::Effect::Projectiles` and materialized by
// `enemy_projectile::apply_projectile_effects`. `SpawnProjectile` itself stays
// for the player pool until that path migrates too.
