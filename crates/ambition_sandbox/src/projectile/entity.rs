//! Per-projectile ECS entity components (Stage 19 Phase 3c-ii).
//!
//! The player projectile pool moved off `PlayerProjectileState::bodies: Vec`
//! onto real entities. Each in-flight player projectile is one entity carrying:
//!
//! - [`crate::projectile::ProjectileGameplay`] — the projectile gameplay marker
//!   + state (kind / faction / lifetime / gravity / damage / bounces).
//! - [`crate::player::BodyKinematics`] — the SHARED kinematic body. Carrying the
//!   exact component the player / enemy / boss carry is what lets Phase 4 plug
//!   projectiles into the generic portal-transit machine "tag + go".
//! - [`ProjectileOwner`] — the firing player entity (attacker attribution +
//!   per-player pool routing).
//! - [`ProjectileSeq`] — a monotonic spawn id. Bevy query iteration order is
//!   unspecified; the step system collects + sorts by this so the per-frame
//!   processing order exactly reproduces the old `Vec` push order (determinism
//!   judge for `scripted_gameplay` + the projectile suites).
//! - [`PlayerProjectile`] — a marker tagging "this is a PLAYER-pool projectile
//!   entity" (the enemy pool stays a `Vec` for now; both coexist).
//! - [`ProjectileOwnerId`] — the spawning actor's string id (empty for player
//!   projectiles; carried for parity with the old `InFlightProjectile`).

use bevy::prelude::*;

/// Marker on every PLAYER-pool projectile entity. The enemy pool is still a
/// `Vec` (`EnemyProjectileState`), so this distinguishes the two during the
/// staged migration.
#[derive(Component)]
pub struct PlayerProjectile;

/// The firing player entity for a projectile. Used for hit attribution
/// (`HitEvent::attacker`) and to route the step loop per-player so each
/// player's projectiles are processed inside that player's trace-ordering
/// window (matching the old per-player `bodies` loop).
#[derive(Component, Clone, Copy, Debug)]
pub struct ProjectileOwner(pub Entity);

/// Monotonic spawn-sequence id. Assigned from [`ProjectileSeqCounter`] at spawn
/// time. The step system sorts in-flight projectiles by this so iteration order
/// is deterministic and reproduces the historical `Vec` order (oldest first).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectileSeq(pub u64);

/// The spawning actor's string id (GNU-ton's apple rain, the lasersword rider).
/// Empty for player projectiles — they attribute via `HitEvent::attacker`
/// instead — but carried so the entity is a faithful image of the old
/// `InFlightProjectile { body, owner_id }`.
#[derive(Component, Clone, Debug, Default)]
pub struct ProjectileOwnerId(pub String);

/// Monotonic source of [`ProjectileSeq`] ids. One global resource: a single
/// counter across all player pools is enough because seq only needs to be a
/// total order, and `step_player_projectiles` filters per owner before sorting.
#[derive(Resource, Default)]
pub struct ProjectileSeqCounter(pub u64);

impl ProjectileSeqCounter {
    /// Take the next id and advance the counter.
    pub fn next(&mut self) -> ProjectileSeq {
        let id = self.0;
        self.0 = self.0.wrapping_add(1);
        ProjectileSeq(id)
    }
}
