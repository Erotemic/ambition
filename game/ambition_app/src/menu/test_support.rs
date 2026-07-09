//! Shared menu pointer fixtures used by backend tests.

use bevy::camera::NormalizedRenderTarget;
use bevy::picking::backend::HitData;
use bevy::picking::events::{Move, Over, Pointer, Press, Release};
use bevy::picking::pointer::{Location, PointerButton, PointerId};
use bevy::prelude::*;

use ambition::menu::{AmbitionMenuControl, MenuControlKind, MenuFocusKey};

use crate::menu::model::MenuPageAction;

pub(crate) fn pointer_location_at(position: Vec2) -> Location {
    Location {
        target: NormalizedRenderTarget::None {
            width: 1,
            height: 1,
        },
        position,
    }
}

pub(crate) fn pointer_location() -> Location {
    pointer_location_at(Vec2::ZERO)
}

pub(crate) fn spawn_control(app: &mut App, action: MenuPageAction) -> Entity {
    app.world_mut()
        .spawn(AmbitionMenuControl::<MenuPageAction> {
            kind: MenuControlKind::OptionToggle,
            action: Some(action),
            focus: MenuFocusKey::default(),
        })
        .id()
}

pub(crate) fn trigger_press(app: &mut App, entity: Entity) {
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        pointer_location(),
        Press {
            button: PointerButton::Primary,
            hit: HitData::new(entity, 0.0, None, None),
        },
        entity,
    ));
}

pub(crate) fn trigger_release(app: &mut App, entity: Entity) {
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        pointer_location(),
        Release {
            button: PointerButton::Primary,
            hit: HitData::new(entity, 0.0, None, None),
        },
        entity,
    ));
}

pub(crate) fn trigger_over(app: &mut App, entity: Entity) {
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        pointer_location(),
        Over {
            hit: HitData::new(entity, 0.0, None, None),
        },
        entity,
    ));
}

pub(crate) fn trigger_move(app: &mut App, entity: Entity, delta: Vec2) {
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        pointer_location(),
        Move {
            hit: HitData::new(entity, 0.0, None, None),
            delta,
        },
        entity,
    ));
}

pub(crate) fn click_control(app: &mut App, action: MenuPageAction) {
    let entity = spawn_control(app, action);
    trigger_press(app, entity);
    trigger_release(app, entity);
    app.update();
}
