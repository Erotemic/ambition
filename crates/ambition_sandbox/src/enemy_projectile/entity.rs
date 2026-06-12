//! ECS entity components for the enemy/boss projectile pool.
//!
//! Each in-flight hostile or boss-authored projectile is one entity carrying shared
//! [`crate::player::BodyKinematics`], shared
//! [`crate::projectile::ProjectileGameplay`], a deterministic
//! [`crate::projectile::ProjectileSeq`], an owner-id string for visuals/traces, and
//! the [`EnemyProjectile`] pool marker.
//!
//! Enemy and player projectile pools are distinct archetypes, but both use the
//! shared kinematic/gameplay halves. Step systems filter by marker and sort by
//! `ProjectileSeq` so processing stays deterministic despite unspecified Bevy query
//! iteration order.
//!
//! Enemy projectiles intentionally carry no `ProjectileOwner(Entity)`: hits report
//! `attacker = None`, while owner-id strings handle visuals and self-filtering.

use bevy::prelude::*;

/// Marker on every ENEMY-pool projectile entity. Distinguishes the enemy pool
/// from the player pool (`crate::projectile::PlayerProjectile`) — both carry
/// the shared `BodyKinematics` + `ProjectileGameplay`, so the marker is what
/// each pool's step system filters on.
#[derive(Component)]
pub struct EnemyProjectile;
