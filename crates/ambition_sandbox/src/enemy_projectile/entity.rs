//! Per-projectile ECS entity components for the ENEMY pool (Stage 19
//! Phase 3c-iii).
//!
//! Mirrors the player pool's `crate::projectile::entity` module: the enemy /
//! boss projectile pool moved off `EnemyProjectileState::bodies: Vec` onto
//! real entities. Each in-flight enemy projectile is one entity carrying:
//!
//! - [`crate::projectile::ProjectileGameplay`] — the projectile gameplay
//!   marker + state (kind / faction / lifetime / gravity / damage / bounces).
//!   Faction is `Enemy` for hostile shots and `Player` for a wielded ranged
//!   boss attack (sentry / meteor / volley) so the step system routes its
//!   damage at enemies instead of the player.
//! - [`crate::player::BodyKinematics`] — the SHARED kinematic body, the exact
//!   component player / enemy / boss carry, so Phase 4 can plug projectiles
//!   into the generic portal-transit machine "tag + go".
//! - [`crate::projectile::ProjectileSeq`] — a monotonic spawn id from the
//!   SHARED [`crate::projectile::ProjectileSeqCounter`]. Bevy query iteration
//!   order is unspecified; the step system collects + sorts by this so the
//!   per-frame processing order exactly reproduces the old `Vec` push order
//!   (the determinism judge for `scripted_gameplay` + the enemy projectile
//!   suites).
//! - [`crate::projectile::ProjectileOwnerId`] — the spawning actor's string id
//!   (`gnu_ton_apple:*`, `lasersword:*`, `player_sentry`, …). Drives visuals
//!   routing + debug traces, mirroring the old `InFlightProjectile.owner_id`.
//! - [`EnemyProjectile`] — a marker tagging "this is an ENEMY-pool projectile
//!   entity" (mirrors the player pool's `PlayerProjectile`). The two pools are
//!   distinct archetypes so each pool's step system queries only its own.
//!
//! There is intentionally NO `ProjectileOwner(Entity)` (unlike the player
//! pool): enemy-projectile hits always set `HitEvent::attacker = None`, so the
//! owning entity is never needed — only the `owner_id` string (visuals + self
//! filter) is carried.

use bevy::prelude::*;

/// Marker on every ENEMY-pool projectile entity. Distinguishes the enemy pool
/// from the player pool (`crate::projectile::PlayerProjectile`) — both carry
/// the shared `BodyKinematics` + `ProjectileGameplay`, so the marker is what
/// each pool's step system filters on.
#[derive(Component)]
pub struct EnemyProjectile;
