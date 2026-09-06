//! Test-only fixtures for ability modules.
//!
//! Ability unit tests mostly need the same minimal primary-player entity: a body,
//! action set, held item, and mana. Keeping that bundle here lets each ability
//! test focus on the behavior it is asserting instead of repeating spawn wiring.

use ambition_combat::held_items::HeldItem;
use ambition_characters::brain::{held_item_by_id, ActionSet};
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use bevy::prelude::*;

pub fn spawn_primary_player_holding(app: &mut App, held_item_id: &str) -> Entity {
    let spec = held_item_by_id(held_item_id).unwrap();
    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            ActorControl::default(),
            ActionSet::default(),
            HeldItem::new(spec),
            ambition_platformer2d_core::movement::MotionModel::default(),
            // Ability systems read the per-tick resolved frame (ADR 0024) and
            // the FULL movement clusters (the transit authority reconciles
            // contacts/attachment through `BodyClusterQueryData`) — both live
            // inside `AncillaryMovementBundle`, like production spawns.
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ae::BodyClusterScratch::new_with_abilities(
                    ae::Vec2::new(100.0, 100.0),
                    ae::AbilitySet::default(),
                ),
            ),
        ))
        .id();
    // Ability systems now key on the controlled subject, not a `PrimaryPlayer`
    // filter. In tests the spawned player IS the controlled body.
    app.insert_resource(ControlledSubject(Some(entity)));
    entity
}

/// A primary player holding `held_item_id` at an explicit `pos` / `facing`, with
/// NO `BodyMana` — the minimal bundle the traversal-ability tests (blink /
/// grapple / mark-recall) spawn. One definition so the body/`BodyBaseSize`
/// bundle can't drift across those modules; each caller passes only the pos /
/// facing it cares about.
pub fn spawn_primary_player_holding_at(
    app: &mut App,
    held_item_id: &str,
    pos: ae::Vec2,
    facing: f32,
) -> Entity {
    let spec = held_item_by_id(held_item_id).unwrap();
    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing,
            },
            ActorControl::default(),
            ActionSet::default(),
            HeldItem::new(spec),
            ambition_platformer2d_core::movement::MotionModel::default(),
            // Ability systems read the per-tick resolved frame (ADR 0024) and
            // the FULL movement clusters (the transit authority reconciles
            // contacts/attachment through `BodyClusterQueryData`) — both live
            // inside `AncillaryMovementBundle`, like production spawns.
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ae::BodyClusterScratch::new_with_abilities(pos, ae::AbilitySet::default()),
            ),
        ))
        .id();
    app.insert_resource(ControlledSubject(Some(entity)));
    entity
}

/// A SECOND driven body holding `held_item_id` — one that nobody possesses.
///
/// ⭐⭐ THE POPULATION AN ABILITY ACTS ON IS `DrivenBodies`, NOT ONE SUBJECT.
/// `ControlledSubject` is singular by construction, so a couch's second seat and
/// a possessed body are invisible to it. This spawns the other half of that
/// union: a body carrying [`DrivingParticipant`] and a stable `SimId` (the
/// rewind-reproducible order `DrivenBodies` sorts by), with no `PrimaryPlayer`
/// and no claim on the controlled slot.
///
/// The caller inserts `ControlledSubject(None)` itself — leaving the resource out
/// makes `DrivenBodies` panic, and an ability test that panics on a missing
/// resource is not measuring the ability.
pub fn spawn_seated_body_holding(
    app: &mut App,
    held_item_id: &str,
    slot: u8,
    sim: &str,
    pos: ae::Vec2,
) -> Entity {
    let spec = held_item_by_id(held_item_id).unwrap();
    app.world_mut()
        .spawn((
            BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            ActorControl::default(),
            ActionSet::default(),
            HeldItem::new(spec),
            ambition_characters::control::DrivingParticipant(
                ambition_characters::control::PlayerSlot(slot),
            ),
            ambition_platformer2d_shared_tangle::sim_id::SimId::placement(sim),
            ambition_platformer2d_core::movement::MotionModel::default(),
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ae::BodyClusterScratch::new_with_abilities(pos, ae::AbilitySet::default()),
            ),
        ))
        .id()
}

/// The in-flight projectile bodies, oldest spawn first — production's ordering.
///
/// ⭐⭐ A COPY, DELIBERATELY, AND THE ALTERNATIVE WAS WORSE. The original is
/// `enemy_projectile::test_support::live_projectile_bodies` in the actor kernel,
/// and three tests here used it across what is now a crate line. Reaching back
/// would give this crate a dependency on the kernel — the exact edge the
/// abilities carve (D33, 2026-09-03) removed, and one no test fixture is worth
/// re-adding.
///
/// ⚠ IT COPIES CLEANLY BECAUSE IT NAMES NOTHING OF THE KERNEL'S: every type in
/// it is from a crate below both — `ambition_projectiles::{InFlightProjectile,
/// LiveProjectile, ProjectileBody, ProjectileSeq}`,
/// `ambition_platformer2d_core::BodyKinematics` and shared_tangle's
/// `ProjectileGameplay`. ⇒ If a third crate ever wants it, that is the signal to
/// move it DOWN into `ambition_projectiles` rather than copy it again.
///
/// Recomposes an `InFlightProjectile` from the entity's split `BodyKinematics` +
/// `ProjectileGameplay` so the historical collision assertions still read.
pub fn live_projectile_bodies(
    app: &mut bevy::app::App,
) -> Vec<ambition_projectiles::InFlightProjectile> {
    use ambition_platformer2d_shared_tangle::projectile::body::ProjectileGameplay;
    use ambition_projectiles::ProjectileSeq;
    use bevy::prelude::With;

    let world = app.world_mut();
    // `try_query_filtered` returns `Err` when the projectile component types
    // were never registered in this World — exactly the "no projectile ever
    // spawned" case some historical collision fixtures assert. Treat that as an
    // empty set rather than panicking.
    let Some(mut q) = world.try_query_filtered::<(
        &ambition_platformer2d_core::BodyKinematics,
        &ProjectileGameplay,
        &ProjectileSeq,
    ), With<ambition_projectiles::LiveProjectile>>() else {
        return Vec::new();
    };
    let mut rows: Vec<(ProjectileSeq, ambition_projectiles::InFlightProjectile)> = q
        .iter(world)
        .map(|(kin, game, seq)| {
            (
                *seq,
                ambition_projectiles::InFlightProjectile {
                    body: ambition_projectiles::ProjectileBody::from_parts(*kin, *game),
                },
            )
        })
        .collect();
    rows.sort_by_key(|(seq, _)| *seq);
    rows.into_iter().map(|(_, body)| body).collect()
}
