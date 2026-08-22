//! **A DEMO MAY NOT ALSO WRITE THE VIEWPORT ITS HOST OWNS.**
//!
//! `apply_gameplay_camera_viewport` owns `Camera::viewport` for every `MainCamera` that presents a
//! `LocalView`, and TwinTrack's own pane cameras ARE such cameras — `spawn_pane_camera` gives each
//! one `MainCamera` and a `PresentsView` link.
//!
//! **why this test lives HERE and not in either obvious place.** TwinTrack's
//! own integration suite is headless: with no `PrimaryWindow` the applier
//! returns before it reads anything, and with no `visible` feature the pane
//! cameras are never spawned at all. The host crate's presentation tests have
//! the window and the cameras and cannot name a demo. `ambition_app` is the one
//! composition that holds both — it links `ambition_demo_twintrack/visible` and
//! builds the windowed host — so this is where a COMPOSITION adding a second
//! writer becomes visible.
//!
//! **the generic owner is already guarded**
//! (`each_camera_renders_into_the_rectangle_of_the_view_it_names`). What had no
//! test, and what this is, is a composition fighting it.

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId, ShellRouter};
use ambition_platformer2d::platformer::camera_layers::MainCamera;

const DISPLAY: Vec2 = Vec2::new(1600.0, 900.0);

/// Every main camera's physical rectangle, in a stable order.
fn physical_viewports(app: &mut App) -> Vec<(UVec2, UVec2)> {
    let world = app.world_mut();
    let mut cameras = world.query_filtered::<&Camera, With<MainCamera>>();
    let mut rects: Vec<(UVec2, UVec2)> = cameras
        .iter(world)
        .filter_map(|camera| camera.viewport.as_ref())
        .map(|viewport| (viewport.physical_position, viewport.physical_size))
        .collect();
    rects.sort_by_key(|(origin, _)| (origin.x, origin.y));
    rects
}

#[test]
fn twintracks_two_panes_keep_two_distinct_viewports() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    // `NoWindow` composes the render graph without a surface, so the display
    // fact the layout resolves against has to be stated. Without one
    // `apply_gameplay_camera_viewport` returns before reading anything and every
    // assertion below would be about a system that never ran.
    let mut resolution = WindowResolution::new(DISPLAY.x as u32, DISPLAY.y as u32);
    resolution.set_scale_factor(1.0);
    resolution.set(DISPLAY.x, DISPLAY.y);
    app.world_mut().spawn((
        Window {
            resolution,
            ..default()
        },
        PrimaryWindow,
    ));
    for _ in 0..ambition_app::app::shared_host_startup_ticks() {
        app.update();
    }
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_twintrack::TWINTRACK_GAMEPLAY_ROUTE,
        )));
    // The plaza builds its bodies, its views and its pane cameras over several
    // frames; the applier then needs a frame with the views on hand.
    let mut arrived = false;
    for _ in 0..240 {
        app.update();
        if app
            .world()
            .resource::<ShellRouter>()
            .active
            .as_ref()
            .is_some_and(|active| {
                active.route_id.as_str() == ambition_demo_twintrack::TWINTRACK_GAMEPLAY_ROUTE
            })
        {
            arrived = true;
        }
        if arrived && physical_viewports(&mut app).len() >= 2 {
            break;
        }
    }
    assert!(arrived, "the shell never reached the TwinTrack route");

    let rects = physical_viewports(&mut app);
    assert_eq!(
        rects.len(),
        2,
        "TwinTrack seats two participants and draws two panes, so two main \
         cameras must carry a viewport; found {rects:?}. A viewport of `None` is \
         a camera drawing full-bleed over the other pane, which is exactly what \
         a second writer clearing it produces",
    );

    // **and they must be DIFFERENT rectangles that do not overlap.** Two
    // cameras both left full-screen would also be "two viewports" in a weaker
    // assertion, and that is the failure being guarded.
    let (left_origin, left_size) = rects[0];
    let (right_origin, _right_size) = rects[1];
    assert!(
        left_origin.x + left_size.x <= right_origin.x,
        "the two panes overlap: {rects:?}",
    );
}
