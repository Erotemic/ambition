
use super::{
    scrollbar_drag, scrollbar_drag_start, scrollbar_press, scrollbar_press_drag, scrollbar_release,
    MenuScrollDragged, MenuScrollbar,
};
use bevy::camera::NormalizedRenderTarget;
use bevy::picking::events::{Drag, DragStart, Pointer, Press, Release};
use bevy::picking::pointer::{Location, PointerButton, PointerId, PointerLocation};
use bevy::prelude::*;

fn location(y: f32) -> Location {
    Location {
        target: NormalizedRenderTarget::None {
            width: 1,
            height: 1,
        },
        position: Vec2::new(0.0, y),
    }
}

/// Feature C: a synthetic `Pointer<DragStart>` + `Pointer<Drag>` on the scrollbar
/// emits the neutral `MenuScrollDragged` fraction proportional to the pointer's
/// vertical position within the track (0 = top, 1 = bottom). Drives the real lib
/// observers; track geometry is set directly (no camera projection needed).
#[test]
fn drag_on_scrollbar_emits_proportional_fraction() {
    let mut app = App::new();
    app.add_message::<MenuScrollDragged>();
    app.add_observer(scrollbar_drag_start);
    app.add_observer(scrollbar_drag);

    // Track spans screen y in [100, 300] (top 100, height 200).
    let bar = app
        .world_mut()
        .spawn(MenuScrollbar {
            track_top_y: 100.0,
            track_height: 200.0,
            ..Default::default()
        })
        .id();

    // DragStart at the very top of the track -> fraction 0.
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        location(100.0),
        DragStart {
            button: PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(bar, 0.0, None, None),
        },
        bar,
    ));
    // Drag to the middle of the track -> fraction 0.5.
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        location(200.0),
        Drag {
            button: PointerButton::Primary,
            distance: Vec2::new(0.0, 100.0),
            delta: Vec2::new(0.0, 100.0),
        },
        bar,
    ));
    app.update();

    let world = app.world_mut();
    let mut reader = world.resource_mut::<Messages<MenuScrollDragged>>();
    let fractions: Vec<f32> = reader.drain().map(|m| m.fraction).collect();
    assert_eq!(fractions.len(), 2, "press + drag each emit one fraction");
    assert!(
        (fractions[0] - 0.0).abs() < 1e-4,
        "press at top = {}",
        fractions[0]
    );
    assert!(
        (fractions[1] - 0.5).abs() < 1e-4,
        "drag to mid = {}",
        fractions[1]
    );
}

/// Fix 1: the manual press+move tracker (the path the CUBE actually uses, since
/// `Pointer<Drag>` continuity doesn't reach through the custom 3D picking
/// backend). A `Pointer<Press>` marks the track held; while held, the live
/// pointer position emits a proportional `MenuScrollDragged` each frame; a
/// `Pointer<Release>` ends it. Drives the real lib observers + system.
#[test]
fn press_and_move_on_scrollbar_emits_proportional_fraction() {
    let mut app = App::new();
    app.add_message::<MenuScrollDragged>();
    app.add_observer(scrollbar_press);
    app.add_observer(scrollbar_release);
    app.add_systems(Update, scrollbar_press_drag);

    // The pointer whose live position the tracker reads each frame.
    let pointer = app
        .world_mut()
        .spawn((PointerId::Mouse, PointerLocation::new(location(100.0))))
        .id();

    // Track spans screen y in [100, 300] (top 100, height 200).
    let bar = app
        .world_mut()
        .spawn(MenuScrollbar {
            track_top_y: 100.0,
            track_height: 200.0,
            ..Default::default()
        })
        .id();

    let drain = |app: &mut App| -> Vec<f32> {
        app.world_mut()
            .resource_mut::<Messages<MenuScrollDragged>>()
            .drain()
            .map(|m| m.fraction)
            .collect()
    };

    // Press at the top of the track -> the press observer emits fraction 0 and
    // marks the track held.
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        location(100.0),
        Press {
            button: PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(bar, 0.0, None, None),
        },
        bar,
    ));
    let press = drain(&mut app);
    assert_eq!(press.len(), 1, "press emits exactly one fraction");
    assert!((press[0] - 0.0).abs() < 1e-4, "press at top = {}", press[0]);
    assert_eq!(
        app.world().get::<MenuScrollbar>(bar).unwrap().pressed_by,
        Some(PointerId::Mouse),
        "press marks the track held"
    );

    // Move the live pointer to the middle of the track; the manual tracker emits
    // fraction 0.5 each frame while held (no `Pointer<Drag>` needed).
    *app.world_mut().get_mut::<PointerLocation>(pointer).unwrap() =
        PointerLocation::new(location(200.0));
    let _ = drain(&mut app); // clear the press message before the tracked frame
    app.update();
    let tracked = drain(&mut app);
    assert!(
        !tracked.is_empty(),
        "the held tracker emits while pressed: {tracked:?}"
    );
    assert!(
        (tracked.last().unwrap() - 0.5).abs() < 1e-4,
        "tracked move to mid = {}",
        tracked.last().unwrap()
    );

    // Release ends the held state.
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        location(300.0),
        Release {
            button: PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(bar, 0.0, None, None),
        },
        bar,
    ));
    assert_eq!(
        app.world().get::<MenuScrollbar>(bar).unwrap().pressed_by,
        None,
        "release clears the held pointer"
    );

    // Move again after release -> the tracker must NOT emit.
    *app.world_mut().get_mut::<PointerLocation>(pointer).unwrap() =
        PointerLocation::new(location(150.0));
    let _ = drain(&mut app);
    app.update();
    let after_release = drain(&mut app);
    assert!(
        after_release.is_empty(),
        "no fractions after release: {after_release:?}"
    );
}
