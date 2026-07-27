//! **C4 slice 2: the versus stage is reachable and seats two fighters.**
//!
//! Slice 1 proved the engine can seat a roster; this proves the shipped host
//! offers it. The distinction is the whole of C4's complaint — the fight worked
//! and existed only where a test could see it, and "a stranger can run it and
//! watch" is what separates an engine demo from an engine.
//!
//! Driven through the real shell composition (`build_visible_app(NoWindow, true)`
//! plus the startup sequence), because a versus route that only a hand-built app
//! can reach is the same defect one layer up.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use ambition::game_shell::{ShellCommand, ShellRouteCatalog, ShellRouteId, ShellRouter};
use ambition_app::app::versus::{VERSUS_GAMEPLAY_ROUTE, VERSUS_ROOM_ID};
use ambition_app::app::{build_visible_app, VisibleRenderMode};

fn versus_app() -> App {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    app
}

/// Step until the host is sitting on its launcher.
///
/// A `GoTo` issued before the router has reached a route is dropped, so the
/// budget matters: it comes from the composed startup rather than a constant,
/// because the run-in has already grown from one card to two and a stale number
/// would silently turn this into "assert the host is still showing card one".
fn settle_to_launcher(app: &mut App) {
    for _ in 0..ambition_app::app::shared_host_startup_ticks() * 2 {
        app.update();
        if app
            .world()
            .resource::<ShellRouter>()
            .active
            .as_ref()
            .is_some_and(|active| active.route_id.as_str() == "ambition_launcher")
        {
            return;
        }
    }
    panic!("the host never reached its launcher, so no route can be chosen from it");
}

/// The route exists in the shipped host's route table.
///
/// A launcher entry nobody registered is the failure this catches: the versus
/// experience installs a route AND an experience registration, and the launcher
/// derives its entries from the latter — so a half-installed experience produces
/// a stage that runs and cannot be chosen.
#[test]
fn the_versus_route_is_registered_in_the_shipped_host() {
    let app = versus_app();
    let routes = app.world().resource::<ShellRouteCatalog>();
    assert!(
        routes
            .get(&ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE))
            .is_some(),
        "the versus route is not in the shipped host's route table, so nothing in \
         the launcher can reach the stage"
    );
}

/// Activating the route builds the arena and seats both fighters.
///
/// This is the assertion C4 was missing. It drives the real router — the same
/// path the launcher takes when somebody picks the entry — rather than inserting
/// a roster by hand, because inserting the roster by hand is what slice 1's unit
/// tests already do and it proves nothing about reachability.
#[test]
fn choosing_versus_seats_two_fighters_in_the_arena() {
    let mut app = versus_app();
    settle_to_launcher(&mut app);
    // The same command the launcher issues when somebody picks the entry.
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));

    // Preparation is a transaction with a load barrier; it takes frames.
    let mut active_room = None;
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition::runtime::demo_fixture::RoomSet>();
        if let Some(id) = rooms
            .iter(world)
            .next()
            .map(|set| set.active_spec().id.clone())
        {
            if id == VERSUS_ROOM_ID {
                active_room = Some(id);
                break;
            }
        }
    }
    assert_eq!(
        active_room.as_deref(),
        Some(VERSUS_ROOM_ID),
        "the versus route never activated its arena"
    );

    // Seating runs on the sim schedule once the room exists.
    for _ in 0..30 {
        app.update();
    }

    let world = app.world_mut();
    let mut worn = world.query::<&ambition::characters::actor::WornCharacter>();
    let mut characters: Vec<String> = worn.iter(world).map(|worn| worn.id().to_string()).collect();
    characters.sort();
    characters.dedup();

    for fighter in ["mary_o", "sanic"] {
        assert!(
            characters.iter().any(|id| id == fighter),
            "`{fighter}` is not on the stage. Bodies present: {characters:?}. The \
             stage seats one fighter from EACH provider on purpose — a match \
             between characters whose art, cues and movesets come from different \
             packages is the case the whole character seam was built for."
        );
    }
}

/// **Leaving versus takes the roster with it.**
///
/// `MatchParticipantRoster` is what seating reads, and it is a resource. Leaving
/// it installed would seat two fighters into whatever the player picked next —
/// Mary-O's level with a Sanic standing in it. That is the failure mode of every
/// global "current match" resource, and it is silent: the bodies look like they
/// belong to the level.
#[test]
fn leaving_versus_does_not_seat_fighters_into_the_next_game() {
    use ambition::actors::character_runtime::MatchParticipantRoster;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..300 {
        app.update();
        if app
            .world()
            .get_resource::<MatchParticipantRoster>()
            .is_some()
        {
            break;
        }
    }
    assert!(
        app.world()
            .get_resource::<MatchParticipantRoster>()
            .is_some(),
        "the versus route never installed its roster"
    );

    // Go somewhere else, the way the launcher's ReturnHome does.
    app.world_mut()
        .write_message(ambition::game_shell::ShellCommand::QuitToHome);
    for _ in 0..300 {
        app.update();
        if app
            .world()
            .get_resource::<MatchParticipantRoster>()
            .is_none()
        {
            break;
        }
    }
    assert!(
        app.world()
            .get_resource::<MatchParticipantRoster>()
            .is_none(),
        "the roster outlived the versus route: the next game the player picks \
         will have two fighters seated into it"
    );
}
