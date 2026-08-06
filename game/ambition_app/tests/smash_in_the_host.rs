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
            .len(),
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
         pad player's {moved_two:.2}px) - the two seats are reading the same source"
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
         against the keyboard player's {keyboard_moved_one:.2}px)"
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
    let catalog = app
        .world()
        .resource::<ambition_platformer2d::character::CharacterCatalog>();

    let missing: Vec<&str> = SMASH_ROSTER
        .iter()
        .copied()
        .filter(|id| catalog.get(id).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "the smash roster names {} fighter(s) the SHIPPED host's assembled \
         catalog does not carry: {missing:?}. `SmashRoster::assemble` drops them \
         silently — the select grid comes up short and looks fine — so this is \
         either a typo or a provider that stopped registering the character.",
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
    // The premise: in Smash the pick IS the controlled body. Without this the
    // test could pass because the dressing never worked at all.
    assert_eq!(
        primary_player_character(&mut app).as_deref(),
        Some(ONI_LEADER),
        "the fighter the lobby picked never reached the body, so there is no \
         redressing to leak"
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
