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
