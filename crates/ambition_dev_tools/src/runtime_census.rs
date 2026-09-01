//! Profiling-only workload census: what Ambition asked the engine to do.
//!
//! A native profile proves the engine was expensive; it cannot say the game
//! was presenting three world-rendering cameras and refreshing two portal
//! captures while it happened. These censuses record that workload on stderr
//! on ONE shared clock, so a slow interval in a perf/Tracy capture can be read
//! against the frame's actual scene.
//!
//! Every row is a single line of the form
//!
//! ```text
//! [census] <kind> t=<seconds> key=value key=value ...
//! ```
//!
//! `scripts/profile_desktop.sh` turns those rows into one CSV per kind.
//!
//! **Off unless asked for.** [`RuntimeCensus::from_env`] reads
//! `AMBITION_PROFILE_CENSUS` once at startup; when it is unset every census
//! system costs one already-resident bool test per frame and returns. The
//! sample cadence (`AMBITION_PROFILE_CENSUS_HZ`, default 1 Hz) bounds the work
//! the enabled path does: no census iterates a per-entity population on a
//! frame that is not a sample frame. Measured on a headless sandbox run, the
//! enabled census is under the run-to-run spread of retired instructions —
//! see `docs/recipes/profiling.md`.

use bevy::diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
use bevy::ecs::archetype::Archetypes;
use bevy::ecs::component::Components;
use bevy::ecs::entity::Entities;
use bevy::ecs::resource::IsResource;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;

use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;

use std::time::Duration;
// ⛔ THE `cfg` BELONGS TO `Instant` ALONE. Keep any unconditional import ABOVE
// it: an attribute drifts onto whatever item follows, and a `Duration` that
// silently vanished on wasm would break the build only on the web.
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

/// Environment variable that turns the censuses on.
pub const CENSUS_ENV: &str = "AMBITION_PROFILE_CENSUS";
/// Environment variable that overrides the sample rate, in samples per second.
pub const CENSUS_HZ_ENV: &str = "AMBITION_PROFILE_CENSUS_HZ";

/// The shared census clock and gate.
///
/// ONE resource decides both whether a census runs and which frame is a sample
/// frame, so every row written in a frame carries the same `t=` and rows from
/// different crates line up without a correlation step. A census that kept its
/// own timer would drift against its neighbours and make "the camera count rose
/// while the pass got slower" unprovable.
#[derive(Resource)]
pub struct RuntimeCensus {
    enabled: bool,
    interval_s: f64,
    #[cfg(not(target_arch = "wasm32"))]
    started_at: Instant,
    next_at: f64,
    /// Seconds since census start for this frame's sample, or `None` when this
    /// frame is not a sample frame.
    due_at: Option<f64>,
}

impl Default for RuntimeCensus {
    fn default() -> Self {
        Self::from_env()
    }
}

impl RuntimeCensus {
    /// Default cadence. Fast enough that a two-second stall lands in its own
    /// row, slow enough that the census is not itself the workload.
    pub const DEFAULT_HZ: f64 = 1.0;

    /// Read the gate and cadence from the environment.
    pub fn from_env() -> Self {
        let enabled = std::env::var(CENSUS_ENV)
            .map(|value| env_is_truthy(&value))
            .unwrap_or(false);
        let hz = std::env::var(CENSUS_HZ_ENV)
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|hz| *hz > 0.0)
            .unwrap_or(Self::DEFAULT_HZ);
        Self {
            enabled,
            interval_s: 1.0 / hz,
            #[cfg(not(target_arch = "wasm32"))]
            started_at: Instant::now(),
            next_at: 0.0,
            due_at: None,
        }
    }

    /// Whether any census should do work at all this run.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Seconds between samples.
    pub fn interval_s(&self) -> f64 {
        self.interval_s
    }

    /// `Some(seconds_since_census_start)` on a sample frame, `None` otherwise.
    ///
    /// This is the ONLY thing a census system should branch on: `let Some(at) =
    /// census.due() else { return; }` keeps a disabled or off-cadence frame at
    /// a single bool test.
    pub fn due(&self) -> Option<f64> {
        self.due_at
    }
}

fn env_is_truthy(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "0" | "false" | "no" | "off"
    )
}

/// Advance the shared census clock. Register FIRST in the frame so every
/// census system in the frame agrees on `due()`.
#[cfg(not(target_arch = "wasm32"))]
pub fn advance_runtime_census(mut census: ResMut<RuntimeCensus>) {
    if !census.enabled {
        return;
    }
    let now = census.started_at.elapsed().as_secs_f64();
    if now < census.next_at {
        // Only clear a stale `due_at`; leaving the resource unmarked on the
        // frames in between keeps change detection quiet.
        if census.due_at.is_some() {
            census.due_at = None;
        }
        return;
    }
    let interval = census.interval_s;
    census.due_at = Some(now);
    census.next_at = now + interval;
}

#[cfg(target_arch = "wasm32")]
pub fn advance_runtime_census(_census: ResMut<RuntimeCensus>) {}

/// Scene entities — everything a resource is not.
///
/// ⛔⛔ THE NAME IS THE POINT. Bevy 0.19 made a resource an entity, so "entity
/// count" is now ambiguous in a way it never used to be. Publishing one number
/// called `entities` would carry that ambiguity into every dashboard and note
/// taken from it; two paths whose names SAY which population they are cannot.
pub const SCENE_ENTITIES: DiagnosticPath = DiagnosticPath::const_new("ambition/ecs/scene_entities");

/// Entities that exist only to hold a resource value. See [`SCENE_ENTITIES`].
pub const RESOURCE_ENTITIES: DiagnosticPath =
    DiagnosticPath::const_new("ambition/ecs/resource_entities");

/// Bodies the physics world is stepping.
pub const BODIES: DiagnosticPath = DiagnosticPath::const_new("ambition/ecs/bodies");

/// Register Ambition's ECS diagnostics and keep them fed.
///
/// ⭐ ONE MEASUREMENT, TWO CONSUMERS. This publishes from [`EcsPopulation`] —
/// the same system param `report_ecs_census` prints from — so the periodic log
/// row and the F1 panel cannot disagree about how many entities there are.
/// The alternative, a second count inside the overlay, is how two numbers with
/// one name get born.
///
/// ⛔ IT IS NOT GATED ON `AMBITION_PROFILE_CENSUS`. The census printer is, and
/// should be: it writes to stderr on a clock nobody asked for. This publishes
/// into `DiagnosticsStore`, which is what the F1 panel reads, and F1 is a thing
/// a developer turns on WITHOUT setting an environment variable and restarting.
pub struct EcsDiagnosticsPlugin;

impl Plugin for EcsDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.register_diagnostic(Diagnostic::new(SCENE_ENTITIES).with_suffix(" entities"))
            .register_diagnostic(Diagnostic::new(RESOURCE_ENTITIES).with_suffix(" resources"))
            .register_diagnostic(Diagnostic::new(BODIES).with_suffix(" bodies"))
            .add_systems(
                Update,
                publish_ecs_diagnostics.run_if(on_timer(ECS_DIAGNOSTIC_SAMPLE_PERIOD)),
            );
    }
}

/// How often the ECS populations are re-counted for the diagnostics store.
///
/// ⭐ THE PATHS STAY REGISTERED; ONLY THE SAMPLING IS PACED. F1 can therefore be
/// opened at any moment without a restart, and still shows a number within a
/// quarter second of the truth — which is finer than a human reading a dashboard
/// can perceive, and finer than the overlay's own refresh.
///
/// ⛔ A DASHBOARD DOES NOT GET TO BILL THE FRAME LOOP FOR FRESHNESS NOBODY CAN
/// SEE. `Query::count()` below already makes one sample cheap; this bounds the
/// cost against a world that grows more archetypes than today's.
const ECS_DIAGNOSTIC_SAMPLE_PERIOD: Duration = Duration::from_millis(250);

fn publish_ecs_diagnostics(mut diagnostics: Diagnostics, population: EcsPopulation) {
    diagnostics.add_measurement(&SCENE_ENTITIES, || population.scene_entities() as f64);
    diagnostics.add_measurement(&RESOURCE_ENTITIES, || population.resource_entities() as f64);
    diagnostics.add_measurement(&BODIES, || population.bodies() as f64);
}

/// The four entity populations the ECS census counts, as one reusable param.
///
/// ⛔⛔ THIS TYPE EXISTS SO THE `Without<IsResource>` CANNOT BE FORGOTTEN. Under
/// Bevy 0.19 a resource IS an entity, so the obvious spelling of "how many
/// entities are there" — `Query<()>` — silently answers "scene content plus
/// every registered resource". Anything that wants a scene-entity count asks
/// this param rather than writing its own query, and the guard test below
/// drives THIS type rather than a hand-copied one.
#[derive(bevy::ecs::system::SystemParam)]
pub struct EcsPopulation<'w, 's> {
    scene: Query<'w, 's, (), Without<IsResource>>,
    resources: Query<'w, 's, (), With<IsResource>>,
    bodies: Query<'w, 's, (), With<BodyKinematics>>,
    players: Query<'w, 's, (), With<PlayerEntity>>,
}

/// ⭐ EVERY COUNT HERE IS `Query::count()`, NOT `iter().count()`. All four
/// queries take no data (`()`) and filter only with `With`/`Without`, so both
/// halves are ARCHETYPAL and Bevy answers from archetype and table sizes without
/// visiting an entity. `iter().count()` forfeits that and walks the world — for
/// the scene population that is every entity in the game, on every sample.
impl EcsPopulation<'_, '_> {
    /// Entities that are scene content: everything a resource is not.
    pub fn scene_entities(&self) -> usize {
        self.scene.count()
    }

    /// Entities that exist only to hold a resource value.
    pub fn resource_entities(&self) -> usize {
        self.resources.count()
    }

    /// Bodies the physics world is stepping.
    pub fn bodies(&self) -> usize {
        self.bodies.count()
    }

    /// Bodies a seated player is driving.
    pub fn players(&self) -> usize {
        self.players.count()
    }
}

/// Whole-world ECS scale, and the two populations that make it grow: bodies
/// and player bodies.
///
/// `&Entities` / `&Archetypes` / `&Components` are metadata params, not data
/// queries — they conflict with nothing and cost no iteration. The body counts
/// are the only iteration here and are bounded by the cast, not by the scene.
pub fn report_ecs_census(
    census: Res<RuntimeCensus>,
    entities: &Entities,
    archetypes: &Archetypes,
    components: &Components,
    population: EcsPopulation,
) {
    let Some(at) = census.due() else {
        return;
    };
    // ⛔⛔ `entities`, `live` AND `resources` ARE THREE DIFFERENT QUESTIONS, AND
    // THE FIRST ONE MISLEADS. `Entities::len()` counts ALLOCATED entity slots,
    // which land on round powers of two — measured 2026-08-29, four
    // structurally different rooms all reported exactly 2048 or exactly 4096
    // while their real content (`Transform` 703 vs 1379) was nothing like a
    // power of two. Three separate entries in the performance notebook quote a
    // "2048-entity" scene that does not exist. `live` iterates, so it is the
    // number of entities that are actually there.
    //
    // ⛔⛔ AND SINCE BEVY 0.19 "ACTUALLY THERE" NEEDS `Without<IsResource>`.
    // Resources became components on singleton entities, so an unfiltered
    // `Query<()>` counts every registered resource as scene content — a bare
    // App reads 16 before anything is spawned, and a full session's resource
    // count is in the hundreds. That is a constant offset, which is the worst
    // kind: it never looks like a bug, it just makes every scene bigger than it
    // is. `resources` is reported BESIDE `live` rather than folded into it
    // because the count is real engine population — it is simply not scene.
    // ⇒ READ `live`; `entities - live - resources` is the reservation slack,
    // and a large gap is itself a finding.
    eprintln!(
        "[census] ecs t={at:.3} entities={} live={} resources={} archetypes={} components={} \
         bodies={} players={}",
        entities.len(),
        population.scene_entities(),
        population.resource_entities(),
        archetypes.len(),
        components.len(),
        population.bodies(),
        population.players(),
    );
}

/// The registered-system population per schedule, sampled rather than reported
/// once at startup: schedules gain systems when a session activates, so the
/// boot-time count is not the count a slow interval ran under.
pub fn report_schedule_load_census(census: Res<RuntimeCensus>, schedules: Res<Schedules>) {
    let Some(at) = census.due() else {
        return;
    };
    let mut total = 0usize;
    let mut visible = 0usize;
    // ⭐ PER-SCHEDULE, NOT JUST A TOTAL. `[census] phases` says which phase of
    // the frame cost the most; this says how many systems that phase is
    // carrying. Together they turn "StateTransition is 10% of the frame" into
    // an answerable question — a phase that is expensive with four systems in
    // it is a different bug from one that is expensive with four hundred.
    let mut populations: Vec<(String, usize)> = Vec::new();
    for (label, schedule) in schedules.iter() {
        let count = schedule.systems_len();
        if count > 0 {
            visible += 1;
            total += count;
            populations.push((format!("{label:?}"), count));
        }
    }
    // Biggest first, and by name within a tie so the row is stable between
    // samples and a diff between two runs means something.
    populations.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let mut row = format!("[census] schedules t={at:.3} schedules={visible} systems={total}");
    for (label, count) in &populations {
        row.push_str(&format!(" {label}={count}"));
    }
    eprintln!("{row}");
}

/// Where the RUN CONDITIONS sit, and how many there are.
///
/// ⛔⛔ ONE-SHOT, AND IT MUST RUN BEFORE THE SCHEDULES DO. `Schedule::initialize`
/// MOVES every condition out of the `ScheduleGraph` into the private executable
/// (`update_schedule` drains them, and hands them back to the graph only to
/// rebuild). There is no public accessor for the built conditions, so a sampled
/// census in `Last` reads a drained graph and reports a confident ZERO —
/// measured, 886 systems and `system_conditions=0`, which is how this ended up
/// registered at startup instead.
///
/// ⚠ IT COUNTS WHAT PLUGIN BUILD REGISTERED. Schedules gain systems later when a
/// session activates, and those are not in this number.
///
/// ⭐⭐ THE STRUCTURAL METRIC, DELIBERATELY NOT A PROFILER ONE. Bevy evaluates a
/// system's conditions once per system per schedule run and a SET's conditions
/// once per run regardless of how many systems the set holds, so "how many
/// conditions are attached, and to what" is a count the schedule graph already
/// knows — no Tracy, no sampling, no observer effect. That matters twice over:
/// Tracy inflates this app's frame roughly 9x, and a deterministic count is a
/// far better regression gate than a wall-clock millisecond.
///
/// ⛔ IT COUNTS ATTACHMENTS, NOT EVALUATIONS, and the gap between them is the
/// whole point. `system_conditions` is what a frame pays per run; `set_conditions`
/// is what the same semantic gate costs once it has been hoisted onto a set.
/// Moving N systems' shared condition onto one set moves N out of the first
/// number and 1 into the second, which is exactly the shape of the improvement
/// and is invisible to any timing measurement small enough to trust.
///
/// The per-condition breakdown names the offenders: one condition attached 87
/// times is a line saying so, rather than a number somebody has to explain.
pub fn report_schedule_conditions_census(schedules: Res<Schedules>) {
    let mut system_conditions = 0usize;
    let mut set_conditions = 0usize;
    let mut sets_with_conditions = 0usize;
    // Condition name -> how many systems carry it. A `BTreeMap` because the row
    // is diffed between runs, and a hash order would make every sample differ.
    let mut by_name: std::collections::BTreeMap<String, usize> = Default::default();
    for (_label, schedule) in schedules.iter() {
        let graph = schedule.graph();
        for (_key, _system, conditions) in graph.systems.iter() {
            system_conditions += conditions.len();
            for condition in conditions {
                *by_name
                    .entry(condition_label(condition.condition.name().as_ref()))
                    .or_default() += 1;
            }
        }
        for (_key, _set, conditions) in graph.system_sets.iter() {
            if conditions.is_empty() {
                continue;
            }
            sets_with_conditions += 1;
            set_conditions += conditions.len();
        }
    }
    // ⛔ A ZERO HERE IS THE INSTRUMENT FAILING, NOT THE ENGINE BEING CLEAN.
    // Every schedule in this app carries conditions; reading none means this ran
    // after the graphs were drained. Say so, so nobody records a 0 as a fact.
    if system_conditions == 0 && set_conditions == 0 {
        eprintln!(
            "[census] conditions t=0.000 unavailable=graph_already_initialized \
             (this must run before the schedules do)"
        );
        return;
    }
    report_schedule_owners(&schedules);
    // The two phases the campaign cannot yet attribute. `PreUpdate` because
    // 0.95ms of it is not the sim; `Update` because in the SHIPPED app the sim
    // lives in `GgrsSchedule`, so nobody knows what its 1.4ms is made of.
    report_schedule_membership(&schedules, "PreUpdate", 0.0);
    report_schedule_membership(&schedules, "Update", 0.0);
    // `PostUpdate` is 0.65ms of a Smash frame — 14% — and nothing has looked at
    // it. Presentation and render extraction live here.
    report_schedule_membership(&schedules, "PostUpdate", 0.0);
    // Who owns `Update` — the campaign's last unexplained phase that is ours.
    report_schedule_owners_in(&schedules, "Update");
    // ⭐ AND THE OTHER TWO PHASES THAT CARRY REAL COST. `PostUpdate` is 31% of
    // what an added FIGHTER costs (presentation and render extraction live
    // there) and nobody had looked at who owns it; `PreUpdate` holds the 0.93ms
    // that is neither the sim nor the rollback driver.
    report_schedule_owners_in(&schedules, "PreUpdate");
    report_schedule_owners_in(&schedules, "PostUpdate");
    // ⭐ The two phases that inflate WORST between a Smash stage and a real room:
    // `StateTransition` 0.14ms -> 2.06ms (15x) and `RunFixedMainLoop` 0.40 ->
    // 2.42ms (6x). Naming their populations is the first question about either.
    report_schedule_membership(&schedules, "StateTransition", 0.0);
    report_schedule_membership(&schedules, "RunFixedMainLoop", 0.0);
    let mut row = format!(
        "[census] conditions t=0.000 system_conditions={system_conditions} \
         set_conditions={set_conditions} sets_with_conditions={sets_with_conditions}"
    );
    // Only the ones worth a name. A condition attached once is not the story;
    // the tail would bury the ones that are.
    let mut ranked: Vec<(&String, &usize)> =
        by_name.iter().filter(|(_, count)| **count >= 4).collect();
    ranked.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    for (name, count) in ranked.iter().take(16) {
        row.push_str(&format!(" {name}={count}"));
    }
    eprintln!("{row}");
}

/// Where the SIM TICK's time goes, phase by phase.
///
/// ⭐⭐ THE INSTRUMENT THE CAMPAIGN ACTUALLY NEEDED. `[census] phases` splits the
/// MAIN schedule and reports `PreUpdate=1.98ms` for a Smash match — but in this
/// app `PreUpdate` holds one exclusive system, `bevy_ggrs`'s
/// `run_ggrs_schedules`, which runs the whole sim as `GgrsSchedule`. So the main
/// split bottoms out at "the simulation costs 2ms" and cannot say which part.
/// This splits THAT.
///
/// ⛔ IT CANNOT USE THE `FramePhaseMark` TRICK. That one interleaves marker
/// SCHEDULES into `MainScheduleOrder`; these are SETS inside one schedule, so
/// the boundary has to be a system ordered between them. The phases are already
/// `.chain()`ed, which is what makes "after this set, before the next" a
/// well-defined place to stand.
///
/// ⚠ THE TOTALS ARE NOT ROLLBACK STATE, deliberately. This resource is mutated
/// inside the sim schedule and never registered for rollback, so a rewind leaves
/// last-branch timings in it. That is correct for an instrument — it measures
/// what the CPU actually did, including work that was later discarded — but it
/// means these numbers are not reproducible across a rewind and must never gate
/// behaviour. The shipped local session runs `check_distance: 0` and never
/// rewinds at all.
/// ⛔ WALL CLOCK, so NOT ON WASM — the same rule every other census in this
/// file follows. `std::time::Instant::now()` panics in a browser, and this
/// file's `Instant` import is already `cfg(not(wasm32))` for that reason.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Default)]
pub struct SimPhaseCensus {
    /// When the previous boundary fired, or `None` before the first.
    last: Option<Instant>,
    /// Accumulated time attributed to each phase, parallel to `names`.
    totals: Vec<f64>,
    names: Vec<&'static str>,
    ticks: u32,
}

#[cfg(not(target_arch = "wasm32"))]
impl SimPhaseCensus {
    fn with_names(names: Vec<&'static str>) -> Self {
        Self {
            last: None,
            totals: vec![0.0; names.len()],
            names,
            ticks: 0,
        }
    }

    /// Start the tick's window. Time before this point belongs to the frame,
    /// not to any sim phase, and is deliberately attributed to neither.
    fn open(&mut self) {
        self.last = Some(Instant::now());
        self.ticks = self.ticks.saturating_add(1);
    }

    /// Whether this window holds enough ticks to be a mean rather than a sample.
    ///
    /// See the block comment in `report_sim_phase_census` for what a one-tick
    /// window cost. The time it accumulated is NOT discarded: the reporter
    /// returns before the reset, so the partial window folds into the next.
    fn is_reportable(&self) -> bool {
        self.ticks >= 2
    }

    /// Close the phase that just ended and open the next.
    fn close(&mut self, index: usize) {
        let now = Instant::now();
        if let Some(last) = self.last {
            if let Some(total) = self.totals.get_mut(index) {
                *total += now.duration_since(last).as_secs_f64() * 1000.0;
            }
        }
        self.last = Some(now);
    }
}

/// The bucket the actor decision chain closes, from the actor monolith.
///
/// ⛔ IT IS AN INDEX INTO A LIST IN ANOTHER CRATE, which is not a shape to copy.
/// It is here because `ambition_dev_tools` cannot name `ActorDecisionSet` —
/// it depends on `shared_tangle`, not on the monolith, and the edge only runs
/// the other way. The alternative was a runtime registration API whose
/// correctness depends on plugin BUILD ORDER, which is the more fragile of the
/// two. `report_sim_phase_census` reports an unclosed bucket rather than letting
/// the neighbouring name quietly widen.
#[cfg(not(target_arch = "wasm32"))]
pub const SIM_PHASE_ACTOR_DECISION: usize = 21;
/// `ActorDecisionSet::Targeting` — `select_actor_targets` and friends.
#[cfg(not(target_arch = "wasm32"))]
pub const SIM_PHASE_DECISION_TARGETING: usize = 22;
/// `ActorDecisionSet::Prepare`.
#[cfg(not(target_arch = "wasm32"))]
pub const SIM_PHASE_DECISION_PREPARE: usize = 23;
/// `ActorDecisionSet::Observe` — where the per-tick perception snapshot is built.
#[cfg(not(target_arch = "wasm32"))]
pub const SIM_PHASE_DECISION_OBSERVE: usize = 24;
/// `ActorDecisionSet::StateMaintenance`.
#[cfg(not(target_arch = "wasm32"))]
pub const SIM_PHASE_DECISION_STATE_MAINTENANCE: usize = 25;
/// `ActorDecisionSet::Decide` — `tick_actor_brains`, and `build_world_view` with it.
#[cfg(not(target_arch = "wasm32"))]
pub const SIM_PHASE_DECISION_DECIDE: usize = 26;
/// `ActorDecisionSet::Publish`.
#[cfg(not(target_arch = "wasm32"))]
pub const SIM_PHASE_DECISION_PUBLISH: usize = 27;

/// Whether the sim-phase census is installed at all, for a crate that registers
/// its own boundary marks.
///
/// ⛔⛔ THE MARKS ARE NOT INSTALLED WHEN THE CENSUS IS OFF, and a caller outside
/// this crate cannot see the `if enabled` in `RuntimeCensusPlugin::build` that
/// says so. `install_actor_decision_census_boundary` in the actor monolith
/// registered seven marks unconditionally and every run with the census OFF —
/// which is every run a player makes — panicked in `mark_sim_phase` with
/// "Resource does not exist". The warning against exactly this was already
/// written here, on the reporter, where the crate that needed it could not read
/// it.
///
/// ⇒ TWO defences, because they answer different questions. This one keeps a
/// mark out of the hottest schedule in the app when nobody asked for it. The
/// `Option` on [`mark_sim_phase`] makes forgetting to ask a no-op rather than a
/// crash.
#[cfg(not(target_arch = "wasm32"))]
pub fn sim_phase_census_enabled() -> bool {
    std::env::var(CENSUS_ENV)
        .map(|value| env_is_truthy(&value))
        .unwrap_or(false)
}

/// The boundary system for one sim phase.
///
/// ⚠ `Option`, and it is load-bearing rather than defensive: this function is
/// `pub` so other crates can close buckets for sets only they can name, and the
/// resource it writes is inserted only when the census is on. A missing census
/// is a mark with nothing to record, not a reason to stop the game.
#[cfg(not(target_arch = "wasm32"))]
pub fn mark_sim_phase(index: usize) -> impl FnMut(Option<ResMut<SimPhaseCensus>>) {
    move |census: Option<ResMut<SimPhaseCensus>>| {
        if let Some(mut census) = census {
            census.close(index);
        }
    }
}

/// OPEN the window, attributing nothing.
///
/// ⛔⛔ WITHOUT THIS THE FIRST BUCKET IS A LIE, and it lied convincingly. A
/// closing boundary attributes "now minus the previous boundary", so with no
/// opening mark the first phase absorbs everything between the PREVIOUS tick's
/// last phase and this one's first — the entire rest of the frame, main
/// schedule and render included. Measured 2026-08-29: it reported
/// `PlayerInput=3.96ms` inside a sim tick that the main-phase census put at
/// 1.98ms TOTAL. A bucket larger than the thing containing it is the tell.
#[cfg(not(target_arch = "wasm32"))]
fn open_sim_phase_window(mut census: ResMut<SimPhaseCensus>) {
    census.open();
}

/// Report the sim-phase split on the census interval, then reset.
///
/// ⭐ REPORTED AS A PER-TICK AVERAGE over the interval, because a single sim
/// tick is microseconds and the interesting quantity is what the tick costs on
/// average, not what one of them did.
/// The SIM schedule's membership — the one schedule `report_schedule_conditions_census`
/// could never name.
///
/// ⛔⛔ WHY THIS EXISTS: the `PreStartup` membership pass reports only MAIN-schedule
/// labels, and the shipped sim lives in `GgrsSchedule`, which does not exist yet
/// at `PreStartup` — it is created when a session activates. So a grep for a
/// system in `GgrsSchedule` against that pass returns zero FOR EVERY SYSTEM,
/// present or absent. Measured 2026-08-29, when exactly that zero was published
/// as "verified: 0 occurrences in GgrsSchedule" for two systems that had been
/// moved out. The move was real; the evidence was not.
///
/// ⇒ SAMPLED, and ONCE. By the time this can see the schedule its graph is
/// drained, so it goes through `report_schedule_membership`, whose executable
/// fallback reads the systems back out of the initialized schedule.
///
/// ⭐ Matched BY NAME rather than by label type, deliberately: `ambition_dev_tools`
/// must not take a `bevy_ggrs` dependency (see `install_ggrs_driver_census`,
/// which lives in the ggrs crate for exactly that reason).
pub fn report_sim_schedule_membership(
    census: Res<RuntimeCensus>,
    schedules: Option<Res<bevy::ecs::schedule::Schedules>>,
    mut reported: Local<bool>,
) {
    if *reported {
        return;
    }
    let Some(at) = census.due() else {
        return;
    };
    let Some(schedules) = schedules else {
        return;
    };
    // Only once the session has actually built it; before that the honest answer
    // is "not yet", and saying it every sample would be noise.
    let present = schedules
        .iter()
        .any(|(label, _)| SIM_SCHEDULE_NAMES.contains(&format!("{label:?}").as_str()));
    if !present {
        return;
    }
    *reported = true;
    for wanted in SIM_SCHEDULE_NAMES {
        if schedules
            .iter()
            .any(|(label, _)| format!("{label:?}") == *wanted)
        {
            report_schedule_membership(&schedules, wanted, at);
            // ⭐ AND WHO OWNS IT. The membership row gives bare system NAMES
            // (`condition_label` drops the path), so crate attribution is lost
            // there. This is the "which crate is asking the SIM for work"
            // question — the one an architecture decomposition actually needs,
            // and it was previously answerable only for main schedules.
            report_schedule_owners_in(&schedules, wanted);
        }
    }
}

/// The labels a sim schedule can wear in this repo. `GgrsSchedule` is the shipped
/// one; `SimSchedule` is what a non-rollback host binds.
const SIM_SCHEDULE_NAMES: &[&str] = &["GgrsSchedule", "SimSchedule"];

#[cfg(not(target_arch = "wasm32"))]
pub fn report_sim_phase_census(census: Res<RuntimeCensus>, mut phases: ResMut<SimPhaseCensus>) {
    let Some(at) = census.due() else {
        return;
    };
    if phases.ticks == 0 {
        return;
    }
    // ⛔⛔ A WINDOW OF ONE TICK IS NOT A MEASUREMENT, AND EMITTING IT COST A DAY.
    // The first report after boot covers the fraction of a second between the
    // census opening and its first due time, which in a starting app is one tick
    // in a world that has not finished spawning. Every phase in it reads 0.000 —
    // not because the phases are free, but because almost nothing had run yet.
    //
    // ⇒ IT WAS AVERAGED IN, REPEATEDLY, BY THE PERSON WHO WROTE THE ROW ABOVE.
    // The obvious summary of a run is "the last few windows", and on 2026-09-01
    // a 1200-tick capture produced about three of them, one of which was this:
    //
    //     (0.000 + 0.341 + 0.332) / 3 = 0.224
    //
    // That 0.224 was published as the hall's `Decide` cost against a true steady
    // value of 0.341, and the same bias sat under a population curve, a density
    // sweep and three A/B decompositions — worst at low populations, where runs
    // are shortest and the zero is the largest share of the mean.
    //
    // ⭐ SO IT IS NOT EMITTED. `ticks=1` is already in the row and a reader could
    // filter on it; three separate analyses did not. A row that cannot be read
    // correctly is worse than a row that is absent, because absence is visible.
    // The window's time is not lost — it is folded into the next one, which is
    // the reading anyone wanted.
    if !phases.is_reportable() {
        return;
    }
    let ticks = phases.ticks as f64;
    let mut row = format!("[census] sim_phases t={at:.3} ticks={}", phases.ticks);
    // ⛔⛔ A CAPPED RUN IS NOT THE SHIPPED ROOM. `AMBITION_ACTOR_POPULATION_CAP`
    // removes authored actors to make a scaling curve possible, and a row taken
    // under it describes a room nobody plays. Say so ON THE ROW — a reader
    // quoting a number will not go looking for the environment it was taken in.
    if let Some(cap) = crate::population_cap::active_cap() {
        row.push_str(&format!(" actor_cap={cap}"));
    }
    // ⛔⛔ AND A RE-BRAINED CAST IS NOT THE AUTHORED ONE EITHER.
    // `AMBITION_ACTOR_BRAIN_OVERRIDE` replaces what every body in the room
    // thinks with, which moves the decision cost far more than removing bodies
    // does. Same rule: say so ON THE ROW.
    if let Some(preset) = crate::brain_override::forced_preset() {
        row.push_str(&format!(" brain_override={preset}"));
    }
    if let Some(profile) = crate::brain_override::forced_profile() {
        row.push_str(&format!(" brain_profile={profile}"));
    }
    // ⛔ The schedule declares no order between these, so their buckets are
    // differences between marks nobody sequenced. Named on the row so a reader
    // cannot mistake the list for a serial partition of the frame.
    // ⭐ WHAT EACH VIEWER ACTUALLY KEPT. `Decide`'s slope follows this number
    // rather than the room's population: it grows superlinearly while the kept
    // set is still growing and flattens when the viewport saturates. Reported
    // beside the phase it explains.
    if let Some((views, offered, kept, kept_max)) = crate::perception_census::drain() {
        row.push_str(&format!(
            " views={views} offered={offered:.1} kept={kept:.1} kept_max={kept_max}"
        ));
    }
    row.push_str(" unmeasured=");
    for (i, index) in SIM_PHASE_UNORDERED.iter().enumerate() {
        if i > 0 {
            row.push('+');
        }
        row.push_str(phases.names.get(*index).copied().unwrap_or("?"));
    }
    let mut ranked: Vec<(&'static str, f64)> = phases
        .names
        .iter()
        .copied()
        .zip(phases.totals.iter().map(|total| total / ticks))
        .collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (name, per_tick) in ranked {
        row.push_str(&format!(" {name}={per_tick:.3}"));
    }
    // ⛔⛔ AN UNCLOSED BUCKET WIDENS ITS NEIGHBOUR SILENTLY, which is exactly how
    // `WorldPrep.BeforeIntegrate` came to mean the whole prefix through it.
    // Ticks ran and this bucket took literally none of them, so the mark is not
    // installed — say it on the row a reader is about to quote, not in a comment
    // they will not open.
    if phases
        .totals
        .get(SIM_PHASE_ACTOR_DECISION)
        .is_some_and(|total| *total == 0.0)
    {
        row.push_str(
            " !! WorldPrep.Decision.* NEVER CLOSED — WorldPrep.BeforeIntegrate \
             still includes the whole actor decision chain",
        );
    }
    eprintln!("{row}");
    phases.totals.iter_mut().for_each(|total| *total = 0.0);
    phases.ticks = 0;
}

/// WHAT the entities ARE — the biggest populations, by the component that names
/// them.
///
/// ⭐⭐ THE QUESTION `entities=2048` CANNOT ANSWER. A Smash match takes this app
/// from 64 entities to 2048 while `bodies` stays at 2, so the population that
/// grew is not the one the other rows name. A phase cost that scales with
/// entities is unattributable until somebody can say WHICH entities.
///
/// ⛔ IT RANKS BY COMPONENT, NOT BY ARCHETYPE, and the difference matters: an
/// archetype is a SET of components and its name is a list nobody can read,
/// while "2000 entities carry `Sprite`" is a sentence. An entity is counted once
/// per component it holds, so the numbers do not sum to the entity count and are
/// not meant to.
pub fn report_entity_populations(
    census: Res<RuntimeCensus>,
    entities: &Entities,
    archetypes: &Archetypes,
    components: &Components,
) {
    let Some(at) = census.due() else {
        return;
    };
    let mut by_component: Vec<(String, usize)> = Vec::new();
    for archetype in archetypes.iter() {
        let count = archetype.len() as usize;
        if count == 0 {
            continue;
        }
        for component_id in archetype.components() {
            let Some(info) = components.get_info(*component_id) else {
                continue;
            };
            let name = short_type_name(info.name().as_ref());
            match by_component.iter_mut().find(|(known, _)| *known == name) {
                Some((_, total)) => *total += count,
                None => by_component.push((name, count)),
            }
        }
    }
    by_component.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let mut row = format!(
        "[census] populations t={at:.3} entities={} archetypes={}",
        entities.len(),
        archetypes.len()
    );
    for (name, count) in by_component.iter().take(18) {
        row.push_str(&format!(" {name}={count}"));
    }
    eprintln!("{row}");
}

/// A component's type name without its path or generics — `Sprite`, not
/// `bevy_sprite::sprite::Sprite`. The tail is what identifies it to a reader,
/// and the path would make the row's width depend on crate layout.
fn short_type_name(name: &str) -> String {
    let head = name.split('<').next().unwrap_or(name);
    head.rsplit("::").next().unwrap_or(head).to_string()
}

/// WHO OWNS the systems in ONE schedule.
///
/// ⭐⭐ THE CUT THAT MAPS ONTO CAPABILITIES. `[census] owners` totals every
/// schedule at once, and `[census] membership` names systems without grouping
/// them. Neither answers "which crate should I gate to attribute `Update`'s
/// 1.42ms", which is the campaign's last open question — the only phase both
/// substantially OURS and unexplained.
///
/// ⛔ A COUNT IS STILL NOT A COST. This ranks who is REGISTERED here, which is
/// how you choose what to gate and measure next; it does not claim any of them
/// is expensive. Eleven probes on this codebase have found population size to be
/// a poor predictor of frame time.
fn report_schedule_owners_in(schedules: &Schedules, wanted: &str) {
    for (label, schedule) in schedules.iter() {
        if format!("{label:?}") != wanted {
            continue;
        }
        let mut by_owner: std::collections::BTreeMap<String, usize> = Default::default();
        for (_key, system, _conditions) in schedule.graph().systems.iter() {
            *by_owner
                .entry(owning_crate(system.name().as_ref()))
                .or_default() += 1;
        }
        let total: usize = by_owner.values().sum();
        let mut ranked: Vec<(&String, &usize)> = by_owner.iter().collect();
        ranked.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
        let mut row = format!(
            "[census] owners_in t=0.000 schedule={wanted} systems={total} crates={}",
            by_owner.len()
        );
        for (name, count) in ranked.iter().take(20) {
            row.push_str(&format!(" {name}={count}"));
        }
        eprintln!("{row}");
        return;
    }
}

/// NAME every system in one schedule, so an unattributed cost stops being
/// "DefaultPlugins" and becomes something a person can act on.
///
/// ⭐⭐ WHY IT PRINTS NAMES RATHER THAN TIMING THEM. `[census] ggrs_driver`
/// measured ~0.95ms of `PreUpdate` sitting OUTSIDE the GGRS driver in a headless
/// Smash match — the third-largest cost in the frame. Timing each system would
/// need this crate to depend on `bevy_ui`, `bevy_picking` and leafwing purely to
/// name their sets, which is an instrument joining the population it measures.
/// The graph already knows the names; reading them costs nothing and turns
/// "DefaultPlugins is 0.95ms" into a list somebody can bracket deliberately.
///
/// ⛔ ONE-SHOT AT `PreStartup`, for the reason in `report_schedule_conditions_census`:
/// `Schedule::initialize` drains the graph, so this is readable exactly once.
fn report_schedule_membership(schedules: &Schedules, wanted: &str, at: f64) {
    for (label, schedule) in schedules.iter() {
        if format!("{label:?}") != wanted {
            continue;
        }
        let mut names: Vec<String> = schedule
            .graph()
            .systems
            .iter()
            .map(|(_key, system, _conditions)| condition_label(system.name().as_ref()))
            .collect();
        names.sort();
        // ⛔⛔ A ZERO HERE IS A DRAINED GRAPH, NOT AN EMPTY SCHEDULE. Any schedule
        // that has already run by `PreStartup` — `StateTransition` and `Startup`
        // among them — has had its systems MOVED into the private executable, so
        // this read finds nothing. Measured: this reported `StateTransition
        // systems=0` while `[census] schedules`, which falls back to
        // `systems_len()`'s executable count, reported EIGHT. Say which it is,
        // because "zero systems costing 2ms" is a conclusion somebody will draw.
        // ⭐ THE EXECUTABLE IS THE OTHER HALF OF THE ANSWER. A drained graph is not
        // an empty schedule — `Schedule::initialize` MOVES the systems into the
        // private executable, and `Schedule::systems()` reads them back from
        // exactly there. So the two accessors are complementary: the graph
        // answers before first run, the executable answers after. Reading only
        // the graph reported `StateTransition systems=0` while
        // `[census] schedules` said EIGHT, and "zero systems costing 2ms" is a
        // conclusion somebody will draw from that.
        if names.is_empty() {
            match schedule.systems() {
                Ok(systems) => {
                    names = systems
                        .map(|(_key, system)| condition_label(system.name().as_ref()))
                        .collect();
                    names.sort();
                }
                Err(_) => {
                    eprintln!(
                        "[census] membership t={at:.3} schedule={wanted} \
                         unavailable=never_initialized"
                    );
                    return;
                }
            }
        }
        eprintln!(
            "[census] membership t={at:.3} schedule={wanted} systems={} {}",
            names.len(),
            names.join(" ")
        );
        return;
    }
    eprintln!("[census] membership t={at:.3} schedule={wanted} unavailable=not_found");
}

/// WHO OWNS the registered systems — the population behind "this app installs
/// every experience it can launch".
///
/// ⭐⭐ THE COST-OWNERSHIP QUESTION, and it is the one a profiler answers worst.
/// A trace says `tick_rolling` was called 1802 times; it does not say that
/// `tick_rolling` belongs to an experience this run never entered. System names
/// carry their crate, so the schedule graph can say which crate is asking the
/// frame for work — before any of it runs, and without attributing a cost to it.
///
/// ⛔ A COUNT IS NOT A COST. A crate with 200 registered systems whose set is
/// gated costs one condition; a crate with 3 ungated ones costs 3 per frame.
/// This row says who is REGISTERED, which is the question "should a shipped
/// title carry this at all" needs, and it does not claim to answer "what did
/// the frame spend".
fn report_schedule_owners(schedules: &Schedules) {
    let mut by_owner: std::collections::BTreeMap<String, usize> = Default::default();
    let mut total = 0usize;
    for (_label, schedule) in schedules.iter() {
        for (_key, system, _conditions) in schedule.graph().systems.iter() {
            total += 1;
            *by_owner
                .entry(owning_crate(system.name().as_ref()))
                .or_default() += 1;
        }
    }
    let mut ranked: Vec<(&String, &usize)> = by_owner.iter().collect();
    ranked.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    let mut row = format!(
        "[census] owners t=0.000 systems={total} crates={}",
        by_owner.len()
    );
    for (name, count) in ranked.iter().take(20) {
        row.push_str(&format!(" {name}={count}"));
    }
    eprintln!("{row}");
}

/// The crate a system's path names, which is the closest thing the graph has to
/// an owner. `<unnamed>` for a closure with no path — those are anonymous by
/// construction and lumping them together is honest.
fn owning_crate(name: &str) -> String {
    let head = name.split('<').next().unwrap_or(name);
    match head.split("::").next() {
        Some(first) if !first.is_empty() => first.to_string(),
        _ => "<unnamed>".to_string(),
    }
}

/// The last path segment of a condition's type name, with generics dropped.
///
/// Condition names arrive as full paths with turbofish payloads
/// (`bevy_ecs::...::resource_changed<ambition_foo::Bar>`), which are unreadable
/// in a census row and would make the row width depend on crate paths. The tail
/// is what identifies the condition to a human reading the census.
fn condition_label(name: &str) -> String {
    let head = name.split('<').next().unwrap_or(name);
    head.rsplit("::").next().unwrap_or(head).to_string()
}

/// Frame times on the census interval.
///
/// The always-on `[frame-census]` line summarizes a five-second window, which
/// is the right cadence for a log somebody reads. It is the wrong cadence for
/// correlation: a two-second stall inside it is averaged away, and the row it
/// lands in cannot be lined up against a camera count sampled on a different
/// clock. This one shares the census clock, so every row here has a `camera`
/// and an `ecs` row at the same `t=`.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Default)]
pub struct FrameIntervalCensus {
    last_frame_at: Option<Instant>,
    interval_ms: Vec<f64>,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn report_frame_interval_census(
    census: Res<RuntimeCensus>,
    mut frames: ResMut<FrameIntervalCensus>,
) {
    if !census.enabled() {
        return;
    }
    let now = Instant::now();
    // The interval from app construction to frame one is startup, not a frame
    // time; counting it would put a fake multi-second spike in the first row of
    // every capture.
    if let Some(previous) = frames.last_frame_at.replace(now) {
        let ms = now.duration_since(previous).as_secs_f64() * 1000.0;
        frames.interval_ms.push(ms);
    }
    let Some(at) = census.due() else {
        return;
    };
    let mut sorted = core::mem::take(&mut frames.interval_ms);
    if sorted.is_empty() {
        return;
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let count = sorted.len();
    let mean = sorted.iter().sum::<f64>() / count as f64;
    use crate::profiling::FrameCensus;
    eprintln!(
        "[census] frame t={at:.3} frames={count} mean={mean:.2} p50={:.2} p95={:.2} p99={:.2} \
         min={:.2} max={:.2}",
        FrameCensus::percentile(&sorted, 0.50),
        FrameCensus::percentile(&sorted, 0.95),
        FrameCensus::percentile(&sorted, 0.99),
        sorted.first().copied().unwrap_or(0.0),
        sorted.last().copied().unwrap_or(0.0),
    );
}

#[cfg(target_arch = "wasm32")]
#[derive(Resource, Default)]
pub struct FrameIntervalCensus;

#[cfg(target_arch = "wasm32")]
pub fn report_frame_interval_census(
    _census: Res<RuntimeCensus>,
    _frames: ResMut<FrameIntervalCensus>,
) {
}

// ─────────────────────────────────────────────────────────────────────
// Schedule-phase census
// ─────────────────────────────────────────────────────────────────────

/// One boundary in the main schedule order.
///
/// Each is a real schedule holding exactly one system, inserted into
/// [`MainScheduleOrder`] immediately before the phase it opens (and one more
/// after the last phase). That placement is what makes the breakdown EXACT:
/// a marker system added to `Update` itself would run wherever the executor
/// happened to order it, silently charging part of `Update` to whatever ran
/// before it. A schedule's position in `MainScheduleOrder` is not ambiguous.
///
/// The index is the phase it opens, so `FramePhaseMark(0)` runs before the
/// first phase and `FramePhaseMark(n)` after the last.
#[cfg(not(target_arch = "wasm32"))]
#[derive(bevy::ecs::schedule::ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct FramePhaseMark(usize);

/// Wall time attributed to each phase of the main schedule.
///
/// ⭐ **THIS IS THE ONE FRAME BREAKDOWN THAT NEEDS NO PROFILER.** Tracy answers
/// it too, but Tracy is a desktop build whose symbol worker can cost more than
/// the game — a measured headless run spent 55% of its cycles inside the
/// profiler and 40% inside the game — and it does not exist at all on the
/// platforms where the answer matters most: web, Android, a Steam Deck in
/// someone else's hands. A handful of `Instant::now()` calls per frame put
/// "which phase owns the frame" in reach of any build that can write to stderr.
///
/// ⭐ THE PHASES ARE READ FROM [`MainScheduleOrder`], NOT HARDCODED. A census
/// that listed `First, PreUpdate, Update, PostUpdate, Last` would quietly
/// mis-attribute `StateTransition` (inserted by `bevy_state`), `SpawnScene`, and
/// any schedule a game inserts of its own. Taking the list from the app means
/// the row describes the schedule the app actually composed.
/// PROCESS CPU time, in milliseconds, or `None` where we cannot ask.
///
/// ⭐ THIS IS WHAT SEPARATES BUSY FROM BLOCKED. `Instant` measures WALL time, so
/// a phase that spends 3 ms waiting on the GPU and one that spends 3 ms working
/// are the same number — which made a real RTX 3090 capture unreadable on
/// 2026-09-01. A CPU clock does not advance while nothing runs, so a phase whose
/// wall time far exceeds its CPU time was WAITING.
///
/// ⛔⛔ PROCESS, NOT THREAD, AND THE FIRST VERSION GOT THIS WRONG.
/// `CLOCK_THREAD_CPUTIME_ID` is per-thread and Bevy does not run consecutive
/// mark schedules on one thread, so differencing across a switch invented time —
/// it read 50.963 ms of "CPU" inside a phase whose wall was 0.558 ms. The
/// process clock is thread-independent and needs no pairing.
///
/// ⚠ IT SUMS EVERY THREAD, so on a multicore machine a busy phase can report
/// MORE CPU than wall. That is the signal, not a bug: cpu/wall is roughly how
/// many cores the phase kept busy, and a ratio near zero is a stall.
#[cfg(all(unix, not(target_arch = "wasm32")))]
fn process_cpu_ms() -> Option<f64> {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid, fully-initialised `timespec` we own, and
    // `clock_gettime` only writes through the pointer we hand it.
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut ts) };
    (rc == 0).then(|| ts.tv_sec as f64 * 1000.0 + ts.tv_nsec as f64 / 1.0e6)
}

#[cfg(all(not(unix), not(target_arch = "wasm32")))]
fn process_cpu_ms() -> Option<f64> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Default)]
pub struct SchedulePhaseCensus {
    /// Phase names in schedule order, from `MainScheduleOrder`, plus a trailing
    /// `outside` for the gap after the last phase.
    names: Vec<String>,
    /// Index of the phase currently open, or `None` before the first mark.
    current: Option<usize>,
    marked_at: Option<Instant>,
    /// Main-thread CPU milliseconds at the last mark, when the clock is available.
    ///
    marked_cpu: Option<f64>,
    /// Accumulated milliseconds per phase since the last report.
    totals_ms: Vec<f64>,
    /// Accumulated main-thread CPU milliseconds per phase, parallel to `totals_ms`.
    cpu_ms: Vec<f64>,
    frames: u32,
}

#[cfg(not(target_arch = "wasm32"))]
impl SchedulePhaseCensus {
    /// The label used for time between the end of the last phase and the start
    /// of the next frame: present/vsync wait when windowed, the runner loop when
    /// headless.
    pub const OUTSIDE: &'static str = "outside";

    fn new(phase_names: Vec<String>) -> Self {
        let width = phase_names.len();
        Self {
            names: phase_names,
            current: None,
            marked_at: None,
            totals_ms: vec![0.0; width],
            marked_cpu: None,
            cpu_ms: vec![0.0; width],
            frames: 0,
        }
    }

    /// Close the open phase and open `phase`.
    fn advance_to(&mut self, phase: usize, now: Instant) {
        let cpu_now = process_cpu_ms();
        if let (Some(previous), Some(open)) = (self.marked_at, self.current) {
            if let Some(slot) = self.totals_ms.get_mut(open) {
                *slot += now.duration_since(previous).as_secs_f64() * 1000.0;
            }
            if let (Some(cpu_now), Some(cpu_prev), Some(slot)) =
                (cpu_now, self.marked_cpu, self.cpu_ms.get_mut(open))
            {
                *slot += (cpu_now - cpu_prev).max(0.0);
            }
        }
        self.current = Some(phase);
        self.marked_at = Some(now);
        self.marked_cpu = cpu_now;
    }
}

/// Build the marker system for boundary `phase`.
#[cfg(not(target_arch = "wasm32"))]
fn mark_frame_phase(phase: usize) -> impl FnMut(ResMut<SchedulePhaseCensus>) {
    move |mut phases: ResMut<SchedulePhaseCensus>| {
        // No `enabled` test: these schedules are inserted only when the census
        // is on, so reaching this system is already the answer.
        let now = Instant::now();
        let phases = phases.bypass_change_detection();
        if phase == 0 {
            phases.frames += 1;
        }
        phases.advance_to(phase, now);
    }
}

/// Report the phase breakdown on a sample frame.
///
/// Runs in the LAST mark schedule, after its marker, so every phase of the
/// frame it reports has already been closed.
#[cfg(not(target_arch = "wasm32"))]
pub fn report_schedule_phase_census(
    census: Res<RuntimeCensus>,
    mut phases: ResMut<SchedulePhaseCensus>,
) {
    let Some(at) = census.due() else {
        return;
    };
    let phases = phases.bypass_change_detection();
    if phases.frames == 0 {
        return;
    }
    // Per-frame means, because a window holds however many frames fit in it and
    // a total would move with the frame rate it is trying to explain.
    let frames = f64::from(phases.frames);
    let mut row = format!("[census] phases t={at:.3} frames={}", phases.frames);
    for (name, total_ms) in phases.names.iter().zip(&phases.totals_ms) {
        row.push_str(&format!(" {name}={:.3}", total_ms / frames));
    }
    eprintln!("{row}");
    // ⭐ THE SAME SPLIT ON THE MAIN THREAD'S CPU CLOCK. `phases` above is WALL
    // time, so a phase BLOCKED on the GPU is indistinguishable from one that is
    // busy — the reason a rendering capture's split is untrustworthy. A thread
    // CPU clock does not tick while blocked, so for each phase
    // `phases - phases_cpu` IS the stall, and `phases_cpu` alone is CPU work
    // that can be trusted even while rendering.
    if phases.cpu_ms.iter().any(|ms| *ms > 0.0) {
        let mut cpu_row = format!("[census] phases_cpu t={at:.3} frames={}", phases.frames);
        for (name, cpu_ms) in phases.names.iter().zip(&phases.cpu_ms) {
            cpu_row.push_str(&format!(" {name}={:.3}", cpu_ms / frames));
        }
        eprintln!("{cpu_row}");
    }
    for total in &mut phases.totals_ms {
        *total = 0.0;
    }
    for cpu in &mut phases.cpu_ms {
        *cpu = 0.0;
    }
    phases.frames = 0;
}

/// Install the phase marks, reading the phase list off the app's own
/// [`MainScheduleOrder`].
///
/// Returns the phase names in order. Call ONLY when the census is enabled: this
/// adds a schedule per boundary, and a schedule that runs every frame is not
/// free. The census is a measuring instrument, and it must not join the
/// population it measures when nobody asked it to.
/// Stand a boundary system after each top-level sim phase, and after each
/// sub-phase of `CoreSimulation`.
///
/// ⛔⛔ **`.after(phase)` DOES NOT PUT A MARK AT THAT PHASE'S TRAILING EDGE**, and
/// this doc said it did until 2026-09-01. `.after(A)` constrains the mark
/// against `A` and against nothing else, so `A`'s successor may start before the
/// mark runs and have part of its work billed to `A`. Every mark that CAN be
/// bracketed is now `.after(A).before(B)`.
///
/// ⚠ ALMOST ALL CAN, and my first repair got this wrong. I read one
/// `configure_sets` block, saw no `.chain()`, and declared eleven phases
/// unordered. `configure_platformer2d_simulation_phases` chains them in a
/// SECOND block further down:
///
/// ```text
/// CoreSimulation -> FeatureCollection -> FeatureInteraction -> LdtkRuntimeSpine
///   -> EncounterSimulation -> Cutscene -> GameplayEffects -> Progression
///   -> ResetProcessing -> FeatureViewSync,  then PresentationVisualSync after it
/// ```
///
/// ⛔ `Trace` is the ONLY genuinely unordered phase — `.after(CoreSimulation)`
/// and nothing else. See `SIM_PHASE_UNORDERED`.
///
/// ⛔ The names below still duplicate the
/// chain's membership, and a phase added there without a line here is simply
/// unattributed — its time lands in whichever neighbour closes next, which is
/// the honest failure mode for a boundary instrument but is a failure mode.
/// The phases the schedule declares NO order between, by bucket index.
///
/// ⛔⛔ UNMEASURED, NOT MERELY UNORDERED. `Trace` is `.after(CoreSimulation)` and
/// nothing else, and nothing is declared after it. A serial mark there advances
/// `last` and steals from whichever chain bucket closes next; an "independent"
/// pair around it is unbounded on both outer sides and measured **2.317 ms**
/// against a chain phase of 0.1 — most of the frame. There is no pair of marks
/// that bounds a phase the schedule does not bound.
///
/// ⇒ Its bucket stays ZERO and the census row says `unmeasured=Trace`. Giving it
/// a number needs spans inside its systems, not a boundary instrument.
#[cfg(not(target_arch = "wasm32"))]
pub const SIM_PHASE_UNORDERED: [usize; 1] = [20];

/// The sim-phase bucket names, in index order.
///
/// ⛔⛔ THE INDICES ARE A CROSS-CRATE CONTRACT. `SIM_PHASE_ACTOR_DECISION` is a
/// bare `usize` the actor monolith passes back to `mark_sim_phase`, so inserting
/// a name ANYWHERE above it silently re-points that mark at another bucket. The
/// test below is what makes that a build failure instead of a plausible number.
#[cfg(not(target_arch = "wasm32"))]
fn sim_phase_names() -> Vec<&'static str> {
    vec![
        "PlayerInput",
        "WorldPrep.BeforeIntegrate",
        "WorldPrep.Integrate",
        "WorldPrep.AfterIntegrate",
        "WorldPrep.ContactDamage",
        "WorldPrep",
        "PlayerSimulation",
        "RoomTransition",
        "Combat",
        "PresentationSync",
        "FeatureCollection",
        "FeatureInteraction",
        "LdtkRuntimeSpine",
        "EncounterSimulation",
        "Cutscene",
        "GameplayEffects",
        "Progression",
        "ResetProcessing",
        "FeatureViewSync",
        "PresentationVisualSync",
        "Trace",
        // ⛔⛔ APPENDED, AND THE ORDER OF THIS LIST IS NOT THE ORDER OF THE CHAIN.
        // `close(index)` bills "now minus the previous mark" into whichever
        // bucket the mark names, so attribution follows the RUNTIME order of the
        // marks and an index is only a label. These seven are closed from the
        // actor monolith, the only crate that can name `ActorDecisionSet`.
        //
        // ⭐ ALL SEVEN OR NONE. They are installed by one function in one crate,
        // so there is no arrangement where the six decision phases are marked and
        // the gate tail is not. That is what makes it safe for bucket
        // `SIM_PHASE_ACTOR_DECISION` to mean the TAIL (`Publish` through
        // `BodyMode`) rather than the whole prefix: the meaning cannot change
        // under a partial install, and the NEVER-CLOSED line catches the empty
        // one.
        "WorldPrep.Decision.Gate",
        "WorldPrep.Decision.Targeting",
        "WorldPrep.Decision.Prepare",
        "WorldPrep.Decision.Observe",
        "WorldPrep.Decision.StateMaintenance",
        "WorldPrep.Decision.Decide",
        "WorldPrep.Decision.Publish",
    ]
}

#[cfg(not(target_arch = "wasm32"))]
fn install_sim_phase_boundaries(app: &mut App) {
    use ambition_platformer2d_shared_tangle::schedule::{
        Platformer2dSimulationPhaseMonolith as Phase, SimScheduleExt as _, WorldPrepSet,
    };

    let sim = app.sim_schedule();
    // Ordered as the chain runs. `CoreSimulation`'s sub-phases come first
    // because the umbrella closes only once they all have.
    // ⛔⛔ AND `WorldPrep.BeforeIntegrate` MEANT THE WHOLE PREFIX BEFORE IT.
    // The failure mode two paragraphs down — "a phase added there without a line
    // here is simply unattributed; its time lands in whichever neighbour closes
    // next" — is one this instrument walked straight into on 2026-08-31. The
    // actor decision chain (`ActorDecisionSet::Targeting` through `Publish`) is
    // `in_set(Phase::WorldPrep)` and `before(WorldPrepSet::BeforeIntegrate)`, so
    // with no mark between `PlayerInput` and `BeforeIntegrate` all six of its
    // sets billed into bucket 1 under a name that says otherwise. The 1.214
    // ms/tick that name carried was read as a movement-prep cost and used to
    // rule a physics engine in or out; it is a prefix, and most of it may be
    // cognition.
    //
    // ⇒ bucket `SIM_PHASE_ACTOR_DECISION` closes after `Publish`, so bucket 1
    // means what it says. `report_sim_phase_census` says so out loud when that
    // mark never fired, because a bucket silently changing meaning depending on
    // which plugins are installed is the defect, not the fix.
    //
    // ⛔⛔ `WorldPrep` IS SPLIT, AND THE HALL IS WHY. A windowed capture on
    // 2026-08-31 walked into `hall_of_characters` and the frame went
    // 7.91ms -> 10.52ms; 91% of the SIMULATION's share of that was `WorldPrep`
    // alone, +1.546 ms/tick, while every other phase stayed flat. One number for
    // a phase containing four sets could say THAT it grew and never WHICH part
    // of it did — and the two candidates want opposite fixes: the body-contact
    // pairing is O(n^2) and wants a broadphase, the movement kernel is O(n) and
    // wants a smaller constant. The sub-sets below are what tells them apart.
    app.insert_resource(SimPhaseCensus::with_names(sim_phase_names()));
    crate::perception_census::enable();

    // ⛔ THE OPENING MARK COMES FIRST, and it is what makes bucket 0 mean
    // `PlayerInput` rather than `PlayerInput plus the whole preceding frame`.
    app.add_systems(sim, open_sim_phase_window.before(Phase::PlayerInput));
    // ⛔⛔ EACH MARK NEEDS BOTH EDGES, AND THESE HAD ONLY ONE UNTIL 2026-09-01.
    // `.after(A)` alone says the mark runs after `A`; it says NOTHING about the
    // mark and `A`'s successor, so the scheduler may start `B` before the mark
    // fires and bill part of `B` into `A`'s bucket. The actor-decision marks in
    // `features/mod.rs` already carried this reasoning and both edges; the
    // parent marks did not, and the comment here claimed `.after(phase)` put a
    // mark "exactly at that phase's trailing edge". It does not.
    //
    // A `.before(next)` is only available where the schedule actually DECLARES a
    // successor. Two groups here do:
    //
    //   PlayerInput -> WorldPrep -> PlayerSimulation -> RoomTransition
    //                -> Combat -> PresentationSync        (CoreSimulation, chained)
    //   BeforeIntegrate -> Integrate -> AfterIntegrate    (WorldPrep, chained)
    //
    // And a THIRD, which my first repair missed by reading only the first
    // `configure_sets` block in that file:
    //
    //   CoreSimulation -> FeatureCollection -> ... -> ResetProcessing
    //     -> FeatureViewSync -> PresentationVisualSync
    //
    // Only `Trace` is genuinely unordered; it gets an independent clock below.
    app.add_systems(
        sim,
        mark_sim_phase(0)
            .after(Phase::PlayerInput)
            .before(Phase::WorldPrep),
    );
    // ⭐ THE SUB-SETS CLOSE BEFORE THEIR UMBRELLA. Each mark bills the span since
    // the previous one, so bucket 5 (`WorldPrep`) ends up holding only what ran
    // inside the phase but in NONE of its four sets — which is a real quantity
    // worth seeing, not a rounding error: `snapshot_body_contact` and the
    // monolith's own `WorldPrep` systems are exactly that.
    //
    // ⚠ `AfterIntegrate` and `ContactDamage` are deliberately NOT chained to each
    // other (see `WorldPrepSet`), so their marks record the order the schedule
    // actually resolved, not an order this instrument imposed.
    app.add_systems(
        sim,
        mark_sim_phase(1)
            .after(WorldPrepSet::BeforeIntegrate)
            .before(WorldPrepSet::Integrate),
    );
    app.add_systems(
        sim,
        mark_sim_phase(2)
            .after(WorldPrepSet::Integrate)
            .before(WorldPrepSet::AfterIntegrate),
    );
    app.add_systems(
        sim,
        mark_sim_phase(3)
            .after(WorldPrepSet::AfterIntegrate)
            .before(WorldPrepSet::ContactDamage),
    );
    app.add_systems(sim, mark_sim_phase(4).after(WorldPrepSet::ContactDamage));
    app.add_systems(
        sim,
        mark_sim_phase(5)
            .after(Phase::WorldPrep)
            .before(Phase::PlayerSimulation),
    );
    app.add_systems(
        sim,
        mark_sim_phase(6)
            .after(Phase::PlayerSimulation)
            .before(Phase::RoomTransition),
    );
    app.add_systems(
        sim,
        mark_sim_phase(7)
            .after(Phase::RoomTransition)
            .before(Phase::Combat),
    );
    app.add_systems(
        sim,
        mark_sim_phase(8)
            .after(Phase::Combat)
            .before(Phase::PresentationSync),
    );
    app.add_systems(
        sim,
        mark_sim_phase(9)
            .after(Phase::PresentationSync)
            .before(Phase::FeatureCollection),
    );
    app.add_systems(
        sim,
        mark_sim_phase(10)
            .after(Phase::FeatureCollection)
            .before(Phase::FeatureInteraction),
    );
    app.add_systems(
        sim,
        mark_sim_phase(11)
            .after(Phase::FeatureInteraction)
            .before(Phase::LdtkRuntimeSpine),
    );
    app.add_systems(
        sim,
        mark_sim_phase(12)
            .after(Phase::LdtkRuntimeSpine)
            .before(Phase::EncounterSimulation),
    );
    app.add_systems(
        sim,
        mark_sim_phase(13)
            .after(Phase::EncounterSimulation)
            .before(Phase::Cutscene),
    );
    app.add_systems(
        sim,
        mark_sim_phase(14)
            .after(Phase::Cutscene)
            .before(Phase::GameplayEffects),
    );
    app.add_systems(
        sim,
        mark_sim_phase(15)
            .after(Phase::GameplayEffects)
            .before(Phase::Progression),
    );
    app.add_systems(
        sim,
        mark_sim_phase(16)
            .after(Phase::Progression)
            .before(Phase::ResetProcessing),
    );
    app.add_systems(
        sim,
        mark_sim_phase(17)
            .after(Phase::ResetProcessing)
            .before(Phase::FeatureViewSync),
    );
    app.add_systems(
        sim,
        mark_sim_phase(18)
            .after(Phase::FeatureViewSync)
            .before(Phase::PresentationVisualSync),
    );
    app.add_systems(sim, mark_sim_phase(19).after(Phase::PresentationVisualSync));
    // ⛔⛔ TRACE IS NOT TIMED, AND CANNOT BE BY THIS INSTRUMENT.
    //
    // It is `.after(CoreSimulation)` and nothing else — nothing is declared
    // after it — so every pair of marks I can place has an unbounded side. A
    // serial mark steals from whichever chain bucket closes next; an
    // "independent" pair `open.before(Trace)` / `close.after(Trace)` is
    // unbounded on BOTH outer sides, which is the very defect this repair was
    // about. It read **2.317 ms** against a chain phase of 0.1 — it was timing
    // most of the frame, not Trace.
    //
    // ⇒ Bucket 20 stays ZERO and the row says `unmeasured=Trace`. A phase
    // nothing orders cannot be given a wall-time slice by a boundary
    // instrument; measuring it needs its own spans, not a mark.
}

#[cfg(not(target_arch = "wasm32"))]
fn install_frame_phase_marks(app: &mut App) -> Vec<String> {
    use bevy::app::MainScheduleOrder;
    use bevy::ecs::schedule::ScheduleLabel;

    let phases: Vec<_> = app
        .world()
        .resource::<MainScheduleOrder>()
        .labels
        .iter()
        .map(|label| label.intern())
        .collect();
    // `{:?}` on a schedule label is its type name, which is exactly the phase
    // name a reader wants: `First`, `Update`, `StateTransition`.
    let mut names: Vec<String> = phases.iter().map(|label| format!("{label:?}")).collect();
    names.push(SchedulePhaseCensus::OUTSIDE.to_string());

    for (index, _) in phases.iter().enumerate() {
        app.add_systems(FramePhaseMark(index), mark_frame_phase(index));
    }
    // The trailing boundary closes the final phase and opens `outside`, which
    // the next frame's boundary 0 closes in turn. The report rides here so
    // every phase of the frame it prints has already been closed.
    app.add_systems(
        FramePhaseMark(phases.len()),
        (mark_frame_phase(phases.len()), report_schedule_phase_census).chain(),
    );

    // ⛔ NOT `MainScheduleOrder::insert_before`. It locates the anchor with
    // `(**current).eq(&before)`, which downcasts to the anchor's CONCRETE type;
    // handing it the `InternedScheduleLabel` we just read back out of `labels`
    // fails that downcast and panics with "Expected First to exist". The list
    // is public, so interleave it directly -- which is also the whole order in
    // one pass instead of N searches.
    let mut interleaved = Vec::with_capacity(phases.len() * 2 + 1);
    for (index, phase) in phases.iter().enumerate() {
        interleaved.push(FramePhaseMark(index).intern());
        interleaved.push(*phase);
    }
    interleaved.push(FramePhaseMark(phases.len()).intern());
    app.world_mut().resource_mut::<MainScheduleOrder>().labels = interleaved;

    names
}

#[cfg(target_arch = "wasm32")]
#[derive(Resource, Default)]
pub struct SchedulePhaseCensus;

#[cfg(target_arch = "wasm32")]
#[derive(Resource, Default)]
pub struct SimPhaseCensus;

/// Register the census clock and the sim-side censuses.
///
/// Sim-side means headless too: a VM with no GPU still gets entity, body, and
/// schedule counts on the same clock as its Tracy and `perf` captures.
pub struct RuntimeCensusPlugin;

impl Plugin for RuntimeCensusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RuntimeCensus>();
        app.init_resource::<FrameIntervalCensus>();
        app.add_systems(First, advance_runtime_census);
        // ⛔ PreStartup, and the phase is load-bearing: `Update` and the sim
        // schedule have not initialized yet, so their graphs still hold the
        // conditions this counts. One frame later they are gone.
        app.add_systems(PreStartup, report_schedule_conditions_census);
        let enabled = app.world().resource::<RuntimeCensus>().enabled();

        // ⛔⛔ THE REPORTERS ARE GATED AT BUILD TIME TOO, and until now they were
        // not — nine of them across this crate and `ambition_render` were
        // registered unconditionally, two lines from the comment below saying an
        // instrument must not join the population it measures. Each early-returns
        // on `census.due()`, so the cost was small; the point is that it was not
        // ZERO, and a census that installs itself into every shipped frame to
        // discover it was not asked for is the exact shape this campaign spent
        // eleven probes cataloguing elsewhere.
        //
        // ⭐ PROVABLY BEHAVIOUR-PRESERVING: `due_at` is only ever set while
        // enabled, so a disabled census could never have reported anything.
        if enabled {
            app.add_systems(
                Last,
                (
                    report_frame_interval_census,
                    report_ecs_census,
                    report_schedule_load_census,
                    report_entity_populations,
                    // ⭐ Names the SIM schedule's systems, which the `PreStartup`
                    // pass structurally cannot: that schedule does not exist
                    // until a session activates. Latches after one report.
                    report_sim_schedule_membership,
                ),
            );
        }

        // ⛔ THE BOUNDARIES ARE NOT REGISTERED WHEN THE CENSUS IS OFF. They run
        // inside the SIM schedule — the hottest schedule in the app — and an
        // instrument must not join the population it measures when nobody asked.
        #[cfg(not(target_arch = "wasm32"))]
        if enabled {
            install_sim_phase_boundaries(app);
            // ⛔⛔ THE REPORTER GOES HERE, NOT IN THE UNCONDITIONAL LIST. It takes
            // `ResMut<SimPhaseCensus>`, and that resource is inserted by
            // `install_sim_phase_boundaries` — which only runs when the census
            // is on. Registered unconditionally it panics with "Resource does
            // not exist" on every run with the census OFF, which is every test
            // in the suite. An instrument that is not asked for must not be
            // able to stop the game.
            app.add_systems(Last, report_sim_phase_census);
        }

        // ⛔ THE PHASE MARKS ARE NOT REGISTERED WHEN THE CENSUS IS OFF. Every
        // other census here costs one bool test on a frame it does not sample.
        // These are SCHEDULES, one per boundary, and a schedule that runs every
        // frame is not free. An instrument must not join the population it
        // measures when nobody asked it to.
        #[cfg(not(target_arch = "wasm32"))]
        if enabled {
            let names = install_frame_phase_marks(app);
            eprintln!("[census] config t=0.000 phases={}", names.join(","));
            app.insert_resource(SchedulePhaseCensus::new(names));
        }

        if enabled {
            let census = app.world().resource::<RuntimeCensus>();
            eprintln!(
                "[census] config t=0.000 interval_s={:.3} source={CENSUS_ENV}",
                census.interval_s()
            );
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// The panel and the log row read the SAME number.
    ///
    /// ⭐⭐ THIS IS THE ONE PROPERTY THE DIAGNOSTICS WORK IS FOR. The campaign's
    /// architecture is one measurement with several consumers; the failure it
    /// prevents is two counts with one name, which is unattributable the moment
    /// they disagree. Both the census printer and the diagnostics publisher take
    /// [`EcsPopulation`], so this asserts the published value equals what the
    /// param reports in the same frame.
    ///
    /// ⛔ AND IT MOVES THE POPULATION, because a publisher that always wrote a
    /// constant would satisfy a single-sample check.
    #[test]
    fn the_published_entity_counts_are_the_ones_the_census_param_reports() {
        #[derive(Resource, Default)]
        struct Sampled((usize, usize));

        fn sample(population: EcsPopulation, mut out: ResMut<Sampled>) {
            out.0 = (population.scene_entities(), population.resource_entities());
        }

        fn published(app: &App) -> (f64, f64) {
            let store = app.world().resource::<bevy::diagnostic::DiagnosticsStore>();
            let read = |path| {
                store
                    .get(path)
                    .and_then(|d| d.value())
                    .expect("the diagnostic is registered and has been measured")
            };
            (read(&SCENE_ENTITIES), read(&RESOURCE_ENTITIES))
        }

        let mut app = App::new();
        app.init_resource::<Sampled>();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_plugins(bevy::diagnostic::DiagnosticsPlugin);
        app.add_plugins(EcsDiagnosticsPlugin);
        app.add_systems(Update, sample);
        // The publisher is PACED (see ECS_DIAGNOSTIC_SAMPLE_PERIOD), so a test
        // about WHAT it publishes has to hand it enough clock to publish at all.
        // One update per period keeps this test's "publish every update" shape.
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            ECS_DIAGNOSTIC_SAMPLE_PERIOD,
        ));

        // ⛔ THE FIRST `update()` HAS A ZERO DELTA. `Time<Real>` has no previous
        // instant to subtract on the first pass, so even `ManualDuration` reports
        // 0ns there and no timed run condition fires. Every update after it
        // advances by a full period, so one priming frame buys "publishes on
        // every update" for the rest of the test.
        app.update();
        app.update();
        let (scene, resources) = app.world().resource::<Sampled>().0;
        assert_eq!(
            published(&app),
            (scene as f64, resources as f64),
            "the published values must be the param's own"
        );

        // Move BOTH populations, then re-read: a publisher wired to a constant
        // agrees with the first sample and not with this one.
        app.world_mut().spawn_empty();
        app.world_mut().spawn_empty();
        app.init_resource::<Sampled>();
        app.update();
        let (scene_after, resources_after) = app.world().resource::<Sampled>().0;
        assert_eq!(
            scene_after,
            scene + 2,
            "premise: the two spawns must have moved the scene population"
        );
        assert_eq!(
            published(&app),
            (scene_after as f64, resources_after as f64),
            "the published values must still be the param's own after it moved"
        );
    }

    /// The ECS populations are counted on a CADENCE, not on every frame.
    ///
    /// ⛔ THIS IS A COST GUARD, NOT A FEATURE. `publish_ecs_diagnostics` used to
    /// sit bare in `Update`, so a panel nobody had opened billed the frame loop
    /// for three world counts on every visible frame. Deleting the run condition
    /// makes this test red on its first assertion.
    ///
    /// The arms STRADDLE the period deliberately: many short frames must not
    /// publish, and one frame past the period must.
    #[test]
    fn the_ecs_populations_are_sampled_on_a_cadence_not_every_frame() {
        fn published(app: &App) -> Option<f64> {
            app.world()
                .resource::<bevy::diagnostic::DiagnosticsStore>()
                .get(&SCENE_ENTITIES)
                .and_then(|d| d.value())
        }

        let short = ECS_DIAGNOSTIC_SAMPLE_PERIOD / 10;
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_plugins(bevy::diagnostic::DiagnosticsPlugin);
        app.add_plugins(EcsDiagnosticsPlugin);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(short));

        // ⛔ THE FIRST `update()` ADVANCES THE CLOCK BY ZERO — `Time<Real>` has
        // no previous instant to subtract on the first pass — so ten frames buy
        // only nine tenths of a period. That off-by-one is exactly the kind of
        // thing that makes a cadence test pass for the wrong reason, so the
        // arms are counted from it rather than around it.
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(
            published(&app),
            None,
            "ten frames worth nine tenths of a sample period must not have \
             counted the world"
        );

        // The eleventh brings the total to a full period, and crosses it.
        app.update();
        assert!(
            published(&app).is_some(),
            "premise: crossing the sample period must publish, or the arm above \
             is only measuring a publisher that never runs"
        );
    }

    /// The scene count must not move when a RESOURCE is added.
    ///
    /// ⛔⛔ THE DEFECT THIS PINS SHIPPED. Until 2026-08-31 this census read an
    /// unfiltered `Query<()>` and called the answer "the number of entities
    /// that are actually there". Under Bevy 0.18 that was true. Under 0.19 a
    /// resource is an entity, so the row reported scene content plus every
    /// registered resource — a constant, invisible inflation of every
    /// performance note taken from it.
    ///
    /// The two arms are what make this a measurement rather than an assertion:
    /// spawning a resource must move `resource_entities` and must NOT move
    /// `scene_entities`, and spawning an ordinary entity must do the opposite.
    /// A test that only checked the first would pass against a query that
    /// counted nothing at all.
    #[test]
    fn a_resource_is_not_scene_content() {
        #[derive(Resource, Default)]
        struct OnlyForThisTest;

        fn sample(population: EcsPopulation, mut out: ResMut<Sampled>) {
            out.0 = (population.scene_entities(), population.resource_entities());
        }

        #[derive(Resource, Default)]
        struct Sampled((usize, usize));

        let mut app = App::new();
        app.init_resource::<Sampled>();
        app.add_systems(Update, sample);
        app.update();
        let (scene_before, resources_before) = app.world().resource::<Sampled>().0;

        // A RESOURCE: `resources` moves, `scene` does not.
        app.init_resource::<OnlyForThisTest>();
        app.update();
        let (scene_after_resource, resources_after_resource) = app.world().resource::<Sampled>().0;
        assert_eq!(
            scene_after_resource, scene_before,
            "a resource is not scene content, but the scene count moved \
             {scene_before} -> {scene_after_resource}"
        );
        assert_eq!(
            resources_after_resource,
            resources_before + 1,
            "the resource population must see the resource this test just added"
        );

        // AN ENTITY: `scene` moves, `resources` does not. Without this arm a
        // query matching nothing would pass the assertion above.
        app.world_mut().spawn_empty();
        app.update();
        let (scene_after_entity, resources_after_entity) = app.world().resource::<Sampled>().0;
        assert_eq!(
            scene_after_entity,
            scene_after_resource + 1,
            "the scene population must see the entity this test just spawned"
        );
        assert_eq!(
            resources_after_entity, resources_after_resource,
            "an ordinary entity is not a resource"
        );
    }

    #[test]
    fn the_gate_is_off_without_the_environment_variable() {
        // The default construction path is what a shipped run takes.
        let census = RuntimeCensus {
            enabled: false,
            interval_s: 1.0,
            started_at: Instant::now(),
            next_at: 0.0,
            due_at: None,
        };
        assert!(!census.enabled());
        assert!(census.due().is_none());
    }

    #[test]
    fn every_phase_gets_a_mark_before_it_and_one_after_the_last() {
        // ⛔ REGRESSION GUARD. The first version of this used
        // `MainScheduleOrder::insert_before`, which finds its anchor by
        // downcasting to the anchor's CONCRETE label type. Handing it an
        // `InternedScheduleLabel` read back out of `labels` fails that
        // downcast, and the app panicked at startup with "Expected First to
        // exist" -- only reachable with the census enabled, so no test that
        // left it off would have seen it.
        use bevy::app::MainScheduleOrder;

        let mut app = App::new();
        app.init_resource::<MainScheduleOrder>();
        let before: Vec<String> = app
            .world()
            .resource::<MainScheduleOrder>()
            .labels
            .iter()
            .map(|label| format!("{label:?}"))
            .collect();

        let names = install_frame_phase_marks(&mut app);

        assert_eq!(
            names.len(),
            before.len() + 1,
            "one name per phase, plus `outside` for the gap after the last"
        );
        assert_eq!(
            &names[..before.len()],
            &before[..],
            "phase names come from the app's own order"
        );
        assert_eq!(
            names.last().map(String::as_str),
            Some(SchedulePhaseCensus::OUTSIDE)
        );

        let after = app.world().resource::<MainScheduleOrder>().labels.clone();
        assert_eq!(
            after.len(),
            before.len() * 2 + 1,
            "a mark before every phase and one trailing mark"
        );
        for (index, phase) in before.iter().enumerate() {
            assert_eq!(
                format!("{:?}", after[index * 2]),
                format!("{:?}", FramePhaseMark(index)),
                "phase {phase} must be preceded by its own mark, or the boundary \
                 it reports is wherever the executor happened to run a system"
            );
            assert_eq!(format!("{:?}", after[index * 2 + 1]), *phase);
        }
    }

    #[test]
    fn a_phase_row_accounts_for_the_whole_frame() {
        // The census earns trust by summing to the frame time it explains, so
        // the transition bookkeeping must lose nothing between marks.
        let mut census = SchedulePhaseCensus::new(vec!["a".into(), "b".into(), "outside".into()]);
        let start = Instant::now();
        census.advance_to(0, start);
        census.advance_to(1, start + core::time::Duration::from_millis(2));
        census.advance_to(2, start + core::time::Duration::from_millis(5));
        census.advance_to(0, start + core::time::Duration::from_millis(9));

        assert_eq!(census.totals_ms[0].round(), 2.0);
        assert_eq!(census.totals_ms[1].round(), 3.0);
        assert_eq!(census.totals_ms[2].round(), 4.0);
        let summed: f64 = census.totals_ms.iter().sum();
        assert_eq!(
            summed.round(),
            9.0,
            "the phases must account for the whole span"
        );
    }

    #[test]
    fn only_the_sample_frame_is_due() {
        let mut app = App::new();
        app.insert_resource(RuntimeCensus {
            enabled: true,
            interval_s: 3600.0,
            started_at: Instant::now(),
            next_at: 0.0,
            due_at: None,
        });
        app.add_systems(Update, advance_runtime_census);
        app.update();
        assert!(
            app.world().resource::<RuntimeCensus>().due().is_some(),
            "the first frame after enabling is a sample frame"
        );
        app.update();
        assert!(
            app.world().resource::<RuntimeCensus>().due().is_none(),
            "a frame inside the interval must not re-sample, or every census row \
             would be per-frame and the census would become the workload"
        );
    }

    #[test]
    fn truthiness_rejects_the_shapes_a_shell_exports_for_off() {
        for off in ["0", "false", "no", "off", "", "  "] {
            assert!(!env_is_truthy(off), "{off:?} must not enable the census");
        }
        for on in ["1", "true", "yes", "on"] {
            assert!(env_is_truthy(on), "{on:?} must enable the census");
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod sim_phase_bracket_tests {
    //! Every sim-phase mark that CAN be bracketed must carry both edges.
    //!
    //! ⛔⛔ **THE DEFECT THIS PINS SHIPPED IN NINETEEN MARKS AT ONCE**, while the
    //! actor-decision marks one level down carried the correct reasoning in a
    //! comment: *"EACH MARK NEEDS BOTH EDGES. `.after(Targeting)` alone has no
    //! upper bound."* The parent marks were all `.after(phase)` and this file's
    //! own doc asserted that put them "exactly at that phase's trailing edge".
    //! It does not: the successor may start before the mark runs, and its work
    //! is then billed to the previous bucket.
    //!
    //! ⚠ A SOURCE guard, not a schedule-graph proof. It cannot tell you the
    //! `.before` names the RIGHT successor — only that a mark which could be
    //! bracketed is not left one-sided. The chain it must agree with lives in
    //! `configure_platformer2d_simulation_phases`.

    const SOURCE: &str = include_str!("runtime_census.rs");

    /// Marks the schedule gives no successor to, and why.
    ///
    /// `4` is `WorldPrepSet::ContactDamage`, attached with `.after(AfterIntegrate)`
    /// and deliberately not chained — *"a LABEL, not a chain position: chaining it
    /// would add edges nobody chose."* `9` is `PresentationSync`, the last link of
    /// the `CoreSimulation` chain. `10..=18` are the `GameplaySimulationRoot`
    /// phases, configured with no `.chain()` at all.
    fn has_no_declared_successor(index: usize) -> bool {
        // 4  = `WorldPrepSet::ContactDamage`, attached `.after(AfterIntegrate)`
        //      and deliberately not chained.
        // 19 = `PresentationVisualSync`, the end of the top-level chain.
        // 20 = `Trace`, which has no serial mark at all — an independent clock.
        index == 4 || index == 19 || super::SIM_PHASE_UNORDERED.contains(&index)
    }

    fn installations() -> Vec<(usize, String)> {
        let mut out = Vec::new();
        for (at, _) in SOURCE.match_indices("mark_sim_phase(") {
            let rest = &SOURCE[at..];
            let open = rest.find('(').unwrap() + 1;
            let close = rest.find(')').unwrap();
            let Ok(index) = rest[open..close].trim().parse::<usize>() else {
                continue;
            };
            let end = rest.find(");").map(|e| e + 2).unwrap_or(rest.len().min(400));
            out.push((index, rest[..end].to_string()));
        }
        out
    }

    #[test]
    fn every_bracketable_mark_carries_both_edges() {
        let found = installations();
        assert!(
            found.len() >= 19,
            "expected the parent marks to be found by text; got {} — if the \
             installation shape changed, this guard stopped guarding",
            found.len()
        );
        for (index, text) in &found {
            if has_no_declared_successor(*index) {
                continue;
            }
            assert!(text.contains(".after("), "mark {index} lost its lower bound");
            assert!(
                text.contains(".before("),
                "mark {index} is one-sided: `.after(..)` alone lets the next \
                 phase start before the mark fires, billing its work to the \
                 previous bucket. Give it `.before(<successor>)`, or add it to \
                 `has_no_declared_successor` with the reason.\n{text}"
            );
        }
    }

    #[test]
    fn the_unbracketable_marks_are_declared_and_not_merely_missing() {
        // ⛔ PREMISE GUARD. Without this, declaring every index successor-less
        // would make the test above pass vacuously.
        let bracketable: Vec<usize> = installations()
            .into_iter()
            .map(|(i, _)| i)
            .filter(|i| !has_no_declared_successor(*i))
            .collect();
        assert!(
            bracketable.len() >= 8,
            "both real chains' marks must still be bracketed; got {bracketable:?}"
        );
    }

    #[test]
    fn only_trace_is_unordered_and_it_names_a_bucket() {
        assert_eq!(super::SIM_PHASE_UNORDERED.len(), 1);
        let names = super::sim_phase_names();
        for index in super::SIM_PHASE_UNORDERED {
            assert!(index < names.len(), "unordered index {index} names no bucket");
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod sim_phase_index_tests {
    use super::*;

    /// ⛔⛔ **A CROSS-CRATE INDEX WITH NOTHING HOLDING IT.**
    /// `SIM_PHASE_ACTOR_DECISION` is a bare `usize` that the actor monolith
    /// hands back to `mark_sim_phase`. Inserting a name above it re-points that
    /// mark at a neighbouring bucket, and nothing would fail — the census would
    /// simply publish cognition time under some other phase's name, which is the
    /// precise defect this bucket was added to end.
    #[test]
    fn the_actor_decision_index_still_names_the_actor_decision_bucket() {
        let names = sim_phase_names();
        assert_eq!(
            names.get(SIM_PHASE_ACTOR_DECISION).copied(),
            Some("WorldPrep.Decision.Gate"),
            "the monolith closes bucket {SIM_PHASE_ACTOR_DECISION}; a name inserted \
             above it points that mark at the wrong phase"
        );
    }

    /// Every decision-phase index names the phase its constant is named for.
    ///
    /// ⛔⛔ SIX CONSECUTIVE INDICES IS A TRANSPOSITION WAITING TO HAPPEN, and a
    /// transposed pair does not fail — it publishes `Targeting`'s milliseconds
    /// under `Decide`'s name, which is the one thing this whole split exists to
    /// get right. Two hypotheses are being separated here and they live in those
    /// two buckets.
    #[test]
    fn each_decision_index_names_its_own_phase() {
        let names = sim_phase_names();
        for (index, expected) in [
            (SIM_PHASE_DECISION_TARGETING, "WorldPrep.Decision.Targeting"),
            (SIM_PHASE_DECISION_PREPARE, "WorldPrep.Decision.Prepare"),
            (SIM_PHASE_DECISION_OBSERVE, "WorldPrep.Decision.Observe"),
            (
                SIM_PHASE_DECISION_STATE_MAINTENANCE,
                "WorldPrep.Decision.StateMaintenance",
            ),
            (SIM_PHASE_DECISION_DECIDE, "WorldPrep.Decision.Decide"),
            (SIM_PHASE_DECISION_PUBLISH, "WorldPrep.Decision.Publish"),
        ] {
            assert_eq!(
                names.get(index).copied(),
                Some(expected),
                "bucket {index} must be {expected}"
            );
        }
    }

    /// Premise guard: the list must not have been trimmed to make the above pass.
    #[test]
    fn every_marked_bucket_has_a_name() {
        let names = sim_phase_names();
        assert_eq!(
            names.len(),
            SIM_PHASE_DECISION_PUBLISH + 1,
            "the last decision bucket is the last one; a total with no name is \
             accumulated and never reported"
        );
        assert_eq!(names[1], "WorldPrep.BeforeIntegrate");
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod mark_without_census_tests {
    use super::*;

    /// ⛔⛔ **A MARK WITHOUT A CENSUS MUST NOT STOP THE GAME.**
    ///
    /// `mark_sim_phase` is `pub` so a crate that owns a set this one cannot name
    /// can close its own bucket. That crate cannot see the `if enabled` guard in
    /// `RuntimeCensusPlugin::build`, and the actor monolith duly registered seven
    /// marks unconditionally: every run with `AMBITION_PROFILE_CENSUS` unset —
    /// every run a player makes — panicked with "Resource does not exist".
    ///
    /// The `Option` is what makes that impossible rather than merely documented.
    #[test]
    fn a_mark_runs_harmlessly_with_no_census_resource() {
        let mut app = App::new();
        app.add_systems(Update, mark_sim_phase(SIM_PHASE_ACTOR_DECISION));
        assert!(
            !app.world().contains_resource::<SimPhaseCensus>(),
            "the premise: this App never installed the census"
        );
        app.update();
        app.update();
    }

    /// Premise guard: with a census present the mark still records.
    ///
    /// Without this, `mark_sim_phase` could have been emptied out entirely and
    /// the arm above would still pass.
    #[test]
    fn a_mark_still_closes_its_bucket_when_the_census_is_there() {
        let mut app = App::new();
        app.insert_resource(SimPhaseCensus::with_names(sim_phase_names()));
        app.add_systems(
            Update,
            (
                open_sim_phase_window,
                mark_sim_phase(SIM_PHASE_ACTOR_DECISION),
            )
                .chain(),
        );
        app.update();
        let census = app.world().resource::<SimPhaseCensus>();
        assert_eq!(census.ticks, 1, "the window opened");
        assert!(
            census.totals[SIM_PHASE_ACTOR_DECISION] > 0.0,
            "and the bucket was closed with a real span"
        );
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod partial_window_tests {
    use super::*;

    /// ⛔⛔ **THE ONE-TICK STARTUP WINDOW WAS AVERAGED INTO PUBLISHED NUMBERS.**
    /// Its phases all read 0.000 — not because they are free, but because almost
    /// nothing had run between the census opening and its first due time. A
    /// 1200-tick capture produces about three windows, one of which was that:
    ///
    /// ```text
    /// (0.000 + 0.341 + 0.332) / 3 = 0.224
    /// ```
    ///
    /// 0.224 was published as the hall's `Decide` cost against a true 0.341, and
    /// the same bias sat under a population curve, a density sweep and three A/B
    /// decompositions — worst at low populations, where runs are shortest.
    #[test]
    fn a_single_tick_window_is_not_a_reportable_mean() {
        let mut census = SimPhaseCensus::with_names(sim_phase_names());
        census.open();
        census.close(0);
        assert_eq!(census.ticks, 1);
        assert!(
            !census.is_reportable(),
            "one tick is a sample, not a mean; reporting it publishes a zero for \
             every phase that had not run yet"
        );

        census.open();
        census.close(0);
        assert!(
            census.is_reportable(),
            "two ticks is the smallest window that can be averaged"
        );
    }

    /// And the suppressed window's time must survive into the next one.
    ///
    /// ⭐ THIS IS THE HALF THAT MAKES SUPPRESSION HONEST rather than lossy. The
    /// reporter returns BEFORE its reset, so a skipped window keeps accumulating
    /// — if it reset instead, suppressing would silently delete real elapsed
    /// time from the run.
    #[test]
    fn a_suppressed_window_keeps_its_accumulation() {
        let mut census = SimPhaseCensus::with_names(sim_phase_names());
        census.open();
        census.close(0);
        let after_one = census.totals[0];
        assert!(after_one > 0.0, "the premise: closing a phase records time");

        census.open();
        census.close(0);
        assert!(
            census.totals[0] > after_one,
            "the second tick must ADD to the first, not replace it: a suppressed \
             window that reset would delete elapsed time from the run"
        );
        assert_eq!(census.ticks, 2, "and both ticks are counted");
    }
}
