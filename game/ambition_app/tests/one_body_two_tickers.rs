//! ⛔⛔ IS ANY BODY ADVANCED TWICE IN ONE TICK, BY TWO DIFFERENT PHASES? Measured
//! BEFORE A4 splits control from body execution, because it cannot be measured
//! after.
//!
//! `boot_budget::no_system_is_registered_twice_in_one_schedule` catches the same
//! system registered twice. `accepted-control-writer-map.md:242` names the gap it
//! leaves: *"it cannot see one BODY ticked by two different systems, which is the
//! failure a control/execution regrouping would actually produce."*
//!
//! ⛔ **THE TIMING IS THE ARGUMENT.** A second writer measured now is a
//! regression against a known baseline. Measured after the extraction it is
//! indistinguishable from the new design, and the question stops being
//! unanswered and becomes UNANSWERABLE.
//!
//! ⭐ **THIS REPORTS. IT DOES NOT GATE.** A baseline is the deliverable; a gate
//! wants a full block and a triaged population, and a half-landed guard is worse
//! than none because the next reader takes a green lane as a checked one.
//!
//! ## How it attributes a write to a phase
//!
//! `PlatformerRuntimeSet` already names the phases A4 will split — `ControlInput`
//! and `ActorSimulation` among them. A probe runs after each phase and records
//! every body whose `BodyKinematics` change tick has MOVED since the previous
//! probe in the same tick. A body credited to two phases was advanced twice.
//!
//! ⛔ **CHANGE TICKS, NOT `is_changed()`.** A probe runs once per tick, so
//! `is_changed()` on it means *"changed since the previous TICK"* — which every
//! moving body is, every tick. `Ref::last_changed()` compared against the value
//! this probe stored a phase ago is the question actually being asked.
//!
//! ⚠ **ECS STATE, NOT A STATIC.** Bevy runs systems on task-pool threads, so a
//! thread-local counter reads zero against a process-global reality. The witness
//! is a `Resource`.
//!
//! ## ⭐⭐ THE BASELINE, MEASURED 2026-09-10 — sandbox composition, 2 bodies, 120 ticks
//!
//! ```text
//! mutable BORROWS   PlayerInput 119 · WorldPrep 120 · WorldPrep/Integrate 240
//!                   WorldPrep/AfterIntegrate 120 · PlayerSimulation 120
//! position CHANGES  WorldPrep/Integrate 123   (123 total, 100%)
//! DOUBLE ADVANCES   0
//! ```
//!
//! ⭐ **THE ATTRIBUTION IS CHECKABLE AGAINST A FACT THE INSTRUMENT DOES NOT
//! KNOW.** `integrate_sim_bodies` is registered `.in_set(WorldPrepSet::Integrate)`
//! (`actor_monolith/src/features/mod.rs:1113`), and 100% of position changes
//! land in `WorldPrep/Integrate`. A discriminating instrument is not enough — it
//! has to discriminate CORRECTLY, and that correspondence is the evidence.
//!
//! ⛔ **FIVE PHASES TAKE A MUTABLE BORROW OF EVERY BODY EVERY TICK AND ONE MOVES
//! IT.** `PlayerSimulation` borrows 120 times and changes nothing. Bevy marks a
//! component changed on any mutable DEREFERENCE, so a borrow count is not a
//! write count — and the writer map's *"38 write-capable sites"* is a count of
//! the first kind.
//!
//! ⚠ **THE POPULATION IS PART OF THE RESULT.** A double advance that needs a
//! mount, a possession, a rider or four seats is OUTSIDE this run. Say "the
//! sandbox composition, 2 bodies, 120 ticks" or the number grows a scope it
//! never had.
//!
//! ⚠ **IT CANNOT SEE TWO WRITES INSIDE ONE PHASE.** Two systems in
//! `ActorSimulation` both advancing one body is invisible here, and that is a
//! real limit: the probe granularity is the phase, because the phase is what A4
//! moves. A per-SYSTEM answer needs a probe after every writer, and Bevy 0.19
//! does not expose a system's component access outside the crate
//! (`SystemWithAccess::access` is `pub(crate)`), so the writer set cannot be
//! derived from the schedule at all — see the report at the end.

// ⚠ GATED, AND THE GATE IS DEFAULT-ON. `rl_sim` is in `default` through
// `desktop_dev`, so an ordinary `cargo test` compiles and runs this. A
// feature-gated test module that nothing enables runs NONE of its tests and
// says so nowhere — the gate is declared here only because
// `Platformer2dSimHarness` lives behind it.
#![cfg(feature = "rl_sim")]
#![allow(clippy::type_complexity)]

use std::collections::BTreeMap;

use ambition_app::AmbitionSim;
use ambition_app::{Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::engine_core::{BodyKinematics, ControlFrame};
use ambition_platformer2d::sim::{Platformer2dSimulationPhaseMonolith as Phase, SimScheduleExt};
use ambition_platformer2d::platformer::schedule::WorldPrepSet;
use bevy::ecs::change_detection::Tick;
use bevy::prelude::*;

/// The phases probed, in schedule order.
///
/// ⛔⛔ THESE ARE `Platformer2dSimulationPhaseMonolith`, AND THE FIRST VERSION OF
/// THIS FILE PROBED `PlatformerRuntimeSet` INSTEAD — WHICH HAS NO MEMBERS.
///
/// MEASURED 2026-09-10: `in_set(PlatformerRuntimeSet::..)` appears **zero**
/// times in `crates/` and `game/`, and nothing calls `configure_sets` on it. It
/// is the *"reusable runtime vocabulary that future crates should depend on"*
/// (`actor_monolith/src/schedule/schedule.rs:5`), and
/// `Platformer2dSimulationPhaseMonolith` is *"the app-level realization"*. The
/// vocabulary is aspirational; the realization is what systems join.
///
/// ⇒ **`.after(<a set with no members>)` IS A SILENT NO-OP.** It constrains
/// nothing, the scheduler places the system anywhere, and an instrument built on
/// it reports a confident number about a probe that ran at an arbitrary moment.
/// That is exactly what happened: 240 of 240 writes credited to whichever probe
/// the scheduler happened to run first, and **zero double ticks — which read as
/// a clean baseline.**
///
/// ⚠ An empty `SystemSet` is not a compile error, not a warning, and not
/// visible at the call site. The only symptom is a number that is too tidy.
/// ⛔⛔ AND THE ENUM'S DECLARATION ORDER IS NOT THE SCHEDULE ORDER. `WorldPrep`
/// is declared BEFORE `PlayerInput` and runs AFTER it. The real chain is at
/// `actor_monolith/src/schedule/schedule.rs:91` —
/// `PlayerInput → WorldPrep → PlayerSimulation → RoomTransition → Combat →
/// PresentationSync`, chained inside `CoreSimulation`.
///
/// ⇒ Probing in declaration order asked the scheduler for a CYCLE
/// (`WorldPrep → probe → PlayerInput → WorldPrep`) and Bevy panicked at
/// initialization. ⭐ That panic is the friendly outcome: penning each probe
/// between two phases makes a wrong order a hard failure, where `.after` alone
/// had silently accepted it and produced a number.
const PHASES: &[(&str, Phase)] = &[
    // ⭐ THE SEAM A4 SPLITS: input projection, then body execution.
    ("PlayerInput", Phase::PlayerInput),
    ("WorldPrep", Phase::WorldPrep),
    ("PlayerSimulation", Phase::PlayerSimulation),
    // ⛔⛔ AND THE PHASE NAMES ARE MISLEADING, WHICH THE FIRST MEASUREMENT
    // SHOWED. `PlayerSimulation` takes a mutable borrow of every body every tick
    // and CHANGES NO POSITION. `WorldPrep` is where bodies actually move:
    // `integrate_sim_bodies` is in `WorldPrepSet::Integrate`
    // (`actor_monolith/src/features/mod.rs:1113`), and `WorldPrepSet`'s three
    // sub-phases are chained inside `Phase::WorldPrep`
    // (`actor_monolith/src/schedule/schedule.rs:141`).
    //
    // ⇒ THE EXECUTION SUB-STRUCTURE A4 CARES ABOUT IS INSIDE ONE PHASE, so
    // probing the six outer phases alone would report a clean zero while every
    // body write happened in a single unopened box.
    ("RoomTransition", Phase::RoomTransition),
    ("Combat", Phase::Combat),
    ("PresentationSync", Phase::PresentationSync),
];

/// Per-body evidence, in the world rather than in a `static`.
#[derive(Resource, Default)]
struct TickWitness {
    /// The change tick AND the position this witness last saw for each body.
    ///
    /// ⛔⛔ THE POSITION IS NOT REDUNDANT. Bevy marks a component changed on any
    /// mutable DEREFERENCE, so a system that takes `&mut BodyKinematics` and
    /// writes nothing still moves the change tick. ⇒ The tick answers *"which
    /// phases took a mutable borrow"*; the value answers *"which phases moved
    /// the body"*, and only the second is what "advanced" means.
    seen: BTreeMap<Entity, (Tick, Vec2)>,
    /// Phases that advanced each body, this tick.
    this_tick: BTreeMap<Entity, Vec<&'static str>>,
    /// ⭐ THE FINDING: every (body, phases) pair where more than one phase wrote.
    doubled: Vec<(Entity, Vec<&'static str>)>,
    /// Which phase each attributed write landed in.
    ///
    /// ⛔⛔ WITHOUT THIS, A ZERO IS UNREADABLE. The first run reported exactly
    /// one write per body per tick and no doubles — which is what a working
    /// instrument reports AND what an instrument that credits every write to a
    /// single phase reports. The distribution is what separates them.
    per_phase: BTreeMap<&'static str, usize>,
    /// The same, counting only phases that CHANGED THE POSITION.
    moved_per_phase: BTreeMap<&'static str, usize>,
    moves_attributed: usize,
    /// ⛔ THE ANTI-VACUITY FLOOR'S SUBJECT. Bodies observed at all, and writes
    /// attributed at all. Zero of either makes "no body ticked twice" a true
    /// statement about nothing.
    bodies_seen: usize,
    writes_attributed: usize,
    ticks: usize,
}

fn probe(phase: &'static str) -> impl Fn(Query<(Entity, Ref<BodyKinematics>)>, ResMut<TickWitness>)
{
    move |bodies: Query<(Entity, Ref<BodyKinematics>)>, mut witness: ResMut<TickWitness>| {
        let mut count = 0usize;
        for (entity, kin) in &bodies {
            count += 1;
            let now = (kin.last_changed(), kin.pos);
            let (borrowed, advanced) = match witness.seen.get(&entity) {
                // First sighting: record, do not credit. A body's first observed
                // change tick says nothing about WHICH phase produced it.
                None => (false, false),
                Some((tick, pos)) => (*tick != now.0, *pos != now.1),
            };
            witness.seen.insert(entity, now);
            if borrowed {
                witness.writes_attributed += 1;
                *witness.per_phase.entry(phase).or_default() += 1;
            }
            if advanced {
                witness.moves_attributed += 1;
                *witness.moved_per_phase.entry(phase).or_default() += 1;
                witness.this_tick.entry(entity).or_default().push(phase);
            }
        }
        witness.bodies_seen = witness.bodies_seen.max(count);
    }
}

/// Runs after every phase: harvests the tick and resets the per-tick record.
///
/// ⚠ IT REFRESHES `seen` FOR EVERY BODY, and that is not bookkeeping. Anything
/// that writes `BodyKinematics` between this system and the NEXT tick's first
/// probe — a system outside the sim schedule — would otherwise be credited to
/// `WorldPrep`, the first phase probed. That is a write the sim schedule did not
/// make, attributed to a phase that did not make it.
fn close_the_tick(bodies: Query<(Entity, Ref<BodyKinematics>)>, mut witness: ResMut<TickWitness>) {
    witness.ticks += 1;
    let doubled: Vec<(Entity, Vec<&'static str>)> = witness
        .this_tick
        .iter()
        .filter(|(_, phases)| phases.len() > 1)
        .map(|(e, p)| (*e, p.clone()))
        .collect();
    witness.doubled.extend(doubled);
    witness.this_tick.clear();
    for (entity, kin) in &bodies {
        witness.seen.insert(entity, (kin.last_changed(), kin.pos));
    }
}

/// A body advanced in a phase that does not normally advance it.
///
/// ⛔⛔ THE POISON MUST BE A DIFFERENT PHASE, NOT A DUPLICATED SYSTEM. Running
/// one system twice is what `boot_budget::no_system_is_registered_twice_in_one_schedule`
/// already catches; a poison shaped like that would test the guard we have. The
/// failure THIS instrument exists for is *"two different phases both advanced
/// this body"*, so the poison writes from `PresentationSync` — a phase whose
/// name says it should not be moving anything.
fn poison_write_in_presentation(mut bodies: Query<&mut BodyKinematics>) {
    for mut kin in &mut bodies {
        // A real write. `set_changed()` alone would move the change tick without
        // moving the body, and the instrument would be catching a marker rather
        // than a mutation.
        kin.pos.x += 0.0001;
    }
}

/// ⛔⛤ A NEUTRAL FRAME DOES NOT MOVE A BODY, AND A BASELINE OVER A STILL BODY
/// IS THE VACUITY THE FLOOR EXISTS TO CATCH.
///
/// MEASURED 2026-09-10 stepping `ControlFrame::default()` for 120 ticks: **479
/// mutable borrows and TEN position changes.** Every phase took its borrow every
/// tick — so a floor counting BORROWS passed comfortably — while the bodies
/// stood almost perfectly still. ⇒ *"No body advanced by two phases"* over ten
/// movements is very nearly a statement about nothing.
///
/// ⚠ THE FLOOR WAS GUARDING THE WRONG QUANTITY. `writes_attributed` counts
/// mutable access, which happens whether or not anything moves. The floor now
/// requires POSITION CHANGES.
fn walking(tick: usize) -> ControlFrame {
    // Reverse every half second so the body keeps moving instead of walking into
    // a wall and stopping — a stalled body is a still body with extra steps.
    let right = (tick / 30) % 2 == 0;
    ControlFrame {
        axis_x: if right { 1.0 } else { -1.0 },
        right_pressed: right,
        left_pressed: !right,
        jump_pressed: tick % 45 == 0,
        ..ControlFrame::default()
    }
}

fn harness() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz()),
    )
    .expect("sandbox sim builds")
}

fn install_probes(sim: &mut Platformer2dSimHarness) {
    sim.app_mut().init_resource::<TickWitness>();
    let label = sim.app_mut().sim_schedule();
    // ⛔⛔ EACH PROBE IS PENNED BETWEEN ITS PHASE AND THE NEXT. `.after(phase)`
    // ALONE DOES NOT PLACE A SYSTEM AT THAT SEAM — it only forbids running
    // before the phase, so the scheduler may run every probe at the very end of
    // the tick, in any order.
    //
    // ⇒ MEASURED 2026-09-10, the first run of this file with `.after` alone:
    // 240 of 240 writes attributed to `ControlInput` and ZERO to
    // `ActorSimulation`, the phase whose whole job is advancing actors. The
    // probes had collapsed to the end of the tick and whichever ran first
    // collected every change. **The zero doubles that run reported was an
    // artifact of the instrument, not a fact about the composition.**
    // The integration sub-phases, penned the same way. `BeforeIntegrate ->
    // Integrate -> AfterIntegrate` is where a body is actually advanced, and a
    // second advance across two of them is the failure this file is named for.
    sim.app_mut().add_systems(
        label,
        probe("WorldPrep/BeforeIntegrate")
            .after(WorldPrepSet::BeforeIntegrate)
            .before(WorldPrepSet::Integrate),
    );
    sim.app_mut().add_systems(
        label,
        probe("WorldPrep/Integrate")
            .after(WorldPrepSet::Integrate)
            .before(WorldPrepSet::AfterIntegrate),
    );
    sim.app_mut().add_systems(
        label,
        probe("WorldPrep/AfterIntegrate")
            .after(WorldPrepSet::AfterIntegrate)
            .before(Phase::PlayerSimulation),
    );
    for (i, (name, set)) in PHASES.iter().enumerate() {
        match PHASES.get(i + 1) {
            Some((_, next)) => sim
                .app_mut()
                .add_systems(label, probe(name).after(*set).before(*next)),
            None => sim.app_mut().add_systems(label, probe(name).after(*set)),
        };
    }
    // ⛔⛔ THE HARVEST RUNS AFTER THE *LAST* PHASE OF THE WHOLE TICK, NOT AFTER
    // THE LAST PHASE PROBED. `CoreSimulation` — which holds the six phases above
    // — is itself the FIRST of twelve sets chained under `GameplaySimulationRoot`
    // (`actor_monolith/src/schedule/schedule.rs:64`), and `Trace` is the last.
    //
    // ⇒ MEASURED 2026-09-10 with the harvest after `PresentationSync`: 239 of
    // 240 body-ticks looked doubled, because every write made in the eleven sets
    // AFTER `CoreSimulation` landed past the harvest and was credited to the
    // NEXT tick's first probe. `PlayerInput` scored 238 — one per body per tick —
    // for writes it did not make.
    //
    // ⚠ A "double tick" that includes the FIRST probe is the signature of a
    // harvest placed too early, and it reads exactly like a real finding.
    sim.app_mut()
        .add_systems(label, close_the_tick.after(Phase::Trace));
}

/// ⭐ THE ARM THAT MAKES THE ZERO READABLE. Without it, "no body advanced twice"
/// and "this instrument cannot see a second write" are the same output.
#[test]
fn the_instrument_catches_a_body_advanced_in_a_second_phase() {
    let mut sim = harness();
    install_probes(&mut sim);
    // ⚠ BEFORE `close_the_tick`, which runs after the same phase. A poison that
    // wrote after the harvest would be attributed to the NEXT tick and the
    // instrument would look blind while working correctly.
    let label = sim.app_mut().sim_schedule();
    sim.app_mut().add_systems(
        label,
        poison_write_in_presentation
            .after(Phase::Combat)
            .before(close_the_tick),
    );

    for tick in 0..30 {
        sim.step_frame(walking(tick));
    }

    let witness = sim.world().resource::<TickWitness>();
    assert!(
        !witness.doubled.is_empty(),
        "a body was written in `PresentationSync` as well as by the phase that \
         normally advances it, and the instrument reported NO double tick across \
         {} ticks and {} attributed writes. The clean run in the sibling test is \
         then a statement about a blind instrument.",
        witness.ticks,
        witness.writes_attributed
    );
}

#[test]
fn no_body_is_advanced_by_two_phases_in_one_tick() {
    // ⛔ ASK THE APP WHICH SCHEDULE THE SIM USES; do not name one. `SimSchedule`
    // is a RESOURCE holding the label, and which label it holds depends on the
    // host — `Update` for a bare app, a fixed schedule under a rollback host. A
    // probe registered into the wrong schedule runs at a time no body moves and
    // reports a serene zero.
    let mut sim = harness();
    install_probes(&mut sim);

    for tick in 0..120 {
        sim.step_frame(walking(tick));
    }

    let witness = sim.world().resource::<TickWitness>();
    let (ticks, bodies, writes, doubled) = (
        witness.ticks,
        witness.bodies_seen,
        witness.writes_attributed,
        witness.doubled.clone(),
    );

    // ⛔⛔ THE FLOOR, BEFORE THE FINDING. "No body advanced twice" over zero
    // bodies is a true statement answering a question nobody asked. On
    // 2026-09-10 the A2 giant road produced ZERO entities and a bare census
    // would have reported it clean.
    assert!(ticks > 0, "the probes never ran: no tick was closed");
    assert!(
        bodies > 0,
        "the probe saw NO body with `BodyKinematics` across {ticks} ticks. \
         Nothing below is a statement about double ticking — it is a statement \
         about an empty world."
    );
    assert!(
        writes > 0,
        "{bodies} bodies existed across {ticks} ticks and NOT ONE had its \
         `BodyKinematics` change tick move between phases. Bodies do move, so \
         this is the instrument failing rather than the composition being clean."
    );
    let moved = sim.world().resource::<TickWitness>().moves_attributed;
    assert!(
        moved >= ticks,
        "only {moved} POSITION CHANGES across {ticks} ticks and {bodies} bodies. \
         A borrow count can be large while every body stands still — measured \
         2026-09-10, a neutral input frame gave 479 borrows and 10 movements — \
         so a double-tick result over this population would be a statement about \
         bodies that are not being advanced at all."
    );

    eprintln!(
        "[double-tick] {ticks} ticks, {bodies} bodies, {writes} phase-attributed \
         writes, {} body-ticks written by more than one phase",
        doubled.len()
    );
    let w = sim.world().resource::<TickWitness>();
    let (per_phase, moved_per_phase, moves) = (
        w.per_phase.clone(),
        w.moved_per_phase.clone(),
        w.moves_attributed,
    );
    eprintln!("[double-tick] mutable BORROWS by phase: {per_phase:?}");
    eprintln!("[double-tick] position CHANGES by phase: {moved_per_phase:?} ({moves} total)");
    assert!(
        !per_phase.is_empty(),
        "no write was attributed to any phase, so the count above is a statement \
         about an instrument that discriminates nothing"
    );

    let mut pairs: BTreeMap<Vec<&'static str>, usize> = BTreeMap::new();
    for (_, phases) in &doubled {
        *pairs.entry(phases.clone()).or_default() += 1;
    }
    eprintln!("[double-tick] phase sets that advanced one body in one tick: {pairs:?}");

    // ⛔⛔ THIS RECORDS A BASELINE. IT DOES NOT GATE, AND THE FIRST VERSION OF
    // THIS FILE ASSERTED `doubled.is_empty()` — a zero the measurement then
    // refuted. ⇒ An assertion written before the measurement encodes what the
    // author EXPECTED, and a guard whose expectation is wrong reports the tree
    // as broken.
    //
    // ⚠ WHAT THE BASELINE IS FOR: after A4 splits control from body execution,
    // re-run this. A phase set that was not here before is a body the extraction
    // gave a second ticker. THAT comparison is the guard, and it needs this
    // number to exist first.
    //
    // ⭐ MEASURED 2026-09-10, the sandbox composition, 2 bodies, 120 ticks —
    // and the population is part of the result. A double tick that needs a
    // mount, a possession, a rider or four seats is OUTSIDE what this ran.
    eprintln!(
        "[double-tick] BASELINE: sandbox composition, {} bodies, {} ticks, \
         {} body-ticks advanced by more than one phase",
        bodies,
        ticks,
        doubled.len()
    );
}
