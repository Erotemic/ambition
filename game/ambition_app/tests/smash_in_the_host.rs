#![cfg(feature = "input")]
//! Smash, from the title screen — the whole way in and the whole way back.
//!
//! Listing it in the multi-game host is what makes the crossover claim its own source comment
//! makes — *"the crossover claim moves to where it belongs — Ambition HOSTING this experience
//! alongside its own"* — a thing that runs rather than a thing that is argued.
//!
//! Gated on the `input` feature: the presses here are REAL keys through the real
//! host input stack, which is where three of the four playtest defects lived.
//!
//! It also exercises the one shape no other provider has: a launcher row that
//! opens a QUESTION. Every other entry activates its gameplay route directly;
//! this one opens character select, which is a frontend route of the provider's
//! own, and the stage arrives only once the screen has decided.

use ambition_platformer2d::characters::smash_capture::SmashHoldState;
use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;

use ambition_app::app::shell_host;
use ambition_demo_smash::select::{SlotOccupant, SmashSelect};
use ambition_demo_smash::select_screen::cursor::SelectCursors;
use ambition_demo_smash::select_screen::layout::SelectLayout;
use ambition_platformer2d::game_shell::{ShellCommand, ShellLauncherCommand, ShellRouter};
use leafwing_input_manager::prelude::Buttonlike;

/// The real shell-host composition PLUS the real host input stack, headless —
/// the same shape `participant_input.rs` uses.
///
/// the input stack is not optional here, it is the point. A test that
/// wrote `SeatMenuFrames` by hand would be testing a resource the host REBUILDS
/// from its participants every frame: the select screen's whole complaint list
/// ("Start does not add a CPU", "there is no start on a keyboard") lived in the
/// span between a key and that resource, which hand-set frames skip over.
fn shell_host_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // PINNED, because `app.update()` is otherwise a unit of WALL CLOCK.
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

/// Put the cursor on something and click it.
///
/// The position is written the way a MOUSE writes it; the click is a real
/// keyboard Enter travelling the host's whole participant chain, which is what
/// this file exists to exercise. That every widget is also REACHABLE by arrows
/// alone is `the_screen_decides::the_arrows_alone_can_work_the_whole_screen`,
/// where a bare demo app makes it cheap to assert.
fn click(app: &mut App, rect: ambition_demo_smash::select_screen::cursor::HitRect) {
    // Same reason as `pad_click` below: four cursors, and `confirm` presses the
    // keyboard, whose seat is a composition fact rather than this helper's.
    {
        let mut cursors = app.world_mut().resource_mut::<SelectCursors>();
        for seat in 0..4 {
            cursors
                .seat_mut(seat)
                .expect("seat is bounded by the loop")
                .move_to(rect.center());
        }
    }
    confirm(app);
}

/// The screen's own geometry — the same rectangles it draws.
///
/// from the HOST's assembled roster. In this composition the grid is the
/// crossover cast (every character tagged `smash`, plus the demo's own four),
/// not the four a standalone demo offers — and a layout built from the wrong
/// count puts every click one cell off.
fn screen(app: &App) -> SelectLayout {
    SelectLayout::for_viewport(
        None,
        app.world()
            .resource::<ambition_demo_smash::select::SmashRoster>()
            // CELLS, not fighters: the grid's last square is RANDOM, and a
            // layout built from the fighter count puts every click one cell off
            // at the end of the last row.
            .cell_count(),
    )
}

/// Where an active slot's placed token is drawn. The token has no independent
/// home/rest coordinate: when it is not carried, its pick determines this rect.
fn placed_token(app: &App, slot: usize) -> ambition_demo_smash::select_screen::cursor::HitRect {
    let layout = screen(app);
    ambition_demo_smash::select_screen::token_rect(
        &layout,
        app.world().resource::<SmashSelect>(),
        app.world()
            .resource::<ambition_demo_smash::select::SmashRoster>(),
        slot,
    )
    .unwrap_or_else(|| panic!("slot {slot} has no placed token on the current page"))
}

/// One person at a keyboard, against one CPU, from the buttons.
///
/// Slot 1 takes the only source; slot 2 has none left, so its button skips
/// straight to CPU. Then a fighter each, then START.
fn decide_a_solo_match(app: &mut App) {
    let layout = screen(app);
    click(app, layout.role_button(0));
    click(app, layout.role_button(1));
    for (slot, character) in [(0usize, 0usize), (1, 1)] {
        let token = placed_token(app, slot);
        click(app, token);
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
        let token = placed_token(&app, slot);
        click(&mut app, token);
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

    // THE WAY OUT. Ambition's own rooms have the kaleidoscope pause menu, so
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

/// A rematch has to be possible.
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

/// THE TWO-PARTICIPANT FLOW, to its end: select → lock in → match → PAUSE.
///
/// The select half is covered above. Two people could start a match together and then not pause
/// it.
///
/// The press has to enter where a real one enters.
#[test]
fn two_participants_start_a_match_and_can_still_pause_it() {
    use ambition_platformer2d::input::InputParticipant;

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");

    // SPAWN A PAD, do not spawn the participant. The host derives its seats
    // from live `Gamepad` entities every frame and despawns any it did not
    // declare, so a hand-spawned `InputParticipant` is gone by the next update —
    // the same "the resource is derived, the pads are the fact" trap the select
    // screen's own tests hit.
    // A pad alone is not a seat: the host seats participants from the DECLARED
    // seat count, so a lobby that never opened has one seat however many pads
    // are plugged in. Both facts are needed.
    // SPAWN PADS. The select screen declares its seat count FROM the live
    // pads and the host derives participants from that declaration, so an
    // inserted `LocalSeatOffer` is clobbered on the next frame — the pads
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

/// PROBE: "even when we add a CPU player in smash there is
/// only ever one player that shows up in game."
///
/// every existing test in this file stops at the ROUTE and the SESSION.
/// ⭐ A SEATED FIGHTER IS NOT GRANTED BOUNDED SENSES, in the real host, through
/// the real seating pass.
///
/// `ensure_perception` hands `Perception::Sighted { viewport_half: 480 }` to
/// every brained non-boss body, and being juked / losing a foe / giving up are
/// exploration mechanics with no place in a match: both fighters are on screen
/// the whole time and each always knows where the other is.
///
/// ⛔ WHAT THIS DEFENDS, because a component check reads like bookkeeping and is
/// not. `DEFAULT_VIEWPORT_HALF.x` is 480 and the smash platform is 480 wide, so
/// two fighters that drifted apart on the SAME STAGE went permanently blind to
/// each other. Over a sixteen-character mirror sweep six characters' median gap
/// sat between 491 and 515px — with nothing at all between 295 and 491 — and
/// three of them dealt no damage in a full minute. Widening only the viewport,
/// with the platform untouched, collapsed all six to 18–278 and every one of
/// them started fighting; the fighters already inside 480 did not move a pixel.
///
/// The seat is what makes the difference, so the seat is what this asserts.
#[test]
fn a_seated_fighter_keeps_its_omniscient_senses() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::actors::features::ecs::perception::Perception;

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

    let world = app.world_mut();
    // ⛔ `Option<&Perception>`, never `&Perception`: the whole point is that a
    // seated fighter does NOT carry the component, and a query that requires it
    // would report zero rows and pass by finding nothing — the check that
    // cannot fail.
    let seats: Vec<(usize, Option<Perception>)> = world
        .query::<(&MatchSeat, Option<&Perception>)>()
        .iter(world)
        .map(|(seat, perception)| (seat.0, perception.copied()))
        .collect();
    assert!(
        seats.len() >= 2,
        "the premise: two seated fighters, found {}",
        seats.len()
    );
    for (seat, perception) in seats {
        let policy = perception.unwrap_or_default();
        assert!(
            policy.knows_bodies_anywhere(),
            "seat {seat} was granted bounded senses ({policy:?}) - a match fighter \
             that drifts past its own viewport can never approach again"
        );
    }
}

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

    // Seat 0 is player-bodied; seat 1 is actor-bodied. `BodyPoseView` is a
    // player-body read model, while actor-bodied presentation uses `ActorAnimIndex`,
    // so headless tests cannot prove that the actor-bodied fighter is rendered.
    assert!(
        seated.len() == 2,
        "the census above is the useful output; this pins the premise"
    );
}

/// Adopted and spawned seats must agree on every per-body field declared by
/// the match roster: fighter abilities, stocks, and opening suspension.
/// Character-owned physical baselines may legitimately differ across seats and
/// are tested by seating/character construction instead.
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
            Option<&ambition_platformer2d::characters::control::ScriptedControl>,
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

/// This is the check the whole couch slice exists to pass, and nothing weaker
/// substitutes for it. A lobby that can OFFER two seats is not evidence that two
/// people's inputs stay apart — the versus stage has the same test for two PADS
/// and it is what caught two seats reading one device.

#[test]
fn a_keyboard_player_and_a_pad_player_drive_different_fighters() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::engine_core::BodyKinematics;
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

    // BOTH SOURCES WORK THE SCREEN. One cursor, two hands — the keyboard
    // takes card one and the PAD takes card two, so this test still proves the
    // pad reaches the lobby at all and not only the match.
    //
    // Joining is explicit now: the keyboard press claims card one for input
    // source 0 and the pad press claims card two for input source 1. The screen
    // must not reconstruct that ownership later by scanning for a free device.
    let layout = screen(&app);
    // Putting them all on the rect makes the press land there whoever it belongs to, which is what
    // this helper was always saying.
    let pad_click = |app: &mut App, rect: ambition_demo_smash::select_screen::cursor::HitRect| {
        {
            let mut cursors = app.world_mut().resource_mut::<SelectCursors>();
            for seat in 0..4 {
                cursors
                    .seat_mut(seat)
                    .expect("seat is bounded by the loop")
                    .move_to(rect.center());
            }
        }
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

    // AND THE CARD SAYS WHICH DEVICE IT IS. The button read `CONTROLLER 1` / `CONTROLLER
    // 2`, which is the slot's own numbering said back to it.
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

    let token_zero = placed_token(&app, 0);
    click(&mut app, token_zero);
    click(&mut app, layout.portrait(0).expect("an authored portrait"));
    let token_one = placed_token(&app, 1);
    pad_click(&mut app, token_one);
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

    // Milestone 3: stable session seats. Two seats, and they are 0 and 1 —
    // not two entities that both think they are player one.
    assert_eq!(
        (seat_one, seat_two),
        (0, 1),
        "two players have to hold two DIFFERENT seats"
    );
    // Each player must drive a distinct body/control subject. Character IDs may
    // match because mirror matches are valid.
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

    // Exercise both input sources: pad isolation alone does not prove that the
    // keyboard seat has control authority.

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

/// A decided match freezes local input topology from human participants, not
/// roster length. CPU fighters do not allocate local GGRS/input seats. Mixed
/// keyboard/pad separation is covered separately; this fixture pins the
/// one-human-plus-CPU case.
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

/// Every fighter `SMASH_ROSTER` names actually exists in the shipped host.
///
/// THE PUPPY SLUG ON THE ACTUAL STAGE — P3.27's end-to-end half.
///
/// `a_crawler_seated_as_a_fighter_keeps_its_own_locomotion` pins the SEAM, and
/// it does it with a synthetic `"crawler"` registered inside a fixture app. This
/// is the other test: Ambition's real `npc_puppy_slug`, the shipped host, the
/// real select screen, the real seating road.
///
/// the two are not redundant, and the difference is the row's whole point. A fixture proves the
/// seating code copies locomotion off whatever definition it is handed.
///
/// Replacing `SmashRoster` is how a crawler gets seated at all, and is not a suggestion that it
/// should ship as a selectable fighter.
///
/// the opponent is the control. Asserting only that the slug crawls at
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

    // both must be BUILDABLE in this composition before the grid is forced —
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
    // THE CONTROL — without it a stage that seated everybody as a slug would
    // pass every assertion above.
    assert!(
        other_speed != slug_speed && !other_clings,
        "both seats came out identical ({seats:?}), so the numbers above are \
         the stage's and not the characters'"
    );

    // AND NOW DRIVE IT — the row asks for the stage to be PLAYED, not
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
    // wait out the opening countdown first. A smash match opens SUSPENDED (`opens_suspended` /
    // `opening_countdown_ticks` — the 3-2-1-GO), so input pressed before GO moves nothing.
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

    // THE OPPONENT IS A CONFOUNDER, and it became one the day the CPU got better at its job.
    // This pressed once and asserted the distance covered, which is only a reading of TOP SPEED
    // while nobody touches the body.
    //
    // it is NOT enough to move the window earlier: probed at the GO beat
    // itself and the goblin is on top of the slug even sooner (28 of 40 frames
    // in hitstun, 0 -> 10.5%). On a 480px stage there is no quiet moment.
    //
    // so the press is RETRIED until one lands undisturbed, and each attempt is constructed
    // exactly like the original — 40 frames from a standstill, which is what the calibrated
    // numbers below encode.
    let mut measured = None;
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
        let mut disturbed = false;
        // THE BODY'S OWN SPEED, not the ground it covered. Measured 2026-08-23:
        // the accepted run had the slug at exactly its authored 80 px/s and
        // still crossed 290px, because the opponent was TWO PIXELS away and
        // shoving it the whole time. Distance is a proxy for top speed and a
        // neighbour can corrupt it; `vel.x` is the quantity this test names.
        let mut peak_speed = 0.0f32;
        Buttonlike::press(&key, app.world_mut());
        for _ in 0..FRAMES {
            app.update();
            disturbed |= hitstun(&app, slug_body) > 0.0;
            let vx = app
                .world()
                .get::<ambition_platformer2d::engine_core::BodyKinematics>(slug_body)
                .map(|kin| kin.vel.x.abs())
                .unwrap_or(0.0);
            peak_speed = peak_speed.max(vx);
        }
        Buttonlike::release(&key, app.world_mut());
        attempts += 1;
        if !disturbed {
            measured = Some(peak_speed);
            break;
        }
    }

    let top_speed = measured.unwrap_or_else(|| {
        panic!(
            "the slug was hit during every one of {attempts} undisturbed-press \
             attempts, so its top speed could not be read from motion at all. \
             That is a finding about the stage rather than about this body, and \
             it makes every number below meaningless"
        )
    });

    assert!(
        top_speed > 1.0,
        "the puppy slug was seated and then did not move when driven \
         (peak {top_speed:.0} px/s over {FRAMES} frames) — a crawler that cannot \
         be played is not a fighter, however correct its `ActorConfig` reads"
    );
    // The DISCRIMINATION is the whole point and it is now read directly: the
    // slug authors 80 px/s and the goblin 170, so 130 sits between them with
    // room on both sides and neither number is a distance that anything else on
    // the stage can add to.
    assert!(
        top_speed < 130.0,
        "the puppy slug peaked at {top_speed:.0} px/s while driven, against its \
         authored 80 and the goblin's 170 — so this body is being driven at a \
         top speed that is not the one its character states, even though \
         `ActorConfig` above reads correctly"
    );
}

/// TWO SEATED FIGHTERS SWING THEIR OWN JABS, NOT THE STAGE'S — P3.26's
/// central claim, on live bodies in the shipped host.
///
/// The row says Smash must consume *each character's actual moves*. The
/// ratchets beside it count who AUTHORS a moveset; this asks the question one
/// step later and where it actually matters: does the authored table reach a
/// seated body, and does it stay that character's?
///
/// the verb IDS are identical for both fighters and that is by design —
/// `jab`, `tilt_up`, `smash_forward` are the genre's standard map, and every
/// character authors the same names. So an id census proves nothing here; the
/// FRAME DATA is where a character lives, and it is what this compares.
///
/// the admiral is BODY-INCOMPLETE, which is why it is the fighter chosen:
/// its prepared definition cannot build a body on its own
/// (`the_cast_that_still_needs_a_body_assist_only_shrinks` counts it among the
/// fourteen), so it is the case where an authored moveset is most likely to be
/// lost on the way to a seat. It is not.
///
/// this is the shape P3.26 already recorded going wrong once: a match's
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

    // This says the two tables are actually different objects.
    assert!(
        (admiral_startup - goblin_startup).abs() > 1e-4
            && (admiral_reach - goblin_reach).abs() > 1e-4,
        "both seats report the same jab ({admiral_startup}s / {admiral_reach}px), \
         so the numbers above are one table read twice"
    );
}

/// OILER RIDES HIS OWN GEYSER, ON A BODY THE SHIPPED HOST SEATED.
///
/// Everything between the authored function and this assertion — provider registration,
/// preparation, `authored_moveset`, the seat's kit, the moveset overlay — is a place it could
/// vanish silently, and the body would go on swinging the stage's generic swipe with nothing in the
/// log.
///
/// the RECOVERY specifically, because it is the move a policy layer reads.
/// `lift_speed` is derived from `Set` impulses only, so a geyser that arrived
/// with the wrong impulse mode would be invisible to `lifting_candidates` and
/// the CPU would drift at a stage it could reach.
///
/// the goblin is the CONTROL and it is not decoration. He is seated beside
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

    // THE CONTROL: the other seat is a DIFFERENT table.
    //
    // A poison that says "nobody else has one" cannot survive the work that gave everybody one, and
    // keeping it would have meant deleting somebody's recovery to keep a test green.
    //
    // so it asks the question that still discriminates, and asks it of the
    // same field: both seats advertise a way home, and they are DIFFERENT ones.
    // One kit handed to both bodies fails here exactly as before.
    let control_lifts: Vec<&str> = control
        .moves
        .iter()
        .filter(|m| m.frame_data().lift_speed > 0.0)
        .map(|m| m.id.as_str())
        .collect();
    assert!(
        !control_lifts.is_empty(),
        "the seat beside Oiler advertises no way home at all — every fighter on \
         this grid has an up-B, so this seat is wearing something older than the \
         cast"
    );
    assert!(
        !control_lifts.contains(&"oil_geyser"),
        "the seat beside Oiler rides HIS geyser ({control_lifts:?}), so both \
         bodies are wearing one table and the assertions above are that table \
         read twice"
    );
    assert!(
        !control.moves.iter().any(|m| m.id == "oil_geyser"),
        "both seats carry the geyser"
    );
}

/// Check selection-grid fighters against the authored-moveset floor.
///
/// Selectable fighters may intentionally use the stage's unarmed floor, so this
/// test checks the architectural floor/control rather than asserting a roster
/// count. Provider-owned movesets must still reach the live prepared cast.
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
    // THE CONTROL ARM FIRED, AND IT WAS INTENDED.
    assert!(
        silent.is_empty(),
        "these fighters reach the grid with no move timelines of their own: \
         {silent:?}. Every selectable character has authored a table since \
         2026-08-16, so one arriving silent is a repertoire that stopped \
         reaching the registry rather than a peaceful character on a fighting \
         grid"
    );
}

/// WHICH BODY DOES A GRID FIGHTER ACTUALLY MOVE ON? Measured, because the
/// planning ledger's answer ("thirteen bodies are floatier than the
/// fourteenth") predates half the grid and understates the difference.
///
/// ⭐⭐ A SEAT ALWAYS GETS A BODY, AND THE QUESTION IS WHAT IT IS COMPOSED OVER.
/// `MatchRules::body_over` reads `SMASH_FIGHTER_BODY.over(authored.unwrap_or(built))`,
/// and `built` is the seed's `ActorTuning.movement` — `BodyMovementTuning::BASELINE`,
/// the WANDERING-ENEMY body — for every fighter whose character authored no feel
/// of its own. The stage's six numbers land correctly on top of it and disturb
/// nothing else, exactly as `MatchBody` promises; what nobody chose is the base.
///
/// ⛔ THIS IS A RATCHET AND IT MAY ONLY SHRINK. The fix is per-character
/// (`MatchBody`'s own doc refuses a mode-owned gravity in advance), so this list
/// empties one authored fighter at a time and must never grow.
#[test]
fn a_grid_fighter_that_authors_no_feel_is_seated_on_the_wandering_enemys_body() {
    // The wandering-enemy body (`BodyMovementTuning::BASELINE`) against the one
    // every player-driven body in the game rides (`DEFAULT_TUNING`). Spelled out
    // rather than imported so the failure message carries the contrast: the
    // headline is not gravity, it is that a fighter builds speed at an EIGHTH of
    // the player's rate and caps its fall at forty percent of the player's.
    const ENEMY_GRAVITY: f32 = 1450.0;
    const ENEMY_RUN_ACCEL: f32 = 650.0;

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    settle(&mut app);

    let (authored, silent): (Vec<String>, Vec<String>) = {
        let registry = app
            .world()
            .resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>(
        );
        let roster = app
            .world()
            .resource::<ambition_demo_smash::select::SmashRoster>();
        // ⭐⭐ TWO AUTHORITIES, AND THE CENSUS MUST ASK BOTH OR IT CANNOT
        // MEASURE ITS OWN FIX. A character states its body on its catalog row
        // (which is its feel everywhere it appears) OR, for its FIGHTER self
        // only, in its `smash_fighter` facet — which reaches the seat as
        // `MatchParticipant::body` and never touches the definition. Asking only
        // the definition would leave this ratchet stuck at 17 while the work
        // was being done.
        roster.ids().map(str::to_string).partition(|id| {
            registry
                .get(id)
                .is_some_and(|character| character.movement_tuning.is_some())
                || ambition_demo_smash::smash_pack::fighter_body(id).is_some()
        })
    };
    assert!(
        !authored.is_empty(),
        "no fighter on the grid authors a movement feel at all, so this census \
         is measuring an empty registry rather than the content: {silent:?}"
    );

    // AND THE CENSUS IS NOT THE PROOF — it names the population, and a seated
    // body is what says the population actually moves that way. Forcing the grid
    // is the same instrument `the_puppy_slug_forced_onto_the_stage_keeps_the_body_it_authored`
    // uses.
    let subject = silent
        .first()
        .cloned()
        .expect("the ratchet below is vacuous once every fighter authors a feel");
    {
        let registry = app
            .world()
            .resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>(
        );
        assert!(
            registry.get(&subject).is_some() && registry.get("goblin").is_some(),
            "`{subject}` or the opponent is not registered in the shipped host, \
             so this cannot force a seat"
        );
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::select::SmashRoster(vec![
            subject.clone(),
            "goblin".to_string(),
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

    let seated: Vec<(usize, f32, f32, f32)> = {
        let world = app.world_mut();
        let mut query = world.query::<(
            &ambition_platformer2d::actors::character_runtime::MatchSeat,
            &ambition_platformer2d::engine_core::AuthoredMovementTuning,
        )>();
        let mut rows: Vec<(usize, f32, f32, f32)> = query
            .iter(world)
            .map(|(seat, tuning)| {
                (
                    seat.0,
                    tuning.0.gravity,
                    tuning.0.run_accel,
                    tuning.0.max_fall_speed,
                )
            })
            .collect();
        rows.sort_by_key(|(seat, ..)| *seat);
        rows
    };
    assert_eq!(
        seated.len(),
        2,
        "the stage seated {} bodies carrying a movement feel, so nothing below \
         measures a forced seat: {seated:?}",
        seated.len()
    );
    let (_, gravity, run_accel, max_fall_speed) = seated[0];
    assert_eq!(
        (gravity, run_accel),
        (ENEMY_GRAVITY, ENEMY_RUN_ACCEL),
        "`{subject}` authors no movement feel and is seated on gravity \
         {gravity} / run accel {run_accel} / fall cap {max_fall_speed}. This \
         assertion is the CURRENT state, not the wanted one: the wandering-enemy \
         baseline is 1450/650/760 and the player body every human drives is \
         2250/5200/1900. When this fighter authors a body of its own, take it \
         off the ratchet below and pick a different subject"
    );

    // THE RATCHET. Every fighter here is on the wandering-enemy body.
    // MEASURED 2026-08-26, not chosen: 17 of 19, with only George Booul and
    // Mary-O authoring a feel of their own.
    //
    // ⚠ `sanic` is on this list for a DIFFERENT REASON than the other sixteen
    // and authoring him a `MovementTuning` would not move him: he is a
    // `SurfaceMomentum` body, so he has no `AxisManeuverState` and reads none of
    // these numbers. Taking him off costs the other motion model a seat for the
    // state, which is the engine gap the ledger records — not an authoring job.
    const ON_THE_ENEMYS_BODY: usize = 17;
    assert_eq!(
        silent.len(),
        ON_THE_ENEMYS_BODY,
        "{} of {} grid fighters author no movement feel and are therefore \
         seated on the wandering enemy's body: {silent:?}. The {} that do: \
         {authored:?}. ⛔ This number may only FALL — a fighter authors its \
         body beside the moveset it already authors; the stage may not own \
         gravity (see `MatchBody`'s own doc)",
        silent.len(),
        silent.len() + authored.len(),
        authored.len(),
    );
}

/// `SmashRoster::assemble` FILTERS to what the catalog carries, and that is
/// correct behaviour — a host that composes only some providers shows only the
/// fighters it has, which is what lets the bare smash app run at all. It also
/// means a misspelled id is indistinguishable from an absent provider: the grid
/// silently comes up one fighter short and the screen still looks fine.
///
/// fighter is a fighter he asked for and did not get. The SHIPPED host is where
/// the distinction is decidable: it composes every provider, so nothing there is
/// legitimately absent and anything filtered out is a typo.
///
/// this is the fifth hand-made PAIRING in the content, and it is checked
/// against the ASSEMBLED catalog rather than by grepping the RON — the other
/// four are a pedestal's dialogue id to its Yarn node, that node's speaker to
/// the character's name, a character row to the map it lives in, and a row's
/// spritesheet to its manifest. Each was written after the pairing had already
/// gone wrong once.
#[test]
fn every_smash_roster_id_resolves_in_the_shipped_host() {
    use ambition_demo_smash::select::SMASH_ROSTER;

    let mut app = shell_host_app();
    settle(&mut app);
    // THE REGISTRY, NOT THE CATALOG — and this test asked the wrong one for five days. Nobody
    // saw the third, because dropping is the SAFE behaviour and safe behaviour is silent.
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
    // and the roster must not be empty for a different reason than the one
    // above: an import that resolved to an empty slice would satisfy every
    // assertion here while proving nothing.
    assert!(
        SMASH_ROSTER.len() >= 8,
        "the smash roster is down to {} entries, which is fewer than the eight \
         Jon set it at — this check would pass over almost nothing",
        SMASH_ROSTER.len()
    );
}

/// A fighter you picked in Smash does not follow you into Ambition.
///
/// match, quit to the title, enter Ambition — and the body you control is still
/// the Oni Leader, while the Oni Leader NPC standing in the room is a second
/// copy of the same character.
///
/// * `MatchParticipantRoster` is a global resource with no lifetime, so a roster
///   Smash published outlived every Smash route.
/// * `dress_the_primary_player_as_their_own_pick` runs in the plain `Update`
///   schedule gated on nothing but that roster, so it redressed whatever body
///   the next experience put the player in.
///
/// the assertion is the CHARACTER, not the resource. A test that only checked
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
    // The premise: in Smash the pick IS a fighter on the stage.
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
            .resource::<ambition_platformer2d::input::SessionSeatingSource>(),
        &ambition_platformer2d::input::SessionSeatingSource::Devices,
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

    // AND THE NPC IS STILL SOMEBODY ELSE. The visible half of the report was
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
    let token_zero = placed_token(app, 0);
    click(app, token_zero);
    click(
        app,
        layout
            .portrait(index)
            .unwrap_or_else(|| panic!("no portrait cell for {character_id}")),
    );
    let token_one = placed_token(app, 1);
    click(app, token_one);
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
// start the match, only one character spawns in. Additionally it does not let me
// make a CPU vs CPU match."*
//
// the existing coverage cannot see this, and the reason is a category
// error. `a_two_participant_roster_actually_seats_two_bodies` counts seats for
// ONE configuration — the one that works — and every other test in this file
// stops at the route or the session. So four different lobbies that fail in
// three different ways all present to the suite as "not tested", and the two
// that deadlock do it by WAITING, which is indistinguishable from "waiting one
// more tick" to anything that only looks at the end state.
//
// so the assertion is the STAGE REACHED, not a seat count. A permanent
// refusal and a temporary wait are different answers and must never again share
// a shape; `MatchStart` is that distinction made observable.

/// How far a decided lobby got.
///
/// every arm is derived from WORLD STATE — the roster resource, the
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
    ActivationStalled,
    /// The match is live, with this many bodies wearing a `MatchSeat`.
    Activated {
        seats: usize,
    },
}

/// Press one slot's role button `presses` times.
///
/// The keyboard is source zero, so it can own at most one card. Pressing an
/// empty card with a source that already owns another card makes the new card a
/// CPU; a different input source must press if a second human is meant to join.
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
    let token = placed_token(app, slot);
    click(app, token);
    click(
        app,
        layout
            .portrait(index)
            .unwrap_or_else(|| panic!("no portrait cell for {character_id}")),
    );
}

/// Press START and report how far the decision got.
///
/// The budget here is only an upper bound on patience, and reaching it IS the stall verdict.
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
            // COUNT THE ORPHANS TOO, or this oracle goes green over a broken
            // game. Seats are not the whole picture: the session also spawns a
            // home body from the stage's `StartingCharacter`, and while a human
            // seat ADOPTED that body the two were the same thing. Once every
            // fighter is built by the match, an unclaimed home body is a third
            // actor standing on the platform — one the camera follows and the
            // player still drives, while their actual fighter waits elsewhere.
            //
            // A seat count cannot see that, so it would report `Activated{2}` about a stage you
            // cannot play.
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

/// A fighter that just lost a stock comes back UNTOUCHABLE for a moment.
///
/// vulnerable during the first instant of materialization."*
///
/// ⛔⛔ THE GRANT IS THE RULESET'S OWN, NOT A BORROWED `Empowered`. It was the
/// generic timed empowerment a star pickup uses, and that is a SINGLE component:
/// granting respawn protection through it OVERWROTE whatever power-up the body
/// was already carrying, and ending the beat removed the component and every
/// semantic in it. `RespawnGrace` now carries its own clock and publishes
/// `Invulnerability::RESPAWN`, a reason bit — which is asserted here rather than
/// the component, because the reason set is what the damage gate actually reads.
///
/// the second half is the one that keeps this honest: an ELIMINATED fighter
/// is not placed and must not be protected either. Protecting a body that is
/// leaving play would be a grant nobody ever takes back.
#[test]
fn a_respawning_fighter_is_briefly_untouchable_and_an_eliminated_one_is_not() {
    use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};

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

    //  suspending the other seats isolates the grant from the fight, which is
    // what a test of the grant should have done from the start. `ScriptedControl`
    // is the engine's own word for "a sequence drives this body" — the same
    // instrument the opening countdown uses.
    {
        let world = app.world_mut();
        let mut others = world.query_filtered::<
            Entity,
            With<ambition_platformer2d::actors::character_runtime::MatchSeat>,
        >();
        // the VICTIM too. Suspending only its opponent was not enough: a
        // CPU-driven body walks itself off a platform-fighter stage, loses the
        // next stock on its own, and takes a fresh grant with it. Nothing here
        // needs anybody to act.
        let ids: Vec<Entity> = others.iter(world).collect();
        for other in ids {
            // Through a CLAIM, the way every authority holds a body: a bare
            // marker is nobody's hold, and the next release would have nothing
            // to clear.
            world.entity_mut(other).insert((
                ambition_platformer2d::characters::control::ScriptedControl,
                ambition_platformer2d::characters::control::ControlHolds::only(
                    ambition_platformer2d::characters::control::ControlHold::Interlude,
                ),
            ));
        }
    }

    let untouchable = |app: &App| {
        app.world()
            .get::<BodyHealth>(victim)
            .expect("a respawned fighter is still a body")
            .health
            .invulnerable
            .holds(Invulnerability::RESPAWN)
    };

    // ⭐ D192: THE STOCK TICK IS NO LONGER THE RETURN TICK. This asserted the
    // grant two updates after the knockout, which was the same moment only while
    // placement happened on the spend tick. Waiting for the beat is not a
    // weakening — the property under test is "a returning fighter is protected",
    // and it is now checked at the moment the fighter actually returns.
    assert!(
        !untouchable(&app),
        "a fighter still WAITING to respawn must not already hold the grant — it \
         has not come back yet, and a grant spent before the body exists on the \
         stage is a beat the opponent can stand through"
    );
    let mut returned = false;
    for _ in 0..240 {
        app.update();
        if app
            .world()
            .get::<ambition_platformer2d::actor::PendingRespawn>(victim)
            .is_none()
        {
            returned = true;
            break;
        }
    }
    assert!(returned, "the fighter never came back within 240 ticks");
    assert!(
        untouchable(&app),
        "a fighter that just lost a stock came back with no protection at all, so \
         the opponent that took the stock can take the next one at the spawn point"
    );
    let grace = app
        .world()
        .get::<ambition_platformer2d::actor::RespawnGrace>(victim)
        .copied()
        .expect("the reason is published by the grant, so the grant must be there");
    assert!(
        grace.remaining > 0.0,
        "the protection has no time left the moment it is granted, so nothing \
         expires it in a way a player could read"
    );

    // and it wears off. A grant with no end is worse than none.
    //
    // the CLAIM is unchanged — the protection ends — and it is now asserted
    // against the protection instead of against a frame count.
    let mut updates = 0;
    let expired = loop {
        if !untouchable(&app) {
            break true;
        }
        if updates >= 2_000 {
            break false;
        }
        app.update();
        updates += 1;
    };
    assert!(
        expired,
        "the respawn protection never ended in {updates} updates: {:?}",
        app.world()
            .get::<ambition_platformer2d::actor::RespawnGrace>(victim)
            .copied()
    );
    // ⭐ AND THE GRANT LEFT WITH ITS REASON. A reason bit cleared while the
    // component that publishes it stays behind would re-arm on the next tick.
    assert!(
        app.world()
            .get::<ambition_platformer2d::actor::RespawnGrace>(victim)
            .is_none(),
        "the reason was retracted but the grant that publishes it is still on the \
         body, so it comes straight back"
    );
}

/// Run out the opening ceremony.
///
/// The Smash ruleset opens 3 — 2 — 1 — GO: every fighter carries
/// `ScriptedControl` until the count ends, so a test that presses a button on
/// the tick the stage appears is pressing it at a held body and measuring the
/// ceremony rather than the input. Waiting is what a player does too.
fn wait_for_the_round_to_go_live(app: &mut App) {
    for _ in 0..600 {
        let held = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<
                &ambition_platformer2d::actors::character_runtime::MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
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

/// SETTLE THE MATCH THE WAY THE FIGHTERS DO — knock every seat but the first
/// out until the stocks loop reaches its own verdict, and hand it back.
///
/// ⛔⛔ WHY NOT `MatchAbandonRequest`, WHICH IS THIS FILE'S USUAL SHORTCUT: an
/// abandon settles as a `NoContest`, and a `NoContest` is the one verdict the
/// simulation does NOT reach. It comes from a latch made outside the sim that
/// does not rewind, so it cannot be "speculative" in the sense the two guards
/// below are about — and since 2026-08-26 it deliberately skips the confirmation
/// wait for exactly that reason. A guard about predicted verdicts settled by the
/// abandon road would be measuring the carve-out rather than the rule.
///
/// ⛔ AND A KNOCKOUT IS NOT ONE `write_message` + ONE `update`. `App::update()`
/// is a FRAME; the stocks loop runs on SIM TICKS, and a frame can carry none. A
/// message written and left for two frames is a knockout that never happened, so
/// this re-writes until the body visibly answers — `PendingRespawn` appearing,
/// or the match settling.
///
/// ⚠ the respawn wait is the rule `stocks.rs::knock_out` documents:
/// `spend_fighter_stocks` refuses a body that is still `PendingRespawn`, so two
/// knockouts back to back are ONE spent stock without it.
fn settle_the_match_by_knockout(app: &mut App) {
    use ambition_platformer2d::actors::features::stocks_match::StocksMatchSettled;

    // ⛔ HOLD THE CALLER'S BOUNDARY ACROSS OUR OWN TICKS. Nothing in this host
    // maintains `ConfirmedFrameBoundary`, and an ABSENT one confirms everything
    // by its own doc — so a helper that ticked without re-inserting it would
    // hand the caller's "predicted" arm a confirmed world.
    let boundary = app
        .world()
        .get_resource::<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>()
        .copied();
    let running = app
        .world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::ActiveMatch>()
        .cloned()
        .expect("a live match to settle");
    let tick = |app: &mut App| {
        app.update();
        if let Some(boundary) = boundary {
            app.world_mut().insert_resource(boundary);
        }
    };
    let settled = |app: &App| {
        app.world()
            .get_resource::<StocksMatchSettled>()
            .is_some_and(|s| s.settled(&running))
    };

    let victims: Vec<Entity> = {
        let world = app.world_mut();
        let mut q = world.query::<(
            Entity,
            &ambition_platformer2d::actors::character_runtime::MatchSeat,
        )>();
        let mut rows: Vec<(usize, Entity)> = q.iter(world).map(|(e, seat)| (seat.0, e)).collect();
        rows.sort_by_key(|(seat, _)| *seat);
        assert!(
            rows.len() > 1,
            "this match seated {} fighters, so knocking out everybody but the \
             first decides nothing",
            rows.len()
        );
        rows.into_iter().skip(1).map(|(_, e)| e).collect()
    };

    // Bounded well above any authored stock count; `settled` is what ends it.
    for _ in 0..60 {
        if settled(app) {
            return;
        }
        for &body in &victims {
            if settled(app) {
                return;
            }
            for _ in 0..240 {
                if app
                    .world()
                    .get::<ambition_platformer2d::actor::PendingRespawn>(body)
                    .is_none()
                {
                    break;
                }
                tick(app);
            }
            app.world_mut()
                .write_message(ambition_platformer2d::actor::BodyKnockedOut {
                    body,
                    cause: ambition_platformer2d::combat::HitSource::LeftTheWorld,
                });
            // Wait for the sim to ANSWER, not merely for a frame to pass.
            for _ in 0..30 {
                tick(app);
                if settled(app)
                    || app
                        .world()
                        .get::<ambition_platformer2d::actor::PendingRespawn>(body)
                        .is_some()
                    || app
                        .world()
                        .get::<ambition_platformer2d::actor::FighterEliminated>(body)
                        .is_some()
                {
                    break;
                }
            }
        }
    }
    assert!(
        settled(app),
        "sixty rounds of knockouts never decided this match, so the guard below \
         is about nothing"
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

/// ⭐⭐ A QUICK FORWARD SMASH DOES NOT DASH FIRST (D204).
///
/// Jon, W8 playtest: *"When I quickly perform a Forward Smash, the fighter
/// currently travels noticeably before the Forward Smash takes over... I should
/// not effectively dash first and then Smash."*
///
/// ⛔⛔ AND THE CAUSE WAS NOT ORDERING, WHICH IS WHERE THE REPORT POINTED. The
/// probe that produced these numbers showed the smash STARTING on the press tick
/// — recognition was never late — and the fighter then accelerating from a
/// standstill to the full run cap through its own startup, **64 world px, more
/// than a body width**. The travel was not a dash that beat the smash to the
/// press; it was the smash's own frames with the stick still held forward and
/// nothing saying a grounded attack roots its owner.
///
/// ⭐ SO THE ASSERTION IS A PAIR, and the second half is the one that keeps this
/// honest: plain forward with NO attack must still walk. A test that only
/// pinned "the smasher barely moves" would be green against a fighter who
/// cannot move at all.
///
/// Real keys through the real host input stack — the flick that makes the press
/// a smash is the genuine two-tick gesture, not `attack_strong_hint` set by hand.
#[test]
fn a_quick_forward_smash_barely_travels_but_plain_forward_still_walks() {
    /// Ticks to watch after the press. The authored startup is shorter than
    /// this, so the window covers the whole windup and then some.
    const WATCHED_TICKS: usize = 20;
    /// A body width. The report's word was "noticeably", and this is the
    /// threshold that makes it measurable: less than one body is imperceptible,
    /// and the defect travelled more than one.
    const A_BODY_WIDTH_PX: f32 = 30.0;

    let seat_zero_x = |app: &mut App| -> f32 {
        let world = app.world_mut();
        let mut q = world.query::<(
            &ambition_platformer2d::actors::character_runtime::MatchSeat,
            &ambition_platformer2d::engine_core::BodyKinematics,
        )>();
        q.iter(world)
            .find(|(seat, _)| seat.0 == 0)
            .map(|(_, kin)| kin.pos.x)
            .expect("seat zero has a body")
    };
    let seat_zero_move = |app: &mut App| -> Option<String> {
        let world = app.world_mut();
        let mut q = world.query::<(
            &ambition_platformer2d::actors::character_runtime::MatchSeat,
            Option<&ambition_platformer2d::combat::moveset::MovePlayback>,
        )>();
        q.iter(world)
            .find(|(seat, _)| seat.0 == 0)
            .and_then(|(_, pb)| pb.map(|p| p.spec.id.clone()))
    };

    // ── the SMASH: flick forward, then press Attack while it is still held ──
    let travelled_smashing = {
        let mut app = open_the_lobby();
        pick_and_start(&mut app, PREPARED_FIGHTER);
        wait_for_the_round_to_go_live(&mut app);

        Buttonlike::press(&KeyCode::ArrowRight, app.world_mut());
        app.update();
        Buttonlike::press(&KeyCode::KeyX, app.world_mut());
        app.update();
        Buttonlike::release(&KeyCode::KeyX, app.world_mut());

        let from = seat_zero_x(&mut app);
        assert_eq!(
            seat_zero_move(&mut app).as_deref(),
            Some("smash_forward"),
            "the press did not become a forward smash at all, so the travel \
             measured below is some other move's"
        );
        for _ in 0..WATCHED_TICKS {
            app.update();
        }
        (seat_zero_x(&mut app) - from).abs()
    };

    // ── the CONTROL: the same forward hold, no attack ──
    let travelled_walking = {
        let mut app = open_the_lobby();
        pick_and_start(&mut app, PREPARED_FIGHTER);
        wait_for_the_round_to_go_live(&mut app);

        Buttonlike::press(&KeyCode::ArrowRight, app.world_mut());
        app.update();
        app.update();

        let from = seat_zero_x(&mut app);
        assert_eq!(
            seat_zero_move(&mut app),
            None,
            "the control case started a move, so it is not measuring walking"
        );
        for _ in 0..WATCHED_TICKS {
            app.update();
        }
        (seat_zero_x(&mut app) - from).abs()
    };

    assert!(
        travelled_walking > A_BODY_WIDTH_PX,
        "plain forward stopped moving this fighter ({travelled_walking:.1}px in \
         {WATCHED_TICKS} ticks), so the smash assertion below proves nothing — \
         a fighter frozen everywhere would pass it"
    );
    assert!(
        travelled_smashing < A_BODY_WIDTH_PX,
        "a quick forward smash travelled {travelled_smashing:.1}px before its \
         startup finished — more than a body width, which reads as dashing and \
         then smashing. Plain forward went {travelled_walking:.1}px over the \
         same window"
    );
}

/// ⭐⭐ POGO IS ROBOT V3'S, NOT THE STAGE'S (D205).
///
/// Jon, W8 playtest: *"`robot_v3` should have Pogo available in Smash. **Do not
/// make Pogo a universal Smash action.** Robot v3 has Pogo because Robot v3 owns
/// that capability. Another fighter without Pogo should not acquire one merely
/// by entering Smash."*
///
/// ⛔⛔ AND IT USED TO BE THE STAGE'S. `SMASH_FIGHTER_KIT` granted `pogo` as a
/// FLOOR, so all fourteen bodies on the grid got a rebounding down-air by
/// walking onto it. It read as correct from the seat Jon tested — Robot v3 is
/// the fighter that authors pogo — and the defect was entirely in the thirteen
/// it also reached. ⇒ the grant moved to the CEILING, where a character's own
/// kit is what decides.
///
/// ⭐ THE ASSERTION IS A PAIR, and it must be: "Robot v3 has pogo" is true of a
/// stage that hands it to everybody, which is the world this test exists to
/// exclude. A fighter that does NOT author pogo must arrive without one.
#[test]
fn robot_v3_brings_its_pogo_to_smash_and_a_fighter_without_one_does_not() {
    /// A grid fighter that authors no pogo of its own.
    const NO_POGO_FIGHTER: &str = "pointed_polygon";

    let seated_pogo = |character: &str| -> bool {
        let mut app = open_the_lobby();
        pick_and_start(&mut app, character);
        wait_for_the_round_to_go_live(&mut app);
        let world = app.world_mut();
        let mut q = world.query::<(
            &ambition_platformer2d::character::WornCharacter,
            &ambition_platformer2d::engine_core::BodyAbilities,
        )>();
        let row = q
            .iter(world)
            .find(|(worn, _)| worn.id() == character)
            .unwrap_or_else(|| panic!("{character} was not seated in the match"));
        row.1.abilities.pogo
    };

    assert!(
        seated_pogo(PREPARED_FIGHTER),
        "Robot v3 lost the pogo it authors when it entered Smash — the stage's \
         ceiling is trimming a character's own capability"
    );
    assert!(
        !seated_pogo(NO_POGO_FIGHTER),
        "{NO_POGO_FIGHTER} arrived in Smash WITH a pogo it never authored, so \
         the stage is handing the verb out rather than letting the character \
         own it — which also makes the assertion above prove nothing"
    );
}

/// ⭐⭐ EXIT MATCH ENDS THE MATCH AS A NO CONTEST (D207).
///
/// Jon, W8 playtest: *"During an active Smash match, the system/pause menu needs
/// an explicit `Exit Match`... It should not award an ordinary winner/loser
/// result... add it at the semantic match-outcome layer rather than encoding it
/// as some special winner value."*
///
/// ⭐ SO THE ASSERTION IS THE VERDICT'S IDENTITY, not its absence of a winner.
/// `NoContest` and `Draw` both have no winner, and a test that asked
/// `winner.is_none()` would have been green against the exact conflation Jon
/// asked to remove — which is the shape the message used to have.
///
/// Real Escape, real arrows, real Enter, through the host's own pause menu.
#[test]
fn exit_match_ends_the_match_as_a_no_contest_and_returns_to_select() {
    use bevy::ecs::message::Messages;

    let mut app = open_the_lobby();
    pick_and_start(&mut app, PREPARED_FIGHTER);
    wait_for_the_round_to_go_live(&mut app);
    assert_eq!(
        active_route(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        "the match never reached the stage, so there is nothing to exit"
    );

    // Open the system menu and pick the row the STAGE contributed. Found by
    // label rather than by index: the row set grows, and a hand-counted number
    // of Down presses is a test pinning a layout.
    tap(&mut app, KeyCode::Escape);
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::game_shell::ShellAbandonOffer>()
            .is_some(),
        "a live match offered the pause menu nothing to exit, so the row is not \
         there to pick"
    );
    tap(&mut app, KeyCode::ArrowDown);
    confirm(&mut app);

    let verdict = {
        let messages = app
            .world()
            .resource::<Messages<ambition_platformer2d::actor::StocksMatchDecided>>();
        let mut cursor = messages.get_cursor();
        cursor.read(messages).last().cloned()
    }
    .expect("Exit Match did not end the match at all");
    assert_eq!(
        verdict.outcome,
        ambition_platformer2d::actor::MatchVerdict::NoContest,
        "an abandoned match was decided as something the fighters achieved"
    );

    // ⭐⭐ AND IT GOES HOME ON THE PRESS. Jon, 2026-08-26: *"skip the no
    // contest presentation for now and just exit to the character select menu
    // immediately."* The budget is what makes this an assertion rather than a
    // restatement: `RETURN_TO_SELECT_AFTER` is 4.5 SECONDS, so a countdown road
    // cannot pass a loop this short, and the old behaviour would have.
    const IMMEDIATE: usize = 30;
    let mut frames = 0;
    while frames < IMMEDIATE {
        if active_route(&app).as_deref() == Some(ambition_demo_smash::SMASH_SELECT_ROUTE) {
            break;
        }
        app.update();
        frames += 1;
    }
    assert_eq!(
        active_route(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_SELECT_ROUTE),
        "an exited match kept the player on the stage for more than {IMMEDIATE} frames, \
         so it is still going home by the winner-card countdown"
    );
    // ⛔ AND NO CARD WAS PUT UP ON THE WAY OUT. A NO CONTEST readout is the
    // presentation Jon asked to skip; asserting only the route would stay green
    // with the card flashing for four and a half seconds first.
    assert!(
        app.world()
            .resource::<ambition_platformer2d::presentation::HudReadouts>()
            .get(&ambition_demo_smash::SMASH_ANNOUNCE_HUD_SLOT.into())
            .is_none(),
        "exiting the match still announced a result, so the no-contest card is \
         still in the road out"
    );
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::game_shell::ShellAbandonOffer>()
            .is_none(),
        "the offer outlived its match, so the select screen's own pause menu now \
         offers to exit a match that is not running"
    );
}

/// ⭐⭐ THE MATCH CLOCK DOES NOT START UNTIL THE CAST IS RELEASED (D212).
///
/// ⛔⛔ THE DEFECT THIS PINS: the timeout and the item cadence both read
/// `ActiveMatch::ticks_since_activation`, which counts from the tick the cast was
/// BUILT. Every second of "3, 2, 1" came off the clock the fighters were about
/// to play against, and each consumer patched around the ceremony its own way —
/// the timeout not at all, the spawner with a hand-written `elapsed == 0`.
///
/// ⭐ THIS IS THE PRODUCTION PATH FOR THE CEREMONY HALF, and it lives here
/// rather than beside the clock's other tests because `PreparedMatch` has no
/// constructor outside `prepare_match`: a unit fixture could only get a
/// countdown by having one added to a production type. A real match has a real
/// one.
#[test]
fn the_match_clock_does_not_start_until_the_cast_is_released() {
    use ambition_platformer2d::actors::character_runtime::live_match_clock::LiveMatchTicks;

    let mut app = open_the_lobby();
    pick_and_start(&mut app, PREPARED_FIGHTER);

    // Step until the cast is BUILT — the point the old reading started counting
    // from — and check the ceremony is genuinely still running, or the
    // assertion below is about nothing.
    let mut held = 0usize;
    for _ in 0..600 {
        app.update();
        held = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<
                &ambition_platformer2d::actors::character_runtime::MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
            >();
            q.iter(world).count()
        };
        if held > 0 {
            break;
        }
    }
    assert!(
        held > 0,
        "no fighter was ever held by an opening ceremony, so this match has none \
         and the test cannot see one excluded"
    );

    let counted = |app: &App| {
        let active = app
            .world()
            .get_resource::<ambition_platformer2d::actors::character_runtime::ActiveMatch>()
            .cloned()
            .expect("a match is live");
        app.world().resource::<LiveMatchTicks>().of(&active)
    };

    // ⭐ THE CEREMONY RUNS AND THE CLOCK DOES NOT. Ticks are passing — the cast
    // exists, the world is moving — and none of them are match time.
    //
    // ⛔ ASKED WHILE THE HOLD IS ON, not for a fixed twenty frames. Twenty was a
    // number that fit inside a three-second ceremony, so it silently encoded the
    // ceremony's LENGTH into a test about what the clock does DURING one; the
    // moment the ceremony got shorter the window outlasted it and the failure
    // read as "the countdown came off the clock" when nothing of the sort had
    // happened. The condition is the hold, and the hold is observable.
    let mut held_frames = 0;
    for _ in 0..600 {
        let still_held = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<
                &ambition_platformer2d::actors::character_runtime::MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
            >();
            q.iter(world).count() > 0
        };
        if !still_held {
            break;
        }
        held_frames += 1;
        assert_eq!(
            counted(&app),
            0,
            "the opening countdown came off the match clock, so a match with a \
             ceremony starts already spent"
        );
        app.update();
    }
    assert!(
        held_frames > 0,
        "the hold was already off before a single frame of the ceremony was \
         observed, so nothing above measured the clock during one"
    );

    wait_for_the_round_to_go_live(&mut app);
    for _ in 0..30 {
        app.update();
    }
    assert!(
        counted(&app) > 0,
        "the clock never started, so this match can never time out"
    );
}

/// ⭐⭐ AND THE ROW IS WITHDRAWN THE MOMENT THE MATCH SETTLES (D211).
///
/// The GPT 5.6 correction pass: withdraw `Exit Match` once
/// `StocksMatchSettled::settled(active)`. A decided match is STILL ACTIVE — the
/// winner card is up and the return countdown is running — so `ActiveMatch`
/// alone cannot answer this, and the row stood for those four seconds offering
/// to stop something already stopped. Pressing it did nothing: the abandon
/// road's once-only latch discards a verdict for a match that already has one.
///
/// ⛔ THE ASSERTION HOLDS ALL THREE AT ONCE — still on the stage, `ActiveMatch`
/// still installed, offer gone. Drop either of the first two and this passes for
/// the wrong reason, because the offer is also withdrawn when the route changes
/// and when the match resource goes away, and both of those happen shortly
/// after.
#[test]
fn a_settled_match_withdraws_the_exit_row_while_the_winner_card_is_still_up() {
    let mut app = open_the_lobby();
    pick_and_start(&mut app, PREPARED_FIGHTER);
    wait_for_the_round_to_go_live(&mut app);
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::game_shell::ShellAbandonOffer>()
            .is_some(),
        "a live match offered nothing to exit, so this test cannot see it withdrawn"
    );

    // Settle it by the engine's own match-level verb — the same one the row
    // itself writes — rather than by reaching into the settlement resource.
    let running = app
        .world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::ActiveMatch>()
        .cloned()
        .expect("a live match");
    app.world_mut().insert_resource(
        ambition_platformer2d::actors::features::stocks_match::MatchAbandonRequest::stop(&running),
    );
    app.update();
    app.update();

    let active = app
        .world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::ActiveMatch>()
        .cloned();
    assert!(
        active.is_some(),
        "the match resource was gone already, so this frame proves nothing about \
         withdrawing on SETTLEMENT"
    );
    assert_eq!(
        active_route(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        "the route changed already, so this frame proves nothing about \
         withdrawing on SETTLEMENT"
    );
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actors::features::stocks_match::StocksMatchSettled>()
            .is_some_and(|settled| settled
                .settled(active.as_ref().expect("checked just above"))),
        "the match did not settle, so the condition under test never became true"
    );

    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::game_shell::ShellAbandonOffer>()
            .is_none(),
        "the match is over and the pause menu still offers to exit it — a row \
         whose press the abandon latch discards"
    );
}

/// ⭐⭐ A FIGHTER COMES BACK WITH ITS RECOVERY, BEFORE IT LANDS (review §10).
///
/// The GPT review, on the recovery budget: *"A fresh or stock-respawned airborne
/// body must start with its intended recovery charge. Do not rely on a later
/// landing event to initialize it. Test the real stock-respawn road: lose stock
/// → respawn airborne → before landing → recovery Up-B is available once."*
///
/// ⛔⛔ AND "BEFORE IT LANDS" IS THE WHOLE TEST. `BodyJumpState::default()` is
/// spent — deliberately, so a body constructed in an unknown state cannot
/// recover forever — so a budget that were only filled on the landing refresh
/// would look correct in every match that got as far as touching the ground, and
/// wrong exactly where it matters: falling toward the blast zone with a stock
/// just spent. This asserts the charge WHILE the body is still in the air.
#[test]
fn a_respawned_fighter_can_recover_before_it_has_landed() {
    use bevy::ecs::message::Messages;

    let mut app = open_the_lobby();
    pick_and_start(&mut app, PREPARED_FIGHTER);
    wait_for_the_round_to_go_live(&mut app);

    let victim = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            Entity,
            With<ambition_platformer2d::actors::character_runtime::MatchSeat>,
        >();
        q.iter(world).next().expect("a live match has fighters")
    };

    // Spend the charge FIRST, so the assertion below cannot be satisfied by a
    // body that simply never used one.
    {
        let world = app.world_mut();
        let mut jump = world
            .get_mut::<ambition_platformer2d::engine_core::BodyJumpState>(victim)
            .expect("a seated fighter carries the jump cluster");
        jump.recovery_charges = 0;
    }

    // One knockout: the roster opens with three stocks, so this is a respawn.
    app.world_mut()
        .resource_mut::<Messages<ambition_platformer2d::actor::BodyKnockedOut>>()
        .write(ambition_platformer2d::actor::BodyKnockedOut {
            body: victim,
            cause: ambition_platformer2d::combat::HitSource::LeftTheWorld,
        });
    // ⭐ THE RETURN TICK, AND EXACTLY IT. The respawn platform catches this
    // fighter on the very next tick, so a test that settled first would be
    // measuring the LANDING refresh — measured: airborne at t+0, on the platform
    // from t+1.
    //
    // D192 moved t+0. It used to be one update after the knockout; now the body
    // waits out the authored beat first, and the update that CLEARS
    // `PendingRespawn` is the one placement runs in — so breaking the loop there
    // lands on t+0 exactly, not the tick after it.
    let mut placed = false;
    for _ in 0..240 {
        app.update();
        if app
            .world()
            .get::<ambition_platformer2d::actor::PendingRespawn>(victim)
            .is_none()
        {
            placed = true;
            break;
        }
    }
    assert!(placed, "the fighter never came back within 240 ticks");

    let (charges, grounded) = {
        let world = app.world();
        (
            world
                .get::<ambition_platformer2d::engine_core::BodyJumpState>(victim)
                .expect("the respawned body still has its jump cluster")
                .recovery_charges,
            world
                .get::<ambition_platformer2d::engine_core::BodyGroundState>(victim)
                .is_some_and(|g| g.on_ground),
        )
    };
    assert!(
        !grounded,
        "the fighter respawned already standing on something, so this measures \
         the LANDING refresh rather than the respawn — which is the exact \
         confusion the review asked to rule out"
    );
    assert!(
        charges >= 1,
        "a fighter that just lost a stock came back airborne with {charges} \
         recoveries, so its only way home is a move it cannot throw"
    );
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

/// A second registered fighter for tests that must distinguish seat ordering.
const OTHER_PREPARED_FIGHTER: &str = ambition_demo_smash::SMASH_GEORGE_BOOUL;

/// The configuration that works. One person, one CPU, both registered.
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

    // AND THE CPU FIGHTS. The discriminator between "seated CPUs never act"
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

/// A CPU in an earlier slot than the person.
///
/// The invariant: seat ORDER cannot decide whether a match starts. Every
/// seat is built the same way from the same prepared plan, so which card holds
/// the person is a detail of the lobby and nothing else.
///
/// Seating ADOPTED the primary player's existing body for a human seat and refused until that body
/// already wore the picked fighter; `dress_the_primary_player_as_their_own_pick` dressed it as
/// `participants.first()` rather than as the participant bound to primary input. A CPU first meant
/// the body wore the CPU's fighter, the human seat waited for a costume it would never be given,
/// and one seat waiting meant no seat was built — the resolve pass returned from the whole
/// system.
///
/// What survives is the requirement, which no longer depends on any of those mechanisms.
#[test]
fn a_cpu_ordered_before_the_person_still_starts_the_match() {
    let mut app = open_the_lobby();
    cycle_role(&mut app, 0, 2); // Absent → Controller → CPU, freeing the source
    cycle_role(&mut app, 1, 1); // …which the person then takes
                                // Different fighter IDs make seat ordering observable.
    pick_fighter(&mut app, 0, OTHER_PREPARED_FIGHTER);
    pick_fighter(&mut app, 1, PREPARED_FIGHTER);

    assert_eq!(
        start_and_report(&mut app),
        MatchStart::Activated { seats: 2 },
        "a lobby whose first card is a CPU did not seat both fighters, so seat \
         ORDER is deciding whether a match starts"
    );
}

/// *"it does not let me make a CPU vs CPU match, and it is very important that
/// that is expressible and easy to do."*
///
/// That clause read like product policy and was really an engine limitation wearing a
/// rationale: with no human seat nothing adopted the session's home body, and the stage would
/// open with an unowned controllable actor standing beside the match.
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

    // AND THEY MUST ACTUALLY FIGHT. Asserting the match ACTIVATES is the trap this file was
    // written to avoid one level down: two bodies that seat correctly and then stand still satisfy
    // every count anybody thought to make.
    let start: Vec<f32> = seat_positions(&mut app);
    for _ in 0..300 {
        app.update();
    }
    let moved: f32 = seat_positions(&mut app)
        .iter()
        .zip(&start)
        .map(|(now, then)| (now - then).abs())
        .fold(0.0, f32::max);

    // AND SOMETHING MUST BE LOOKING AT THEM.
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
        // Keep the tolerance narrow enough that a stale origin snapshot cannot
        // satisfy the framing assertion when cast resolution returns early.
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

/// A refusal the ENGINE produced reaches the PERSON who chose the roster.
///
/// Nothing displayed it.
///
/// the refusal is INSERTED here rather than provoked through the grid, and
/// that is deliberate. Provoking it needs an id the composition cannot seat,
/// and the grid now filters to exactly the ids it CAN seat — so the only honest
/// way to reach this arm through the UI is to break the filter, which would
/// make the test a test of the filter. What is being pinned is the BINDING: a
/// standing refusal is on screen, in the player's words, instead of "Ready".
/// The refusal's own content is pinned where it can never go vacuous:
/// `prepared_match::tests::an_unbuildable_character_is_refused_by_name`, which
/// names an id no composition will ever register.
///
/// its host-level twin is DELETED, and the deletion is the point. That
/// test picked `npc_emmy_noether` — a portrait the grid drew and seating could not
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

/// THE SECOND MATCH OF A SESSION MUST ALSO BE A MATCH.
///
/// *"a fresh restart and then player vs cpu works, but the next match does
/// not work … there is some bad global state, we need to be careful about this,
/// over-relying on global state has happened several times."* What he sees is
/// fighters standing still in the air with the menu still responding — so the
/// bodies are built and something that should be driving them is not.
///
/// `coming_back_to_the_select_screen_offers_a_fresh_match` was green over
/// this the whole time, and the reason is the exact trap this repo keeps
/// falling into: it asserts the screen is RESET — the roster gone, the slots
/// empty, START not still asked for — and every one of those is a PRESENCE
/// check. A second match that opens and then never moves satisfies all of them.
/// Only an assertion about MOTION catches it.
///
/// So this drives the whole cycle a person drives: decide a match, watch it
/// move, quit to the title, decide another, and watch THAT move.
#[test]
fn a_second_match_in_the_same_session_still_fights() {
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

    // BACK TO THE SELECT SCREEN, not the title: that is the shorter loop a
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

    // Exercise both in-experience retirement and full launcher exit.
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    let third = run_a_match(&mut app, "the third, after quitting to the title");
    assert!(
        third > 1.0,
        "a match started after quitting to the title seated two fighters and \
         they never moved ({third:.2}px)"
    );
}

/// Quitting a paused match must restore global mode and clock state before the
/// next session. The test exits through the bare title command, starts a CPU-only
/// match with no primary player to repair time state, and requires its fighters
/// to move.
#[test]
fn quitting_a_paused_match_to_the_title_does_not_freeze_the_next_one() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    app.world_mut()
        .spawn(bevy::input::gamepad::Gamepad::default());
    for _ in 0..ambition_app::app::shared_host_startup_ticks() * 2 {
        app.update();
    }
    settle(&mut app);

    // This fixture only asks for one local human. The keyboard/source-zero
    // drives that card; the second card is deliberately CPU.
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

    // MID-MATCH, AND PAUSED — which is how a person reaches "Quit to Title"
    // at all: the row only exists on the pause menu.
    //
    // The route reached the launcher, the resource census came back CLEAN, and the world was
    // still stopped — so the next match built its fighters, seated them, framed them, and never
    // advanced a tick. *"the characters are just stuck in air"*, with a menu that still
    // answered, because menus do not run on sim time.
    //
    // Asserting through the writer that already got it right would pin the fix and leave the gap.
    // This is the F10 path, and the in-world system menu's, and the scripted sweep's.
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
    // The two measurements are one assertion only if presentation cannot fail on its own, and in
    // this repo it can and does, silently.
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
/// not `seat_positions`, and the difference is the whole point. That
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

/// A MATCH THAT HAS JUST ENDED IN A DRAW MUST NOT RESTART ITSELF.
///
/// In a platform fighter that is simply false. `take_eliminated_fighters_out_of_play` DESPAWNS
/// an eliminated fighter, and a simultaneous final-stock ring-out is a supported draw — so a
/// match that has legitimately just finished sits at `ActiveMatch = current`, zero seats, for
/// the whole 4.5 seconds the winner card is up. Activation fell through and rebuilt the entire
/// prepared cast with fresh stocks, underneath the announcement.
///
/// the KO is injected, for the reason `stocks.rs` gives at length: earning
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
    // ONE update, then read: the decision lands on the very first tick and a
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
            decided[0].outcome,
            ambition_platformer2d::actor::MatchVerdict::Draw,
            "a simultaneous final-stock ring-out was not a draw"
        );
    }
    // A stocks verdict names the match it is about, so the honest question is whether the one
    // that is running has been decided.
    assert!(
        ambition_platformer2d::actors::features::stocks_match::the_live_match_is_settled(
            app.world()
        ),
        "the ruleset does not consider this match settled"
    );

    let seats_after_the_draw = seat_positions(&mut app).len();
    assert_eq!(
        seats_after_the_draw, 0,
        "the eliminated fighters were not despawned, so this test is not in the \
         state it is about — zero seats with a live receipt is the whole premise"
    );

    // AND NOW THE FRAMES THE WINNER CARD IS UP FOR. Two seconds, well
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

/// A phone can work this lobby: the prompt says a surface owns the screen,
/// and it says so in the screen's own words.
///
/// The touch overlay reads exactly this resource to decide what is drawn and
/// what is tappable. `ControlContextKind::Empty` hides the move stick AND the
/// confirm buttons — and a hidden node takes no drags — so on `Empty` the only
/// live controls are Menu and Back, and a screen steered by a cursor cannot be
/// worked at all.
///
/// both halves, because they have different owners and either can regress
/// alone. The context comes from smash's capturing `SELECT_CONTEXT` claim
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

/// THE COORDINATES `capture_scene` DOCUMENTS STILL SEAT A MATCH.
///
/// It is not: [`click`] is `SelectCursors::seat_mut(0).move_to(rect.center())` and THEN
/// `tap(Enter)`, and the POSITION is the load-bearing half. A key is an edge with no position,
/// so the tool's `Enter` fired wherever the cursor already sat, all four slots stayed `NOT
/// PLAYING`, and every `--route smash_gameplay` capture for days photographed an empty stage —
/// which is why no Smash change had ever been looked at.
///
/// so the tool grew the one step that carries a position: `touch:XxY`,
/// two real `TouchInput` messages down the phone road. This is the guard on
/// the literal numbers its doc block prints. They are literals on purpose —
/// re-deriving them here would pin the LAYOUT, which `layout::tests` already
/// does, and would agree with a stale doc forever.
///
/// the whole road, not the arithmetic: a finger through this host's real
/// input stack, ending in a `MatchParticipantRoster` of two CPUs on two
/// fighters that each AUTHOR A REPERTOIRE — which is the state a watcher has to
/// be able to photograph to answer "do the two kits behave differently at all".
#[test]
fn the_capture_tools_documented_taps_seat_two_cpus_on_two_fighters() {
    use ambition_demo_smash::select::SlotPick;
    use bevy::input::touch::{TouchInput, TouchPhase};

    // The `--press touch:...` list in `capture_scene`'s header, in order.
    const ROLE_BUTTON_0: Vec2 = Vec2::new(167.0, 523.0);
    const ROLE_BUTTON_1: Vec2 = Vec2::new(482.0, 523.0);
    // ⛔⛔ AND THE FIRST ONE HAD DRIFTED AGAIN — measured 2026-08-26, and the
    // check below is why nobody saw it. `PORTRAIT_A` was landing on grid CELL 0
    // (`player_robot_v3`), not cell 1, so the command this row exists to
    // document answered *"do the two AUTHORED kits read differently"* with the
    // wrong half of the pair. The assertion only said the two picks DIFFER, and
    // two wrong fighters differ perfectly well.
    //
    // ⭐ SO THE FIGHTERS ARE NAMED NOW, not just counted. A grid cell is a
    // POSITION and the roster behind it grows — appending a fighter moves cell
    // one — so a literal that is only checked for being ON the grid is a literal
    // that goes quietly wrong every time the cast changes.
    //
    // Re-derived from `SelectLayout::portrait` centres, not by eye: cell 1 is
    // (559.05, 105.25) and cell 4 is (801.9, 105.25).
    const PORTRAIT_A: Vec2 = Vec2::new(559.0, 105.0);
    const PORTRAIT_B: Vec2 = Vec2::new(802.0, 105.0);
    /// The pair the documented capture is FOR: the demo's own authored fighter
    /// against Ambition's own. Named, because a cell index is not a character.
    const WANTED: [&str; 2] = ["smash_george_booul", "npc_pirate_admiral"];
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

    // Tokens are state-derived now: both CPUs begin on Random, offset there by
    // slot so they remain individually manipulable. Ask the live state where
    // they are instead of maintaining a second set of layout constants.
    let token_zero = placed_token(&app, 0).center();
    let token_one = placed_token(&app, 1).center();
    tap(&mut app, token_zero);
    tap(&mut app, PORTRAIT_A);
    tap(&mut app, token_one);
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
    // ⭐⭐ WHICH fighters, not merely two of them. See `WANTED`.
    let chosen: Vec<String> = {
        let grid = app
            .world()
            .resource::<ambition_demo_smash::select::SmashRoster>();
        picks
            .iter()
            .map(|pick| match pick {
                Some(SlotPick::Fighter(cell)) => grid
                    .ids()
                    .nth(*cell)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("<cell {cell} is off the grid>")),
                other => format!("{other:?}"),
            })
            .collect()
    };
    assert_eq!(
        chosen,
        WANTED.map(str::to_string).to_vec(),
        "the documented taps seat {chosen:?}, and the capture they document is \
         supposed to put the demo's authored fighter beside Ambition's. A grid \
         cell is a POSITION: appending a fighter moves cell one, so these \
         literals go wrong every time the cast changes and only naming the \
         characters catches it"
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

    // AND THE CLAIM THAT SURVIVES A ROSTER REORDER: both seats wear a
    // fighter that AUTHORS ITS OWN MOVE TIMELINES.
    //
    // Every reorder of `SMASH_ROSTER` re-flows the grid under these two literal points, and
    // "different" stays true however far they slide: for months they sat on Sanic, whose repertoire
    // is the shared stand-in table, and the documented command kept answering this row's standing
    // product question with a body that has nothing to show.
    //
    // the property the command is FOR is not "two cells" but "two authored
    // kits", so that is what is asserted, against the same oracle
    // `smash_roster_movesets::the_grid_fighters_with_a_real_repertoire_only_grow`
    // ratchets — `PreparedCharacterDefinition::authored_moveset`, the field the
    // stage actually reads. deliberately NOT two hard-coded ids: naming
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
    // statement about a field nothing fills.
    //
    //  the guard asks the hazard directly instead: is the field FILLED for anybody?
    assert!(
        app.world()
            .resource::<ambition_demo_smash::select::SmashRoster>()
            .ids()
            .any(|id| registry
                .get(id)
                .is_some_and(|definition| definition.authored_moveset.is_some())),
        "no fighter on the grid reports an authored moveset, so the check below \
         is a statement about a field nothing fills"
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

/// Fighters from different home games must use this match's common health pool
/// when reporting damage percent. The test applies the same damage through the
/// real channel to fighters with different authored home-game health scales and
/// requires equal percent readings.
#[test]
fn a_fighter_from_another_game_reads_its_percent_against_this_stages_pool() {
    use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};

    // The grid seats Tall Mary-O rather than the short form.
    const CROSSOVER: &str = "mary_o_tall";
    // Use a native fighter with a different home-game health scale.
    const NATIVE: &str = "player_robot_v3";

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    settle(&mut app);

    // THE POISON, and without it the assertion below is unfalsifiable.
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
        let token = placed_token(&app, slot);
        click(&mut app, token);
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
    //
    // That is the mechanic working; it just makes a one-shot injection a coin flip. Each
    // attempt is the original test — the same hit at each body, four frames to resolve — and
    // the loop stops as soon as both have registered one.
    const BITE: i32 = 20;
    let mut after = bodies.clone();
    for _ in 0..12 {
        bite_both(&mut app, &bodies, BITE);
        after = seated(&mut app);
        if after.iter().all(|(_, _, _, taken, _)| *taken >= BITE) {
            break;
        }
        // Let any evade window and its endlag expire before trying again.
        for _ in 0..30 {
            app.update();
        }
    }
    for (id, _, _, taken, _) in &after {
        assert!(
            *taken >= BITE,
            "the hit never reached {id} in twelve attempts, so this test \
             measures nothing: {after:?}"
        );
    }
    fn bite_both(app: &mut App, bodies: &[(String, Entity, i32, i32, f32)], bite: i32) {
        for (_, body, ..) in bodies {
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
                    damage: bite,
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
    }
    // WHAT ONE POINT OF DAMAGE READS AS, per fighter. Compared as a scale
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

/// AND WHAT DOES A FIGHTER WITH NO TABLE ACTUALLY SWING?
///
/// `smash_roster_movesets`'s kit census reads the CHARACTER, and four of the
/// fourteen resolve to nothing there — Mary-O, Sanic, Alice and Bob author no
/// moveset and no action set, because standing in a room and talking is what
/// they were authored for. Read at the character, every one of their sixteen
/// presses is silent.
///
/// that is not what a player gets, and the difference is the stage. The seat is
/// armed on the way IN, by this experience's roster preparation
/// (`smash_seating_melee`), so a report that stopped at the character would say
/// four fighters cannot attack — which is false, and exactly the kind of
/// true-measurement-wrong-conclusion this repo keeps paying for.
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
            "mary_o_tall".to_string(),
            "npc_alice".to_string(),
        ]));
    decide_a_solo_match(&mut app);
    settle(&mut app);
    for _ in 0..90 {
        app.update();
    }

    // THE THIRD ROUTE: what does this experience hand a character that states
    // no kit of its own? An empty seat below is then either a preparation that
    // handed nothing over or one that did not reach the publisher.
    eprintln!(
        "[unarmed declaration] smash_seating_melee = {:?}",
        ambition_demo_smash::smash_seating_melee()
    );

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
            // THE SECOND ROUTE, and the report is wrong without it. A
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

/// melee and projectiles ask DIFFERENT QUESTIONS about who may be hit,
/// and only one of them knows what a match is:
///
/// ```text
///   melee        `targeting::team_allows_damage(attacker_team, victim_team)`
///                — when both bodies are SEATED, the teams decide
///   projectile   `damage_lands(firer_faction, victim_faction, ..)`
///                — factions decide; `MatchTeam` appears nowhere in
///                  `projectile/systems.rs`
/// ```
///
/// So a stage that seats every fighter under ONE faction — which is what a
/// crossover grid does, since a Hall NPC and a demo protagonist are not enemies
/// of each other outside the match — gives melee a clean answer and leaves every
/// shot spared as an ally.
#[test]
fn report_the_factions_and_teams_a_seated_fighter_carries() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;

    let mut app = shell_host_app();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    settle(&mut app);
    app.world_mut()
        .insert_resource(ambition_demo_smash::select::SmashRoster(vec![
            "perfect_cellular_automaton".to_string(),
            "goblin".to_string(),
        ]));
    decide_a_solo_match(&mut app);
    settle(&mut app);
    for _ in 0..90 {
        app.update();
    }

    let world = app.world_mut();
    let mut query = world.query::<(
        &MatchSeat,
        Option<&ambition_platformer2d::combat::components::ActorFaction>,
        Option<&ambition_platformer2d::combat::targeting::MatchTeam>,
    )>();
    let mut rows: Vec<(usize, String, String)> = query
        .iter(world)
        .map(|(seat, faction, team)| {
            (
                seat.0,
                format!("{faction:?}"),
                team.map_or("-".to_string(), |t| t.as_str().to_string()),
            )
        })
        .collect();
    rows.sort_by_key(|(seat, ..)| *seat);
    eprintln!("[seat factions] {rows:?}");
    assert_eq!(rows.len(), 2, "the stage seated {} fighters", rows.len());
}

/// A FIGHTER WITH NO DASH STILL RUNS THE LENGTH OF THE STAGE.
///
/// also remove everyone's ability to dash in smash. Dash should be an ability for
/// ambition, it doesn't map into a smash vocabulary."* `AbilitySet::dash` left
/// [`ambition_demo_smash::SMASH_FIGHTER_KIT`] that day.
///
/// The gate now reads `dash || dodge` (`apply_intent`), and the kit assertion below is what
/// stops this file passing over a fighter that quietly lost both.
#[test]
fn a_fighter_with_no_dash_still_covers_ground_on_the_stage() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::engine_core::BodyKinematics;

    let mut app = open_the_lobby();
    decide_a_solo_match(&mut app);
    for _ in 0..300 {
        app.update();
        if active_route(&app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
            break;
        }
    }
    settle(&mut app);

    let body = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        let mut rows: Vec<(usize, Entity)> = q.iter(world).map(|(e, s)| (s.0, e)).collect();
        rows.sort_by_key(|(seat, _)| *seat);
        assert!(!rows.is_empty(), "a started match seated no fighter at all");
        rows[0].1
    };

    // NON-VACUITY, and it is the whole reason the distance below means
    // anything. A fighter that still carried the dash would cover this ground
    // whatever the kit said, and a fighter that had lost the dodge along with it
    // would also pass a pure distance check while being the actual defect.
    {
        let abilities = app
            .world()
            .get::<ambition_platformer2d::engine_core::BodyAbilities>(body)
            .expect("a seated fighter carries its ability set")
            .abilities;
        assert!(
            !abilities.dash,
            "this fighter still owns the traversal burst, so the distance below \
             proves nothing about a stage that removed it"
        );
        assert!(
            abilities.dodge,
            "the fighter lost the DODGE along with the dash — that is the silent \
             half of D146 and it is what `apply_intent`'s gate exists to prevent"
        );
        assert!(
            abilities.move_horizontal,
            "running is `move_horizontal` and this fighter does not have it"
        );
    }

    wait_for_the_round_to_go_live(&mut app);
    let x = |app: &App, body: Entity| app.world().get::<BodyKinematics>(body).unwrap().pos.x;
    let start = x(&app, body);

    // Hold a direction — ordinary locomotion, no burst press anywhere.
    Buttonlike::press(&KeyCode::ArrowRight, app.world_mut());
    let mut covered = 0.0_f32;
    let mut updates = 0;
    // the ceiling is a GIVE-UP bound, not the measurement: the loop exits the
    // instant the property holds, and the assertion below is about the distance.
    while updates < 1_200 {
        app.update();
        updates += 1;
        covered = (x(&app, body) - start).abs();
        if covered >= 200.0 {
            break;
        }
    }
    Buttonlike::release(&KeyCode::ArrowRight, app.world_mut());

    assert!(
        covered >= 200.0,
        "a fighter holding a direction covered only {covered:.1}px in {updates} \
         updates. The stage is one contiguous 480px platform — crossing it is \
         `move_horizontal` against the body's own top speed and nothing else — so \
         a fighter that cannot cross it has lost its LOCOMOTION, not its dash"
    );
    eprintln!("[d146] a dash-less fighter covered {covered:.1}px in {updates} updates");
}

/// THE RUNNING ATTACK COMES OUT OF THE RUN, ON A FIGHTER THAT CANNOT DASH.
///
/// the two phases are the same press. X with a direction held is a
/// forward tilt from a standstill and the dash attack out of a run, so the only
/// thing that differs between them is the gait — which is what makes this a
/// measurement of the gait rather than of the move table. Both phases assert the
/// `running` fact AT THE MOMENT OF THE PRESS, so neither leans on a tick count.
#[test]
fn a_dash_less_fighter_presses_attack_out_of_a_run_and_gets_the_dash_attack() {
    use ambition_platformer2d::combat::moveset::{ActorMoveset, MovePlayback};
    use ambition_platformer2d::engine_core::BodyMotionFacts;

    const ATTACK_KEY: KeyCode = KeyCode::KeyX;

    let (mut app, body) = a_person_fighting_as(OTHER_PREPARED_FIGHTER);

    // Non-vacuity, both halves. A fighter that kept the traversal dash would
    // reach the old selector's state and prove nothing; a fighter that authored
    // no running attack could not answer either press.
    assert!(
        !app.world()
            .get::<ambition_platformer2d::engine_core::BodyAbilities>(body)
            .expect("a seated fighter carries its ability set")
            .abilities
            .dash,
        "this fighter still owns the traversal burst, so reaching its dash \
         attack below would say nothing about a stage that removed it"
    );
    let dash_attack_id = app
        .world()
        .get::<ActorMoveset>(body)
        .and_then(|moveset| {
            moveset
                .0
                .move_for_verb(&ambition_platformer2d::entity_catalog::dash_stance_verb(
                    ambition_platformer2d::entity_catalog::ATTACK_VERB,
                ))
                .map(|spec| spec.id.clone())
        })
        .expect("this fighter authors no running attack, so no press could reach one");

    let running = |app: &App| {
        app.world()
            .get::<BodyMotionFacts>(body)
            .is_some_and(|facts| facts.running)
    };
    let playing = |app: &App| {
        app.world()
            .get::<MovePlayback>(body)
            .map(|playback| playback.spec.id.clone())
    };

    // ── PHASE A: standing. The same press must NOT be the running attack. ──
    assert!(
        !running(&app),
        "the fighter is already running before anybody held a direction"
    );
    Buttonlike::press(&ATTACK_KEY, app.world_mut());
    let standing_press = {
        let mut moved: Option<String> = None;
        for _ in 0..12 {
            app.update();
            if let Some(id) = playing(&app) {
                assert!(
                    !running(&app),
                    "the body reached the run gait during the standing phase, so \
                     phase A is measuring the same state as phase B"
                );
                moved = Some(id);
                break;
            }
        }
        moved
    };
    Buttonlike::release(&ATTACK_KEY, app.world_mut());
    assert!(
        standing_press.is_some(),
        "a standing fighter pressed Attack and no move started at all, so the \
         comparison below has nothing to compare"
    );
    assert_ne!(
        standing_press.as_deref(),
        Some(dash_attack_id.as_str()),
        "a STANDING fighter's attack press produced the running attack, which \
         means the gait is not what selects it"
    );

    // Let the standing move finish; a body in recovery refuses the next press.
    for _ in 0..40 {
        app.update();
    }

    // ── PHASE B: the same press, out of a run. ──
    Buttonlike::press(&KeyCode::ArrowRight, app.world_mut());
    let reached = hold_until(&mut app, running);
    assert!(
        reached.is_some(),
        "a fighter held a direction for the whole patience budget and never \
         reached the run gait. Either `run_commit_frac` is unreachable on this \
         body or the kernel stopped publishing the fact"
    );

    let mut out_of_the_run: Option<String> = None;
    for _ in 0..24 {
        // re-pressed each round rather than held: the press is an EDGE, and a
        // held key that was already spent starts nothing.
        Buttonlike::press(&ATTACK_KEY, app.world_mut());
        app.update();
        Buttonlike::release(&ATTACK_KEY, app.world_mut());
        app.update();
        if let Some(id) = playing(&app) {
            if id == dash_attack_id {
                out_of_the_run = Some(id);
                break;
            }
        }
    }
    Buttonlike::release(&KeyCode::ArrowRight, app.world_mut());

    assert_eq!(
        out_of_the_run.as_deref(),
        Some(dash_attack_id.as_str()),
        "a fighter in the run gait pressed Attack twenty-four times and never \
         got `{dash_attack_id}`. The running attack is authored, the gait is \
         published, and the press reaches the body — so the selector is reading \
         something other than the gait"
    );
    eprintln!(
        "[gait] standing press = {standing_press:?}, out of the run = {out_of_the_run:?}, \
         gait reached after {reached:?} updates"
    );
}

// ─── — Shield is its OWN action, not a flavour of Special ───────
//
// participant control/action. Shield input -> can hold/release shield. Special
// input -> activates authored special behavior. One cannot accidentally
// masquerade as the other."*
//
// Every case below drives the SHIPPED keyboard preset (`arrows_zxc`) through the
// host's real participant chain: `E` is the shield key and `G` is the special
// key on that preset. Reading them off the preset rather than naming a device
// would test a table; pressing them is what tests the game.

/// The shield key on the default keyboard preset (`arrows_zxc`).
const SHIELD_KEY: KeyCode = KeyCode::KeyE;
/// The special key on the same preset — deliberately a DIFFERENT physical key,
/// which is what makes the two masquerade cases below separable at all.
const SPECIAL_KEY: KeyCode = KeyCode::KeyG;

/// Seat a person (seat 0, on the keyboard) as `fighter` against one CPU, start
/// the match, and hand back the person's body once the opening hold is off.
fn a_person_fighting_as(fighter: &str) -> (App, Entity) {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;

    let mut app = open_the_lobby();
    cycle_role(&mut app, 0, 1); // the person takes the only source (the keyboard)
    cycle_role(&mut app, 1, 1); // no source left, so: CPU
    pick_fighter(&mut app, 0, fighter);
    pick_fighter(&mut app, 1, PREPARED_FIGHTER);
    assert_eq!(
        start_and_report(&mut app),
        MatchStart::Activated { seats: 2 },
        "the lobby refused to start a person-versus-CPU match"
    );
    wait_for_the_round_to_go_live(&mut app);

    let body = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        let mut rows: Vec<(usize, Entity)> = q.iter(world).map(|(e, s)| (s.0, e)).collect();
        rows.sort_by_key(|(seat, _)| *seat);
        assert!(!rows.is_empty(), "a started match seated no fighter at all");
        rows[0].1
    };
    (app, body)
}

fn guard_is_up(app: &App, body: Entity) -> bool {
    app.world()
        .get::<ambition_platformer2d::engine_core::body_clusters::BodyShieldState>(body)
        .map(|shield| shield.active)
        .unwrap_or(false)
}

/// Run until `property` holds or the give-up ceiling expires; report whether it
/// ever held and how many updates it took.
///
/// The ceiling is patience, never the measurement.
fn hold_until(app: &mut App, mut property: impl FnMut(&App) -> bool) -> Option<usize> {
    for updates in 0..180 {
        if property(app) {
            return Some(updates);
        }
        app.update();
    }
    None
}

/// THE PROBE. A SMASH FIGHTER'S SHIELD BUTTON RAISES THEIR GUARD.
///
/// Every one of the fourteen smash fighters is granted `AbilitySet::shield` by `SMASH_FIGHTER_KIT`,
/// and thirteen of them throw an ordinary special, so thirteen fighters had their guard erased
/// every frame by a gate that had never heard of them.
///
/// George Booul is the fighter this demo owns and his special is his own — which
/// is exactly the population the old gate refused, and the non-vacuity assertions
/// below say so out loud rather than trusting the roster to stay that way.
#[test]
fn a_smash_fighters_shield_input_raises_and_lowers_their_guard() {
    let (mut app, body) = a_person_fighting_as(OTHER_PREPARED_FIGHTER);

    eprintln!(
        "[d146-shield] seat 0 PlayerEntity = {}",
        app.world()
            .get::<ambition_platformer2d::platformer::markers::PlayerEntity>(body)
            .is_some()
    );

    // NON-VACUITY, both terms. A fighter without the shield ability could
    // never raise a guard, and a fighter whose special IS a shield move would
    // pass through the very exception this test exists to delete.
    let abilities = app
        .world()
        .get::<ambition_platformer2d::engine_core::BodyAbilities>(body)
        .expect("a seated fighter carries its ability set")
        .abilities;
    assert!(
        abilities.shield,
        "this fighter has no shield ability, so nothing below could raise a guard"
    );
    let special = app
        .world()
        .get::<ambition_platformer2d::characters::brain::ActionSet>(body)
        .expect("a seated fighter carries its action set")
        .special
        .clone();
    eprintln!("[d146-shield] seat 0 special = {special:?}");
    assert!(
        !matches!(
            special.as_ref(),
            Some(ambition_platformer2d::characters::brain::SpecialActionSpec::Special(key))
                if key == "bubble_shield"
        ),
        "this fighter's special IS the bubble shield, so it would ride the old \
         gate's exception and prove nothing about the other thirteen"
    );

    assert!(
        !guard_is_up(&app, body),
        "the guard is up before anybody touched the shield button"
    );

    // HOLD.
    Buttonlike::press(&SHIELD_KEY, app.world_mut());
    let raised = hold_until(&mut app, |app| guard_is_up(app, body));
    assert!(
        raised.is_some(),
        "a smash fighter held the SHIELD button and no guard came up. Shield is \
         its own participant action — it is not a variant of Special, and it must \
         not depend on which special this body happens to carry"
    );

    // RELEASE. A guard that cannot come down is a guard nobody can play around.
    Buttonlike::release(&SHIELD_KEY, app.world_mut());
    let lowered = hold_until(&mut app, |app| !guard_is_up(app, body));
    assert!(
        lowered.is_some(),
        "the guard never came down after the shield button was released"
    );
    eprintln!(
        "[d146-shield] guard up after {:?} updates, down after {:?}",
        raised, lowered
    );
}

/// SPECIAL CANNOT MASQUERADE AS SHIELD. (, half one)
///
/// Pressing Special must not raise a guard on a body whose special is not a
/// shield move. Without this the fix could have been "keep the guard alive for
/// everybody", which is not a separation of the two actions — it is the same
/// conflation pointing the other way.
///
/// NON-VACUOUS: the press has to reach the body. A test where Special did
/// nothing at all would pass this perfectly, so it asserts the special MOVE
/// started as well as that no guard came up.
#[test]
fn pressing_special_does_not_raise_a_guard_on_a_fighter_whose_special_is_not_one() {
    use ambition_platformer2d::combat::moveset::MovePlayback;

    let (mut app, body) = a_person_fighting_as(OTHER_PREPARED_FIGHTER);
    assert!(
        !guard_is_up(&app, body),
        "the guard is up before anybody pressed anything"
    );

    Buttonlike::press(&SPECIAL_KEY, app.world_mut());
    let mut fired: Option<String> = None;
    let mut raised = false;
    for _ in 0..180 {
        app.update();
        raised |= guard_is_up(&app, body);
        if fired.is_none() {
            fired = app
                .world()
                .get::<MovePlayback>(body)
                .map(|playback| playback.spec.id.clone());
        }
    }
    Buttonlike::release(&SPECIAL_KEY, app.world_mut());

    assert!(
        fired.is_some(),
        "pressing SPECIAL started no move at all, so this fighter never received \
         the press and the guard claim below is vacuous"
    );
    assert!(
        !raised,
        "pressing SPECIAL raised this fighter's guard (it played {fired:?}). \
         Special activates the authored special behaviour; a guard is the Shield \
         action's job, and a special that raises one by accident is exactly the \
         masquerade Jon named"
    );
}

/// SHIELD CANNOT MASQUERADE AS SPECIAL. (, half two)
///
/// Holding the shield button raises a guard and starts NO authored move.
#[test]
fn holding_shield_raises_a_guard_and_fires_no_authored_move() {
    use ambition_platformer2d::combat::moveset::MovePlayback;

    let (mut app, body) = a_person_fighting_as(OTHER_PREPARED_FIGHTER);

    Buttonlike::press(&SHIELD_KEY, app.world_mut());
    let mut raised = false;
    let mut played: Option<String> = None;
    for _ in 0..180 {
        app.update();
        raised |= guard_is_up(&app, body);
        if played.is_none() {
            played = app
                .world()
                .get::<MovePlayback>(body)
                .map(|playback| playback.spec.id.clone());
        }
    }
    Buttonlike::release(&SHIELD_KEY, app.world_mut());

    assert!(
        raised,
        "holding SHIELD raised no guard, so the other half of this claim is vacuous"
    );
    assert_eq!(
        played, None,
        "holding SHIELD started an authored move. Shield holds a guard; it does \
         not activate authored behaviour, and a shield that fires a special is \
         the masquerade pointing the other way"
    );
}

/// A CPU FIGHTER RAISES A GUARD OF ITS OWN, IN A REAL MATCH.
///
/// pretending to press a physical controller trigger."* This is the link no unit
/// test reaches: a CPU seat carries no `PlayerEntity` and no persona gate, so
/// whether a brain-requested guard survives to `BodyShieldState` is only
/// answerable in an assembled match.
///
/// WHICH brain, said plainly, because the obvious reading is wrong. The
/// shipped smash CPU is `template: Fighter` (`SMASH_CATALOG_RON`'s
/// `autonomous_profiles`), so the guard this observes is the fighter brain's
/// `MovementVerb::Shield`, not the smash brain's reactive block. What the two
/// prove together is the whole claim: a brain — either brain — names a defensive
/// intent in its OWN vocabulary and the body raises a real guard, with no
/// physical button anywhere in the chain. The smash brain's half is
/// `brain::smash::tests::defense_blinks_a_lunge_and_blocks_a_walk_in`, which now runs
/// through `SpecificAction::Shield` rather than writing `shield_held` beside it.
///
/// the person has to ATTACK, not merely approach. A guard is offered to a
/// fighter that is losing an exchange AND has something incoming — pressing it
/// against nothing is how you get grabbed, and the day shielding was offered on
/// "cornered" alone the stage became two statues holding guard forever. So this
/// walks in and swings.
#[test]
fn a_cpu_fighter_raises_a_guard_without_pressing_a_physical_button() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::engine_core::BodyKinematics;

    let (mut app, person) = a_person_fighting_as(OTHER_PREPARED_FIGHTER);
    let cpu = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        let mut rows: Vec<(usize, Entity)> = q.iter(world).map(|(e, s)| (s.0, e)).collect();
        rows.sort_by_key(|(seat, _)| *seat);
        rows[1].1
    };
    assert_ne!(person, cpu, "the two seats are the same body");
    assert!(
        !guard_is_up(&app, cpu),
        "the CPU is already guarding before the match has been played"
    );

    // Walk TOWARD the opponent (whichever side it opened on) and swing.
    let x = |app: &App, body: Entity| {
        app.world()
            .get::<BodyKinematics>(body)
            .map(|kin| kin.pos.x)
            .unwrap_or(0.0)
    };
    let toward = if x(&app, cpu) > x(&app, person) {
        KeyCode::ArrowRight
    } else {
        KeyCode::ArrowLeft
    };
    Buttonlike::press(&toward, app.world_mut());

    let mut guarded = None;
    for updates in 0..1_800 {
        // Mash the attack key: a hostile mid-swing is what puts a guard on the
        // CPU's option list at all.
        if updates % 12 == 0 {
            Buttonlike::press(&KeyCode::KeyX, app.world_mut());
        } else if updates % 12 == 3 {
            Buttonlike::release(&KeyCode::KeyX, app.world_mut());
        }
        app.update();
        if guard_is_up(&app, cpu) {
            guarded = Some(updates);
            break;
        }
    }
    Buttonlike::release(&toward, app.world_mut());
    Buttonlike::release(&KeyCode::KeyX, app.world_mut());
    assert!(
        guarded.is_some(),
        "a CPU fighter never raised a guard across a whole pressured exchange. A \
         CPU body carries no `PlayerEntity` and no persona gate, so if a \
         brain-requested `shield_held` does not arrive at `BodyShieldState` the \
         defensive half of the CPU's vocabulary is decorative"
    );
    eprintln!("[d146-shield] the CPU guarded after {guarded:?} updates");
}

// for smash. My preferred smash layout for an xbox controller is a=normal,
// x=special, b=jump, y=grab (we don't have grab yet), left trigger is shield.
// The rest of the bindings are normal I think."*
//
// …and the sentence that made it an architecture task rather than a table edit:
// *"Well, B=jump is the way I like my smash controller, It's probably non
// standard. Will need to have control profiles eventually."*

/// Put a value on a pad button through the raw device seam, the way every other
/// gamepad probe in this repo does.
fn pad_hold(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
    app.world_mut()
        .write_message(bevy::input::gamepad::RawGamepadEvent::Button(
            bevy::input::gamepad::RawGamepadButtonChangedEvent::new(pad, button, value),
        ));
}

/// Put every hand on `rect`, then confirm with this physical pad. The all-hand
/// placement avoids asserting the local channel index in a helper; the raw pad
/// event is what decides which cursor actually clicks.
fn pad_click(
    app: &mut App,
    pad: Entity,
    rect: ambition_demo_smash::select_screen::cursor::HitRect,
) {
    {
        let mut cursors = app.world_mut().resource_mut::<SelectCursors>();
        for seat in 0..4 {
            cursors
                .seat_mut(seat)
                .expect("seat is bounded by the loop")
                .move_to(rect.center());
        }
    }
    pad_hold(app, pad, GamepadButton::South, 1.0);
    app.update();
    pad_hold(app, pad, GamepadButton::South, 0.0);
    app.update();
    settle(app);
}

/// Seat a keyboard person at slot 0 and a PAD player at slot 1 as `fighter`,
/// start the match, and hand back the pad, the pad player's body, and the app.
///
/// Slot 1 is the pad because the pad explicitly presses that card's role control.
fn a_pad_player_fighting_as(fighter: &str) -> (App, Entity, Entity) {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;

    let mut app = shell_host_app();
    // ONE pad: under the couch policy that is two seats, the keyboard's and this
    // one's.
    let pad = app
        .world_mut()
        .spawn(bevy::input::gamepad::Gamepad::default())
        .id();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    settle(&mut app);

    cycle_role(&mut app, 0, 1); // keyboard explicitly claims card one
    let second_card = screen(&app).role_button(1);
    pad_click(&mut app, pad, second_card); // pad explicitly claims card two
    assert_eq!(
        app.world().resource::<SmashSelect>().slot(1).occupant,
        SlotOccupant::Controller { device: 1 },
        "seat 1 is not on the pad, so every measurement below would be about the \
         keyboard seat wearing the pad's name"
    );
    pick_fighter(&mut app, 0, PREPARED_FIGHTER);
    pick_fighter(&mut app, 1, fighter);
    assert_eq!(
        start_and_report(&mut app),
        MatchStart::Activated { seats: 2 },
        "the lobby refused to start a keyboard-versus-pad match"
    );
    wait_for_the_round_to_go_live(&mut app);

    let body = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        let mut rows: Vec<(usize, Entity)> = q.iter(world).map(|(e, s)| (s.0, e)).collect();
        rows.sort_by_key(|(seat, _)| *seat);
        assert!(
            rows.len() >= 2,
            "a two-seat match seated {} bodies",
            rows.len()
        );
        rows[1].1
    };
    (app, pad, body)
}

/// Every move id this body reaches through a `special*` verb, read off its OWN
/// authored moveset.
///
/// derived, never a hand-listed id. Which move George's neutral special
/// is, is his sheet's business; what this test is about is whether the PAD
/// reaches it. A literal `"bivalence"` here would go quiet the day he is
/// re-authored and would still pass.
fn authored_specials_of(app: &mut App, body: Entity) -> std::collections::BTreeSet<String> {
    use ambition_platformer2d::combat::moveset::ActorMoveset;
    app.world()
        .get::<ActorMoveset>(body)
        .map(|moveset| {
            moveset
                .0
                .verbs
                .iter()
                .filter(|(verb, _)| verb.starts_with("special"))
                .map(|(_, id)| id.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Hold `button` and report the first authored move that starts, if any.
///
/// `app.update()` is NOT one tick of sim time — the loop is a CEILING on
/// patience, and the answer is whatever the property said, never the count.
fn move_started_while_holding(
    app: &mut App,
    pad: Entity,
    body: Entity,
    button: GamepadButton,
) -> Option<String> {
    use ambition_platformer2d::combat::moveset::MovePlayback;
    pad_hold(app, pad, button, 1.0);
    let mut fired = None;
    for _ in 0..180 {
        app.update();
        if fired.is_none() {
            fired = app
                .world()
                .get::<MovePlayback>(body)
                .map(|playback| playback.spec.id.clone());
        }
    }
    pad_hold(app, pad, button, 0.0);
    for _ in 0..30 {
        app.update();
    }
    fired
}

/// X IS SPECIAL ON A PAD, AND IT REACHES THE FIGHTER'S AUTHORED SPECIAL.
///
/// The default pad is fully assigned, so `presets.rs` deliberately declined to double-bind one and
/// left it to a remap pass that never came. A pad player could not throw a special at all.
///
/// A game's `BindingLayout` is what makes it possible without a remap, because a
/// layout PERMUTES an assigned pad rather than adding to it: X was Attack, Attack
/// moved to A, and X is free for Special.
///
/// non-vacuity is the `authored_specials_of` set, not a literal id. If the
/// press reached nothing at all `fired` is `None`; if it reached the wrong verb
/// the id is outside the set. Both fail with the id printed.
#[test]
fn on_the_smash_pad_x_fires_the_fighters_authored_special() {
    let (mut app, pad, body) = a_pad_player_fighting_as(OTHER_PREPARED_FIGHTER);

    let specials = authored_specials_of(&mut app, body);
    assert!(
        !specials.is_empty(),
        "this fighter authors no special at all, so pressing X could not prove \
         anything about reaching one"
    );

    let fired = move_started_while_holding(&mut app, pad, body, GamepadButton::West);
    let fired = fired.expect(
        "pressing X on the pad started NO move. Either the smash layout never \
         reached this seat's InputMap, or Special still has no gamepad button",
    );
    assert!(
        specials.contains(&fired),
        "pressing X played `{fired}`, which is not one of this fighter's authored \
         specials {specials:?} — X is bound to the wrong verb under the smash layout"
    );
}

/// Y ON THE PAD STARTS THE FIGHTER'S AUTHORED GRAB.
///
/// a passing capture chain is not evidence about this, which is the
/// general form worth keeping: a hand-driven chain pins the FUNCTION and says
/// nothing about the WIRING.
///
/// non-vacuity is asserted twice, because either half could make this pass
/// for the wrong reason. The fighter has to author a grab at all (otherwise
/// pressing Y correctly does nothing), and the pad has to bind Y to Grab
/// (otherwise this measures some other button's verb).
#[test]
fn on_the_smash_pad_y_starts_the_fighters_authored_grab() {
    use ambition_platformer2d::combat::moveset::ActorMoveset;

    let (mut app, pad, body) = a_pad_player_fighting_as(OTHER_PREPARED_FIGHTER);

    // Non-vacuity 1: this fighter answers the grab verb.
    let grab_id = app
        .world()
        .get::<ActorMoveset>(body)
        .and_then(|moveset| {
            moveset
                .0
                .move_for_verb(ambition_platformer2d::entity_catalog::GRAB_VERB)
                .map(|spec| spec.id.clone())
        })
        .expect(
            "this fighter authors no grab, so pressing Y could not prove anything              about reaching one",
        );

    // Non-vacuity 2: Y is where the smash layout puts Grab.
    {
        use ambition_platformer2d::input::{
            ActionBindings, PhysicalControl, Platformer2dInputActionMonolith,
        };
        use leafwing_input_manager::prelude::InputMap;
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &InputMap<Platformer2dInputActionMonolith>,
            With<ambition_platformer2d::input::InputParticipant>,
        >();
        let bound: Vec<PhysicalControl> = q
            .iter(world)
            .next()
            .map(|map| {
                ActionBindings::from_map(map)
                    .controls(&Platformer2dInputActionMonolith::Grab)
                    .iter()
                    .filter(|control| matches!(control, PhysicalControl::Button(_)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(
            bound,
            vec![PhysicalControl::Button(GamepadButton::North)],
            "inside a smash match Grab has to be on Y, or the press below is              about some other button"
        );
    }

    let fired = move_started_while_holding(&mut app, pad, body, GamepadButton::North);
    let fired = fired.expect(
        "pressing Y on the pad started NO move. The fighter authors a grab and          the pad binds Y to Grab, so the break is between them — the seat's          control frame, or the action scheme's Grab slot, which STRIPS          `grab_pressed` when the slot is absent",
    );
    assert_eq!(
        fired, grab_id,
        "pressing Y played `{fired}` rather than this fighter's grab `{grab_id}`"
    );
}

/// A HELD PERSON CAN MASH THEIR WAY OUT, WITH A REAL BUTTON.
///
/// Reading the schedule says that is right; reading is what missed the grab.
///
/// the hold is MANUFACTURED and the press is not, which is the split that
/// makes this a measurement. Who grabbed whom is setup — the CPU's grab
/// TIMING is a separate open question and waiting for one would
/// make this a test of the brain. What is under test is only whether a pad
/// press survives to `mash_credit`.
#[test]
fn on_the_smash_pad_a_held_player_can_mash_free() {
    use ambition_platformer2d::combat::capture::CapturedBy;

    let (mut app, pad, body) = a_pad_player_fighting_as(OTHER_PREPARED_FIGHTER);

    // Somebody else on the stage is the captor; which body it is does not
    // matter to the question, only that it is not the captive.
    let captor = {
        use ambition_platformer2d::actors::character_runtime::MatchSeat;
        let world = app.world_mut();
        let mut seats = world.query::<(bevy::prelude::Entity, &MatchSeat)>();
        seats
            .iter(world)
            .map(|(entity, _)| entity)
            .find(|entity| *entity != body)
            .expect("a two-seat match has a second body")
    };

    // Re-establish the hold each round because ordinary combat can interrupt it;
    // the test must observe mash release rather than an unrelated interruption.
    let hold = |app: &mut App| {
        // Inserting only the first would leave a body held by somebody with no clock and nothing to
        // mash out of.
        app.world_mut().entity_mut(body).insert((
            CapturedBy {
                captor,
                hold_offset_local: ambition_platformer2d::engine_core::Vec2::new(20.0, -2.0),
                prior_gravity_scale: 1.0,
            },
            ambition_platformer2d::characters::smash_capture::SmashHoldState::lasting(
                ambition_platformer2d::combat::rules::ResolvedCombatTuning::default()
                    .grab_hold_seconds(0),
            ),
        ));
    };

    let mut best = 0.0_f32;
    for _ in 0..24 {
        // Re-establish from a KNOWN zero, so any rise below is this round's press.
        hold(&mut app);
        pad_hold(&mut app, pad, GamepadButton::South, 1.0);
        app.update();
        pad_hold(&mut app, pad, GamepadButton::South, 0.0);
        app.update();
        if let Some(held) = app.world().get::<SmashHoldState>(body) {
            best = best.max(held.mash_credit);
            if best > 0.0 {
                break;
            }
        }
    }

    assert!(
        best > 0.0,
        "twenty-four presses of A, each against a freshly zeroed hold, moved \
         `mash_credit` not once. A held person's frame is blanked by capture, \
         so if the escape sampler stops running while the frame is still theirs, \
         mashing does nothing and a hold is unbreakable by a human"
    );
}

/// A HOLDING PLAYER'S ATTACK PRESS IS A PUMMEL, NOT A JAB.
///
/// the next thing a person does after the grab they could not throw.
/// `988807b99` made the grab reachable; this asks whether the rest of the
/// capture context is. It is a DIFFERENT road from `grab_pressed`: a pummel and
/// the four throws are selected inside `trigger_moveset_moves` from
/// `gesture.pressed` — the resolved ATTACK gesture — and only when
/// `captive_of(entity, ..)` says this body is holding somebody. So a press that
/// works perfectly in free state proves nothing here, and vice versa.
///
/// the hold is manufactured for the same reason as
/// [`on_the_smash_pad_a_held_player_can_mash_free`] — who grabbed whom is setup,
/// and the CPU's grab TIMING is a separate open question.
#[test]
fn on_the_smash_pad_attacking_while_holding_pummels() {
    use ambition_platformer2d::combat::capture::CapturedBy;
    use ambition_platformer2d::combat::moveset::{ActorMoveset, MovePlayback};
    use ambition_platformer2d::entity_catalog::CAPTURE_PUMMEL_VERB;

    let (mut app, pad, body) = a_pad_player_fighting_as(OTHER_PREPARED_FIGHTER);

    // Non-vacuity: this fighter answers the pummel verb at all.
    let pummel_id = app
        .world()
        .get::<ActorMoveset>(body)
        .and_then(|moveset| {
            moveset
                .0
                .move_for_verb(CAPTURE_PUMMEL_VERB)
                .map(|spec| spec.id.clone())
        })
        .expect("this fighter authors no pummel, so pressing A while holding could not reach one");

    // Somebody to hold. Which body does not matter; that it is not the captor does.
    let captive = {
        use ambition_platformer2d::actors::character_runtime::MatchSeat;
        let world = app.world_mut();
        let mut seats = world.query::<(bevy::prelude::Entity, &MatchSeat)>();
        seats
            .iter(world)
            .map(|(entity, _)| entity)
            .find(|entity| *entity != body)
            .expect("a two-seat match has a second body")
    };

    // re-established each round: a live match keeps breaking holds, and a
    // press that lands on a body which stopped holding would be an ordinary jab.
    let mut started: Option<String> = None;
    for _ in 0..24 {
        app.world_mut().entity_mut(captive).insert((
            CapturedBy {
                captor: body,
                hold_offset_local: ambition_platformer2d::engine_core::Vec2::new(20.0, -2.0),
                prior_gravity_scale: 1.0,
            },
            ambition_platformer2d::characters::smash_capture::SmashHoldState::lasting(
                ambition_platformer2d::combat::rules::ResolvedCombatTuning::default()
                    .grab_hold_seconds(0),
            ),
        ));
        pad_hold(&mut app, pad, GamepadButton::South, 1.0);
        app.update();
        pad_hold(&mut app, pad, GamepadButton::South, 0.0);
        app.update();
        if let Some(playback) = app.world().get::<MovePlayback>(body) {
            let id = playback.spec.id.clone();
            if id == pummel_id {
                started = Some(id);
                break;
            }
        }
    }

    assert_eq!(
        started.as_deref(),
        Some(pummel_id.as_str()),
        "twenty-four presses of A while holding a body never started the pummel \
         `{pummel_id}`. Either the capture context is not reached from a real \
         press, or the press fell through to the ordinary attack menu — which is \
         a captor throwing a jab with somebody in its hands"
    );
}

/// FORWARD + ATTACK WHILE HOLDING IS THE FORWARD THROW, NOT THE BACK ONE.
#[test]
fn on_the_smash_pad_forward_and_attack_while_holding_throws() {
    use ambition_platformer2d::combat::capture::CapturedBy;
    use ambition_platformer2d::combat::moveset::{ActorMoveset, MovePlayback};
    use ambition_platformer2d::engine_core::BodyKinematics;
    use ambition_platformer2d::entity_catalog::CAPTURE_THROW_FORWARD_VERB;

    let (mut app, pad, body) = a_pad_player_fighting_as(OTHER_PREPARED_FIGHTER);

    let throw_id = app
        .world()
        .get::<ActorMoveset>(body)
        .and_then(|moveset| {
            moveset
                .0
                .move_for_verb(CAPTURE_THROW_FORWARD_VERB)
                .map(|spec| spec.id.clone())
        })
        .expect("this fighter authors no forward throw, so this could not reach one");

    let captive = {
        use ambition_platformer2d::actors::character_runtime::MatchSeat;
        let world = app.world_mut();
        let mut seats = world.query::<(bevy::prelude::Entity, &MatchSeat)>();
        seats
            .iter(world)
            .map(|(entity, _)| entity)
            .find(|entity| *entity != body)
            .expect("a two-seat match has a second body")
    };

    let mut started: Option<String> = None;
    for _ in 0..24 {
        app.world_mut().entity_mut(captive).insert((
            CapturedBy {
                captor: body,
                hold_offset_local: ambition_platformer2d::engine_core::Vec2::new(20.0, -2.0),
                prior_gravity_scale: 1.0,
            },
            ambition_platformer2d::characters::smash_capture::SmashHoldState::lasting(
                ambition_platformer2d::combat::rules::ResolvedCombatTuning::default()
                    .grab_hold_seconds(0),
            ),
        ));
        // FORWARD is the captor's own facing, not a screen direction.
        let facing = app
            .world()
            .get::<BodyKinematics>(body)
            .map(|kin| kin.facing)
            .unwrap_or(1.0);
        let toward = if facing >= 0.0 {
            GamepadButton::DPadRight
        } else {
            GamepadButton::DPadLeft
        };
        pad_hold(&mut app, pad, toward, 1.0);
        app.update();
        pad_hold(&mut app, pad, GamepadButton::South, 1.0);
        app.update();
        pad_hold(&mut app, pad, GamepadButton::South, 0.0);
        pad_hold(&mut app, pad, toward, 0.0);
        app.update();
        if let Some(playback) = app.world().get::<MovePlayback>(body) {
            let id = playback.spec.id.clone();
            if id == throw_id {
                started = Some(id);
                break;
            }
        }
    }

    assert_eq!(
        started.as_deref(),
        Some(throw_id.as_str()),
        "twenty-four forward+attack presses while holding never started the \
         forward throw `{throw_id}`. A throw is how a hold ENDS on the captor's \
         terms, so without it a person can grab and pummel and then only wait \
         out the clock"
    );
}

/// THE LEFT TRIGGER SHIELDS, THROUGH THE SEMANTIC SHIELD ACTION.
///
/// action with its own `BodyShieldState`; this is the pad half of it.
///
/// BOTH left shoulder buttons, on purpose and asserted separately. "Left
/// trigger" on an Xbox pad names the ANALOG trigger, which Bevy spells
/// `LeftTrigger2` because it spells the BUMPER `LeftTrigger` — so the layout
/// gives Shield both rather than guessing, which is also what a fighting game
/// does. One action on two buttons is fine; two actions on one button is the
/// hazard, and `every_button_the_smash_layout_claims_drives_exactly_one_verb`
/// is where that is pinned.
///
/// and it must not be Special doing this. A guard that came up because the
/// trigger fired a shield-flavoured special would be exactly the masquerade
/// slice 2 deleted, so the authored-move channel is asserted SILENT.
#[test]
fn on_the_smash_pad_the_left_trigger_raises_a_real_guard() {
    use ambition_platformer2d::combat::moveset::MovePlayback;

    let (mut app, pad, body) = a_pad_player_fighting_as(OTHER_PREPARED_FIGHTER);
    assert!(
        !guard_is_up(&app, body),
        "the guard is up before anybody touched a trigger"
    );

    for button in [GamepadButton::LeftTrigger, GamepadButton::LeftTrigger2] {
        pad_hold(&mut app, pad, button, 1.0);
        let mut played: Option<String> = None;
        let raised = hold_until(&mut app, |app| {
            app.world()
                .get::<ambition_platformer2d::engine_core::body_clusters::BodyShieldState>(body)
                .map(|shield| shield.active)
                .unwrap_or(false)
        });
        // Whatever the guard did, ask the move channel the same question.
        for _ in 0..30 {
            app.update();
            if played.is_none() {
                played = app
                    .world()
                    .get::<MovePlayback>(body)
                    .map(|playback| playback.spec.id.clone());
            }
        }
        pad_hold(&mut app, pad, button, 0.0);

        assert!(
            raised.is_some(),
            "holding {button:?} on the smash pad raised no guard — the layout's \
             Shield binding did not reach this seat"
        );
        assert_eq!(
            played, None,
            "holding {button:?} started `{played:?}`. Shield holds a guard; it \
             does not activate authored behaviour, and a trigger that fires a \
             special is the masquerade Jon named"
        );

        let lowered = hold_until(&mut app, |app| !guard_is_up(app, body));
        assert!(
            lowered.is_some(),
            "the guard never came down after {button:?} was released, so the \
             next case would be measuring this one's leftovers"
        );
    }
}

#[test]
fn on_the_smash_pad_b_jumps_and_a_attacks() {
    use ambition_platformer2d::engine_core::BodyKinematics;

    let (mut app, pad, body) = a_pad_player_fighting_as(OTHER_PREPARED_FIGHTER);
    let specials = authored_specials_of(&mut app, body);

    // How far off its resting height this body gets while `button` is held.
    fn excursion_while_holding(
        app: &mut App,
        pad: Entity,
        body: Entity,
        button: GamepadButton,
        frames: usize,
    ) -> f32 {
        let rest = app.world().get::<BodyKinematics>(body).unwrap().pos.y;
        pad_hold(app, pad, button, 1.0);
        let mut furthest = 0.0f32;
        for _ in 0..frames {
            app.update();
            let now = app.world().get::<BodyKinematics>(body).unwrap().pos.y;
            furthest = furthest.max((now - rest).abs());
        }
        pad_hold(app, pad, button, 0.0);
        // Land, so the next case is not measuring this one's arc.
        for _ in 0..120 {
            app.update();
        }
        furthest
    }

    // B — the button that is Blink in Ambition and Jump here.
    let jumped = excursion_while_holding(&mut app, pad, body, GamepadButton::East, 60);
    assert!(
        jumped > 24.0,
        "B never left the ground ({jumped:.2}px). Under the smash layout B is \
         Jump; under the base preset it is Blink, so this is what a layout that \
         did not reach the seat looks like"
    );

    // A — the button that is Jump in Ambition and the normal attack here.
    //
    // both halves, because either alone passes a build where A does both.
    // The move channel says A reaches an attack; the height says it is not ALSO
    // still jumping. `y` is sampled over a short window on purpose — 30 frames
    // into a jump this body is already ~127px up, and a short window is also the
    // one least likely to catch a launch from the CPU across the stage.
    let hopped = excursion_while_holding(&mut app, pad, body, GamepadButton::South, 30);
    let fired = move_started_while_holding(&mut app, pad, body, GamepadButton::South).expect(
        "pressing A on the pad started no move at all — A is Attack under the \
         smash layout, and an unbound A is what the permutation failing looks like",
    );
    assert!(
        !specials.contains(&fired),
        "pressing A played `{fired}`, one of this fighter's SPECIALS {specials:?}. \
         A is the normal attack; the special lives on X"
    );
    assert!(
        hopped < 24.0,
        "pressing A moved this fighter {hopped:.2}px vertically against the {jumped:.2}px \
         B moved it — A is still Jump, so the layout swapped nothing"
    );
}

/// THE RULING: THE PROFILE LEAVES WITH THE GAME.
///
/// standard."* Non-standard is precisely why it may not become the default —
/// A=Jump stays right for Ambition, and the ONE regression that would matter
/// most is a smash match teaching the whole host that B jumps.
///
/// This is the behavioural half of that claim; the pure half
/// (`installing_the_smash_layout_does_not_move_the_generic_preset`, in
/// `ambition_input::layout`) says the shared preset function is untouched. Here
/// the same host that just played a smash match is asked what its seat's pad
/// says afterwards, through the live `InputMap` the router actually reads.
#[test]
fn quitting_a_smash_match_gives_the_pad_back() {
    use ambition_platformer2d::input::{
        ActionBindings, PhysicalControl, Platformer2dInputActionMonolith,
    };
    use leafwing_input_manager::prelude::InputMap;

    fn pad_verbs(app: &mut App, action: Platformer2dInputActionMonolith) -> Vec<PhysicalControl> {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &InputMap<Platformer2dInputActionMonolith>,
            With<ambition_platformer2d::input::InputParticipant>,
        >();
        // Any seat: the layout is a fact about the GAME, so every participant
        // in the composition carries the same answer.
        q.iter(world)
            .next()
            .map(|map| {
                ActionBindings::from_map(map)
                    .controls(&action)
                    .iter()
                    .filter(|control| matches!(control, PhysicalControl::Button(_)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    let (mut app, _pad, _body) = a_pad_player_fighting_as(OTHER_PREPARED_FIGHTER);

    assert_eq!(
        pad_verbs(&mut app, Platformer2dInputActionMonolith::Jump),
        vec![PhysicalControl::Button(GamepadButton::East)],
        "inside a smash match Jump has to be on B, or nothing else here is a \
         measurement of anything"
    );
    assert_eq!(
        pad_verbs(&mut app, Platformer2dInputActionMonolith::Special),
        vec![PhysicalControl::Button(GamepadButton::West)],
        "inside a smash match Special has to be on X"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(
        active_route(&app).as_deref(),
        Some(shell_host::AMBITION_LAUNCHER_ROUTE),
        "the match never ended, so the release below was never asked for"
    );
    // Give the recipe→map rebuild its frame.
    for _ in 0..10 {
        app.update();
    }

    assert_eq!(
        pad_verbs(&mut app, Platformer2dInputActionMonolith::Jump),
        vec![PhysicalControl::Button(GamepadButton::South)],
        "⛔ ONE SMASH MATCH REDEFINED THE HOST'S PAD. A=Jump is Ambition's \
         default and a mode's layout is a LOAN: the declaration goes back when \
         its experience leaves, exactly like the stage's combat rules"
    );
    assert!(
        pad_verbs(&mut app, Platformer2dInputActionMonolith::Special).is_empty(),
        "Special kept a gamepad button after the smash experience left. The \
         default pad is fully assigned and declines to double-bind one; only a \
         layout may claim one, and only for as long as it is declared"
    );
}

/// the fault was ONE inverted sign, and it was in the shared floor. An
/// authored `launch_dir` is a vector in the victim's own acceleration frame,
/// where `y` points TOWARD THE FEET — that is the authoring contract's own words
/// (`HitVolume::launch_dir`: *"(+x = facing, +y = gravity-down)"*), it is what
/// all ~100 authored literals in the tree wrote, and `player_robot_moveset`
/// already had a running test asserting the d-air's `y > 0` means DOWN.
/// `hit_response::knockback_velocity` negated `y` anyway, to satisfy a doc
/// comment on its own struct that claimed the opposite. So every up-tilt,
/// up-air and up-smash in the game spiked its victim into the floor, and every
/// down-air lifted them.
///
/// measured, not read. In this composition at 1427%, George's authored
/// up-tilt resolves to `LaunchSpeed(3269.4)` — exactly `130 + 2.20 × 1427 / 1.0`
/// — reaches the body as `pending_launch`, and the victim's own DI rotates it by
/// the full `SMASH_DI_MAX_ANGLE` of 0.31 rad while preserving the speed. Every
/// stage of the chain was healthy. The launch pointed down.
mod launched {
    use super::*;
    use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
    use ambition_platformer2d::engine_core::BodyGroundState;
    use ambition_platformer2d::engine_core::Vec2 as EVec2;
    use ambition_platformer2d::platformer::body::BodyKinematics;

    const UP_TILT_DAMAGE: i32 = 11;
    const UP_TILT_KNOCKBACK: f32 = 130.0;
    const UP_TILT_GROWTH: f32 = 2.20;
    const UP_TILT_LAUNCH_DIR: EVec2 = EVec2::new(0.1, -1.0);

    /// Two spots on the smash stage's floor, far enough apart that the
    /// attacker's own CPU cannot reach the victim inside the reading.
    const ATTACKER_X: f32 = 200.0;
    const VICTIM_X: f32 = 520.0;

    /// What one strike did to the victim, in the terms the report is written in.
    struct Launch {
        left_the_ground: bool,
        peak_rise: f32,
    }

    fn two_seated_fighters(app: &mut App) -> (Entity, Entity) {
        let roster = app
            .world()
            .resource::<ambition_demo_smash::select::SmashRoster>()
            .0
            .clone();
        let portrait_of = |id: &str| {
            roster
                .iter()
                .position(|e| e == id)
                .unwrap_or_else(|| panic!("{id} is not on the assembled grid: {roster:?}"))
        };
        let layout = screen(app);
        // Twice each: the first press takes a source, the second cycles to CPU.
        click(app, layout.role_button(0));
        click(app, layout.role_button(0));
        click(app, layout.role_button(1));
        click(app, layout.role_button(1));
        for (slot, character) in [(0usize, "smash_george_booul"), (1, "npc_alice")] {
            let token = placed_token(app, slot);
            click(app, token);
            click(
                app,
                layout
                    .portrait(portrait_of(character))
                    .expect("an authored portrait"),
            );
        }
        click(app, layout.start_button());
        // Past the 3-2-1 opening hold.
        for _ in 0..240 {
            app.update();
        }
        let bodies: Vec<(String, Entity)> = {
            let world = app.world_mut();
            let mut q = world.query::<(Entity, &WornCharacter)>();
            q.iter(world)
                .map(|(e, w)| (w.id().to_string(), e))
                .collect()
        };
        let find = |needle: &str| {
            bodies
                .iter()
                .find(|(id, _)| id.contains(needle))
                .unwrap_or_else(|| panic!("{needle} never took the stage: {bodies:?}"))
                .1
        };
        (find("george"), find("alice"))
    }

    /// Strike a PARKED, GROUNDED fighter with one authored volume and watch.
    ///
    /// The two bodies are parked far apart first, so the only strike inside the
    /// measurement window is this one — a live CPU match otherwise throws its own
    /// hits into the middle of the reading. The volume is `World`-anchored for
    /// the same reason: it is exactly one contact, at a place this test chose.
    fn strike_a_grounded_fighter(
        app: &mut App,
        attacker: Entity,
        victim: Entity,
        percent: i32,
        launch_dir: Option<EVec2>,
    ) -> Launch {
        let park = |app: &mut App, e: Entity, x: f32| {
            let mut kin = app.world_mut().get_mut::<BodyKinematics>(e).unwrap();
            kin.pos = EVec2::new(x, 200.0);
            kin.vel = EVec2::ZERO;
        };
        // `app.update()` is not one tick of sim time, so this LOOPS on the
        // property (the victim is standing) rather than counting frames. The
        // attacker is re-parked every pass so he cannot walk back into the
        // reading while the victim is settling.
        park(app, victim, VICTIM_X);
        // ⭐⭐ SETTLED MEANS STANDING **AND THAWED**, and the second half was
        // missing.
        //
        // ⛔⛔ MEASURED 2026-08-25. This helper is called twice on ONE victim —
        // once at 0% and once at 1427% — and the second strike was landing while
        // the body was still FROZEN by the first: `hitstop_timer` read 0.088 at
        // the moment of contact. The launch that came out was 495.8 px/s where
        // the knockback formula says 3269, so the caller read the pair as "the
        // percent meter is not reaching the launch" while the meter was fine and
        // the FIXTURE was overlapping its own two readings.
        //
        // ⭐ THE PROOF IS THE ORDER. Struck FIRST, the 1427% up-tilt launches at
        // 3096.9 px/s and rises 398px — what the formula predicts. Struck
        // SECOND, 495.8 and 38.1. Same damage input, same victim weight, same
        // empty stale queue, same attacker: the only difference was the freeze.
        //
        // ⇒ waiting for the thaw is what makes the two readings comparable. It
        // is not a relaxation — with it the ratio is ~100x rather than sitting a
        // hair either side of the 10x bar, which is why this test could flip on
        // a change that moved hitlag by a frame.
        let mut grounded = false;
        for _ in 0..200 {
            park(app, attacker, ATTACKER_X);
            app.update();
            let standing = app
                .world()
                .get::<BodyGroundState>(victim)
                .is_some_and(|g| g.on_ground);
            let thawed = app
                .world()
                .get::<ambition_platformer2d::characters::actor::BodyCombat>(victim)
                .is_some_and(|c| !c.is_in_hitlag() && c.hitstun_timer <= 0.0);
            if standing && thawed {
                grounded = true;
                break;
            }
        }
        assert!(
            grounded,
            "the victim never came to rest on the stage, so 'it left the ground' \
             cannot mean anything"
        );
        {
            let mut health = app.world_mut().get_mut::<BodyHealth>(victim).unwrap();
            let pool = health.health;
            let policy = health.policy();
            *health = BodyHealth::restored(pool, percent, policy);
        }
        // ⛔⛔ THE VICTIM IS PARKED AGAIN HERE, IMMEDIATELY BEFORE THE STRIKE,
        // and this is the difference between measuring a launch and measuring a
        // walk. The settle loop above waits for the victim to be STANDING, and
        // a standing CPU fighter is not a still one — it walks. The strike is
        // anchored in WORLD space at `start`, so a victim that is already
        // moving is being handed a box centred where it no longer will be.
        //
        // Measured at strike time before this line existed: +162px/s at 0% and
        // -312px/s at 1427%, from positions 110px apart. The fast one walked
        // clear of a 48px half-extent box in about a seventh of a second and
        // took NO damage at all - its meter came out of the sampling loop
        // exactly where it went in. That reads as "the damage road refused the
        // hit" and is really "the hit was thrown where the victim was not".
        //
        // ⚠ It only started failing when the hitlag law changed because the
        // freeze duration moves how much sim time the FIRST strike consumes,
        // which lands the second one on a different phase of the victim's walk.
        // The old law was not correct here; it was lucky here.
        park(app, victim, VICTIM_X);
        let start = app.world().get::<BodyKinematics>(victim).unwrap().pos;
        let strike = app
            .world_mut()
            .spawn((
                ambition_platformer2d::combat::strike::Hitbox {
                    // An ordinary hit, not a gust.
                    owner: attacker,
                    source: ambition_platformer2d::combat::strike::HitSide::Enemy,
                    anchor: ambition_platformer2d::combat::strike::HitboxAnchor::World {
                        center: start,
                    },
                    half_extent: EVec2::new(48.0, 48.0),
                    shape: None,
                    facing: 1.0,
                    damage: UP_TILT_DAMAGE,
                    knockback:
                        ambition_platformer2d::combat::strike::HitboxKnockback::LaunchSpeed {
                            base: UP_TILT_KNOCKBACK,
                            growth: Some(UP_TILT_GROWTH),
                        },
                    launch_dir,
                    frame_down: EVec2::new(0.0, 1.0),
                    strike_sfx: None,
                    reaction: None,
                },
                ambition_platformer2d::combat::strike::HitboxHits::default(),
                ambition_platformer2d::combat::strike::HitboxLifetime { remaining_s: 0.1 },
            ))
            .id();

        let mut left_the_ground = false;
        let mut peak_rise = 0.0f32;
        let mut previous = start;
        // Eight ticks of reaction: long enough for a launch to express itself
        // and short enough that a hard one has not yet reached the blast zone.
        // Both percents sit inside their own hitstun for the whole window, so
        // the brain cannot steer the reading.
        //
        // EIGHT TICKS IN WHICH THE BODY CAN MOVE, not eight ticks of wall clock.
        let in_hitlag = |app: &App| {
            app.world()
                .get::<ambition_platformer2d::characters::actor::BodyCombat>(victim)
                .is_some_and(|c| c.is_in_hitlag())
        };
        let mut reacting_ticks = 0usize;
        for _ in 0..240 {
            if reacting_ticks >= 8 {
                break;
            }
            park(app, attacker, ATTACKER_X);
            app.update();
            if !in_hitlag(app) {
                reacting_ticks += 1;
            }
            let pos = app.world().get::<BodyKinematics>(victim).unwrap().pos;
            // A blast-zone respawn TELEPORTS; past that the displacement is
            // about the respawn point, not about the launch.
            if (pos - previous).length() > 200.0 {
                break;
            }
            previous = pos;
            peak_rise = peak_rise.max(start.y - pos.y);
            if !app
                .world()
                .get::<BodyGroundState>(victim)
                .is_some_and(|g| g.on_ground)
            {
                left_the_ground = true;
            }
        }
        if app.world().get_entity(strike).is_ok() {
            app.world_mut().entity_mut(strike).despawn();
        }
        Launch {
            left_the_ground,
            peak_rise,
        }
    }

    /// asserts the BODY LEFT THE GROUND and gained height, not that a field
    /// was set: a launch that is written and then eaten sets every field.
    #[test]
    fn an_up_tilt_takes_a_grounded_fighter_off_the_floor() {
        let mut app = open_the_lobby();
        let (attacker, victim) = two_seated_fighters(&mut app);
        let launched =
            strike_a_grounded_fighter(&mut app, attacker, victim, 0, Some(UP_TILT_LAUNCH_DIR));
        assert!(
            launched.left_the_ground,
            "an authored up-launcher left a standing fighter standing — the \
             launch is pointing into the floor"
        );
        assert!(
            launched.peak_rise > 0.0,
            "the victim left the ground without gaining any height ({:.2}px), so \
             nothing about this reads as being launched UP",
            launched.peak_rise
        );
    }

    /// THE OTHER HALF OF THE REPORT, and the guard that would have caught
    /// it. *"alice is at 1427% and Booul is hitting her, but she's not going
    /// anywhere."* The same move on the same fighter has to send them materially
    /// further at 1427% than at 0% — that is the whole percent meter.
    #[test]
    fn an_up_tilt_launches_much_further_at_a_high_percent() {
        // ⭐⭐ ONE MATCH PER READING, and the second one is why.
        //
        // ⛔⛔ MEASURED 2026-08-25. Both strikes used to land on ONE victim, and
        // the second arrived while the body was still carrying the first: at the
        // moment of contact `hitstop_timer` read 0.088, so the launch that came
        // out was 495.8 px/s where the knockback formula says 3269. The caller
        // read that as "the percent meter is not reaching the launch" while the
        // meter was fine and the FIXTURE was overlapping its own two readings.
        //
        // ⭐ THE PROOF WAS THE ORDER: struck FIRST the 1427% up-tilt launches at
        // 3096.9 px/s and rises 398px — exactly what the formula predicts —
        // and struck SECOND, 495.8 and 38.1. Same damage input, same victim
        // weight, same empty stale queue, same attacker.
        //
        // ⛔ AND WAITING FOR THE THAW WAS NOT ENOUGH. Adding "settled means
        // standing AND unfrozen" moved the second reading to 23.2px and left it
        // failing, because a body that has already been launched once differs
        // from a fresh one in more ways than the freeze. A shared victim cannot
        // be scrubbed back to new; a new victim can.
        //
        // ⇒ each percent gets its own match. It costs one extra boot and it is
        // the only version of this fixture where the two readings differ in the
        // one variable the test's name claims.
        let strike_at = |percent: i32| {
            let mut app = open_the_lobby();
            let (attacker, victim) = two_seated_fighters(&mut app);
            strike_a_grounded_fighter(
                &mut app,
                attacker,
                victim,
                percent,
                Some(UP_TILT_LAUNCH_DIR),
            )
        };
        let fresh = strike_at(0);
        let cooked = strike_at(1427);
        assert!(
            fresh.left_the_ground && cooked.left_the_ground,
            "both strikes have to launch at all before their sizes can be \
             compared: {:.2}px at 0%, {:.2}px at 1427%",
            fresh.peak_rise,
            cooked.peak_rise
        );
        assert!(
            cooked.peak_rise > fresh.peak_rise * 10.0,
            "the SAME move on the SAME fighter lifted them {:.1}px at 0% and \
             {:.1}px at 1427% — the percent meter is not reaching the launch",
            fresh.peak_rise,
            cooked.peak_rise
        );
    }
}

// ── how loud is one tick? ────────────────────────────────────────────────

/// Every cue that reached the SFX channel this tick, by its hashed id.
///
/// read off `OwnedSfxMessage`, which is the channel `audio_play_sfx_messages`
/// consumes — so this counts what the audio backend would be asked to play, not
/// what some upstream system intended.
fn cues_this_tick(app: &mut App) -> Vec<String> {
    use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxMessage};
    let Some(messages) = app
        .world()
        .get_resource::<bevy::prelude::Messages<OwnedSfxMessage>>()
    else {
        return Vec::new();
    };
    let mut cursor = messages.get_cursor();
    cursor
        .read(messages)
        .filter_map(|m| match &m.request {
            SfxMessage::Play { id, .. } => Some(format!("{id}")),
            _ => None,
        })
        .collect()
}

/// NO CUE IS ASKED FOR MORE THAN ONCE IN A SINGLE TICK.
///
/// the claim is deliberately about ONE CUE, not about total loudness. A
/// busy scene legitimately plays many DIFFERENT cues at once — that is a mix
/// problem, and it is a taste call. The same cue twice in one tick is never
/// intended by anyone, and it is mechanically ugly rather than merely loud.
#[test]
fn no_single_cue_is_asked_for_twice_in_one_tick_during_a_grab() {
    use std::collections::HashMap;

    let (mut app, pad, body) = a_pad_player_fighting_as(OTHER_PREPARED_FIGHTER);

    let mut worst: (usize, String) = (0, String::new());
    let mut heard_anything = false;
    let mut total_cues = 0usize;
    let mut ticks_with_move = 0usize;
    let mut totals: HashMap<String, usize> = HashMap::new();
    for tick in 0..600 {
        app.update();
        if app
            .world()
            .get::<ambition_platformer2d::combat::moveset::MovePlayback>(body)
            .is_some()
        {
            ticks_with_move += 1;
        }
        // Mash attack, and reach for a grab every so often: an exchange, not a
        // single move.
        match tick % 20 {
            0 => pad_hold(&mut app, pad, GamepadButton::South, 1.0),
            4 => pad_hold(&mut app, pad, GamepadButton::South, 0.0),
            10 => pad_hold(&mut app, pad, GamepadButton::North, 1.0),
            14 => pad_hold(&mut app, pad, GamepadButton::North, 0.0),
            _ => {}
        }
        let cues = cues_this_tick(&mut app);
        total_cues += cues.len();
        if !cues.is_empty() {
            heard_anything = true;
        }
        let mut counts: HashMap<&String, usize> = HashMap::new();
        for cue in &cues {
            *counts.entry(cue).or_default() += 1;
            *totals.entry(cue.clone()).or_default() += 1;
        }
        for (cue, n) in counts {
            if n > worst.0 {
                worst = (n, cue.clone());
            }
        }
    }
    let _ = body;
    println!(
        "[sfx-census] {total_cues} cue(s) over 600 ticks, {ticks_with_move} of them          mid-move; worst single tick = {} copies of one cue",
        worst.0.max(1)
    );

    // THE ZERO FLOOR, and it is the whole reason this test is trustworthy.
    // A run that produced NO cues at all would satisfy "no cue played twice"
    // while measuring nothing — which is exactly what a first attempt at this
    // measurement did on a narrower fixture, reporting a confident `0`.
    assert!(
        heard_anything,
        "not one cue reached the SFX channel across 600 ticks of a real match, \
         so this measured nothing at all"
    );
    assert!(
        worst.0 <= 1,
        "cue {} was asked for {} times in a SINGLE tick — that is not {} times \
         louder, it is one sample against phase-aligned copies of itself",
        worst.1,
        worst.0,
        worst.0
    );

    // A flurry is a cue repeating at high rate over TIME, not N copies inside one tick, so a
    // per-tick check alone would have reported that scene clean.
    //
    // the ceiling is deliberately generous — 10 plays/second sustained. A
    // rapid-jab move legitimately machine-guns a cue for a few frames; what
    // nobody authors is one cue holding that rate across a whole second.
    const CUE_PLAYS_PER_SECOND: usize = 10;
    let seconds = 600 / 60;
    for (cue, plays) in &totals {
        assert!(
            *plays <= CUE_PLAYS_PER_SECOND * seconds,
            "cue {cue} played {plays} times in {seconds}s — {:.1}/s sustained, \
             which is the shape of a flurry even though no single tick doubled it",
            *plays as f32 / seconds as f32
        );
    }
}

/// THE PROBE. A FIGHTER'S HUD PANEL DRAWS ONE FACE, NOT THE WHOLE PORTRAIT PAGE.
///
/// `HudStanding` used to carry an image path and nothing else, so the renderer
/// loaded a portrait sheet whole into a 56px box. Oiler's page is 2048x320 —
/// eight frames — and it drew as a strip of eight tiny faces. The select grid
/// had already hit this and cropped by hand; the HUD had no way to, because a
/// path cannot say which frame it means.
///
/// The invariant is the one that outlives the fix: whatever the panel draws, it
/// is at most ONE frame of the page. A sheet wider than its own frame must come
/// back cropped, and the crop must be that frame's size — an assertion that the
/// rect merely EXISTS would pass on a rect covering the whole strip.
#[test]
fn a_fighter_with_a_multi_frame_portrait_gets_one_frame_on_the_hud() {
    use ambition_platformer2d::character::{
        portrait_for_declared_character, CharacterCatalog, PortraitSheetRegistry,
    };
    use ambition_platformer2d::presentation::HudReadouts;

    // Oiler is the character the select screen's own note names, and he is on
    // the grid, so this is the shipped population rather than a fixture.
    const MULTI_FRAME_FIGHTER: &str = "npc_oiler";
    let (mut app, _body) = a_person_fighting_as(MULTI_FRAME_FIGHTER);
    app.update();

    // NON-VACUITY, first: a single-frame sheet would pass every assertion below
    // while proving nothing, so establish that this fighter's page really is
    // wider than one frame before reading the HUD at all.
    let (frame_width, page_frames) = {
        let world = app.world();
        let registry = world.resource::<PortraitSheetRegistry>();
        let catalog = world.resource::<CharacterCatalog>();
        let reference =
            portrait_for_declared_character(Some(registry), catalog, None, MULTI_FRAME_FIGHTER)
                .expect("the seated fighter resolves a portrait");
        let manifest = registry
            .get(&reference.manifest)
            .expect("its manifest is baked");
        let frames: usize = manifest.clips.values().map(|clip| clip.frames.len()).sum();
        (manifest.frame_width as f32, frames)
    };
    assert!(
        page_frames > 1,
        "{MULTI_FRAME_FIGHTER}'s portrait page holds {page_frames} frame(s), so this test \
         cannot tell a crop from no crop. Point it at a fighter whose portraits animate."
    );

    let standing = {
        let world = app.world();
        let readouts = world.resource::<HudReadouts>();
        // SEAT ZERO's panel, not "whichever panel has a portrait" — the person
        // is the one fighting as the multi-frame character, and the CPU beside
        // them is not. A search would silently read the wrong panel the day the
        // opponent's art changes.
        let slot = ambition_demo_smash::FIGHTER_HUD_SLOTS[0].into();
        readouts
            .get(&slot)
            .and_then(|readout| readout.standing_of())
            .cloned()
            .expect("a live match published no standing for the person's own panel")
    };

    let frame = standing.portrait_frame.expect(
        "the HUD published a portrait path with no frame, so the renderer will draw the \
         whole page — every clip this character owns — inside one 56px panel",
    );
    assert_eq!(
        frame.width(),
        frame_width,
        "the HUD's crop is {} wide but one portrait frame is {frame_width}; a crop that \
         spans more than one frame is the same bug wearing a rect",
        frame.width(),
    );
}

/// THE SAME COUCH, CLAIMED IN THE OTHER ORDER: a PAD takes card one and the
/// KEYBOARD takes card two.
///
/// ⛔⛔ THIS IS THE POISON THE FORWARD TEST COULD NOT BE. Its arrangement —
/// keyboard on card one, pad on card two — happens to agree with the stale
/// assumption baked into seat spawning ("primary = keyboard and pad, extra seats
/// = pad only"), so both the correct implementation and the broken one pass it.
/// Reversed, the broken one gives the keyboard player NO usable input at all and
/// leaves the keyboard driving player one as a second controller.
///
/// ⭐ The roster has recorded the truth the whole time; this pins that the
/// binding layer reads it.
#[test]
fn a_pad_claiming_the_first_card_leaves_the_keyboard_driving_the_second() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;
    use ambition_platformer2d::engine_core::BodyKinematics;
    use bevy::input::gamepad::GamepadButton;

    fn pad_set(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
        app.world_mut()
            .write_message(bevy::input::gamepad::RawGamepadEvent::Button(
                bevy::input::gamepad::RawGamepadButtonChangedEvent::new(pad, button, value),
            ));
    }

    let mut app = shell_host_app();
    let pad = app
        .world_mut()
        .spawn(bevy::input::gamepad::Gamepad::default())
        .id();
    settle(&mut app);
    launch_row(&mut app, "Smash");
    settle(&mut app);

    let layout = screen(&app);
    let pad_click = |app: &mut App, rect: ambition_demo_smash::select_screen::cursor::HitRect| {
        {
            let mut cursors = app.world_mut().resource_mut::<SelectCursors>();
            for seat in 0..4 {
                cursors
                    .seat_mut(seat)
                    .expect("seat is bounded by the loop")
                    .move_to(rect.center());
            }
        }
        pad_set(app, pad, GamepadButton::South, 1.0);
        app.update();
        pad_set(app, pad, GamepadButton::South, 0.0);
        app.update();
        settle(app);
    };

    // THE PAD GOES FIRST. This is the whole point of the fixture.
    pad_click(&mut app, layout.role_button(0));
    click(&mut app, layout.role_button(1));
    assert_eq!(
        app.world().resource::<SmashSelect>().slot(0).occupant,
        SlotOccupant::Controller { device: 1 },
        "card ONE was not claimed by the pad, so this fixture is not the reversed \
         arrangement it exists to test"
    );
    let occupants: Vec<_> = (0..4)
        .map(|i| app.world().resource::<SmashSelect>().slot(i).occupant)
        .collect();
    assert_eq!(
        app.world().resource::<SmashSelect>().slot(1).occupant,
        SlotOccupant::Controller { device: 0 },
        "card TWO was not claimed by the keyboard; slots are {occupants:?}"
    );

    let token_zero = placed_token(&app, 0);
    pad_click(&mut app, token_zero);
    pad_click(&mut app, layout.portrait(0).expect("an authored portrait"));
    let token_one = placed_token(&app, 1);
    click(&mut app, token_one);
    click(&mut app, layout.portrait(1).expect("an authored portrait"));
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
        "a pad player and a keyboard player have to seat two fighters; got {}",
        bodies.len()
    );
    let (_, pad_body) = bodies[0];
    let (_, keyboard_body) = bodies[1];

    wait_for_the_round_to_go_live(&mut app);
    let x = |app: &App, body: Entity| app.world().get::<BodyKinematics>(body).unwrap().pos.x;

    // THE KEYBOARD PLAYER — in seat TWO this time — walks right.
    let (before_pad, before_keyboard) = (x(&app, pad_body), x(&app, keyboard_body));
    Buttonlike::press(&KeyCode::ArrowRight, app.world_mut());
    for _ in 0..40 {
        app.update();
    }
    Buttonlike::release(&KeyCode::ArrowRight, app.world_mut());
    let keyboard_moved = x(&app, keyboard_body) - before_keyboard;
    let pad_moved = x(&app, pad_body) - before_pad;
    assert!(
        keyboard_moved.abs() > 1.0,
        "the keyboard player claimed card TWO, pressed right, and their fighter did \
         not move ({keyboard_moved:.2}px). Their seat was built gamepad-only because \
         the binding layer assumed non-primary seats are pad seats — so the person on \
         the keyboard has no controls at all."
    );
    assert!(
        pad_moved.abs() < keyboard_moved.abs() * 0.25,
        "a KEYBOARD press moved the PAD player's fighter ({pad_moved:.2}px against the \
         keyboard player's {keyboard_moved:.2}px) — the keyboard is still aliased onto \
         player one as an unintended second controller.\n\
         ⚠ CHECK THE SIGNS AND THE SEPARATION ({:.2}px apart on x) before believing \
         that: two overlapping fighters push each other apart, which this shape of \
         assertion has misreported as crosstalk before.",
        (x(&app, keyboard_body) - x(&app, pad_body)).abs()
    );

    // AND THE PAD STILL DRIVES ITS OWN SEAT. Without this arm a build that gave
    // NOBODY the keyboard would pass everything above.
    for _ in 0..120 {
        app.update();
    }
    let (before_pad, before_keyboard) = (x(&app, pad_body), x(&app, keyboard_body));
    pad_set(&mut app, pad, GamepadButton::DPadRight, 1.0);
    for _ in 0..40 {
        app.update();
    }
    pad_set(&mut app, pad, GamepadButton::DPadRight, 0.0);
    let pad_moved = x(&app, pad_body) - before_pad;
    let keyboard_moved = x(&app, keyboard_body) - before_keyboard;
    assert!(
        pad_moved.abs() > 1.0,
        "the pad player in card ONE pressed right and their fighter did not move \
         ({pad_moved:.2}px)"
    );
    assert!(
        keyboard_moved.abs() < pad_moved.abs() * 0.25,
        "the pad moved the KEYBOARD player's fighter ({keyboard_moved:.2}px against \
         {pad_moved:.2}px); separation {:.2}px",
        (x(&app, keyboard_body) - x(&app, pad_body)).abs()
    );
}

/// ⛔⛔ LEAVING THE STAGE IS NOT RETRACTABLE, SO IT WAITS FOR A CONFIRMED FRAME.
///
/// The return countdown used to arm on `StocksMatchDecided`, which a SPECULATIVE
/// frame can write, and it lives in a `Local` that GGRS never rewinds. A decision
/// that was later rolled back still sent the player back to the lobby out of a
/// match that was still being fought, and there is no retraction to write — the
/// only fix is not to commit.
///
/// ⭐ THE ARMS STRADDLE `fully_confirmed`, with everything else held still: same
/// settled match, same route, same countdown budget, one field of the boundary
/// different. ⚠ this host publishes NO boundary of its own (there is no GGRS
/// session behind it), so the inserted one survives — which is what makes the
/// first arm possible at all.
#[test]
fn the_return_countdown_does_not_arm_on_a_speculative_verdict() {
    use ambition_platformer2d::engine_core::ConfirmedFrameBoundary;

    let mut app = open_the_lobby();
    pick_and_start(&mut app, PREPARED_FIGHTER);
    wait_for_the_round_to_go_live(&mut app);
    assert!(
        app.world()
            .get_resource::<ConfirmedFrameBoundary>()
            .is_none(),
        "this host publishes a boundary of its own, so the values inserted below \
         are overwritten and neither arm means anything"
    );

    // PREDICTED: the world is ahead of what can never be simulated again.
    app.world_mut().insert_resource(ConfirmedFrameBoundary {
        current: 120,
        confirmed: 100,
        session: 0,
    });
    let running = app
        .world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::ActiveMatch>()
        .cloned()
        .expect("a live match");
    settle_the_match_by_knockout(&mut app);

    for _ in 0..600 {
        app.update();
        // Re-assert the prediction: the sim keeps running and nothing else here
        // maintains this resource.
        app.world_mut().insert_resource(ConfirmedFrameBoundary {
            current: 120,
            confirmed: 100,
            session: 0,
        });
    }
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actors::features::stocks_match::StocksMatchSettled>()
            .is_some_and(|settled| settled.settled(&running)),
        "the match never settled at all, so the refusal below is about nothing"
    );
    assert_eq!(
        active_route(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        "a verdict reached on a SPECULATIVE frame sent the player back to the \
         lobby — and a `Local` countdown cannot be rewound, so nothing was ever \
         going to take that back"
    );

    // CONFIRMED: the same settlement, now unrewindable.
    app.world_mut().insert_resource(ConfirmedFrameBoundary {
        current: 120,
        confirmed: 120,
        session: 0,
    });
    for _ in 0..600 {
        if active_route(&app).as_deref() == Some(ambition_demo_smash::SMASH_SELECT_ROUTE) {
            break;
        }
        app.update();
        app.world_mut().insert_resource(ConfirmedFrameBoundary {
            current: 120,
            confirmed: 120,
            session: 0,
        });
    }
    assert_eq!(
        active_route(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_SELECT_ROUTE),
        "a CONFIRMED verdict never returned the player to select, so the guard \
         above is refusing everything rather than refusing predictions"
    );
}

/// ⭐⭐ "SUDDEN DEATH" STAYS UP, AND THIS IS THE HARNESS THAT WAS MISSING.
///
/// The card lasted about ONE SIMULATION TICK. `announce_the_opening_countdown`
/// owns the slot for any UNSETTLED match, and sudden death deliberately leaves
/// the match unsettled — it is the match CONTINUING, not a result — so the
/// announcer cleared the card on the very next tick while `SuddenDeathBegan`
/// fires once and cannot rewrite it. The fix stands the announcer down on
/// `SuddenDeathEntered`, the canonical latch.
///
/// ⛔⛔ IT WENT IN UNPROVEN, and the reason was written down: `PreparedMatch` has
/// private fields and no constructor, so a unit fixture could only build a system
/// that early-returns — a check that cannot fail. What it needed was a REAL
/// timed match reaching its limit.
///
/// ⭐ AND THE CLOCK IS THE SHORTCUT, not the rule. The stage's limit is eight
/// minutes; nothing about this card depends on how those minutes were spent, and
/// `LiveMatchTicks` IS the state that says how long a match has been fought. So
/// the fixture states a match that has been fought nearly to its limit and lets
/// the real timeout, the real tiebreak and the real announcer do the rest.
#[test]
fn the_sudden_death_card_outlives_the_tick_that_raised_it() {
    use ambition_platformer2d::actors::character_runtime::live_match_clock::LiveMatchTicks;
    use ambition_platformer2d::actors::features::stocks_match::SuddenDeathEntered;

    let mut app = open_the_lobby();
    pick_and_start(&mut app, PREPARED_FIGHTER);
    wait_for_the_round_to_go_live(&mut app);

    let active = app
        .world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::ActiveMatch>()
        .cloned()
        .expect("a live match");
    let limit = app
        .world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::PreparedMatch>()
        .map(|prepared| prepared.rules().time_limit_ticks)
        .expect("the staged match declares a plan");
    assert!(
        limit > 0,
        "this stage runs no clock at all, so there is no timeout to reach"
    );

    // Nearly out of time. Microseconds of live gameplay is what the clock
    // counts; ten ticks short of the limit leaves the timeout to happen for
    // real.
    let micros = u64::from(limit.saturating_sub(10)) * 1_000_000 / 60;
    app.world_mut()
        .insert_resource(LiveMatchTicks::from_snapshot(
            Some(active.instance()),
            micros,
        ));

    let mut raised_on = None;
    for frame in 0..600 {
        app.update();
        if app
            .world()
            .get_resource::<SuddenDeathEntered>()
            .is_some_and(|entered| entered.entered(&active))
        {
            raised_on = Some(frame);
            break;
        }
    }
    assert!(
        raised_on.is_some(),
        "a match ten ticks from its limit never reached sudden death, so this \
         fixture never gets to the card at all"
    );

    // ⛔ THE ASSERTION IS THE DURATION. One tick of the right word is exactly
    // what the bug looked like.
    let reads_sudden_death = |app: &App| {
        app.world()
            .resource::<ambition_platformer2d::presentation::HudReadouts>()
            .get(&ambition_platformer2d::presentation::HudSlotId::from(
                ambition_demo_smash::SMASH_ANNOUNCE_HUD_SLOT,
            ))
            .is_some_and(|readout| readout.text() == "SUDDEN DEATH")
    };
    let mut held = 0;
    for _ in 0..120 {
        if reads_sudden_death(&app) {
            held += 1;
        }
        app.update();
    }
    assert!(
        held > 30,
        "the sudden-death card was up for {held} of 120 frames — the opening \
         announcer is taking the slot back, which is what made it unreadable"
    );
}

/// ⛔⛔ AND THE WINNER CARD IS NOT RETRACTABLE EITHER.
///
/// It read `StocksMatchDecided`, which a SPECULATIVE frame can write — so a
/// rolled-back verdict left NO CONTEST on screen over a match that was still
/// being fought, and there is no retraction to write.
///
/// ⛔ WAITING FOR CONFIRMATION ON THE MESSAGE WOULD NOT DO IT. A reader that
/// keeps its cursor is still bounded by a two-frame channel, so a confirmation
/// arriving later than that loses the announcement rather than delaying it. ⇒
/// the VERDICT moved onto `StocksMatchSettled`, which is rollback state stamped
/// with the match it is about — state has no cursor.
///
/// ⭐ THE ARMS STRADDLE `fully_confirmed` with everything else held still.
#[test]
fn the_winner_card_does_not_show_a_speculative_verdict() {
    use ambition_platformer2d::engine_core::ConfirmedFrameBoundary;

    let mut app = open_the_lobby();
    pick_and_start(&mut app, PREPARED_FIGHTER);
    wait_for_the_round_to_go_live(&mut app);
    assert!(
        app.world()
            .get_resource::<ConfirmedFrameBoundary>()
            .is_none(),
        "this host publishes a boundary of its own, so the values inserted below \
         are overwritten and neither arm means anything"
    );

    let card = |app: &App| -> Option<String> {
        app.world()
            .resource::<ambition_platformer2d::presentation::HudReadouts>()
            .get(&ambition_platformer2d::presentation::HudSlotId::from(
                ambition_demo_smash::SMASH_ANNOUNCE_HUD_SLOT,
            ))
            .map(|readout| readout.text().to_string())
    };
    let predicted = ConfirmedFrameBoundary {
        current: 120,
        confirmed: 100,
        session: 0,
    };
    let confirmed = ConfirmedFrameBoundary {
        current: 120,
        confirmed: 120,
        session: 0,
    };

    app.world_mut().insert_resource(predicted);
    let running = app
        .world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::ActiveMatch>()
        .cloned()
        .expect("a live match");
    settle_the_match_by_knockout(&mut app);
    for _ in 0..30 {
        app.update();
        app.world_mut().insert_resource(predicted);
    }
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actors::features::stocks_match::StocksMatchSettled>()
            .is_some_and(|settled| settled.settled(&running)),
        "the match never settled, so the refusal below is about nothing"
    );
    assert_eq!(
        card(&app),
        None,
        "a verdict reached on a SPECULATIVE frame was put on the winner card — \
         and a HUD readout cannot be taken back"
    );

    app.world_mut().insert_resource(confirmed);
    for _ in 0..30 {
        app.update();
        app.world_mut().insert_resource(confirmed);
    }
    let shown = card(&app).unwrap_or_default();
    assert!(
        shown.starts_with("WINNER"),
        "a CONFIRMED verdict never reached the card ({shown:?}), so the guard \
         above is refusing everything rather than refusing predictions"
    );
}
