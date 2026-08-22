//! Test-only helpers for open-visual projectile entities.
//!
//! Open-visual projectiles are ECS entities. These helpers inject the same
//! body shape production materialization uses while keeping collision/routing
//! tests independent of message scheduling.

use crate::combat::components::ActorFaction;
use crate::projectile::ProjectileSpawn;
use crate::projectile::{
    build_in_flight_projectile, ProjectileGameplay, ProjectileOwner, ProjectileSeq,
    ProjectileSeqCounter,
};
use bevy::prelude::*;

/// Directly spawn an open-visual projectile entity for collision/routing tests.
/// Builds the body via the same request lowering production materialization uses
/// and assigns the next monotonic `ProjectileSeq` so injected bodies keep a
/// stable order.
pub(crate) fn spawn_test_projectile(
    app: &mut App,
    request: ProjectileSpawn,
    faction: ActorFaction,
) {
    let projectile = build_in_flight_projectile(request);
    let seq = {
        let mut counter = app
            .world_mut()
            .get_resource_or_insert_with(ProjectileSeqCounter::default);
        counter.next()
    };
    // Damage routes off the FIRER's real faction (looked up from the projectile's
    // owner). So an `ActorFaction::Player` test shot must carry a Player-faction
    // OWNER to route as a player shot — spawn a bare faction-carrier entity and
    // own the projectile to it.
    let owner = app.world_mut().spawn(faction).id();
    app.world_mut().spawn((
        projectile.body.kin,
        projectile.body.game,
        seq,
        ProjectileOwner(owner),
        crate::projectile::LiveProjectile,
        Name::new("Test projectile"),
    ));
}

pub(crate) fn spawn_ownerless_projectile(app: &mut App, request: ProjectileSpawn) {
    let projectile = build_in_flight_projectile(request);
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
        crate::projectile::LiveProjectile,
        Name::new("Ownerless projectile (test)"),
    ));
}

/// Collect the in-flight test projectile bodies, sorted by spawn sequence
/// (oldest first), matching production sequence ordering. Recomposes an [`crate::projectile::InFlightProjectile`] from the
/// entity's split `BodyKinematics` + `ProjectileGameplay` so the historical
/// collision tests can keep asserting on the reconstructed flight body.
pub(crate) fn live_projectile_bodies(app: &mut App) -> Vec<crate::projectile::InFlightProjectile> {
    let world = app.world_mut();
    // `try_query_filtered` returns `Err` when the projectile component types
    // were never registered in this World — exactly the "no projectile ever
    // spawned" case some historical collision fixtures assert. Treat that as
    // an empty set rather than panicking.
    let Some(mut q) = world.try_query_filtered::<(
        &crate::actor::BodyKinematics,
        &ProjectileGameplay,
        &ProjectileSeq,
    ), With<crate::projectile::LiveProjectile>>() else {
        return Vec::new();
    };
    let mut rows: Vec<(ProjectileSeq, crate::projectile::InFlightProjectile)> = q
        .iter(world)
        .map(|(kin, game, seq)| {
            (
                *seq,
                crate::projectile::InFlightProjectile {
                    body: crate::projectile::ProjectileBody::from_parts(*kin, *game),
                },
            )
        })
        .collect();
    rows.sort_by_key(|(seq, _)| *seq);
    rows.into_iter().map(|(_, body)| body).collect()
}
