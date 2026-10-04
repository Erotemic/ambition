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
    ambition_platformer2d_world::rooms::insert_room_set(
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

/// Two rooms side by side, `hall` live at the activation room. With `lab_live`,
/// `lab` is live beside it as a second room. The primary body stands in
/// `body_in`. The map is open over both rooms.
fn map_over_two_rooms(lab_live: bool, body_in: &str) -> bevy::prelude::App {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        spawn_live_room, ActiveSessionScope, InRoomInstance, LiveRoomInstance,
    };
    use ambition_platformer2d_world::rooms::{insert_room_set, RoomSet, RoomSpec};
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    let room = |id: &str| {
        let world = ambition_platformer2d_world::prelude::AuthoredWorld::new(
            id,
            Vec2::new(640.0, 480.0),
            Vec2::new(32.0, 400.0),
            Vec::new(),
        );
        RoomSpec::new(id, world)
    };
    let rooms = RoomSet::from_parts_or_panic("hall", vec![room("hall"), room("lab")], Vec::new());
    let lab = rooms.definition_by_id("lab").expect("lab is authored");
    insert_room_set(app.world_mut(), rooms);
    let lab_room = LiveRoomInstance::from_ordinal(1);
    if lab_live {
        spawn_live_room(app.world_mut(), lab_room, lab);
    }
    let body_room = if body_in == "lab" { lab_room } else { LiveRoomInstance::ACTIVATION };
    app.world_mut().spawn((
        ambition_platformer2d_shared_tangle::body::PrimaryBody,
        InRoomInstance(body_room),
    ));
    app.insert_resource(super::MapMenuState {
        open: true,
        rooms: ["hall", "lab"]
            .iter()
            .enumerate()
            .map(|(i, id)| super::MapRoomNode {
                id: id.to_string(),
                world_min: Vec2::new(640.0 * i as f32, 0.0),
                world_size: Vec2::new(640.0, 480.0),
            })
            .collect(),
        ..Default::default()
    });
    app.world_mut().spawn(super::ui::MapMenuCanvas);
    app.world_mut().spawn((super::ui::MapMenuStatus, Text::default()));
    app.add_systems(Update, super::sync_map_menu);
    app.update();
    app
}

/// The status line of the map, and the room whose box has the active colour.
fn shown_active(app: &mut bevy::prelude::App) -> (String, Vec<String>) {
    use bevy::prelude::*;
    let status = app
        .world_mut()
        .query_filtered::<&Text, With<super::ui::MapMenuStatus>>()
        .single(app.world())
        .expect("one status line")
        .0
        .clone();
    let active = Color::srgba(0.55, 0.92, 0.62, 0.95);
    let mut boxes: Vec<String> = app
        .world_mut()
        .query::<(&super::ui::MapRoomBox, &BackgroundColor)>()
        .iter(app.world())
        .filter(|(_, colour)| colour.0 == active)
        .map(|(room_box, _)| room_box.room_id.clone())
        .collect();
    boxes.sort();
    (status, boxes)
}

/// OW1: with two live rooms the map shows the primary body's room as active,
/// and a crossing between the two live rooms moves it. Before, the map read
/// the sole live room, so while two rooms were live it did not run. The
/// control is one live room, which shows the room the body stands in.
#[test]
fn the_map_shows_the_primary_bodys_room_while_two_rooms_are_live() {
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
    use bevy::prelude::*;

    let mut one = map_over_two_rooms(false, "hall");
    let (status, boxes) = shown_active(&mut one);
    assert!(status.contains("hall active"), "control: one live room: {status}");
    assert_eq!(boxes, vec!["hall"], "control: one live room");

    let mut two = map_over_two_rooms(true, "lab");
    let (status, boxes) = shown_active(&mut two);
    assert!(status.contains("lab active"), "two live rooms: {status}");
    assert_eq!(boxes, vec!["lab"], "two live rooms");

    // The body crosses back into the hall, which is still live. No resource
    // changes, so only the room the map last showed tells the map to redraw.
    let body = two
        .world_mut()
        .query_filtered::<Entity, With<ambition_platformer2d_shared_tangle::body::PrimaryBody>>()
        .single(two.world())
        .unwrap();
    two.world_mut()
        .entity_mut(body)
        .insert(InRoomInstance(LiveRoomInstance::ACTIVATION));
    two.update();
    let (status, boxes) = shown_active(&mut two);
    assert!(status.contains("hall active"), "after the crossing: {status}");
    assert_eq!(boxes, vec!["hall"], "after the crossing, the boxes were not redrawn");
}
