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
