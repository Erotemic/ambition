//! Test-only fixtures for ability modules.
//!
//! Ability unit tests mostly need the same minimal primary-player entity: a body,
//! action set, held item, and mana. Keeping that bundle here lets each ability
//! test focus on the behavior it is asserting instead of repeating spawn wiring.

use crate::actor::BodyKinematics;
use crate::actor::{BodyBaseSize, BodyMana};
use crate::actor::{PlayerEntity, PrimaryPlayer};
use crate::control::PlayerInputFrame;
use crate::features::HeldItem;
use ambition_characters::brain::{held_item_by_id, ActionSet, ActorControl};
use ambition_engine_core as ae;
use ambition_platformer_primitives::markers::ControlledSubject;
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
            BodyBaseSize {
                base_size: ae::Vec2::new(24.0, 40.0),
            },
            PlayerInputFrame::default(),
            ActorControl::default(),
            ActionSet::default(),
            HeldItem::new(spec),
            BodyMana::default(),
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
            BodyBaseSize {
                base_size: ae::Vec2::new(24.0, 40.0),
            },
            PlayerInputFrame::default(),
            ActorControl::default(),
            ActionSet::default(),
            HeldItem::new(spec),
            crate::features::MotionModel::default(),
        ))
        .id();
    app.insert_resource(ControlledSubject(Some(entity)));
    entity
}
