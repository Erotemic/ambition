//! The distinction is the whole of C4's complaint — the fight worked and existed only where a test
//! could see it, and "a stranger can run it and watch" is what separates an engine demo from an
//! engine.
//!
//! Driven through the real shell composition (`build_visible_app(NoWindow, true)`
//! plus the startup sequence), because a versus route that only a hand-built app
//! can reach is the same defect one layer up.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use ambition_app::app::versus::{VERSUS_GAMEPLAY_ROUTE, VERSUS_ROOM_ID};
use ambition_app::app::versus_rules::{MatchPhase, VersusMatch};
use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::game_shell::{
    ShellCommand, ShellRouteCatalog, ShellRouteId, ShellRouter,
};

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

/// Step until the round is actually LIVE.
///
/// A round opens on a countdown: the fighters are reset, healed and visible, and
/// for 90 simulation ticks nothing they or the CPU decides reaches the fight.
/// Every test below that presses a button or waits for a knockout is asking a
/// question about the FIGHT, and asking it during the count gets the honest
/// answer "nobody can act yet" — which reads as a broken controller chain.
///
/// So this is the line between "the stage exists" and "the round is being fought", and it is
/// deliberately a wait on the PHASE rather than a bigger magic number.
fn settle_into_a_live_round(app: &mut App) {
    for _ in 0..600 {
        app.update();
        if matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Fighting
        ) {
            return;
        }
    }
    panic!(
        "the round never went live: the countdown is still counting after 600 ticks, \
         so the stage is showing a card over a fight that never starts"
    );
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
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
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
    settle_into_a_live_round(&mut app);

    let world = app.world_mut();
    let mut worn = world.query::<&ambition_platformer2d::characters::actor::WornCharacter>();
    let mut characters: Vec<String> = worn.iter(world).map(|worn| worn.id().to_string()).collect();
    characters.sort();

    // Require exactly the seated roster so duplicate starting bodies cannot hide
    // behind presence-only assertions.
    assert_eq!(
        characters,
        vec![
            "arena_duelist_close".to_string(),
            "arena_duelist_long".to_string()
        ],
        "the stage's cast must be exactly its roster. The two fighters are the \
         arena's own — the demo casts author no move list, and giving one attacks \
         to make versus work would be authoring against its design — but they \
         SHARE the demos' art by id, so the crossover the character seam was \
         built for is still what the stage draws."
    );
}

/// Leaving versus takes the roster with it.
#[test]
fn leaving_versus_does_not_seat_fighters_into_the_next_game() {
    use ambition_platformer2d::actors::character_runtime::MatchParticipantRoster;

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
        .write_message(ambition_platformer2d::game_shell::ShellCommand::QuitToHome);
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

/// It presses a real button on a real pad rather than writing `SlotControls`
/// directly, because every one of those slices could be individually correct
/// while the chain has a gap, and a test that starts halfway down the chain
/// cannot see the gap.
#[test]
fn two_controllers_make_versus_a_two_player_game() {
    use ambition_platformer2d::characters::control::DrivingParticipant;
    use ambition_platformer2d::engine_core::BodyKinematics;

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
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    // Two HUMAN seats, one per controller.
    let world = app.world_mut();
    let mut drivers = world.query::<(Entity, &DrivingParticipant)>();
    let mut seated: Vec<(u8, Entity)> = drivers
        .iter(world)
        .map(|(entity, driver)| (driver.0 .0, entity))
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
         somewhere between the controller, SlotControls[1] and \
         DrivingParticipant(1) the \
         chain is broken, and the second player is a spectator"
    );
    assert!(
        moved_one.abs() < 1.0,
        "player two's controller moved player one's fighter ({moved_one:.2}px) — the \
         two seats are reading the same device"
    );

    // ...and the reverse, so a passing test cannot mean "one pad drives both".
    pad_set(&mut app, pad_two, GamepadButton::DPadRight, 0.0);
    for _ in 0..240 {
        app.update();
    }
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
    // A RATIO, not an absolute. Residual coast is a few pixels; a fighter being
    // driven by a controller it does not own would travel a comparable distance
    // to the one that owns it, so the two failures are orders of magnitude apart
    // and the threshold does not have to guess where "stopped" is.
    assert!(
        two_moved.abs() < one_moved * 0.25,
        "player one's controller moved player two's fighter: player one went \
         {one_moved:.2}px and player two went {two_moved:.2}px on the same input, \
         which is cross-talk rather than coast"
    );
}

/// A seated fighter wears its character all the way down.
///
/// Wearing a character is not a label: `apply_worn_character_gameplay` is the one writer that
/// turns `WornCharacter` into a persona — the body's name, its action set, its moveset and its
/// identity kit. It is a QUERY, and a body missing any required column does not match it,
/// silently.
///
/// This asserts the derive actually ran on the seated body, because "the cast is
/// right" is what a body looks like from a query and says nothing about whether
/// the body can do anything.
#[test]
fn a_seated_fighter_derives_its_character_and_not_just_its_name() {
    use ambition_platformer2d::characters::brain::ActionSet;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    // The CPU opponent, named by its SEAT.
    //
    // With every fighter built the same way neither carries the marker, so that filter selected
    // whichever seat the query reached first. `MatchSeat` is the identity a match fighter has, and
    // its own doc says why every other way is a guess.
    let world = app.world_mut();
    let mut seated = world.query::<(
        &ambition_platformer2d::actors::character_runtime::MatchSeat,
        &ambition_platformer2d::characters::actor::WornCharacter,
        &Name,
        &ActionSet,
    )>();
    let (_, worn, name, action_set) = seated
        .iter(world)
        .find(|(seat, ..)| seat.0 == 1)
        .expect("the versus stage seats an opponent in seat 1");

    assert_eq!(worn.id(), "arena_duelist_close");
    assert_ne!(
        name.as_str(),
        "arena_duelist_close",
        "the body still carries seating's placeholder name, so the persona \
         derive never matched it — which means it has none of the character's \
         gameplay either, only its id"
    );

    // The action set stays EMPTY on purpose and that is not a gap: these
    // fighters author their moves on their `CharacterDefinition`, which the
    // persona derive prefers over anything the catalog row implies. An
    // `ActionSet` melee would be a second opinion that never wins. The moveset
    // itself is checked by `both_fighters_can_actually_hit_each_other`.
    let _ = action_set;
}

/// The whole point: one fighter presses attack and the other loses health.
///
/// Couch versus worked and was two people walking into each other, because
/// neither demo cast authors a move list. This is the assertion that the
/// arena's own fighters changed that — and it drives the REAL path (press the
/// button, let the move play, let the volume resolve) rather than writing
/// damage, because every intermediate seam between the button and the HP is
/// exactly what could be missing.
#[test]
fn both_fighters_can_actually_hit_each_other() {
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::characters::control::DrivingParticipant;
    use ambition_platformer2d::engine_core::BodyKinematics;

    let mut app = versus_app();
    let pad_one = app.world_mut().spawn(Gamepad::default()).id();
    let pad_two = app.world_mut().spawn(Gamepad::default()).id();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    let world = app.world_mut();
    let mut drivers = world.query::<(Entity, &DrivingParticipant)>();
    let mut seated: Vec<(u8, Entity)> = drivers
        .iter(world)
        .map(|(entity, driver)| (driver.0 .0, entity))
        .collect();
    seated.sort_by_key(|(slot, _)| *slot);
    assert_eq!(seated.len(), 2, "the arena did not seat two players");
    let (attacker, victim) = (seated[0].1, seated[1].1);
    let pads = [pad_one, pad_two];

    // Walk the attacker into range.
    let toward = |app: &App| -> f32 {
        let a = app.world().get::<BodyKinematics>(attacker).unwrap().pos.x;
        let v = app.world().get::<BodyKinematics>(victim).unwrap().pos.x;
        (v - a).signum()
    };
    let dir = toward(&app);
    let walk = if dir > 0.0 {
        GamepadButton::DPadRight
    } else {
        GamepadButton::DPadLeft
    };
    pad_set(&mut app, pads[0], walk, 1.0);
    for _ in 0..240 {
        app.update();
        let a = app.world().get::<BodyKinematics>(attacker).unwrap().pos.x;
        let v = app.world().get::<BodyKinematics>(victim).unwrap().pos.x;
        if (v - a).abs() < 28.0 {
            break;
        }
    }
    pad_set(&mut app, pads[0], walk, 0.0);
    for _ in 0..10 {
        app.update();
    }

    let start_hp = app.world().get::<BodyHealth>(victim).unwrap().current();
    assert!(start_hp > 0, "the victim started the fight already dead");

    // Swing, repeatedly. One press could land in a frame the victim has drifted
    // out of; a fighter that can never hit is what this is looking for.
    for _ in 0..12 {
        pad_set(&mut app, pads[0], GamepadButton::West, 1.0);
        for _ in 0..3 {
            app.update();
        }
        pad_set(&mut app, pads[0], GamepadButton::West, 0.0);
        for _ in 0..20 {
            app.update();
        }
        if app.world().get::<BodyHealth>(victim).unwrap().current() < start_hp {
            break;
        }
    }

    let end_hp = app.world().get::<BodyHealth>(victim).unwrap().current();
    assert!(
        end_hp < start_hp,
        "player one swung twelve times in range and the other fighter is still \
         on {end_hp}/{start_hp} HP. Versus is two people walking into each other: \
         the hand-authored moveset never reached the body, or the swing never \
         resolved a volume against it."
    );
}

/// A round can be lost, and a match can be won. (L8)
///
/// Drives the rules through the resource rather than by landing a hundred real
/// swings: the swing path is proven by
/// `both_fighters_can_actually_hit_each_other`, and repeating it here would make
/// this a slow test of the same thing. What is under test is the RULE — that
/// zero health ends a round, that the round is counted to the other seat, that
/// the fighters come back, and that two rounds take the match.
#[test]
fn a_ko_wins_a_round_and_two_rounds_win_the_match() {
    use ambition_app::app::versus_rules::ROUNDS_TO_WIN;
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::engine_core::BodyKinematics;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    let seats = |app: &mut App| -> Vec<(usize, Entity)> {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        let mut v: Vec<(usize, Entity)> = q.iter(world).map(|(e, s)| (s.0, e)).collect();
        v.sort_by_key(|(seat, _)| *seat);
        v
    };
    let centre = {
        let world = app.world_mut();
        let mut q = world.query::<&ambition_platformer2d::engine_core::RoomGeometry>();
        q.iter(world)
            .next()
            .map(|geometry| geometry.0.spawn)
            .expect("the arena has geometry")
    };
    let all = seats(&mut app);
    assert_eq!(
        all.len(),
        2,
        "the fighters carry no MatchSeat, so the rules cannot tell who is who"
    );
    let seat_one = all[1].1;
    // KO seat 1, twice. Seat 0 should take both rounds and the match.
    for round in 1..=ROUNDS_TO_WIN {
        // Move the fighter off its seat FIRST. Knocking out a fighter that
        // never moved makes "it was returned to its seat" true for free, which
        // is the assertion you write when you have not checked whether the
        // reset does anything. Written directly because the point is to test
        // the RESET, not to reproduce a walk.
        //
        // The seat is COMPUTED, not sampled: the CPU walks, so "where it is
        // now" stopped being its seat the moment the opponent got a brain.
        let seat_x = ambition_platformer2d::actors::character_runtime::seat_placement(1, centre)
            .0
            .x;
        app.world_mut()
            .get_mut::<BodyKinematics>(seat_one)
            .unwrap()
            .pos
            .x = seat_x - 220.0;
        app.update();
        let knocked_down_at = app.world().get::<BodyKinematics>(seat_one).unwrap().pos;
        app.world_mut()
            .get_mut::<BodyHealth>(seat_one)
            .unwrap()
            .health
            .current = 0;
        app.update();

        let state = app.world().resource::<VersusMatch>();
        assert_eq!(
            state.wins("blue"),
            round,
            "a fighter hit zero health and the round was not counted to the \
             other TEAM. Seat 1 is red, so blue takes the round."
        );

        // Ride out the KO hold, then check the fighters came BACK: a versus
        // stage whose second round starts with one fighter still at zero health
        // is one round long.
        for _ in 0..600 {
            app.update();
            if matches!(
                app.world().resource::<VersusMatch>().phase,
                MatchPhase::Fighting { .. }
            ) {
                break;
            }
        }
        if round < ROUNDS_TO_WIN {
            assert!(
                app.world().get::<BodyHealth>(seat_one).unwrap().current() > 0,
                "the next round started with the knocked-out fighter still dead"
            );
            let back_at = app.world().get::<BodyKinematics>(seat_one).unwrap().pos;
            assert!(
                (back_at.x - seat_x).abs() < 1.0,
                "the fighter was knocked out at x={:.1} and the next round \
                 started with it at x={:.1} instead of its seat at x={seat_x:.1} \
                 — the reset puts nobody back",
                knocked_down_at.x,
                back_at.x
            );
        }
    }

    // Two rounds took the match, and the match reset for the next one — a
    // scoreboard that stays on 2-0 forever is a match nobody can play again.
    let state = app.world().resource::<VersusMatch>();
    assert!(
        state.rounds_won.is_empty(),
        "the match did not reset after it was won: {:?}",
        state.rounds_won
    );
    // Back to round one, and no longer holding the last match's result. The
    // loop above already rode past the fresh match's countdown, which is why the
    // phase claim here is the negative one — `round` is what says the match
    // reset rather than continued.
    assert_eq!(state.round, 1, "a fresh match starts at round one");
    assert!(
        !matches!(state.phase, MatchPhase::Ko { .. } | MatchPhase::Won { .. }),
        "the match stayed on its own victory card instead of restarting: {:?}",
        state.phase
    );
}

/// The CPU opponent actually fights. (L11)
///
/// Player-vs-CPU is the mode anybody with ONE controller gets, which makes it
/// the default versus experience and the one a stranger sees first. It shipped
/// as a fight against a statue: the seated body had a target, an `ActorControl`
/// and a faction, and no `Brain` — the enemy spawn path inserts one beside the
/// cluster and seating did not, so every component that would explain the
/// stillness was present and correct.
///
/// Asserts MOVEMENT, not damage. Whether a given brain profile is a good
/// opponent is a tuning question with no single right answer; whether it does
/// anything at all is not.
#[test]
fn the_cpu_opponent_is_not_a_statue() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::engine_core::BodyKinematics;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    // No gamepads were connected, so seat 1 is the CPU.
    let cpu = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 1)
            .map(|(entity, _)| entity)
            .expect("the versus stage seats an opponent")
    };
    assert!(
        app.world()
            .get::<ambition_platformer2d::characters::brain::Brain>(cpu)
            .is_some(),
        "the CPU fighter has no Brain at all, so nothing will ever write its \
         ActorControl and it cannot move whatever else is right"
    );

    // PATH LENGTH, not displacement. Comparing the position before and after
    // measures nothing if a whole ROUND happened in between: this fighter walks
    // at its opponent, falls into the arena, dies, and `begin_round` puts it back
    // on its seat — so both samples read "at the seat" and the test reported
    // "moved 0.4px" about a body that had covered 350. It is the same shape as
    // asserting a pendulum never moved because it came back.
    let mut travelled = 0.0f32;
    let mut previous = app.world().get::<BodyKinematics>(cpu).unwrap().pos;
    for _ in 0..300 {
        app.update();
        let Some(kin) = app.world().get::<BodyKinematics>(cpu) else {
            break;
        };
        let step = (kin.pos - previous).length();
        // A round reset TELEPORTS the body back to its seat. Counting that jump
        // as travel would let a fighter pass this test by dying repeatedly.
        if step < 64.0 {
            travelled += step;
        }
        previous = kin.pos;
    }
    assert!(
        travelled > 8.0,
        "the CPU opponent covered {travelled:.1}px in five seconds with a player \
         standing in front of it. Player-vs-CPU is what anybody with one \
         controller plays, and it is a fight against a statue."
    );
}

/// Seat 0 can lose a round.
///
/// Seat 0 is the adopted PRIMARY PLAYER, and the primary player's death runs
/// `death_respawn_player`: teleport to the room spawn, full heal, banner. That
/// happens inside the damage pass, long before any rules layer looks at health
/// — so seat 0 was never observed at zero, seat 1 could never be awarded a
/// round, and best-of-three was rigged one way.
///
/// Drives real health to zero and asserts the round is counted to the OTHER
/// seat, which is the only statement that distinguishes "the fighter died" from
/// "the fighter respawned and nobody noticed".
#[test]
fn seat_zero_can_lose_a_round_and_is_not_respawned_out_from_under_the_rules() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::engine_core::BodyKinematics;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    let seat_zero = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the stage seats a player")
    };
    let where_it_stood = app.world().get::<BodyKinematics>(seat_zero).unwrap().pos;

    // A REAL lethal hit, through the real damage path. Writing health to zero
    // directly proves nothing: the death CONSEQUENCE lives in the damage pass,
    // so a hand-zeroed body never invokes it and the test passes whether or not
    // the fix exists. (It did, first time round.)
    let hp = app.world().get::<BodyHealth>(seat_zero).unwrap().current();
    let volume: ambition_platformer2d::engine_core::CombatVolume =
        ambition_platformer2d::engine_core::Aabb::new(
            where_it_stood,
            ambition_platformer2d::engine_core::Vec2::new(40.0, 40.0),
        )
        .into();
    app.world_mut()
        .write_message(ambition_platformer2d::combat::events::HitEvent {
            strike_sfx: None,
            volume,
            damage: hp + 10,
            source: ambition_platformer2d::combat::events::HitSource::Melee,
            attacker: None,
            // the two consumers are a surviving fork and this test is not
            // the place to remove it. `HitTarget::Body` / `HitTarget::Body`
            // are documented as a deliberate split — the relational
            // actor-vs-actor path exists so an Enemy-faction body can damage a
            // Boss-faction one without the bipartite assumption — but "which
            // variant names a body" is now decided by how that body was
            // CONSTRUCTED, which is exactly the coupling the rest of this
            // landing deleted. Named here so the next reader finds it on
            // purpose rather than by having a fixture stop landing hits.
            target: ambition_platformer2d::combat::events::HitTarget::Body(seat_zero),
            mode: ambition_platformer2d::combat::events::HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
    for _ in 0..4 {
        app.update();
    }
    assert!(
        app.world().get::<BodyHealth>(seat_zero).unwrap().current() <= 0,
        "the lethal hit did not reach seat 0 at all, so this measures nothing"
    );

    let state = app.world().resource::<VersusMatch>();
    assert_eq!(
        state.wins("red"),
        1,
        "seat 0 hit zero health and the red team was not awarded the round. The \
         exploration respawn healed it before the rules could look, so seat 0 \
         cannot lose and the match is rigged."
    );
    let after = app.world().get::<BodyKinematics>(seat_zero).unwrap().pos;
    assert!(
        (after - where_it_stood).length() < 200.0,
        "seat 0 was teleported to the room spawn on death — that is the \
         exploration respawn, and a round is not an exploration event"
    );
}

/// Coming back to Versus starts a new match.
///
/// `VersusMatch` is a long-lived resource and `run_versus_rules` simply returns
/// when no roster exists, so leaving mid-match froze the score rather than
/// ending it. Walking back in resumed somebody else's game — 1-0, or a KO
/// countdown already running.
#[test]
fn returning_to_versus_starts_a_fresh_match() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    let enter = |app: &mut App| {
        app.world_mut()
            .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
        // Wait for the ROSTER, not for seated bodies: the previous match's
        // bodies can still be around while the route is switching, so a seat
        // count breaks the loop before re-entry has happened at all.
        for _ in 0..900 {
            app.update();
            if app
                .world()
                .get_resource::<ambition_platformer2d::actors::character_runtime::MatchParticipantRoster>()
                .is_some()
            {
                break;
            }
        }
        settle_into_a_live_round(app);
    };
    enter(&mut app);

    // Win a round, then walk out mid-match.
    let seat_one = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 1)
            .map(|(entity, _)| entity)
            .expect("two seats")
    };
    app.world_mut()
        .get_mut::<BodyHealth>(seat_one)
        .unwrap()
        .health
        .current = 0;
    app.update();
    assert_eq!(
        app.world().resource::<VersusMatch>().wins("blue"),
        1,
        "the fixture never won a round, so leaving proves nothing"
    );

    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::QuitToHome);
    for _ in 0..600 {
        app.update();
        if app
            .world()
            .get_resource::<ambition_platformer2d::actors::character_runtime::MatchParticipantRoster>()
            .is_none()
        {
            break;
        }
    }
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actors::character_runtime::MatchParticipantRoster>()
            .is_none(),
        "the fixture never actually left the stage, so re-entry is not being tested"
    );
    {
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        assert_eq!(
            q.iter(world).count(),
            0,
            "the previous match's fighters outlived their session"
        );
    }
    enter(&mut app);

    // And both fighters start the new match on full health.
    {
        let world = app.world_mut();
        let mut q = world.query::<(&MatchSeat, &BodyHealth)>();
        let mut rows: Vec<(usize, i32, i32)> = q
            .iter(world)
            .map(|(seat, health)| (seat.0, health.current(), health.health.max))
            .collect();
        rows.sort_by_key(|(seat, ..)| *seat);
        for (seat, current, max) in rows {
            assert_eq!(
                current, max,
                "seat {seat} started the new match on {current}/{max} — a match \
                 that begins with somebody already hurt is the previous match \
                 leaking into it"
            );
        }
    }

    let state = app.world().resource::<VersusMatch>();
    assert!(
        state.rounds_won.is_empty(),
        "walking back into Versus resumed the previous match's score: {:?}",
        state.rounds_won
    );
    // Not mid-KO and not mid-victory. Which of `Starting`/`Fighting` it is
    // depends on how far `enter` had to step to find the roster, and pinning
    // that would be pinning the helper rather than the claim: what the previous
    // match must not leave behind is a HOLD.
    assert!(
        !matches!(state.phase, MatchPhase::Ko { .. } | MatchPhase::Won { .. }),
        "the new match started mid-KO, inheriting the old one's hold: {:?}",
        state.phase
    );
}

/// A KO stops the fight.
///
/// Only the rules and the HUD ever read `MatchPhase`.
///
/// Asserts the SIM CLOCK is zeroed during the hold, because that is the fix —
/// the engine's own freeze primitive, rather than this module trying to name
/// and silence every system that could still act.
#[test]
fn a_knockout_freezes_the_fight_until_the_next_round() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        if q.iter(world).count() == 2 {
            break;
        }
    }
    settle_into_a_live_round(&mut app);
    let scale = |app: &App| {
        app.world()
            .resource::<ambition_platformer2d::time::ClockState>()
            .time_scale
    };
    assert!(
        scale(&app) > 0.5,
        "the fight was already frozen before anybody was knocked out"
    );

    let seat_one = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 1)
            .map(|(entity, _)| entity)
            .expect("two seats")
    };
    app.world_mut()
        .get_mut::<BodyHealth>(seat_one)
        .unwrap()
        .health
        .current = 0;
    for _ in 0..6 {
        app.update();
    }
    assert!(
        matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Ko { .. }
        ),
        "the fixture never reached the KO hold"
    );
    // The engine RAMPS a clock scale rather than snapping it (the time-control
    // smoother's job), so the freeze arrives over a few frames rather than on
    // the tick the KO lands. Deliberately not asserted as instantaneous: an
    // instant stop is a feel decision this stage has not made.
    for _ in 0..40 {
        app.update();
    }
    assert!(
        scale(&app) < 0.01,
        "the simulation kept running through the KO card at scale {} — the \
         surviving fighter is still playing while the round is over",
        scale(&app)
    );

    // ...AND THE FREEZE ENDS. The hold wrote a scale-0 request and then a scale-1 request in
    // the SAME frame, and the clock reducer keeps the strongest slow by `min` — deliberately,
    // so ordering cannot decide a freeze — so the release resolved to 0 every time. The next
    // round began under a clock that only recovered because an unrelated system asks for full
    // pace every frame, and then only by RAMPING: seconds of slow motion at the start of round
    // two.
    let mut thawed = false;
    for _ in 0..900 {
        app.update();
        if matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Fighting { .. }
        ) {
            thawed = true;
            break;
        }
    }
    assert!(thawed, "the KO hold never ended");
    // Two ticks for the request to reach the clock, not the forty the ramp
    // above needed. `ClockResetRequest` SNAPS — that is what distinguishes a
    // round starting at full speed from one sliding up to it.
    app.update();
    app.update();
    assert!(
        scale(&app) > 0.99,
        "the next round started at clock scale {} — the KO freeze is still \
         wearing off while both fighters are supposed to be playing",
        scale(&app)
    );
}

/// A decided round stops being fought.
///
/// `a_knockout_freezes_the_fight_until_the_next_round` asserts the clock, and the clock is a
/// RAMP: the KO asks for scale zero and the smoother slides down to it over the following
/// second, which is the genre's own beat and staying. Nothing reads `MatchPhase` — not input,
/// not the brains, not move triggering — so for the length of that ramp both fighters went on
/// accepting control, walking and swinging, after the score had been incremented and the winner
/// named.
///
/// Asserted on the CPU fighter specifically, because that is the half that was
/// hardest to fix: `ScriptedControl` was blanked only after the PLAYER brains,
/// in `PlayerInput`, and actor brains write their frame a whole phase later in
/// `WorldPrep`. A marker that suppresses humans and not opponents suspends
/// nothing in a stage whose default mode is player-versus-CPU.
#[test]
fn a_decided_round_takes_the_controls_away() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::control::ActorControlFrame;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::characters::control::ActorControl;
    use ambition_platformer2d::characters::control::ScriptedControl;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        if q.iter(world).count() == 2 {
            break;
        }
    }
    settle_into_a_live_round(&mut app);
    let seat = |app: &mut App, want: usize| -> Entity {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == want)
            .map(|(entity, _)| entity)
            .expect("both seats are filled")
    };
    // Seat 1 is the CPU with no second pad connected; seat 0 is the human.
    let cpu = seat(&mut app, 1);
    let human = seat(&mut app, 0);

    // The control this test is about has to EXIST while the round is live,
    // otherwise "it went neutral" below is a fact about a fighter who was doing
    // nothing anyway.
    let mut acted_while_fighting = false;
    for _ in 0..60 {
        app.update();
        if app.world().get::<ActorControl>(cpu).unwrap().0 != ActorControlFrame::neutral() {
            acted_while_fighting = true;
            break;
        }
    }
    assert!(
        acted_while_fighting,
        "the CPU never produced a control frame during the round, so this test \
         cannot tell a suspended fighter from an idle one"
    );
    assert!(
        app.world().get::<ScriptedControl>(cpu).is_none(),
        "the fighters were already suspended while the round was live"
    );

    // Decide the round on the HUMAN's seat, so the fighter under test is alive
    // and still has every reason to keep playing.
    app.world_mut()
        .get_mut::<BodyHealth>(human)
        .unwrap()
        .health
        .current = 0;
    app.update();
    assert!(
        matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Ko { .. } | MatchPhase::Won { .. }
        ),
        "the fixture never decided the round"
    );
    assert!(
        app.world().get::<ScriptedControl>(cpu).is_some(),
        "the round was decided and the surviving fighter still answers input"
    );

    // Not "the frame happens to be neutral" — WRITE a full-throttle attacking
    // frame and watch it be taken away again. That is the difference between a
    // fighter who has stopped and one who has not been asked to.
    for _ in 0..3 {
        let mut control = app.world_mut().get_mut::<ActorControl>(cpu).unwrap();
        control.0.locomotion = ambition_platformer2d::engine_core::LocalAxes::new(1.0, 0.0);
        control.0.melee_pressed = true;
        app.update();
        let frame = app.world().get::<ActorControl>(cpu).unwrap().0;
        assert_eq!(
            frame,
            ActorControlFrame::neutral(),
            "a fighter kept its control frame after the round was decided: \
             {frame:?} — it is still walking and swinging under the KO card"
        );
    }

    // And it gets them back, or round two is unplayable.
    for _ in 0..900 {
        app.update();
        if matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Fighting { .. }
        ) {
            break;
        }
    }
    assert!(
        app.world().get::<ScriptedControl>(cpu).is_none(),
        "the next round started with the fighters still suspended"
    );
}

/// A round boundary tells the fighter's PROVIDER to reset its own state.
///
/// `begin_round` restores health, position, facing and every engine-owned body
/// cluster, and its comment claimed that as a clean start. It is not one for a
/// fighter authored by a provider: Sanic's ball-dash charge and rolling form and
/// Mary-O's spark cadence are components this module has never heard of, and a
/// round that begins with a stored charge begins with a free launch.
///
/// The seam is `BodyRestarted`, and this test stands in for a provider rather
/// than importing one — which is the point. Any crate that attaches state to a
/// body gets the announcement and answers for itself, without the ruleset
/// learning a single provider type.
#[test]
fn a_round_boundary_tells_the_provider_to_reset_its_own_state() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;

    /// Stands in for `BallDash::charge` — provider-owned, transient, and the
    /// kind of state a generic reset cannot name.
    #[derive(Component)]
    struct ProviderCharge(f32);

    let mut app = versus_app();
    app.add_observer(
        |restart: On<ambition_platformer2d::engine_core::BodyRestarted>,
         mut charges: Query<&mut ProviderCharge>| {
            if let Ok(mut charge) = charges.get_mut(restart.entity) {
                charge.0 = 0.0;
            }
        },
    );
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        if q.iter(world).count() == 2 {
            break;
        }
    }
    settle_into_a_live_round(&mut app);
    let seat_one = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 1)
            .map(|(entity, _)| entity)
            .expect("two seats")
    };

    app.world_mut()
        .entity_mut(seat_one)
        .insert(ProviderCharge(1.0));
    app.world_mut()
        .get_mut::<BodyHealth>(seat_one)
        .unwrap()
        .health
        .current = 0;

    for _ in 0..900 {
        app.update();
        if matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Fighting { .. }
        ) {
            break;
        }
    }
    assert!(
        app.world().get::<BodyHealth>(seat_one).unwrap().current() > 0,
        "the round never restarted, so nothing here is under test"
    );
    // ONE tick, and the reason is the shape of the seam rather than a fudge.
    //
    // The reset happens in `CombatSet::Settle`, and the engine announces
    // restarts at the FRONT of the sim tick (`announce_body_restarts` in
    // `WorldPrep`) — so the trigger lands at the top of the following tick,
    // before `PlayerInput`, which is where provider systems read their own
    // state. The guarantee is "cleared before the provider acts again", not
    // "cleared in the same phase that reset the body", and this is what that
    // costs to observe from outside.
    app.update();
    assert_eq!(
        app.world().get::<ProviderCharge>(seat_one).unwrap().0,
        0.0,
        "the next round began with the fighter's provider-owned state intact — \
         a charge stored in the round that ended is a free launch in the round \
         that starts"
    );
}

/// The health readout is a GAUGE, and it tracks damage.
///
/// The declared HUD published strings and nothing else, so a health readout
/// could only ever be "47/60". A number is precise; a bar is readable, and in a
/// fight what a player needs at a glance is "am I nearly dead".
///
/// Asserts the published FILL rather than any drawn pixels: what the stage owes
/// the HUD is a fraction, and how wide that gets drawn is presentation's
/// business — which is the whole reason the fraction is what crosses the seam.
#[test]
fn the_versus_health_readout_is_a_gauge_that_follows_damage() {
    use ambition_app::app::versus_rules::HEALTH_HUD_SLOTS;
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;
    let (seat_0_slot, seat_1_slot) = (HEALTH_HUD_SLOTS[0], HEALTH_HUD_SLOTS[1]);

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        if q.iter(world).count() == 2 {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    let fill = |app: &App, slot: &str| -> Option<f32> {
        app.world()
            .resource::<ambition_platformer2d::presentation::HudReadouts>()
            .get(&ambition_platformer2d::presentation::HudSlotId::from(slot))
            .and_then(|readout| readout.fill())
    };
    assert_eq!(
        fill(&app, seat_0_slot),
        Some(1.0),
        "the left fighter's health readout published no gauge, so the HUD can \
         only draw a number"
    );
    assert_eq!(fill(&app, seat_1_slot), Some(1.0));

    // Hurt seat 1 and watch its bar, and only its bar, move.
    let seat_one = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 1)
            .map(|(entity, _)| entity)
            .expect("two seats")
    };
    {
        let mut health = app.world_mut().get_mut::<BodyHealth>(seat_one).unwrap();
        let max = health.health.max;
        health.health.current = max / 4;
    }
    app.update();

    let hurt = fill(&app, seat_1_slot).expect("the gauge is still published");
    assert!(
        hurt > 0.0 && hurt < 0.5,
        "the hurt fighter's gauge reads {hurt}, which does not follow its health"
    );
    assert_eq!(
        fill(&app, seat_0_slot),
        Some(1.0),
        "hurting one fighter moved the other's bar"
    );
}

/// Every seated fighter is actually DRAWN.
///
/// view, a hurtbox, a moveset, health and a team — and no picture, and not even
/// the placeholder rectangle a body with unresolvable art is supposed to fall
/// back to.
///
/// The cause is named in the marker's own documentation: "the authored render
/// pass only spawns visuals for `spec.enemy_spawns`, and the dynamic pass only
/// for EncounterMob / reward chests, so a directly-staged actor would render
/// invisibly." A seated fighter is a directly-staged actor and seating did not
/// mark it as one.
///
/// The seat-0 fighter looked fine throughout, which is what hid it: seat 0 is
/// the adopted PRIMARY PLAYER and renders through the player path entirely, so
/// exactly half the cast was proof of nothing.
#[test]
fn every_seated_fighter_has_something_on_screen() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        if q.iter(world).count() == 2 {
            break;
        }
    }
    for _ in 0..60 {
        app.update();
    }

    // Seats that are SPAWNED bodies (not the adopted player) must each have a
    // feature visual. The adopted seat is excluded by construction — it carries
    // no `FeatureId` because it is the player.
    let world = app.world_mut();
    let mut seats = world.query::<(
        &MatchSeat,
        &ambition_platformer2d::actors::features::FeatureId,
    )>();
    let spawned: Vec<(usize, String)> = seats
        .iter(world)
        .map(|(seat, id)| (seat.0, id.0.clone()))
        .collect();
    assert!(
        !spawned.is_empty(),
        "no seat is a spawned body, so this is measuring nothing"
    );

    let mut visuals = world.query::<&ambition_platformer2d::render::rendering::FeatureVisual>();
    let drawn: Vec<String> = visuals.iter(world).map(|v| v.id.clone()).collect();
    for (seat, id) in &spawned {
        assert!(
            drawn.contains(id),
            "seat {seat} (`{id}`) has a body and NOTHING on screen. Drawn: \
             {drawn:?}. A fighter nobody can see is not a fighter — and the \
             placeholder rectangle that is supposed to cover unresolvable art \
             does not cover a body no render family ever claimed."
        );
    }
}

/// Four controllers make it a 2v2.
///
/// L17 proved a 2v2 works through seating; nothing a player could PICK offered
/// one, and `SlotControls` slots 2 and 3 had never carried a real device.
///
/// This is also the arrangement teams were built for. With four human fighters
/// `effective_faction` maps every one of them to `ActorFaction::Player`, so
/// faction distinguishes nobody and `MatchTeam` is the only thing deciding who
/// may hit whom — a 2v2 is not a bigger 1v1, it is the first arrangement where
/// the relation is load-bearing.
#[test]
fn four_controllers_make_versus_a_two_versus_two() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::combat::targeting::{damage_lands_between, FriendlyFire, MatchTeam};

    let mut app = versus_app();
    for _ in 0..4 {
        app.world_mut().spawn(Gamepad::default());
    }
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        if q.iter(world).count() == 4 {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    let world = app.world_mut();
    let mut q = world.query::<(
        Entity,
        &MatchSeat,
        &MatchTeam,
        &ambition_platformer2d::combat::components::ActorFaction,
    )>();
    let mut fighters: Vec<(
        Entity,
        usize,
        String,
        ambition_platformer2d::combat::components::ActorFaction,
    )> = q
        .iter(world)
        .map(|(entity, seat, team, faction)| (entity, seat.0, team.0.clone(), *faction))
        .collect();
    fighters.sort_by_key(|(_, seat, ..)| *seat);
    assert_eq!(
        fighters.len(),
        4,
        "four controllers seated {} fighters — the stage still only offers a 1v1",
        fighters.len()
    );

    // Partners stand on the same SIDE, which is what `seat_for`'s alternation
    // means: evens left, odds right. A 2v2 whose partners start opposite each
    // other is two 1v1s sharing a screen.
    assert_eq!(
        fighters
            .iter()
            .map(|(_, _, team, _)| team.as_str())
            .collect::<Vec<_>>(),
        vec!["blue", "red", "blue", "red"]
    );

    // And the relation actually decides. Every ordered pair.
    let no_ff = FriendlyFire { enabled: false };
    for (entity, seat, team, faction) in &fighters {
        for (other, other_seat, other_team, other_faction) in &fighters {
            if entity == other {
                continue;
            }
            assert_eq!(
                damage_lands_between(
                    *faction,
                    *other_faction,
                    Some(&MatchTeam::new(team.clone())),
                    Some(&MatchTeam::new(other_team.clone())),
                    no_ff,
                    None,
                    *other,
                ),
                team != other_team,
                "seat {seat} ({team}) vs seat {other_seat} ({other_team}): a hit \
                 must land across teams and never within one"
            );
        }
    }
}

/// Four pads, four bodies, each moving only its own.
///
/// The 1v1 has this end to end; the 2v2 asserted seating and the damage
/// relation and stopped short of four live devices. Slots 2 and 3 of
/// `SlotControls` had never carried one, so "four players" rested on the
/// two-player writer generalising — which is the kind of assumption that has
/// been wrong four times already this session.
#[test]
fn four_pads_each_move_their_own_fighter_and_nobody_else_s() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::engine_core::BodyKinematics;

    let mut app = versus_app();
    let pads: Vec<Entity> = (0..4)
        .map(|_| app.world_mut().spawn(Gamepad::default()).id())
        .collect();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        if q.iter(world).count() == 4 {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    let bodies = |app: &mut App| -> Vec<(usize, Entity)> {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        let mut v: Vec<(usize, Entity)> = q.iter(world).map(|(e, s)| (s.0, e)).collect();
        v.sort_by_key(|(seat, _)| *seat);
        v
    };
    let seated = bodies(&mut app);
    assert_eq!(seated.len(), 4, "the stage did not seat four");

    // The device order is connection order, so pad N drives seat N.
    let order = app
        .world()
        .resource::<ambition_platformer2d::input::LocalDeviceOrder>()
        .devices()
        .to_vec();
    assert_eq!(
        order, pads,
        "the pads were not assigned in connection order"
    );

    for (seat, body) in &seated {
        let before: Vec<f32> = seated
            .iter()
            .map(|(_, e)| app.world().get::<BodyKinematics>(*e).unwrap().pos.x)
            .collect();
        pad_set(&mut app, pads[*seat], GamepadButton::DPadRight, 1.0);
        // Long enough to walk a measurable distance. NOT a wait for the round to
        // go live — the round already is, and this is inside the loop that
        // presses each pad in turn.
        for _ in 0..30 {
            app.update();
        }
        pad_set(&mut app, pads[*seat], GamepadButton::DPadRight, 0.0);
        let after: Vec<f32> = seated
            .iter()
            .map(|(_, e)| app.world().get::<BodyKinematics>(*e).unwrap().pos.x)
            .collect();

        let moved = app.world().get::<BodyKinematics>(*body).unwrap().pos.x - before[*seat];
        assert!(
            moved > 1.0,
            "pad {seat} pressed right and seat {seat} did not move ({moved:.2}px) \
             — slots 2 and 3 have never carried a real device before this"
        );
        // Everybody else is either still or coasting from their own earlier
        // turn; nobody should be ACCELERATING on somebody else's input.
        for (other, _) in &seated {
            if other == seat {
                continue;
            }
            let drift = (after[*other] - before[*other]).abs();
            assert!(
                drift < moved * 0.5,
                "pad {seat} moved seat {other} by {drift:.2}px while moving its \
                 own by {moved:.2}px — two seats are reading one device"
            );
        }
        // Let the coast die down before the next seat's turn.
        for _ in 0..120 {
            app.update();
        }
    }
}

/// A 2v2 scoreboard shows four fighters, and the round goes to the other
/// TEAM.
///
/// Two defects met here, and neither is visible in a 1v1.
///
/// The scoring rule was `1 - loser.min(1)` — "the other index", which is the
/// other SIDE only when there are exactly two bodies. Seat 2 is on blue, so blue
/// going down clamped to 1 and awarded the round to index 0: blue. A fighter's
/// defeat scored for its own team.
///
/// The HUD declared two health slots and wrote every seat above zero into the
/// right-hand one, so seats 1, 2 and 3 overwrote each other and a four-player
/// match showed two bars — one of them displaying whichever body the query
/// happened to reach last, which is worse than showing nothing because it looks
/// like information.
#[test]
fn a_two_versus_two_shows_four_gauges_and_scores_by_team() {
    use ambition_app::app::versus_rules::HEALTH_HUD_SLOTS;
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;

    let mut app = versus_app();
    for _ in 0..4 {
        app.world_mut().spawn(Gamepad::default());
    }
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        if q.iter(world).count() == 4 {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    // FOUR gauges, four distinct fills. Reading the fills rather than the slot
    // count is the point: two slots would have left two of these `None`.
    let gauges: Vec<Option<f32>> = HEALTH_HUD_SLOTS
        .iter()
        .map(|slot| {
            app.world()
                .resource::<ambition_platformer2d::presentation::HudReadouts>()
                .get(&ambition_platformer2d::presentation::HudSlotId::from(*slot))
                .and_then(|readout| readout.fill())
        })
        .collect();
    assert_eq!(
        gauges,
        vec![Some(1.0), Some(1.0), Some(1.0), Some(1.0)],
        "four fighters are seated and the HUD published {} gauges — a fighter \
         with no bar cannot tell how close to losing they are",
        gauges.iter().filter(|fill| fill.is_some()).count()
    );

    // Now wipe out BLUE — seats 0 and 2 — and watch who is credited.
    let blue: Vec<Entity> = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .filter(|(_, seat)| seat.0 % 2 == 0)
            .map(|(entity, _)| entity)
            .collect()
    };
    assert_eq!(blue.len(), 2, "the blue team is not a pair");
    for fighter in &blue {
        app.world_mut()
            .get_mut::<BodyHealth>(*fighter)
            .unwrap()
            .health
            .current = 0;
    }
    app.update();
    app.update();

    let state = app.world().resource::<VersusMatch>();
    assert_eq!(
        state.wins("red"),
        1,
        "blue was wiped out and red was not awarded the round (score: {:?}). \
         Seat 2 is a blue fighter, and the old rule mapped its defeat back onto \
         index 0 — so blue scored for knocking itself out.",
        state.rounds_won
    );
    assert_eq!(
        state.wins("blue"),
        0,
        "the losing team scored: {:?}",
        state.rounds_won
    );
}

/// The last round's attacks do not follow the fighters into the next one.
///
/// A KO hold FREEZES the world; it does not empty it.
#[test]
fn a_round_boundary_leaves_the_last_rounds_attacks_behind() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::combat::moveset::MovePlayback;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        if q.iter(world).count() == 2 {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    let (seat_zero, seat_one) = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        let mut rows: Vec<(Entity, usize)> = q.iter(world).map(|(e, seat)| (e, seat.0)).collect();
        rows.sort_by_key(|(_, seat)| *seat);
        (rows[0].0, rows[1].0)
    };

    // The survivor is mid-smash when the KO lands, a shot is in the air, and it
    // is holding a BUFFERED attack — the class `transit_body` deliberately keeps
    // ("axis maneuver state … are time facts, not place facts"), which is true
    // of a teleport and false of a round boundary.
    app.world_mut()
        .get_mut::<ambition_platformer2d::engine_core::BodyActionBuffer>(seat_zero)
        .expect("a fighter carries the shared action buffer")
        .attack = 0.25;
    // The survivor is mid-smash when the KO lands, and a shot is in the air.
    let smash = ambition_app::app::versus_fighters::duelist_moveset(
        ambition_app::app::versus_fighters::DuelistNumbers {
            jab_damage: 2,
            smash_damage: 9,
            reach_px: 40.0,
            smash_windup_s: 0.25,
        },
    )
    .move_by_id("smash_forward")
    .expect("the archetype has a smash")
    .clone();
    app.world_mut()
        .entity_mut(seat_zero)
        .insert(MovePlayback::new(smash, 1.0));
    // FIRED, not hand-spawned. That was invisible while `begin_round` despawned
    // `With<LiveProjectile>`, because a marker was all the boundary looked at.
    app.world_mut().write_message(
        ambition_platformer2d::projectiles::ProjectileSpawnRequest::named(
            seat_zero,
            ambition_platformer2d::projectiles::InFlightProjectile {
                body: ambition_platformer2d::projectiles::ProjectileBody::from_spec(
                    ambition_platformer2d::projectiles::ProjectileSpec {
                        origin: ambition_platformer2d::engine_core::Vec2::new(400.0, 300.0),
                        direction: ambition_platformer2d::engine_core::Vec2::new(1.0, 0.0),
                        damage: 1,
                        speed: 200.0,
                        // Long enough that it cannot expire on its own inside the
                        // KO hold — an expiry would look exactly like the cull
                        // this test is about.
                        max_lifetime: 30.0,
                        half_extent: ambition_platformer2d::engine_core::Vec2::splat(4.0),
                        gravity: 0.0,
                        bounces: 0,
                        world_hit: ambition_platformer2d::projectiles::WorldHitPolicy::Bouncing,
                        charge_tier: 0,
                    },
                ),
            },
            ambition_platformer2d::projectiles::ProjectileKind::Fireball,
            ambition_platformer2d::projectiles::ProjectileStart::StepNextTick,
        ),
    );
    app.update();
    let shot = {
        let world = app.world_mut();
        let mut q = world
            .query_filtered::<Entity, With<ambition_platformer2d::projectiles::LiveProjectile>>();
        q.iter(world)
            .next()
            .expect("the projectile request materialized the shot this fixture fired")
    };

    app.world_mut()
        .get_mut::<BodyHealth>(seat_one)
        .unwrap()
        .health
        .current = 0;
    for _ in 0..6 {
        app.update();
    }
    assert!(
        matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Ko { .. }
        ),
        "the fixture never reached the KO hold, so nothing below is about a \
         round boundary"
    );

    for _ in 0..900 {
        app.update();
        if matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Fighting { .. }
        ) {
            break;
        }
    }
    assert!(
        matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Fighting { .. }
        ),
        "the KO hold never ended"
    );

    assert!(
        app.world().get::<MovePlayback>(seat_zero).is_none(),
        "round two began with a fighter still swinging round one's smash — the \
         KO froze the move, it did not end it"
    );
    assert!(
        app.world().get_entity(shot).is_err(),
        "a projectile from the previous round is still in the air at the start \
         of this one, and the fighter it is about to hit has not moved yet"
    );
    assert_eq!(
        app.world()
            .get::<ambition_platformer2d::engine_core::BodyActionBuffer>(seat_zero)
            .map(|buffer| buffer.attack),
        Some(0.0),
        "round two began with a buffered attack from round one still queued. A \
         round boundary is a RESET, not a teleport: `transit_body` keeps the \
         maneuver timers on purpose and `reset_body_clusters` is the verb that \
         means this body starts again."
    );
}

/// The SHIPPED host has render-only frames too, and nothing writes sim state
/// on them.
///
/// The engine-side sweep in `rollback_coverage` watches the RL-sim composition and says so in
/// its own docs — it cannot see `VersusMatch`, which the shell app registers. Reasoned once,
/// checked here.
///
/// `SimTick` standing still is the proof the sim really did not run — without it a frame that
/// quietly kept simulating would report a clean sweep.
#[test]
fn no_render_only_frame_of_the_shipped_host_writes_rollback_state() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        if q.iter(world).count() == 2 {
            break;
        }
    }
    settle_into_a_live_round(&mut app);
    // The match must actually be running, or this sweeps a stage that never
    // started and every resource is trivially unchanged.
    {
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        assert_eq!(
            q.iter(world).count(),
            2,
            "the versus stage never seated its fighters, so this would sweep an \
             idle host"
        );
    }
    let watched = crate::rollback_coverage::restored_resource_type_names(app.world());
    assert!(
        watched.iter().any(|name| name.ends_with("VersusMatch")),
        "the shell app's own rollback state is not in the watched set, so this \
         would miss the exact resource the sweep was written for"
    );

    // `Update` keeps running; the GGRS-hosted sim gets no step.
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.update();

    let tick_before = *app
        .world()
        .resource::<ambition_platformer2d::time::SimTick>();
    let baseline = app.world().read_change_tick();
    for _ in 0..4 {
        app.update();
    }
    let now = app.world().read_change_tick();
    assert_eq!(
        *app.world()
            .resource::<ambition_platformer2d::time::SimTick>(),
        tick_before,
        "the simulation kept stepping through the frames this test calls \
         render-only, so a clean result below would mean nothing"
    );

    let world = app.world();
    let mut written: Vec<String> = Vec::new();
    for (info, _) in world.iter_resources() {
        let name = info.name().to_string();
        if !watched.contains(&name) {
            continue;
        }
        let Some(ticks) = world.get_resource_change_ticks_by_id(info.id()) else {
            continue;
        };
        if ticks.is_changed(baseline, now) {
            written.push(name);
        }
    }
    written.sort();
    assert!(
        written.is_empty(),
        "these rollback-registered resources changed during frames in which the \
         shipped host's simulation did not step:\n  {}\n\n\
         A resimulation replays sim steps, not render frames, so whatever the \
         render frame contributed is lost and the restored value disagrees with \
         the one that was live. `VersusMatch` shipped exactly this way — properly \
         registered AND advanced by a system in `Update` — which is why this \
         sweep exists.",
        written.join("\n  ")
    );
}

/// A fighter could take a hit after losing.
///
/// Asserts on the CLOCK TARGET rather than the live scale, because the live scale ramps on purpose
/// (a KO decelerating into the card is the genre's own beat, and it is the same primitive hitstop
/// uses).
#[test]
fn the_freeze_is_requested_on_the_tick_the_knockout_lands() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        if q.iter(world).count() == 2 {
            break;
        }
    }
    settle_into_a_live_round(&mut app);
    let target = |app: &App| {
        app.world()
            .resource::<ambition_platformer2d::time::time_control::RequestedClockScale>()
            .sim_clock
    };
    assert!(
        target(&app) > 0.5,
        "the fight was already being asked to stop before anybody was knocked out"
    );

    let seat_one = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 1)
            .map(|(entity, _)| entity)
            .expect("two seats")
    };
    app.world_mut()
        .get_mut::<BodyHealth>(seat_one)
        .unwrap()
        .health
        .current = 0;

    // Step until the rules NOTICE. The freeze must have been asked for by the
    // end of that very tick — not one tick later.
    let mut noticed = false;
    for _ in 0..8 {
        app.update();
        if matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Ko { .. } | MatchPhase::Won { .. }
        ) {
            noticed = true;
            break;
        }
    }
    assert!(noticed, "the rules never noticed the knockout");
    assert_eq!(
        target(&app),
        0.0,
        "the round was decided and the simulation was still not asked to stop. \
         The next tick is a full-speed tick of input, attacks and damage over a \
         fight that is already over."
    );
}

/// Two defects in one counter. Route entry reset the match with `default()`,
/// which announced nothing, so the opening card never appeared at the start of a
/// match — only after a later round reset. And the number was derived by summing
/// WINS, so a draw (which scores for nobody) made round two announce itself as
/// round one.
#[test]
fn the_round_counter_counts_rounds_and_not_wins() {
    use ambition_app::app::versus_rules::{MatchPhase, VersusMatch, ANNOUNCE_HUD_SLOT};

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    let mut opened_with_a_card = false;
    for _ in 0..900 {
        app.update();
        if app
            .world()
            .resource::<ambition_platformer2d::presentation::HudReadouts>()
            .get(&ambition_platformer2d::presentation::HudSlotId::from(
                ANNOUNCE_HUD_SLOT,
            ))
            .is_some_and(|readout| readout.text().contains("ROUND 1"))
        {
            opened_with_a_card = true;
            break;
        }
    }
    assert!(
        opened_with_a_card,
        "the match opened with no round card at all — the documented \
         \"ROUND 1 — FIGHT\" beat never happened, because route entry reset the \
         match through the constructor that announces nothing"
    );

    // A DRAW: nobody scores, and the round still advances.
    {
        let mut state = app.world_mut().resource_mut::<VersusMatch>();
        state.phase = MatchPhase::Ko {
            winner: None,
            remaining_s: 0.01,
        };
    }
    for _ in 0..600 {
        app.update();
        if matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Fighting { .. }
        ) {
            break;
        }
    }
    let state = app.world().resource::<VersusMatch>();
    assert!(
        state.rounds_won.values().all(|wins| *wins == 0),
        "a draw scored for somebody: {:?}",
        state.rounds_won
    );
    assert_eq!(
        state.round, 2,
        "the round after a draw is still round {}. Rounds played and rounds WON \
         are different facts, and a counter derived from the win totals repeats \
         itself every time nobody scores.",
        state.round
    );
}

/// A fighter is hittable through what its AUTHOR said, and a committed smash
/// changes it.
///
/// That is a reasonable default and says nothing about what a fighter is doing: a smash that costs
/// nothing to whiff is a game where you always smash.
///
/// This asserts the two halves that make the seam real from a stage a stranger
/// plays: the seated fighter carries its provider's document at all (not the
/// sprite fallback), and the resolved volume CHANGES when the smash is out —
/// wider and leaning forward, which is what makes committing punishable.
#[test]
fn a_seated_fighter_is_damageable_through_its_authored_hurtbox() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::actors::character_runtime::{
        AuthoredHurtboxes, HurtboxSelection, ResolvedHurtboxes,
    };

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        if q.iter(world).count() == 2 {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    let seat_zero = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("two seats")
    };

    // The document reached the body. Without this the next assertion could pass
    // on the sprite-derived fallback and prove nothing about authoring.
    assert!(
        app.world().get::<AuthoredHurtboxes>(seat_zero).is_some(),
        "the seated fighter carries no authored hurtbox document, so it is still \
         damageable through a box derived from its art"
    );

    let standing = app
        .world()
        .get::<ResolvedHurtboxes>(seat_zero)
        .expect("hurtboxes resolve every tick")
        .clone();
    assert_ne!(
        standing.source,
        HurtboxSelection::Unauthored,
        "the resolver fell back to the sprite box for a character that authored \
         volumes — the document is being ignored"
    );
    let half_width = |resolved: &ResolvedHurtboxes| -> f32 {
        match resolved.volumes.first().map(|volume| &volume.shape) {
            Some(ambition_platformer2d::entity_catalog::VolumeShape::Rect {
                half_extents, ..
            }) => half_extents.0,
            other => panic!("expected an authored rect hurtbox, got {other:?}"),
        }
    };
    let standing_half = half_width(&standing);

    // Now force the smash and let its move clock run. The move id is the
    // duelists' own (`smash_forward`), so this is the authored override rather
    // than a pose profile.
    let smash = ambition_platformer2d::combat::moveset::MovePlayback::new(
        ambition_app::app::versus_fighters::duelist_moveset(
            ambition_app::app::versus_fighters::LONG_GUARD,
        )
        .moves
        .iter()
        .find(|spec| spec.id == "smash_forward")
        .expect("the duelists author a forward smash")
        .clone(),
        1.0,
    );
    app.world_mut().entity_mut(seat_zero).insert(smash);
    app.update();

    let committed = app
        .world()
        .get::<ResolvedHurtboxes>(seat_zero)
        .expect("hurtboxes resolve every tick")
        .clone();
    assert_eq!(
        committed.source,
        HurtboxSelection::MoveOverride,
        "the smash is out and the resolver is still using {:?} — the per-move \
         timeline is the whole reason a hurtbox document has one",
        committed.source
    );
    assert!(
        half_width(&committed) > standing_half,
        "the committed smash did not widen the fighter's hurtbox ({} vs {}), so \
         whiffing it costs nothing and the safe option is never worth taking",
        half_width(&committed),
        standing_half
    );
}

/// The round-start countdown is a simulation phase, not a card.
///
/// That is a defensible presentation choice and it has one property no fighting game accepts: the
/// two players do not start equal, because one of them is reading the banner.
///
/// So this asserts the three things that make it a phase rather than a graphic:
///
/// 1. a fresh round is `Starting`, not `Fighting`;
/// 2. a controller held down through the whole count moves the fighter NOWHERE;
/// 3. the round goes live on its own, and the same input then works.
///
/// (2) is the one that matters. Every earlier version of this feature would have
/// passed (1) and (3) while the fight ran underneath.
#[test]
fn a_round_opens_on_a_countdown_that_nobody_can_act_through() {
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_platformer2d::engine_core::BodyKinematics;

    let mut app = versus_app();
    let pad = app.world_mut().spawn(Gamepad::default()).id();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    // Deliberately NOT `settle_into_a_live_round` — the count is the subject.
    for _ in 0..30 {
        app.update();
    }

    let world = app.world_mut();
    let mut drivers = world.query::<(Entity, &DrivingParticipant)>();
    let body = drivers
        .iter(world)
        .find_map(|(entity, driver)| (driver.0 == PlayerSlot(0)).then_some(entity))
        .expect("the versus stage seats a human in slot zero");

    assert!(
        matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Starting { .. }
        ),
        "round one went live immediately: {:?}",
        app.world().resource::<VersusMatch>().phase
    );

    // Hold right for the whole count, the way somebody impatient would.
    let start_x = app.world().get::<BodyKinematics>(body).unwrap().pos.x;
    pad_set(&mut app, pad, GamepadButton::DPadRight, 1.0);
    let mut ticks_counted = 0;
    for _ in 0..600 {
        app.update();
        if matches!(
            app.world().resource::<VersusMatch>().phase,
            MatchPhase::Fighting
        ) {
            break;
        }
        ticks_counted += 1;
        let drift = (app.world().get::<BodyKinematics>(body).unwrap().pos.x - start_x).abs();
        assert!(
            drift < 1.0,
            "the fighter moved {drift:.2}px during the countdown — the count is a \
             card over a live fight, which is the defect it replaced"
        );
    }
    assert!(
        ticks_counted > 30,
        "the countdown lasted {ticks_counted} ticks; that is not long enough to \
         be a countdown, so something is ending it early"
    );

    // And the SAME held input works the moment the round is live. A suppression
    // that never lifts is not a countdown, it is a soft lock — and holding the
    // button across the boundary is the case that catches it.
    let live_x = app.world().get::<BodyKinematics>(body).unwrap().pos.x;
    for _ in 0..30 {
        app.update();
    }
    let moved = app.world().get::<BodyKinematics>(body).unwrap().pos.x - live_x;
    assert!(
        moved > 1.0,
        "the round went live and the held input still did nothing ({moved:.2}px): \
         the controls were taken away and never given back"
    );
}

/// A fighter knocked beyond the stage blast zone loses the round.
/// Seat 1 exercises the actor path used by additional fighters rather than the
/// adopted primary-player reset path.
#[test]
fn a_fighter_knocked_off_the_stage_loses_the_round() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::engine_core::BodyKinematics;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    let seat_one = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 1)
            .map(|(entity, _)| entity)
            .expect("the stage seats an opponent")
    };
    assert!(
        app.world().get::<BodyHealth>(seat_one).unwrap().current() > 0,
        "seat 1 is already down before the round started; this measures nothing"
    );

    // Throw it off. Not a teleport past the margin — a real LAUNCH, straight
    // down at speed, so the fall is the thing under test and the gate has to
    // catch a body that arrived under its own physics. The stage's blast margin
    // is 96px past a 540px-tall world, so this has real distance to cover.
    {
        let mut kin = app.world_mut().get_mut::<BodyKinematics>(seat_one).unwrap();
        kin.pos.x = 60.0; // clear of the main platform, over open air
        kin.vel = ambition_platformer2d::engine_core::Vec2::new(0.0, 900.0);
    }

    let mut fell_out = false;
    for _ in 0..240 {
        app.update();
        if app.world().get::<BodyHealth>(seat_one).unwrap().current() <= 0 {
            fell_out = true;
            break;
        }
    }
    let resting = app.world().get::<BodyKinematics>(seat_one).unwrap().pos;
    assert!(
        fell_out,
        "seat 1 was launched into open air below the stage and is still standing \
         after four seconds, at {resting:?} — it is falling forever, which is what \
         an out-of-bounds gate nobody reads looks like from the outside"
    );

    // And the round rule, which already scored on health, scores this.
    let state = app.world().resource::<VersusMatch>();
    assert!(
        state.wins("blue") >= 1,
        "seat 1 fell out of the world and the blue team was not awarded the \
         round: the KO reached the body but not the rules"
    );
}

/// Thrown off the SIDE is a loss, not a suggestion.
///
/// The out-of-bounds gate measured distance past the world along the fall
/// direction only, so a fighter launched horizontally off the stage died only
/// once their arc happened to carry them below it — which reads as the throw
/// not having worked, and which is where a platform fighter actually loses most
/// of its stocks.
///
/// The sides are opt-in engine-wide (a platformer walking off the left edge of
/// a corridor is a room transition, not a death), so this also pins that THIS
/// stage opted in.
#[test]
fn a_fighter_thrown_off_the_side_loses_the_round() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::engine_core::BodyKinematics;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    let seat_one = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 1)
            .map(|(entity, _)| entity)
            .expect("the stage seats an opponent")
    };
    assert!(app.world().get::<BodyHealth>(seat_one).unwrap().current() > 0);

    // Parked 180px past the right edge of a 960px stage — beyond the 160px side
    // margin — and HIGH INSIDE the world vertically, at rest.
    //
    // The vertical placement is the whole test. The red probe caught that, which is the only
    // reason this comment exists.
    //
    // From y=300, reaching the floor blast line (540 + 96) takes ~0.68s, or
    // about 41 ticks. The budget below is 10. Gravity cannot reach the answer
    // inside it, so a death here is the SIDE zone or it is nothing.
    {
        let mut kin = app.world_mut().get_mut::<BodyKinematics>(seat_one).unwrap();
        kin.pos = ambition_platformer2d::engine_core::Vec2::new(1140.0, 300.0);
        kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
    }

    let mut died = false;
    for _ in 0..10 {
        app.update();
        if app.world().get::<BodyHealth>(seat_one).unwrap().current() <= 0 {
            died = true;
            break;
        }
    }
    let resting = app.world().get::<BodyKinematics>(seat_one).unwrap().pos;
    assert!(
        died,
        "seat 1 sat 180px off the right of a 960px-wide stage and was still \
         standing ten ticks later, at {resting:?}. The side blast zone is not \
         armed, so the only way to score a knock-off is to hope the victim's \
         arc eventually drops them below the stage."
    );
    assert!(
        resting.y < 540.0,
        "the body died at {resting:?}, already below the world — that is the \
         FLOOR blast zone answering, and this case is supposed to be unable to \
         reach it"
    );

    let state = app.world().resource::<VersusMatch>();
    assert!(
        state.wins("blue") >= 1,
        "seat 1 left the stage sideways and the blue team was not awarded the \
         round: the KO reached the body but not the rules"
    );
}

/// A KO makes a sound.
///
/// Every fighter in a versus round carries `RulesetOwnsDeath`, which means EVERY versus KO was
/// silent, and a KO is the whole payoff of the genre.
///
/// The arm's own comment lists what an arena must not have — bounty coin,
/// heart, death explosion, split offspring, held-item drop, respawn timer — and
/// it is all ECONOMY. A body dying in its own voice was never on that list.
#[test]
fn a_knockout_is_announced_in_the_losers_own_voice() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::engine_core::BodyKinematics;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    let seat_one = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 1)
            .map(|(entity, _)| entity)
            .expect("the stage seats an opponent")
    };

    // Off the right of the stage, past the 160px side margin, high inside the
    // world so only the side zone can score it.
    {
        let mut kin = app.world_mut().get_mut::<BodyKinematics>(seat_one).unwrap();
        kin.pos = ambition_platformer2d::engine_core::Vec2::new(1140.0, 300.0);
        kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
    }

    let mut heard_death = false;
    for _ in 0..10 {
        app.update();
        let world = app.world_mut();
        let mut cues =
            world.resource_mut::<bevy::ecs::message::Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>();
        let mut reader = cues.get_cursor();
        if reader.read(&cues).any(|cue| {
            matches!(
                cue.request,
                ambition_platformer2d::sfx::SfxMessage::Death { .. }
            )
        }) {
            heard_death = true;
        }
        cues.update();
        if heard_death {
            break;
        }
    }
    assert!(
        app.world().get::<BodyHealth>(seat_one).unwrap().current() <= 0,
        "fixture: seat 1 must actually be knocked out, or this measures silence \
         from a fighter that never died"
    );
    assert!(
        heard_death,
        "seat 1 was knocked out of the stage and NOTHING was heard. A round that \
         ends with no sound is a round nobody notices."
    );

    // SEAT 0 TOO, ON ITS OWN STAGE.
    //
    // Settling into the NEXT round does not rescue it either (probed: phase `Fighting`, not
    // scripted, still no damage), so the confound is the round transition itself and the fix is
    // a fresh stage rather than a longer wait.
    //
    // and what this half proves is smaller than its old comment claimed. There is no other path
    // now; both seats are actors wearing characters, which is the landing working. What is
    // still worth pinning is that the LOCAL seat's knockout is announced, because it is the one
    // a person hears about themselves.
    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        if rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID)
        {
            break;
        }
    }
    settle_into_a_live_round(&mut app);
    let seat_zero = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the stage seats a fighter in slot zero")
    };
    let where_it_stood = app.world().get::<BodyKinematics>(seat_zero).unwrap().pos;
    let hp = app.world().get::<BodyHealth>(seat_zero).unwrap().current();
    let volume: ambition_platformer2d::engine_core::CombatVolume =
        ambition_platformer2d::engine_core::Aabb::new(
            where_it_stood,
            ambition_platformer2d::engine_core::Vec2::new(40.0, 40.0),
        )
        .into();
    app.world_mut()
        .write_message(ambition_platformer2d::combat::events::HitEvent {
            strike_sfx: None,
            volume,
            damage: hp + 10,
            source: ambition_platformer2d::combat::events::HitSource::Melee,
            attacker: None,
            // the two consumers are a surviving fork and this test is not
            // the place to remove it. `HitTarget::Body` / `HitTarget::Body`
            // are documented as a deliberate split — the relational
            // actor-vs-actor path exists so an Enemy-faction body can damage a
            // Boss-faction one without the bipartite assumption — but "which
            // variant names a body" is now decided by how that body was
            // CONSTRUCTED, which is exactly the coupling the rest of this
            // landing deleted. Named here so the next reader finds it on
            // purpose rather than by having a fixture stop landing hits.
            target: ambition_platformer2d::combat::events::HitTarget::Body(seat_zero),
            mode: ambition_platformer2d::combat::events::HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });

    let mut heard_seat_zero = false;
    for _ in 0..8 {
        app.update();
        let world = app.world_mut();
        let mut cues =
            world.resource_mut::<bevy::ecs::message::Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>();
        let mut reader = cues.get_cursor();
        if reader.read(&cues).any(|cue| {
            matches!(
                cue.request,
                ambition_platformer2d::sfx::SfxMessage::Death { .. }
            )
        }) {
            heard_seat_zero = true;
        }
        cues.update();
        if heard_seat_zero {
            break;
        }
    }
    assert!(
        app.world().get::<BodyHealth>(seat_zero).unwrap().current() <= 0,
        "fixture: seat 0 must actually be down, or this measures silence from a \
         fighter that never died"
    );
    assert!(
        heard_seat_zero,
        "seat 0 lost the round and NOTHING was heard. The ruleset owns where the \
         body goes and who scores it — not the sound of losing."
    );
}

/// A round boundary culls what the round created, without naming it. (3A)
///
/// Every transient family added afterwards — a strike volume, a summon, a lingering hitbox — needed
/// another query in that function, and forgetting one fails silently: the entity is simply still
/// there in a round that never asked for it.
///
/// This asserts the boundary works on a family the RULES never mention: the
/// entity below is round-scoped and nothing else, so the only thing that can
/// despawn it is the scope itself.
#[test]
fn a_round_boundary_culls_round_scoped_entities_the_rules_never_name() {
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::platformer::lifecycle::{ActiveRoundScope, RoundScopedEntity};

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    settle_into_a_live_round(&mut app);

    let round = app
        .world()
        .resource::<ActiveRoundScope>()
        .current()
        .expect("a live round has a round scope");
    // Not a projectile, not anything the versus rules have heard of.
    let debris = app.world_mut().spawn(RoundScopedEntity(round)).id();
    app.update();
    assert!(
        app.world().get_entity(debris).is_ok(),
        "the round is still live; nothing should have been culled yet"
    );

    // Force the round over by emptying a fighter, and let the KO hold elapse.
    {
        let world = app.world_mut();
        let mut q = world.query::<(
            &ambition_platformer2d::actors::character_runtime::MatchSeat,
            &mut BodyHealth,
        )>();
        for (seat, mut health) in q.iter_mut(world) {
            if seat.0 == 1 {
                health.health.current = 0;
            }
        }
    }
    for _ in 0..600 {
        app.update();
        if app.world().get_entity(debris).is_err() {
            break;
        }
    }

    assert!(
        app.world().get_entity(debris).is_err(),
        "a round-scoped entity outlived its round. The boundary is back to \
         enumerating families, and the one it does not name is the one that leaks"
    );
}

/// The disagreement path is REAL, and nothing exercised it.
///
/// `reconcile_roster_with_frozen_topology` has two arms once a match is seated: correct the
/// paperwork when the frozen topology would build the SAME fighters, and refuse to touch
/// anything when it would not. That is the shape `tracks.md` calls out about K2b: *"the
/// coverage is all implicit, which is exactly what makes risk 1 dangerous."*
///
/// This drives a real seated round, then makes the live roster name fighters the
/// frozen topology would never build, and asserts the reconciler leaves BOTH the
/// participants and the stale stamp alone. A repair here would be a silent
/// reseat of bodies already fighting.
///
/// it asserts the stamp stays STALE on purpose. That is the visible
/// difference between the two arms: agreement updates it, disagreement must not,
/// because updating it would claim the session and the roster agree.
#[test]
fn a_roster_that_disagrees_with_the_frozen_topology_is_left_alone() {
    use ambition_platformer2d::actor::MatchParticipantRoster;

    let mut app = versus_app();
    settle_to_launcher(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(VERSUS_GAMEPLAY_ROUTE)));
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::runtime::demo_fixture::RoomSet>();
        let seated = rooms
            .iter(world)
            .next()
            .is_some_and(|set| set.active_spec().id == VERSUS_ROOM_ID);
        if seated {
            break;
        }
    }
    settle_into_a_live_round(&mut app);

    // The match is live. Now make the roster describe a DIFFERENT fight than the
    // frozen topology would build, exactly as a mid-session topology change
    // would, and clear the stamp so the reconciler re-examines it.
    let (before, stamped_generation) = {
        let mut roster = app.world_mut().resource_mut::<MatchParticipantRoster>();
        for participant in &mut roster.participants {
            participant.character = format!("{}_impostor", participant.character).into();
        }
        // `activate(None)`, not `Proposed`. The match is LIVE — these
        // bodies are on the stage — so the roster stays activated and only the
        // record of which topology decided it is cleared. Making it `Proposed`
        // would describe a match nobody has agreed to while its fighters are
        // mid-round, which is not a state the route can produce.
        roster.activate(None);
        (roster.participants.clone(), roster.seat_topology())
    };
    assert!(
        stamped_generation.is_none(),
        "the fixture failed to clear the stamp, so the reconciler will return early \
         and this test would pass without reaching either arm"
    );

    for _ in 0..30 {
        app.update();
    }

    let roster = app.world().resource::<MatchParticipantRoster>();
    assert_eq!(
        roster.participants, before,
        "the reconciler REBUILT a roster whose fighters disagree with the live \
         match — that reseats bodies already fighting, which its own comment calls \
         the worse bug"
    );
    assert!(
        roster.seat_topology().is_none(),
        "the disagreement was stamped as agreed. The stamp is the difference \
         between the two arms: correcting it claims the session and the roster \
         describe the same match, and here they do not"
    );
}

/// A CEILING NARROWS A KIT; IT CANNOT INVENT ONE.
///
/// ⭐⭐ THE DUEL DECLARES `MatchAbilities::at_most(VERSUS_FIGHTER_KIT)`, which
/// grants nothing and permits that kit — `(authored ∪ NONE) ∩ kit`. That is the
/// right operator and it has a silent edge: a character that authored NO
/// abilities intersects to `AbilitySet::NONE` and arrives on the stage unable to
/// move, attack or jump. Nothing refuses it, because "this fighter may do
/// nothing" is a legal thing for a ceiling to conclude.
///
/// ⛔ SO THE GUARD IS ON THE CAST, NOT ON THE OPERATOR. Both duellists dress
/// themselves (`with_abilities(VERSUS_FIGHTER_KIT)`), and this asserts that the
/// day somebody seats a third character here it is dressed too — which is the
/// only moment the edge above is reachable.
#[test]
fn every_fighter_the_duel_can_seat_authors_the_abilities_its_ceiling_narrows() {
    use ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry;

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    // The registry is filled by a `Startup` system, so a build that has never
    // updated has no registered characters at all.
    app.update();

    let roster = ambition_app::app::versus::versus_roster(2);
    assert!(
        roster.participants.len() >= 2,
        "the duel seated {} fighters, so this census cannot fail",
        roster.participants.len()
    );

    let registry = app.world().resource::<PreparedCharacterRegistry>();
    let undressed: Vec<String> = roster
        .participants
        .iter()
        .filter(|seat| {
            registry
                .get(seat.character.as_str())
                .is_none_or(|character| character.abilities.is_none())
        })
        .map(|seat| seat.character.as_str().to_string())
        .collect();
    assert!(
        undressed.is_empty(),
        "these duel fighters author no abilities: {undressed:?}. The stage's \
         ceiling is `at_most`, which GRANTS nothing — so an undressed fighter \
         intersects to `AbilitySet::NONE` and stands on the stage unable to \
         move, jump or attack, with nothing anywhere refusing it"
    );
}
