//! Per-projectile ECS entity components (Stage 19 Phase 3c-ii).
//!
//! Projectiles moved off the historical Vec-backed player/enemy pools onto real
//! ECS entities. Each in-flight projectile is one entity carrying:
//!
//! - [`crate::ProjectileGameplay`] — the projectile gameplay marker
//!   + state (kind / faction / lifetime / gravity / damage / bounces).
//! - [`ambition_platformer2d_core::BodyKinematics`] — the SHARED kinematic body. Carrying the
//!   exact component the player / enemy / boss carry is what lets Phase 4 plug
//!   projectiles into the generic portal-transit machine "tag + go".
//! - [`ProjectileOwner`] — the firing body entity (attacker attribution,
//!   allegiance freezing, and presentation-source inheritance).
//! - [`ProjectileSeq`] — a monotonic spawn id. Bevy query iteration order is
//!   unspecified; the step system collects + sorts by this so the per-frame
//!   processing order exactly reproduces the old `Vec` push order (determinism
//!   judge for `scripted_gameplay` + the projectile suites).
//! - [`crate::ProjectileKind`] — present only when the shot uses the engine's named
//!   projectile vocabulary. Open-visual shots need no second family marker.
//! - [`crate::ProjectileVisualId`] — the presentation selection for every shot.

use bevy::prelude::*;

/// Marker on EVERY in-flight projectile entity. The unified stepper, reset
/// lifecycle, perception, and debug tooling all query this one occurrence fact.
/// Combat side is frozen separately by the actor-domain `ProjectileAllegiance`;
/// presentation is selected by `ProjectileVisualId` and optional `ProjectileKind`.
///
/// ⭐⭐ IT REQUIRES THE VICTIM LEDGER, so every road that makes a projectile gets
/// one and no road has to remember. `step_projectiles` takes
/// `&mut ProjectileHits`, which means a shot spawned without one is not in the
/// query at all — it sits still and hits nothing. Three separate hand-built test
/// fixtures each listed what production spawns and each would have needed
/// patching; a fourth was one edit away. `#[require]` is the list.
#[derive(Component)]
#[require(ambition_platformer2d_shared_tangle::projectile::ProjectileHits)]
pub struct LiveProjectile;

/// The firing body entity for a projectile. Used for hit attribution, for
/// freezing faction/team allegiance at materialization, and for inheriting the
/// firer's presentation source. The unified step loop is global and does not
/// partition projectiles by owner.
#[derive(Component, Clone, Copy, Debug)]
pub struct ProjectileOwner(pub Entity);

impl bevy::ecs::entity::MapEntities for ProjectileOwner {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.0 = mapper.get_mapped(self.0);
    }
}

/// Monotonic spawn-sequence id. Assigned from [`ProjectileSeqCounter`] at spawn
/// time. The step system sorts in-flight projectiles by this so iteration order
/// is deterministic and reproduces the historical `Vec` order (oldest first).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectileSeq(pub u64);

/// Monotonic source of [`ProjectileSeq`] ids. One global resource is the
/// authority because every live projectile shares one deterministic processing
/// order regardless of producer, presentation family, or allegiance.
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
