//! **A match that activates INSIDE a rollback window survives being rewound.**
//!
//! Two independent GPT 5.6 reviews (2026-07-29) named the same defect, and both
//! were right: `ActiveMatch` is published from inside the simulation schedule on
//! the tick the last seat is filled, it GATES two behaviours — seating returns
//! early while it exists, and the countdown treats it as proof the match is live
//! — and it was not rollback state. Neither were `MatchSeat` (which seat a body
//! is), `MatchTeam` (who may hit whom), or `RulesetOwnsDeath` (who owns a KO).
//!
//! A rewind across activation therefore restored the fighters — or un-spawned
//! them — while the latch kept pointing at a future in which they existed:
//! seating refused to rebuild the roster it had just lost, and the countdown
//! carried on against whatever remained.
//!
//! ## Why no instrument caught it
//!
//! The rollback coverage sweep exists precisely to name unregistered simulation
//! state, and it reported green over all four for as long as they existed. Two
//! separate blindnesses, both worth recording because both are recurring:
//!
//! * **no swept population contained a match**, so the component sweep never saw
//!   a body wearing `MatchSeat`. Same shape as A19 (`PogoTarget`, `ChestFeature`,
//!   `PortalHostScanned` were never in the population, not missed within it);
//! * **a MODULE-FAMILY waiver swallowed the resource.** `ActiveMatch` lives in
//!   `ambition_actors::character_runtime::`, which carried a blanket waiver
//!   reading *"character art load bookkeeping; decoded-ness has no simulation
//!   consequence"* — written when that module held only art loading, and still
//!   in force after the module grew seating. Third instance of that class after
//!   `BossAnimFrame` (A9/A18) and `CharacterRoster`/`CharacterRosterRegistry`.
//!
//! Both are fixed in `rollback_coverage.rs`, and **that sweep is the guard** —
//! it goes red the moment any of the four registrations is removed.
//!
//! ## ⚠ What the tests in THIS file do and do not establish
//!
//! They are corroborating, not load-bearing, and saying so is the point: both
//! were checked with the registrations REMOVED and both still passed. Do not
//! read a green run here as evidence that the rewind hole is closed.
//!
//! The reason is a fixture limit, established by probe rather than argued:
//!
//! * seating completes on the session's FIRST simulated frame, so activation is
//!   always GGRS frame 1;
//! * a sync test with `check_distance: 4` issues its first load at frame 6, of
//!   frame ~1 — it never restores frame 0.
//!
//! So no rewind in this fixture ever restores a PRE-activation world, which is
//! the only state in which the unregistered latch could strand a roster. The
//! scenario that reaches it needs activation delayed past `check_distance`,
//! which is what the shipped versus route does naturally (a body is constructed
//! several ticks after the route is entered). That fixture is queue row AC24.
//!
//! What they DO check, and what they would catch: that a match's roster and
//! teams survive ordinary resimulation, and that the activation count never
//! disagrees with the number of bodies wearing a seat.
//!
//! ⚠ **and none of this makes activation a transaction.** Fighters are still
//! constructed seat by seat over several ticks; only the latch is published
//! atomically. A rewind landing between two seats leaves a body that exists and
//! a roster that is incomplete — legal today, retried by seating, and the reason
//! AA2's lifecycle half stays open.

#![cfg(feature = "rl_sim")]

use ambition::actors::character_runtime::{
    ActiveMatch, ControllerBinding, MatchParticipant, MatchParticipantRoster, MatchSeat,
};
use ambition_app::rl_sim::{AgentAction, AmbitionSim, SandboxSim, SandboxSimOptions, TimestepMode};

/// A sync-test sim: every frame is saved, rewound and resimulated, so the
/// activation tick is inside a rollback window by construction rather than by a
/// forced rewind somebody has to remember to trigger.
fn match_sim() -> SandboxSim {
    SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds")
}

/// Two CPU seats from the robot lineage — the ids a plain sandbox actually
/// prepares. `seat_character` returns `None` for an unprepared id SILENTLY, so a
/// roster naming versus-route content seats nothing and every assertion below
/// would pass over an empty world.
fn two_cpu_roster() -> MatchParticipantRoster {
    let cpu = |character: &str, team: &str| {
        MatchParticipant::new(character)
            .driven_by(ControllerBinding::Cpu {
                brain_profile: Some("medium_striker".to_string()),
            })
            .on_team(team)
    };
    MatchParticipantRoster {
        participants: vec![
            cpu("player_robot_v3", "blue"),
            cpu("player_robot_v2", "red"),
        ],
        opens_suspended: true,
        seat_topology: Some(11),
        fighter_abilities: None,
    }
}

fn seats(sim: &mut SandboxSim) -> Vec<(usize, String)> {
    let world = sim.world_mut();
    let mut query = world.query::<(&MatchSeat, &ambition::combat::targeting::MatchTeam)>();
    let mut rows: Vec<(usize, String)> = query
        .iter(world)
        .map(|(seat, team)| (seat.0, team.as_str().to_string()))
        .collect();
    rows.sort();
    rows
}

/// Frames of ordinary simulation before the roster is introduced, so the world
/// under test is a settled one rather than a boot frame.
const FRAMES_BEFORE_THE_ROSTER: usize = 20;

/// **Put the roster in, and make THAT the rollback baseline.**
///
/// The two obvious arrangements are both wrong, and each was written first:
///
/// * **roster as frame-zero setup.** Then seating activates on tick 0 — probed,
///   not assumed — and GGRS cannot rewind to before its own frame zero. Nothing
///   ever resimulates across the activation boundary, and the test passes with
///   the registrations REMOVED. It was checking that a match survives a rewind
///   window it was never inside.
/// * **roster inserted mid-run without rebasing.** That is a direct `world_mut`
///   mutation behind the rollback cursor, and the harness contract says so:
///   resimulating frames 18–20 replays a world with no roster in it, and the
///   sync test reports the mismatch immediately (it did).
///
/// So: simulate, insert, and REBASE. Frame zero of the new session is "the
/// roster exists and nothing is seated"; activation lands on frame one. A
/// sync-test rewind to frame zero therefore genuinely crosses it — it restores
/// a world in which the fighters do not yet exist, which is precisely the state
/// the reviews say the latch used to survive.
fn introduce_the_roster(sim: &mut SandboxSim) {
    for _ in 0..FRAMES_BEFORE_THE_ROSTER {
        sim.step(AgentAction::default());
    }
    sim.world_mut().insert_resource(two_cpu_roster());
    sim.rebase_rollback_history()
        .expect("the roster insert becomes the rollback baseline");
}

/// A match's roster and teams survive ordinary resimulation, checksum-clean.
///
/// ⚠ **not** the reviews' acceptance condition (*rewinding around the first
/// active frame reconstructs the identical roster*) — see the module header for
/// why this fixture cannot reach that frame, and AC24 for the one that would.
#[test]
fn a_match_roster_survives_resimulation_checksum_clean() {
    let mut sim = match_sim();
    introduce_the_roster(&mut sim);

    let mut activated_on = None;
    for tick in 0..120 {
        sim.step(AgentAction::default());
        // Checked EVERY tick, not once at the end. The activation tick is the
        // one under test, and a divergence there is repaired by later frames
        // agreeing with each other — a run that only looks at the end can watch
        // the interesting frame go wrong and report success.
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("tick {tick}: {error}"));
        if activated_on.is_none() && sim.world().get_resource::<ActiveMatch>().is_some() {
            // The GGRS FRAME, not the harness tick. Harness tick 0 advances GGRS
            // to frame 1, so the tick index cannot distinguish "activated on the
            // baseline" from "activated one frame after it" — and that
            // distinction is the whole fixture.
            activated_on = Some((
                tick,
                sim.rollback_execution_stats()
                    .map(|stats| stats.last_simulated_frame)
                    .unwrap_or(0),
            ));
        }
    }

    let (activated_on, activation_frame) = activated_on.expect(
        "no roster seat ever produced a live match, so this test rewound a world \
         with no match in it and would have passed either way",
    );
    // The window must genuinely surround the activation frame on BOTH sides, or
    // "rewound across activation" is a claim about a frame nothing revisited.
    assert!(
        activation_frame >= 1,
        "the match activated on GGRS frame {activation_frame}; a rewind cannot \
         reach a world before the session's own baseline, so this test would \
         prove nothing"
    );
    assert!(
        activated_on + 8 < 120,
        "the match activated at tick {activated_on}, too late for the run to \
         resimulate across it"
    );

    let stats = sim
        .rollback_execution_stats()
        .expect("GGRS instrumentation is installed");
    assert!(
        stats.lifetime_load_runs > 0,
        "nothing was ever rewound, so the checksum agreement above is agreement \
         with itself: {stats:?}"
    );

    let roster = seats(&mut sim);
    assert_eq!(
        roster,
        vec![(0, "blue".to_string()), (1, "red".to_string())],
        "the cast did not survive the rewind window it activated inside — a \
         seat, a team, or a whole body is missing"
    );
    let active = sim
        .world()
        .get_resource::<ActiveMatch>()
        .expect("the match is still live");
    assert_eq!(active.seats(), 2);
    assert_eq!(
        active.seat_topology(),
        Some(11),
        "the activation forgot which frozen topology decided it"
    );
}

/// The latch and the world must not be able to disagree.
///
/// `ActiveMatch::seats()` is *how many seats this match activated with* and
/// `match_participants` is *how many bodies are wearing one now*. The whole
/// point of keeping the count rather than a bool is that those two can be
/// COMPARED — so a rewind that dropped a fighter has to show up here rather
/// than as a countdown running against a cast of one.
#[test]
fn the_activation_count_still_matches_the_bodies_after_resimulation() {
    let mut sim = match_sim();
    introduce_the_roster(&mut sim);

    for _ in 0..120 {
        sim.step(AgentAction::default());
        let Some(seats_declared) = sim.world().get_resource::<ActiveMatch>().map(|m| m.seats())
        else {
            continue;
        };
        let world = sim.world_mut();
        let mut query = world.query::<&MatchSeat>();
        let bodies = query.iter(world).count();
        assert_eq!(
            seats_declared, bodies,
            "the activation says {seats_declared} seats and {bodies} bodies wear \
             one: the latch and the world disagree, which is exactly the state a \
             rewind across activation used to leave behind"
        );
    }
}
