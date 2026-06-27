//! Test-only fixtures for ability modules.
//!
//! Ability unit tests mostly need the same minimal primary-player entity: a body,
//! action set, held item, and mana. Keeping that bundle here lets each ability
//! test focus on the behavior it is asserting instead of repeating spawn wiring.

use ambition_characters::brain::{held_item_by_id, ActionSet};
use ambition_engine_core as ae;
use crate::features::HeldItem;
use crate::player::{BodyBaseSize, PlayerInputFrame, BodyMana};
use crate::actor::{PlayerEntity, PrimaryPlayer};
use crate::actor::BodyKinematics;
use bevy::prelude::*;

pub(crate) fn spawn_primary_player_holding(app: &mut App, held_item_id: &str) -> Entity {
    let spec = held_item_by_id(held_item_id).unwrap();
    app.world_mut()
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
            ActionSet::default(),
            HeldItem::new(spec),
            BodyMana::default(),
        ))
        .id()
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
    app.world_mut()
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
            ActionSet::default(),
            HeldItem::new(spec),
        ))
        .id()
}
