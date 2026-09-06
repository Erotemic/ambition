//! Rollback coverage for match activation state.
//!
//! These tests verify that match roster/team state survives ordinary resimulation and
//! that activation counts agree with seated bodies. The fixture activates on the first
//! simulated frame, so it does not exercise rewinding to a pre-activation world; that
//! boundary requires a route whose activation occurs after rollback history exists.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::versus_match::{
    ActiveMatch, ControllerBinding, MatchParticipant, MatchParticipantRoster, MatchSeat,
};

/// A sync-test sim: every frame is saved, rewound and resimulated, so the
/// activation tick is inside a rollback window by construction rather than by a
/// forced rewind somebody has to remember to trigger.
fn match_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
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
        seating: ambition_platformer2d::actor::RosterSeating::activated_at(11),
        // A fixture's roster has no publisher: nothing else in this App claims
        // one, which is the case `None` is for.
        published_by: None,
        rules: ambition_platformer2d::versus_match::MatchRules {
            item_spawns: None,
            opens_suspended: true,
            // No ceremony in a rollback fixture: the stage that owns the opening
            // is not part of what these tests exercise.
            opening_countdown_ticks: 0,
            time_limit_ticks: 0,
            abilities: None,
            body: None,
            stocks: None,
            health_pool: None,
            ..Default::default()
        },
    }
}

fn seats(sim: &mut Platformer2dSimHarness) -> Vec<(usize, String)> {
    let world = sim.world_mut();
    let mut query = world.query::<(
        &MatchSeat,
        &ambition_platformer2d::combat::targeting::MatchTeam,
    )>();
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

/// Insert the roster after ordinary simulation has begun, then make that state
/// the rollback baseline. The test must exercise activation inside rollback
/// history without mutating the live world behind the rollback cursor.
fn introduce_the_roster(sim: &mut Platformer2dSimHarness) {
    for _ in 0..FRAMES_BEFORE_THE_ROSTER {
        sim.step(AgentAction::default());
    }
    sim.world_mut().insert_resource(two_cpu_roster());
    sim.rebase_rollback_history()
        .expect("the roster insert becomes the rollback baseline");
}

/// A match's roster and teams survive ordinary resimulation, checksum-clean.
///
/// not the reviews' acceptance condition (*rewinding around the first
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

// ── AC24 ─────────────────────────────────────────────────────────────────────
//
// The fixture the two tests above cannot be: one whose rollback window genuinely
// contains a PRE-activation frame.
//
// The blocker was never the assertions, it was the arrival time. The window and the event were
// adjacent and never overlapped.
//
// A roster derived from `SimTick`, which is itself registered rollback state, is neither: it is
// reconstructed identically on every replay of a given frame, so no rebase is needed and there
// is nothing behind the cursor to diverge.
//
// That also makes this the shipped shape rather than a test trick. The versus
// route's bodies are constructed several ticks after the route is entered, which
// is exactly "activation lands mid-window"; this reproduces that timing without
// needing the route.

use ambition_platformer2d::sim::{Platformer2dSimulationPhaseMonolith, SimScheduleExt};
use ambition_platformer2d::time::SimTick;
use bevy::prelude::{Commands, IntoScheduleConfigs, Res, ResMut, Resource};

/// Offset from the first observed rewind to roster activation.
///
/// This is relative to the rollback window, not global `SimTick`, because shell
/// activation advances the simulation clock before the rollback frame clock starts.
const ROSTER_ARRIVES_AFTER_THE_WINDOW_OPENS: u64 = 8;

/// The tick the roster is due on, decided once the window is known to be
/// live. Deliberately NOT rollback state: a rewind must not un-decide when the
/// roster was scheduled, or the replay would disagree with the original run
/// about a fact that is not simulation.
#[derive(Resource, Default)]
struct RosterDueAt(Option<u64>);

/// Every frame that was simulated, and whether the match was live at the end
/// of it.
///
/// Deliberately NOT rollback state, which is what makes it an instrument: it
/// keeps the entries a rewind erases from the world, so the log is a record of
/// what the session actually did rather than of what survived. A frame index
/// that appears twice is a resimulation, and a resimulated index lower than the
/// activation index is the crossing this fixture exists to produce.
#[derive(Resource, Default)]
struct ActivationTrace(Vec<(u64, bool)>);

/// The `else` branch is the load-bearing half and the easy one to omit: without
/// it the roster would persist through a rewind to a frame that never had one,
/// seating would activate early on the replay, and the sync test would report a
/// divergence that is the fixture's fault rather than the engine's.
fn the_roster_arrives_on_a_tick(mut commands: Commands, tick: Res<SimTick>, due: Res<RosterDueAt>) {
    if due.0.is_some_and(|due| tick.get() >= due) {
        commands.insert_resource(two_cpu_roster());
    } else {
        commands.remove_resource::<MatchParticipantRoster>();
    }
}

/// Runs in `Platformer2dSimulationPhaseMonolith::Trace`, after everything: `ActiveMatch` is published
/// through `Commands` during `PlayerInputSet::CharacterProjection`, so a reader
/// in that same set would record the tick before the one it activated on.
fn trace_the_activation(
    mut trace: ResMut<ActivationTrace>,
    tick: Res<SimTick>,
    active: Option<Res<ActiveMatch>>,
) {
    trace.0.push((tick.get(), active.is_some()));
}

fn late_arriving_roster_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            app.init_resource::<ActivationTrace>();
            app.init_resource::<RosterDueAt>();
            let sim = app.sim_schedule();
            app.add_systems(
                sim,
                (
                    the_roster_arrives_on_a_tick.before(
                        ambition_platformer2d::actors::character_runtime::prepare_the_match,
                    ),
                    trace_the_activation.in_set(Platformer2dSimulationPhaseMonolith::Trace),
                ),
            );
            Ok(())
        },
    )
    .expect("Ambition GGRS sync-test harness builds")
}

/// Step until the rollback window is DEMONSTRABLY open, then schedule the
/// roster just past that point.
///
/// the thing this replaces was a constant, and the constant was about the
/// wrong clock — see [`ROSTER_ARRIVES_AFTER_THE_WINDOW_OPENS`]. What proves the
/// window is open is a RESIMULATED frame: a tick the trace has already recorded
/// appearing again. Nothing else in reach distinguishes "GGRS is running" from
/// "GGRS exists but has never had to rewind", and the second one is the state
/// this fixture kept mistaking for the first.
fn schedule_the_roster_once_the_window_is_open(sim: &mut Platformer2dSimHarness) {
    let mut highest: Option<u64> = None;
    for step in 0..600 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("settling step {step}: {error}"));
        let mut rewound_at = None;
        {
            let trace = &sim.world_mut().resource::<ActivationTrace>().0;
            for (frame, _) in trace {
                match highest {
                    Some(top) if *frame < top => {
                        rewound_at = Some(top);
                        break;
                    }
                    Some(top) => highest = Some(top.max(*frame)),
                    None => highest = Some(*frame),
                }
            }
        }
        if let Some(top) = rewound_at {
            let due = top + ROSTER_ARRIVES_AFTER_THE_WINDOW_OPENS;
            sim.world_mut().insert_resource(RosterDueAt(Some(due)));
            // The trace so far is the SETTLING, not the run under test: keeping
            // it would make "the first frame with a live match" the wrong frame
            // and hand the crossing search a haystack of pre-roster history.
            sim.world_mut().resource_mut::<ActivationTrace>().0.clear();
            return;
        }
    }
    panic!(
        "600 steps and no frame was ever resimulated, so the sync-test window \
         never opened and this fixture cannot state its acceptance condition"
    );
}

/// A rewind that lands BEFORE the activation reconstructs the identical
/// match.
///
/// This is the reviews' acceptance condition, and the first fixture in the
/// repository that can state it honestly. The proof is not arithmetic about
/// `check_distance`: the trace records every frame the session simulated, so the
/// crossing is READ OFF the run — a frame index earlier than the activation
/// index, simulated after the activation index had already been reached.
///
/// It fails on the right assertion, too: the restored frame comes back carrying a live match,
/// so the world the run rewound to was not pre-activation after all.
#[test]
fn rewinds_across_the_activation_frame_and_reconstructs_the_same_match() {
    let mut sim = late_arriving_roster_sim();
    sim.world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
    schedule_the_roster_once_the_window_is_open(&mut sim);

    for tick in 0..(ROSTER_ARRIVES_AFTER_THE_WINDOW_OPENS as usize + 40) {
        sim.step(AgentAction::default());
        // Every tick, not once at the end: a divergence ON the activation frame
        // is repaired by later frames agreeing with each other, so a run that
        // only checks the end can watch the interesting frame go wrong and
        // report success.
        if let Err(error) = sim.rollback_health() {
            let audit = sim
                .world()
                .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
            let mut seen: Vec<String> = audit
                .divergences
                .iter()
                .map(|d| format!("{:?} {}", d.boundary, d.type_name))
                .collect();
            seen.sort();
            seen.dedup();
            panic!(
                "tick {tick}: {error}\n  comparisons={} divergent types: {seen:?}",
                audit.comparisons
            );
        }
    }

    let trace = std::mem::take(&mut sim.world_mut().resource_mut::<ActivationTrace>().0);
    let activated_at = trace
        .iter()
        .find(|(_, active)| *active)
        .map(|(frame, _)| *frame)
        .expect(
            "no roster seat ever produced a live match, so this test rewound a \
             world with no match in it and would have passed either way",
        );

    // THE CROSSING, observed. Walk the trace in the order the session executed
    // it: once a frame at or after the activation has been simulated, any later
    // entry for an earlier frame is a rewind that restored a world in which the
    // match did not yet exist.
    let mut reached_activation = false;
    let mut crossed_from = None;
    for (frame, _) in &trace {
        if *frame >= activated_at {
            reached_activation = true;
        } else if reached_activation {
            crossed_from = Some(*frame);
            break;
        }
    }
    let crossed_from = crossed_from.unwrap_or_else(|| {
        panic!(
            "no frame earlier than the activation (tick {activated_at}) was ever \
             resimulated, so this run never rewound ACROSS the activation and \
             proves exactly what the two fixtures above already proved. Trace: \
             {trace:?}"
        )
    });
    assert!(
        !trace
            .iter()
            .any(|(frame, active)| *frame == crossed_from && *active),
        "the frame this run rewound to (tick {crossed_from}) is recorded as \
         having a live match, so the restored world was not pre-activation \
         after all"
    );

    // …and the match that came back is the match that was there.
    let roster = seats(&mut sim);
    assert_eq!(
        roster,
        vec![(0, "blue".to_string()), (1, "red".to_string())],
        "the cast did not survive a rewind to before it existed"
    );
    let active = sim
        .world()
        .get_resource::<ActiveMatch>()
        .expect("the match is live again after the window closed");
    assert_eq!(active.seats(), 2);
    assert_eq!(
        active.seat_topology(),
        Some(11),
        "the reconstructed activation forgot which frozen topology decided it"
    );
}

/// A stocks fighter's PERCENT and death policy survive a real rewind.
///
/// `CanonicalCodecStrategy` uses that encoding for the STORED value, so this was not a checksum
/// omission: a fighter at 188% came back from any rewind as a fresh one, its knockback scaling
/// reset, and later damage began draining a pool the ruleset says only the world may empty.
///
/// (That probe originally read 3760%, against the seeded fighter's tiny pool; the fixture
/// authors a 100-point pool now so the number is legible.)
#[test]
fn a_fighters_percent_and_policy_survive_a_rewind() {
    use ambition_platformer2d::characters::actor::{BodyHealth, DeathPolicy};

    let mut sim = match_sim();
    introduce_the_roster(&mut sim);
    // Seat the match, then let it settle so the bodies exist and nothing is
    // mid-construction.
    for _ in 0..40 {
        sim.step(AgentAction::default());
    }

    // Put one seated fighter well past 100% under the stocks policy, and make
    // THAT the rollback baseline — a direct `world_mut` write behind the cursor
    // is the harness's one documented way to lie to itself.
    let (before_percent, before_policy) = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&mut BodyHealth, bevy::prelude::With<MatchSeat>>();
        let mut health = q
            .iter_mut(world)
            .next()
            .expect("the roster seated at least one fighter");
        health.set_policy(DeathPolicy::Unbounded);
        // A POOL THE NUMBER CAN BE READ AGAINST. The seeded fighter's pool
        // is tiny, so `damage(188)` against it reported *3760%* — correct for
        // what this asserts (percent is preserved, and percent is NOT health)
        // and nonsense to anybody who opens the file. 188 over 100 is a Smash
        // percentage a reader recognises, still above 100%, and still proving
        // the meter is not the pool.
        health.health.max = 100;
        health.damage(188);
        (health.damage_percent(), health.policy())
    };
    sim.rebase_rollback_history()
        .expect("the damaged fighter becomes the rollback baseline");
    assert!(
        before_percent > 1.0,
        "the fixture meant to put a fighter ABOVE 100%, and it is at {:.0}%",
        before_percent * 100.0
    );
    // pinned, not just bounded: the previous version asserted only `> 1.0` and
    // sat at 3760% for weeks because the seeded pool was tiny and nothing said
    // what the number was SUPPOSED to be. A reader opening this now sees the
    // fixture's intent, and a change to the seeded pool cannot quietly turn it
    // back into nonsense.
    assert!(
        (before_percent - 1.88).abs() < 1e-3,
        "188 damage over the 100-point pool this fixture authors is 188%, and it          is at {:.0}%",
        before_percent * 100.0
    );
    assert_eq!(before_policy, DeathPolicy::Unbounded);

    // Every frame of this sim is saved, rewound and resimulated.
    for tick in 0..30 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("tick {tick}: {error}"));
    }

    let (after_percent, after_policy) = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&BodyHealth, bevy::prelude::With<MatchSeat>>();
        let health = *q
            .iter(world)
            .next()
            .expect("the seated fighter is still there");
        (health.damage_percent(), health.policy())
    };
    assert!(
        (after_percent - before_percent).abs() < 1e-3,
        "the fighter went into the rewind at {:.0}% and came out at {:.0}%. The \
         meter is what knockback scales off, so this is a fighter that launches \
         like a fresh one after every rollback",
        before_percent * 100.0,
        after_percent * 100.0
    );
    assert_eq!(
        after_policy,
        DeathPolicy::Unbounded,
        "the death policy came back as the default, so damage now drains this \
         body's pool and it can die by HP in a ruleset where only the world kills"
    );
}

/// Two LOCAL SEATS under a rollback host, driven independently.
///
/// under a sync test EVERY frame is saved, rewound and resimulated, so this is
/// not "two seats moved": it is two seats whose inputs survive being replayed.
/// A seat whose frame is authored outside the rollback's input path diverges on
/// the first rewind, and `rollback_health` says so on the tick it happens.
#[test]
fn two_local_seats_drive_independently_under_a_rollback_host() {
    use ambition_platformer2d::engine_core::BodyKinematics;
    use ambition_platformer2d::input::ControlFrame;

    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10)
            // THE MATCH OWNS THE STAGE. Without this the composition also lowers Ambition's
            // home avatar, and that avatar already holds the session's control channel — so a LOCAL
            // seat asking for one is a second claimant and `prepare_match` refuses the roster by
            // name.
            .seating_a_match()
            // The session must actually CARRY seat two, or its authored frames
            // are written and never asked for — inert rather than wrong, and a
            // test that did not say this would pass while proving nothing.
            .with_rollback_players(2),
    )
    .expect("a two-player sync-test harness builds");

    for _ in 0..FRAMES_BEFORE_THE_ROSTER {
        sim.step(AgentAction::default());
    }
    let human = |character: &str, slot: u8, team: &str| {
        MatchParticipant::new(character)
            .driven_by(ControllerBinding::Human {
                source: ambition_platformer2d::actor::LocalInputSource::Pad(slot),
            })
            .on_team(team)
    };
    sim.world_mut().insert_resource(MatchParticipantRoster {
        participants: vec![
            human("player_robot_v3", 0, "blue"),
            human("player_robot_v2", 1, "red"),
        ],
        seating: ambition_platformer2d::actor::RosterSeating::default(),
        published_by: None,
        rules: ambition_platformer2d::versus_match::MatchRules {
            item_spawns: None,
            // Not suspended: this test is about INPUT reaching a body, and an
            // opening hold would keep both fighters still and pass for the wrong
            // reason.
            opens_suspended: false,
            // No ceremony in a rollback fixture: the stage that owns the opening
            // is not part of what these tests exercise.
            opening_countdown_ticks: 0,
            time_limit_ticks: 0,
            abilities: None,
            body: None,
            stocks: None,
            health_pool: None,
            ..Default::default()
        },
    });
    sim.rebase_rollback_history()
        .expect("the roster insert becomes the rollback baseline");

    let mut seated = Vec::new();
    for tick in 0..120 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("tick {tick}: {error}"));
        let world = sim.world_mut();
        let mut q = world.query::<(bevy::prelude::Entity, &MatchSeat)>();
        seated = q.iter(world).map(|(e, s)| (s.0, e)).collect();
        seated.sort_by_key(|(slot, _)| *slot);
        if seated.len() >= 2 {
            break;
        }
    }
    assert_eq!(
        seated.len(),
        2,
        "a two-human roster seated {} bodies under a rollback host",
        seated.len()
    );

    // THE SEAT SET SURVIVES RESIMULATION. Every frame here is saved, rewound
    // and resimulated, and activation is inside that window because the roster
    // insert was rebased — so this is the two-human form of *"rewinding around
    // the first active frame reconstructs the identical roster"*, checked on
    // every tick rather than at the end.
    //
    // the SLOTS AND WHO IS IN THEM, not just the count. A resim that rebuilt
    // two seats numbered 0 and 0, or swapped which body wore which, would keep
    // the count and lose the match.
    //
    // A rewind despawns and respawns the cast, Bevy hands out freed indices LIFO, so the second
    // body legitimately comes back wearing the first one's old index. `Entity` is not identity
    // across a rewind; `Rollback` is, and what this test actually means by "which body wore
    // which" is the fighter.
    let worn = |sim: &mut Platformer2dSimHarness| -> Vec<(usize, String)> {
        let world = sim.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            &ambition_platformer2d::characters::actor::WornCharacter,
        )>();
        let mut rows: Vec<(usize, String)> = q
            .iter(world)
            .map(|(seat, worn)| (seat.0, worn.id().to_string()))
            .collect();
        rows.sort();
        rows
    };
    let cast = worn(&mut sim);
    assert_eq!(
        cast.len(),
        2,
        "the two seated bodies must each wear a character; got {cast:?}"
    );
    for tick in 0..30 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("holding the seats, tick {tick}: {error}"));
        assert_eq!(
            worn(&mut sim),
            cast,
            "tick {tick}: the seat set changed under resimulation — two humans \
             seated at activation must be the same two fighters in the same \
             slots after every rewind"
        );
    }

    // resolved from the SEAT every time, never cached. A cached `Entity`
    // survives a rewind as a stale handle that may now name the other fighter —
    // the same recycling that broke the assertion above, one step more subtle
    // because it reads a plausible number instead of failing.
    let x = |sim: &mut Platformer2dSimHarness, slot: usize| {
        let world = sim.world_mut();
        let mut q = world.query::<(&MatchSeat, &BodyKinematics)>();
        q.iter(world)
            .find(|(seat, _)| seat.0 == slot)
            .map(|(_, kin)| kin.pos.x)
            .expect("a seated body has kinematics")
    };
    let (start_one, start_two) = (x(&mut sim, 0), x(&mut sim, 1));

    // Seat TWO walks right, through the seat-frame seam. Seat one is untouched.
    for tick in 0..40 {
        sim.drive_seat(
            1,
            ControlFrame {
                axis_x: 1.0,
                ..Default::default()
            },
        );
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("driving seat two, tick {tick}: {error}"));
    }
    let moved_two = x(&mut sim, 1) - start_two;
    let moved_one = x(&mut sim, 0) - start_one;
    assert!(
        moved_two.abs() > 1.0,
        "seat two authored 40 frames of right and its fighter moved {moved_two:.2}px \
         — the seat-frame seam does not reach a seated body under this host"
    );
    assert!(
        moved_one.abs() < moved_two.abs() * 0.25,
        "seat two's input moved seat ONE's fighter ({moved_one:.2}px against \
         {moved_two:.2}px): the two seats share an input path"
    );
}

/// `count` HUMAN participants, from the ids a plain sandbox actually prepares.
fn human_roster(count: usize) -> MatchParticipantRoster {
    let ids = ["player_robot_v3", "player_robot_v2"];
    let teams = ["blue", "red"];
    MatchParticipantRoster {
        participants: (0..count)
            .map(|slot| {
                MatchParticipant::new(ids[slot])
                    .driven_by(ControllerBinding::Human {
                        source: ambition_platformer2d::actor::LocalInputSource::Pad(slot as u8),
                    })
                    .on_team(teams[slot])
            })
            .collect(),
        seating: ambition_platformer2d::actor::RosterSeating::default(),
        published_by: None,
        rules: ambition_platformer2d::versus_match::MatchRules {
            item_spawns: None,
            opens_suspended: false,
            // No ceremony in a rollback fixture: the stage that owns the opening
            // is not part of what these tests exercise.
            opening_countdown_ticks: 0,
            time_limit_ticks: 0,
            abilities: None,
            body: None,
            stocks: None,
            health_pool: None,
            ..Default::default()
        },
    }
}

/// Seating is one-shot. Keep the roster fixed for the session; this fixture
/// varies activation state without introducing unsupported mid-match roster edits.
///
/// The roster is a pure function of `SimTick` in the same shape the
/// activation fixture uses — absent, then present — and what is new here is that
/// its participants are HUMAN.
fn the_human_roster_arrives_on_a_tick(
    mut commands: Commands,
    tick: Res<SimTick>,
    due: Res<RosterDueAt>,
) {
    if due.0.is_some_and(|due| tick.get() >= due) {
        commands.insert_resource(human_roster(2));
    } else {
        commands.remove_resource::<MatchParticipantRoster>();
    }
}

fn late_arriving_human_roster_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10)
            // THE MATCH OWNS THE STAGE. Without this the composition also lowers Ambition's
            // home avatar, and that avatar already holds the session's control channel — so a LOCAL
            // seat asking for one is a second claimant and `prepare_match` refuses the roster by
            // name.
            .seating_a_match()
            // The session must CARRY seat two, or the second seat's frames are
            // authored and never asked for.
            .with_rollback_players(2),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            app.init_resource::<ActivationTrace>();
            app.init_resource::<RosterDueAt>();
            let sim = app.sim_schedule();
            app.add_systems(
                sim,
                (
                    the_human_roster_arrives_on_a_tick.before(
                        ambition_platformer2d::actors::character_runtime::prepare_the_match,
                    ),
                    // The trace is not decoration here: the settling helper reads
                    // it to decide when the rollback window is demonstrably open.
                    trace_the_activation.in_set(Platformer2dSimulationPhaseMonolith::Trace),
                ),
            );
            Ok(())
        },
    )
    .expect("Ambition GGRS sync-test harness builds")
}

/// A rewind across the activation of a TWO-HUMAN match reconstructs both
/// seats. AC24's shape, one seat further.
///
/// `rewinds_across_the_activation_frame_and_reconstructs_the_same_match` proves
/// this for two CPU participants. Human seats are the interesting case and the
/// one couch multiplayer actually ships: they are the participants that carry a
/// `device_slot`, that the frozen seat topology sizes the session for, and whose
/// per-seat input latches the rollback has to hold. A CPU seat exercises none of
/// that.
///
/// Under a sync test every frame is saved, rewound and resimulated, so the
/// activation frame is inside a rollback window by construction.
/// `rollback_health` is asserted on every tick, so a divergence is reported on
/// the tick it happens rather than as a confusing count later.
#[test]
fn rewinds_across_a_two_human_activation_and_reconstructs_both_seats() {
    let mut sim = late_arriving_human_roster_sim();
    schedule_the_roster_once_the_window_is_open(&mut sim);

    let seat_slots = |sim: &mut Platformer2dSimHarness| -> Vec<usize> {
        let world = sim.world_mut();
        let mut query = world.query::<&MatchSeat>();
        let mut slots: Vec<usize> = query.iter(world).map(|seat| seat.0).collect();
        slots.sort();
        slots
    };

    let mut saw_the_unseated_phase = false;
    for tick in 0..(ROSTER_ARRIVES_AFTER_THE_WINDOW_OPENS as usize + 40) {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("tick {tick}: {error}"));
        if seat_slots(&mut sim).is_empty() {
            saw_the_unseated_phase = true;
        }
    }

    assert!(
        saw_the_unseated_phase,
        "the fixture never observed the pre-roster phase, so there was no \
         activation to rewind across and the assertion below would hold on a \
         session that was seated from tick zero"
    );

    // The SLOTS, not the count. A resimulation that rebuilt two seats both
    // numbered 0 keeps the count and loses the match.
    assert_eq!(
        seat_slots(&mut sim),
        vec![0, 1],
        "a two-human match did not come back from the rewind as seats 0 and 1"
    );

    for tick in 0..20 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("holding the seats, tick {tick}: {error}"));
        assert_eq!(
            seat_slots(&mut sim),
            vec![0, 1],
            "tick {tick}: the seat set changed under continued resimulation"
        );
    }
}
