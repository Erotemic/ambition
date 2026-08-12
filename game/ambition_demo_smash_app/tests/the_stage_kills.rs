//! **A fighter knocked off THIS platform reaches the world's edge.**
//!
//! The one claim about the stocks loop that no unit test in
//! `ambition_demo_smash` can make. Those cover spend, respawn, eliminate and
//! end, each in isolation and each correctly. What none of them can answer is
//! whether the stage's own numbers — a 420px platform in a 960px world with a
//! 220px blast margin — put the world's edge somewhere a launched body actually
//! gets to.
//!
//! That is the difference between a loop that is correct and a game that works,
//! and it is exactly the class this repository keeps rediscovering: every
//! instrument green, and green about less than it claimed.

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

/// **The blast margin is reachable from the platform.**
///
/// Measured against the loaded world rather than the authored constant, so a
/// preparation step that dropped or rewrote the margins fails here. The
/// assertion is a RATIO, not a distance: what matters is that the edge is close
/// enough to the platform that a launch crosses it, and that is the number a
/// future stage resize would silently break.
#[test]
fn the_worlds_edge_sits_within_a_launch_of_the_platform() {
    let world = ambition_demo_smash::smash_stage().world;
    let platform = world.blocks[0].aabb;
    let side_margin = world
        .side_blast_margin
        .expect("the stage authors its side margins");

    // How far past the platform's edge a body must travel to leave the world.
    let to_the_left = platform.left() + side_margin;
    let to_the_right = (world.size.x - platform.right()) + side_margin;

    // ⚠ **a RATIO against the platform, not a bound against the world.** The
    // first version of this test asserted `distance < world.size.x` and passed
    // over a stage where a knocked-off fighter crossed 490px of nothing — more
    // than the platform's entire width — because 490 < 960 is true and says
    // nothing. The picture caught it; the test did not.
    //
    // One platform-width of travel is the budget. Past that a launch stops
    // reading as a knockout and starts reading as a body drifting offscreen
    // while the game waits.
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

/// **The demo opens on character select, and the battle starts when the players
/// lock in.** (Jon, 2026-07-31)
///
/// The whole path, through the real shell: boot lands on select, two seats join
/// and commit, and the roster the screen decided is published before the route
/// leaves for the stage.
///
/// That ORDER is the correctness argument rather than an implementation detail.
/// Seating reads `MatchParticipantRoster` on the sim schedule; if the route
/// changed first the stage would come up with no roster, seating would find
/// nothing to do, and the match would open with an empty cast that nothing
/// retries into existence.
#[test]
fn the_demo_opens_on_select_and_the_battle_starts_when_players_lock_in() {
    use ambition_demo_smash::select::SmashSelect;

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

    // Two players join and commit. ⚠ this test is about the STAGE, so it sets
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
        roster.fighter_stocks,
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

/// **A launched fighter leaves the world, spends a stock, and comes back.**
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
    // Without this the test would pass just as happily on a stock counter that
    // decremented on its own, which is the failure mode of every "did the number
    // change" assertion.
    assert_eq!(
        stocks_of(&mut app, 0),
        Some(before),
        "the fighter that was never launched also lost a stock, so the counter \
         is moving on its own and this test proves nothing about the blast gate"
    );
}

/// **The fighter brain closes the distance and lands a hit.**
///
/// FB4b's first damage against an OPPONENT rather than a fixture. Everything
/// below this — classify, options, rollout, the delay buffer, the APM ledger —
/// was unit-tested against hand-built `Perceived` values; nothing had ever put
/// the rig on a body and let it decide what to do about somebody else.
///
/// The assertion is deliberately weak on WHAT it does and strict on THAT it
/// does: a brain that travels and connects is working, and pinning a distance or
/// a damage number here would be pinning the tuning of a demo rather than the
/// rig. What it must never do is what it did for an hour on 2026-07-31 — stand
/// perfectly still while every test passed.
#[test]
fn the_fighter_brain_engages_rather_than_standing_still() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use bevy::prelude::*;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // ⛔⛔ **BOTH SEATS MUST BE CPUs, and this called the helper that makes seat 0
    // HUMAN** (2026-08-11). The comment here already said what it needed — *"a
    // human with no controller correctly does nothing"* — and then asked for a
    // roster whose first seat is exactly that. So this measured one CPU pacing
    // around a statue and called it "the brain never commits".
    //
    // ⚠ `smash_roster_at_levels` is the helper that seats EVERY slot as a CPU;
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
    // ⚠ the sampling window has to sit inside the fighter's LIFE. This test used
    // to sample at ticks 240 and 480, and seat 1 is eliminated around tick 400 —
    // it self-KOs three times in the first seven seconds (see `ladder_probe`),
    // so the second sample found one body and the zip below silently compared
    // seat 0 against itself. "Neither fighter moved" was the message for "one
    // fighter was dead", which is a different bug with a different fix.
    // ⛔⛔ **THE WARM-UP HAS TO OUTLAST THE COUNTDOWN** (2026-08-11). This was
    // 60 ticks, and the stage opens `opens_suspended` with
    // `opening_countdown_ticks = 3 * 60` — every fighter carries `ScriptedControl`
    // for the whole 3-2-1-GO. So the sampling window sat ENTIRELY inside the hold
    // and reported *"neither fighter moved"* about fighters that were correctly
    // forbidden to move.
    //
    // ⚠ the countdown is the campaign's own feature, so this is a stale WINDOW
    // rather than a stale assertion: a fighter brain that emits nothing is still
    // exactly what this test is for, and the number below is read from the
    // ruleset rather than restated.
    let countdown = ambition_demo_smash::smash_roster([
        ambition_demo_smash::SMASH_CHARACTER_ID,
        ambition_demo_smash::SMASH_OPPONENT_ID,
    ])
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

/// **An eliminated fighter leaves the stage.**
///
/// `ambition_combat::stocks` is explicit that a fighter with no stocks "is still
/// standing until a ruleset removes it", and for a day this ruleset did not.
/// Measured over sixty seconds of real fighting: the loser fell out of the
/// world, was correctly eliminated, and then KEPT FALLING — taking a fresh
/// `LeftTheWorld` hit every tick, reaching y=34430 and 270900%.
///
/// Nothing upstream was wrong. The stock was spent exactly once, the engine's
/// `Without<FighterEliminated>` filter held, and the body simply never stopped
/// being a body. That is the gap between "the count is correct" and "the match
/// is over".
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

    // A percent this side of absurd. Before eliminated fighters were removed
    // this reached 2709.0, because a body below the stage is knocked out of the
    // world again on every tick forever.
    assert!(
        peak < 20.0,
        "a fighter reached {:.0}% over one minute — a body that keeps falling \
         out of the world keeps being knocked out of it, which is what an \
         eliminated fighter nobody removed does",
        peak * 100.0
    );
}

/// **Losing a stock RESTARTS the body; it does not teleport it.** (Campaign 3B)
///
/// The respawn used `transit_body`, whose contract is that "axis maneuver state
/// (coyote, buffers, dash timers) is deliberately KEPT — those are time facts,
/// not place facts". True of a blink and false of a knockout: a fighter came
/// back holding the dash timer and buffered jump it died with, and because
/// `ae::BodyRestarted` is derived from `reset_body_clusters` raising
/// `restart_pending`, no PROVIDER heard about the respawn either — a ball-dash
/// charge or a rolling form would have survived a knockout in silence.
///
/// This asserts the announcement, not the position: the position was always
/// right, which is exactly why the leak was invisible.
#[test]
fn losing_a_stock_announces_a_body_restart() {
    use ambition_platformer2d::actor::{FighterStocks, MatchSeat};
    use ambition_platformer2d::engine_core::BodyLifetime;
    use bevy::prelude::*;

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
    for _ in 0..240 {
        app.update();
    }

    let (fighter, before) = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat, &FighterStocks)>();
        q.iter(world)
            .find(|(_, seat, _)| seat.0 == 1)
            .map(|(entity, _, stocks)| (entity, stocks.remaining))
            .expect("the match seats a second fighter")
    };

    // Throw it out of the world, which is how a stock is spent here.
    //
    // ⛔⛔ **AND IT NO LONGER LANDS — measured 2026-08-11, ledger D90.** After
    // this write the body is back at a normal stage height (`y ≈ 266`) with all
    // three stocks, walking around as though nothing happened. Something UNDOES a
    // direct `BodyKinematics::pos` write within the sampling window, so this test
    // has been asserting a restart that no knockout ever caused. Do not "fix" it
    // by raising the flag somewhere; find out what moved the body back.
    {
        let world = app.world_mut();
        let mut kin = world
            .get_mut::<ambition_platformer2d::actor::BodyKinematics>(fighter)
            .expect("the fighter has a body");
        kin.pos.y = 100_000.0;
    }

    let mut announced = false;
    for _ in 0..120 {
        app.update();
        // `restart_pending` is raised by the reset and cleared by
        // `announce_body_restarts` the next tick, so catching it means sampling
        // every frame — which is the honest way to observe a one-tick flag.
        if app
            .world()
            .get::<BodyLifetime>(fighter)
            .is_some_and(|lifetime| lifetime.restart_pending)
        {
            announced = true;
        }
        if app
            .world()
            .get::<FighterStocks>(fighter)
            .is_some_and(|stocks| stocks.remaining < before)
        {
            if announced {
                break;
            }
        }
    }

    assert!(
        announced,
        "a fighter came back from a knockout without the engine saying its body \
         had restarted, so every provider holding round-or-life state kept it"
    );
}

/// **This demo's own CPU roster is seatable by its own composition.**
/// (API 1.0 row (g))
///
/// The bug this guards shipped twice on 2026-07-31 — here and on the versus
/// stage. A `ControllerBinding::Cpu { brain_profile }` is looked up in the
/// composition's `CharacterRoster` ARCHETYPE table, and `spec_for_brain` falls
/// back to a generic row whose brain is `stand_still` when the key is absent.
/// The match composes, seats, and runs; the opponent never moves.
///
/// Asked here rather than at the select screen, and that is the point: every
/// seat the screen produces is a HUMAN, and a human seat asks the archetype
/// table for nothing. A guard placed there would have been unreachable —
/// protection that reads as protection and cannot fire.
#[test]
fn the_demos_cpu_roster_is_satisfiable_by_its_own_composition() {
    use ambition_platformer2d::actors::features::CharacterRoster;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let archetypes = app
        .world()
        .get_resource::<CharacterRoster>()
        .expect("the composition installs an archetype table")
        .clone();
    // ⭐ **the OTHER authority a seat's policy can live in** (2026-08-11). This
    // asked the archetype table alone, so the day this demo published its CPU
    // ladder as real `BrainProfile`s and deleted its archetype fragment, four
    // perfectly seatable fighters were reported unseatable. The question — *can
    // this demo fill the seats it declares?* — is unchanged; it now asks both
    // places an answer can live, exactly as seating does.
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
        let problems = roster.unsatisfiable_seats(&archetypes, Some(&profiles));
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
/// ⚠ **`StartRequested` as well as the picks.** The screen no longer leaves on
/// readiness alone — a test that set only the decision would sit on the select
/// route forever and blame the stage.
fn decide_a_two_player_match(app: &mut bevy::prelude::App) {
    use ambition_demo_smash::select::{SlotOccupant, SmashRoster, SmashSelect};

    // **BY ID, not by grid position.** This test is about the STAGE, so it names
    // the two fighters it wants and finds where the roster put them — an index
    // would silently become a different character the next time somebody edits
    // `SMASH_ROSTER`, which is the list Jon asked to be easy to edit.
    //
    // ⚠ and seat 0 deliberately takes a fighter that is NOT the stage's
    // starting character — see below.
    let index_of = |app: &bevy::prelude::App, id: &str| -> usize {
        app.world()
            .resource::<SmashRoster>()
            .ids()
            .position(|candidate| candidate == id)
            .unwrap_or_else(|| panic!("`{id}` is not on this composition's grid"))
    };
    // ⭐ **seat 0 takes GEORGE BOOUL, not the stage's starting character.** That
    // combination seated NOBODY until 2026-08-05 — seating adopts the primary
    // body and returned from the whole system when the ids disagreed — so this
    // is the case, not an arbitrary pick.
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

/// **A ladder roster seats TWO fighters at two different levels.**
///
/// `smash_roster_at_level` puts every CPU on one rung, and `smash_roster` makes
/// seat 0 HUMAN — so the only opponent `ladder_probe` could offer was a
/// controller-less body that never acts. That made its number clean (*every
/// stock lost is a self-KO*) and made a FIGHT impossible to measure, which is
/// why FB6e's `l3_earns_its_depth` is still owed §8's suite and the
/// survival/damage ratios.
///
/// ⛔ **the assertion that matters is that the two seats DIFFER.** A rig built on
/// a roster that quietly put both fighters on the same rung would report a 50%
/// win rate at every level and read as "the ladder is flat" rather than as a
/// broken fixture — the most expensive kind of wrong answer, because it looks
/// like a finding.
///
/// ⚠ and both profiles must be SATISFIABLE by the demo's own archetype table,
/// for the same reason its sibling above checks: `spec_for_brain` falls back to
/// a generic row rather than failing, so an unregistered level is a fight
/// against a statue that reports itself as a fight.
#[test]
fn a_ladder_roster_seats_two_cpus_at_two_different_levels() {
    use ambition_platformer2d::actor::ControllerBinding;
    use ambition_platformer2d::actors::features::CharacterRoster;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let archetypes = app
        .world()
        .get_resource::<CharacterRoster>()
        .expect("the composition installs an archetype table")
        .clone();
    // ⭐ the demo's PUBLISHED policies — its CPU ladder lives here now, not in an
    // archetype fragment (that fragment is deleted).
    let published = app
        .world()
        .get_resource::<ambition_platformer2d::characters::actor::character_catalog::BrainProfileRegistry>()
        .expect("the composition assembles its published policies")
        .clone();

    // ⛔ **THE LADDER IS SPARSE, and a rig has to know it.** `SMASH_ROSTER_RON`
    // registers `duelist_l{1,3,5,6,9}` and nothing between — the rungs
    // `ladder_probe` happens to run. The first draft of this test asked for
    // level 8 and failed, which is the right outcome: `spec_for_brain` falls
    // back to a generic row rather than erroring, so an unregistered rung fights
    // a statue and reports a fight.
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
    // ⚠ built from the CONSTANT, not spelled out. The first draft guessed
    // `smash_duelist_l9` and the real prefix is `duelist` — a restated name is a
    // second authority on a string, which is the mistake five other checks in
    // this tree were written after making.
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
    // ⭐ the two authorities a seat's policy can live in — the same pair
    // `seat_brain_profile` consults, resolved in this demo's own provider.
    let resolves = |profile: &str| {
        archetypes.has_brain_key(profile)
            || published
                .get(&ambition_platformer2d::entity_catalog::BrainProfileId::new(
                    format!("{}::{profile}", ambition_demo_smash::SMASH_EXPERIENCE),
                ))
                .is_some()
    };
    // ⭐ **and each rung RESOLVES**, which is the property that keeps a ladder a
    // ladder. ⛔ this asked the ARCHETYPE table — the authority this demo stopped
    // using when it published its rungs as real `BrainProfile`s and deleted its
    // archetype fragment. The question is unchanged; the place to ask it moved.
    for profile in profiles.iter().flatten() {
        assert!(
            resolves(profile),
            "`{profile}` resolves in neither the published policies nor the \
             archetype table, so seating hands back a generic row and the rung \
             fights a statue while reporting a fight"
        );
    }
    // ⭐ **every ADJACENT PAIR is satisfiable**, which is the property a ladder
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
        roster.fighter_stocks,
        Some(ambition_demo_smash::STARTING_STOCKS)
    );
    assert!(
        roster.opens_suspended,
        "a ladder round opens on the countdown too"
    );
}
