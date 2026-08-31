//! Test-only fixtures for ability modules.
//!
//! Ability unit tests mostly need the same minimal primary-player entity: a body,
//! action set, held item, and mana. Keeping that bundle here lets each ability
//! test focus on the behavior it is asserting instead of repeating spawn wiring.

use crate::features::HeldItem;
use ambition_characters::brain::{held_item_by_id, ActionSet};
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use bevy::prelude::*;

pub(crate) fn spawn_primary_player_holding(app: &mut App, held_item_id: &str) -> Entity {
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
pub(crate) fn spawn_primary_player_holding_at(
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
pub(crate) fn spawn_seated_body_holding(
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
