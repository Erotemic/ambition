//! Test-only helpers for the entity-era enemy projectile pool.
//!
//! The enemy pool's in-flight bodies are ECS entities now, so the ~15 unit
//! tests that used to read `EnemyProjectileState.bodies` and inject via
//! `EnemyProjectileState::spawn(..)` go through these instead. Mirrors the
//! player pool's `spawn_player_projectile` / `projectile_bodies` test helpers.

use crate::enemy_projectile::entity::EnemyProjectile;
use crate::enemy_projectile::{EnemyProjectileSpawn, EnemyProjectileState};
use crate::projectile::{
    ProjectileFaction, ProjectileGameplay, ProjectileOwnerId, ProjectileSeq, ProjectileSeqCounter,
};
use bevy::prelude::*;

/// Directly spawn an in-flight enemy projectile entity — the entity-era
/// equivalent of the old `EnemyProjectileState::spawn(..)` /
/// `spawn_with_faction(..)` test setup. Builds the body via the shared
/// `EnemyProjectileState::build` mapping (so it matches the production
/// `SpawnProjectile` path exactly) and assigns the next monotonic
/// `ProjectileSeq` so injected bodies keep a stable order.
pub(crate) fn spawn_enemy_projectile(
    app: &mut App,
    request: EnemyProjectileSpawn,
    faction: ProjectileFaction,
) {
    let projectile = EnemyProjectileState::build(request, faction);
    let seq = {
        let mut counter = app
            .world_mut()
            .get_resource_or_insert_with(ProjectileSeqCounter::default);
        counter.next()
    };
    app.world_mut().spawn((
        projectile.body.kin,
        projectile.body.game,
        seq,
        ProjectileOwnerId(projectile.owner_id),
        crate::projectile::LiveProjectile,
        EnemyProjectile,
        Name::new("Enemy projectile (test)"),
    ));
}

/// Collect the in-flight enemy projectile bodies, sorted by spawn sequence
/// (oldest first) — the same order the old `EnemyProjectileState.bodies` Vec
/// presented. Recomposes an [`crate::projectile::InFlightProjectile`] from the
/// entity's split `BodyKinematics` + `ProjectileGameplay` + `ProjectileOwnerId`
/// so the tests keep asserting on `.body.kin` / `.body.game` / `.owner_id`
/// exactly as before.
pub(crate) fn enemy_projectile_bodies(app: &mut App) -> Vec<crate::projectile::InFlightProjectile> {
    let world = app.world_mut();
    // `try_query_filtered` returns `Err` when the projectile component types
    // were never registered in this World — which is exactly the "no enemy
    // projectile ever spawned" case the empty-pool tests assert. Treat that as
    // an empty pool rather than panicking.
    let Some(mut q) = world.try_query_filtered::<(
        &crate::player::BodyKinematics,
        &ProjectileGameplay,
        &ProjectileOwnerId,
        &ProjectileSeq,
    ), With<EnemyProjectile>>() else {
        return Vec::new();
    };
    let mut rows: Vec<(ProjectileSeq, crate::projectile::InFlightProjectile)> = q
        .iter(world)
        .map(|(kin, game, owner, seq)| {
            (
                *seq,
                crate::projectile::InFlightProjectile {
                    body: crate::projectile::ProjectileBody::from_parts(*kin, *game),
                    owner_id: owner.0.clone(),
                },
            )
        })
        .collect();
    rows.sort_by_key(|(seq, _)| *seq);
    rows.into_iter().map(|(_, body)| body).collect()
}
