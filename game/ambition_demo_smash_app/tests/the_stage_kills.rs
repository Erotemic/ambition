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

    // ⭐⭐ **WATCH FOR THE RESTART ANNOUNCEMENT**, before the launch that causes
    // it (ledger D90). `BodyLifetime::restart_pending` is a ONE-SIM-TICK flag —
    // the reset raises it and `announce_body_restarts` clears it in the next
    // `WorldPrep` — and a fixed-tick host advances several sim ticks per
    // `app.update()`, so sampling the flag between updates can miss it entirely.
    // The engine publishes this trigger for exactly that reason; an observer
    // cannot miss what a poll can.
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

    // ⚠ **the loop above BREAKS on the stock change, and the restart comes after
    // it.** The spend and the ruleset's respawn are different steps in different
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
    // Without this the test would pass just as happily on a stock counter that
    // decremented on its own, which is the failure mode of every "did the number
    // change" assertion.
    assert_eq!(
        stocks_of(&mut app, 0),
        Some(before),
        "the fighter that was never launched also lost a stock, so the counter \
         is moving on its own and this test proves nothing about the blast gate"
    );

    // ⭐⭐ **AND THE BODY ANNOUNCED THAT IT STARTED AGAIN** (ledger D90). A stock
    // spent without an announcement leaves every provider holding round-or-life
    // state — a charge meter, a combo counter, a per-life buff — carrying it into
    // the next stock. That is invisible in a stock count and visible in a fight.
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
        // ⚠ non-vacuity, the same shape as the stock assertion above: an
        // announcement everybody gets says nothing about this knockout.
        assert!(
            !seen.contains(&untouched),
            "the fighter that was never launched also announced a restart"
        );
    }

    // ⚠ **and it came back where the RULESET says**, not where it died. A body
    // that respawns at its blast position is outside the stage and falls again.
    {
        use ambition_platformer2d::actor::BodyKinematics;
        let respawn = ambition_demo_smash::respawn_placement(ambition_demo_smash::stage_centre());
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

/// **THE 3-2-1-GO IS ON THE SCREEN.**
///
/// ⛔⛔ reported from the couch, 2026-08-15: *"I think there is also a countdown
/// to start the match, but there is no visual indication of that countdown, like
/// a 3, 2, 1, go."* There WAS a countdown — the fighters are held and released
/// by it — and it announced itself into a channel nothing draws. Every existing
/// test proved the HOLD (the fighters do not move during the ceremony); none
/// asked whether a player could see why.
///
/// ⭐ so this watches the slot the stage DECLARES, `smash_announce`: the centred
/// card the HUD renders, beside the fighter percents that were always visible.
/// Before the rewiring the slot was declared and never written once, so this
/// finds an empty card for the whole ceremony.
///
/// ⚠ **it asserts the COUNT, not the tick.** Which frame carries "2" is a tuning
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

/// **THE CAMERA COMES BACK NO FASTER THAN IT LEFT.**
///
/// ⛔⛔ reported from the couch, 2026-08-15: *"the camera zooms out when someone
/// flys off the stage, and that is good, but when they die in the blast zone
/// instead of having a smooth camera transition back, it just snaps back to the
/// main stage."*
///
/// Both halves of that sentence are one number, which is why this measures the
/// RATIO rather than either rate. Measured before the fix, over four knockouts
/// in one CPU-versus-CPU match:
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
/// ⚠ **the non-vacuity guard is the OPEN**, and it is doing real work: a match
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
                // ⚠ skip the opening frames: the first resolve ADOPTS the cast's
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

/// **A MATCH SOMEBODY WINS ACTUALLY ENDS.**
///
/// ⛔⛔ reported from the couch, 2026-08-15: *"there seems like several cases
/// where everyone but one player dying will not cause a match to end
/// correctly."* Every other test in this file measures a KNOCKOUT — the launch,
/// the blast boundary, the stock spend, the respawn, the body that stops
/// falling. None of them asks the question a viewer asks, which is whether the
/// match is over when only one fighter is left.
///
/// ⚠ **"several cases" is the shape of a SCHEDULING AMBIGUITY, and that is what
/// it was.** `decide_stocks_match` reads the sides off the bodies that still
/// exist; `take_eliminated_fighters_out_of_play` despawns an eliminated body.
/// Both sat in `CombatSet::Settle` with nothing ordering them, and the ruleset's
/// `.chain()` inserts an `ApplyDeferred` that makes the despawn visible part-way
/// through the set. Lose the last loser's row and `last_side_standing` sees ONE
/// side — and one side is not a match, so it answers `None` forever. Whether a
/// match ended depended on how the scheduler broke a tie, which is why it
/// happened in some compositions and not others.
///
/// ⭐ PROBED RED: with `take_eliminated_fighters_out_of_play` ordered
/// `.before(MatchOutcomeDecided)` instead of after — the broken order, made
/// explicit — this runs a full match, watches a fighter be eliminated, and never
/// settles.
///
/// ⚠ **the elimination is asserted first**, because a match that simply never
/// got anybody killed would settle nothing and the claim below would be about a
/// fight that did not happen.
#[test]
fn a_match_whose_last_loser_is_removed_still_decides() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::combat::components::FighterStocks;
    use ambition_platformer2d::combat::stocks::StocksMatchSettled;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // BOTH seats CPU, so somebody actually loses. `smash_roster` makes seat 0 a
    // human with no controller, which is a match one fighter cannot lose.
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

    // ⚠ **seats, not stocks.** A fighter reduced to zero is ELIMINATED and
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
        if app
            .world()
            .get_resource::<StocksMatchSettled>()
            .is_some_and(|s| s.0)
        {
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

/// ⛔⛔ **DELETED 2026-08-11 (ledger D90): `losing_a_stock_announces_a_body_
/// restart`.** It teleported a fighter to `y = 100_000` and waited for
/// `BodyLifetime::restart_pending` to appear.
///
/// Two things were wrong with it, and the second only became visible after the
/// first was measured:
///
/// 1. **a raw `BodyKinematics::pos` write is not "this fighter lost a stock".**
///    Measured: one app update later the body sat at a normal stage position with
///    all THREE stocks — something noticed the nonsense position and relocated
///    it, which is not a knockout. The test spent its life asserting a restart no
///    KO had caused.
/// 2. **`restart_pending` is a ONE-SIM-TICK flag** — raised by the reset, cleared
///    by `announce_body_restarts` in the next `WorldPrep` — and a fixed-tick host
///    advances several sim ticks per `app.update()`. Polling it between updates
///    can miss it entirely, whatever caused it.
///
/// ⭐ **its intent is fully covered by
/// `a_launched_fighter_is_taken_by_the_world_and_spends_a_stock`**, which causes
/// a REAL knockout — a real launch, the real blast boundary — and now proves the
/// whole chain from one: exactly one stock spent, the other fighter untouched, a
/// `BodyRestarted` trigger observed for that body and not the other, and a
/// respawn at the ruleset's placement. An observer cannot miss what a poll can.

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
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // ⭐ **THE authority a seat's policy lives in.** This asked an archetype
    // table alone (2026-08-11), so the day this demo published its CPU ladder as
    // real `BrainProfile`s and deleted its archetype fragment, four perfectly
    // seatable fighters were reported unseatable; it then asked both, and asks
    // the only one since P2.18 deleted the archetype arm. The question — *can
    // this demo fill the seats it declares?* — has never changed.
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

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // ⭐ the demo's PUBLISHED policies — its CPU ladder lives here now, not in an
    // archetype fragment (that fragment is deleted), and since P2.18 there is
    // nowhere else a seat's policy could come from.
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
    // ⭐ **the ONE authority a seat's policy can live in**, resolved in this
    // demo's own provider exactly as `seat_brain_profile` resolves it.
    //
    // ⛔ this was `archetypes.has_brain_key(profile) || published.get(..)`, and
    // the first term made the guard unfalsifiable in the direction that matters:
    // an archetype table could answer for a rung whose policy was never
    // published, which is the state D87's deletion was supposed to have ended.
    // `seat_brain_profile` has one arm since 2026-08-13 (P2.18), so a term that
    // seating cannot use has no business in a guard about what seating can do.
    let resolves = |profile: &str| {
        published
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

/// **THE THREE PLATFORM-FIGHTER VERBS REACH A LIVE SEATED BODY** — campaign
/// P4.29 (shield/parry), P4.30 (grounded dodge) and P4.32 (ledge), whose rows
/// all read *"authored, ▢ unverified in play"*.
///
/// ⭐ **the gap those three ▢ marks name is not a capability, it is a
/// MEASUREMENT.** The verbs are authored on the fighters' `CharacterDefinition`
/// and the engine has had the machinery all along — a bubble shield with a parry
/// window, a dodge roll with i-frames, a full ledge system. What nothing checked
/// was the whole distance between the two: a definition authors an `AbilitySet`,
/// preparation folds it, seating builds a body, and a match may MASK it
/// (`fighter_abilities` is an intersection, so a stage that forgot a verb
/// silently removes it). Every step of that had its own test; the chain did not.
///
/// ⛔ **which is exactly how these three went missing before.** The row records
/// it: a capability had ONE authoring surface, the enemy archetype, so a fighter
/// seating through `combatant` could not have them; the match then stamped one
/// flat set over every body, and three verbs were simply absent from that set.
/// Both halves would pass a test of either end alone.
///
/// ⚠ **and the poison is a verb the fighters DELIBERATELY do not author.**
/// `fly` and `blink` are the exploration protagonist's traversal kit and are
/// stated absent on purpose ("this is a platform fighter's ground game").
///
/// ⛔⛔ **what the two falsifiers actually measured, which is narrower than
/// "the character decides" and is the real contract** (run 2026-08-13):
///
/// ```text
///   character drops `shield`         -> body cannot shield   (character NECESSARY)
///   character adds `fly`, mask omits -> body still cannot    (mask NECESSARY)
///   character adds `fly`, mask adds  -> body CAN fly         (both SUFFICIENT)
/// ```
///
/// ⇒ that is `AbilitySet` INTERSECTION, end to end, through a live stage: a
/// character states what its body can do, a ruleset states what this mode
/// permits, and a verb needs both. The middle row is the one worth keeping —
/// authoring a capability onto a character is NOT enough to smuggle it into a
/// mode, which is what makes P3.25's mask a real restriction rather than
/// decoration. The first row is P4.29/30/32's actual subject.
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
        // ⭐ P4.29 / P4.30 / P4.32, on the LIVE body, through the real route.
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
        // ⛔ THE POISON: verbs these fighters state they do NOT have.
        assert!(
            !abilities.fly && !abilities.blink_through_hard_walls,
            "seat {seat} came out able to fly or blink, which its character does \
             not author — so the body is wearing a generic set (the engine's, or \
             a match-wide grant) rather than its own"
        );
    }
}

/// **YOU CAN SEE THE THING THAT DECIDES EVERY MATCH.**
///
/// ⛔⛔ **every stock in a platform fighter is spent OFF the stage, and the
/// camera was framing the empty platform while it happened** (queue D128 item 2,
/// measured 2026-08-16 by photographing a real two-CPU match through the shipped
/// shell). f330 drew a fighter past the left screen edge, f345 drew the other
/// one behind the virtual joystick, and at f360 the stage was EMPTY — both
/// fighters gone, camera still on the platform, the knockout happening
/// off-screen. A watcher could not see the one moment the genre is about.
///
/// ⭐ **the framing policy was never the problem** — `frame_the_cast` already
/// framed every live seat. Three things downstream threw that framing away, and
/// this test is red on each of them:
///
/// ```text
///   the ROOM CLAMP        a blast zone is OUTSIDE `world.size` by construction,
///                         so the region a body can die in is precisely the
///                         region a room-clamped camera cannot look at. On this
///                         stage the view is also WIDER than the world, so
///                         `clamp_or_center` pinned the camera at the world
///                         centre for the entire match.
///   `stable_center`       the synthesized cast body had `size: ZERO` against a
///                         `base_size` of the framing VIEW, so the crouch
///                         compensation shifted the frame half a screen: the
///                         cast's centre at y=366 targeted as y=202.
///   the TARGET EASE       8 Hz lags a launched body by ~v/8 — 46 units past the
///                         edge at the moment of the knockout.
/// ```
///
/// ⚠ **the non-vacuity guard is the LEAVING**, and it is doing real work: a
/// match where nobody was ever knocked off the platform keeps every fighter
/// inside any frame at all, and this fixture is a live fight rather than a
/// scripted one. So it first proves a body reached the blast zone — OUTSIDE the
/// room's own bounds — and only then asks what the camera was showing.
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

/// **AND THE FRAME DOES NOT CUT WHEN A FIGHTER LEAVES PLAY.**
///
/// The companion to [`the_camera_closes_no_faster_than_it_opened`], and it
/// exists because that one made this one reachable. That test measures the view
/// SIZE, which was the only thing the cast framing could move while the room
/// clamp had the centre pinned at the world's middle. Now that the centre
/// travels — it must, or a fighter cannot be followed off the stage — it has
/// the same discontinuity the size had: an eliminated body is taken out of play
/// and the cast's box collapses between two frames, jumping its centre back to
/// the platform.
///
/// ⭐ measured on this fixture before the framing was carried as an eased BOX:
/// a 248.8-unit collapse of the cast's own centre came out as a **209.6-unit
/// camera step in one frame**, against ~33 in an ordinary frame. It is now 27.
///
/// ⚠ **it compares the ELIMINATION frame against the ordinary ones**, which is
/// the only comparison that means anything here: a cast centre that is tracking
/// a fast fight moves a long way per frame quite correctly, and a threshold in
/// units would be a guess about how hard fighters hit.
///
/// ⚠ **the non-vacuity guard is the JUMP the framing had to absorb**: a match
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
