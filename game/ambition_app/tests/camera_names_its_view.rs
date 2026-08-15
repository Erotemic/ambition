//! **A CAMERA SAYS WHICH VIEW IT PRESENTS.**
//!
//! `camera_follow` used to read the view as `Single<…, With<LocalView>>` beside a
//! query for the main camera, pairing them by the coincidence that there is one
//! of each. That is not a pairing — it is a uniqueness assumption at both ends,
//! and D116 M2 exists because the second view is coming: a second view turns the
//! `Single` into a panic, and a second camera turns the pair into two cameras
//! fighting over one snapshot.
//!
//! ⚠ **the fallback is why this test has to exist.** A camera carrying no
//! `PresentsView` takes the only view, because every fixture in the tree spawns
//! a bare `MainCamera` and a single-view composition has exactly one honest
//! answer. That fallback is also what would hide the binding being dropped: the
//! game would keep working, and the link would quietly have no producer until
//! the day a split layout landed and picked a view at random. So the thing
//! asserted here is that the SHIPPED host binds it.

use bevy::prelude::*;

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::platformer::camera_layers::MainCamera;
use ambition_platformer2d::sim_view::{LocalView, PresentsView};

#[test]
fn the_shipped_hosts_main_camera_names_the_view_it_presents() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    for _ in 0..ambition_app::app::shared_host_startup_ticks() {
        app.update();
    }

    let presented = {
        let world = app.world_mut();
        let mut cameras = world.query_filtered::<&PresentsView, With<MainCamera>>();
        cameras.iter(world).next().copied()
    };
    let Some(PresentsView(view)) = presented else {
        panic!(
            "the host's main camera carries no `PresentsView`, so nothing states \
             which view it presents and `camera_follow` is back to assuming there \
             is exactly one of each"
        );
    };

    // ⛔ and it must name a REAL view. An `Entity` is just a number; a link to a
    // despawned or wrong entity reads as a valid binding and resolves to nothing.
    let names_a_view = {
        let world = app.world_mut();
        let mut views = world.query_filtered::<Entity, With<LocalView>>();
        views.iter(world).any(|candidate| candidate == view)
    };
    assert!(
        names_a_view,
        "the main camera presents {view:?}, which is not a local view"
    );
}

/// **THE VIEW OWNS ITS FRAMING, AND SOMETHING WRITES IT.**
///
/// `CameraViewState` was a process-global `Resource` describing *"the gameplay
/// view"* — the sixth of that shape, after D116 M2a deleted five. Five readers
/// took it as `Res`: the foreground, label layout, nameplates, actor draw, and
/// the debug overlay. With two views a global cannot answer *whose* framing it
/// is, so all five would have drawn one view's framing over both.
///
/// ⚠ **the non-vacuity half is the point.** Asserting the component EXISTS proves
/// nothing on its own: it is spawned with the view, so it would be present even
/// if `camera_follow` never wrote it and every overlay drew a default frame
/// forever. What is asserted is that the presented view's state has been
/// UPDATED — it carries the room's real framing rather than the `Default` a
/// freshly spawned view starts with.
#[test]
fn the_presented_view_carries_camera_state_that_is_actually_written() {
    // ⚠ **the DIRECT-GAMEPLAY persona, not the launcher.** `camera_follow` reads
    // the session's `RoomGeometry`, so on a launcher route it does not run at all
    // and the view's state stays exactly `Default` — which the assertion below
    // caught on the first run, correctly, by refusing to accept a component that
    // exists and is never written.
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, false);
    for _ in 0..ambition_app::app::shared_host_startup_ticks() {
        app.update();
    }

    let view = {
        let world = app.world_mut();
        let mut cameras = world.query_filtered::<&PresentsView, With<MainCamera>>();
        cameras
            .iter(world)
            .next()
            .copied()
            .map(|PresentsView(view)| view)
            .expect("the shipped host binds its camera to a view")
    };

    let state = app
        .world()
        .entity(view)
        .get::<ambition_platformer2d::sim_view::CameraViewState>()
        .expect(
            "the presented view carries no `CameraViewState`, so every overlay that \
             frames against it has nothing to read — which is the blank-framing shape \
             the global at least could not have",
        )
        .clone();

    let fresh = ambition_platformer2d::sim_view::CameraViewState::default();
    assert!(
        state.visible_view != fresh.visible_view || state.center_world != fresh.center_world,
        "the presented view's camera state is still exactly `Default` after a full \
         startup ({state:?}). The component exists and nothing writes it, so the \
         overlays are framing against a view that never moved — an assertion that it \
         merely EXISTS would have passed here."
    );
}

/// **A SINGLETON THAT MOVED FROM A RESOURCE INTO A LOOKUP IS STILL A SINGLETON.**
///
/// Moving `CameraViewState` onto the view deleted the sixth process-global "the
/// gameplay view" — and the resolver that replaced it opened with
/// `cameras.iter().next()`. That is fine while one main camera exists and is
/// exactly wrong the moment split-screen lands: two cameras each correctly
/// naming a DIFFERENT view do not produce two views, they produce whichever
/// camera the archetype iteration happened to yield first, handed to all five
/// presentation consumers. Same defect as the resource, one indirection deeper,
/// and it would only surface the day the second view arrived (GPT 5.6,
/// 2026-08-15).
///
/// So the interim contract is loudly single-view-only rather than quietly
/// arbitrary: with several main cameras the resolver REFUSES. The consumers are
/// not view-keyed yet — that is the rest of D116 M2 — and until they are, `None`
/// with a reason beats a confidently wrong frame.
///
/// ⚠ **this is a poison, and it is built to be one.** The two views carry
/// deliberately different states, so a resolver that picks either one returns a
/// value that IS one of them and looks entirely healthy. Only the refusal
/// distinguishes "keyed by view" from "picked one".
#[test]
fn the_presented_view_refuses_to_pick_between_two_cameras_that_name_different_views() {
    use ambition_platformer2d::sim_view::{CameraViewState, PresentedViewState};
    use bevy::ecs::system::RunSystemOnce as _;

    let mut world = World::new();

    let framed = |width: f32| CameraViewState {
        visible_view: ambition_platformer2d::engine_core::Vec2::new(width, width * 0.5),
        ..default()
    };
    let left_view = world.spawn((LocalView, framed(640.0))).id();
    let right_view = world.spawn((LocalView, framed(320.0))).id();
    world.spawn((MainCamera, PresentsView(left_view)));
    world.spawn((MainCamera, PresentsView(right_view)));

    let resolved = world
        .run_system_once(|presented: PresentedViewState| presented.get().cloned())
        .expect("the resolver system should run");

    assert!(
        resolved.is_none(),
        "two main cameras name two different views and the resolver answered with \
         one of them ({resolved:?}). Every consumer of `PresentedViewState` would \
         then draw whichever view iterated first over BOTH — the process-global \
         'the gameplay view' restored as a lookup. It must refuse until the \
         consumers are keyed by view."
    );

    // ⭐ **and the refusal must be about AMBIGUITY, not about the resolver being
    // broken.** Drop one camera and the same population resolves cleanly to the
    // view that camera names — so the `None` above is a decision, not a failure.
    let sole_camera = {
        let mut cameras = world.query_filtered::<Entity, With<MainCamera>>();
        let entity = cameras.iter(&world).next().expect("two were spawned");
        entity
    };
    let named = world
        .entity(sole_camera)
        .get::<PresentsView>()
        .copied()
        .expect("spawned with a link");
    let others: Vec<Entity> = {
        let mut cameras = world.query_filtered::<Entity, With<MainCamera>>();
        cameras
            .iter(&world)
            .filter(|candidate| *candidate != sole_camera)
            .collect()
    };
    for other in others {
        world.despawn(other);
    }

    let resolved = world
        .run_system_once(|presented: PresentedViewState| presented.get().cloned())
        .expect("the resolver system should run");
    let expected = if named.0 == left_view { 640.0 } else { 320.0 };
    assert_eq!(
        resolved.map(|state| state.visible_view.x),
        Some(expected),
        "one camera naming one view is unambiguous and must resolve to THAT view's \
         state; if this fails the refusal above is just a broken resolver"
    );
}
