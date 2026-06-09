//! `SpawnProjectile` — the decoupled spawn seam (Stage 19 Phase 3b).
//!
//! Firing code (player charge/motion release, enemy/boss volley consumers)
//! no longer pushes directly into a `bodies: Vec`. It WRITES a
//! [`SpawnProjectile`] message describing the new in-flight projectile and
//! which pool it belongs to; a per-pool consumer system drains the messages
//! and performs the actual `Vec` push. The Vec pools still exist (Phase 3c
//! turns them into entities) — this step only decouples *spawn* from *storage*
//! so the storage can change underneath without touching every fire site.
//!
//! Timing is preserved by where the consumers are scheduled:
//! * Player pool — consumed AFTER `update_projectiles`, so a freshly-fired
//!   body lands in `bodies` and first ticks next frame (the old push happened
//!   after the per-frame tick loop → same one-frame latency).
//! * Enemy pool — consumed BEFORE `update_enemy_projectiles`, so a body
//!   spawned this tick advances one step this frame (matching the EFFECTS-stage
//!   consumers that previously pushed directly before the update).

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
