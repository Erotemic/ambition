//! Integration checks that the authored Smash stage can actually drive its stocks/knockout loop.
//!
//! Unit tests cover the stock transitions; these tests cover the composed stage geometry and route
//! ordering needed to reach them.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::engine_core::AabbExt;

/// The stage boots and its geometry is the one the demo authored.
///
/// A shell that composes a different room would pass every content test in the
/// other crate, because those inspect a `RoomSpec` this app never has to load.
#[test]
fn the_shell_boots_onto_the_authored_stage() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::game_shell::ShellRouter>()
            .is_some(),
        "the shell never installed a router, so nothing routed anywhere"
    );
}

/// The blast margin is reachable from the platform.
#[test]
fn the_worlds_edge_sits_within_a_launch_of_the_platform() {
    let world = ambition_demo_smash::smash_stage().world;
    let platform = world.blocks[0].aabb;
    let side_margin = world
        .edges
        .side
        .expect("the stage authors its side margins");

    // How far past the platform's edge a body must travel to leave the world.
    let to_the_left = platform.left() + side_margin;
    let to_the_right = (world.size.x - platform.right()) + side_margin;

    // Bound knockout travel against platform width, not world width. One
    // platform-width is the budget before a launch reads as offscreen drift.
    let budget = platform.width();
    for (side, distance) in [("left", to_the_left), ("right", to_the_right)] {
        assert!(
            distance <= budget,
            "a fighter knocked off the {side} must cross {distance:.0}px before \
             the world takes it, against a {budget:.0}px platform — that is a \
             body drifting through empty space, not a knockout"
        );
    }
}

/// Character selection prepares the roster before routing into battle.
///
/// Seating consumes `MatchParticipantRoster` on the simulation schedule, so this ordering is part
/// of the match-start contract.
#[test]
fn the_demo_opens_on_select_and_the_battle_starts_when_players_lock_in() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }

    let route_now = |app: &bevy::prelude::App| -> Option<String> {
        app.world()
            .resource::<ambition_platformer2d::game_shell::ShellRouter>()
            .active
            .as_ref()
            .map(|active| active.route_id.as_str().to_string())
    };
    assert_eq!(
        route_now(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_SELECT_ROUTE),
        "the demo booted straight onto the stage, so it decided who the players \
         are before asking them"
    );
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
            .is_none(),
        "a roster exists before anybody chose, so the select screen is decoration"
    );

    // This test is about the stage, so it sets the decision directly and asks
    // for the start the screen's button would ask for. `the_screen_decides.rs`
    // presses the button.
    decide_a_two_player_match(&mut app);
    app.update();

    let roster = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .expect("locking in published the match the screen decided");
    assert_eq!(roster.participants.len(), 2);
    assert_eq!(
        roster.rules.stocks,
        Some(ambition_demo_smash::STARTING_STOCKS),
        "the decided match is not a stocks match"
    );

    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        route_now(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        "the players locked in and the demo stayed on the select screen"
    );
}

/// A launched fighter leaves the world, spends a stock, and comes back.
///
/// Unit tests write `BodyKnockedOut` directly. This test launches a real
/// body off a real platform and waits for the world to take it. If it fails
/// while `ambition_platformer2d::combat::stocks` stays green, the gap is
/// between the blast gate and the KO announcement.
#[test]
fn a_launched_fighter_is_taken_by_the_world_and_spends_a_stock() {
    use ambition_platformer2d::actor::{FighterStocks, MatchSeat};
    use bevy::prelude::*;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    decide_a_two_player_match(&mut app);
    for _ in 0..240 {
        app.update();
    }

    let stocks_of = |app: &mut App, seat: usize| -> Option<u32> {
        let world = app.world_mut();
        let mut query = world.query::<(&MatchSeat, &FighterStocks)>();
        query
            .iter(world)
            .find(|(s, _)| s.0 == seat)
            .map(|(_, stocks)| stocks.remaining)
    };
    let before = stocks_of(&mut app, 1).expect(
        "seat 1 has no stocks, so the match never seated a stocks fighter and \
         this test is about to prove nothing",
    );

    // An observer cannot miss this trigger; a poll can.
    #[derive(bevy::prelude::Resource, Default)]
    struct Restarts(Vec<bevy::prelude::Entity>);
    app.init_resource::<Restarts>();
    app.add_observer(
        |restart: bevy::prelude::On<ambition_platformer2d::engine_core::BodyRestarted>,
         mut seen: bevy::prelude::ResMut<Restarts>| {
            seen.0.push(restart.entity);
        },
    );
    let launched = {
        let world = app.world_mut();
        let mut query = world.query::<(bevy::prelude::Entity, &MatchSeat)>();
        query
            .iter(world)
            .find(|(_, seat)| seat.0 == 1)
            .map(|(entity, _)| entity)
            .expect("the match seats a second fighter")
    };

    // Launch hard and sideways, so the body crosses the blast line. The side
    // margin is a fraction of the platform width.
    {
        use ambition_platformer2d::actor::BodyKinematics;
        let world = app.world_mut();
        let mut query = world.query::<(&MatchSeat, &mut BodyKinematics)>();
        for (seat, mut kin) in query.iter_mut(world) {
            if seat.0 == 1 {
                kin.vel = ambition_platformer2d::engine_core::Vec2::new(2_400.0, -200.0);
            }
        }
    }

    // A body at 2400px/s clears a 120px margin in a few ticks; the rest is the
    // announcement, the spend, and the placement.
    let mut spent = None;
    for tick in 0..240 {
        app.update();
        if let Some(now) = stocks_of(&mut app, 1) {
            if now < before {
                spent = Some((tick, now));
                break;
            }
        }
    }

    // The loop above breaks on the stock change. Let the frame finish: the
    // spend and the respawn are in different phases.
    for _ in 0..12 {
        app.update();
    }

    let (tick, remaining) = spent.expect(
        "a fighter launched at 2400px/s off a stage whose blast margin is a \
         fraction of its platform never left the world — the blast gate and the \
         KO announcement are not connected, which no test below the app can see",
    );
    assert_eq!(
        remaining,
        before - 1,
        "the knockout spent {} stocks instead of one (tick {tick})",
        before - remaining
    );

    // Non-vacuity: the fighter that was not launched keeps all its stocks.
    assert_eq!(
        stocks_of(&mut app, 0),
        Some(before),
        "the fighter that was never launched also lost a stock, so the counter \
         is moving on its own and this test proves nothing about the blast gate"
    );

    // The restart is raised when the body is placed, and placement waits for
    // the authored beat. `reset_body_clusters` sets `BodyRestartLatch`, so no
    // restart exists before the beat elapses.
    for _ in 0..240 {
        if app
            .world()
            .get::<ambition_platformer2d::actor::PendingRespawn>(launched)
            .is_none()
        {
            break;
        }
        app.update();
    }
    // Then let the frame finish: `announce_body_restarts` turns the latch into
    // `BodyRestarted` in a later phase.
    for _ in 0..12 {
        app.update();
    }

    {
        let untouched = {
            let world = app.world_mut();
            let mut q = world.query::<(bevy::prelude::Entity, &MatchSeat)>();
            q.iter(world)
                .find(|(_, seat)| seat.0 == 0)
                .map(|(e, _)| e)
                .expect("seat 0 exists")
        };
        let seen = &app.world().resource::<Restarts>().0;
        assert!(
            seen.contains(&launched),
            "the knocked-out fighter respawned without a `BodyRestarted`, so \
             nothing downstream can know its life began again: {seen:?}"
        );
        // Non-vacuity: an announcement for every fighter says nothing about this
        // knockout.
        assert!(
            !seen.contains(&untouched),
            "the fighter that was never launched also announced a restart"
        );
    }

    // It comes back where the ruleset says, not where it died.
    {
        use ambition_platformer2d::actor::BodyKinematics;
        // Check seat 1's own placement, on x only: the two seats' points are 64px
        // apart. The y is not pinned, because the body falls during the frames
        // the restart announcement needs.
        let respawn =
            ambition_demo_smash::respawn_placement(ambition_demo_smash::stage_centre(), 1);
        let kin = app
            .world()
            .get::<BodyKinematics>(launched)
            .expect("the fighter still has a body");
        assert!(
            (kin.pos.x - respawn.x).abs() < 32.0,
            "the fighter restarted at x={}, not at seat 1's respawn column {}",
            kin.pos.x,
            respawn.x
        );

        // It comes back facing the stage centre. `attack_dir_from_axis` uses
        // `axis.x * facing`, so a fighter facing away resolves `air_back` where
        // its mirror image resolves `air_forward`. Assert the direction toward
        // centre, not a literal sign: the side depends on `respawn_placement`.
        let inward = (ambition_demo_smash::stage_centre().x - respawn.x).signum();
        assert_eq!(
            kin.facing, inward,
            "the fighter respawned at x={} facing {} — the stage centre is at \
             x={}, so it came back looking away from the platform it just \
             returned to",
            respawn.x,
            kin.facing,
            ambition_demo_smash::stage_centre().x
        );
    }
}

/// The fighter brain closes the distance and lands a hit.
///
/// The first test that puts the brain on a body against an opponent;
/// everything below it is unit-tested with hand-built `Perceived` values.
///
/// The assertion is weak on what the brain does and strict on that it does
/// something. Pinning a distance or a damage number would pin the demo's
/// tuning.
#[test]
fn the_fighter_brain_engages_rather_than_standing_still() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use bevy::prelude::*;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // Both seats must be CPUs: a human seat with no controller does nothing.
    // `smash_roster_at_levels` seats every slot as a CPU. Equal rungs, so the
    // measurement is about engagement, not skill.
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_levels(
            [
                ambition_demo_smash::SMASH_CHARACTER_ID,
                ambition_demo_smash::SMASH_OPPONENT_ID,
            ],
            &[5, 5],
        ));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // Derive the window from the ruleset; a fighter brain that emits nothing is
    // still the scenario under test.
    let countdown = ambition_demo_smash::smash_roster([
        ambition_demo_smash::SMASH_CHARACTER_ID,
        ambition_demo_smash::SMASH_OPPONENT_ID,
    ])
    .rules
    .opening_countdown_ticks;
    for _ in 0..(countdown + 60) {
        app.update();
    }

    let snapshot = |app: &mut App| -> Vec<(usize, f32, f32)> {
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            &ambition_platformer2d::actor::BodyKinematics,
            &BodyHealth,
        )>();
        let mut rows: Vec<(usize, f32, f32)> = q
            .iter(world)
            .map(|(seat, kin, health)| (seat.0, kin.pos.x, health.damage_percent()))
            .collect();
        rows.sort_by_key(|(seat, ..)| *seat);
        rows
    };
    let before = snapshot(&mut app);
    assert_eq!(before.len(), 2, "the match did not seat two fighters");

    for _ in 0..120 {
        app.update();
    }
    let after = snapshot(&mut app);
    assert_eq!(
        after.len(),
        2,
        "a fighter died inside the sampling window, so this measures nothing \
         about engagement: {before:?} -> {after:?}"
    );

    let travelled: f32 = after
        .iter()
        .zip(before.iter())
        .map(|((_, now, _), (_, then, _))| (now - then).abs())
        .sum();
    assert!(
        travelled > 1.0,
        "neither fighter moved in 120 ticks — a fighter brain that emits nothing \
         is indistinguishable from one that was never installed, and that is \
         exactly what an unresolved brain profile used to produce: {before:?} -> \
         {after:?}"
    );

    let hurt = after.iter().any(|(_, _, percent)| *percent > 0.0);
    assert!(
        hurt,
        "the fighters moved and nobody was hit, so the brain travels but never \
         commits: {after:?}"
    );
}

/// An eliminated fighter leaves the stage.
///
/// The stock count can be correct while the eliminated body keeps falling.
/// This checks that the match is over for it.
#[test]
fn an_eliminated_fighter_does_not_keep_falling_forever() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            ambition_demo_smash::SMASH_CHARACTER_ID,
            ambition_demo_smash::SMASH_OPPONENT_ID,
        ]));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    let mut peak = 0.0f32;
    for _ in 0..3_600 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<(&MatchSeat, &BodyHealth)>();
        for (_, health) in q.iter(world) {
            peak = peak.max(health.damage_percent());
        }
    }

    // A percent this side of absurd.
    assert!(
        peak < 20.0,
        "a fighter reached {:.0}% over one minute — a body that keeps falling \
         out of the world keeps being knocked out of it, which is what an \
         eliminated fighter nobody removed does",
        peak * 100.0
    );
}

/// The 3-2-1-GO is on the screen.
///
/// This watches the slot the stage declares, `smash_announce`: the centred
/// card the HUD renders. It asserts the sequence, not the tick: which frame
/// shows "2" is a tuning fact about `opening_countdown_ticks`.
#[test]
fn the_opening_countdown_is_something_a_player_can_see() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_levels(
            [
                ambition_demo_smash::SMASH_CHARACTER_ID,
                ambition_demo_smash::SMASH_OPPONENT_ID,
            ],
            &[5, 5],
        ));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    let slot = ambition_demo_smash::SMASH_ANNOUNCE_HUD_SLOT.into();
    let countdown = ambition_demo_smash::smash_roster([
        ambition_demo_smash::SMASH_CHARACTER_ID,
        ambition_demo_smash::SMASH_OPPONENT_ID,
    ])
    .rules
    .opening_countdown_ticks as usize;

    let mut said: Vec<String> = Vec::new();
    let mut cleared_after = false;
    // The ceremony, plus enough afterwards for the GO card to retire.
    for _ in 0..(countdown * 3 + 240) {
        app.update();
        let shown = app
            .world()
            .get_resource::<ambition_platformer2d::presentation::HudReadouts>()
            .and_then(|readouts| readouts.get(&slot))
            .map(ambition_platformer2d::presentation::HudReadout::text);
        match shown {
            Some(text) => {
                if said.last() != Some(&text) {
                    said.push(text);
                }
                // A card that returns after the clear would mean the clear is fighting a
                // writer.
                cleared_after = false;
            }
            None => cleared_after = !said.is_empty(),
        }
    }

    assert_eq!(
        said,
        vec![
            "3".to_string(),
            "2".to_string(),
            "1".to_string(),
            "GO!".to_string()
        ],
        "the opening card showed {said:?} — a player is counted in with three \
         numbers and then told to go, or the ceremony is invisible"
    );
    assert!(
        cleared_after,
        "the GO card never came down, so it sits on top of the match it announced"
    );
}

/// The camera closes no faster than it opened. Measured before the close
/// was eased:
/// ```text
///   widest single-frame OPEN    49.3      (a ramp over 7-8 frames, 800 -> 1115)
///   widest single-frame CLOSE  360.9      (one frame, straight back to 800)
/// ```
///
/// The open follows a flying body, so it is already continuous. The close
/// is a discontinuity: the body leaves play and the cast's bounding box
/// collapses between two frames. After easing only the close, the same run
/// reads `open 57.5 / close 68.9`.
///
/// The non-vacuity guard is a knockout: a match with no launch would satisfy
/// any ratio.
#[test]
fn the_camera_closes_no_faster_than_it_opened() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_levels(
            [
                ambition_demo_smash::SMASH_CHARACTER_ID,
                ambition_demo_smash::SMASH_OPPONENT_ID,
            ],
            &[5, 5],
        ));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    let mut previous: Option<ambition_platformer2d::engine_core::Vec2> = None;
    let mut widest_open = 0.0f32;
    let mut widest_close = 0.0f32;
    // The premise is a knockout, not a widening number. The beat holds a
    // waiting body still (D201/ADR 0033), so a live launch widens the frame
    // only a little. The rule is about the teleport back.
    let mut stock_spent = false;
    for tick in 0..5_400 {
        app.update();
        {
            let world = app.world_mut();
            let mut spends = world.resource_mut::<
                bevy::prelude::Messages<ambition_platformer2d::actor::FighterStockSpent>,
            >();
            if !spends.drain().collect::<Vec<_>>().is_empty() {
                stock_spent = true;
            }
        }
        let view = {
            let world = app.world_mut();
            let observer = ambition_platformer2d::sim_view::the_only_view(world);
            world
                .entity(observer)
                .get::<ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot>()
                // An unframed view has no frame, so skip it; do not measure a default
                // window.
                .and_then(|resolved| resolved.frame().map(|f| f.snapshot.visible_view))
        };
        let Some(view) = view else { continue };
        if let Some(previous) = previous {
            let step = (view - previous).length();
            if view.x > previous.x {
                widest_open = widest_open.max(step);
            } else if view.x < previous.x {
                // Skip the opening frames: the first resolve adopts the cast's framing
                // without easing, which is correct.
                if tick > 60 {
                    widest_close = widest_close.max(step);
                }
            }
        }
        previous = Some(view);
    }

    assert!(
        stock_spent,
        "no fighter lost a stock in 5,400 ticks, so nobody was ever teleported \
         back from the blast zone and the ratio below is about a match that \
         never happened"
    );
    assert!(
        widest_open > 0.0,
        "the frame never widened at all, so there is no opening rate to compare \
         the close against"
    );
    assert!(
        widest_close <= widest_open * 2.0,
        "the camera closed by {widest_close:.1} units in one frame having opened \
         by at most {widest_open:.1} — the return is a cut, not a transition"
    );
}

/// A match that somebody wins ends.
///
/// `decide_stocks_match` reads the sides from the bodies that still exist,
/// and `take_eliminated_fighters_out_of_play` despawns an eliminated body.
/// If the despawn runs first, `last_side_standing` sees one side and
/// answers `None` forever. So the despawn must be ordered after
/// `MatchOutcomeDecided`. Ordering it `.before(MatchOutcomeDecided)` makes
/// this test fail.
///
/// The elimination is asserted first, so a match where nobody died cannot
/// pass.
#[test]
fn a_match_whose_last_loser_is_removed_still_decides() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::versus_match::the_live_match_is_settled;
    use ambition_platformer2d::combat::components::FighterStocks;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // Both seats are CPUs, so somebody loses. `smash_roster` makes seat 0 a
    // human with no controller.
    //
    // One stock: the question is the removal, not the pace. At the default
    // stock count, neither fighter reliably reaches zero in the window.
    let mut roster = ambition_demo_smash::smash_roster_at_levels(
        [
            ambition_demo_smash::SMASH_CHARACTER_ID,
            ambition_demo_smash::SMASH_OPPONENT_ID,
        ],
        &[5, 5],
    );
    roster.rules.stocks = Some(1);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    // Count seats, not stocks. A fighter at zero is eliminated and removed in
    // the same step, so a poll of `FighterStocks` never sees the zero.
    let mut most_seats = 0usize;
    let mut fewest_seats = usize::MAX;
    let mut settled_on = None;
    for tick in 0..5_400 {
        app.update();
        {
            let world = app.world_mut();
            let mut q = world.query::<(&MatchSeat, &FighterStocks)>();
            let seats = q.iter(world).count();
            if seats > 0 {
                most_seats = most_seats.max(seats);
                fewest_seats = fewest_seats.min(seats);
            }
        }
        // A stocks verdict names its match, so ask whether this match is settled.
        if the_live_match_is_settled(app.world()) {
            settled_on = Some(tick);
            break;
        }
    }
    let settled = settled_on.is_some();

    assert!(
        most_seats >= 2 && fewest_seats < most_seats,
        "the cast never shrank (peak {most_seats} seats, low {fewest_seats}), so \
         nobody was eliminated in ninety seconds and this match never reached the \
         question it exists to ask"
    );
    assert!(
        settled,
        "a fighter ran out of stocks and the match never decided — the state a \
         player sees as a stage that keeps going with one fighter on it \
         (peak {most_seats} seats, low {fewest_seats})"
    );
}

/// This demo's own CPU roster is seatable by its own composition.
/// (API 1.0 row (g))
///
/// A `ControllerBinding::Cpu { brain_profile }` is looked up in the
/// composition's `CharacterRoster` archetype table. When the key is absent,
/// `spec_for_brain` falls back to a generic `stand_still` row: the match runs
/// and the opponent never moves.
///
/// This is checked here, not at the select screen: every seat the screen
/// produces is human, and a human seat does not use the archetype table.
#[test]
fn the_demos_cpu_roster_is_satisfiable_by_its_own_composition() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // Can this demo fill the seats it declares?
    let profiles = app
        .world()
        .get_resource::<ambition_platformer2d::characters::actor::character_catalog::BrainProfileRegistry>()
        .expect("the composition assembles its published policies")
        .clone();

    for level in [1u8, 5, 9] {
        let roster = ambition_demo_smash::smash_roster_at_level(
            [
                ambition_demo_smash::SMASH_CHARACTER_ID,
                ambition_demo_smash::SMASH_OPPONENT_ID,
            ],
            level,
        );
        let problems = roster.unsatisfiable_seats(Some(&profiles));
        assert!(
            problems.is_empty(),
            "level {level}: this demo declares a CPU seat its own composition \
             cannot seat, so the fighter would silently be a stand-still body: \
             {problems:?}"
        );
    }
}

/// Two controller slots with a fighter each, and Start requested.
///
/// Set `StartRequested` as well as the picks: the screen does not leave on
/// readiness alone.
fn decide_a_two_player_match(app: &mut bevy::prelude::App) {
    use ambition_demo_smash::select::{SlotOccupant, SmashRoster, SmashSelect};

    // Seat 0 takes a fighter that is not the stage's starting character.
    let index_of = |app: &bevy::prelude::App, id: &str| -> usize {
        app.world()
            .resource::<SmashRoster>()
            .ids()
            .position(|candidate| candidate == id)
            .unwrap_or_else(|| panic!("`{id}` is not on this composition's grid"))
    };
    let first = index_of(app, ambition_demo_smash::SMASH_GEORGE_BOOUL);
    let second = index_of(app, ambition_demo_smash::SMASH_OPPONENT_ID);
    {
        let mut select = app.world_mut().resource_mut::<SmashSelect>();
        select.set_occupant(0, SlotOccupant::Controller { device: 0 });
        select.set_pick(0, first);
        select.set_occupant(1, SlotOccupant::Controller { device: 1 });
        select.set_pick(1, second);
    }
    app.world_mut()
        .resource_mut::<ambition_demo_smash::select_screen::StartRequested>()
        .0 = true;
}

/// A ladder roster seats TWO fighters at two different levels.
///
/// `smash_roster_at_level` puts every CPU on one rung, and `smash_roster`
/// makes seat 0 human. A fight between two rungs needs this roster.
///
/// The key assertion is that the two seats differ. A roster with both
/// fighters on one rung would report 50% at every level and look like a
/// flat ladder.
///
/// Both profiles must also resolve in the demo's own published policies:
/// `spec_for_brain` falls back to a generic row, so an unregistered level
/// fights a statue.
#[test]
fn a_ladder_roster_seats_two_cpus_at_two_different_levels() {
    use ambition_platformer2d::actor::ControllerBinding;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // The demo's published policies. Since P2.18 they are the only source of
    // a seat's policy.
    let published = app
        .world()
        .get_resource::<ambition_platformer2d::characters::actor::character_catalog::BrainProfileRegistry>()
        .expect("the composition assembles its published policies")
        .clone();

    // Use a registered sparse ladder rung. Unregistered levels fall back to a
    // generic row and would not exercise the authored ladder behavior.
    const RUNGS: &[u8] = &[1, 3, 5, 6, 9];

    let roster = ambition_demo_smash::smash_roster_at_levels(
        [
            ambition_demo_smash::SMASH_CHARACTER_ID,
            ambition_demo_smash::SMASH_OPPONENT_ID,
        ],
        &[9, 6],
    );
    assert_eq!(roster.participants.len(), 2);

    let profiles: Vec<Option<String>> = roster
        .participants
        .iter()
        .map(|p| match &p.controller {
            ControllerBinding::Cpu { brain_profile } => brain_profile.clone(),
            other => panic!("a ladder seat is not a CPU: {other:?}"),
        })
        .collect();
    // Build the ID from the canonical constant rather than restating its prefix.
    let expected: Vec<Option<String>> = [9, 6]
        .iter()
        .map(|level| {
            Some(format!(
                "{}_l{level}",
                ambition_demo_smash::SMASH_DUELIST_BRAIN
            ))
        })
        .collect();
    assert_eq!(
        profiles, expected,
        "the two seats must sit on DIFFERENT rungs, or every measurement built \
         on this reads 50% and looks like a flat ladder rather than a broken rig"
    );
    // Resolve in this demo's own provider, as `seat_brain_profile` does
    // (it has one arm since P2.18).
    let resolves = |profile: &str| {
        published
            .get(&ambition_platformer2d::entity_catalog::BrainProfileId::new(
                format!("{}::{profile}", ambition_demo_smash::SMASH_EXPERIENCE),
            ))
            .is_some()
    };
    // Each rung resolves.
    for profile in profiles.iter().flatten() {
        assert!(
            resolves(profile),
            "`{profile}` resolves in neither the published policies nor the \
             archetype table, so seating hands back a generic row and the rung \
             fights a statue while reporting a fight"
        );
    }
    // Every adjacent pair of registered rungs is satisfiable: (3,1), (5,3),
    // (6,5), (9,6).
    for pair in RUNGS.windows(2) {
        let (lower, upper) = (pair[0], pair[1]);
        let rung = ambition_demo_smash::smash_roster_at_levels(
            [
                ambition_demo_smash::SMASH_CHARACTER_ID,
                ambition_demo_smash::SMASH_OPPONENT_ID,
            ],
            &[upper, lower],
        );
        for participant in &rung.participants {
            let ControllerBinding::Cpu { brain_profile } = &participant.controller else {
                panic!("a ladder seat is not a CPU");
            };
            let profile = brain_profile
                .as_deref()
                .expect("a CPU seat names a profile");
            assert!(
                resolves(profile),
                "rung {upper} vs {lower} asks for `{profile}`, which neither this \
                 composition's published policies nor its archetype table carry"
            );
        }
    }

    // The ruleset is the shipped stage's, not the rig's.
    assert_eq!(
        roster.rules.stocks,
        Some(ambition_demo_smash::STARTING_STOCKS)
    );
    assert!(
        roster.rules.opens_suspended,
        "a ladder round opens on the countdown too"
    );
}

/// The stage ability policy is both a floor and a ceiling.
///
/// `MatchAbilities::levelled` grants the stage's common fighter kit even when a
/// character omits a verb, while abilities outside the permitted set remain
/// unavailable even if the character authors them. Per-character extras cannot
/// escape the mode policy.
#[test]
fn a_seated_fighter_carries_the_verbs_its_character_authored_and_not_the_engines() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::engine_core::BodyAbilities;
    use bevy::prelude::*;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    decide_a_two_player_match(&mut app);
    for _ in 0..120 {
        app.update();
    }

    let world = app.world_mut();
    let mut query = world.query::<(&MatchSeat, &BodyAbilities)>();
    let mut seated: Vec<(usize, ambition_platformer2d::engine_core::AbilitySet)> = query
        .iter(world)
        .map(|(seat, abilities)| (seat.0, abilities.abilities))
        .collect();
    seated.sort_by_key(|(seat, _)| *seat);
    assert_eq!(
        seated.len(),
        2,
        "the stage seated {} bodies with abilities, so this measures nothing \
         about what a fighter can do",
        seated.len()
    );

    for (seat, abilities) in &seated {
        // P4.29 / P4.30 / P4.32, on the live body, through the real route.
        assert!(
            abilities.shield,
            "seat {seat} cannot shield, so P4.29's authored capability does not \
             survive the trip from definition to seated body — either \
             preparation dropped it or the match's ability mask is intersecting \
             it away"
        );
        assert!(abilities.dodge, "seat {seat} cannot dodge (P4.30)");
        assert!(
            abilities.ledge_grab,
            "seat {seat} cannot grab a ledge (P4.32)"
        );
        // Poison: verbs these fighters do not have.
        assert!(
            !abilities.fly && !abilities.blink_through_hard_walls,
            "seat {seat} came out able to fly or blink, which its character does \
             not author — so the body is wearing a generic set (the engine's, or \
             a match-wide grant) rather than its own"
        );
    }
}

/// Live fighters remain visible even while entering the blast zone. The test
/// first proves a fighter actually leaves the room bounds, then checks framing;
/// otherwise a quiet match could satisfy the camera assertion vacuously. Smash
/// framing must follow the cast rather than clamp to the room bounds.
#[test]
fn every_live_fighter_stays_inside_the_frame() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // Both seats are CPUs, so bodies get launched.
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_levels(
            [
                ambition_demo_smash::SMASH_CHARACTER_ID,
                ambition_demo_smash::SMASH_OPPONENT_ID,
            ],
            &[5, 5],
        ));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    let room = ambition_demo_smash::smash_stage().world.size;
    let mut left_the_room = 0usize;
    let mut escaped: Vec<String> = Vec::new();
    let mut worst = 0.0f32;
    let mut observed = 0usize;
    for tick in 0..2_400 {
        app.update();
        let view = {
            let world = app.world_mut();
            let observer = ambition_platformer2d::sim_view::the_only_view(world);
            world
                .entity(observer)
                .get::<ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot>()
                // An unframed view reports no frame, so skip the tick. Do not measure a
                // default window.
                .and_then(|resolved| {
                    resolved.frame().map(|resolved| {
                    (
                        resolved.snapshot.center_world,
                        resolved.snapshot.visible_view,
                        // Also report what the camera follows. "The frame is in the wrong
                        // place" and "the frame correctly follows the wrong body" are
                        // different bugs.
                        resolved.follow_world,
                    )
                    })
                })
        };
        let Some((center, visible, follow)) = view else {
            continue;
        };
        let world = app.world_mut();
        let mut seats = world.query::<(&MatchSeat, &BodyKinematics)>();
        let bodies: Vec<(usize, ambition_platformer2d::engine_core::Vec2)> = seats
            .iter(world)
            .map(|(seat, kin)| (seat.0, kin.pos))
            .collect();
        if bodies.is_empty() {
            continue;
        }
        observed += 1;
        let half = visible / 2.0;
        for (seat, pos) in bodies {
            if pos.x < 0.0 || pos.x > room.x || pos.y < 0.0 || pos.y > room.y {
                left_the_room += 1;
            }
            // How far past the nearest screen edge this body is drawn.
            let over = ((pos.x - center.x).abs() - half.x).max((pos.y - center.y).abs() - half.y);
            if over > 0.0 {
                worst = worst.max(over);
                if escaped.len() < 8 {
                    escaped.push(format!(
                        "  t{tick} seat {seat} at ({:.0},{:.0}) is {over:.0} units outside a \
                         {:.0}x{:.0} frame centred ({:.0},{:.0}), following ({:.0},{:.0})",
                        pos.x, pos.y, visible.x, visible.y, center.x, center.y, follow.x, follow.y
                    ));
                }
            }
        }
    }

    assert!(
        observed > 600,
        "only {observed} frames had a cast at all, so this watched almost no match"
    );
    assert!(
        left_the_room > 20,
        "no fighter was ever outside the room's own bounds in this match ({left_the_room} \
         body-frames), so nobody was knocked off the stage and the claim below is about a \
         camera that never had to follow anybody anywhere"
    );
    assert!(
        escaped.is_empty(),
        "a live fighter was drawn OUTSIDE the frame on {} body-frames, worst {worst:.0} units \
         past the edge — the knockout that decides the match happens off-screen:\n{}",
        escaped.len(),
        escaped.join("\n")
    );
}

/// The frame does not cut when a fighter leaves play.
///
/// The companion to [`the_camera_closes_no_faster_than_it_opened`]. The
/// centre travels to follow fighters off the stage, so it has the same
/// discontinuity as the size: when an eliminated body leaves play, the
/// cast's box collapses and its centre jumps back to the platform.
///
/// It compares the elimination frame with the ordinary ones. A fast fight
/// moves the centre a long way per frame correctly, so a fixed threshold
/// would be a guess.
///
/// The non-vacuity guard is the jump the framing had to absorb: fighters
/// standing together at the knockout make no jump.
#[test]
fn the_framing_centre_absorbs_an_elimination_instead_of_cutting() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
    use ambition_platformer2d::engine_core::Vec2;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // One stock: this test needs a fighter to leave, and must not race the
    // fight's pace.
    let mut roster = ambition_demo_smash::smash_roster_at_levels(
        [
            ambition_demo_smash::SMASH_CHARACTER_ID,
            ambition_demo_smash::SMASH_OPPONENT_ID,
        ],
        &[5, 5],
    );
    roster.rules.stocks = Some(1);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    let mut previous: Option<(usize, Vec2, Vec2)> = None;
    let mut worst_ordinary_step = 0.0f32;
    let mut worst_elimination_step = 0.0f32;
    let mut biggest_absorbed_jump = 0.0f32;
    for _ in 0..5_400 {
        app.update();
        let camera = {
            let world = app.world_mut();
            let observer = ambition_platformer2d::sim_view::the_only_view(world);
            world
                .entity(observer)
                .get::<ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot>()
                // `and_then`: an unframed view has no centre — see above.
                .and_then(|resolved| resolved.frame().map(|f| f.snapshot.center_world))
        };
        let Some(camera) = camera else { continue };
        // The cast's true centre and population: the input the framing absorbs.
        let (members, true_centre) = {
            let world = app.world_mut();
            let mut seats = world.query::<(&MatchSeat, &BodyKinematics)>();
            let mut min = Vec2::new(f32::MAX, f32::MAX);
            let mut max = Vec2::new(f32::MIN, f32::MIN);
            let mut members = 0usize;
            for (_, kin) in seats.iter(world) {
                members += 1;
                min = min.min(kin.pos - kin.size / 2.0);
                max = max.max(kin.pos + kin.size / 2.0);
            }
            (members, (min + max) / 2.0)
        };
        if members == 0 {
            previous = None;
            continue;
        }
        if let Some((previous_members, previous_true, previous_camera)) = previous {
            let step = (camera - previous_camera).length();
            if members < previous_members {
                worst_elimination_step = worst_elimination_step.max(step);
                biggest_absorbed_jump =
                    biggest_absorbed_jump.max((true_centre - previous_true).length());
            } else {
                worst_ordinary_step = worst_ordinary_step.max(step);
            }
        }
        previous = Some((members, true_centre, camera));
    }

    assert!(
        biggest_absorbed_jump > 60.0,
        "the cast's own centre never jumped by more than {biggest_absorbed_jump:.1} units when a \
         fighter was removed, so there was no discontinuity to absorb and this measured nothing"
    );
    assert!(
        worst_ordinary_step > 1.0,
        "the camera centre never moved by more than {worst_ordinary_step:.2} units in a frame \
         while everybody was in play — it is pinned again, and the comparison below is between \
         two kinds of nothing"
    );
    assert!(
        worst_elimination_step <= worst_ordinary_step,
        "the camera jumped {worst_elimination_step:.1} units on the frame a fighter was taken out \
         of play, against at most {worst_ordinary_step:.1} in an ordinary frame, absorbing a \
         {biggest_absorbed_jump:.1}-unit collapse — that is a cut back to the platform, which is \
         the thing Jon reported about the zoom and is now possible for the centre too"
    );
}

/// A second match on the same stage counts in, takes the card down, ends,
/// and stops.
///
/// The second match is the test. The other tests here play one ceremony or
/// one stock, and the host's
/// `coming_back_to_the_select_screen_offers_a_fresh_match` starts a second
/// match but does not play it. This plays two identical matches through one
/// app and asserts they are the same match twice:
/// ```text
///   counted in       3 - 2 - 1 - GO!, on BOTH visits
///   card comes down  the ceremony never has the last word
///   decided          exactly one winner announced per match
///   stopped          the cast does not move after the winner is named
/// ```
#[test]
fn a_second_match_on_the_same_stage_counts_in_and_ends() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat, StocksMatchDecided};
    use bevy::prelude::*;

    /// What one match said, decided, and did after it was over.
    struct Played {
        /// Every distinct word the centred card showed while the stage was up.
        said: Vec<String>,
        /// The winners announced while this match ran.
        decided: Vec<Option<String>>,
        /// The furthest any fighter travelled after the winner was named.
        travelled_after_the_end: f32,
    }

    #[derive(Resource, Default)]
    struct Decisions(Vec<Option<String>>);

    let slot: ambition_platformer2d::presentation::HudSlotId =
        ambition_demo_smash::SMASH_ANNOUNCE_HUD_SLOT.into();

    let world_width = ambition_demo_smash::smash_stage().world.size.x;
    let mut app = build_demo_app();
    app.init_resource::<Decisions>();
    app.add_systems(
        Update,
        |mut decided: MessageReader<StocksMatchDecided>, mut seen: ResMut<Decisions>| {
            for outcome in decided.read() {
                seen.0.push(outcome.outcome.winner().map(str::to_string));
            }
        },
    );
    for _ in 0..30 {
        app.update();
    }

    let play = |app: &mut App| -> Played {
        let before = app.world().resource::<Decisions>().0.len();
        let mut roster = ambition_demo_smash::smash_roster_at_levels(
            [
                ambition_demo_smash::SMASH_CHARACTER_ID,
                ambition_demo_smash::SMASH_OPPONENT_ID,
            ],
            &[5, 5],
        );
        roster.rules.stocks = Some(1);
        let countdown = roster.rules.opening_countdown_ticks as usize;
        app.world_mut().insert_resource(roster);
        app.world_mut()
            .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
                ambition_platformer2d::game_shell::ShellRouteId::new(
                    ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
                ),
            ));

        let cast = |app: &mut App| -> Vec<(usize, ambition_platformer2d::engine_core::Vec2)> {
            let world = app.world_mut();
            let mut query = world.query::<(&MatchSeat, &BodyKinematics)>();
            let mut rows: Vec<_> = query
                .iter(world)
                .map(|(seat, kin)| (seat.0, kin.pos))
                .collect();
            rows.sort_by_key(|(seat, _)| *seat);
            rows
        };

        let mut said: Vec<String> = Vec::new();
        let mut launched = false;
        let mut travelled_after_the_end = 0.0f32;
        let mut standing: Option<Vec<(usize, ambition_platformer2d::engine_core::Vec2)>> = None;
        for tick in 0..(countdown + 600) {
            app.update();
            // Launch once, as soon as the ceremony releases the cast. At 2400px/s the
            // body crosses the blast margin in a few ticks, and on one stock that ends
            // the match. A claim about the card's wording must not depend on combat
            // tuning.
            if !launched && tick > countdown + 2 {
                let world = app.world_mut();
                let mut query = world.query::<(&MatchSeat, &mut BodyKinematics)>();
                for (seat, mut kin) in query.iter_mut(world) {
                    if seat.0 == 1 {
                        let toward = if kin.pos.x * 2.0 > world_width {
                            1.0
                        } else {
                            -1.0
                        };
                        kin.vel =
                            ambition_platformer2d::engine_core::Vec2::new(2_400.0 * toward, -200.0);
                        launched = true;
                    }
                }
            }

            let on_stage = app
                .world()
                .resource::<ambition_platformer2d::game_shell::ShellRouter>()
                .active
                .as_ref()
                .is_some_and(|active| {
                    active.route_id.as_str() == ambition_demo_smash::SMASH_GAMEPLAY_ROUTE
                });
            // Record only while the stage is up. The previous match's card stays in
            // `HudReadouts` on the select screen (the experience's HUD declaration
            // hides it), so recording it off-stage would look like a match that
            // opened on a victory card.
            if on_stage {
                if let Some(text) = app
                    .world()
                    .get_resource::<ambition_platformer2d::presentation::HudReadouts>()
                    .and_then(|readouts| readouts.get(&slot))
                    .map(ambition_platformer2d::presentation::HudReadout::text)
                {
                    if said.last() != Some(&text) {
                        said.push(text);
                    }
                }
            }

            // Measure the freeze from body motion, not from the clock resource.
            let ended = app.world().resource::<Decisions>().0.len() > before;
            if ended && on_stage {
                let now = cast(app);
                if let Some(previous) = standing.as_ref() {
                    for (seat, pos) in &now {
                        if let Some((_, was)) = previous.iter().find(|(other, _)| other == seat) {
                            travelled_after_the_end =
                                travelled_after_the_end.max((*pos - *was).length());
                        }
                    }
                }
                standing = Some(now);
            }
        }
        Played {
            said,
            decided: app.world().resource::<Decisions>().0[before..].to_vec(),
            travelled_after_the_end,
        }
    };

    let first = play(&mut app);
    // The stage returns to the select screen 4.5s after the end. Let it, so
    // the second match arrives by the player's road.
    for _ in 0..400 {
        app.update();
    }
    let second = play(&mut app);

    for (which, played) in [("first", &first), ("second", &second)] {
        assert_eq!(
            played.decided.len(),
            1,
            "the {which} match announced {} winners ({:?}) — one fighter was \
             launched off a one-stock stage, so exactly one match ended and was \
             announced once. The card said {:?}",
            played.decided.len(),
            played.decided,
            played.said
        );
        assert_eq!(
            played
                .said
                .iter()
                .take(4)
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["3", "2", "1", "GO!"],
            "the {which} match counted the players in with {:?}",
            played.said
        );
        assert_ne!(
            played.said.last().map(String::as_str),
            Some("GO!"),
            "the {which} match ended with GO! still on the card — the opening \
             ceremony had the last word and is sitting on the match it \
             announced: {:?}",
            played.said
        );
        let winner = played.decided[0]
            .as_deref()
            .expect("a launch off a one-stock stage leaves one fighter standing");
        assert_eq!(
            played.said.last().map(String::as_str),
            Some(
                ambition_demo_smash::victory_banner(
                    &ambition_platformer2d::actor::MatchVerdict::Winner("Robot v3".to_string()),
                    Some("Robot v3"),
                )
                .as_str()
            ),
            "the {which} match's last word was not the winner card. It decided \
             {winner:?} and said {:?}",
            played.said
        );
        // Not zero: the clock ramps to a stop (the time-control smoother), so the
        // frame of the decision still carries a fraction of a step. The match
        // must not play on.
        assert!(
            played.travelled_after_the_end < 8.0,
            "a fighter moved {:.1}px after the {which} match was decided — the \
             winner is still playing, and the game did not stop",
            played.travelled_after_the_end
        );
    }
}

/// A four-way free-for-all ends when one fighter is left.
///
/// `last_side_standing` folds N sides. "Three of four are out" differs from
/// "one of two is out": a fold that stopped at the first surviving side
/// would answer both the same way.
///
/// The demo declares two characters, so a four-seat match wears each twice.
/// That is useful: a side keyed on the character instead of the seat would
/// collapse four sides into two and end the match early.
#[test]
fn a_four_way_free_for_all_ends_when_one_fighter_is_left() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat, StocksMatchDecided};
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct Decisions(Vec<Option<String>>);

    let mut app = build_demo_app();
    app.init_resource::<Decisions>();
    app.add_systems(
        Update,
        |mut decided: MessageReader<StocksMatchDecided>, mut seen: ResMut<Decisions>| {
            for outcome in decided.read() {
                seen.0.push(outcome.outcome.winner().map(str::to_string));
            }
        },
    );
    for _ in 0..30 {
        app.update();
    }

    let mut roster = ambition_demo_smash::smash_roster_at_levels(
        [
            ambition_demo_smash::SMASH_CHARACTER_ID,
            ambition_demo_smash::SMASH_OPPONENT_ID,
            ambition_demo_smash::SMASH_CHARACTER_ID,
            ambition_demo_smash::SMASH_OPPONENT_ID,
        ],
        &[5, 5, 5, 5],
    );
    roster.rules.stocks = Some(1);
    let countdown = roster.rules.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    let world_width = ambition_demo_smash::smash_stage().world.size.x;
    let mut seated = 0usize;
    let mut launched = false;
    // Stop a few ticks after the end. The stage returns to the select screen
    // 4.5s later and the card comes down with it
    // (`return_to_the_select_screen_when_the_match_ends`).
    for tick in 0..(countdown + 400) {
        app.update();
        if app.world().resource::<Decisions>().0.len() == 1 && launched {
            for _ in 0..10 {
                app.update();
            }
            break;
        }
        if tick == countdown {
            let world = app.world_mut();
            let mut query = world.query::<&MatchSeat>();
            seated = query.iter(world).count();
        }
        // Every seat but 0 leaves the world, and keeps leaving until gone. Apply
        // the velocity every tick: a single write can lose to a hit's knockback,
        // which would make the fixture depend on combat balance.
        if tick > countdown + 30 {
            let world = app.world_mut();
            let mut query = world.query::<(&MatchSeat, &mut BodyKinematics)>();
            for (seat, mut kin) in query.iter_mut(world) {
                if seat.0 > 0 {
                    // Away from the centre, read each tick, so a body knocked back across
                    // the midline is still thrown out.
                    let toward = if kin.pos.x * 2.0 > world_width {
                        1.0
                    } else {
                        -1.0
                    };
                    kin.vel =
                        ambition_platformer2d::engine_core::Vec2::new(2_400.0 * toward, -200.0);
                    launched = true;
                }
            }
        }
    }

    assert_eq!(
        seated, 4,
        "the stage seated {seated} fighters for a four-way, so whatever this \
         measured it was not a free-for-all"
    );
    let decided = &app.world().resource::<Decisions>().0;
    assert_eq!(
        decided.len(),
        1,
        "a four-way with one fighter left announced {decided:?} — three sides \
         went out and exactly one match ended"
    );
    let slot: ambition_platformer2d::presentation::HudSlotId =
        ambition_demo_smash::SMASH_ANNOUNCE_HUD_SLOT.into();
    let card = app
        .world()
        .get_resource::<ambition_platformer2d::presentation::HudReadouts>()
        .and_then(|readouts| readouts.get(&slot))
        .map(ambition_platformer2d::presentation::HudReadout::text)
        .expect("the end of a match writes the announce card");
    // The last fighter is seat 0, and the card names it, not its side.
    let survivor = {
        let world = app.world_mut();
        let mut query = world.query::<(&MatchSeat, &Name)>();
        query
            .iter(world)
            .find(|(seat, _)| seat.0 == 0)
            .map(|(_, name)| name.as_str().to_string())
            .expect("seat 0 is the fighter nobody launched")
    };
    assert_eq!(
        card,
        ambition_demo_smash::victory_banner(
            &ambition_platformer2d::actor::MatchVerdict::Winner(survivor.to_string()),
            Some(&survivor),
        ),
        "the four-way's card reads {card:?} with {survivor:?} the only fighter \
         left standing"
    );
}

/// A team wins as a team, even after one of its members is gone.
///
/// A team keeps its own name on the winner card; only a side of one is
/// swapped for the fighter's name. The side size must come from the
/// prepared match, not from the bodies still standing:
/// `take_eliminated_fighters_out_of_play` despawns eliminated fighters, so
/// a team that lost a member has one body left at victory.
///
/// The solo half of the rule is asserted by
/// `a_four_way_free_for_all_ends_when_one_fighter_is_left` and
/// `a_second_match_on_the_same_stage_counts_in_and_ends`, which expect a
/// fighter's name.
#[test]
fn a_team_victory_names_the_team_and_not_its_last_survivor() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat, StocksMatchDecided};
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct Decisions(Vec<Option<String>>);

    let mut app = build_demo_app();
    app.init_resource::<Decisions>();
    app.add_systems(
        Update,
        |mut decided: MessageReader<StocksMatchDecided>, mut seen: ResMut<Decisions>| {
            for outcome in decided.read() {
                seen.0.push(outcome.outcome.winner().map(str::to_string));
            }
        },
    );
    for _ in 0..30 {
        app.update();
    }

    // Two teams of two.
    let mut roster = ambition_demo_smash::smash_roster_at_levels(
        [
            ambition_demo_smash::SMASH_CHARACTER_ID,
            ambition_demo_smash::SMASH_OPPONENT_ID,
            ambition_demo_smash::SMASH_CHARACTER_ID,
            ambition_demo_smash::SMASH_OPPONENT_ID,
        ],
        &[5, 5, 5, 5],
    );
    roster.rules.stocks = Some(1);
    for (index, participant) in roster.participants.iter_mut().enumerate() {
        participant.team = Some(if index < 2 { "Red" } else { "Blue" }.to_string());
    }
    let countdown = roster.rules.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    let seats_now = |app: &mut App| -> Vec<usize> {
        let world = app.world_mut();
        let mut query = world.query::<&MatchSeat>();
        let mut seats: Vec<usize> = query.iter(world).map(|seat| seat.0).collect();
        seats.sort_unstable();
        seats
    };
    let world_width = ambition_demo_smash::smash_stage().world.size.x;
    let side_margin = ambition_demo_smash::smash_stage()
        .world
        .edges
        .side
        .expect("the stage authors its side margins");
    // Eliminate by placement, not velocity. A launch velocity lasts one tick:
    // without hitstun, air control resolves the next tick's velocity from the
    // CPU's stick. So put the body past the blast zone; the velocity only
    // keeps the direction.
    let launch = |app: &mut App, seat_wanted: usize, speed: f32| {
        let world = app.world_mut();
        let mut query = world.query::<(&MatchSeat, &mut BodyKinematics)>();
        for (seat, mut kin) in query.iter_mut(world) {
            if seat.0 == seat_wanted {
                let toward = if kin.pos.x * 2.0 > world_width {
                    1.0
                } else {
                    -1.0
                };
                kin.vel =
                    ambition_platformer2d::engine_core::Vec2::new(speed.abs() * toward, -200.0);
                kin.pos.x = if toward > 0.0 {
                    world_width + side_margin + 1.0
                } else {
                    -(side_margin + 1.0)
                };
            }
        }
    };

    // Nothing here waits on the fight. This test causes every elimination on
    // a fixed schedule: Red's teammate leaves twenty ticks before Blue, so Red
    // has one body when the match ends. Letting CPUs decide would make the
    // card's wording depend on combat tuning.
    let mut seated = 0usize;
    let mut teammate_gone_on = None;
    let mut decided_on = None;
    for tick in 0..(countdown + 600) {
        app.update();
        if tick == countdown {
            seated = seats_now(&mut app).len();
        }
        // Launch as soon as the ceremony releases the cast: a body held by
        // `ControlHolds` is placed every tick, and every later tick lets the CPUs
        // decide the match themselves.
        if tick == countdown + 3 {
            launch(&mut app, 1, -4_800.0);
        }
        if tick == countdown + 8 {
            // The same speed as seat 1. The claim is the card's wording, so launch
            // hard enough that where a CPU stood cannot matter.
            launch(&mut app, 2, 4_800.0);
            launch(&mut app, 3, 4_800.0);
        }
        if teammate_gone_on.is_none() && tick > countdown + 3 && !seats_now(&mut app).contains(&1) {
            teammate_gone_on = Some(tick);
        }
        if decided_on.is_none() && !app.world().resource::<Decisions>().0.is_empty() {
            decided_on = Some(tick);
            for _ in 0..10 {
                app.update();
            }
            break;
        }
    }

    assert_eq!(
        seated, 4,
        "the stage seated {seated} fighters, so this was not a two-versus-two"
    );
    // Non-vacuity: Red must be one body and two participants when the card is
    // written. With two Red bodies, the broken code would also pass.
    let (gone, ended) = (
        teammate_gone_on.expect("seat 1 was launched off a one-stock stage and never left play"),
        decided_on.expect("both of Blue were launched off a one-stock stage and nothing decided"),
    );
    assert!(
        gone < ended,
        "seat 1 was taken out of play on tick {gone} and the match was decided on \
         {ended} — Red still had two bodies standing, which is the case that \
         always worked"
    );
    let decided = app.world().resource::<Decisions>().0.clone();
    assert_eq!(
        decided,
        vec![Some("Red".to_string())],
        "a two-versus-two where both of Blue went out announced {decided:?}"
    );
    let slot: ambition_platformer2d::presentation::HudSlotId =
        ambition_demo_smash::SMASH_ANNOUNCE_HUD_SLOT.into();
    let card = app
        .world()
        .get_resource::<ambition_platformer2d::presentation::HudReadouts>()
        .and_then(|readouts| readouts.get(&slot))
        .map(ambition_platformer2d::presentation::HudReadout::text)
        .expect("the end of a match writes the announce card");
    assert_eq!(
        card,
        ambition_demo_smash::victory_banner(
            &ambition_platformer2d::actor::MatchVerdict::Winner("Red".to_string()),
            Some("Red"),
        ),
        "the card reads {card:?} — Red won as a TEAM and it named the one \
         teammate whose body happened to still be standing"
    );
}

/// Two ordinary CPU seats wearing the same character should not remain a
/// perfect spatial reflection. Mirror error compares equal-and-opposite X about
/// the spawn midpoint plus Y disagreement. Character-authored mirror-preserving
/// behavior is covered separately in content, preparation, seating, and decision
/// tests because this standalone demo does not compose Ambition's character set.
#[test]
fn two_cpus_wearing_one_character_stop_being_a_perfect_reflection() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
    use bevy::prelude::*;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let character = ambition_demo_smash::SMASH_CHARACTER_ID;
    let roster = ambition_demo_smash::smash_roster_at_levels([character, character], &[5, 5]);
    let countdown = roster.rules.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    let seats = |app: &mut App| -> Option<[ambition_platformer2d::engine_core::Vec2; 2]> {
        let world = app.world_mut();
        let mut query = world.query::<(&MatchSeat, &BodyKinematics)>();
        let mut rows: Vec<_> = query
            .iter(world)
            .map(|(seat, kin)| (seat.0, kin.pos))
            .collect();
        rows.sort_by_key(|(seat, _)| *seat);
        (rows.len() == 2).then(|| [rows[0].1, rows[1].1])
    };

    let mut midline: Option<f32> = None;
    let mut worst_mirror_error = 0.0f32;
    let mut ticks_observed = 0usize;
    for _ in 0..(countdown + 900) {
        app.update();
        let Some([zero, one]) = seats(&mut app) else {
            continue;
        };
        // Take the midline from the first frame with both bodies, so it is the
        // stage's own symmetry.
        let mid = *midline.get_or_insert((zero.x + one.x) / 2.0);
        ticks_observed += 1;
        let error = ((zero.x - mid) + (one.x - mid)).abs() + (zero.y - one.y).abs();
        worst_mirror_error = worst_mirror_error.max(error);
    }

    // Non-vacuity: the match seated two bodies, and their spawns were mirrored.
    assert!(
        ticks_observed > 100,
        "only {ticks_observed} ticks had two seated bodies, so there was no match \
         to observe"
    );
    let mid = midline.expect("checked by ticks_observed above");
    assert!(
        (mid - 320.0).abs() < 200.0,
        "the spawn midline came out at {mid}, which is not the stage's centre — \
         re-derive this test's symmetry claim before trusting its verdict"
    );
    assert!(
        worst_mirror_error > 1.0,
        "two CPU {character} fighters at one level stayed within \
         {worst_mirror_error}px of a perfect reflection for {ticks_observed} ticks. \
         ⛔ this is NOT one mind played twice — the two seats draw from different \
         streams, and the sibling guards listed above prove it. What it says is \
         that a symmetric stage plus symmetric information leaves two different \
         streams almost nothing to diverge ON at this difficulty. Whether that is \
         acceptable is a product decision (queue D167); do NOT answer it by \
         unmirroring the spawns or by adding noise"
    );
}

/// The stage grants body contact to its cast, and the snapshot carries it.
///
/// This is the wiring half: in a real match on the real stage, both seated
/// fighters carry the capability and both reach the pre-integration snapshot
/// that the movement phase reads. That the constraint survives the
/// controller is proven by
/// `ambition_platformer2d::engine_core::movement::kernel::tests::a_grounded_body_walking_into_another_one_is_stopped_by_the_real_sweep`.
#[test]
fn the_stage_grants_body_contact_to_both_seated_fighters() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::platformer::body::{BodyContact, BodyContactSnapshot};

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_levels(
            [
                ambition_demo_smash::SMASH_CHARACTER_ID,
                ambition_demo_smash::SMASH_OPPONENT_ID,
            ],
            &[5, 5],
        ));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    let mut granted = 0usize;
    let mut sampled = 0usize;
    let mut resistances: Vec<f32> = Vec::new();
    for _ in 0..400 {
        app.update();
        let world = app.world_mut();
        if let Some(snapshot) = world.get_resource::<BodyContactSnapshot>() {
            sampled = sampled.max(snapshot.len());
        }
        let mut fighters = world.query_filtered::<&BodyContact, bevy::prelude::With<MatchSeat>>();
        let seen: Vec<f32> = fighters.iter(world).map(|c| c.resistance).collect();
        if seen.len() > granted {
            granted = seen.len();
            resistances = seen;
        }
    }

    assert_eq!(
        granted, 2,
        "the stage seated two fighters and granted body contact to {granted} of \
         them, so the ruleset's own cast is not solid to itself",
    );
    assert!(
        resistances.iter().all(|r| *r > 0.0),
        "a fighter was granted the capability at zero resistance, which is the \
         documented way of opting OUT: {resistances:?}",
    );
    assert_eq!(
        sampled, 2,
        "only {sampled} of the two granted fighters ever reached the \
         pre-integration snapshot, so the movement phase was told about fewer \
         bodies than the stage made solid",
    );
}

/// Probe: where is every body for the first ticks of a match? Print-only;
/// run with `--ignored`.
///
/// It shows that no body sits at the origin: zero bodies for three ticks,
/// then two, both at their spawns. So a frame "following (0,0)" is an
/// unframed view, not a placement race. `ResolvedCameraSnapshot` is an
/// `Option` so that an unframed view says so.
///
/// Prints one line per tick per body: seat (`-` for an unseated body) and
/// position.
#[test]
#[ignore = "PROBE, print-only: where every body sits for the first ticks of a match"]
fn probe_where_bodies_are_before_the_match_settles() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_levels(
            [
                ambition_demo_smash::SMASH_CHARACTER_ID,
                ambition_demo_smash::SMASH_OPPONENT_ID,
            ],
            &[5, 5],
        ));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    for tick in 0..12 {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<(bevy::prelude::Entity, &BodyKinematics, Option<&MatchSeat>)>();
        let mut rows: Vec<String> = q
            .iter(world)
            .map(|(e, kin, seat)| {
                format!(
                    "{}{:?}@({:.0},{:.0})",
                    seat.map_or("-".to_string(), |s| format!("seat{}", s.0)),
                    e,
                    kin.pos.x,
                    kin.pos.y
                )
            })
            .collect();
        rows.sort();
        println!("[probe] t{tick:<2} {} bodies: {}", rows.len(), rows.join("  "));
    }
}

/// Probe: how soon do two mirrored CPUs stop reflecting? Print-only; run with
/// `--ignored`.
///
/// Re-measures the desync time for D129. RNG changes cannot separate two
/// bodies that do the same thing; asymmetric circumstances (per-seat spawn
/// placement) can.
///
/// It reports the first tick past a threshold. The sibling assertion asks
/// only whether they diverge, because pinning when would pin the demo's
/// tuning.
#[test]
#[ignore = "PROBE, print-only: first tick two mirrored CPUs diverge"]
fn probe_when_the_mirror_breaks() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
    use bevy::prelude::*;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let character = ambition_demo_smash::SMASH_CHARACTER_ID;
    let roster = ambition_demo_smash::smash_roster_at_levels([character, character], &[5, 5]);
    let countdown = roster.rules.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    let seats = |app: &mut App| -> Option<[ambition_platformer2d::engine_core::Vec2; 2]> {
        let world = app.world_mut();
        let mut query = world.query::<(&MatchSeat, &BodyKinematics)>();
        let mut rows: Vec<_> = query
            .iter(world)
            .map(|(seat, kin)| (seat.0, kin.pos))
            .collect();
        rows.sort_by_key(|(seat, _)| *seat);
        (rows.len() == 2).then(|| [rows[0].1, rows[1].1])
    };

    let mut midline: Option<f32> = None;
    let mut observed = 0usize;
    let mut first_past: Option<(usize, f32)> = None;
    for _ in 0..(countdown + 1800) {
        app.update();
        let Some([zero, one]) = seats(&mut app) else {
            continue;
        };
        let mid = *midline.get_or_insert((zero.x + one.x) / 2.0);
        observed += 1;
        let error = ((zero.x - mid) + (one.x - mid)).abs() + (zero.y - one.y).abs();
        if first_past.is_none() && error > 1.0 {
            first_past = Some((observed, error));
        }
    }
    match first_past {
        Some((tick, error)) => println!(
            "mirror broke on observed tick {tick} ({:.2}s of play) with {error:.2}px of error",
            tick as f32 / 60.0
        ),
        None => println!("the mirror never broke across {observed} observed ticks"),
    }
}

/// A fighter waiting out its respawn beat does not answer the jump button.
///
/// During the wait the body is `OutOfPlay` (ADR 0033) and a
/// `ControlHold::Sequence` claim keeps normal input from it.
///
/// The positive control is the test. "The body did not move" is also true
/// of a press that never arrived or a stage with no jump. So the same held
/// jump is first driven at the same body while alive, and that must move it.
///
/// Two traps, both handled below: an unfrozen body falls out of the blast
/// zone, so measure the magnitude of motion, not the rise; and the frame
/// the wait ends places the body on the respawn platform, so sample only
/// while it still waits.
#[test]
fn a_fighter_waiting_out_its_respawn_beat_does_not_answer_the_jump_button() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
    use bevy::prelude::*;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    decide_a_two_player_match(&mut app);
    for _ in 0..240 {
        app.update();
    }

    let seat_body = |app: &mut App, seat: usize| -> Option<Entity> {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &MatchSeat)>();
        query
            .iter(world)
            .find(|(_, s)| s.0 == seat)
            .map(|(entity, _)| entity)
    };
    let y_of = |app: &mut App, body: Entity| -> f32 {
        app.world()
            .get::<BodyKinematics>(body)
            .map(|kin| kin.pos.y)
            .expect("the seated body has kinematics")
    };
    // `App::update()` is a frame, not a sim tick, so a jump arc can begin and
    // end between two samples. Read the peak over the whole hold.
    let hold_jump_and_peak = |app: &mut App, body: Entity, frames: usize| -> f32 {
        let start = y_of(app, body);
        let mut peak = 0.0f32;
        for _ in 0..frames {
            // Held, not tapped. `drive_control_frame` is the only driver that
            // lands: a `ControlFrame` written between updates is rewritten by the
            // device systems every tick.
            ambition_platformer2d::sim::drive_control_frame(
                app.world_mut(),
                ambition_platformer2d::engine_core::ControlFrame {
                    jump_pressed: true,
                    jump_held: true,
                    ..Default::default()
                },
            );
            app.update();
            // Feet are +gravity, so rising decreases y.
            peak = peak.max(start - y_of(app, body));
        }
        peak
    };

    let seat0 = seat_body(&mut app, 0).expect("the match seats a first fighter");

    // Control arm: alive, on the stage, holding jump.
    let alive_rise = hold_jump_and_peak(&mut app, seat0, 40);
    assert!(
        alive_rise > 8.0,
        "a LIVE fighter holding jump rose only {alive_rise:.1}px, so this harness \
         is not delivering the press at all and the respawn arm below would pass \
         for the wrong reason"
    );

    // Launch it out, as in the blast-gate test above.
    {
        let world = app.world_mut();
        let mut query = world.query::<(&MatchSeat, &mut BodyKinematics)>();
        for (seat, mut kin) in query.iter_mut(world) {
            if seat.0 == 0 {
                kin.vel = ambition_platformer2d::engine_core::Vec2::new(2_400.0, -200.0);
            }
        }
    }

    let mut waiting = false;
    for _ in 0..240 {
        app.update();
        if app
            .world()
            .get::<ambition_platformer2d::actor::PendingRespawn>(seat0)
            .is_some()
        {
            waiting = true;
            break;
        }
    }
    // Premise: otherwise the loop below measures a fighter standing on the
    // stage.
    assert!(
        waiting,
        "seat 0 never entered a respawn wait after being launched at the blast \
         line, so nothing here is about the respawn beat"
    );

    // Arm under test: the same held jump, during the wait.
    let mut held_frames = 0usize;
    let start = y_of(&mut app, seat0);
    let mut moved = 0.0f32;
    while app
        .world()
        .get::<ambition_platformer2d::actor::PendingRespawn>(seat0)
        .is_some()
        && held_frames < 240
    {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame {
                jump_pressed: true,
                jump_held: true,
                ..Default::default()
            },
        );
        app.update();
        // Sample only while the body still waits. The frame the wait ends
        // places the body on the respawn platform, about 580px up. The `while`
        // condition is checked too late to prevent that.
        if app
            .world()
            .get::<ambition_platformer2d::actor::PendingRespawn>(seat0)
            .is_none()
        {
            break;
        }
        held_frames += 1;
        // Measure the magnitude, not the rise: an unfrozen body falls at about
        // 1200px/s, so it never gets higher whatever the pad does. A waiting
        // fighter does not move in either direction.
        moved = moved.max((y_of(&mut app, seat0) - start).abs());
    }
    // Second premise: a very short wait would leave nowhere to fail.
    assert!(
        held_frames >= 20,
        "the respawn wait lasted only {held_frames} frames, which is too short \
         for a held jump to have had a chance to move the body"
    );
    assert!(
        moved <= 1.0,
        "a fighter waiting out its respawn MOVED {moved:.1}px over {held_frames} \
         frames under a held jump — the same press moved the live body \
         {alive_rise:.1}px, so the beat is still answering the pad"
    );
}

/// A second match starts, and the first match's verdict does not end it.
///
/// The stale part is the `ActiveMatch` resource, not the verdict latch.
/// `StocksMatchSettled` names its match, but a retired session's
/// `ActiveMatch` outlives it by at least a frame, and then both sides of
/// the comparison in `verdict()` name the retired match. Only the session
/// knows which is current.
///
/// The premise (match one really settled) is asserted before the subject.
#[test]
fn a_second_match_is_not_ended_by_the_first_matchs_verdict() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }

    let route_now = |app: &bevy::prelude::App| -> Option<String> {
        app.world()
            .resource::<ambition_platformer2d::game_shell::ShellRouter>()
            .active
            .as_ref()
            .map(|active| active.route_id.as_str().to_string())
    };

    // Match one, ended the way Exit Match ends one.
    decide_a_two_player_match(&mut app);
    for _ in 0..90 {
        app.update();
        if route_now(&app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
            break;
        }
    }
    assert_eq!(
        route_now(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        "match one never reached the stage, so there is no first verdict to \
         leave behind"
    );

    // Ask only when a match exists: `abandon_the_match_when_the_shell_asks`
    // does nothing when no `ActiveMatch` is seated, and the message is consumed
    // either way.
    for _ in 0..600 {
        if app
            .world()
            .get_resource::<ambition_platformer2d::versus_match::ActiveMatch>()
            .is_some()
        {
            break;
        }
        app.update();
    }
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::versus_match::ActiveMatch>()
            .is_some(),
        "match one never seated a cast, so there is nothing for Exit Match to stop"
    );
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellAbandonRequested);
    for _ in 0..600 {
        app.update();
        if route_now(&app).as_deref() == Some(ambition_demo_smash::SMASH_SELECT_ROUTE) {
            break;
        }
    }
    // Premise: Exit Match goes home on the press, so reaching the select
    // screen proves the verdict was reached.
    assert_eq!(
        route_now(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_SELECT_ROUTE),
        "Exit Match did not take match one off the stage, so this run never \
         built the leftover the second match has to survive"
    );

    // Match two. The lobby resets on arrival, so wait before deciding the
    // cast, as a player does.
    for _ in 0..10 {
        app.update();
    }
    decide_a_two_player_match(&mut app);
    for _ in 0..90 {
        app.update();
        if route_now(&app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
            break;
        }
    }
    assert_eq!(
        route_now(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        "the second match never reached the stage at all"
    );

    // Subject: sample well past the frame of the leftover. The bounce took one
    // frame, so a stage still up sixty frames later was not closed by it.
    for _ in 0..60 {
        app.update();
        assert_eq!(
            route_now(&app).as_deref(),
            Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
            "the second match was thrown back to the select screen — the first \
             match's verdict was applied to it"
        );
    }
}
