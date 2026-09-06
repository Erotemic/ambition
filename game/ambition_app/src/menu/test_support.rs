//! Shared menu pointer fixtures used by backend tests.
//!
//! ⛔⛔ **THESE HELPERS BYPASS HIT-TESTING, AND EVERY MENU POINTER TEST IN THIS
//! REPO IS BUILT ON THEM.** `trigger_press` / `trigger_release` construct a
//! `Pointer<Press>` with a `HitData` naming the target entity and trigger it
//! DIRECTLY on that entity. Nothing asks whether a pointer at those coordinates
//! would actually have hit it.
//!
//! ⇒ So a green menu-pointer suite certifies **"the observer does the right
//! thing when it fires"** and says NOTHING about **"a click at these pixels
//! reaches this entity."** A button that is covered by another node, outside its
//! camera's render target, sized to zero, or opted out of picking passes every
//! test here.
//!
//! ⚠ **AND IT IS NOT LAZINESS — the fixture CANNOT do better.** Measured
//! 2026-09-06: the shipped rendered app fixture settles with **no window at
//! all** (`window=None`), and hit-testing needs a render target. Direct
//! triggering is the only thing available, which is exactly why the gap is
//! invisible: there is no version of these tests that would have exposed it.
//!
//! ⭐ **WHAT THAT MEANS FOR A BUG REPORT.** When a control is reported as
//! unclickable and every test is green, the tests have not cleared the control —
//! they have not looked. Ask for the one observation this fixture cannot make:
//! **does the control respond to HOVER on a real window?** Hover proves picking
//! reaches it, which splits "downstream of `Interaction`" (testable here) from
//! "upstream in picking" (not testable here at all). See Q70 in
//! `docs/planning/awaiting-maintainer-decision.md`.

use bevy::camera::NormalizedRenderTarget;
use bevy::picking::backend::HitData;
use bevy::picking::events::{Move, Over, Pointer, Press, Release};
use bevy::picking::pointer::{Location, PointerButton, PointerId};
use bevy::prelude::*;

use ambition_platformer2d::menu::{AmbitionMenuControl, MenuControlKind, MenuFocusKey};

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
        .spawn((
            Button,
            AmbitionMenuControl::<MenuPageAction> {
                kind: MenuControlKind::OptionToggle,
                action: Some(action),
                focus: MenuFocusKey::default(),
            },
        ))
        .id()
}

pub(crate) fn trigger_press(app: &mut App, entity: Entity) {
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        pointer_location(),
        Press {
            button: PointerButton::Primary,
            hit: HitData::new(entity, 0.0, None, None),
            // bevy 0.19: consecutive-press counter; a single synthetic press is 1.
            count: 1,
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
