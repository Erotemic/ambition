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
