#![cfg(feature = "input")]
//! **Smash, from the title screen — the whole way in and the whole way back.**
//!
//! The demo has had a stage, a ruleset and a select screen for a day, and until
//! now the only composition that could reach them was its own app. Listing it in
//! the multi-game host is what makes the crossover claim its own source comment
//! makes — *"the crossover claim moves to where it belongs — Ambition HOSTING
//! this experience alongside its own"* — a thing that runs rather than a thing
//! that is argued.
//!
//! Gated on the `input` feature: the presses here are REAL keys through the real
//! host input stack, which is where three of the four playtest defects lived.
//!
//! It also exercises the one shape no other provider has: a launcher row that
//! opens a QUESTION. Every other entry activates its gameplay route directly;
//! this one opens character select, which is a frontend route of the provider's
//! own, and the stage arrives only once the screen has decided.

use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;

use ambition_platformer2d::game_shell::{ShellCommand, ShellLauncherCommand, ShellRouter};
use ambition_app::app::shell_host;
use ambition_demo_smash::select::{SeatSelection, SmashSelect};
use leafwing_input_manager::prelude::Buttonlike;

/// The real shell-host composition PLUS the real host input stack, headless —
/// the same shape `participant_input.rs` uses.
///
/// ⚠ **the input stack is not optional here, it is the point.** A test that
/// wrote `SeatMenuFrames` by hand would be testing a resource the host REBUILDS
/// from its participants every frame: the select screen's whole complaint list
/// ("Start does not add a CPU", "there is no start on a keyboard") lived in the
/// span between a key and that resource, which hand-set frames skip over.
fn shell_host_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::platformer::schedule::GameMode>();
    app.insert_resource(shell_host::AmbitionShellHosted);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    app.add_plugins(ambition_platformer2d::host::PlatformerHostPlugins);
    shell_host::compose_ambition_shell_host(&mut app);
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

fn active_route(app: &App) -> Option<String> {
    app.world()
        .resource::<ShellRouter>()
        .active
        .as_ref()
        .map(|active| active.route_id.as_str().to_owned())
}

/// Move the launcher cursor onto the row labelled `label` and confirm it.
/// Derived from the catalog rather than an index: a literal silently becomes a
/// different game the day somebody registers a provider before this one.
fn launch_row(app: &mut App, label: &str) {
    let index = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellLaunchCatalog>()
        .entries
        .iter()
        .position(|entry| entry.label == label)
        .unwrap_or_else(|| panic!("no launcher row labelled {label:?}"));
    app.world_mut()
        .write_message(ShellLauncherCommand::Activate(index));
    settle(app);
}

/// Tap a key on the KEYBOARD and release it. The screen reads edges, so a held
/// key is not a second press.
fn tap(app: &mut App, key: KeyCode) {
    Buttonlike::press(&key, app.world_mut());
    app.update();
    Buttonlike::release(&key, app.world_mut());
    app.update();
}

/// Sit down / lock in.
fn confirm(app: &mut App) {
    tap(app, KeyCode::Enter);
}

/// Add a CPU to the lowest empty seat. DOWN, not Start: on a keyboard
/// `Platformer2dInputActionMonolith::Start` is Escape, which belongs to the pause menu — Jon,
/// 2026-07-31: *"there is no start on a keyboard."*
fn add_cpu(app: &mut App) {
    tap(app, KeyCode::ArrowDown);
}

#[test]
fn the_title_screen_opens_character_select_and_the_screen_starts_the_match() {
    let mut app = shell_host_app();
    settle(&mut app);

    launch_row(&mut app, "Smash");
    assert_eq!(
        active_route(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_SELECT_ROUTE),
        "the Smash row opens the select screen; landing on the stage would mean \
         the host chose the fighters"
    );
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
            .is_none(),
        "nothing has been decided yet"
    );

    // One person, one keyboard: add a CPU, sit down, lock in.
    add_cpu(&mut app);
    assert_eq!(
        app.world().resource::<SmashSelect>().seat(1),
        SeatSelection::Cpu { character: 1 },
        "a seat no controller reaches has to be able to become a CPU"
    );
    confirm(&mut app);
    confirm(&mut app);
    settle(&mut app);

    let roster = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .expect("the screen decided a match and published it")
        .clone();
    assert_eq!(roster.participants.len(), 2);

    // The stage. Preparation and activation are the SAME lifecycle every other
    // provider rides — the select screen only asked for the route.
    for _ in 0..40 {
        app.update();
        if active_route(&app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
            break;
        }
    }
    assert_eq!(
        active_route(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        "the decided match never reached the stage"
    );
    assert!(
        app.world()
            .resource::<ambition_platformer2d::game_shell::ActiveGameplaySession>()
            .0
            .is_some(),
        "the stage route activated no gameplay session"
    );

    // **THE WAY OUT.** Ambition's own rooms have the kaleidoscope pause menu, so
    // the host suppresses the universal one while they are live; a demo's mode
    // must NOT suppress it, or a player who entered from the title screen has no
    // Quit to Title and no way back to the launcher at all.
    assert!(
        !app.world()
            .resource::<ambition_platformer2d::game_shell::ShellPauseMenuSuppressed>()
            .0,
        "the universal pause menu is suppressed during a Smash match, so the row \
         that leaves it does not exist"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(
        active_route(&app).as_deref(),
        Some(shell_host::AMBITION_LAUNCHER_ROUTE),
        "quitting a hosted demo returns to the host's title screen"
    );
}

/// **A rematch has to be possible.**
///
/// The roster the select screen publishes is an ordinary resource that outlives
/// the session, and every seat stayed locked in — so coming back to the screen
/// found it already decided and already spent: nothing to press, nowhere to go.
/// Only a host that can return here makes that reachable, which is why it lands
/// with the launcher row.
#[test]
fn coming_back_to_the_select_screen_offers_a_fresh_match() {
    let mut app = shell_host_app();
    settle(&mut app);

    launch_row(&mut app, "Smash");
    add_cpu(&mut app);
    confirm(&mut app);
    confirm(&mut app);
    for _ in 0..40 {
        app.update();
        if active_route(&app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
            break;
        }
    }

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    launch_row(&mut app, "Smash");
    settle(&mut app);

    assert_eq!(
        active_route(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_SELECT_ROUTE)
    );
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
            .is_none(),
        "the previous match's roster is still standing, so the screen believes a \
         match is already under way and will never start another"
    );
    assert_eq!(
        app.world().resource::<SmashSelect>().seat(0),
        SeatSelection::Empty,
        "every seat is still locked in from last time, so nothing can be pressed"
    );

    // And it really does start again.
    add_cpu(&mut app);
    confirm(&mut app);
    confirm(&mut app);
    settle(&mut app);
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
            .is_some(),
        "a second match could not be decided"
    );
}

/// **THE TWO-PARTICIPANT FLOW, to its end: select → lock in → match → PAUSE.**
///
/// The select half is covered above. This is the tail, and it exists because
/// the tail was BROKEN: `populate_menu_control_frame_from_actions` folded
/// participants with `single()`, which returns `Err` the moment a second one
/// exists, so the global `MenuControlFrame` went neutral for everybody and
/// pressing Start opened nothing. Two people could start a match together and
/// then not pause it. (GPT 5.6 review, finding 4.)
///
/// ⚠ **it presses a KEY, not `MenuControlFrame`.** Setting the frame by hand is
/// what the select screen's tests originally did, and it is exactly how that
/// screen came to be fully tested and completely inert — an injected value skips
/// the system that was broken. The press has to enter where a real one enters.
#[test]
fn two_participants_start_a_match_and_can_still_pause_it() {
    use ambition_platformer2d::input::InputParticipant;

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");

    // A SECOND participant, which is the condition that used to silence the
    // global menu frame.
    //
    // ⚠ SPAWN A PAD, do not spawn the participant. The host derives its seats
    // from live `Gamepad` entities every frame and despawns any it did not
    // declare, so a hand-spawned `InputParticipant` is gone by the next update —
    // the same "the resource is derived, the pads are the fact" trap the select
    // screen's own tests hit.
    // A pad alone is not a seat: the host seats participants from the DECLARED
    // seat count, so a lobby that never opened has one seat however many pads
    // are plugged in. Both facts are needed.
    // ⚠ SPAWN PADS. The select screen declares its seat count FROM the live
    // pads and the host derives participants from that declaration, so an
    // inserted `DeclaredInputSeats` is clobbered on the next frame — the pads
    // are the fact, the same trap the select screen's own tests carry a note
    // about.
    for _ in 0..2 {
        app.world_mut().spawn(bevy::input::gamepad::Gamepad::default());
    }
    settle(&mut app);
    settle(&mut app);
    let seated = {
        let world = app.world_mut();
        let mut q = world.query::<&InputParticipant>();
        q.iter(world).count()
    };
    assert!(
        seated >= 2,
        "two participants have to actually exist or this proves nothing: {seated}"
    );

    assert!(
        !app.world().resource::<ambition_platformer2d::game_shell::ShellPauseMenu>().open,
        "nothing is paused before anybody presses anything"
    );

    // Escape is `Platformer2dInputActionMonolith::Start` on a keyboard: the pause press.
    tap(&mut app, KeyCode::Escape);
    settle(&mut app);

    assert!(
        app.world().resource::<ambition_platformer2d::game_shell::ShellPauseMenu>().open,
        "with two participants seated, Escape must still reach the pause menu — a couch \
         game you can start together and cannot pause is the regression this pins"
    );
}

/// **PROBE (2026-08-01, Jon): "even when we add a CPU player in smash there is
/// only ever one player that shows up in game."**
///
/// ⛔ every existing test in this file stops at the ROUTE and the SESSION. None
/// of them counts the bodies that were seated, so a roster of two that puts one
/// fighter on the stage passes the whole suite — which is exactly the state Jon
/// is describing, and exactly why couch multiplayer cannot be checked: the thing
/// you would verify is the thing nothing asserts.
#[test]
fn a_two_participant_roster_actually_seats_two_bodies() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");

    add_cpu(&mut app);
    confirm(&mut app);
    confirm(&mut app);
    settle(&mut app);

    let declared = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .expect("the screen decided a match")
        .participants
        .len();
    assert_eq!(declared, 2, "the roster is the premise of this test");

    for _ in 0..40 {
        app.update();
        if active_route(&app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
            break;
        }
    }
    // Give activation room to seat everybody.
    for _ in 0..60 {
        app.update();
    }

    let seated = {
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        let mut seats: Vec<usize> = q.iter(world).map(|seat| seat.0).collect();
        seats.sort_unstable();
        seats
    };
    assert_eq!(
        seated.len(),
        declared,
        "the roster declared {declared} participants and {} bodies were seated: {seated:?}",
        seated.len()
    );

    // ⚠ SEATED IS NOT VISIBLE. Jon reports one player on screen; the seating is
    // fine, so the difference has to be downstream. Report what each seated body
    // carries rather than guessing which component the renderer wants.
    let report: Vec<String> = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        let rows: Vec<(Entity, usize)> = q.iter(world).map(|(e, s)| (e, s.0)).collect();
        rows.into_iter()
            .map(|(entity, seat)| {
                let mut names: Vec<String> = world
                    .inspect_entity(entity)
                    .expect("seated body exists")
                    .map(|info| info.name().shortname().to_string())
                    .collect();
                names.sort();
                format!("seat {seat}: {} components [{}]", names.len(), names.join(", "))
            })
            .collect()
    };
    for row in &report {
        eprintln!("[seat-census] {row}");
    }

    // WHO each seat actually IS. The roster names them `Duelist A`/`Duelist B`;
    // this says whether the BODY agrees, which is a different question and the
    // one the touch bezel answers by showing the subject's verbs.
    {
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            Option<&ambition_platformer2d::characters::actor::WornCharacter>,
            Option<&ambition_platformer2d::engine_core::BodyAbilities>,
            &Name,
        )>();
        let mut rows: Vec<String> = q
            .iter(world)
            .map(|(seat, worn, abilities, name)| {
                format!(
                    "seat {}: name={:?} worn={:?} abilities={:?}",
                    seat.0,
                    name.as_str(),
                    worn.map(|w| w.0.as_str()),
                    abilities.map(|a| format!("{:?}", a.abilities))
                )
            })
            .collect();
        rows.sort();
        for row in rows {
            eprintln!("[seat-identity] {row}");
        }
    }

    // ⚠ **THE TWO FIGHTERS ARE BUILT BY DIFFERENT PATHS**, and the census above
    // is how you see it: both carry 84 components and the SETS DIFFER. Seat 0 is
    // player-bodied (`PlayerVisual`, `BodyPoseView`, `PresentedPose`,
    // `Transform`, `GlobalTransform`); seat 1 is actor-bodied (`ActorIdentity`,
    // `Perception`, `RoomVisual`, `RuntimeStagedActor`) with no transform and no
    // pose view at all.
    //
    // ⛔ **this test deliberately does NOT assert that seat 1 has a
    // `BodyPoseView`.** That was the first draft and it is the WRONG PORT:
    // `BodyPoseView` is the player-bodied read model, and an actor-bodied
    // fighter is drawn through the id-keyed `ActorAnimIndex` instead — a
    // RESOURCE rebuilt in the render presentation plugin, explicitly "NOT the
    // sim schedule … so a headless / RL build never pays for poses it won't
    // draw". So a headless test CANNOT tell whether seat 1 reaches the screen,
    // and an assertion here would have been a confident measurement of the wrong
    // thing.
    //
    // That is also the answer to why Jon's report ("add a CPU and only one
    // player shows up") survived a green suite: the split is invisible to every
    // headless test by construction, and the only instrument that can settle it
    // is a photograph of the stage route.
    assert!(
        seated.len() == 2,
        "the census above is the useful output; this pins the premise"
    );
}


/// **An ADOPTED seat and a SPAWNED seat must agree on everything the ROSTER
/// declares.**
///
/// ⛔ Seating has had to unify this ONE FIELD AT A TIME, four times, each found by
/// looking at a picture rather than by a test:
///
/// * **health** — a spawned seat took the authored maximum; the adopted player
///   kept whatever its session established.
/// * **box** — a mirror match could put two different body shapes on the stage,
///   and the wrong one was always player one.
/// * **mass** — the same character weighed different amounts by seat.
/// * **abilities** (2026-08-01) — player one had fly, blink and
///   blink-through-walls; player two had jump and attack.
///
/// The shape never changes: *an adopted body keeps what the session gave it*. A
/// fifth field is a matter of time, so this asserts the RULE rather than the
/// four instances.
///
/// ⚠ scoped to what the ROSTER declares, deliberately. Per-CHARACTER differences
/// are the point of a fighting game — the versus duelists author 60 and 52 health
/// as a deliberate trade — so "both seats are identical" would be the wrong
/// assertion. What a match DECLARES applies to every seat in it; what a character
/// authors does not.
#[test]
fn an_adopted_seat_and_a_spawned_seat_agree_on_every_declared_field() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    add_cpu(&mut app);
    confirm(&mut app);
    confirm(&mut app);
    settle(&mut app);
    for _ in 0..40 {
        app.update();
        if active_route(&app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
            break;
        }
    }
    for _ in 0..60 {
        app.update();
    }

    let rows: Vec<(usize, String, Option<u32>)> = {
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            &ambition_platformer2d::engine_core::BodyAbilities,
            Option<&ambition_platformer2d::actor::FighterStocks>,
        )>();
        let mut rows: Vec<(usize, String, Option<u32>)> = q
            .iter(world)
            .map(|(seat, abilities, stocks)| {
                (
                    seat.0,
                    format!("{:?}", abilities.abilities),
                    stocks.map(|s| s.started_with),
                )
            })
            .collect();
        rows.sort_by_key(|(seat, ..)| *seat);
        rows
    };
    assert!(
        rows.len() >= 2,
        "this test needs an adopted seat AND a spawned one; got {}",
        rows.len()
    );

    let (first_seat, first_abilities, first_stocks) = &rows[0];
    for (seat, abilities, stocks) in &rows[1..] {
        assert_eq!(
            abilities, first_abilities,
            "seat {seat} and seat {first_seat} do not have the same verbs, and the \
             match declared one set for everybody. An adopted seat keeps what the \
             session gave it unless the roster overrides — that is the bug this \
             pins, found four times in four different fields."
        );
        assert_eq!(
            stocks, first_stocks,
            "seat {seat} and seat {first_seat} started with different stocks"
        );
    }
}


