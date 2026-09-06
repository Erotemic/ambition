//! Unit tests for map state: zoom clamping and the short-room-label helper.

use super::*;
use super::{MapMenuState, MAP_ZOOM_MAX, MAP_ZOOM_MIN};

#[test]
fn map_zoom_in_clamps_to_max() {
    let mut map = MapMenuState::default();
    for _ in 0..20 {
        map.zoom_in();
    }
    assert!(map.zoom <= MAP_ZOOM_MAX + 1e-4);
    assert!(map.zoom > 1.0);
}

#[test]
fn map_zoom_out_clamps_to_min() {
    let mut map = MapMenuState::default();
    for _ in 0..20 {
        map.zoom_out();
    }
    assert!(map.zoom >= MAP_ZOOM_MIN - 1e-4);
    assert!(map.zoom < 1.0);
}

#[test]
fn map_zoom_reset_returns_to_one() {
    let mut map = MapMenuState::default();
    map.zoom_in();
    map.zoom_in();
    map.zoom_reset();
    assert_eq!(map.zoom, 1.0);
}

#[test]
fn map_zoom_step_is_round_trip_friendly() {
    let mut map = MapMenuState::default();
    let initial = map.zoom;
    map.zoom_in();
    let zoomed = map.zoom;
    map.zoom_out();
    assert!(
        (map.zoom - initial).abs() < 1e-3,
        "zoom_in then zoom_out should return near 1.0 (got {} from {})",
        map.zoom,
        zoomed
    );
}

#[test]
fn short_room_label_initializes_underscore_id() {
    assert_eq!(short_room_label("central_hub_complex"), "CHC");
    assert_eq!(short_room_label("water_world"), "WW");
    assert_eq!(short_room_label("goblin_encounter"), "GE");
}

#[test]
fn short_room_label_uppercase_truncates_single_word() {
    assert_eq!(short_room_label("alpha"), "ALPHA");
    assert_eq!(short_room_label("verylongname"), "VERYLONG");
}

/// ⛔⛔ THE PLUGIN INSTALLS THE SYSTEMS IT OWNS, and this exists because the
/// carve that moved them here could have LOST them silently.
///
/// `handle_map_menu_hotkeys` and `map_menu_pointer_dismiss` used to be registered
/// by `ambition_app`'s plugin list, which named two functions in this crate. They
/// now come from `MapStatePlugin` — but nothing else in the tree fails if they
/// simply stop being added: there is no app-level map-menu test, so the map would
/// go quietly dead and the suite would stay green.
///
/// ⚠ IT ASSERTS THE SCHEDULE, NOT A BEHAVIOUR, deliberately. What these systems DO
/// is covered by the tests around this one; what no other test can see is whether
/// anybody still runs them.
#[test]
fn the_plugin_installs_the_map_menu_systems_it_owns() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(super::MapStatePlugin);
    // The schedule builds its graph on first run, so an uninitialized one reports
    // nothing — the same trap `boot_budget`'s duplicate check had to be taught.
    let mut update = app
        .world_mut()
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .remove(Update)
        .expect("the plugin added systems to Update");
    update
        .initialize(app.world_mut())
        .expect("the Update schedule initializes");
    // ⚠ BY COUNT, NOT BY NAME, and not by choice: Bevy strips system names unless
    // its `debug` feature is on, so every row here reads
    // "<Enable the debug feature to see the name>". A name-based assertion passes
    // vacuously in this build — it would find nothing and say nothing.
    let installed = update.systems().expect("initialized").count();
    assert_eq!(
        installed, 3,
        "`MapStatePlugin` installs {installed} Update system(s); it owns exactly \
         three — `handle_map_menu_hotkeys`, `map_menu_pointer_dismiss` and \
         `sync_map_menu`. ⇒ If this DROPPED, the map menu is dead in every \
         composition and nothing else in the tree says so (there is no app-level \
         map-menu test). If it GREW, raise this number deliberately and say what \
         joined them.\n\n\
         ⭐ It went 2 → 3 on 2026-09-06 when `sync_map_menu` was carved out of the \
         shell, and this assertion is what made that a decision instead of a drift."
    );
}
