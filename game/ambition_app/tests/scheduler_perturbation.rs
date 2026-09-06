#![cfg(feature = "rl_sim")]
//! Verify that adding benign read-only systems does not change simulation results.
//!
//! Bevy schedules are partial orders, so an undeclared writer dependency can change
//! behavior when unrelated nodes alter graph topology. The perturbed graph adds systems
//! that execute, read real simulation state, and mutate nothing. A divergence therefore
//! identifies an implicit simulation ordering dependency; the repair belongs between the
//! actual writers/readers, not between the diagnostic probes.

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith as Phase;
use bevy::prelude::*;

/// How long to run each graph. Long enough for enemies to wake, chase, swing and
/// take damage — the reaction timers are where an ordering flip shows up first.
const FRAMES: usize = 180;

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

/// The authoritative-state digest.
///
/// Every body's position, velocity, health and reaction timers, keyed by a stable identity and
/// SORTED — so the digest is a function of the simulation's state and not of the order a query
/// happened to visit it in.
///
/// The reaction timers are in deliberately: they are the facts AC3 is about to
/// move, and a fact whose owner changes is exactly where an implicit ordering
/// assumption goes unnoticed.
fn digest(sim: &mut Platformer2dSimHarness) -> String {
    let world = sim.world_mut();
    let mut rows: Vec<String> = world
        .query::<(
            &ambition_platformer2d::actor::BodyKinematics,
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
/// no `.before`/`.after` against anything. They are placed by PHASE only —
/// which is the coarsest placement that puts them inside the region under test,
/// and leaves the executor free to interleave them with the phase's own systems
/// however it likes. That freedom is the perturbation.
fn perturb(sim: &mut Platformer2dSimHarness) {
    use ambition_platformer2d::platformer::schedule::SimScheduleExt as _;
    let app = sim.app_mut();
    app.init_resource::<ProbeRuns>();
    let schedule = app.sim_schedule();

    fn read_bodies_early(
        bodies: Query<(&ambition_platformer2d::actor::BodyKinematics, &BodyHealth)>,
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

/// The guard. Same inputs, same content, two graphs — one with three extra
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

/// The instrument can move.
///
/// a guard that compares two things which cannot differ is a guard that will
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
            mut bodies: Query<&mut ambition_platformer2d::actor::BodyKinematics>,
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
