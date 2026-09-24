//! Test-only fixtures for ability modules.
//!
//! Most ability unit tests need the same minimal primary-player entity: a
//! body, action set, held item, and mana. This keeps that spawn in one place.

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
            // the full movement clusters; both are in
            // `AncillaryMovementBundle`, as in production spawns.
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ae::BodyClusterScratch::new_with_abilities(
                    ae::Vec2::new(100.0, 100.0),
                    ae::AbilitySet::default(),
                ),
            ),
            // Identity and mint stream, as on every production body
            // (`ensure_sim_id` runs before `CoreSimulation`). Abilities that
            // spawn a dynamic entity mint `SimId::spawned(body, counter.next())`,
            // and ADR 0030 refuses a caster without them.
            ambition_platformer2d_shared_tangle::sim_id::SimId::placement(
                "test_primary_player",
            ),
            ambition_platformer2d_shared_tangle::sim_id::SimIdCounter::default(),
            // The pool the Ambition home body is built holding.
            crate::mana::bank(),
        ))
        .id();
    // Ability systems key on the controlled subject; in tests the spawned
    // player is the controlled body.
    app.insert_resource(ControlledSubject(Some(entity)));
    entity
}

/// Set `body`'s Mana level; the body must hold Mana.
pub fn set_mana(app: &mut App, body: Entity, current: f32) {
    let mut bank = app
        .world_mut()
        .get_mut::<ambition_platformer2d_core::resources::ActorResources>(body)
        .expect("the fixture body holds a bank");
    bank.level_of_mut(&crate::mana::MANA)
        .expect("the fixture body holds Mana")
        .current = current;
}

/// `body`'s current Mana; the body must hold Mana.
pub fn mana(app: &App, body: Entity) -> f32 {
    crate::mana::level(app.world().get(body))
        .expect("the fixture body holds Mana")
        .current
}

/// A primary player holding `held_item_id` at an explicit `pos` / `facing`,
/// with no Mana: the minimal bundle for traversal-ability tests (blink,
/// grapple, mark-recall). One definition, so the bundle cannot drift.
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
            // the full movement clusters; both are in
            // `AncillaryMovementBundle`, as in production spawns.
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ae::BodyClusterScratch::new_with_abilities(pos, ae::AbilitySet::default()),
            ),
        ))
        .id();
    app.insert_resource(ControlledSubject(Some(entity)));
    entity
}

/// A second driven body holding `held_item_id`, not possessed by anyone.
///
/// Abilities act on `DrivenBodies`, not one subject: `ControlledSubject` is
/// singular, so a couch's second seat and a possessed body are not in it.
/// This body has a [`DrivingParticipant`] and a stable `SimId` (the order
/// `DrivenBodies` sorts by), no `PrimaryPlayer`, and no controlled slot.
///
/// The caller inserts `ControlledSubject(None)`; without that resource
/// `DrivenBodies` panics.
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
            // Its own pool: these fixtures test who fires, not who was
            // declared Mana.
            crate::mana::bank(),
        ))
        .id()
}

/// The in-flight projectile bodies, oldest spawn first — production's ordering.
///
/// A copy of the kernel's
/// `enemy_projectile::test_support::live_projectile_bodies`, so this crate
/// does not depend on the kernel (the edge the D33 split removed). It uses
/// only types from crates below both (`ambition_projectiles`,
/// `ambition_platformer2d_core::BodyKinematics`, shared_tangle's
/// `ProjectileGameplay`). If a third crate needs it, move it down into
/// `ambition_projectiles` instead of copying again.
///
/// Rebuilds an `InFlightProjectile` from the entity's `BodyKinematics` and
/// `ProjectileGameplay` for the collision assertions.
pub fn live_projectile_bodies(
    app: &mut bevy::app::App,
) -> Vec<ambition_projectiles::InFlightProjectile> {
    use ambition_platformer2d_shared_tangle::projectile::body::ProjectileGameplay;
    use ambition_projectiles::ProjectileSeq;
    use bevy::prelude::With;

    let world = app.world_mut();
    // `try_query_filtered` returns `Err` when the projectile types were never
    // registered in this World (no projectile ever spawned). Treat that as an
    // empty set.
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
