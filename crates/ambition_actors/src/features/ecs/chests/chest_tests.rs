//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod chest_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

//! The player -> static-chest open path as a minimal-App harness:
//! a buffered interact over an overlapping, unopened chest inserts
//! `Opened`; an unbuffered player or a non-overlapping chest does not.
use super::*;
use crate::actor::BodyAnimFacts;
use crate::control::SlotInteractionState;
use ambition_engine_core::BodyBaseSize;
use ambition_engine_core::BodyKinematics;
use ambition_platformer_primitives::markers::ControlledSubject;
use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};
use bevy::prelude::{App, Entity, Update};

fn app() -> App {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.init_resource::<SlotInteractionState>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_systems(Update, open_ecs_chests);
    app
}

fn player(app: &mut App, pos: ae::Vec2, buffered: bool) -> Entity {
    // The buffered interact is SLOT state now, not a per-body component.
    if buffered {
        app.world_mut()
            .resource_mut::<SlotInteractionState>()
            .primary_mut()
            .interact_buffer_timer = 0.5;
    }
    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos,
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
                ..Default::default()
            },
            BodyBaseSize {
                base_size: ae::Vec2::new(28.0, 46.0),
            },
            BodyAnimFacts::default(),
        ))
        .id();
    app.world_mut()
        .insert_resource(ControlledSubject(Some(entity)));
    entity
}

fn chest(app: &mut App, id: &str, pos: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new(id),
            FeatureName::new("Chest"),
            CenteredAabb::from_center_size(pos, ae::Vec2::new(24.0, 24.0)),
            ChestFeature::new(ambition_interaction::Chest::new(id, None)),
        ))
        .id()
}

#[test]
fn buffered_interact_opens_an_overlapping_chest() {
    let mut app = app();
    let center = ae::Vec2::new(64.0, 64.0);
    player(&mut app, center, true);
    let c = chest(&mut app, "c1", center);
    app.update();
    assert!(
        app.world().get::<Opened>(c).is_some(),
        "buffered interact over the chest opens it"
    );
}

#[test]
fn unbuffered_player_leaves_chest_closed() {
    let mut app = app();
    let center = ae::Vec2::new(64.0, 64.0);
    player(&mut app, center, false);
    let c = chest(&mut app, "c1", center);
    app.update();
    assert!(
        app.world().get::<Opened>(c).is_none(),
        "no buffered interact -> chest stays closed"
    );
}

#[test]
fn distant_chest_is_not_opened() {
    let mut app = app();
    player(&mut app, ae::Vec2::new(64.0, 64.0), true);
    let c = chest(&mut app, "c1", ae::Vec2::new(2000.0, 2000.0));
    app.update();
    assert!(
        app.world().get::<Opened>(c).is_none(),
        "a non-overlapping chest stays closed even with a buffered interact"
    );
}
