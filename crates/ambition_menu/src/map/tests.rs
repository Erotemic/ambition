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
fn installing_the_map_menu_adds_the_systems_it_owns() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(super::MapStatePlugin);
    // ⇒ THE INSTALL IS A SEPARATE CALL, and this test follows it there. The plugin
    // declares vocabulary; a composition asks for the systems. Asserting against
    // the PLUGIN would now certify an empty schedule.
    super::install_map_menu_systems(&mut app);
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
    // ⛔ AND THE STARTUP HALF, because this guard counted UPDATE only and silently
    // did not cover `populate_map_rooms` when it was carved in. A count guard's
    // population is whatever schedule it names, and adding a system to a DIFFERENT
    // schedule slips past it without a word.
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

/// The installed systems are SESSION-GATED, which is why "no input" does not crash a
/// session-less host.
///
/// ⛔⛔ THIS TEST EXISTS BECAUSE THE ONE I TRIED FIRST WAS WRONG. The installer's
/// docstring said calling it without input "should crash loudly at startup", so I wrote
/// a `#[should_panic]` app with Bevy's `InputPlugin` (supplying `ButtonInput`) and no
/// `MenuControlFrame`. It did not panic — `install_map_menu_systems` puts its Update
/// systems behind `.run_if(session_world_exists)`, so a composition with no session
/// world never runs them at all. The doc has been corrected; this pins the mechanism
/// that made it wrong.
///
/// ⇒ The real contract: the parameter failure arrives on the first update where a
/// SESSION WORLD EXISTS and the input resources do not. A quiet session-less host is
/// not evidence the composition is correct.
#[test]
fn the_installed_map_systems_are_session_gated() {
    use bevy::prelude::*;

    let mut app = App::new();
    // `InputPlugin` supplies `ButtonInput<KeyCode>` and NOT `MenuControlFrame`, so the
    // hotkey system would fail its parameters if it ran. No session world exists, so it
    // does not run — and this update completing is the assertion.
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin));
    app.add_plugins(super::MapStatePlugin);
    super::install_map_menu_systems(&mut app);
    app.update();

    // ⚠ ANTI-VACUITY: the systems must actually BE there. Otherwise this passes on an
    // empty schedule and says nothing about gating.
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
/// ⛔⛔ A POISON COULD NOT WITNESS THIS CARVE, and the reason is COVERAGE rather than
/// disuse. Removing `install_map_simulation_systems` from the composition leaves
/// `app_it` at 578/578 — but `track_room_visits` writes `ResMut<AmbitionGameSave>`, so
/// visited rooms persist and the consumers are real. The suite simply asserts nothing
/// about map-visit persistence. ⇒ "The suite stayed green" is a statement about the
/// SUITE, so the move needs a check that can see it.
///
/// ⚠ A count, not names: Bevy strips system names without its `debug` feature, so a
/// name-based assertion would compare placeholders and pass vacuously.
///
/// ⚠ TWO, AND I EXPECTED THREE. Bevy inserts `apply_deferred` between chained members
/// CONDITIONALLY — only where the earlier system leaves deferred work to flush — not
/// automatically at every `.chain()` seam. Neither of these takes `Commands`, so there
/// is no sync point. (`install_fx_pipeline`'s guard reads 11 for 8 systems because three
/// of ITS members do.) ⇒ Do not derive an expected count from the chain shape; run it.
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
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
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

/// ⛔⛔ THE VISITED SET HAS THE SAVE'S LIFETIME, NOT THE PROCESS'S. It was
/// hydrated once per process behind a `Local<bool>`: a new game kept the old
/// game's rooms, and a second save activated later was never read. Three saves
/// in one process: the set must follow each one exactly -- rooms the previous
/// save visited must NOT carry over. A GPT review named both failures.
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

    // A different save becomes live in the SAME process -- a load, in place.
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

    // NEW GAME: the save is wiped the way `NewGameReset` wipes it.
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

/// ⛔⛔ AFTER A NEW GAME, THE ROOM YOU ARE STANDING IN IS VISITED AGAIN. The
/// last-room edge lived in a `Local<Option<String>>`: the player starts a new
/// game in the same room, the local still names it, and the fresh save never
/// learns they are there -- the map opens empty in the room they are standing
/// in. The edge is now the save's own flag, so it re-derives after the wipe.
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

    // NEW GAME, standing in the same room.
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
