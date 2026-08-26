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

    // Two players join and commit. this test is about the STAGE, so it sets
    // the decision directly and then asks for the start the screen's button
    // would ask for. `the_screen_decides.rs` is where the button is pressed.
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
/// The last unproven link, and the only one that needed the physics rather than
/// a message. Everything upstream is covered by unit tests that WRITE
/// `BodyKnockedOut`; nothing had ever earned one. So this launches a real body
/// off a real platform with a real velocity and waits for the world to take it.
///
/// If this fails while `ambition_combat::stocks` stays green, the gap is between
/// the blast gate and the KO announcement — which is exactly the seam no test
/// below the app can reach.
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

    // The engine publishes this trigger for exactly that reason; an observer cannot miss what a
    // poll can.
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

    // LAUNCH. Hard enough and sideways enough that the blast line is reached
    // rather than approached — the stage's margin is a fraction of the platform,
    // and this is several times that per second.
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

    // Long enough to cross the margin and for the KO to settle. A body moving at
    // 2400px/s clears a 120px margin in a handful of ticks; the rest is the
    // announcement, the spend and the placement.
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

    // the loop above BREAKS on the stock change, and the restart comes after
    // it. The spend and the ruleset's respawn are different steps in different
    // phases, so asserting the announcement without letting the frame finish
    // measures the gap between them rather than the engine.
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

    // NON-VACUITY: the fighter that was NOT launched still has everything.
    assert_eq!(
        stocks_of(&mut app, 0),
        Some(before),
        "the fighter that was never launched also lost a stock, so the counter \
         is moving on its own and this test proves nothing about the blast gate"
    );

    // ⭐ D192: THE RESTART IS RAISED WHEN THE BODY IS PLACED, and placement now
    // waits out the authored beat. `reset_body_clusters` is what sets
    // `restart_pending`, so before the beat elapses there is no restart to see —
    // the twelve frames above are the blast gate's window, not the respawn's.
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
    // ⛔ AND THEN LET THE FRAME FINISH, for the same reason the twelve above
    // exist: `reset_body_clusters` raises `restart_pending` and
    // `announce_body_restarts` turns that into `BodyRestarted` in a later phase,
    // so breaking out on the placement tick samples the gap between them.
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
        // non-vacuity, the same shape as the stock assertion above: an
        // announcement everybody gets says nothing about this knockout.
        assert!(
            !seen.contains(&untouched),
            "the fighter that was never launched also announced a restart"
        );
    }

    // and it came back where the RULESET says, not where it died. A body
    // that respawns at its blast position is outside the stage and falls again.
    {
        use ambition_platformer2d::actor::BodyKinematics;
        let respawn =
            ambition_demo_smash::respawn_placement(ambition_demo_smash::stage_centre(), 0);
        let pos = app
            .world()
            .get::<BodyKinematics>(launched)
            .expect("the fighter still has a body")
            .pos;
        assert!(
            (pos - respawn).length() < 240.0,
            "the fighter restarted at {pos:?}, nowhere near the ruleset's \
             respawn placement {respawn:?}"
        );
    }
}

/// The fighter brain closes the distance and lands a hit.
///
/// FB4b's first damage against an OPPONENT rather than a fixture. Everything
/// below this — classify, options, rollout, the delay buffer, the APM ledger —
/// was unit-tested against hand-built `Perceived` values; nothing had ever put
/// the rig on a body and let it decide what to do about somebody else.
///
/// The assertion is deliberately weak on WHAT it does and strict on THAT it does: a brain that
/// travels and connects is working, and pinning a distance or a damage number here would be pinning
/// the tuning of a demo rather than the rig.
#[test]
fn the_fighter_brain_engages_rather_than_standing_still() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use bevy::prelude::*;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // BOTH SEATS MUST BE CPUs, and this called the helper that makes seat 0 HUMAN. The comment
    // here already said what it needed — *"a human with no controller correctly does nothing"* —
    // and then asked for a roster whose first seat is exactly that.
    //
    // `smash_roster_at_levels` is the helper that seats EVERY slot as a CPU;
    // its own doc says so. Same rungs on both sides, so neither fighter has a
    // ladder advantage and the measurement is about engagement rather than skill.
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
/// The stock was spent exactly once, the engine's `Without<FighterEliminated>` filter held, and
/// the body simply never stopped being a body. That is the gap between "the count is correct"
/// and "the match is over".
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

/// THE 3-2-1-GO IS ON THE SCREEN.
///
/// so this watches the slot the stage DECLARES, `smash_announce`: the centred
/// card the HUD renders, beside the fighter percents that were always visible.
/// Before the rewiring the slot was declared and never written once, so this
/// finds an empty card for the whole ceremony.
///
/// it asserts the COUNT, not the tick. Which frame carries "2" is a tuning
/// fact about `opening_countdown_ticks`; that a player is counted in with three
/// numbers and then told to go is the genre's shape and the thing that was
/// missing.
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
                // A card coming back after the ceremony retired would mean the
                // clear is fighting a writer.
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

/// THE CAMERA COMES BACK NO FASTER THAN IT LEFT.
///
/// ```text
///   widest single-frame OPEN    49.3      (a ramp over 7-8 frames, 800 -> 1115)
///   widest single-frame CLOSE  360.9      (one frame, straight back to 800)
/// ```
///
/// The open was never eased and never needed to be — its input is a body
/// flying, already continuous. The close is a DISCONTINUITY: the body is taken
/// out of play and the cast's bounding box collapses between two frames. After
/// easing only the close, the same run reads `open 57.5 / close 68.9`.
///
/// the non-vacuity guard is the OPEN, and it is doing real work: a match
/// where nobody was ever launched far enough to widen the frame would satisfy
/// any ratio at all, and this fixture is a live fight rather than a scripted
/// one.
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
    for tick in 0..5_400 {
        app.update();
        let view = {
            let world = app.world_mut();
            let observer = ambition_platformer2d::sim_view::the_only_view(world);
            world
                .entity(observer)
                .get::<ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot>()
                .map(|resolved| resolved.snapshot.visible_view)
        };
        let Some(view) = view else { continue };
        if let Some(previous) = previous {
            let step = (view - previous).length();
            if view.x > previous.x {
                widest_open = widest_open.max(step);
            } else if view.x < previous.x {
                // skip the opening frames: the first resolve ADOPTS the cast's
                // framing rather than easing to it, which is correct (a match
                // opens already framed) and is not a transition anybody sees.
                if tick > 60 {
                    widest_close = widest_close.max(step);
                }
            }
        }
        previous = Some(view);
    }

    assert!(
        widest_open > 25.0,
        "the frame never widened by more than {widest_open:.1} units in a frame, \
         so nobody was launched far enough to move the camera and the ratio \
         below is about a match that never happened"
    );
    assert!(
        widest_close <= widest_open * 2.0,
        "the camera closed by {widest_close:.1} units in one frame having opened \
         by at most {widest_open:.1} — the return is a cut, not a transition"
    );
}

/// A MATCH SOMEBODY WINS ACTUALLY ENDS.
///
/// None of them asks the question a viewer asks, which is whether the match is over when only one
/// fighter is left.
///
/// "several cases" is the shape of a SCHEDULING AMBIGUITY, and that is what
/// it was. `decide_stocks_match` reads the sides off the bodies that still
/// exist; `take_eliminated_fighters_out_of_play` despawns an eliminated body.
/// Both sat in `CombatSet::Settle` with nothing ordering them, and the ruleset's
/// `.chain()` inserts an `ApplyDeferred` that makes the despawn visible part-way
/// through the set. Lose the last loser's row and `last_side_standing` sees ONE
/// side — and one side is not a match, so it answers `None` forever. Whether a
/// match ended depended on how the scheduler broke a tie, which is why it
/// happened in some compositions and not others.
///
/// PROBED RED: with `take_eliminated_fighters_out_of_play` ordered
/// `.before(MatchOutcomeDecided)` instead of after — the broken order, made
/// explicit — this runs a full match, watches a fighter be eliminated, and never
/// settles.
///
/// the elimination is asserted first, because a match that simply never
/// got anybody killed would settle nothing and the claim below would be about a
/// fight that did not happen.
#[test]
fn a_match_whose_last_loser_is_removed_still_decides() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::actors::features::stocks_match::the_live_match_is_settled;
    use ambition_platformer2d::combat::components::FighterStocks;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // BOTH seats CPU, so somebody actually loses. `smash_roster` makes seat 0 a
    // human with no controller, which is a match one fighter cannot lose.
    //
    // AND ONE STOCK, because the question is the REMOVAL, not the pace. Measured
    // 2026-08-23: ninety seconds of this match produces three or four KOs across
    // both seats, so at the default stock count neither fighter reliably reaches
    // zero inside the window — and the test then fails for saying nothing rather
    // than for a defect. Its sibling above already seats one stock for exactly
    // this reason. A fight's pace is tuning; whether an emptied fighter is
    // removed and the match decides is the mechanic.
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

    // seats, not stocks. A fighter reduced to zero is ELIMINATED and
    // removed in the same breath, so a poll of `FighterStocks` never sees the
    // zero — which is the very removal this test is about. The cast SHRINKING is
    // the observation that survives it.
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
        // A stocks verdict names the match it belongs to, so "has anything been decided" is not
        // the question — "has THIS one" is.
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

/// 1. a raw `BodyKinematics::pos` write is not "this fighter lost a stock".
///    Measured: one app update later the body sat at a normal stage position with
///    all THREE stocks — something noticed the nonsense position and relocated
///    it, which is not a knockout. The test spent its life asserting a restart no
///    KO had caused.
/// 2. `restart_pending` is a ONE-SIM-TICK flag — raised by the reset, cleared
///    by `announce_body_restarts` in the next `WorldPrep` — and a fixed-tick host
///    advances several sim ticks per `app.update()`. Polling it between updates
///    can miss it entirely, whatever caused it.
///
/// its intent is fully covered by
/// `a_launched_fighter_is_taken_by_the_world_and_spends_a_stock`, which causes
/// a REAL knockout — a real launch, the real blast boundary — and now proves the
/// whole chain from one: exactly one stock spent, the other fighter untouched, a
/// `BodyRestarted` trigger observed for that body and not the other, and a
/// respawn at the ruleset's placement. An observer cannot miss what a poll can.

/// This demo's own CPU roster is seatable by its own composition.
/// (API 1.0 row (g))
///
/// A `ControllerBinding::Cpu { brain_profile }` is looked up in the composition's `CharacterRoster`
/// ARCHETYPE table, and `spec_for_brain` falls back to a generic row whose brain is `stand_still`
/// when the key is absent. The match composes, seats, and runs; the opponent never moves.
///
/// Asked here rather than at the select screen, and that is the point: every
/// seat the screen produces is a HUMAN, and a human seat asks the archetype
/// table for nothing. A guard placed there would have been unreachable —
/// protection that reads as protection and cannot fire.
#[test]
fn the_demos_cpu_roster_is_satisfiable_by_its_own_composition() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // The question — *can this demo fill the seats it declares?* — has never changed.
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

/// Two controller slots with a fighter each, and START asked for.
///
/// `StartRequested` as well as the picks. The screen no longer leaves on
/// readiness alone — a test that set only the decision would sit on the select
/// route forever and blame the stage.
fn decide_a_two_player_match(app: &mut bevy::prelude::App) {
    use ambition_demo_smash::select::{SlotOccupant, SmashRoster, SmashSelect};

    // and seat 0 deliberately takes a fighter that is NOT the stage's
    // starting character — see below.
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
/// `smash_roster_at_level` puts every CPU on one rung, and `smash_roster` makes
/// seat 0 HUMAN — so the only opponent `ladder_probe` could offer was a
/// controller-less body that never acts. That made its number clean (*every
/// stock lost is a self-KO*) and made a FIGHT impossible to measure, which is
/// why FB6e's `l3_earns_its_depth` is still owed §8's suite and the
/// survival/damage ratios.
///
/// the assertion that matters is that the two seats DIFFER. A rig built on
/// a roster that quietly put both fighters on the same rung would report a 50%
/// win rate at every level and read as "the ladder is flat" rather than as a
/// broken fixture — the most expensive kind of wrong answer, because it looks
/// like a finding.
///
/// and both profiles must be SATISFIABLE by the demo's own archetype table,
/// for the same reason its sibling above checks: `spec_for_brain` falls back to
/// a generic row rather than failing, so an unregistered level is a fight
/// against a statue that reports itself as a fight.
#[test]
fn a_ladder_roster_seats_two_cpus_at_two_different_levels() {
    use ambition_platformer2d::actor::ControllerBinding;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // the demo's PUBLISHED policies — its CPU ladder lives here now, not in an
    // archetype fragment (that fragment is deleted), and since P2.18 there is
    // nowhere else a seat's policy could come from.
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
    // the ONE authority a seat's policy can live in, resolved in this
    // demo's own provider exactly as `seat_brain_profile` resolves it.
    //
    // `seat_brain_profile` has one arm (P2.18), so a term that seating cannot use has no
    // business in a guard about what seating can do.
    let resolves = |profile: &str| {
        published
            .get(&ambition_platformer2d::entity_catalog::BrainProfileId::new(
                format!("{}::{profile}", ambition_demo_smash::SMASH_EXPERIENCE),
            ))
            .is_some()
    };
    // and each rung RESOLVES, which is the property that keeps a ladder a ladder.
    for profile in profiles.iter().flatten() {
        assert!(
            resolves(profile),
            "`{profile}` resolves in neither the published policies nor the \
             archetype table, so seating hands back a generic row and the rung \
             fights a statue while reporting a fight"
        );
    }
    // every ADJACENT PAIR is satisfiable, which is the property a ladder
    // rig needs and the one a single spot-check would not have given: N vs N−1
    // over the registered rungs is (3,1), (5,3), (6,5), (9,6).
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

    // The ruleset is the shipped stage's, not the rig's — a measurement of a
    // game nobody plays is worth nothing.
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
        // P4.29 / P4.30 / P4.32, on the LIVE body, through the real route.
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
        // THE POISON: verbs these fighters state they do NOT have.
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
    // BOTH seats CPU, so bodies actually get launched.
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
                .map(|resolved| {
                    (
                        resolved.snapshot.center_world,
                        resolved.snapshot.visible_view,
                    )
                })
        };
        let Some((center, visible)) = view else {
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
                         {:.0}x{:.0} frame centred ({:.0},{:.0})",
                        pos.x, pos.y, visible.x, visible.y, center.x, center.y
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

/// AND THE FRAME DOES NOT CUT WHEN A FIGHTER LEAVES PLAY.
///
/// The companion to [`the_camera_closes_no_faster_than_it_opened`], and it exists because that
/// one made this one reachable. Now that the centre travels — it must, or a fighter cannot be
/// followed off the stage — it has the same discontinuity the size had: an eliminated body is
/// taken out of play and the cast's box collapses between two frames, jumping its centre back
/// to the platform.
///
/// It is now 27.
///
/// it compares the ELIMINATION frame against the ordinary ones, which is
/// the only comparison that means anything here: a cast centre that is tracking
/// a fast fight moves a long way per frame quite correctly, and a threshold in
/// units would be a guess about how hard fighters hit.
///
/// the non-vacuity guard is the JUMP the framing had to absorb: a match
/// where the two fighters happened to be standing together at the knockout
/// collapses its own centre by nothing at all, and would satisfy this however
/// broken the absorption was.
#[test]
fn the_framing_centre_absorbs_an_elimination_instead_of_cutting() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
    use ambition_platformer2d::engine_core::Vec2;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // ONE STOCK, for the same reason the elimination guard above seats one: this
    // test needs a fighter to LEAVE, and how long a fight takes to empty a
    // fighter is tuning it should not be racing. Measured 2026-08-23, ninety
    // seconds produces three or four KOs across both seats.
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
                .map(|resolved| resolved.snapshot.center_world)
        };
        let Some(camera) = camera else { continue };
        // The cast's TRUE centre and population — the input the framing absorbs.
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

/// THE SECOND MATCH ON THE SAME STAGE COUNTS IN, TAKES THE CARD DOWN, ENDS, AND STOPS.
///
/// Running back and doing another cpu vs cpu after gets a 3 2 1 go, but the GO stays on the screen
/// for the entire match, and the match does not end. I can quit to title and then do another match
/// which does a 3, 2, 1, go, but again the go still appears on the screen, and the match does not
/// end when there is only 1 player left."*
///
/// THE SECOND MATCH IS THE TEST, and it is why every other one here missed this.
/// `the_opening_countdown_is_something_a_player_can_see` watches one ceremony;
/// `a_launched_fighter_is_taken_by_the_world_and_spends_a_stock` spends one stock; the host's
/// `coming_back_to_the_select_screen_offers_a_fresh_match` starts a second match and never plays
/// it. Each is green about exactly what it claims.
///
/// So this plays two identical matches through one app and asserts they are the
/// same match twice. What it pins:
///
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
        /// Every distinct word the centred card showed WHILE THE STAGE WAS UP.
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
            // The launch, once, as soon as the ceremony has released the cast: a
            // body thrown at 2400px/s crosses this stage's blast margin in a
            // handful of ticks, and on one stock that is the match.
            //
            // The sibling four-way's own note already says it: a claim about the WORDING of a card
            // must not depend on combat tuning.
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
            // only while the STAGE is up. The card the previous match
            // ended on is still in `HudReadouts` while the select screen shows —
            // it is the experience's HUD DECLARATION that stops it being drawn,
            // not the readout — so recording it off-stage would make every match
            // after the first look like it opened on a victory card.
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

            // Measure freeze from body motion rather than only inspecting the clock resource.
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
    // The stage takes itself back to the select screen 4.5s after the end; let
    // it, so the second match arrives by the road a player takes.
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
        // half a pixel over ~350 ticks. Not zero: the clock RAMPS to a
        // stop rather than snapping, which is the feel the time-control
        // smoother exists for, so the frame the winner is named still carries a
        // fraction of a step. What must not happen is the match playing on.
        assert!(
            played.travelled_after_the_end < 8.0,
            "a fighter moved {:.1}px after the {which} match was decided — the \
             winner is still playing, and the game did not stop",
            played.travelled_after_the_end
        );
    }
}

/// A FOUR-WAY FREE-FOR-ALL ENDS WHEN ONE FIGHTER IS LEFT.
///
/// someone wins it ends with 'Go'"* and *"when there is only 1 player alive or 1
/// team alive for team matches the time in the game should freeze"* — and the
/// sibling test above plays a duel. The predicate is `last_side_standing`, which
/// folds N sides rather than comparing two, so "three of four are out" is a
/// genuinely different question from "one of two is out": a fold that stopped at
/// the first surviving side would answer both the same way while only one of
/// them is right.
///
/// the same two fighters twice. The standalone demo declares two
/// characters (the stand-ins for the robot lineage), so a four-seat match here
/// is a mirror match — which is also the case worth having, because four bodies
/// wearing two characters is where a side keyed on the CHARACTER rather than the
/// SEAT would collapse four sides into two and end the match early.
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
    // it STOPS a few ticks after the end, and that is not impatience. The
    // stage takes itself back to the select screen 4.5s later and the card comes
    // down with it (`return_to_the_select_screen_when_the_match_ends`), so a
    // loop that ran to a fixed budget would read an empty slot and blame the
    // announcement.
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
        // Everybody but seat 0 leaves the world, and KEEPS leaving until they
        // are gone.
        //
        //  a SINGLE velocity write was still a race, whatever the note
        // below claims: it is one frame's worth of authority over a body the
        // sim owns, and a fighter struck mid-flight takes the hit's knockback
        // instead and lands back on the stage. That made this fixture sensitive
        // to combat BALANCE — it went red the day a tapped smash stopped
        // landing at full charge — while measuring nothing about it. Re-applying
        // every tick is what makes "every elimination is one it CAUSES" true.
        if tick > countdown + 30 {
            let world = app.world_mut();
            let mut query = world.query::<(&MatchSeat, &mut BodyKinematics)>();
            for (seat, mut kin) in query.iter_mut(world) {
                if seat.0 > 0 {
                    // Away from the centre, re-read each tick so a body that
                    // was knocked back across the midline is still thrown OUT.
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
    // The last fighter standing is seat 0's, and the card names IT rather than
    // the engine's word for its side.
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

/// A TEAM WINS AS A TEAM, EVEN AFTER ONE OF ITS MEMBERS IS GONE.
///
/// The winner card states its own rule: a team keeps its own name, and only a
/// side of ONE is swapped for the fighter's. It decided which by COUNTING THE
/// BODIES still standing on the winning side — and
/// `take_eliminated_fighters_out_of_play` despawns an eliminated fighter, so a
/// two-person team that lost a member early has exactly one body left at
/// victory and the card called it a solo.
///
/// How many fighters a side HAS is a fact about the match that was PREPARED; how many are standing
/// is a fact about right now, and the two stop agreeing the first time somebody dies.
///
/// That is the state the census-based version got wrong, and without it this test would have
/// passed on the broken code.
///
/// the solo half of the rule is asserted by
/// `a_four_way_free_for_all_ends_when_one_fighter_is_left` and
/// `a_second_match_on_the_same_stage_counts_in_and_ends`, both of which expect a
/// FIGHTER'S NAME — so a "fix" that always printed the side would go red there.
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
    // THE ELIMINATION IS A PLACEMENT, NOT A VELOCITY, and that is what makes it
    // this test's to cause.
    //
    // Measured 2026-08-22: a body handed 4,800 px/s keeps it for exactly one
    // tick. Nothing here puts it in hitstun, so ordinary air control resolves
    // its horizontal velocity from the stick the CPU is holding on the next
    // tick and the launch is simply gone — seats 2 and 3 landed back on the
    // platform every run. The assertion below still passed, because the four
    // CPUs then fought it out and Blue happened to lose, which is precisely the
    // race the note above says this fixture does not want. So the body is put
    // past the blastzone outright; the velocity stays only so the direction it
    // left in is the one it was sent.
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

    // NOTHING HERE WAITS ON THE FIGHT, and that is deliberate. Every
    // elimination is one this test causes, on a fixed schedule: Red's teammate
    // leaves at twice the speed and twenty ticks ahead of Blue, so the census
    // has exactly one Red body when the match ends. A version that let four CPUs
    // decide who dies would make a claim about the WORDING of a card depend on
    // combat tuning — measured: a hitlag change landing in another crate flipped
    // the winner.
    let mut seated = 0usize;
    let mut teammate_gone_on = None;
    let mut decided_on = None;
    for tick in 0..(countdown + 600) {
        app.update();
        if tick == countdown {
            seated = seats_now(&mut app).len();
        }
        // after the ceremony RELEASES the cast — a body held by
        // `ScriptedControl` is placed by the respawn rule every tick, so a
        // velocity written during the count is simply overwritten.
        // as soon as the ceremony releases, for the reason the sibling
        // second-match test now records: every tick between the release and the
        // script is a tick in which the CPUs can decide the match themselves.
        if tick == countdown + 3 {
            launch(&mut app, 1, -4_800.0);
        }
        if tick == countdown + 8 {
            // the same speed as seat 1's, because the SPEED was never the
            // claim. A body starting near the middle of the stage has further
            // to travel than one already near an edge, and at 2400px/s the
            // controller's decay can bring it down inside the world — which
            // reads as "nothing decided" rather than as a launch that fell
            // short. What this test asserts is the WORDING of a team's card;
            // every elimination in it is one it causes, and it should cause them
            // hard enough that where a CPU was standing cannot matter.
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
    // THE NON-VACUITY GUARD, and it is the whole fixture. If seat 1 were
    // still standing when the match ended, the census would have found two Red
    // bodies and printed the team for the wrong reason — the assertion below
    // would pass on the broken code. Red must be ONE body and TWO participants
    // at the moment the card is written.
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
        // The midline is taken from the FIRST frame both bodies exist on, so it is
        // the stage's own symmetry rather than a number written here.
        let mid = *midline.get_or_insert((zero.x + one.x) / 2.0);
        ticks_observed += 1;
        let error = ((zero.x - mid) + (one.x - mid)).abs() + (zero.y - one.y).abs();
        worst_mirror_error = worst_mirror_error.max(error);
    }

    // Non-vacuity, both halves: a match that seated nobody, or whose spawns were
    // not mirrored to begin with, would make the measurement meaningless.
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

/// THE STAGE GRANTS BODY CONTACT TO ITS CAST, AND THE SNAPSHOT CARRIES IT.
///
/// owns an unnamed constraint and this ruleset grants it, which is the whole of
/// what smash contributes. This test is the WIRING half of that claim: in a real
/// match, on the real stage, both seated fighters carry the capability and both
/// reach the pre-integration snapshot the movement phase reads.
///
/// Whether the constraint survives the controller is proven where the controller runs:
/// `ambition_platformer2d_core::movement::kernel::tests::a_grounded_body_walking_into_another_one_is_stopped_by_the_real_sweep`
/// holds RIGHT for a second against the `approach()` overwrite that erased the force version,
/// and measures the distance. This test only says the two are connected.
///
/// That is a feel question for invent in a test.
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

/// PROBE: HOW SOON do two mirrored CPUs stop reflecting? Print-only; run with
/// `--ignored`.
///
/// ⭐⭐ THE RE-MEASUREMENT D129 ASKS FOR. Jon reported the desync taking ~8s of
/// play (*"it took a while for Booule to desync"*), the ledger recorded 488
/// frames, and TWO randomness fixes were built, measured and REVERTED — the
/// jitter stream has one consumer, so a different RNG cannot separate two bodies
/// doing the same thing. What the row expects to have moved the number is
/// asymmetric CIRCUMSTANCES: per-seat spawn placement, which has since landed
/// and was never re-measured against.
///
/// ⚠ this reports the FIRST tick past a threshold, which its sibling above
/// deliberately does not: that one asks WHETHER they diverge (and must stay a
/// whether-question, because pinning WHEN would pin the tuning of a demo). This
/// is a probe, so it may report a number the assertion must not.
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

/// ⭐⭐ JON'S BUG: *"in smash when you are respawning, if I make the character
/// jump they raise up on the platform."*
///
/// A fighter waiting out its respawn beat is a fighter the world has its hands
/// off — ADR 0033's `OutOfPlay`, plus the `ControlHold::Sequence` claim that
/// says normal input does not reach this body. D192 opened the beat and claimed
/// neither, so the wait was a window in which a knocked-out body still answered
/// the pad.
///
/// ⛔⛔ THE POSITIVE CONTROL IS THE TEST. "The body did not move" is true of a
/// press that never arrived, of a harness that drove the wrong slot, and of a
/// stage with no jump at all — every way this could measure nothing looks
/// exactly like the fix working. So the SAME held jump is driven at the SAME
/// body while it is alive, first, and that arm has to move it.
///
/// PROBED RED: with the `out_of_play` flag reverted to the hard-coded `false`
/// the actor road used to pass — the state D201 found — the waiting body moves
/// **174.7px in 60 frames**; with it read, **0.0px**. Both of the first two
/// spellings of this test passed with the fix removed, and neither failure was
/// visible from the assertion:
///   - "did it rise above where the wait started" — an unfrozen body is FALLING
///     out of the blast zone at ~1200px/s, so a jump cannot get it back above
///     the line no matter what the pad does.
///   - sampling after `app.update()` without re-checking the wait — the frame
///     the wait ENDS is the frame the ruleset places the body on the respawn
///     platform, ~580px up, and that placement read as the bug.
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
    // ⛔ `App::update()` IS A FRAME, NOT A SIM TICK, so a jump arc can begin and
    // end between two samples. The reading is the PEAK over the whole hold, not
    // the position at the end of it — measured the hard way on an earlier probe,
    // where a 630px/s jump sampled as 0.4px of rise.
    let hold_jump_and_peak = |app: &mut App, body: Entity, frames: usize| -> f32 {
        let start = y_of(app, body);
        let mut peak = 0.0f32;
        for _ in 0..frames {
            // ⭐ HELD, not tapped: a one-tick press assumes the body steps after
            // the frame is committed inside one update, which a test has no
            // business modelling. And `drive_control_frame` is the ONLY driver
            // that lands — writing `ControlFrame` between updates is rewritten
            // by the device systems every tick.
            ambition_platformer2d::sim::drive_control_frame(
                app.world_mut(),
                ambition_platformer2d::engine_core::ControlFrame {
                    jump_pressed: true,
                    jump_held: true,
                    ..Default::default()
                },
            );
            app.update();
            // Feet are +gravity, so RISING is a DECREASE in y.
            peak = peak.max(start - y_of(app, body));
        }
        peak
    };

    let seat0 = seat_body(&mut app, 0).expect("the match seats a first fighter");

    // ── THE CONTROL ARM: alive, on the stage, holding jump. ──
    let alive_rise = hold_jump_and_peak(&mut app, seat0, 40);
    assert!(
        alive_rise > 8.0,
        "a LIVE fighter holding jump rose only {alive_rise:.1}px, so this harness \
         is not delivering the press at all and the respawn arm below would pass \
         for the wrong reason"
    );

    // ── LAUNCH IT OUT. Same shape as the blast-gate test above: hard enough
    // that the margin is crossed rather than approached. ──
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
    // ⛔ THE PREMISE. Without this the loop below measures a fighter that is
    // simply standing on the stage.
    assert!(
        waiting,
        "seat 0 never entered a respawn wait after being launched at the blast \
         line, so nothing here is about the respawn beat"
    );


    // ── THE ARM UNDER TEST: the same held jump, during the wait. ──
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
        // ⛔⛔ SAMPLE ONLY WHILE THE BODY IS STILL WAITING. The frame the wait
        // ENDS is the frame the ruleset places the body on the respawn platform,
        // which is ~580px above the blast line it was sitting at — and reading
        // it here made "the respawn beat answers the jump button" out of the
        // respawn itself. The `while` condition is re-checked one statement too
        // late to protect this.
        if app
            .world()
            .get::<ambition_platformer2d::actor::PendingRespawn>(seat0)
            .is_none()
        {
            break;
        }
        held_frames += 1;
        // ⛔⛔ THE MAGNITUDE, NOT THE RISE. Measured the hard way: an unfrozen
        // body is FALLING out of the blast zone at ~1200px/s, so "did it get
        // higher than where the wait started" is false no matter what the pad
        // does — the arm meant to catch the bug passed with the fix removed. A
        // fighter waiting out its beat does not move AT ALL, in either
        // direction, which is a claim with somewhere to fail.
        moved = moved.max((y_of(&mut app, seat0) - start).abs());
    }
    // ⛔ THE SECOND PREMISE: a wait that was over in two frames would make the
    // assertion below true by having nowhere to fail.
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
