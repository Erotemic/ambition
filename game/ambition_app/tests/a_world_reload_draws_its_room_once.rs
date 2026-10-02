//! A developer world reload draws the rebuilt room once (view half V4).
//!
//! The reload publishes the rebuilt room as the session's next live room. Each
//! live room's visuals (V4a), parallax (V4c) and LDtk levels (V4b) are given to
//! it by their own reconcilers, so a reload that also spawned them itself drew
//! the room twice.

use bevy::prelude::{App, With, Without};

/// The room's drawn population: static visuals, their per-room markers, the
/// parallax roots, and the LDtk bundles.
fn drawn(app: &mut App) -> (usize, usize, usize, usize) {
    let world = app.world_mut();
    let visuals = world
        .query_filtered::<(), With<ambition_platformer2d::platformer::lifecycle::RoomVisual>>()
        .iter(world)
        .count();
    let markers = world
        .query_filtered::<(), With<ambition_platformer2d::render::rendering::PresentedRoomVisuals>>()
        .iter(world)
        .count();
    let parallax = world
        .query_filtered::<(), (
            With<ambition_platformer2d::render::rendering::ParallaxLayerVisual>,
            Without<ambition_platformer2d::render::rendering::MirroredParallaxLayer>,
        )>()
        .iter(world)
        .count();
    let ldtk = world
        .query_filtered::<(), With<ambition_platformer2d::ldtk_map::LdtkWorldRoot>>()
        .iter(world)
        .count();
    (visuals, markers, parallax, ldtk)
}

/// An equivalent reload (the same project read again) rebuilds the same room,
/// so it draws exactly what was drawn before it. The control is the session
/// before the reload: one marker, and a room that is drawn.
#[test]
fn an_equivalent_world_reload_draws_its_room_once() {
    let mut app = crate::an_edit_reaches_the_shipped_game::a_running_shipped_session();
    for _ in 0..10 {
        app.update();
    }
    let before = drawn(&mut app);
    assert!(before.0 > 0 && before.1 == 1, "control: the running room is drawn once: {before:?}");

    let applied = app
        .world()
        .resource::<ambition_platformer2d::dev_tools::WorldSourceHotReload>()
        .applied_count;
    crate::an_edit_reaches_the_shipped_game::press_apply_reload(&mut app);
    assert!(
        app.world()
            .resource::<ambition_platformer2d::dev_tools::WorldSourceHotReload>()
            .applied_count
            > applied,
        "setup: the reload did not apply"
    );
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(drawn(&mut app), before, "the reloaded room is not drawn exactly as before (visuals, markers, parallax, LDtk)");
}
