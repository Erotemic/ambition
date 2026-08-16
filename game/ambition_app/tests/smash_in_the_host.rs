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

use ambition_app::app::shell_host;
use ambition_demo_smash::select::{SlotOccupant, SmashSelect};
use ambition_demo_smash::select_screen::cursor::SelectCursor;
use ambition_demo_smash::select_screen::layout::SelectLayout;
use ambition_platformer2d::game_shell::{ShellCommand, ShellLauncherCommand, ShellRouter};
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
    // ⛔ **PINNED, because `app.update()` is otherwise a unit of WALL CLOCK.**
    //
    // Under Bevy's default `TimeUpdateStrategy::Automatic` the clock advances by
    // real elapsed time, so how much simulation a `settle()` covers depends on
    // how busy the machine is: almost none when idle, many fixed steps under
    // load. This module asserts DISTANCES two fighters moved, which is exactly
    // the count-like claim that turns into a coin flip.
    //
    // Measured 2026-08-03, not assumed: three full runs of `app_it` alone are
    // green, and TWO CONCURRENT full runs fail in BOTH processes with
    // "the keyboard moved the PAD player's fighter". Isolating
    // `AMBITION_DATA_DIR` per process did not change it, which rules out the
    // shared settings/save files this app reads at startup.
    //
    // ⚠ third occurrence of one defect. `shell_host_startup` pins for this
    // reason, `shell_host_rendered` was fixed for it (dev/journals/code_smells.md,
    // whose lesson is "a test that steps a real Bevy App and asserts anything
    // count-like or state-like MUST pin the timestep"), and this module was
    // written without it.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
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

/// Click whatever the cursor is over.
fn confirm(app: &mut App) {
    tap(app, KeyCode::Enter);
}

/// **Put the cursor on something and click it.**
///
/// The position is written the way a MOUSE writes it; the click is a real
/// keyboard Enter travelling the host's whole participant chain, which is what
/// this file exists to exercise. That every widget is also REACHABLE by arrows
/// alone is `the_screen_decides::the_arrows_alone_can_work_the_whole_screen`,
/// where a bare demo app makes it cheap to assert.
fn click(app: &mut App, rect: ambition_demo_smash::select_screen::cursor::HitRect) {
    app.world_mut()
        .resource_mut::<SelectCursor>()
        .move_to(rect.center());
    confirm(app);
}

/// The screen's own geometry — the same rectangles it draws.
///
/// ⚠ from the HOST's assembled roster. In this composition the grid is the
/// crossover cast (every character tagged `smash`, plus the demo's own four),
/// not the four a standalone demo offers — and a layout built from the wrong
/// count puts every click one cell off.
fn screen(app: &App) -> SelectLayout {
    SelectLayout::for_viewport(
        None,
        app.world()
            .resource::<ambition_demo_smash::select::SmashRoster>()
            // ⚠ CELLS, not fighters: the grid's last square is RANDOM, and a
            // layout built from the fighter count puts every click one cell off
            // at the end of the last row.
            .cell_count(),
    )
}

/// **One person at a keyboard, against one CPU, from the buttons.**
///
/// Slot 1 takes the only source; slot 2 has none left, so its button skips
/// straight to CPU. Then a fighter each, then START.
fn decide_a_solo_match(app: &mut App) {
    let layout = screen(app);
    click(app, layout.role_button(0));
    click(app, layout.role_button(1));
    for (slot, character) in [(0usize, 0usize), (1, 1)] {
        click(app, layout.token_home(slot));
        click(
            app,
            layout.portrait(character).expect("an authored portrait"),
        );
    }
    click(app, layout.start_button());
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

    // One person, one keyboard: take a controller, add a CPU, pick two
    // fighters, start.
    let layout = screen(&app);
    click(&mut app, layout.role_button(0));
    click(&mut app, layout.role_button(1));
    assert_eq!(
        app.world().resource::<SmashSelect>().slot(1).occupant,
        SlotOccupant::Cpu,
        "a card no controller reaches has to be able to become a CPU"
    );
    for (slot, character) in [(0usize, 0usize), (1, 1)] {
        click(&mut app, layout.token_home(slot));
        click(
            &mut app,
            layout.portrait(character).expect("an authored portrait"),
        );
    }
    click(&mut app, layout.start_button());
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
    decide_a_solo_match(&mut app);
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
        app.world().resource::<SmashSelect>().slot(0).occupant,
        SlotOccupant::Absent,
        "every slot is still decided from last time, so nothing can be pressed"
    );
    assert!(
        !app.world()
            .resource::<ambition_demo_smash::select_screen::StartRequested>()
            .0,
        "the previous START is still asked for, so the screen leaves on the \
         frame it opens"
    );

    // And it really does start again.
    decide_a_solo_match(&mut app);
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
        app.world_mut()
            .spawn(bevy::input::gamepad::Gamepad::default());
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
        !app.world()
            .resource::<ambition_platformer2d::game_shell::ShellPauseMenu>()
            .open,
        "nothing is paused before anybody presses anything"
    );

    // Escape is `Platformer2dInputActionMonolith::Start` on a keyboard: the pause press.
    tap(&mut app, KeyCode::Escape);
    settle(&mut app);

    assert!(
        app.world()
            .resource::<ambition_platformer2d::game_shell::ShellPauseMenu>()
            .open,
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

    decide_a_solo_match(&mut app);
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
                format!(
                    "seat {seat}: {} components [{}]",
                    names.len(),
                    names.join(", ")
                )
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
/// fifth field is a matter of time, so this asserts the RULE for the fields a
/// MATCH declares. ⚠ health, box and mass are levelled by the seating
/// transaction against each body's OWN character and are therefore not
/// comparable across seats — see the note below.
///
/// ⚠ scoped to what the ROSTER declares, deliberately. Per-CHARACTER differences
/// are the point of a fighting game — the versus duelists author 60 and 52 health
/// as a deliberate trade — so "both seats are identical" would be the wrong
/// assertion. What a match DECLARES applies to every seat in it; what a character
/// authors does not.
///
/// ⛔ **and the roster declares exactly three per-body things**, all checked here:
/// `fighter_abilities`, `fighter_stocks`, and `opens_suspended` (which stamps
/// `ScriptedControl`). An earlier version of this test named health, body box and
/// mass in its rationale and compared NONE of them (GPT 5.6, 2026-08-01) — those
/// three come from the character's `PhysicalBaseline`, not from the match, so
/// comparing them ACROSS seats would fail the moment somebody authored an
/// asymmetric pair. They are the seating transaction's job, not this test's, and
/// naming them here made a two-field check read as a five-field one.
#[test]
fn an_adopted_seat_and_a_spawned_seat_agree_on_every_roster_declared_field() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    decide_a_solo_match(&mut app);
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

    let rows: Vec<(usize, String, Option<u32>, bool)> = {
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            &ambition_platformer2d::engine_core::BodyAbilities,
            Option<&ambition_platformer2d::actor::FighterStocks>,
            Option<&ambition_platformer2d::characters::brain::ScriptedControl>,
        )>();
        let mut rows: Vec<(usize, String, Option<u32>, bool)> = q
            .iter(world)
            .map(|(seat, abilities, stocks, held)| {
                (
                    seat.0,
                    format!("{:?}", abilities.abilities),
                    stocks.map(|s| s.started_with),
                    held.is_some(),
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

    let (first_seat, first_abilities, first_stocks, first_held) = &rows[0];
    for (seat, abilities, stocks, held) in &rows[1..] {
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
        assert_eq!(
            held, first_held,
            "seat {seat} and seat {first_seat} disagree about the opening hold — \
             one fighter could act before the other, which `opens_suspended` \
             declares for the whole match"
        );
    }
}

/// **Jon's couch milestones 1, 2 and 5**: a keyboard player and a gamepad player
/// take different seats and drive different fighters.
///
/// ⚠ This is the check the whole couch slice exists to pass, and nothing weaker
/// substitutes for it. A lobby that can OFFER two seats is not evidence that two
/// people's inputs stay apart — the versus stage has the same test for two PADS
/// and it is what caught two seats reading one device.
///
/// Five things had to be built or repaired to reach it, and four were bugs found
/// by measuring the previous fix rather than by reading it: the keyboard counting
/// as a source, the pad going to the seat that needs one, the keyboard seat being
/// deaf to every real pad (`Entity::PLACEHOLDER`, not `None`), the select screen
/// iterating the same seat count its lobby declares, and the couch policy
/// surviving the route change into the match.

#[test]
fn a_keyboard_player_and_a_pad_player_drive_different_fighters() {
    use ambition_platformer2d::actors::actor::BodyKinematics;
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use bevy::input::gamepad::GamepadButton;

    fn pad_set(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
        app.world_mut()
            .write_message(bevy::input::gamepad::RawGamepadEvent::Button(
                bevy::input::gamepad::RawGamepadButtonChangedEvent::new(pad, button, value),
            ));
    }

    let mut app = shell_host_app();
    // ONE pad. Under the couch policy that is two seats — the keyboard's and
    // this one's. Under the old pad-only count it was one, and this test could
    // not have been written.
    let pad = app
        .world_mut()
        .spawn(bevy::input::gamepad::Gamepad::default())
        .id();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    settle(&mut app);

    // **BOTH SOURCES WORK THE SCREEN.** One cursor, two hands — the keyboard
    // takes card one and the PAD takes card two, so this test still proves the
    // pad reaches the lobby at all and not only the match.
    //
    // ⚠ **which slot gets which DEVICE is not decided by who pressed the
    // button.** `first_free_device` hands out the lowest unclaimed source in
    // card order, so card one is the keyboard (device 0) and card two is the pad
    // (device 1) whichever hand did the clicking. That is the whole point:
    // pressing a button is not a claim on a chair, and a screen where it was
    // could seat two people on one device by pressing in the wrong order.
    let layout = screen(&app);
    let pad_click = |app: &mut App, rect: ambition_demo_smash::select_screen::cursor::HitRect| {
        app.world_mut()
            .resource_mut::<SelectCursor>()
            .move_to(rect.center());
        pad_set(app, pad, GamepadButton::South, 1.0);
        app.update();
        pad_set(app, pad, GamepadButton::South, 0.0);
        app.update();
        settle(app);
    };

    click(&mut app, layout.role_button(0));
    pad_click(&mut app, layout.role_button(1));
    assert_eq!(
        app.world().resource::<SmashSelect>().slot(1).occupant,
        SlotOccupant::Controller { device: 1 },
        "the pad's click did not reach the screen, or card two took the \
         keyboard's own source"
    );

    // **AND THE CARD SAYS WHICH DEVICE IT IS.** (Jon, 2026-08-07: *"the UI has
    // no way to indicate which player is connected to which input device, so idk
    // if that is the problem or not"* — asked while debugging exactly this
    // configuration.) The button read `CONTROLLER 1` / `CONTROLLER 2`, which is
    // the slot's own numbering said back to it.
    settle(&mut app);
    {
        let world = app.world_mut();
        let mut q = world.query::<(
            &ambition_demo_smash::select_screen::RoleButtonLabel,
            &bevy::prelude::Text,
        )>();
        let mut labels: Vec<(usize, String)> =
            q.iter(world).map(|(l, t)| (l.0, t.0.clone())).collect();
        labels.sort();
        assert_eq!(
            labels
                .iter()
                .take(2)
                .map(|(_, text)| text.as_str())
                .collect::<Vec<_>>(),
            vec!["KEYBOARD", "PAD 1"],
            "the two seated cards do not name their devices: {labels:?}"
        );
    }

    click(&mut app, layout.token_home(0));
    click(&mut app, layout.portrait(0).expect("an authored portrait"));
    pad_click(&mut app, layout.token_home(1));
    pad_click(&mut app, layout.portrait(1).expect("an authored portrait"));
    click(&mut app, layout.start_button());
    settle(&mut app);

    for _ in 0..60 {
        app.update();
        if active_route(&app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
            break;
        }
    }
    for _ in 0..90 {
        app.update();
    }

    let bodies: Vec<(usize, Entity)> = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        let mut rows: Vec<(usize, Entity)> = q.iter(world).map(|(e, s)| (s.0, e)).collect();
        rows.sort_by_key(|(seat, _)| *seat);
        rows
    };
    assert!(
        bodies.len() >= 2,
        "a keyboard player and a pad player have to seat two fighters; got {}",
        bodies.len()
    );
    let (seat_one, body_one) = bodies[0];
    let (seat_two, body_two) = bodies[1];

    // **Milestone 3: stable session seats.** Two seats, and they are 0 and 1 —
    // not two entities that both think they are player one.
    assert_eq!(
        (seat_one, seat_two),
        (0, 1),
        "two players have to hold two DIFFERENT seats"
    );
    // **Milestone 4: distinct controlled ACTORS** — two bodies, one per player.
    //
    // ⚠ NOT distinct characters. The first draft asserted that too and failed:
    // both players joined with the cursor at slot 0 and picked
    // `smash_duelist_a`. That is a MIRROR MATCH, which every platform fighter
    // allows and this one should — "distinct actors" is about who each person is
    // driving, not about the roster forbidding a rematch as the same fighter.
    assert_ne!(
        body_one, body_two,
        "both seats are driving the same body, so one player is a spectator with \
         a cursor"
    );

    // The round opens on a countdown; a body held by it cannot be moved by
    // anybody, which would make the measurement below about the ceremony.
    wait_for_the_round_to_go_live(&mut app);
    let x = |app: &App, body: Entity| app.world().get::<BodyKinematics>(body).unwrap().pos.x;
    let (start_one, start_two) = (x(&app, body_one), x(&app, body_two));

    // **BOTH DIRECTIONS, because one proves half of it.** (GPT 5.6, 2026-08-01)
    //
    // ⛔ the first version of this drove ONLY the pad and asserted the other body
    // stayed still. That shows the pad does not leak — and says nothing about
    // whether the KEYBOARD player has any control authority at all. A seat wired
    // to nothing passes it perfectly.

    // Player two walks right on the PAD. Nothing is touched on the keyboard.
    pad_set(&mut app, pad, GamepadButton::DPadRight, 1.0);
    for _ in 0..40 {
        app.update();
    }
    let moved_two = x(&app, body_two) - start_two;
    let moved_one = x(&app, body_one) - start_one;
    assert!(
        moved_two.abs() > 1.0,
        "the pad player pressed right and their fighter did not move \
         ({moved_two:.2}px): the second seat is a spectator"
    );
    // A RATIO, not an absolute: a fighter driven by a source it does not own
    // would travel a comparable distance, so the two failures are orders of
    // magnitude apart and the threshold does not have to guess where "still" is.
    assert!(
        moved_one.abs() < moved_two.abs() * 0.25,
        "the pad moved the KEYBOARD player's fighter ({moved_one:.2}px against the \
         pad player's {moved_two:.2}px) - the two seats are reading the same source.\n\
         ⚠ READ THE SIGNS BEFORE BELIEVING THAT SENTENCE. Two seats on one device \
         move the SAME way. Opposite signs — especially the presser going BACKWARDS \
         — is CONTACT push-apart between two overlapping fighters, which this \
         message reported as crosstalk for a day on 2026-08-07 (it was \
         `realize_seat` keying a body by its CHARACTER, so a mirror match put both \
         seats on one anti-clump slot; fixed in 65d31c116). The bodies are {:.2}px \
         apart on x at this measurement.",
        (x(&app, body_two) - x(&app, body_one)).abs()
    );

    // Let the pad player settle, then do it the other way round.
    pad_set(&mut app, pad, GamepadButton::DPadRight, 0.0);
    for _ in 0..120 {
        app.update();
    }
    let (before_one, before_two) = (x(&app, body_one), x(&app, body_two));

    // Player ONE walks right on the KEYBOARD. The pad is released.
    Buttonlike::press(&KeyCode::ArrowRight, app.world_mut());
    for _ in 0..40 {
        app.update();
    }
    Buttonlike::release(&KeyCode::ArrowRight, app.world_mut());
    let keyboard_moved_one = x(&app, body_one) - before_one;
    let keyboard_moved_two = x(&app, body_two) - before_two;
    assert!(
        keyboard_moved_one.abs() > 1.0,
        "the KEYBOARD player pressed right and their fighter did not move \
         ({keyboard_moved_one:.2}px): seat one has no control authority, which \
         the pad-only half of this test could never have caught"
    );
    assert!(
        keyboard_moved_two.abs() < keyboard_moved_one.abs() * 0.25,
        "the keyboard moved the PAD player's fighter ({keyboard_moved_two:.2}px \
         against the keyboard player's {keyboard_moved_one:.2}px).\n\
         ⚠ SAME WARNING AS THE PAD HALF ABOVE — check the signs and the separation \
         ({:.2}px apart on x here) before concluding the seats share a source. This \
         is the assertion that misreported contact push-apart as crosstalk on \
         2026-08-07.",
        (x(&app, body_two) - x(&app, body_one)).abs()
    );
}

/// **The seating is FROZEN in a build a player runs.** (queue S35)
///
/// ⛔ Until 2026-08-01 nothing in a shipped build ever created
/// `LocalSeatTopology`. The only non-test caller was the rollback observatory,
/// behind `#[cfg(feature = "dev_tools")]`, so `reconcile_roster_with_frozen_topology`
/// returned on its first line every frame and `assign_local_seat_devices` always
/// used live discovery — the exact behaviour its own doc calls the bug. Every
/// test passed because tests construct the resource by hand.
///
/// This asserts the thing none of them did: that a decided match leaves a frozen
/// topology behind, sized by the ROSTER rather than by how many pads happen to be
/// plugged in.
///
/// ⛔ **and it counts HUMANS, which it did not.** It asserted
/// `declared_seats == roster.participants.len()`, CPUs included — and
/// `declared_seats` is read for two things that both mean *how many people are
/// playing on this machine*: it sizes the ggrs session's local handles, and it
/// picks solo-vs-couch in `assign_local_seat_devices`, where `players < 2` means
/// "leave leafwing's any-pad behaviour alone". So this one-human-one-CPU match
/// built a two-handle session whose second handle nothing ever wrote, and put a
/// solo player on the COUCH branch, which assigns pads positionally — fine while
/// their pad is at index 0 and nothing at all the moment it is not.
///
/// ⚠ **what this fixture can no longer distinguish, said plainly.** With one
/// human and no pads the roster's human count and the device count are both 1,
/// so "the roster wins over the devices" is not separable here any more. The
/// case that separates them is two humans on one keyboard, and
/// `a_keyboard_player_and_a_pad_player_drive_different_fighters` is the test
/// that builds it. What is left here is the claim that only this fixture makes:
/// a CPU does not buy a seat at the input table.
#[test]
fn a_decided_match_freezes_the_local_seating() {
    use ambition_platformer2d::input::LocalSeatTopology;

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    decide_a_solo_match(&mut app);
    settle(&mut app);

    let declared = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .expect("the screen decided a match")
        .participants
        .len();
    assert_eq!(declared, 2, "the roster is this test's premise");

    for _ in 0..60 {
        app.update();
        if active_route(&app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
            break;
        }
    }
    for _ in 0..30 {
        app.update();
    }

    let topology = app
        .world()
        .get_resource::<LocalSeatTopology>()
        .cloned()
        .expect(
            "a decided match left no frozen seating — the mechanism that stops the \
             roster and the session disagreeing is not installed in this build",
        );
    assert!(
        topology.is_frozen(),
        "the topology exists but was never captured"
    );
    let humans = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .expect("the screen decided a match")
        .participants
        .iter()
        .filter(|p| {
            matches!(
                p.controller,
                ambition_platformer2d::actor::ControllerBinding::Human { .. }
            )
        })
        .count();
    assert_eq!(
        humans, 1,
        "one human and one CPU is this test's premise — if the screen stopped \
         adding a CPU this proves nothing"
    );
    assert!(
        humans < declared,
        "…and the two counts must DIFFER, or the assertion below cannot tell \
         'counts humans' from 'counts participants'"
    );
    assert_eq!(
        topology.declared_seats(),
        Some(humans),
        "the frozen seating counts the people playing, not the roster's rows: a \
         CPU needs a body and a brain, never a device or a rollback handle"
    );
    assert_eq!(topology.players(), humans);
}

/// **Every fighter `SMASH_ROSTER` names actually exists in the shipped host.**
///
/// **THE PUPPY SLUG ON THE ACTUAL STAGE** — P3.27's end-to-end half.
///
/// `a_crawler_seated_as_a_fighter_keeps_its_own_locomotion` pins the SEAM, and
/// it does it with a synthetic `"crawler"` registered inside a fixture app. This
/// is the other test: Ambition's real `npc_puppy_slug`, the shipped host, the
/// real select screen, the real seating road.
///
/// ⭐ **the two are not redundant, and the difference is the row's whole point.**
/// A fixture proves the seating code copies locomotion off whatever definition
/// it is handed. It cannot prove the puppy slug's SHIPPED definition survives
/// catalog assembly, preparation, the grid's seatability filter and the match's
/// ability mask — the trip P3.27 asks about, and the one a deleted archetype row
/// used to break.
///
/// ⚠ **it forces the grid, and that is what "forced seat" means here.**
/// `npc_puppy_slug` is not on `SMASH_ROSTER`: being buildable and being offered
/// on a select grid are different questions (D73). Replacing `SmashRoster` is
/// how a crawler gets seated at all, and is not a suggestion that it should ship
/// as a selectable fighter.
///
/// ⛔ **the opponent is the control.** Asserting only that the slug crawls at
/// 80 px/s would pass if the stage seated EVERYBODY at 80 — the claim has to be
/// that the two seats DIFFER, in the direction their characters authored.
#[test]
fn the_puppy_slug_forced_onto_the_stage_keeps_the_body_it_authored() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;

    const SLUG: &str = "npc_puppy_slug";
    const OPPONENT: &str = "goblin";

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    settle(&mut app);

    // ⚠ both must be BUILDABLE in this composition before the grid is forced —
    // `SmashRoster::assemble` drops what the prepared registry cannot seat, so
    // forcing an unbuildable id would produce an empty stage and a confusing
    // failure two hundred lines later.
    {
        let registry = app
            .world()
            .resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>();
        for id in [SLUG, OPPONENT] {
            assert!(
                registry.get(id).is_some(),
                "`{id}` is not registered in the shipped host, so this test \
                 cannot force it onto the grid"
            );
        }
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::select::SmashRoster(vec![
            SLUG.to_string(),
            OPPONENT.to_string(),
        ]));

    decide_a_solo_match(&mut app);
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

    let seats: Vec<(usize, f32, bool)> = {
        let world = app.world_mut();
        let mut query = world.query::<(&MatchSeat, &ambition_platformer2d::actor::ActorConfig)>();
        let mut rows: Vec<(usize, f32, bool)> = query
            .iter(world)
            .map(|(seat, config)| {
                (
                    seat.0,
                    config.tuning.max_run_speed,
                    config.tuning.surface_walker,
                )
            })
            .collect();
        rows.sort_by_key(|(seat, ..)| *seat);
        rows
    };
    assert_eq!(
        seats.len(),
        2,
        "the stage seated {} bodies, so nothing below measures a forced seat: \
         {seats:?}",
        seats.len()
    );

    let (_, slug_speed, slug_clings) = seats[0];
    let (_, other_speed, other_clings) = seats[1];
    assert_eq!(
        slug_speed, 80.0,
        "the puppy slug is seated at {slug_speed} px/s and it authors 80.0 — \
         somewhere between its definition and this stage a fighter default \
         replaced the body it states, which is exactly what P3.27 asks: \
         {seats:?}"
    );
    assert!(
        slug_clings,
        "the puppy slug lost `surface_walker` by being seated, so the stage \
         decided what kind of body a crawler is: {seats:?}"
    );
    // ⛔ THE CONTROL — without it a stage that seated everybody as a slug would
    // pass every assertion above.
    assert!(
        other_speed != slug_speed && !other_clings,
        "both seats came out identical ({seats:?}), so the numbers above are \
         the stage's and not the characters'"
    );

    // ⭐⭐ **AND NOW DRIVE IT** — the row asks for the stage to be PLAYED, not
    // only seated, and the two are different claims. Everything above reads
    // `ActorConfig`, which is what the seating code wrote down; this presses a
    // key and measures where the body actually went. A number that arrives in a
    // component and never reaches motion would pass every assertion above.
    let slug_body = {
        let world = app.world_mut();
        let mut query = world.query::<(&MatchSeat, Entity)>();
        let mut rows: Vec<(usize, Entity)> = query.iter(world).map(|(s, e)| (s.0, e)).collect();
        rows.sort_by_key(|(seat, _)| *seat);
        rows[0].1
    };
    let x = |app: &App, body: Entity| -> f32 {
        app.world()
            .get::<ambition_platformer2d::actors::actor::BodyKinematics>(body)
            .expect("a seated body has kinematics")
            .pos
            .x
    };

    // ⚠ **wait out the opening countdown first.** A smash match opens SUSPENDED
    // (`opens_suspended` / `opening_countdown_ticks` — the 3-2-1-GO), so input
    // pressed before GO moves nothing. A first version of this pressed 40 frames
    // after seating and measured 0.00px, which reads exactly like "the crawler
    // cannot be driven" and was the countdown.
    for _ in 0..240 {
        app.update();
    }

    const FRAMES: usize = 40;
    let hitstun = |app: &App, body: Entity| -> f32 {
        app.world()
            .get::<ambition_platformer2d::characters::actor::BodyCombat>(body)
            .map(|combat| combat.hitstun_timer)
            .unwrap_or(0.0)
    };

    // ⛔⛔ **THE OPPONENT IS A CONFOUNDER, and it became one the day the CPU got
    // better at its job** (2026-08-15). This pressed once and asserted the
    // distance covered, which is only a reading of TOP SPEED while nobody
    // touches the body. Measured when the fighter brain stopped mirroring its
    // own attack directions: the goblin now connects inside the window, the slug
    // spent 18 of these 40 frames in hitstun, took 1.5% damage and peaked at
    // 215 px/s — knockback, not locomotion — and the assertion below read that
    // as "this body is being driven at somebody else's top speed".
    //
    // ⚠ **it is NOT enough to move the window earlier**: probed at the GO beat
    // itself and the goblin is on top of the slug even sooner (28 of 40 frames
    // in hitstun, 0 -> 10.5%). On a 480px stage there is no quiet moment.
    //
    // ⭐ so the press is RETRIED until one lands undisturbed, and each attempt is
    // constructed exactly like the original — 40 frames from a standstill, which
    // is what the calibrated numbers below encode. The direction alternates so
    // the slug walks back toward the middle instead of off the lip, and an
    // attempt is thrown away if the body was in hitstun at any point during it
    // OR in the settling frames before it.
    let mut travelled = None;
    let mut attempts = 0usize;
    for attempt in 0..12 {
        // Settle: let any carried launch bleed off, and refuse to start an
        // attempt while still reeling. `hitstun_timer` ends WITH the carried
        // momentum window, so a few clear frames on top of it is a body under
        // its own power again.
        let mut clear = 0usize;
        for _ in 0..240 {
            app.update();
            clear = if hitstun(&app, slug_body) > 0.0 {
                0
            } else {
                clear + 1
            };
            if clear >= 20 {
                break;
            }
        }
        if clear < 20 {
            continue;
        }

        let key = if attempt % 2 == 0 {
            KeyCode::ArrowRight
        } else {
            KeyCode::ArrowLeft
        };
        let before = x(&app, slug_body);
        let mut disturbed = false;
        Buttonlike::press(&key, app.world_mut());
        for _ in 0..FRAMES {
            app.update();
            disturbed |= hitstun(&app, slug_body) > 0.0;
        }
        Buttonlike::release(&key, app.world_mut());
        attempts += 1;
        if !disturbed {
            travelled = Some((x(&app, slug_body) - before).abs());
            break;
        }
    }

    let travelled = travelled.unwrap_or_else(|| {
        panic!(
            "the slug was hit during every one of {attempts} undisturbed-press \
             attempts, so its top speed could not be read from motion at all. \
             That is a finding about the stage rather than about this body, and \
             it makes every number below meaningless"
        )
    });

    assert!(
        travelled > 1.0,
        "the puppy slug was seated and then did not move when driven \
         ({travelled:.2}px over {FRAMES} frames) — a crawler that cannot be \
         played is not a fighter, however correct its `ActorConfig` reads"
    );
    // ⛔ THE POISON, and the reason this is not just "it moved": a body driven at
    // somebody else's top speed still passes "it moved".
    //
    // ⚠ **the bound is MEASURED, not derived.** 40 frames × 80 px/s "should" be
    // ~53px; the slug actually covers 34.85px, because it accelerates from a
    // standstill and never reaches its top speed in that window. Modelling it
    // would have put the line in the wrong place — so this ran the same test
    // with the GOBLIN in the slug's seat (170 px/s, the other fighter already on
    // this stage) and measured 92.50px. 60px sits between the two with room on
    // both sides.
    assert!(
        travelled < 60.0,
        "the puppy slug covered {travelled:.2}px in {FRAMES} frames. Measured \
         under exactly these conditions, its authored 80 px/s produces ~34.8px \
         and the goblin's 170 px/s produces ~92.5px — so this body is being \
         driven at a top speed that is not the one its character states, even \
         though `ActorConfig` above reads correctly"
    );
}

/// **TWO SEATED FIGHTERS SWING THEIR OWN JABS, NOT THE STAGE'S** — P3.26's
/// central claim, on live bodies in the shipped host.
///
/// The row says Smash must consume *each character's actual moves*. The
/// ratchets beside it count who AUTHORS a moveset; this asks the question one
/// step later and where it actually matters: does the authored table reach a
/// seated body, and does it stay that character's?
///
/// ⭐ **the verb IDS are identical for both fighters and that is by design** —
/// `jab`, `tilt_up`, `smash_forward` are the genre's standard map, and every
/// character authors the same names. So an id census proves nothing here; the
/// FRAME DATA is where a character lives, and it is what this compares.
///
/// ⛔ **the admiral is BODY-INCOMPLETE**, which is why it is the fighter chosen:
/// its prepared definition cannot build a body on its own
/// (`the_cast_that_still_needs_a_body_assist_only_shrinks` counts it among the
/// fourteen), so it is the case where an authored moveset is most likely to be
/// lost on the way to a seat. It is not.
///
/// ⚠ this is the shape P3.26 already recorded going wrong once: a match's
/// borrowed action-set grant regenerated the moveset FROM ITSELF, and eleven
/// authored timelines became one derived swipe on the only path that seats a
/// fighter.
#[test]
fn two_seated_fighters_carry_their_own_frame_data_for_the_same_verb() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    settle(&mut app);
    // A long blade and a short fist: both author `jab`, and the two tables
    // disagree about every number in it.
    app.world_mut()
        .insert_resource(ambition_demo_smash::select::SmashRoster(vec![
            "npc_pirate_admiral".to_string(),
            "goblin".to_string(),
        ]));
    decide_a_solo_match(&mut app);
    settle(&mut app);
    for _ in 0..60 {
        app.update();
    }

    let world = app.world_mut();
    let mut query = world.query::<(
        &MatchSeat,
        &ambition_platformer2d::combat::moveset::ActorMoveset,
    )>();
    let mut jabs: Vec<(usize, f32, f32, i32)> = query
        .iter(world)
        .filter_map(|(seat, moveset)| {
            let jab = moveset.0.moves.iter().find(|m| m.id == "jab")?;
            let frames = jab.frame_data();
            Some((seat.0, frames.startup_s, frames.reach, frames.max_damage))
        })
        .collect();
    jabs.sort_by_key(|(seat, ..)| *seat);

    assert_eq!(
        jabs.len(),
        2,
        "expected two seated fighters carrying a `jab`; got {jabs:?}. A seat with \
         no jab at all means the authored table did not reach the body, which is \
         the failure this test exists for"
    );
    let (_, admiral_startup, admiral_reach, admiral_damage) = jabs[0];
    let (_, goblin_startup, goblin_reach, goblin_damage) = jabs[1];

    // The admiral's authored jab: slower, longer, harder. `pirate_admiral_moveset`
    // says it in words — "even the jab is a blade: it starts slower than the
    // goblin's whole punish window and reaches half a body further".
    assert!(
        admiral_startup > goblin_startup,
        "the admiral's jab starts in {admiral_startup}s and the goblin's in \
         {goblin_startup}s — the blade is not slower than the fist, so at least \
         one seat is not swinging its own table"
    );
    assert!(
        admiral_reach > goblin_reach,
        "the admiral reaches {admiral_reach}px and the goblin {goblin_reach}px"
    );
    assert!(
        admiral_damage > goblin_damage,
        "the admiral's jab does {admiral_damage} and the goblin's {goblin_damage}"
    );

    // ⛔ THE CONTROL — three comparisons between two seats would ALL hold if the
    // stage handed both bodies one table and the ordering came from somewhere
    // else. This says the two tables are actually different objects.
    assert!(
        (admiral_startup - goblin_startup).abs() > 1e-4
            && (admiral_reach - goblin_reach).abs() > 1e-4,
        "both seats report the same jab ({admiral_startup}s / {admiral_reach}px), \
         so the numbers above are one table read twice"
    );
}

/// **OILER RIDES HIS OWN GEYSER, ON A BODY THE SHIPPED HOST SEATED.**
///
/// ⭐ the acceptance claim for the kit Jon asked for on 2026-08-16, measured
/// where it matters: not "the table compiles" (his own unit tests say that) but
/// *the table reached a fighter the select screen produced*. Everything between
/// the authored function and this assertion — provider registration,
/// preparation, `authored_moveset`, the seat's kit, the moveset overlay — is a
/// place it could vanish silently, and the body would go on swinging the
/// stage's generic swipe with nothing in the log.
///
/// ⛔ **the RECOVERY specifically, because it is the move a policy layer reads.**
/// `lift_speed` is derived from `Set` impulses only, so a geyser that arrived
/// with the wrong impulse mode would be invisible to `lifting_candidates` and
/// the CPU would drift at a stage it could reach.
///
/// ⚠ **the goblin is the CONTROL and it is not decoration.** He is seated beside
/// Oiler and authors a table too — so if the stage were handing both bodies one
/// kit, the assertions above would still hold for whichever fighter's table won.
/// The goblin has no way home at all by design (`goblin_moveset`: *"a goblin
/// that could pogo off a body would out-recover a character built around
/// recovery being its problem"*), so "exactly one of these two seats advertises
/// a lift" is the claim that says two different tables arrived.
#[test]
fn oiler_seated_in_the_host_rides_his_own_geyser() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::entity_catalog::AttackDir;

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    settle(&mut app);
    app.world_mut()
        .insert_resource(ambition_demo_smash::select::SmashRoster(vec![
            "npc_oiler".to_string(),
            "goblin".to_string(),
        ]));
    decide_a_solo_match(&mut app);
    settle(&mut app);
    for _ in 0..60 {
        app.update();
    }

    let world = app.world_mut();
    let mut query = world.query::<(
        &MatchSeat,
        &ambition_platformer2d::combat::moveset::ActorMoveset,
    )>();
    let mut seats: Vec<(
        usize,
        ambition_platformer2d::entity_catalog::MovesetContract,
    )> = query
        .iter(world)
        .map(|(seat, moveset)| (seat.0, moveset.0.clone()))
        .collect();
    seats.sort_by_key(|(seat, _)| *seat);
    assert_eq!(
        seats.len(),
        2,
        "expected two seated fighters; got {}. A seat with no moveset at all \
         means nothing reached the body",
        seats.len()
    );

    // Which seat is which is decided by the roster order above, but reading it
    // off the TABLE rather than off the index is what keeps this honest if that
    // ordering ever changes.
    let (oiler, control) = if seats[0].1.moves.iter().any(|m| m.id == "oil_geyser") {
        (&seats[0].1, &seats[1].1)
    } else {
        (&seats[1].1, &seats[0].1)
    };

    let geyser = oiler
        .moves
        .iter()
        .find(|m| m.id == "oil_geyser")
        .unwrap_or_else(|| {
            panic!(
                "neither seat carries `oil_geyser`. Oiler's authored table did \
                 not reach a body, so he is swinging the stage's floor: {:?}",
                oiler.moves.iter().map(|m| &m.id).collect::<Vec<_>>()
            )
        });
    let frames = geyser.frame_data();
    assert_eq!(
        frames.lift_speed,
        ambition_content::oiler_moveset::GEYSER_SPEED,
        "the geyser arrived without the rise it was authored with, so every \
         policy layer that reads `lift_speed` is blind to it"
    );

    // And the press reaches it — from the AIR, which is the only posture that
    // matters for a way home.
    assert_eq!(
        oiler
            .move_for_directional_verb("special", AttackDir::Up, false)
            .map(|m| m.id.as_str()),
        Some("oil_geyser"),
        "the up-special press does not resolve to the geyser on a live body"
    );

    // ⛔ THE CONTROL: the other seat is a different table, and it has no way
    // home. One kit handed to both bodies would fail here.
    let control_lifts: Vec<&str> = control
        .moves
        .iter()
        .filter(|m| m.frame_data().lift_speed > 0.0)
        .map(|m| m.id.as_str())
        .collect();
    assert!(
        control_lifts.is_empty(),
        "the seat beside Oiler also advertises a way home ({control_lifts:?}), \
         so both bodies are wearing one table and the assertions above are that \
         table read twice"
    );
    assert!(
        !control.moves.iter().any(|m| m.id == "oil_geyser"),
        "both seats carry the geyser"
    );
}

/// **HOW MANY OF THE GRID'S FIGHTERS STATE THEIR OWN MOVES** — P3.26's number.
///
/// P3.24's ratchet asks this of the whole prepared cast. This asks it of the
/// SELECTION GRID, which is the population P3.26 is actually about: a fighter a
/// player can pick and whose attacks are the stage's generic floor is the case
/// the row names, and a migrated NPC nobody can select is not.
///
/// ⭐ **the shipped host is the only place this is decidable.** `SMASH_ROSTER`
/// is filtered to what the composition can seat, so in a partial composition an
/// unauthored fighter and an absent one look identical.
///
/// ⛔⛔ **AND THE SILENT COLUMN IS NOT A BACKLOG.** Every one of the seven
/// authors `default_action_set: "peaceful"` — `melee: None, ranged: None,
/// special: None` — and Mary-O's row says why in as many words: *"Mary-O
/// Classic is deliberately only the run/jump floor."* These are peaceful
/// characters that Jon put on a FIGHTING grid, which is what
/// `DeclaredCombatRules::unarmed_melee` exists to make possible.
///
/// ⇒ so this ratchet does NOT say "author seven movesets". Authoring one for
/// Mary-O would contradict an explicit design decision. **Whether the floor is
/// scaffolding or permanent architecture is a product question, and it is
/// Jon's** — see `awaiting-maintainer-decision.md`.
///
/// ⚠ **a floor and a control**, like its siblings, and the control deliberately
/// does NOT instruct a deletion: reaching the whole grid would mean the peaceful
/// cast had been re-authored as fighters, which is a decision to notice rather
/// than a milestone to celebrate.
///
/// **The split as measured 2026-08-13 — seven and seven:**
///
/// ```text
///   states its own moves        silent, fights with the stage's floor
///   player_robot_v3             mary_o
///   smash_george_booul          sanic
///   npc_pirate_admiral          npc_alice
///   npc_ninja_shadow_oni_leader npc_bob
///   perfect_cellular_automaton  npc_noether
///   goblin                      npc_carl_stargan
///   special_patent_clerk
///   npc_oiler                   ⭐ crossed over 2026-08-16
/// ```
///
/// ⚠ **Oiler crossing is NOT the paragraph above being walked back.** The
/// silent column is peaceful characters on a fighting grid, deliberately; he
/// left it because Jon asked for a kit by name, not because the ratchet
/// instructed one. `oiler_moveset` is what he swings and
/// `oiler_seated_in_the_host_rides_his_own_geyser` below is the proof it
/// reaches a live body.
///
/// ⚠ the numbers are NOT asserted, deliberately — a count assertion here would
/// fail on every authoring commit and teach people to edit the number. The
/// floor and the control are what must hold; the table is the reader's.
#[test]
fn the_grid_fighters_that_state_their_own_moves_only_grow() {
    let mut app = shell_host_app();
    settle(&mut app);
    let registry = app
        .world()
        .resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>(
    );
    let roster = app
        .world()
        .resource::<ambition_demo_smash::select::SmashRoster>();

    let (authored, silent): (Vec<&str>, Vec<&str>) = roster.ids().partition(|id| {
        registry
            .get(id)
            .is_some_and(|character| character.authored_moveset.is_some())
    });
    assert!(
        !authored.is_empty(),
        "no fighter on the grid states its own move timelines, so every pick \
         fights with the stage's unarmed declaration: {silent:?}"
    );
    assert!(
        !silent.is_empty(),
        "every fighter on the grid now states its own moves ({authored:?}). \
         ⛔ do NOT read that as P3.26 finishing: the fighters that were silent \
         were peaceful ON PURPOSE, so this means somebody gave the peaceful \
         cast attacks. Check that was intended before deleting \
         `DeclaredCombatRules::unarmed_melee` — it is what lets a peaceful \
         character be selectable at all"
    );
}

/// ⚠ **`SmashRoster::assemble` FILTERS to what the catalog carries, and that is
/// correct behaviour** — a host that composes only some providers shows only the
/// fighters it has, which is what lets the bare smash app run at all. It also
/// means a misspelled id is indistinguishable from an absent provider: the grid
/// silently comes up one fighter short and the screen still looks fine.
///
/// Jon set that roster explicitly (*"we may go more than 8"*), so a dropped
/// fighter is a fighter he asked for and did not get. The SHIPPED host is where
/// the distinction is decidable: it composes every provider, so nothing there is
/// legitimately absent and anything filtered out is a typo.
///
/// ⭐ **this is the fifth hand-made PAIRING in the content, and it is checked
/// against the ASSEMBLED catalog rather than by grepping the RON** — the other
/// four are a pedestal's dialogue id to its Yarn node, that node's speaker to
/// the character's name, a character row to the map it lives in, and a row's
/// spritesheet to its manifest. Each was written after the pairing had already
/// gone wrong once.
#[test]
fn every_smash_roster_id_resolves_in_the_shipped_host() {
    use ambition_demo_smash::select::SMASH_ROSTER;

    let mut app = shell_host_app();
    settle(&mut app);
    // ⛔⛔ **THE REGISTRY, NOT THE CATALOG — and this test asked the wrong one
    // for five days** (fixed 2026-08-12). `SmashRoster::assemble` filters on the
    // prepared REGISTRY, and says why in its own doc: *"a catalog row says what a
    // character IS; `register_character` is what makes one BUILDABLE, and only
    // the second is what a seat needs."* This checked the catalog, so a fighter
    // with a row and no registration passed here and was dropped from the grid
    // anyway — which is exactly what happened to `npc_carl_stargan`, one of the
    // three fighters Jon added by name on 2026-08-11. Two of the three landed.
    // Nobody saw the third, because dropping is the SAFE behaviour and safe
    // behaviour is silent.
    let registry = app
        .world()
        .resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>(
    );

    let missing: Vec<&str> = SMASH_ROSTER
        .iter()
        .copied()
        .filter(|id| registry.get(id).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "the smash roster names {} fighter(s) the SHIPPED host cannot SEAT: \
         {missing:?}. `SmashRoster::assemble` filters on this same registry and \
         drops them silently — the select grid comes up short and looks fine — so \
         this is a typo, a provider that stopped registering the character, or a \
         character with a catalog row that nobody ever registered.",
        missing.len()
    );
    // ⛔ and the roster must not be empty for a different reason than the one
    // above: an import that resolved to an empty slice would satisfy every
    // assertion here while proving nothing.
    assert!(
        SMASH_ROSTER.len() >= 8,
        "the smash roster is down to {} entries, which is fewer than the eight \
         Jon set it at — this check would pass over almost nothing",
        SMASH_ROSTER.len()
    );
}

/// **A fighter you picked in Smash does not follow you into Ambition.**
///
/// Jon's report, reproduced end to end: select Shadow Oni Leader, start the
/// match, quit to the title, enter Ambition — and the body you control is still
/// the Oni Leader, while the Oni Leader NPC standing in the room is a second
/// copy of the same character.
///
/// Two independent causes, and both are fixed here:
///
/// * `MatchParticipantRoster` is a global resource with no lifetime, so a roster
///   Smash published outlived every Smash route.
/// * `dress_the_primary_player_as_their_own_pick` runs in the plain `Update`
///   schedule gated on nothing but that roster, so it redressed whatever body
///   the next experience put the player in.
///
/// ⚠ the assertion is the CHARACTER, not the resource. A test that only checked
/// the roster was gone would pass against a fix that released it one frame after
/// the body had already been redressed.
#[test]
fn a_fighter_picked_in_smash_does_not_follow_the_player_into_ambition() {
    let mut app = shell_host_app();
    settle(&mut app);

    launch_row(&mut app, "Smash");
    pick_and_start(&mut app, ONI_LEADER);
    for _ in 0..40 {
        app.update();
        if active_route(&app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
            break;
        }
    }
    assert_eq!(
        active_route(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        "the decided match never reached the stage, so this is not testing the leak"
    );
    // **The premise: in Smash the pick IS a fighter on the stage.**
    //
    // ⚠ this used to read the PRIMARY PLAYER's worn character, because the pick
    // reached the player by re-dressing the session's home body — which is
    // precisely the leak vector the rest of this test is about. A match
    // experience now declares no home body at all, so the premise is asserted
    // where the fighter actually is: on a seat. There is no longer anything to
    // redress, which is a stronger answer to Jon's report than the guard that
    // used to make the redressing behave.
    let seated: Vec<String> = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d::character::WornCharacter,
            With<ambition_platformer2d::actors::character_runtime::MatchSeat>,
        >();
        q.iter(world).map(|worn| worn.id().to_owned()).collect()
    };
    assert!(
        seated.iter().any(|id| id == ONI_LEADER),
        "the fighter the lobby picked never reached the stage, so this is not \
         testing anything: seated {seated:?}"
    );
    assert_eq!(
        primary_player_character(&mut app),
        None,
        "a MATCH built a home avatar. Nothing should own a controllable body \
         beside the match's own cast"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
            .is_none(),
        "the roster Smash published outlived every Smash route"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::runtime::rollback::local_session::SessionSeatingSource>(),
        &ambition_platformer2d::runtime::rollback::local_session::SessionSeatingSource::Devices,
        "the seat count Smash decided outlived the match, so the next \
         experience's session is sized by a match that has ended"
    );

    launch_row(&mut app, "Ambition");
    for _ in 0..60 {
        app.update();
        if active_route(&app).as_deref()
            == Some(ambition_content::provider::AMBITION_GAMEPLAY_ROUTE)
        {
            break;
        }
    }
    assert_eq!(
        active_route(&app).as_deref(),
        Some(ambition_content::provider::AMBITION_GAMEPLAY_ROUTE),
        "Ambition never opened, so nothing here is about what the player controls"
    );
    settle(&mut app);

    let controlled = primary_player_character(&mut app);
    assert_eq!(
        controlled.as_deref(),
        Some(ambition_content::character_catalog::PLAYABLE_ROSTER[0]),
        "the body Ambition constructed is wearing {controlled:?} — the fighter \
         the Smash lobby picked, not the character Ambition's provider starts \
         with"
    );

    // **AND THE NPC IS STILL SOMEBODY ELSE.** The visible half of the report was
    // a duplicate: a controlled body wearing the same character as the NPC
    // standing in the room. Whatever Ambition staged, exactly one body may be
    // the player's.
    let oni_bodies = bodies_wearing(&mut app, ONI_LEADER);
    assert!(
        oni_bodies <= 1,
        "{oni_bodies} bodies are wearing {ONI_LEADER} in Ambition, so the room's \
         NPC has a copy of itself walking around under the player's control"
    );
}

const ONI_LEADER: &str = "npc_ninja_shadow_oni_leader";

/// Pick `character_id` for slot 0, a CPU for slot 1, and press START.
///
/// By ID through the assembled roster, never by grid index: the crossover cast
/// is whatever the host composed, so a literal index silently becomes a
/// different fighter the day a provider is added.
fn pick_and_start(app: &mut App, character_id: &str) {
    let index = app
        .world()
        .resource::<ambition_demo_smash::select::SmashRoster>()
        .0
        .iter()
        .position(|id| id == character_id)
        .unwrap_or_else(|| panic!("{character_id} is not in this host's smash roster"));
    let layout = screen(app);
    click(app, layout.role_button(0));
    click(app, layout.role_button(1));
    click(app, layout.token_home(0));
    click(
        app,
        layout
            .portrait(index)
            .unwrap_or_else(|| panic!("no portrait cell for {character_id}")),
    );
    click(app, layout.token_home(1));
    click(app, layout.portrait(0).expect("an authored portrait"));
    click(app, layout.start_button());
    settle(app);
}

/// What the PRIMARY player's body is wearing, if there is one.
fn primary_player_character(app: &mut App) -> Option<String> {
    app.world_mut()
        .query_filtered::<&ambition_platformer2d::character::WornCharacter, ambition_platformer2d::actor::PrimaryPlayerOnly>()
        .iter(app.world())
        .next()
        .map(|worn| worn.id().to_owned())
}

/// How many bodies — of any kind — are wearing `character_id`.
fn bodies_wearing(app: &mut App, character_id: &str) -> usize {
    app.world_mut()
        .query::<&ambition_platformer2d::character::WornCharacter>()
        .iter(app.world())
        .filter(|worn| worn.id() == character_id)
        .count()
}

// ── HOW FAR A LOBBY'S DECISION GETS ─────────────────────────────────────────
//
// Jon, 2026-08-06: *"if I have a player fight another player or CPU, and then I
// start the match, only one character spawns in. Additionally it does not let me
// make a CPU vs CPU match."*
//
// ⛔ **the existing coverage cannot see this, and the reason is a category
// error.** `a_two_participant_roster_actually_seats_two_bodies` counts seats for
// ONE configuration — the one that works — and every other test in this file
// stops at the route or the session. So four different lobbies that fail in
// three different ways all present to the suite as "not tested", and the two
// that deadlock do it by WAITING, which is indistinguishable from "waiting one
// more tick" to anything that only looks at the end state.
//
// ⭐ **so the assertion is the STAGE REACHED, not a seat count.** A permanent
// refusal and a temporary wait are different answers and must never again share
// a shape; `MatchStart` is that distinction made observable.

/// **How far a decided lobby got.**
///
/// ⚠ every arm is derived from WORLD STATE — the roster resource, the
/// `MatchSeat` bodies, `ActiveMatch`, `MatchPreparationProblems`. Deliberately not
/// from log capture: the adoption mismatch happens to warn today and the
/// character-id refusal happens to be silent, and an oracle keyed on that
/// difference would be pinned to which failures currently remember to speak.
#[derive(Clone, Debug, PartialEq, Eq)]
enum MatchStart {
    /// START did nothing: the screen refused to publish a roster at all.
    SelectionRefused,
    /// A roster was published and the composition said it cannot seat it.
    PreparationRefused,
    /// A roster was published, the stage opened, and no match ever activated.
    /// **This is the shape the bug hides in** — nothing is wrong, everything is
    /// merely still waiting, forever.
    ActivationStalled,
    /// The match is live, with this many bodies wearing a `MatchSeat`.
    Activated { seats: usize },
}

/// Press one slot's role button `presses` times.
///
/// The button cycles `Absent → Controller → Cpu → Absent`, and it SKIPS the
/// controller rung when no input source is free. On a keyboard-only host that
/// is exactly one source, which is what makes these sequences readable:
/// one press on the first slot is a person, one press on the second is a CPU
/// (no source left), and two presses on the first frees the source again for
/// whoever asks next.
fn cycle_role(app: &mut App, slot: usize, presses: usize) {
    for _ in 0..presses {
        let layout = screen(app);
        click(app, layout.role_button(slot));
    }
}

/// Drag `slot`'s token onto the portrait of `character_id`.
///
/// By id through the assembled roster like [`pick_and_start`], never by grid
/// index — and this probe leans on that harder than any other test here,
/// because the whole point of one of its cases is WHICH character was picked.
fn pick_fighter(app: &mut App, slot: usize, character_id: &str) {
    let index = app
        .world()
        .resource::<ambition_demo_smash::select::SmashRoster>()
        .0
        .iter()
        .position(|id| id == character_id)
        .unwrap_or_else(|| panic!("{character_id} is not in this host's smash roster"));
    let layout = screen(app);
    click(app, layout.token_home(slot));
    click(
        app,
        layout
            .portrait(index)
            .unwrap_or_else(|| panic!("no portrait cell for {character_id}")),
    );
}

/// Press START and report how far the decision got.
///
/// ⚠ **it polls for a TERMINAL state rather than settling a fixed number of
/// frames and looking once.** Seating is a retry, so "no match yet" is the
/// ordinary reading on almost every early frame; a fixed settle would report
/// whichever answer the frame budget happened to land on. The budget here is
/// only an upper bound on patience, and reaching it IS the stall verdict.
fn start_and_report(app: &mut App) -> MatchStart {
    let layout = screen(app);
    click(app, layout.start_button());
    settle(app);

    // No roster means START declined — `start_the_battle_when_asked` asks
    // `SmashSelect::roster()` for one and gets `None` from a screen that is not
    // ready. Nothing left to wait for.
    if app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .is_none()
    {
        return MatchStart::SelectionRefused;
    }

    for _ in 0..300 {
        app.update();
        if app
            .world()
            .get_resource::<ambition_platformer2d::actors::character_runtime::MatchPreparationProblems>()
            .is_some()
        {
            return MatchStart::PreparationRefused;
        }
        if app
            .world()
            .get_resource::<ambition_platformer2d::actors::character_runtime::ActiveMatch>()
            .is_some()
        {
            // ⛔ **COUNT THE ORPHANS TOO, or this oracle goes green over a broken
            // game.** Seats are not the whole picture: the session also spawns a
            // home body from the stage's `StartingCharacter`, and while a human
            // seat ADOPTED that body the two were the same thing. Once every
            // fighter is built by the match, an unclaimed home body is a third
            // actor standing on the platform — one the camera follows and the
            // player still drives, while their actual fighter waits elsewhere.
            //
            // A seat count cannot see that, so it would report `Activated{2}`
            // about a stage you cannot play. Any controllable body that is not a
            // match seat is a defect, so it is part of the verdict.
            let world = app.world_mut();
            let mut seated =
                world.query::<&ambition_platformer2d::actors::character_runtime::MatchSeat>();
            let seats = seated.iter(world).count();
            let mut loose = world.query_filtered::<Entity, (
                With<ambition_platformer2d::actors::control::components::LocalPlayer>,
                Without<ambition_platformer2d::actors::character_runtime::MatchSeat>,
            )>();
            let orphans = loose.iter(world).count();
            assert_eq!(
                orphans, 0,
                "the match activated with {seats} seats and {orphans} controllable \
                 bodies that are NOT in it. A stage that builds its own cast must \
                 not also be handed a home avatar nobody claimed — the camera \
                 follows it, input drives it, and the fighter the player chose \
                 stands somewhere else"
            );
            return MatchStart::Activated { seats };
        }
    }
    MatchStart::ActivationStalled
}

/// **A fighter that just lost a stock comes back UNTOUCHABLE for a moment.**
///
/// Jon: *"After losing a stock, a returning fighter should not be immediately
/// vulnerable during the first instant of materialization."* The grant is the
/// engine's generic `Empowered` — the same timed untouchable a star pickup uses,
/// already rollback-registered — and it is inserted by the RULESET, which is why
/// the same character in Ambition receives nothing.
///
/// ⛔ the second half is the one that keeps this honest: an ELIMINATED fighter
/// is not placed and must not be protected either. Protecting a body that is
/// leaving play would be a grant nobody ever takes back.
#[test]
fn a_respawning_fighter_is_briefly_untouchable_and_an_eliminated_one_is_not() {
    use ambition_platformer2d::actors::features::empowerment::{Empowered, Empowerment};

    let mut app = open_the_lobby();
    cycle_role(&mut app, 0, 2);
    cycle_role(&mut app, 1, 2);
    pick_fighter(&mut app, 0, PREPARED_FIGHTER);
    pick_fighter(&mut app, 1, PREPARED_FIGHTER);
    assert_eq!(
        start_and_report(&mut app),
        MatchStart::Activated { seats: 2 }
    );
    wait_for_the_round_to_go_live(&mut app);

    let bodies: Vec<Entity> = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            Entity,
            With<ambition_platformer2d::actors::character_runtime::MatchSeat>,
        >();
        q.iter(world).collect()
    };
    let victim = *bodies.first().expect("a live match has fighters");

    // Knock it out ONCE: the roster opens with three stocks, so this is a
    // respawn rather than an elimination.
    app.world_mut()
        .resource_mut::<Messages<ambition_platformer2d::actor::BodyKnockedOut>>()
        .write(ambition_platformer2d::actor::BodyKnockedOut {
            body: victim,
            cause: ambition_platformer2d::combat::HitSource::LeftTheWorld,
        });
    app.update();
    app.update();

    let protection = app.world().get::<Empowered>(victim).copied();
    let protection = protection.expect(
        "a fighter that just lost a stock came back with no protection at all, so \
         the opponent that took the stock can take the next one at the spawn point",
    );
    assert!(
        protection.traits.holds(Empowerment::UNTOUCHABLE),
        "the returning fighter was granted an empowerment that does not make it \
         untouchable, which is the only trait respawn protection is about"
    );
    assert!(
        protection.remaining.is_some_and(|seconds| seconds > 0.0),
        "the protection is HELD rather than timed, so nothing expires it and the \
         fighter is invincible for the rest of the match"
    );

    // ⛔ **and it wears off.** A grant with no end is worse than none.
    for _ in 0..300 {
        app.update();
    }
    assert!(
        app.world()
            .get::<Empowered>(victim)
            .copied()
            .is_none_or(|later| !later.traits.holds(Empowerment::UNTOUCHABLE)),
        "five seconds after respawning, the fighter is still untouchable"
    );
}

/// **Run out the opening ceremony.**
///
/// The Smash ruleset opens 3 — 2 — 1 — GO: every fighter carries
/// `ScriptedControl` until the count ends, so a test that presses a button on
/// the tick the stage appears is pressing it at a held body and measuring the
/// ceremony rather than the input. Waiting is what a player does too.
///
/// ⛔ **bounded, and it ASSERTS rather than giving up quietly.** A silent
/// timeout here would turn "the hold never came off" — a real and previously
/// shipped bug — into a test that simply measured a shorter fight.
fn wait_for_the_round_to_go_live(app: &mut App) {
    for _ in 0..600 {
        let held = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<
                &ambition_platformer2d::actors::character_runtime::MatchSeat,
                With<ambition_platformer2d::characters::brain::ScriptedControl>,
            >();
            q.iter(world).count()
        };
        if held == 0 {
            return;
        }
        app.update();
    }
    panic!(
        "the opening hold never came off in ten seconds of ticks, so every \
         fighter in this match is a statue"
    );
}

/// Every seated fighter's x, in seat order.
fn seat_positions(app: &mut App) -> Vec<f32> {
    let world = app.world_mut();
    let mut q = world.query::<(
        &ambition_platformer2d::actors::character_runtime::MatchSeat,
        &ambition_platformer2d::engine_core::BodyKinematics,
    )>();
    let mut rows: Vec<(usize, f32)> = q
        .iter(world)
        .map(|(seat, kin)| (seat.0, kin.pos.x))
        .collect();
    rows.sort_by_key(|(seat, _)| *seat);
    rows.into_iter().map(|(_, x)| x).collect()
}

/// Open the lobby from the title screen.
fn open_the_lobby() -> App {
    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    app
}

/// A fighter this host has REGISTERED, so seating can build one.
const PREPARED_FIGHTER: &str = "player_robot_v3";

/// A SECOND registered fighter, so a case can give two seats different picks.
///
/// ⛔ **not decoration — the first draft of `a_cpu_ordered_before_the_person`
/// gave both seats `PREPARED_FIGHTER` and PASSED**, and passed for a reason that
/// had nothing to do with the thing it was testing. A probe that cannot fail
/// through its own motivating case is not a probe. Measured, not reasoned:
/// running it is what said so.
///
/// ⚠ **the mechanism it caught is gone; the requirement it left behind is not.**
/// The primary body used to be DRESSED as `participants.first()`, so two seats
/// wanting the same character accidentally landed on the costume the human seat
/// was waiting for. Nothing dresses a home body now — every fighter is built by
/// the match — but "two seats, two characters" is still what makes a per-seat
/// claim distinguishable from a match-wide one, which is why these cases still
/// pick two.
const OTHER_PREPARED_FIGHTER: &str = ambition_demo_smash::SMASH_GEORGE_BOOUL;

/// **The configuration that works.** One person, one CPU, both registered.
///
/// Here as the regression guard: everything else in this probe changes, and
/// this must not.
#[test]
fn a_person_against_a_cpu_starts_a_two_fighter_match() {
    let mut app = open_the_lobby();
    cycle_role(&mut app, 0, 1); // the person takes the only source
    cycle_role(&mut app, 1, 1); // no source left, so: CPU
                                // Two DIFFERENT fighters, so this proves two characters seat rather than
                                // that one character seats twice — see `OTHER_PREPARED_FIGHTER`.
    pick_fighter(&mut app, 0, PREPARED_FIGHTER);
    pick_fighter(&mut app, 1, OTHER_PREPARED_FIGHTER);

    assert_eq!(
        start_and_report(&mut app),
        MatchStart::Activated { seats: 2 },
        "the one lobby that has ever worked stopped working"
    );

    // **AND THE CPU FIGHTS.** The discriminator between "seated CPUs never act"
    // and "a match with NO local input channel never runs its simulation": this
    // lobby has one channel, the two-CPU lobby has none.
    let start = seat_positions(&mut app);
    for _ in 0..300 {
        app.update();
    }
    let moved: f32 = seat_positions(&mut app)
        .iter()
        .zip(&start)
        .map(|(now, then)| (now - then).abs())
        .fold(0.0, f32::max);
    eprintln!("[one-channel-match] furthest seat travelled {moved:.1}px");
}

/// **A CPU in an earlier slot than the person.**
///
/// **The invariant: seat ORDER cannot decide whether a match starts.** Every
/// seat is built the same way from the same prepared plan, so which card holds
/// the person is a detail of the lobby and nothing else.
///
/// ⛔ **it used to deadlock, and the history is why the test exists.** Nothing
/// about the case is exotic — it is the second card being the one you give away.
/// Seating ADOPTED the primary player's existing body for a human seat and
/// refused until that body already wore the picked fighter;
/// `dress_the_primary_player_as_their_own_pick` dressed it as
/// `participants.first()` rather than as the participant bound to primary input.
/// A CPU first meant the body wore the CPU's fighter, the human seat waited for
/// a costume it would never be given, and **one seat waiting meant no seat was
/// built** — the resolve pass returned from the whole system.
///
/// ⚠ **none of that code exists now**: the adopt path, the dressing system and
/// the all-or-nothing resolve pass are all deleted, and preparation answers
/// every permanent question before an entity exists. What survives is the
/// requirement, which no longer depends on any of those mechanisms.
#[test]
fn a_cpu_ordered_before_the_person_still_starts_the_match() {
    let mut app = open_the_lobby();
    cycle_role(&mut app, 0, 2); // Absent → Controller → CPU, freeing the source
    cycle_role(&mut app, 1, 1); // …which the person then takes
                                // ⚠ **DIFFERENT fighters, and that is the whole case.** With both seats on
                                // one character the first draft passed while proving nothing — see
                                // `OTHER_PREPARED_FIGHTER` for the mechanism that made it pass and why two
                                // picks are still required now that the mechanism is gone.
    pick_fighter(&mut app, 0, OTHER_PREPARED_FIGHTER);
    pick_fighter(&mut app, 1, PREPARED_FIGHTER);

    assert_eq!(
        start_and_report(&mut app),
        MatchStart::Activated { seats: 2 },
        "a lobby whose first card is a CPU did not seat both fighters, so seat \
         ORDER is deciding whether a match starts"
    );
}

/// **Two CPUs, which Jon asked for by name.**
///
/// *"it does not let me make a CPU vs CPU match, and it is very important that
/// that is expressible and easy to do."*
///
/// ⚠ **written when this FAILED, and the tense matters.** `SmashSelect::ready()`
/// USED TO require `humans_decided() >= 1`, so START was inert. That clause read
/// like product policy and was really an engine limitation wearing a rationale:
/// with no human seat nothing adopted the session's home body, and the stage
/// would open with an unowned controllable actor standing beside the match. Both
/// are fixed — the adoption where it belongs, in how a match builds its cast,
/// and the clause is gone. Left in the past tense rather than deleted because
/// the reason a rule was dropped is the part that stops it coming back.
#[test]
fn two_cpus_can_fight_each_other() {
    let mut app = open_the_lobby();
    cycle_role(&mut app, 0, 2);
    cycle_role(&mut app, 1, 2);
    pick_fighter(&mut app, 0, PREPARED_FIGHTER);
    pick_fighter(&mut app, 1, PREPARED_FIGHTER);

    assert_eq!(
        start_and_report(&mut app),
        MatchStart::Activated { seats: 2 },
        "a lobby of two CPUs cannot start a match, so nobody can watch the AI \
         fight itself and no ladder measurement is reachable from the game"
    );

    // ⛔ **AND THEY MUST ACTUALLY FIGHT.** Asserting the match ACTIVATES is the
    // trap this file was written to avoid one level down: two bodies that seat
    // correctly and then stand still satisfy every count anybody thought to
    // make. Jon asked to watch the AI fight itself, and a stage holding two
    // statues is not that.
    let start: Vec<f32> = seat_positions(&mut app);
    for _ in 0..300 {
        app.update();
    }
    let moved: f32 = seat_positions(&mut app)
        .iter()
        .zip(&start)
        .map(|(now, then)| (now - then).abs())
        .fold(0.0, f32::max);

    // **AND SOMETHING MUST BE LOOKING AT THEM.**
    //
    // ⛔ this is where the two diagnostics that used to sit here ended up, and
    // recording the route matters: a `[view-census]` print showed both seats
    // published to `DynamicFeatureViews` under ONE `FeatureId`, which is how the
    // mirror-match identity collision was found; a `[cpu-census]` print showed
    // correct factions, correct targets and zero velocity, which is how the
    // whole-system `return` in `tick_actor_brains` was found. Both are fixed and
    // both prints are gone — a diagnostic that outlives its diagnosis is noise
    // that the next reader has to re-derive.
    //
    // What replaces them is the question neither could answer: a match with no
    // local participant has no `ControlledSubject`, so the camera resolver
    // returned and framed NOTHING. Jon saw exactly that — *"when I seated 2 CPUs
    // and pressed start, nothing shows up. No stage."* The cast is declared now
    // (`FramedCast`), and this asserts the camera is actually pointed at it.
    {
        let world = app.world_mut();
        let local_view = ambition_platformer2d::sim_view::the_only_view(world);
        let resolved = world
            .entity(local_view)
            .get::<ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot>()
            .expect(
                "no camera snapshot was ever resolved in a live match, so this \
                 composition cannot say what a CPU-versus-CPU match looks like",
            );
        let follow = resolved.follow_world;
        let mut q = world.query::<(
            &ambition_platformer2d::actors::character_runtime::MatchSeat,
            &ambition_platformer2d::engine_core::BodyKinematics,
        )>();
        let mut cast: Vec<(usize, f32, f32)> = q
            .iter(world)
            .map(|(seat, kin)| (seat.0, kin.pos.x, kin.pos.y))
            .collect();
        cast.sort_by_key(|(slot, _, _)| *slot);
        assert_eq!(
            cast.len(),
            2,
            "a two-CPU match must have two bodies to frame"
        );
        let centre = ((cast[0].1 + cast[1].1) / 2.0, (cast[0].2 + cast[1].2) / 2.0);
        // ⛔ **TIGHT, and the first version of this was not.** It allowed ±200px
        // around the pair's span, and PROBED GREEN with the cast declaration
        // disabled: with nothing to frame the resolver returns and leaves the
        // previous snapshot standing, which in this fixture reads (0, 0) — and
        // (0, 0) sat inside that window. A camera parked at the world origin
        // passing a test about whether the camera found the fighters is exactly
        // the green-by-construction shape this file exists to avoid.
        //
        // Re-probed with the declaration disabled at this tolerance: RED, by
        // 218px on x and 231px on y.
        let slack = 32.0;
        assert!(
            (follow.x - centre.0).abs() <= slack && (follow.y - centre.1).abs() <= slack,
            "the camera is at ({:.0}, {:.0}) while the two fighters straddle \
             ({:.0}, {:.0}). A match with no local participant has no \
             `ControlledSubject`; unless something DECLARES the cast the \
             resolver frames nothing and the last snapshot stands — which is \
             precisely what a CPU-versus-CPU match looked like on Jon's screen.",
            follow.x,
            follow.y,
            centre.0,
            centre.1
        );
    }
    assert!(
        moved > 4.0,
        "neither CPU moved more than {moved:.1}px in five seconds. They seated, \
         they carry brains, and nothing decided anything — which reads as a \
         seating success and is a fight between two statues"
    );
}

/// **A refusal the ENGINE produced reaches the PERSON who chose the roster.**
///
/// ⛔ **`MatchPreparationProblems` had no reader in the product, and that is
/// half of the defect this whole landing is about.** Preparation names every
/// permanent reason a decided roster cannot become a match — a fighter nothing
/// registered, a CPU with no brain profile, a replay seat this build cannot
/// drive — before one entity exists. Nothing displayed it. So the screen kept
/// offering START, the match never opened, and the player was looking at a
/// deadlock wearing an invitation, which is exactly the experience Jon reported
/// on 2026-08-06 in its original form.
///
/// ⚠ **the refusal is INSERTED here rather than provoked through the grid, and
/// that is deliberate.** Provoking it needs an id the composition cannot seat,
/// and the grid now filters to exactly the ids it CAN seat — so the only honest
/// way to reach this arm through the UI is to break the filter, which would
/// make the test a test of the filter. What is being pinned is the BINDING: a
/// standing refusal is on screen, in the player's words, instead of "Ready".
/// The refusal's own content is pinned where it can never go vacuous:
/// `prepared_match::tests::an_unbuildable_character_is_refused_by_name`, which
/// names an id no composition will ever register.
///
/// ⛔ **its host-level twin is DELETED, and the deletion is the point.** That
/// test picked `npc_noether` — a portrait the grid drew and seating could not
/// build — and its own doc said it would go vacuous the day the Hall cast was
/// registered. That day came: Noether is in `PLAYABLE_ROSTER`, the grid filters
/// on the prepared registry, and the test had become a check that a working
/// thing works. A reproduction that content repaired is history, not coverage.
#[test]
fn a_preparation_refusal_is_shown_instead_of_ready() {
    use ambition_platformer2d::actors::character_runtime::{
        MatchPreparationProblems, RosterProblem,
    };

    let mut app = open_the_lobby();
    cycle_role(&mut app, 0, 2);
    cycle_role(&mut app, 1, 2);
    pick_fighter(&mut app, 0, PREPARED_FIGHTER);
    pick_fighter(&mut app, 1, OTHER_PREPARED_FIGHTER);
    settle(&mut app);

    let prompt = |app: &mut App| -> Vec<String> {
        let world = app.world_mut();
        let mut q = world.query_filtered::<&bevy::prelude::Text, bevy::prelude::With<
            ambition_demo_smash::select_screen::SelectPrompt,
        >>();
        q.iter(world).map(|text| text.0.clone()).collect()
    };
    let before = prompt(&mut app);
    assert!(
        !before.is_empty(),
        "the select screen draws no prompt at all, so this measures nothing"
    );
    assert!(
        before.iter().all(|line| !line.contains("cannot start")),
        "the screen is already refusing before anything refused: {before:?}"
    );

    app.world_mut().insert_resource(MatchPreparationProblems {
        problems: vec![RosterProblem {
            seat: 1,
            detail: "asks for a character this composition cannot build".to_string(),
        }],
    });
    settle(&mut app);

    let after = prompt(&mut app);
    assert!(
        after
            .iter()
            .any(|line| line.contains("cannot start") && line.contains("cannot build")),
        "preparation refused the roster and the screen never said so. It read \
         {after:?} — a permanent failure presenting as a wait, which is the \
         exact shape this landing exists to remove."
    );
}

/// **THE SECOND MATCH OF A SESSION MUST ALSO BE A MATCH.** (Jon, 2026-08-07)
///
/// ⛔ *"a fresh restart and then player vs cpu works, but the next match does
/// not work … there is some bad global state, we need to be careful about this,
/// over-relying on global state has happened several times."* What he sees is
/// fighters standing still in the air with the menu still responding — so the
/// bodies are built and something that should be driving them is not.
///
/// ⚠ **`coming_back_to_the_select_screen_offers_a_fresh_match` was green over
/// this the whole time**, and the reason is the exact trap this repo keeps
/// falling into: it asserts the screen is RESET — the roster gone, the slots
/// empty, START not still asked for — and every one of those is a PRESENCE
/// check. A second match that opens and then never moves satisfies all of them.
/// Only an assertion about MOTION catches it.
///
/// So this drives the whole cycle a person drives: decide a match, watch it
/// move, quit to the title, decide another, and watch THAT move.
#[test]
fn a_second_match_in_the_same_session_still_fights() {
    // ⚠ the VISIBLE host, not `shell_host_app()`. The headless fixture composes
    // the sim and the shell but not the rollback session, and Jon's freeze is a
    // whole-binary symptom: the menu keeps responding while the fighters stand
    // still, which is what a stalled SIMULATION looks like from the outside.
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    for _ in 0..ambition_app::app::shared_host_startup_ticks() * 2 {
        app.update();
    }
    settle(&mut app);

    let run_a_match = |app: &mut App, which: &str| {
        if active_route(app).as_deref() != Some(ambition_demo_smash::SMASH_SELECT_ROUTE) {
            launch_row(app, "Smash");
            settle(app);
        }
        decide_a_solo_match(app);
        settle(app);
        for _ in 0..120 {
            app.update();
            if active_route(app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
                break;
            }
        }
        assert_eq!(
            active_route(app).as_deref(),
            Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
            "{which} match never reached the stage"
        );
        for _ in 0..90 {
            app.update();
        }
        let seats = seat_positions(app);
        assert_eq!(
            seats.len(),
            2,
            "{which} match seated {} fighters, not two",
            seats.len()
        );
        let start = seats;
        for _ in 0..300 {
            app.update();
        }
        let moved: f32 = seat_positions(app)
            .iter()
            .zip(&start)
            .map(|(now, then)| (now - then).abs())
            .fold(0.0, f32::max);
        moved
    };

    let first = run_a_match(&mut app, "the first");
    assert!(
        first > 1.0,
        "even the FIRST match's fighters never moved ({first:.2}px), so this \
         fixture is measuring something other than what Jon reported"
    );

    // ⚠ BACK TO THE SELECT SCREEN, not the title: that is the shorter loop a
    // person actually takes between matches, and the one the existing
    // `coming_back_to_the_select_screen_offers_a_fresh_match` stops at.
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_platformer2d::game_shell::ShellRouteId::new(
            ambition_demo_smash::SMASH_SELECT_ROUTE,
        ),
    ));
    settle(&mut app);

    let second = run_a_match(&mut app, "the second");
    assert!(
        second > 1.0,
        "the SECOND match of the session seated two fighters and they never \
         moved ({second:.2}px against the first match's {first:.2}px). The \
         bodies exist and nothing drives them — state from the first match \
         outlived it."
    );

    // ⚠ AND THE LONGER LOOP, because Jon took both: *"I quit to title and
    // restarted the smash game"*. A route change inside the experience and a
    // full exit through the launcher retire different things, and only running
    // both says the match lifecycle is owned rather than that one path happens
    // to clean up after itself.
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    let third = run_a_match(&mut app, "the third, after quitting to the title");
    assert!(
        third > 1.0,
        "a match started after quitting to the title seated two fighters and \
         they never moved ({third:.2}px)"
    );
}

/// **WALKING OUT OF A PAUSED MATCH MUST NOT STOP THE NEXT ONE.**
/// (Jon, 2026-08-07)
///
/// ⛔ *"Doing a match, quitting to title in the middle of it, and then starting
/// a new cpu vs cpu match still causes the freeze. A quit to title should not
/// leave a dirty global state."*
///
/// [`a_second_match_in_the_same_session_still_fights`] already drove that shape
/// and was green, which is worth recording because it says where the bug was
/// NOT: not in the roster, the plan, the seating, the registry or the camera.
/// Every one of those is correct in the frozen match. So is the resource census
/// over the same sequence, which came back CLEAN — **the leaked state was never
/// a resource anybody had thought to release**, so every cleanup list was right
/// and all of them were beside the point.
///
/// It was **two globals the pause writes and the session does not own**, and it
/// takes both to freeze:
///
/// 1. **`GameMode`**, the Bevy `States` that decides whether the world advances.
///    Quitting from a paused match left it `Paused` with no session to explain
///    it. Session retirement resets it now.
/// 2. **`ClockState` / `RequestedClockScale`.** Pausing forces the sim clock to
///    **zero** so presentation stops dead, and the system that asks for the
///    neutral pace back — `emit_player_time_intent_system` — returned early when
///    there was no `PrimaryPlayer`. A CPU-versus-CPU match has none. So the mode
///    said `Playing`, `SimTick` counted up, brains decided, and every tick moved
///    **zero sim seconds**: fighters hanging in the air at their spawn pixel with
///    a menu that still answered, because menus do not run on sim time.
///
/// ⚠ **the second one only bites the SECOND match**, which is why a fresh binary
/// looked fine: the clock boots at 1.0 and nothing had zeroed it yet.
///
/// Four things have to be true together, and the test does all four:
/// * the match is **paused** when it is left — that is how you reach "Quit to
///   Title" at all, and it is what zeroes the clock;
/// * the quit is the **bare command**, which is what F10 and the in-world system
///   menu send; only the pause menu used to resume on its way out;
/// * the next match has **no local player**, so nothing asks for time back;
/// * it is measured for **motion**, because everything else about it is correct.
#[test]
fn quitting_a_paused_match_to_the_title_does_not_freeze_the_next_one() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    // ⚠ **A PAD IS PLUGGED IN, because Jon's is.** *"I do have a controller and
    // keyboard connected"* — and this repo has learned five times in one day
    // that a couch defect is invisible with one source. Two sources is also
    // what makes the channel count MOVE: a person-versus-CPU match claims one
    // of two, and the CPU match after it claims neither.
    app.world_mut()
        .spawn(bevy::input::gamepad::Gamepad::default());
    for _ in 0..ambition_app::app::shared_host_startup_ticks() * 2 {
        app.update();
    }
    settle(&mut app);

    // `channels` is the whole point of the fixture, so the roles are cycled
    // until the SCREEN agrees rather than a press count being guessed: the
    // button skips the controller rung when no source is free, so the number of
    // presses a CPU costs depends on what is plugged in and who took it.
    fn set_role(app: &mut App, slot: usize, want_person: bool) {
        for _ in 0..4 {
            let seated = matches!(
                app.world().resource::<SmashSelect>().slot(slot).occupant,
                SlotOccupant::Controller { .. }
            );
            let is_cpu = matches!(
                app.world().resource::<SmashSelect>().slot(slot).occupant,
                SlotOccupant::Cpu
            );
            if (want_person && seated) || (!want_person && is_cpu) {
                return;
            }
            cycle_role(app, slot, 1);
        }
        panic!(
            "slot {slot} would not take {} — it reads {:?}",
            if want_person { "a person" } else { "a CPU" },
            app.world().resource::<SmashSelect>().slot(slot).occupant
        );
    }

    // (how far the SIMULATION moved a fighter, how far its SPRITE moved, how
    // many of the seats were drawn at all)
    let run_a_match = |app: &mut App, which: &str, channels: usize| -> (f32, f32, usize) {
        if active_route(app).as_deref() != Some(ambition_demo_smash::SMASH_SELECT_ROUTE) {
            launch_row(app, "Smash");
            settle(app);
        }
        set_role(app, 0, channels >= 1);
        set_role(app, 1, false);
        pick_fighter(app, 0, PREPARED_FIGHTER);
        pick_fighter(app, 1, OTHER_PREPARED_FIGHTER);
        let layout = screen(app);
        click(app, layout.start_button());
        for _ in 0..300 {
            app.update();
            if active_route(app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
                break;
            }
        }
        assert_eq!(
            active_route(app).as_deref(),
            Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
            "{which} never reached the stage"
        );
        for _ in 0..90 {
            app.update();
        }
        let start = seat_positions(app);
        assert_eq!(start.len(), 2, "{which} seated {} fighters", start.len());
        let drawn_before = drawn_seat_positions(app);
        for _ in 0..300 {
            app.update();
        }
        let simulated = seat_positions(app)
            .iter()
            .zip(&start)
            .map(|(now, then)| (now - then).abs())
            .fold(0.0, f32::max);
        let drawn_after = drawn_seat_positions(app);
        (
            simulated,
            travel(&drawn_before, &drawn_after),
            drawn_after.len(),
        )
    };

    let (person_sim, person_drawn, person_sprites) =
        run_a_match(&mut app, "the person-versus-CPU match", 1);
    assert!(
        person_sim > 1.0 && person_drawn > 1.0 && person_sprites == 2,
        "even the FIRST match did not move ({person_sim:.2}px simulated, \
         {person_drawn:.2}px drawn across {person_sprites} sprites), so this \
         fixture is measuring something other than what Jon reported"
    );

    // MID-MATCH, AND **PAUSED** — which is how a person reaches "Quit to Title"
    // at all: the row only exists on the pause menu.
    //
    // ⛔ **this is the whole defect, and the test was green without these four
    // lines.** `GameMode` is a Bevy `States` global; pausing writes it from
    // outside the session, and nothing handed it back when the session died. The
    // route reached the launcher, the resource census came back CLEAN, and the
    // world was still stopped — so the next match built its fighters, seated
    // them, framed them, and never advanced a tick. *"the characters are just
    // stuck in air"*, with a menu that still answered, because menus do not run
    // on sim time.
    //
    // ⚠ **quit by the BARE command, not through the pause menu.** The menu used
    // to resume on its way out, which is exactly what hid this: `QuitToHome` has
    // four writers and only that one remembered. Asserting through the writer
    // that already got it right would pin the fix and leave the gap. This is the
    // F10 path, and the in-world system menu's, and the scripted sweep's.
    {
        use ambition_platformer2d::platformer::schedule::GameMode;
        app.world_mut()
            .resource_mut::<bevy::state::state::NextState<GameMode>>()
            .set(GameMode::Paused);
        settle(&mut app);
        assert_eq!(
            app.world()
                .resource::<bevy::state::state::State<GameMode>>()
                .get(),
            &GameMode::Paused,
            "the match did not pause, so quitting from a paused match is not \
             what this test is about to do"
        );
    }
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    {
        use ambition_platformer2d::platformer::schedule::GameMode;
        assert_eq!(
            app.world()
                .resource::<bevy::state::state::State<GameMode>>()
                .get(),
            &GameMode::Playing,
            "the session was retired and the world is still stopped. A mode that \
             stops the world describes a LIVE session; there is no session"
        );
    }

    let (cpu_sim, cpu_drawn, cpu_sprites) =
        run_a_match(&mut app, "the CPU-versus-CPU match after it", 0);
    assert!(
        cpu_sim > 1.0,
        "a CPU-versus-CPU match started after quitting a person's match seated \
         two fighters and the SIMULATION never moved them ({cpu_sim:.2}px, \
         against the first match's {person_sim:.2}px)."
    );
    // ⛔ **AND SEPARATELY, WHAT IS ON SCREEN.** Jon is describing what he SEES —
    // *"the characters are just stuck in air"* — and a fighter whose body is
    // advancing while its sprite is not looks exactly like a frozen game. The
    // two measurements are one assertion only if presentation cannot fail on
    // its own, and in this repo it can and does, silently.
    assert_eq!(
        cpu_sprites, 2,
        "the second match's fighters are simulating and {cpu_sprites} of them \
         are DRAWN. A stage nobody can see is the same defect as a stage that \
         does not move."
    );
    assert!(
        cpu_drawn > 1.0,
        "the second match's fighters moved {cpu_sim:.2}px in the simulation and \
         their sprites moved {cpu_drawn:.2}px. The world is advancing and the \
         picture of it is not — which is what a person calls a freeze."
    );
}

/// Where each seat's sprite is DRAWN, by the body id the match gave it.
///
/// ⚠ **not `seat_positions`, and the difference is the whole point.** That
/// reads `BodyKinematics` — the simulation's own answer. This reads the entity
/// the renderer spawned for the same body, which is the only thing a person
/// ever sees. A match can pass the first and fail the second.
fn drawn_seat_positions(app: &mut App) -> Vec<(String, Vec2)> {
    let world = app.world_mut();
    let mut q = world.query::<(
        &ambition_platformer2d::render::rendering::FeatureVisual,
        &GlobalTransform,
    )>();
    let mut rows: Vec<(String, Vec2)> = q
        .iter(world)
        .filter(|(visual, _)| visual.id.contains("#seat"))
        .map(|(visual, at)| (visual.id.clone(), at.translation().truncate()))
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

/// The furthest any one sprite moved between two readings, matched by id.
///
/// A sprite that was DESPAWNED and rebuilt somewhere else is not travel, so
/// only ids present in both readings count.
fn travel(before: &[(String, Vec2)], after: &[(String, Vec2)]) -> f32 {
    after
        .iter()
        .filter_map(|(id, now)| {
            before
                .iter()
                .find(|(was, _)| was == id)
                .map(|(_, then)| now.distance(*then))
        })
        .fold(0.0, f32::max)
}

/// **A MATCH THAT HAS JUST ENDED IN A DRAW MUST NOT RESTART ITSELF.**
///
/// ⛔ found by review (GPT 5.6, 2026-08-07) rather than by playing, and it is the
/// sharpest kind of finding: activation asked *"is there an `ActiveMatch` AND are
/// there `MatchSeat` bodies"* and read a receipt with no bodies as a dead
/// session's paperwork. In a platform fighter that is simply false.
/// `take_eliminated_fighters_out_of_play` DESPAWNS an eliminated fighter, and a
/// simultaneous final-stock ring-out is a supported draw — so a match that has
/// legitimately just finished sits at `ActiveMatch = current`, zero seats, for
/// the whole 4.5 seconds the winner card is up. Activation fell through and
/// rebuilt the entire prepared cast with fresh stocks, underneath the
/// announcement.
///
/// ⚠ **the KO is injected, for the reason `stocks.rs` gives at length**: earning
/// a simultaneous final-stock ring-out from two CPUs is a test of the arena, not
/// of this seam. Everything else here is real — a real lobby, a real prepared
/// plan, a real session, real seated bodies, the real elimination and despawn.
/// The stocks are set to one because that is the state a last stock IS.
#[test]
fn a_draw_does_not_rebuild_the_cast_it_just_finished() {
    use ambition_platformer2d::combat::components::FighterStocks;
    use ambition_platformer2d::combat::stocks::{BodyKnockedOut, StocksMatchDecided};

    let mut app = open_the_lobby();
    cycle_role(&mut app, 0, 2);
    cycle_role(&mut app, 1, 2);
    pick_fighter(&mut app, 0, PREPARED_FIGHTER);
    pick_fighter(&mut app, 1, OTHER_PREPARED_FIGHTER);
    assert_eq!(
        start_and_report(&mut app),
        MatchStart::Activated { seats: 2 },
        "the fixture never got a live two-seat match to finish"
    );

    // Down to the last stock apiece.
    let bodies: Vec<Entity> = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<
            ambition_platformer2d::actors::character_runtime::MatchSeat,
        >>();
        q.iter(world).collect()
    };
    assert_eq!(
        bodies.len(),
        2,
        "expected two seated fighters to finish off"
    );
    for body in &bodies {
        app.world_mut()
            .entity_mut(*body)
            .insert(FighterStocks::new(1));
    }

    // BOTH, in the same frame: the double ring-out that is a draw.
    for body in &bodies {
        app.world_mut()
            .resource_mut::<Messages<BodyKnockedOut>>()
            .write(BodyKnockedOut {
                body: *body,
                cause: ambition_platformer2d::combat::HitSource::LeftTheWorld,
            });
    }
    // ⚠ ONE update, then read: the decision lands on the very first tick and a
    // Bevy message is gone two frames later. A cursor opened after settling
    // would report an empty buffer and read as "the draw never happened".
    app.update();
    {
        let messages = app.world().resource::<Messages<StocksMatchDecided>>();
        let mut cursor = messages.get_cursor();
        let decided: Vec<_> = cursor.read(messages).cloned().collect();
        assert_eq!(
            decided.len(),
            1,
            "the double knockout did not end the match, so what follows would be \
             measuring an ordinary live match"
        );
        assert_eq!(
            decided[0].winner, None,
            "a simultaneous final-stock ring-out was awarded to a side"
        );
    }
    assert_eq!(
        app.world()
            .get_resource::<ambition_platformer2d::combat::stocks::StocksMatchSettled>()
            .map(|settled| settled.0),
        Some(true),
        "the ruleset does not consider this match settled"
    );

    let seats_after_the_draw = seat_positions(&mut app).len();
    assert_eq!(
        seats_after_the_draw, 0,
        "the eliminated fighters were not despawned, so this test is not in the \
         state it is about — zero seats with a live receipt is the whole premise"
    );

    // ⭐ **AND NOW THE FRAMES THE WINNER CARD IS UP FOR.** Two seconds, well
    // inside the 4.5 the announcement stands, and every one of them a tick on
    // which activation runs and asks whether it has work to do.
    for _ in 0..120 {
        app.update();
    }

    let world = app.world_mut();
    let mut seats = world.query::<&ambition_platformer2d::actors::character_runtime::MatchSeat>();
    let rebuilt = seats.iter(world).count();
    assert_eq!(
        rebuilt, 0,
        "{rebuilt} fighters came back after the match they were in ended in a \
         draw. The winner card is still on screen and a fresh cast with full \
         stocks is fighting underneath it: activation read 'no bodies' as 'no \
         match' and rebuilt the plan it had already built."
    );
    assert!(
        world
            .get_resource::<ambition_platformer2d::actors::character_runtime::ActiveMatch>()
            .is_some(),
        "the finished match's receipt is gone, so nothing can tell the ruleset \
         which match it just decided"
    );
}

/// **A phone can work this lobby: the prompt says a surface owns the screen,
/// and it says so in the screen's own words.**
///
/// The touch overlay reads exactly this resource to decide what is drawn and
/// what is tappable. `ControlContextKind::Empty` hides the move stick AND the
/// confirm buttons — and a hidden node takes no drags — so on `Empty` the only
/// live controls are Menu and Back, and a screen steered by a cursor cannot be
/// worked at all.
///
/// ⚠ **both halves, because they have different owners and either can regress
/// alone.** The context comes from smash's capturing `SELECT_CONTEXT` claim
/// (`declare_the_select_input_context`); the verb comes from its published
/// `UiCue` (`publish_the_select_ui_cue`). Dropping the claim gives `Empty`;
/// dropping the cue leaves the generic "Select" — which is why this asserts the
/// specific verb rather than merely that some verb exists.
#[test]
fn the_smash_lobby_hands_a_touch_screen_a_live_prompt() {
    use ambition_platformer2d::input::{SeatInputContexts, SELECT_CONTEXT};
    use ambition_platformer2d::sim_view::{ControlContextKind, ControlPrompt};

    let app = open_the_lobby();
    assert_eq!(
        active_route(&app).as_deref(),
        Some("smash_select"),
        "the launcher row opens character select"
    );
    assert_eq!(
        app.world()
            .resource::<SeatInputContexts>()
            .primary()
            .owner(),
        Some(SELECT_CONTEXT),
        "the screen claims the seat, so nothing else is arbitrating these presses"
    );

    let prompt = app.world().resource::<ControlPrompt>();
    assert_eq!(
        prompt.context,
        ControlContextKind::Menu,
        "`Empty` here would hide the stick and the confirm buttons, leaving a \
         phone with no way to move the cursor"
    );
    assert_eq!(
        prompt.menu_confirm.as_deref(),
        Some("Choose"),
        "the lobby's own verb, not the generic fallback — a `Select` here means \
         the cue never reached the prompt"
    );
}

/// **THE COORDINATES `capture_scene` DOCUMENTS STILL SEAT A MATCH.**
///
/// ⛔⛔ **the tool could not reach this state at all until 2026-08-16, and the
/// reason was one sentence in its own doc block that was false about this
/// file** (queue D130). It claimed `--press Down,Enter,Enter` was "exactly"
/// what these drivers do. It is not: [`click`] is
/// `SelectCursor::move_to(rect.center())` and THEN `tap(Enter)`, and the
/// POSITION is the load-bearing half. A key is an edge with no position, so the
/// tool's `Enter` fired wherever the cursor already sat, all four slots stayed
/// `NOT PLAYING`, and every `--route smash_gameplay` capture for days
/// photographed an empty stage — which is why no Smash change had ever been
/// looked at.
///
/// ⭐ **so the tool grew the one step that carries a position: `touch:XxY`,
/// two real `TouchInput` messages down the phone road.** This is the guard on
/// the literal numbers its doc block prints. They are literals on purpose —
/// re-deriving them here would pin the LAYOUT, which `layout::tests` already
/// does, and would agree with a stale doc forever.
///
/// ⚠ **the whole road, not the arithmetic**: a finger through this host's real
/// input stack, ending in a `MatchParticipantRoster` of two CPUs on two
/// fighters that each AUTHOR A REPERTOIRE — which is the state a watcher has to
/// be able to photograph to answer "do the two kits behave differently at all".
///
/// ⛔⛔ **"two DIFFERENT fighters" was the old claim and it was a check that
/// could not fail** (fixed 2026-08-16, queue D128). See the assertion at the
/// bottom for what replaced it and why the replacement is not two hard-coded
/// ids.
#[test]
fn the_capture_tools_documented_taps_seat_two_cpus_on_two_fighters() {
    use ambition_demo_smash::select::SlotPick;
    use bevy::input::touch::{TouchInput, TouchPhase};

    // The `--press touch:...` list in `capture_scene`'s header, in order.
    const ROLE_BUTTON_0: Vec2 = Vec2::new(167.0, 523.0);
    const ROLE_BUTTON_1: Vec2 = Vec2::new(482.0, 523.0);
    const TOKEN_HOME_0: Vec2 = Vec2::new(586.0, 446.0);
    const TOKEN_HOME_1: Vec2 = Vec2::new(622.0, 446.0);
    // ⛔⛔ **THESE TWO WERE `747x121` AND `425x121` UNTIL 2026-08-16 AND THEY
    // SEATED THE WRONG PAIR** (queue D128). Those are grid cells 3 and 0 —
    // Sanic, who has no authored repertoire at all, and Player Robot v3 — so
    // the command this row points at to ask *"do the two AUTHORED kits read
    // differently"* answered with a body that has no authored kit. The check
    // below only asserted the two picks DIFFER, so it stayed green the whole
    // time. `532x121` is cell 1 (George Booul) and `855x121` is cell 4 (the
    // Pirate Admiral): the demo's own fighter against Ambition's, the pair the
    // question is about.
    const PORTRAIT_A: Vec2 = Vec2::new(532.0, 121.0);
    const PORTRAIT_B: Vec2 = Vec2::new(855.0, 121.0);
    const START: Vec2 = Vec2::new(1191.0, 446.0);

    /// One tap of the glass, the way winit reports one: a `Started` and an
    /// `Ended` message a frame apart, folded by Bevy's own
    /// `touch_screen_input_system`. Nothing here writes `Touches`.
    fn glass_tap(app: &mut App, id: u64, at: Vec2) {
        for phase in [TouchPhase::Started, TouchPhase::Ended] {
            app.world_mut().write_message(TouchInput {
                phase,
                position: at,
                window: Entity::PLACEHOLDER,
                force: None,
                id,
            });
            app.update();
        }
        app.update();
    }

    let mut app = open_the_lobby();
    let layout = screen(&app);

    // Each documented point is still ON the widget it names. A layout change
    // that moved one would otherwise show up as a capture of the wrong screen.
    assert!(
        layout.role_button(0).contains(ROLE_BUTTON_0)
            && layout.role_button(1).contains(ROLE_BUTTON_1),
        "the documented role-button taps are off their buttons: {:?} / {:?}",
        layout.role_button(0),
        layout.role_button(1)
    );
    assert!(
        layout.token_home(0).contains(TOKEN_HOME_0) && layout.token_home(1).contains(TOKEN_HOME_1),
        "the documented token taps are off the tokens: {:?} / {:?}",
        layout.token_home(0),
        layout.token_home(1)
    );
    assert!(
        layout.start_button().contains(START),
        "the documented START tap is off the button: {:?}",
        layout.start_button()
    );

    let mut id = 0;
    let mut tap = |app: &mut App, at: Vec2| {
        id += 1;
        glass_tap(app, id, at);
    };
    // Twice each: the first press takes the only source as a CONTROLLER, the
    // second cycles it to CPU. Two CPUs is the match a capture wants — nobody
    // is holding a pad in front of a screenshot.
    tap(&mut app, ROLE_BUTTON_0);
    tap(&mut app, ROLE_BUTTON_0);
    tap(&mut app, ROLE_BUTTON_1);
    tap(&mut app, ROLE_BUTTON_1);
    assert_eq!(
        (
            app.world().resource::<SmashSelect>().slot(0).occupant,
            app.world().resource::<SmashSelect>().slot(1).occupant
        ),
        (SlotOccupant::Cpu, SlotOccupant::Cpu),
        "two taps a card did not reach CPU on both cards"
    );

    // Pick up a token, drop it on a portrait — the two-tap idiom, once per
    // slot, onto two different faces.
    tap(&mut app, TOKEN_HOME_0);
    tap(&mut app, PORTRAIT_A);
    tap(&mut app, TOKEN_HOME_1);
    tap(&mut app, PORTRAIT_B);

    let picks: Vec<Option<SlotPick>> = (0..2)
        .map(|slot| app.world().resource::<SmashSelect>().slot(slot).pick)
        .collect();
    assert!(
        matches!(picks[0], Some(SlotPick::Fighter(_)))
            && matches!(picks[1], Some(SlotPick::Fighter(_))),
        "a documented portrait tap chose no fighter: {picks:?} — a `Random` \
         here means the tap missed the grid and the token went home"
    );

    tap(&mut app, START);
    settle(&mut app);

    let roster = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .expect("the documented tap sequence never started a match")
        .clone();
    assert_eq!(roster.participants.len(), 2, "{roster:?}");
    assert!(
        roster
            .participants
            .iter()
            .all(|seat| seat.controller.brain_profile().is_some()),
        "a seat the screen made a CPU arrived as a human, so the capture \
         photographs a fighter nobody is driving"
    );

    // ⭐⭐ **AND THE CLAIM THAT SURVIVES A ROSTER REORDER: both seats wear a
    // fighter that AUTHORS ITS OWN MOVE TIMELINES.**
    //
    // ⛔ this used to be `assert_ne!(picks[0], picks[1])` and nothing else —
    // "two different fighters" — which is a check that cannot fail for the
    // reason it exists. Every reorder of `SMASH_ROSTER` re-flows the grid under
    // these two literal points, and "different" stays true however far they
    // slide: for months they sat on Sanic, whose repertoire is the shared
    // stand-in table, and the documented command kept answering this row's
    // standing product question with a body that has nothing to show.
    //
    // ⭐ the property the command is FOR is not "two cells" but "two authored
    // kits", so that is what is asserted, against the same oracle
    // `smash_roster_movesets::the_grid_fighters_with_a_real_repertoire_only_grow`
    // ratchets — `PreparedCharacterDefinition::authored_moveset`, the field the
    // stage actually reads. ⛔ deliberately NOT two hard-coded ids: naming
    // `smash_george_booul` and `npc_pirate_admiral` here would pin WHICH
    // fighters, and the header's promise is about what a watcher can SEE, which
    // any two authored fighters keep. Reorder the roster and this fails with the
    // fighter it drifted onto; author a kit for Sanic and it goes on passing.
    let registry = app
        .world()
        .resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>(
    );
    let seated: Vec<(String, bool)> = roster
        .participants
        .iter()
        .map(|seat| {
            let authored = registry
                .get(seat.character.as_str())
                .is_some_and(|definition| definition.authored_moveset.is_some());
            (seat.character.as_str().to_string(), authored)
        })
        .collect();
    // A vacuity guard on the oracle itself: an `authored_moveset` that had
    // silently become `None` for EVERYBODY would make the assertion below a
    // statement about a field nothing fills, so prove the grid still has a
    // generic fighter to fail against.
    assert!(
        app.world()
            .resource::<ambition_demo_smash::select::SmashRoster>()
            .ids()
            .any(|id| registry
                .get(id)
                .is_some_and(|definition| definition.authored_moveset.is_none())),
        "every fighter on the grid reports an authored moveset, so the check \
         below cannot distinguish the authored pair from the generic floor and \
         is proving nothing"
    );
    assert!(
        seated.iter().all(|(_, authored)| *authored),
        "a documented portrait tap seated a fighter with NO authored moveset: \
         {seated:?}. The grid re-flowed under these literal points and the \
         command in `capture_scene`'s header now photographs the generic floor \
         — which is exactly how this row's product question got answered wrong \
         twice. Re-derive the two portrait points from `SelectLayout::portrait` \
         for the cells the authored fighters now occupy, and fix the header."
    );
    assert_ne!(
        seated[0].0, seated[1].0,
        "both documented portrait taps landed on the SAME fighter, so the \
         capture cannot show two kits side by side"
    );
}

/// **Every fighter on this stage is read against ONE percent, whatever game it
/// came from.** (queue D131, found by capturing a real match 2026-08-16)
///
/// ⛔⛔ **an authored `max_health` is a statement made under the AUTHORING
/// GAME's rules, and this stage seats fourteen games' worth of cast.**
/// `damage_percent()` is `accumulated / max`, so the pool is the scale a percent
/// is READ against. Mary-O and Sanic are one-hit-kill platformer protagonists
/// and both author `max_health: 1` — exactly right at home. Seated here, one
/// seven-second match through this very composition read:
///
/// ```text
/// mary_o             42 damage    4200%
/// sanic               8 damage     800%
/// player_robot_v3    11 damage      18%
/// smash_george_booul  9 damage       9%
/// ```
///
/// Four fighters divided by 1, 1, 60 and 100. It looked exactly like percent
/// accruing on a clock on half the cast — the meter was honest, the division was
/// correct, and the denominators were four different games'.
///
/// ⚠ **the fix that failed first was per-CHARACTER**: this demo stamped its
/// reference onto the three ids it registers, which is three of fourteen. The
/// pool is a rule of the MATCH now
/// (`MatchParticipantRoster::fighter_health_pool`), so a character joining from
/// anywhere is read against the stage's own hundred.
///
/// ⭐ **the hit is written, the SCALE is measured.** The claim is not that a
/// swing connects — `duel_arena` and `the_repertoire_gets_used` own that — it is
/// that the same damage is the same percent on two bodies whose home games sized
/// them a hundred times apart. So the damage arrives down the real channel and
/// the reading is what is asserted.
#[test]
fn a_fighter_from_another_game_reads_its_percent_against_this_stages_pool() {
    use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};

    const CROSSOVER: &str = "mary_o";
    // ⚠ **Ambition's own robot, not this demo's George.** The pair is Jon's
    // capture's pair, and it is also the honest poison: `player_robot_v3`
    // authors a real 60-point pool, so the two characters disagree about their
    // meter by a factor of sixty BEFORE the match says anything. George authors
    // no pool at all now — the demo stopped stamping its reference onto the
    // three ids it registers — which would make "they disagree" true for a
    // duller reason.
    const NATIVE: &str = "player_robot_v3";

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    settle(&mut app);

    // ⛔ **THE POISON, and without it the assertion below is unfalsifiable.**
    // The two characters have to really arrive at this stage carrying different
    // pools, or "both seats agree" would also be true of a build that had
    // stopped reading authored vitals at all.
    let authored = |app: &App, id: &str| -> Option<i32> {
        app.world()
            .resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>()
            .get(id)
            .expect("this stage cannot seat a character it has not prepared")
            .vitals
            .max_health
    };
    let crossover_pool = authored(&app, CROSSOVER);
    let native_pool = authored(&app, NATIVE);
    assert_ne!(
        crossover_pool, native_pool,
        "{CROSSOVER} and {NATIVE} author the same pool now, so this test can no \
         longer tell a levelled match from an unlevelled one. Pick two \
         characters whose home games disagree, or delete this test with the \
         defect it guards."
    );
    assert_eq!(
        crossover_pool,
        Some(1),
        "{CROSSOVER} is a one-hit-kill platformer protagonist and its own game \
         says so; if that changed, this test is measuring a different world"
    );
    assert_eq!(
        native_pool,
        Some(60),
        "{NATIVE} is supposed to bring a real authored pool of its own, so the \
         two characters disagree by a factor of sixty before the match speaks"
    );

    let roster = app
        .world()
        .resource::<ambition_demo_smash::select::SmashRoster>()
        .0
        .clone();
    let portrait_of = |id: &str| {
        roster
            .iter()
            .position(|entry| entry == id)
            .unwrap_or_else(|| panic!("{id} is not on the assembled grid: {roster:?}"))
    };

    let layout = screen(&app);
    click(&mut app, layout.role_button(0));
    click(&mut app, layout.role_button(1));
    for (slot, character) in [(0usize, CROSSOVER), (1, NATIVE)] {
        click(&mut app, layout.token_home(slot));
        click(
            &mut app,
            layout
                .portrait(portrait_of(character))
                .expect("an authored portrait"),
        );
    }
    click(&mut app, layout.start_button());
    // Past the 3-2-1 opening hold, so nothing below is refused for being held.
    for _ in 0..240 {
        app.update();
    }

    let seated = |app: &mut App| -> Vec<(String, Entity, i32, i32, f32)> {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &WornCharacter, &BodyHealth)>();
        let mut rows: Vec<(String, Entity, i32, i32, f32)> = q
            .iter(world)
            .map(|(entity, worn, health)| {
                (
                    worn.id().to_string(),
                    entity,
                    health.max(),
                    health.damage_taken(),
                    health.damage_percent(),
                )
            })
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows
    };

    let bodies = seated(&mut app);
    assert_eq!(
        bodies.len(),
        2,
        "the screen has to seat both fighters or nothing below measures anything: \
         {bodies:?}"
    );
    let pools: Vec<i32> = bodies.iter().map(|(_, _, max, ..)| *max).collect();
    assert_eq!(
        pools,
        vec![
            ambition_demo_smash::SMASH_PERCENT_REFERENCE,
            ambition_demo_smash::SMASH_PERCENT_REFERENCE
        ],
        "two seats in one match are being read against two different hundreds: \
         {bodies:?}"
    );

    // The SAME damage at each of them, down the real channel.
    const BITE: i32 = 20;
    for (_, body, ..) in &bodies {
        let at = app
            .world()
            .get::<ambition_platformer2d::platformer::body::BodyKinematics>(*body)
            .expect("a seated fighter has a body")
            .pos;
        let volume: ambition_platformer2d::engine_core::CombatVolume =
            ambition_platformer2d::engine_core::Aabb::new(
                at,
                ambition_platformer2d::engine_core::Vec2::new(40.0, 40.0),
            )
            .into();
        app.world_mut()
            .write_message(ambition_platformer2d::combat::events::HitEvent {
                strike_sfx: None,
                volume,
                damage: BITE,
                source: ambition_platformer2d::combat::events::HitSource::Melee,
                attacker: None,
                target: ambition_platformer2d::combat::events::HitTarget::Body(*body),
                mode: ambition_platformer2d::combat::events::HitMode::Knockback,
                knockback: None,
                ignored_targets: Vec::new(),
            });
    }
    for _ in 0..4 {
        app.update();
    }

    let after = seated(&mut app);
    for (id, _, _, taken, _) in &after {
        assert!(
            *taken >= BITE,
            "the hit never reached {id}, so this test measures nothing: {after:?}"
        );
    }
    // **WHAT ONE POINT OF DAMAGE READS AS, per fighter.** Compared as a scale
    // rather than as a total, because these two are in a live match and the
    // brains land their own hits — what must agree is the exchange rate, not the
    // running score. Before the match declared its own pool this was 1.0 for
    // Mary-O (a point of damage is a whole meter) and 0.01 for George.
    let scale = |(id, _, _, taken, pct): &(String, Entity, i32, i32, f32)| {
        assert!(
            *taken > 0,
            "{id} has taken no damage, so its scale is undefined"
        );
        *pct / *taken as f32
    };
    let crossover_scale = scale(&after[0]);
    let native_scale = scale(&after[1]);
    assert!(
        (crossover_scale - native_scale).abs() < 1e-6,
        "one point of damage reads as {crossover_scale} on {} and {native_scale} \
         on {} — a fighter's HOME GAME is still sizing its meter: {after:?}",
        after[0].0,
        after[1].0
    );
    let declared = 1.0 / ambition_demo_smash::SMASH_PERCENT_REFERENCE as f32;
    assert!(
        (crossover_scale - declared).abs() < 1e-6,
        "both fighters agree, and they agree on the wrong number: this stage \
         declares {} as a full meter, so a point of damage is {declared} of one, \
         not {crossover_scale}",
        ambition_demo_smash::SMASH_PERCENT_REFERENCE
    );
}

/// **AND WHAT DOES A FIGHTER WITH NO TABLE ACTUALLY SWING?** (Jon, 2026-08-16)
///
/// `smash_roster_movesets`'s kit census reads the CHARACTER, and four of the
/// fourteen resolve to nothing there — Mary-O, Sanic, Alice and Bob author no
/// moveset and no action set, because standing in a room and talking is what
/// they were authored for. Read at the character, every one of their sixteen
/// presses is silent.
///
/// ⛔ **that is not what a player gets, and the difference is the stage.** The
/// unarmed floor lives in `DeclaredCombatRules::unarmed_melee` now — *"a STAGE
/// states what an unarmed fighter swings for"* — so the seat is armed on the way
/// in. A report that stopped at the character would say four fighters cannot
/// attack, which is false and is exactly the kind of true-measurement-wrong-
/// conclusion this repo keeps paying for.
///
/// So this seats two of them for real and asks the LIVE body.
#[test]
fn report_what_an_unarmed_fighter_swings_once_the_stage_has_armed_it() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::entity_catalog::AttackDir::*;

    let presses: [(
        &str,
        &str,
        ambition_platformer2d::entity_catalog::AttackDir,
        bool,
    ); 16] = [
        ("jab", "attack", Neutral, true),
        ("ftilt", "attack", Forward, true),
        ("utilt", "attack", Up, true),
        ("dtilt", "attack", Down, true),
        ("fsmash", "smash", Forward, true),
        ("usmash", "smash", Up, true),
        ("dsmash", "smash", Down, true),
        ("nair", "attack", Neutral, false),
        ("fair", "attack", Forward, false),
        ("bair", "attack", Back, false),
        ("uair", "attack", Up, false),
        ("dair", "attack", Down, false),
        ("nspecial", "special", Neutral, true),
        ("sspecial", "special", Forward, true),
        ("uspecial", "special", Up, true),
        ("dspecial", "special", Down, true),
    ];

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    settle(&mut app);
    // Two of the four with no table of their own: a crossover protagonist and a
    // Hall NPC, so the answer is not about one provider.
    app.world_mut()
        .insert_resource(ambition_demo_smash::select::SmashRoster(vec![
            "mary_o".to_string(),
            "npc_alice".to_string(),
        ]));
    decide_a_solo_match(&mut app);
    settle(&mut app);
    for _ in 0..90 {
        app.update();
    }

    // ⛔⛔ **THE THIRD ROUTE: what did the STAGE declare?** The seat's kit is
    // built from `DeclaredCombatRules::unarmed_melee` for a character that
    // states none of its own, so an empty seat is either a stage that declared
    // nothing or a declaration that did not reach the publisher.
    let declared = app
        .world()
        .get_resource::<ambition_platformer2d::combat::rules::DeclaredCombatRules>()
        .map(|rules| rules.unarmed_melee.is_some());
    eprintln!("[unarmed declaration] DeclaredCombatRules::unarmed_melee present = {declared:?}");

    let world = app.world_mut();
    let mut query = world.query::<(
        &MatchSeat,
        &ambition_platformer2d::combat::moveset::ActorMoveset,
        Option<&ambition_platformer2d::combat::components::CombatKit>,
    )>();
    let mut rows: Vec<(usize, String)> = query
        .iter(world)
        .map(|(seat, moveset, kit)| {
            let mut resolved: Vec<String> = Vec::new();
            for (label, base, dir, grounded) in &presses {
                let id = moveset
                    .0
                    .move_for_directional_verb(base, *dir, *grounded)
                    .map(|mv| mv.id.clone())
                    .unwrap_or_else(|| "SILENT".to_string());
                resolved.push(format!("{label}={id}"));
            }
            // ⛔⛔ **THE SECOND ROUTE, and the report is wrong without it.** A
            // moveset is one road to a swing; `CombatKit::innate_melee` is the
            // other — the preset swipe an action set carries — and a body with
            // an empty timeline table can still hit somebody through it. Reading
            // only the first would report "this fighter cannot attack" off a
            // measurement that never asked.
            (
                seat.0,
                format!(
                    "  seat {}  moves={:<3} innate={:?}\n           {}",
                    seat.0,
                    moveset.0.moves.len(),
                    kit.map(|kit| (
                        kit.innate_melee.is_some(),
                        kit.innate_ranged.is_some(),
                        kit.innate_special.is_some()
                    )),
                    resolved.join(" ")
                ),
            )
        })
        .collect();
    rows.sort_by_key(|(seat, _)| *seat);
    eprintln!(
        "[unarmed on the stage]\n{}",
        rows.iter()
            .map(|(_, row)| row.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(rows.len(), 2, "the stage seated {} fighters", rows.len());
}
