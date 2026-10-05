//! The shipped game's window cameras layer into one main texture.
//!
//! A camera that does not clear (the HUD, the cube menu) draws over the
//! cameras before it only if Bevy gives them the same main texture: the same
//! usages, format and MSAA. On 2026-10-05 a gameplay camera put in
//! `CompositingSpace::Srgb` got a texture of its own, and every game was a
//! black stage under ghosting menus on Jon's GPU host
//! (`ambition_render::rendering::window_camera_stack`).

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::render::rendering::window_camera_stack::{incompatible_window_stacks, window_cameras};

fn gameplay_after_startup() -> bevy::prelude::App {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, false);
    for _ in 0..ambition_app::app::shared_host_startup_ticks() {
        app.update();
    }
    app
}

#[test]
fn the_shipped_window_cameras_share_one_main_texture() {
    let mut app = gameplay_after_startup();
    let cameras = window_cameras(app.world_mut());
    // ⛔ Premise: a stack to check — a clearing camera and one that draws over it.
    assert!(
        cameras.iter().any(|camera| camera.clears) && cameras.iter().any(|camera| !camera.clears),
        "no layered window stack to check: {cameras:#?}"
    );
    let found = incompatible_window_stacks(cameras.clone());
    assert!(found.is_empty(), "{}", found.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"));

    // The control: the gameplay camera in gamma space is reported, by name.
    let main = {
        let world = app.world_mut();
        world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::camera_layers::MainCamera>>()
            .iter(world)
            .next()
            .expect("a gameplay camera")
    };
    app.world_mut().entity_mut(main).insert(bevy::camera::CompositingSpace::Srgb);
    let found = incompatible_window_stacks(window_cameras(app.world_mut()));
    assert!(
        found.iter().any(|incompatibility| incompatibility.below.name.contains(&format!("{main}"))),
        "a gamma gameplay camera under the HUD was not reported: {found:#?}"
    );
}
