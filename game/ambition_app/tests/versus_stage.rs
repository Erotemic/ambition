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

/// Push a raw gamepad button value, the way the device backend would.
fn pad_set(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
    app.world_mut()
        .write_message(bevy::input::gamepad::RawGamepadEvent::Button(
            bevy::input::gamepad::RawGamepadButtonChangedEvent::new(pad, button, value),
        ));
}

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

    // EXACTLY the roster, not "at least the roster". The first version of this
    // asserted presence and passed while the arena held two Mary-Os: the session
    // spawns a player body wearing the starting character, and seating spawned a
    // second one beside it. Presence is the assertion you write when you have not
    // looked at the screen.
    assert_eq!(
        characters,
        vec!["mary_o".to_string(), "sanic".to_string()],
        "the stage's cast must be exactly its roster. One fighter comes from each \
         provider on purpose — a match between characters whose art, cues and \
         movesets come from different packages is the case the whole character \
         seam was built for."
    );
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

/// **C4 slice 6: two controllers make it couch versus.**
///
/// Everything below this line existed before and drove nobody: the second seat
/// got a body in slice 3, `SlotControls[1]` got a writer in slice 4, and the
/// controllers got partitioned in slice 5. This is the assertion that the three
/// meet — plug in two pads, pick Versus, and two people are playing.
///
/// It presses a real button on a real pad rather than writing `SlotControls`
/// directly, because every one of those slices could be individually correct
/// while the chain has a gap, and a test that starts halfway down the chain
/// cannot see the gap.
#[test]
fn two_controllers_make_versus_a_two_player_game() {
    use ambition::actors::actor::BodyKinematics;
    use ambition::characters::brain::{Brain, PlayerSlot};

    let mut app = versus_app();
    // Both pads present BEFORE the stage is chosen. The roster decides at stage
    // entry on purpose — a pad arriving mid-match is a roster edit, and this
    // stage has no rules for that.
    let pad_one = app.world_mut().spawn(Gamepad::default()).id();
    let pad_two = app.world_mut().spawn(Gamepad::default()).id();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    for _ in 0..30 {
        app.update();
    }

    // Two HUMAN seats, one per controller.
    let world = app.world_mut();
    let mut brains = world.query::<(Entity, &Brain)>();
    let mut seated: Vec<(u8, Entity)> = brains
        .iter(world)
        .filter_map(|(entity, brain)| match brain {
            Brain::Player(PlayerSlot(slot)) => Some((*slot, entity)),
            _ => None,
        })
        .collect();
    seated.sort_by_key(|(slot, _)| *slot);
    let slots: Vec<u8> = seated.iter().map(|(slot, _)| *slot).collect();
    assert_eq!(
        slots,
        vec![0, 1],
        "with two controllers connected the versus stage must seat two PLAYERS, \
         not a player and a CPU — this is the difference between watching the \
         fight and being in it"
    );
    let (_, body_one) = seated[0];
    let (_, body_two) = seated[1];
    let start_one = app.world().get::<BodyKinematics>(body_one).unwrap().pos.x;
    let start_two = app.world().get::<BodyKinematics>(body_two).unwrap().pos.x;

    // Player two walks right. Nothing is touched on player one's pad.
    pad_set(&mut app, pad_two, GamepadButton::DPadRight, 1.0);
    for _ in 0..40 {
        app.update();
    }
    let moved_one = app.world().get::<BodyKinematics>(body_one).unwrap().pos.x - start_one;
    let moved_two = app.world().get::<BodyKinematics>(body_two).unwrap().pos.x - start_two;
    assert!(
        moved_two > 1.0,
        "player two pressed right and their fighter did not move ({moved_two:.2}px): \
         somewhere between the controller, SlotControls[1] and Brain::Player(1) the \
         chain is broken, and the second player is a spectator"
    );
    assert!(
        moved_one.abs() < 1.0,
        "player two's controller moved player one's fighter ({moved_one:.2}px) — the \
         two seats are reading the same device"
    );

    // ...and the reverse, so a passing test cannot mean "one pad drives both".
    // Let player two's fighter COME TO REST before measuring again. A body that
    // was walking a moment ago is still decelerating, and 2px of coast read as
    // "player one's controller moved player two" the first time this ran.
    pad_set(&mut app, pad_two, GamepadButton::DPadRight, 0.0);
    let mut resting = 0;
    let mut last = f32::NAN;
    for _ in 0..240 {
        app.update();
        let x = app.world().get::<BodyKinematics>(body_two).unwrap().pos.x;
        if (x - last).abs() < 0.001 {
            resting += 1;
            if resting >= 10 {
                break;
            }
        } else {
            resting = 0;
            last = x;
        }
    }
    assert!(
        resting >= 10,
        "player two's fighter never stopped after the stick was released"
    );
    let before_two = app.world().get::<BodyKinematics>(body_two).unwrap().pos.x;
    let before_one = app.world().get::<BodyKinematics>(body_one).unwrap().pos.x;
    pad_set(&mut app, pad_one, GamepadButton::DPadRight, 1.0);
    for _ in 0..40 {
        app.update();
    }
    let one_moved = app.world().get::<BodyKinematics>(body_one).unwrap().pos.x - before_one;
    let two_moved = app.world().get::<BodyKinematics>(body_two).unwrap().pos.x - before_two;
    assert!(
        one_moved > 1.0,
        "player one pressed right and their fighter did not move ({one_moved:.2}px)"
    );
    assert!(
        two_moved.abs() < 1.0,
        "player one's controller moved player two's fighter ({two_moved:.2}px)"
    );
}
