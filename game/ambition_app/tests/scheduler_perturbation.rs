#![cfg(feature = "rl_sim")]
//! **Adding an unrelated READER must not change what the simulation computes.**
//! (AC2)
//!
//! A Bevy schedule is a partial order. Two systems with no declared edge between
//! them run in whatever order the executor picks, and that choice is not stable
//! against unrelated changes: inserting a third system can re-partition the
//! graph and flip which of the two goes first. Where a real dependency was left
//! implicit — "it happens to run after the writer" — the simulation is correct by
//! accident, and the accident is one unrelated system away from ending.
//!
//! That is a *deterministic* wrongness, not a flaky one, which is what makes it
//! expensive: every run agrees, so nothing looks broken until the day the graph
//! changes and every run agrees on a different answer.
//!
//! ⭐ **so this compares two GRAPHS, not two runs of one graph.** The desync
//! canary next door already proves a graph is self-consistent: `SyncTestSession`
//! saves, advances, rewinds and resimulates with the same inputs, and the
//! checksums match. It cannot see this defect at all, because both sides of its
//! comparison are the same schedule. Here, side B is the same simulation plus
//! benign systems that read real state and mutate nothing.
//!
//! ⚠ **what the perturbation systems must be, and why each clause matters:**
//!
//! - they must ACTUALLY EXECUTE — a system filtered out by a run condition
//!   perturbs nothing, and the comparison would then be two identical graphs
//!   agreeing about nothing. `the_perturbation_systems_really_ran` is the check;
//! - they must READ REAL SIMULATION STATE, so their nodes carry genuine
//!   component accesses the executor must schedule around. A system taking no
//!   parameters conflicts with nothing and can be placed anywhere for free;
//! - they must MUTATE NO SIMULATION STATE, or a divergence would be their own
//!   fault rather than evidence about the graph;
//! - they must NOT be ordered against the writers they probe, because the
//!   ordering is exactly the thing under test.
//!
//! ⚠ **what the falsification below does and does not establish, stated plainly
//! because the difference matters.** `the_comparison_can_actually_see_a_divergence`
//! proves the DIGEST is live: a system that nudges every body's velocity makes
//! the two runs differ, so the comparison is reading the simulation and not
//! comparing two empty strings. It does NOT prove that these three particular
//! probes are capable of flipping any specific pair of systems — nothing can
//! prove that except a real ordering defect, and the graph is currently clean.
//! ⇒ so read a GREEN result as *"no implicit ordering was disturbed by this
//! perturbation"*, which is a real claim and a weaker one than *"the graph
//! contains no implicit ordering"*. The value grows as AC3 moves state: the same
//! probes then run against a graph whose writers have changed owners.
//!
//! ⛔ **if this goes red, the repair is the semantic dependency, not the leaf.**
//! Ordering the benign probe against whatever it perturbed would make the test
//! green while leaving the real pair of systems as unordered as they were. The
//! fix belongs in the phase/set membership or the dataflow of the two systems
//! that actually disagree.

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith as Phase;
use bevy::prelude::*;

/// How long to run each graph. Long enough for enemies to wake, chase, swing and
/// take damage — the reaction timers are where an ordering flip shows up first.
const FRAMES: usize = 180;

/// Counts each perturbation system's executions, so "they ran" is measured
/// rather than assumed. Not simulation state: nothing reads it back.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
struct ProbeRuns {
    early: u64,
    combat: u64,
    late: u64,
}

fn sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("combat_calibration_lab"),
    )
    .expect("the calibration lab sim builds")
}

/// A scripted input stream with fighting in it. A body standing still exercises
/// none of the ordering this is about.
fn scripted(frame: usize) -> AgentAction {
    AgentAction {
        move_x: if frame % 24 < 12 { 1.0 } else { -1.0 },
        jump: frame % 17 == 0,
        jump_held: frame % 17 < 5,
        dash: frame % 29 == 3,
        attack: frame % 11 == 2,
        projectile: frame % 13 == 4,
        ..AgentAction::default()
    }
}

/// **The authoritative-state digest.**
///
/// Every body's position, velocity, health and reaction timers, keyed by a
/// stable identity and SORTED — so the digest is a function of the simulation's
/// state and not of the order a query happened to visit it in. (An
/// order-sensitive digest would report a divergence for a difference that is not
/// one, which is the failure mode that makes a guard like this get deleted.)
///
/// The reaction timers are in deliberately: they are the facts AC3 is about to
/// move, and a fact whose owner changes is exactly where an implicit ordering
/// assumption goes unnoticed.
fn digest(sim: &mut Platformer2dSimHarness) -> String {
    let world = sim.world_mut();
    let mut rows: Vec<String> = world
        .query::<(
            &ambition_platformer2d::actors::features::BodyKinematics,
            &BodyHealth,
            Option<&BodyCombat>,
        )>()
        .iter(world)
        .map(|(kin, health, combat)| {
            let c = combat.copied().unwrap_or_default();
            format!(
                "{:.6},{:.6}|{:.6},{:.6}|{}|{:.6},{:.6},{:.6},{:.6}",
                kin.pos.x,
                kin.pos.y,
                kin.vel.x,
                kin.vel.y,
                health.current(),
                c.damage_invuln_timer,
                c.hitstun_timer,
                c.recoil_lock_timer,
                c.hitstop_timer,
            )
        })
        .collect();
    rows.sort();
    rows.join("\n")
}

/// Run `FRAMES` steps and return the per-frame digests.
fn run(mut sim: Platformer2dSimHarness) -> (Vec<String>, Platformer2dSimHarness) {
    let digests = (0..FRAMES)
        .map(|frame| {
            sim.step(scripted(frame));
            digest(&mut sim)
        })
        .collect();
    (digests, sim)
}

/// Install the benign readers. Each takes a real query or resource, counts
/// itself, and writes nothing the simulation reads.
///
/// ⚠ **no `.before`/`.after` against anything.** They are placed by PHASE only —
/// which is the coarsest placement that puts them inside the region under test,
/// and leaves the executor free to interleave them with the phase's own systems
/// however it likes. That freedom is the perturbation.
fn perturb(sim: &mut Platformer2dSimHarness) {
    use ambition_platformer2d::platformer::schedule::SimScheduleExt as _;
    let app = sim.app_mut();
    app.init_resource::<ProbeRuns>();
    let schedule = app.sim_schedule();

    fn read_bodies_early(
        bodies: Query<(&ambition_platformer2d::actors::features::BodyKinematics, &BodyHealth)>,
        mut runs: ResMut<ProbeRuns>,
    ) {
        let mut seen = 0u64;
        for (kin, health) in &bodies {
            // Touch the values so the read cannot be optimised into nothing.
            seen = seen.wrapping_add((kin.pos.x.to_bits() ^ health.current() as u32) as u64);
        }
        runs.early = runs.early.wrapping_add(1).wrapping_add(seen & 0);
    }

    fn read_combat_state(bodies: Query<&BodyCombat>, mut runs: ResMut<ProbeRuns>) {
        let mut seen = 0u64;
        for combat in &bodies {
            seen = seen.wrapping_add(combat.hitstop_timer.to_bits() as u64);
        }
        runs.combat = runs.combat.wrapping_add(1).wrapping_add(seen & 0);
    }

    fn read_time_late(
        time: Res<ambition_platformer2d::time::WorldTime>,
        mut runs: ResMut<ProbeRuns>,
    ) {
        let seen = time.scaled_dt.to_bits() as u64;
        runs.late = runs.late.wrapping_add(1).wrapping_add(seen & 0);
    }

    app.add_systems(schedule, read_bodies_early.in_set(Phase::PlayerSimulation));
    app.add_systems(schedule, read_combat_state.in_set(Phase::Combat));
    app.add_systems(schedule, read_time_late.in_set(Phase::Progression));
}

/// **The guard.** Same inputs, same content, two graphs — one with three extra
/// readers in it. Every frame's authoritative state must agree.
#[test]
fn inserting_unrelated_readers_does_not_change_the_simulation() {
    let (plain, _plain_sim) = run(sim());

    let mut perturbed_sim = sim();
    perturb(&mut perturbed_sim);
    let (perturbed, perturbed_sim) = run(perturbed_sim);

    let runs = *perturbed_sim.world().resource::<ProbeRuns>();
    assert!(
        runs.early > 0 && runs.combat > 0 && runs.late > 0,
        "a perturbation system never executed, so graph B is not perturbed and \
         this comparison is two identical graphs agreeing about nothing: {runs:?}"
    );

    if let Some((frame, (a, b))) = plain
        .iter()
        .zip(perturbed.iter())
        .enumerate()
        .find(|(_, (a, b))| a != b)
        .map(|(frame, pair)| (frame, pair))
    {
        panic!(
            "the simulation computed a different world once three systems that \
             READ state and write none of it were added to the graph — so \
             something in the simulation depends on an execution order nobody \
             declared, and it is one unrelated system away from changing.\n\
             \n\
             First divergence at frame {frame}.\n\
             \n\
             ⛔ do NOT repair this by ordering the probe systems. They are the \
             instrument. Find the two real systems whose order flipped and give \
             them the phase membership or dataflow dependency they always \
             needed.\n\
             \n\
             plain:\n{a}\n\nperturbed:\n{b}"
        );
    }
}

/// **The instrument can move.**
///
/// ⛔ a guard that compares two things which cannot differ is a guard that will
/// pass forever, including on the day it should not. This drives the same
/// comparison with a perturbation system that WRITES — the one property the real
/// probes are forbidden — and requires the digests to diverge. It is the
/// demonstration AC2 asks for, kept rather than deleted because it is cheap and
/// it is the only thing standing between the test above and a green tautology.
#[test]
fn the_comparison_can_actually_see_a_divergence() {
    let (plain, _) = run(sim());

    let mut poisoned = sim();
    {
        use ambition_platformer2d::platformer::schedule::SimScheduleExt as _;
        let app = poisoned.app_mut();
        let schedule = app.sim_schedule();
        // A real conflicting writer: it nudges every body's velocity. This is
        // precisely what the benign probes must never do.
        fn shove_every_body(
            mut bodies: Query<&mut ambition_platformer2d::actors::features::BodyKinematics>,
        ) {
            for mut kin in &mut bodies {
                kin.vel.x += 0.001;
            }
        }
        app.add_systems(schedule, shove_every_body.in_set(Phase::Combat));
    }
    let (poisoned, _) = run(poisoned);

    assert_ne!(
        plain, poisoned,
        "a system that mutates every body's velocity every frame produced a \
         bit-identical world, so the digest is not reading the simulation and \
         the guard above proves nothing"
    );
}
