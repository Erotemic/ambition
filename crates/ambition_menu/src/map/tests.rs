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

/// The map menu installer registers its systems. No app-level map-menu test
/// exists, so without this the map could stop running and the suite would
/// stay green. It checks the schedule, not behavior; other tests cover what
/// the systems do.
#[test]
fn installing_the_map_menu_adds_the_systems_it_owns() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(super::MapStatePlugin);
    // The install is a separate call from the plugin (the plugin declares
    // vocabulary only), so test the installer.
    super::install_map_menu_systems(&mut app);
    // The schedule builds its graph on first run; an uninitialized schedule
    // reports nothing.
    let mut update = app
        .world_mut()
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .remove(Update)
        .expect("the plugin added systems to Update");
    update
        .initialize(app.world_mut())
        .expect("the Update schedule initializes");
    // Counted, not named: Bevy strips system names without its `debug`
    // feature, so a name check would pass vacuously. Count Startup too, so
    // `populate_map_rooms` is covered; a count guard only sees the schedule it
    // names.
    #[cfg(feature = "ldtk")]
    {
        let mut startup = app
            .world_mut()
            .resource_mut::<bevy::ecs::schedule::Schedules>()
            .remove(bevy::prelude::Startup)
            .expect("the install added a Startup system");
        startup
            .initialize(app.world_mut())
            .expect("the Startup schedule initializes");
        assert_eq!(
            startup.systems().expect("initialized").count(),
            1,
            "`install_map_menu_systems` should add exactly one Startup system \
             (`populate_map_rooms`); the app's `after_map_menu_spawn` profile mark \
             brackets its set and has nothing to time without it"
        );
    }

    let installed = update.systems().expect("initialized").count();
    assert_eq!(
        installed, 3,
        "`install_map_menu_systems` added {installed} Update system(s); it owns \
         exactly \
         three — `handle_map_menu_hotkeys`, `map_menu_pointer_dismiss` and \
         `sync_map_menu`. ⇒ If this DROPPED, the map menu is dead in every \
         composition and nothing else in the tree says so (there is no app-level \
         map-menu test). If it GREW, raise this number deliberately and say what \
         joined them.\n\n\
         ⭐ It went 2 → 3 on 2026-09-06 when `sync_map_menu` was carved out of the \
         shell, and this assertion is what made that a decision instead of a drift."
    );
}

/// The installed systems are session-gated, so a host with no session world
/// does not crash without input.
///
/// `install_map_menu_systems` puts its Update systems behind
/// `.run_if(session_world_exists)`. The parameter failure comes on the first
/// update where a session world exists without the input resources. A quiet
/// session-less host is not evidence that the composition is correct.
#[test]
fn the_installed_map_systems_are_session_gated() {
    use bevy::prelude::*;

    let mut app = App::new();
    // `InputPlugin` supplies `ButtonInput<KeyCode>` but not
    // `MenuControlFrame`, so the hotkey system would fail if it ran. No
    // session world exists, so it does not run; completing the update is the
    // assertion.
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin));
    app.add_plugins(super::MapStatePlugin);
    super::install_map_menu_systems(&mut app);
    app.update();

    // The systems must be present, or this passes on an empty schedule.
    let mut update = app
        .world_mut()
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .remove(Update)
        .expect("the installer added systems to Update");
    update
        .initialize(app.world_mut())
        .expect("the Update schedule initializes");
    assert_eq!(
        update.systems_len(),
        3,
        "the installer's Update systems are missing, so 'they did not run' proves \
         nothing about the session gate"
    );
}

/// The simulation installer registers both map-truth systems.
///
/// Removing `install_map_simulation_systems` from the composition leaves the
/// app suite green, although `track_room_visits` persists visits through
/// `ResMut<AmbitionGameSave>`. The suite does not check map-visit persistence,
/// so this test checks the install.
///
/// Counted, not named (Bevy strips names without `debug`). Two, not three:
/// Bevy inserts `apply_deferred` between chained members only where the
/// earlier system leaves deferred work, and neither takes `Commands`. Do not
/// derive the count from the chain shape; run it.
#[test]
fn the_map_simulation_installer_registers_both_systems() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(super::MapStatePlugin);
    super::install_map_simulation_systems(&mut app, Update);

    let mut update = app
        .world_mut()
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .remove(Update)
        .expect("the installer added systems to Update");
    update
        .initialize(app.world_mut())
        .expect("the Update schedule initializes");
    assert_eq!(
        update.systems_len(),
        2,
        "the map simulation half lost a system: track_room_visits + sync_map_from_save"
    );
}

/// A session world with one room, so `track_room_visits` has a room to record.
#[cfg(test)]
fn one_room_session(app: &mut bevy::prelude::App, room_id: &str) {
    use ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope;
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    let world = ambition_platformer2d_world::prelude::AuthoredWorld::new(
        room_id,
        bevy::prelude::Vec2::new(640.0, 480.0),
        bevy::prelude::Vec2::new(32.0, 400.0),
        Vec::new(),
    );
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
            room_id,
            vec![ambition_platformer2d_world::rooms::RoomSpec::new(
                room_id, world,
            )],
            Vec::new(),
        ),
    );
}

fn visited(app: &bevy::prelude::App) -> Vec<String> {
    app.world()
        .resource::<super::MapMenuState>()
        .visited
        .iter()
        .cloned()
        .collect()
}

/// The visited set has the save's lifetime, not the process's. Three saves in
/// one process: the set follows each one exactly, and rooms from the previous
/// save do not carry over.
#[test]
fn the_visited_set_follows_whichever_save_is_live() {
    use ambition_persistence::save::AmbitionGameSave;
    use ambition_persistence::save_data::AmbitionGameSaveData;
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(super::MapStatePlugin);
    let mut first = AmbitionGameSaveData::default();
    first.set_flag(super::room_visited_flag("hall"), true);
    first.set_flag(super::room_visited_flag("lab"), true);
    app.insert_resource(AmbitionGameSave(first));
    app.add_systems(Update, super::sync_map_from_save);
    app.update();
    assert_eq!(
        visited(&app),
        vec!["hall", "lab"],
        "the first save hydrates"
    );

    // A different save becomes live in the same process (a load in place).
    let mut second = AmbitionGameSaveData::default();
    second.set_flag(super::room_visited_flag("vault"), true);
    *app.world_mut()
        .resource_mut::<AmbitionGameSave>()
        .data_mut() = second;
    app.update();
    assert_eq!(
        visited(&app),
        vec!["vault"],
        "the second save was not read, or the first save's rooms carried over"
    );

    // New game: the save is wiped the way `NewGameReset` wipes it.
    *app.world_mut()
        .resource_mut::<AmbitionGameSave>()
        .data_mut() = AmbitionGameSaveData::default();
    app.update();
    assert!(
        visited(&app).is_empty(),
        "a new game shows rooms the previous game visited: {:?}",
        visited(&app)
    );
}

/// After a new game, the room you stand in is visited again. The last-room
/// edge is the save's own flag, so it re-derives after the wipe; the map does
/// not open empty in the current room.
#[test]
fn a_new_game_records_the_room_the_player_is_already_standing_in() {
    use ambition_persistence::save::AmbitionGameSave;
    use ambition_persistence::save_data::AmbitionGameSaveData;
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(super::MapStatePlugin);
    app.init_resource::<AmbitionGameSave>();
    one_room_session(&mut app, "hall");
    super::install_map_simulation_systems(&mut app, Update);
    app.update();
    assert_eq!(
        visited(&app),
        vec!["hall"],
        "premise: the first visit is recorded"
    );
    assert!(
        app.world()
            .resource::<AmbitionGameSave>()
            .data()
            .flag(&super::room_visited_flag("hall")),
        "premise: and flagged on the save"
    );

    // New game, standing in the same room.
    *app.world_mut()
        .resource_mut::<AmbitionGameSave>()
        .data_mut() = AmbitionGameSaveData::default();
    app.update();
    assert!(
        app.world()
            .resource::<AmbitionGameSave>()
            .data()
            .flag(&super::room_visited_flag("hall")),
        "the fresh save never learned the player is in `hall`: the last-room \
         edge outlived the save it was an edge of"
    );
    assert_eq!(visited(&app), vec!["hall"]);
}
