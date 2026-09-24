//! Profiling-only workload census: what Ambition asked the engine to do.
//!
//! A native profile shows that the engine was expensive. It cannot show that
//! the game presented three world cameras and refreshed two portal captures at
//! that time. These censuses write that workload to stderr on one shared clock,
//! so you can read a slow interval in a perf/Tracy capture against the scene.
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
//! `AMBITION_PROFILE_CENSUS` once at startup. When it is unset, each census
//! system does one bool test per frame and returns. The sample cadence
//! (`AMBITION_PROFILE_CENSUS_HZ`, default 1 Hz) bounds the enabled cost: no
//! census iterates a per-entity population on a frame that is not a sample
//! frame. See `docs/recipes/profiling.md`.

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
// The `cfg` applies only to `Instant`. Keep unconditional imports above it, or
// they disappear on wasm.
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

/// Environment variable that turns the censuses on.
pub const CENSUS_ENV: &str = "AMBITION_PROFILE_CENSUS";
/// Environment variable that overrides the sample rate, in samples per second.
pub const CENSUS_HZ_ENV: &str = "AMBITION_PROFILE_CENSUS_HZ";

/// The shared census clock and gate.
///
/// One resource decides both whether a census runs and which frame is a sample
/// frame. Thus every row in a frame has the same `t=`, and rows from different
/// crates line up. A census with its own timer would drift.
#[derive(Resource)]
pub struct RuntimeCensus {
    enabled: bool,
    interval_s: f64,
    #[cfg(not(target_arch = "wasm32"))]
    started_at: Instant,
    // Gated like `started_at`: its only reader, `advance_runtime_census`, is not
    // built on wasm.
    #[cfg(not(target_arch = "wasm32"))]
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
            #[cfg(not(target_arch = "wasm32"))]
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
    /// Census systems branch only on this: `let Some(at) = census.due() else {
    /// return; }` keeps a disabled or off-cadence frame at one bool test.
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
/// Since Bevy 0.19 a resource is an entity, so "entity count" is ambiguous.
/// Two paths whose names state their population remove that ambiguity.
pub const SCENE_ENTITIES: DiagnosticPath = DiagnosticPath::const_new("ambition/ecs/scene_entities");

/// Entities that exist only to hold a resource value. See [`SCENE_ENTITIES`].
pub const RESOURCE_ENTITIES: DiagnosticPath =
    DiagnosticPath::const_new("ambition/ecs/resource_entities");

/// Bodies the physics world is stepping.
pub const BODIES: DiagnosticPath = DiagnosticPath::const_new("ambition/ecs/bodies");

/// Register Ambition's ECS diagnostics and keep them fed.
///
/// This publishes from [`EcsPopulation`], the same param that
/// `report_ecs_census` prints from, so the log row and the F1 panel agree.
///
/// It is not gated on `AMBITION_PROFILE_CENSUS`. It feeds `DiagnosticsStore`
/// for the F1 panel, which a developer can open without a restart.
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
/// The paths stay registered and only the sampling is paced, so F1 can open at
/// any time and show a value less than 250 ms old. Pacing bounds the cost if
/// the world grows more archetypes.
const ECS_DIAGNOSTIC_SAMPLE_PERIOD: Duration = Duration::from_millis(250);

fn publish_ecs_diagnostics(mut diagnostics: Diagnostics, population: EcsPopulation) {
    diagnostics.add_measurement(&SCENE_ENTITIES, || population.scene_entities() as f64);
    diagnostics.add_measurement(&RESOURCE_ENTITIES, || population.resource_entities() as f64);
    diagnostics.add_measurement(&BODIES, || population.bodies() as f64);
}

/// The four entity populations the ECS census counts, as one reusable param.
///
/// This type exists so that the `Without<IsResource>` filter is not forgotten.
/// Since Bevy 0.19 a resource is an entity, so a bare `Query<()>` counts scene
/// content plus every resource. Code that wants a scene-entity count uses this
/// param. The tests drive this type directly.
#[derive(bevy::ecs::system::SystemParam)]
pub struct EcsPopulation<'w, 's> {
    scene: Query<'w, 's, (), Without<IsResource>>,
    resources: Query<'w, 's, (), With<IsResource>>,
    bodies: Query<'w, 's, (), With<BodyKinematics>>,
    players: Query<'w, 's, (), With<PlayerEntity>>,
}

/// Every count uses `Query::count()`, not `iter().count()`. The queries take no
/// data and filter only with `With`/`Without`, so Bevy answers from archetype
/// sizes without visiting entities. `iter().count()` walks the whole world.
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
    // Three different counts:
    // - `entities` is `Entities::len()`: allocated slots, which round to powers of
    //   two. Do not read it as scene size.
    // - `live` is scene content (`Without<IsResource>`).
    // - `resources` is resource entities. It is real engine population, but not
    //   scene content, so it is reported beside `live`, not added to it.
    // Read `live`. `entities - live - resources` is reservation slack.
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
    // Per schedule, not only a total. `[census] phases` gives the cost of each
    // phase; this gives how many systems it carries.
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

/// Where the run conditions sit, and how many there are.
///
/// This is a one-shot and must run before the schedules do.
/// `Schedule::initialize` moves every condition out of the `ScheduleGraph` into
/// the private executable, and there is no public accessor for built
/// conditions. A sampled census in `Last` would read a drained graph and report
/// zero.
///
/// It counts only what plugin build registered. Systems added later, when a
/// session activates, are not included.
///
/// This is a structural metric, not a timing. Bevy evaluates a system's
/// conditions once per system per run, and a set's conditions once per run, so
/// the graph already knows the counts. A deterministic count is a better
/// regression gate than a wall-clock time.
///
/// It counts attachments, not evaluations. Moving a shared condition from N
/// systems onto one set moves N out of `system_conditions` and 1 into
/// `set_conditions`. The per-condition breakdown names the conditions attached
/// most often.
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
    // Every schedule in this app has conditions, so zero means this ran after the
    // graphs were drained. Say so, so that nobody records a zero as a fact.
    if system_conditions == 0 && set_conditions == 0 {
        eprintln!(
            "[census] conditions t=0.000 unavailable=graph_already_initialized \
             (this must run before the schedules do)"
        );
        return;
    }
    report_schedule_owners(&schedules);
    // Name the members of the phases that profiling cannot yet attribute.
    report_schedule_membership(&schedules, "PreUpdate", 0.0);
    report_schedule_membership(&schedules, "Update", 0.0);
    report_schedule_membership(&schedules, "PostUpdate", 0.0);
    report_schedule_owners_in(&schedules, "Update");
    // And the owners of the other two costly phases.
    report_schedule_owners_in(&schedules, "PreUpdate");
    report_schedule_owners_in(&schedules, "PostUpdate");
    // The two phases that grow most between a Smash stage and a real room.
    report_schedule_membership(&schedules, "StateTransition", 0.0);
    report_schedule_membership(&schedules, "RunFixedMainLoop", 0.0);
    let mut row = format!(
        "[census] conditions t=0.000 system_conditions={system_conditions} \
         set_conditions={set_conditions} sets_with_conditions={sets_with_conditions}"
    );
    // Name only conditions attached 4 or more times; the tail hides the rest.
    let mut ranked: Vec<(&String, &usize)> =
        by_name.iter().filter(|(_, count)| **count >= 4).collect();
    ranked.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    for (name, count) in ranked.iter().take(16) {
        row.push_str(&format!(" {name}={count}"));
    }
    eprintln!("{row}");
}

/// Where the sim tick's time goes, phase by phase.
///
/// `[census] phases` splits the main schedule, but there `PreUpdate` holds one
/// exclusive system (`bevy_ggrs`'s `run_ggrs_schedules`) that runs the whole
/// sim as `GgrsSchedule`. This census splits the sim itself.
///
/// It cannot use `FramePhaseMark`: that adds marker schedules to
/// `MainScheduleOrder`, but these phases are sets inside one schedule. So each
/// boundary is a system ordered between two chained sets.
///
/// The totals are not rollback state. A rewind leaves timings from the
/// discarded branch in them. That is correct for an instrument, but these
/// numbers must never gate behaviour. (The local session uses
/// `check_distance: 0` and does not rewind.)
///
/// Wall clock, so not on wasm: `Instant::now()` panics in a browser.
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
    /// A one-tick window is not reported (see `report_sim_phase_census`). Its time
    /// is kept: the reporter returns before the reset, so it folds into the next
    /// window.
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
/// This is an index into a list in this crate that another crate uses. It
/// replaces a runtime registration API that would depend on plugin build
/// order. `report_sim_phase_census` reports an unclosed bucket instead of
/// letting the neighbouring bucket silently grow.
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
/// The marks are not installed when the census is off, and a caller in another
/// crate cannot see the `if enabled` in `RuntimeCensusPlugin::build`. Check
/// this before registering marks, so the sim schedule carries none when not
/// needed. The `Option` on [`mark_sim_phase`] is the second defence: a mark
/// without a census is a no-op, not a panic.
#[cfg(not(target_arch = "wasm32"))]
pub fn sim_phase_census_enabled() -> bool {
    std::env::var(CENSUS_ENV)
        .map(|value| env_is_truthy(&value))
        .unwrap_or(false)
}

/// The boundary system for one sim phase.
///
/// The `Option` is required. This function is `pub` so other crates can close
/// buckets for sets that only they can name, and the resource exists only when
/// the census is on. Without a census, the mark does nothing.
#[cfg(not(target_arch = "wasm32"))]
pub fn mark_sim_phase(index: usize) -> impl FnMut(Option<ResMut<SimPhaseCensus>>) {
    move |census: Option<ResMut<SimPhaseCensus>>| {
        if let Some(mut census) = census {
            census.close(index);
        }
    }
}

/// Open the window, attributing nothing.
///
/// Without this, the first bucket is wrong. A closing mark records "now minus
/// the previous mark", so with no opening mark the first phase absorbs the rest
/// of the previous frame (main schedule and render included). A bucket larger
/// than the whole sim tick is the symptom.
#[cfg(not(target_arch = "wasm32"))]
fn open_sim_phase_window(mut census: ResMut<SimPhaseCensus>) {
    census.open();
}

/// Report the membership of the sim schedule, which
/// `report_schedule_conditions_census` cannot see.
///
/// The `PreStartup` pass reports only main-schedule labels. `GgrsSchedule` does
/// not exist until a session activates, so that pass reports zero for every
/// sim system, present or not.
///
/// This runs once, on a sample frame after the sim schedule exists. Its graph
/// is drained by then, so it uses `report_schedule_membership`, which falls
/// back to the initialized executable.
///
/// Matched by name, not label type: `ambition_dev_tools` must not depend on
/// `bevy_ggrs` (see `install_ggrs_driver_census` in the ggrs crate).
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
    // Wait until the session builds the schedule; do not repeat "not yet".
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
            // The membership row drops crate paths, so also report which crates own the
            // sim's systems.
            report_schedule_owners_in(&schedules, wanted);
        }
    }
}

/// The labels a sim schedule can wear in this repo. `GgrsSchedule` is the shipped
/// one; `SimSchedule` is what a non-rollback host binds.
const SIM_SCHEDULE_NAMES: &[&str] = &["GgrsSchedule", "SimSchedule"];

/// Report the sim-phase split on the census interval, then reset.
///
/// Values are per-tick means over the interval, because one sim tick is only
/// microseconds.
#[cfg(not(target_arch = "wasm32"))]
pub fn report_sim_phase_census(
    census: Res<RuntimeCensus>,
    mut phases: ResMut<SimPhaseCensus>,
    // `Option`: a composition can run this census without developer tools.
    brains: Option<Res<ambition_characters::brain::AuthoredBrainOverride>>,
    population_cap: Option<Res<ambition_characters::actor::AuthoredPopulationCap>>,
) {
    let Some(at) = census.due() else {
        return;
    };
    if phases.ticks == 0 {
        return;
    }
    // Do not emit a one-tick window. The first window after boot holds one tick in
    // a world that has not finished spawning, so every phase reads 0.000.
    // Averaging that row with later ones biases means low, most at low
    // populations. `ticks=1` is on the row, but readers did not filter on it. The
    // window's time is not lost: it folds into the next window.
    if !phases.is_reportable() {
        return;
    }
    let ticks = phases.ticks as f64;
    let mut row = format!("[census] sim_phases t={at:.3} ticks={}", phases.ticks);
    // A population-capped run does not describe the authored room. Say so on the
    // row, because readers quote rows without their environment. Read the
    // resource the sim read, not the environment, so the row reports what was in
    // force.
    if let Some(cap) = population_cap.as_deref().and_then(|cap| cap.cap()) {
        row.push_str(&format!(" actor_cap={cap}"));
    }
    // The same for a brain override, which changes decision cost more than
    // removing bodies. `None` (no developer tools) prints nothing.
    if let Some(forced) = brains.as_deref() {
        if let Some(preset) = forced.preset() {
            row.push_str(&format!(" brain_override={preset}"));
        }
        if let Some(profile) = forced.profile() {
            row.push_str(&format!(" brain_profile={profile}"));
        }
    }
    // Perception census: what each viewer kept. `Decide` cost follows `kept`, not
    // room population. `visible` is peers inside the viewport; `kept` is what the
    // attention budget let through (equal until the budget binds, see
    // `TACTICAL_ATTENTION`).
    // The phases in `unmeasured=` have no declared order, so they have no bucket.
    if let Some(census) = ambition_characters::perception::census::drain() {
        row.push_str(&format!(
            " views={} offered={:.1} visible={:.1} kept={:.1} kept_max={}",
            census.views,
            census.offered_mean,
            census.visible_mean,
            census.kept_mean,
            census.kept_max
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
    // An unclosed bucket silently widens its neighbour. If ticks ran and this
    // bucket got no time, its mark is not installed; say so on the row.
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

/// What the entities are: the largest populations, by the component that names
/// them.
///
/// `entities` alone cannot say which population grew (a Smash match goes from
/// 64 to 2048 entities while `bodies` stays at 2).
///
/// This ranks by component, not by archetype, because an archetype name is an
/// unreadable component list. An entity counts once per component it holds, so
/// the numbers do not sum to the entity count.
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

/// Who owns the systems in one schedule.
///
/// `[census] owners` totals all schedules, and `[census] membership` names
/// systems without grouping them. This groups one schedule's systems by crate,
/// which shows what to gate to attribute that schedule's cost.
///
/// A count is not a cost. This shows who is registered, not who is expensive.
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
        // Every owner, same reason as `report_schedule_owners` below.
        for (name, count) in ranked.iter() {
            row.push_str(&format!(" {name}={count}"));
        }
        eprintln!("{row}");
        return;
    }
}

/// Name every system in one schedule, so an unattributed cost stops being
/// "DefaultPlugins" and becomes something a person can act on.
///
/// This prints names instead of timing systems. Timing them would make this
/// crate depend on `bevy_ui`, `bevy_picking` and leafwing only to name their
/// sets. The graph already has the names, at no cost.
///
/// The graph is drained by `Schedule::initialize` (see
/// `report_schedule_conditions_census`), so the executable is the fallback.
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
        // An empty graph can be a drained graph, not an empty schedule. A schedule
        // that already ran (`StateTransition`, `Startup`) has its systems moved into
        // the private executable, and `Schedule::systems()` reads them from there.
        // The graph answers before the first run; the executable answers after.
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

/// Who owns the registered systems — the population behind "this app installs
/// every experience it can launch".
///
/// System names carry their crate, so the schedule graph shows which crate
/// asks the frame for work, including experiences this run never entered.
///
/// A count is not a cost. A crate with 200 systems in a gated set costs one
/// condition; a crate with 3 ungated systems costs 3 per frame. This row
/// answers "should a shipped title carry this crate", not "what did the frame
/// spend".
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
    // Print every owner, not a top N. Readers search this row for a crate and read
    // absence as "registers no systems", which is only true if the list is
    // complete. Ranked, so the largest owners come first.
    for (name, count) in ranked.iter() {
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
/// That placement makes the breakdown exact. A marker system inside `Update`
/// would run wherever the executor put it and charge part of `Update` to the
/// phase before it.
///
/// The index is the phase it opens, so `FramePhaseMark(0)` runs before the
/// first phase and `FramePhaseMark(n)` after the last.
#[cfg(not(target_arch = "wasm32"))]
#[derive(bevy::ecs::schedule::ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct FramePhaseMark(usize);

/// Process CPU time, in milliseconds, or `None` where it is not available.
///
/// This separates busy from blocked. `Instant` measures wall time, so a phase
/// that waits 3 ms on the GPU looks like one that works 3 ms. A CPU clock does
/// not advance while nothing runs.
///
/// Process, not thread: Bevy does not run consecutive mark schedules on one
/// thread, so differencing a per-thread clock across threads gives wrong
/// values.
///
/// It sums every thread, so a busy phase can show more CPU than wall time.
/// cpu/wall is roughly the number of busy cores; a ratio near zero is a stall.
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

/// Wall time attributed to each phase of the main schedule.
///
/// This frame breakdown needs no profiler, so it works on web, Android and
/// other builds where Tracy is not available or costs more than the game.
///
/// The phases come from [`MainScheduleOrder`], not a hardcoded list, so
/// `StateTransition`, `SpawnScene` and game-inserted schedules are attributed
/// correctly.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Default)]
pub struct SchedulePhaseCensus {
    /// Phase names in schedule order, from `MainScheduleOrder`, plus a trailing
    /// `outside` for the gap after the last phase.
    names: Vec<String>,
    /// Index of the phase currently open, or `None` before the first mark.
    current: Option<usize>,
    marked_at: Option<Instant>,
    /// Process CPU milliseconds at the last mark, when the clock is available.
    /// Every thread, not the main one — see `process_cpu_ms`.
    marked_cpu: Option<f64>,
    /// Accumulated milliseconds per phase since the last report.
    totals_ms: Vec<f64>,
    /// Accumulated process CPU milliseconds per phase, parallel to `totals_ms`.
    /// Summed across every thread, so this can exceed the wall time beside it.
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
/// Runs in the last mark schedule, after its marker, so every phase of the
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
    // The same split on the process CPU clock. Wall time cannot separate a phase
    // that is blocked on the GPU from one that is busy; CPU time can.
    // `phases - phases_cpu` is not the stall: the process clock sums every thread,
    // so the difference can be negative. Read the ratio cpu/wall instead (see
    // `process_cpu_ms`).
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

/// The phases the schedule declares no order between, by bucket index.
///
/// These are unmeasured, not only unordered. `Trace` is `.after(CoreSimulation)`
/// and nothing is ordered after it. A serial mark there would take time from
/// the next chain bucket. A pair of marks around it is unbounded on both outer
/// sides and times most of the frame. Its bucket stays zero and the row says
/// `unmeasured=Trace`. Measuring it needs spans inside its systems.
#[cfg(not(target_arch = "wasm32"))]
pub const SIM_PHASE_UNORDERED: [usize; 1] = [20];

/// The sim-phase bucket names, in index order.
///
/// The indices are a contract with the `SIM_PHASE_*` constants. Inserting a name
/// above one of them points that mark at another bucket. The tests in
/// `sim_phase_index_tests` catch this.
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
        // Appended, so this order is not the chain order. `close(index)` bills
        // "now minus the previous mark" to the named bucket, so attribution follows
        // the runtime order of the marks and an index is only a label.
        //
        // `install_sim_phase_boundaries` installs all seven together, so bucket
        // `SIM_PHASE_ACTOR_DECISION` always means the tail (`Publish` through
        // `BodyMode`), not the whole prefix. The "NEVER CLOSED" row text catches an
        // empty bucket.
        "WorldPrep.Decision.Gate",
        "WorldPrep.Decision.Targeting",
        "WorldPrep.Decision.Prepare",
        "WorldPrep.Decision.Observe",
        "WorldPrep.Decision.StateMaintenance",
        "WorldPrep.Decision.Decide",
        "WorldPrep.Decision.Publish",
    ]
}

/// Put a boundary system after each top-level sim phase, and after each
/// sub-phase of `CoreSimulation` and of the actor decision chain.
///
/// `.after(A)` alone does not put a mark at `A`'s trailing edge: `A`'s
/// successor can start before the mark and have part of its work billed to
/// `A`. So every mark that can be bracketed is `.after(A).before(B)`. The
/// chains come from `configure_platformer2d_simulation_phases`:
///
/// ```text
/// CoreSimulation -> FeatureCollection -> FeatureInteraction -> LdtkRuntimeSpine
///   -> EncounterSimulation -> Cutscene -> GameplayEffects -> Progression
///   -> ResetProcessing -> FeatureViewSync,  then PresentationVisualSync after it
/// ```
///
/// Only `Trace` is unordered; see `SIM_PHASE_UNORDERED`. These names copy the
/// chain's membership. A phase added there without a mark here is billed to
/// whichever neighbour closes next.
#[cfg(not(target_arch = "wasm32"))]
fn install_sim_phase_boundaries(app: &mut App) {
    use ambition_platformer2d_shared_tangle::schedule::{
        ActorDecisionSet, Platformer2dSimulationPhaseMonolith as Phase, PlayerInputSet,
        SimScheduleExt as _, WorldPrepSet,
    };

    let sim = app.sim_schedule();
    // Ordered as the chain runs. `CoreSimulation`'s sub-phases come first because
    // the umbrella closes only after all of them.
    //
    // `WorldPrep` is split into its sub-sets so that a growth in `WorldPrep` can
    // be assigned: body-contact pairing (O(n^2)) and the movement kernel (O(n))
    // need different fixes.
    app.insert_resource(SimPhaseCensus::with_names(sim_phase_names()));
    ambition_characters::perception::census::enable();

    // The opening mark makes bucket 0 mean `PlayerInput` only, not `PlayerInput`
    // plus the previous frame.
    app.add_systems(sim, open_sim_phase_window.before(Phase::PlayerInput));
    // Each mark needs both edges where the schedule declares a successor:
    //
    //   PlayerInput -> WorldPrep -> PlayerSimulation -> RoomTransition
    //                -> Combat -> PresentationSync        (CoreSimulation, chained)
    //   BeforeIntegrate -> Integrate -> AfterIntegrate    (WorldPrep, chained)
    //   CoreSimulation -> FeatureCollection -> ... -> ResetProcessing
    //     -> FeatureViewSync -> PresentationVisualSync
    app.add_systems(
        sim,
        mark_sim_phase(0)
            .after(Phase::PlayerInput)
            .before(Phase::WorldPrep),
    );
    // Sub-sets close before their umbrella. Each mark bills the span since the
    // previous one, so bucket 5 (`WorldPrep`) holds only work in the phase but in
    // none of its sets (for example `snapshot_body_contact`).
    //
    // `AfterIntegrate` and `ContactDamage` are not chained to each other (see
    // `WorldPrepSet`), so their marks record the order the schedule resolved.
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
    // `Trace` gets no mark; see `SIM_PHASE_UNORDERED`.

    // The actor decision chain. Every decision set is `in_set(WorldPrep)` and
    // `before(BeforeIntegrate)`, so without these marks all of it bills to bucket
    // 1. Six sub-marks separate `Targeting` (`select_actor_targets`, O(n²)) from
    // `Decide` (`tick_actor_brains`, which builds a `WorldView` for every actor).
    //
    // The tail mark closes after `BodyMode`, not `Publish`, because the two gate
    // sets run between publication and `BeforeIntegrate`.
    //
    // Each mark needs both edges: the sets are chained to each other, not to these
    // systems, so a mark with only `.after` may run after a later set.
    app.add_systems(
        sim,
        (
            mark_sim_phase(SIM_PHASE_DECISION_TARGETING)
                .after(ActorDecisionSet::Targeting)
                .before(ActorDecisionSet::Prepare),
            mark_sim_phase(SIM_PHASE_DECISION_PREPARE)
                .after(ActorDecisionSet::Prepare)
                .before(ActorDecisionSet::Observe),
            mark_sim_phase(SIM_PHASE_DECISION_OBSERVE)
                .after(ActorDecisionSet::Observe)
                .before(ActorDecisionSet::StateMaintenance),
            mark_sim_phase(SIM_PHASE_DECISION_STATE_MAINTENANCE)
                .after(ActorDecisionSet::StateMaintenance)
                .before(ActorDecisionSet::Decide),
            mark_sim_phase(SIM_PHASE_DECISION_DECIDE)
                .after(ActorDecisionSet::Decide)
                .before(ActorDecisionSet::Publish),
            mark_sim_phase(SIM_PHASE_DECISION_PUBLISH)
                .after(ActorDecisionSet::Publish)
                .before(PlayerInputSet::ControlGate),
            // The tail: `ControlGate` and `BodyMode`, which are chained after
            // `Publish` and before `BeforeIntegrate`.
            mark_sim_phase(SIM_PHASE_ACTOR_DECISION)
                .after(PlayerInputSet::BodyMode)
                .before(WorldPrepSet::BeforeIntegrate),
        ),
    );
}

/// Install the phase marks, reading the phase list from the app's own
/// [`MainScheduleOrder`]. Returns the phase names in order.
///
/// Call only when the census is enabled: each boundary is a schedule that runs
/// every frame.
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

    // Do not use `MainScheduleOrder::insert_before`. It finds the anchor by
    // downcasting to its concrete type, so an `InternedScheduleLabel` read back
    // from `labels` fails and panics ("Expected First to exist"). The list is
    // public, so interleave it directly in one pass.
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
        // `PreStartup` is required: `Update` and the sim schedule are not initialized
        // yet, so their graphs still hold the conditions this counts.
        app.add_systems(PreStartup, report_schedule_conditions_census);
        let enabled = app.world().resource::<RuntimeCensus>().enabled();

        // Reporters are registered only when enabled. Each one returns early on
        // `census.due()`, but an instrument must not add systems to frames that did
        // not ask for it. This changes no behaviour: `due_at` is set only while
        // enabled.
        if enabled {
            app.add_systems(
                Last,
                (
                    report_frame_interval_census,
                    report_ecs_census,
                    report_schedule_load_census,
                    report_entity_populations,
                    // The sim schedule exists only after a session activates, so the
                    // `PreStartup` pass cannot see it. Latches after one report.
                    report_sim_schedule_membership,
                ),
            );
        }

        // The boundaries run inside the sim schedule, the hottest schedule in the app.
        #[cfg(not(target_arch = "wasm32"))]
        if enabled {
            install_sim_phase_boundaries(app);
            // Register the reporter here: it needs `SimPhaseCensus`, which
            // `install_sim_phase_boundaries` inserts only when the census is on.
            app.add_systems(Last, report_sim_phase_census);
        }

        // Phase marks are schedules, one per boundary, that run every frame.
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

    /// The panel and the log row read the same number.
    ///
    /// Both the census printer and the diagnostics publisher take
    /// [`EcsPopulation`], so the published value must equal the param's value in
    /// the same frame. The test changes the population, so a publisher that writes
    /// a constant fails.
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
        // The publisher is paced (`ECS_DIAGNOSTIC_SAMPLE_PERIOD`), so advance one
        // full period per update.
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            ECS_DIAGNOSTIC_SAMPLE_PERIOD,
        ));

        // The first `update()` has a zero delta (`Time<Real>` has no previous
        // instant), so no timed condition fires. One priming frame is needed.
        app.update();
        app.update();
        let (scene, resources) = app.world().resource::<Sampled>().0;
        assert_eq!(
            published(&app),
            (scene as f64, resources as f64),
            "the published values must be the param's own"
        );

        // Move both populations, then re-read: a publisher wired to a constant
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
    /// This is a cost guard. Without the run condition, a panel nobody opened
    /// counts the world on every frame, and the first assertion fails. The arms
    /// straddle the period: many short frames must not publish, and one frame past
    /// the period must.
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

        // The first `update()` advances the clock by zero, so ten frames give only
        // nine tenths of a period.
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

    /// The scene count must not move when a resource is added.
    ///
    /// Since Bevy 0.19 a resource is an entity, so an unfiltered `Query<()>` adds
    /// every resource to the scene count. Two arms: adding a resource must move
    /// `resource_entities` and not `scene_entities`; spawning an entity must do
    /// the opposite. The second arm stops a query that counts nothing from
    /// passing.
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

        // A resource: `resources` moves, `scene` does not.
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

        // An entity: `scene` moves, `resources` does not. Without this arm a
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
        // Regression guard: `MainScheduleOrder::insert_before` panics with an
        // interned label ("Expected First to exist"). Only reachable with the census
        // enabled, so no other test sees it.
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
    //! `.after(phase)` alone does not put a mark at the phase's trailing edge:
    //! the successor may start first, and its work is billed to the previous
    //! bucket.
    //!
    //! This is a source guard, not a schedule-graph proof. It cannot check that
    //! `.before` names the correct successor, only that a bracketable mark is not
    //! one-sided. The chain it must agree with is in
    //! `configure_platformer2d_simulation_phases`.

    const SOURCE: &str = include_str!("runtime_census.rs");

    /// Marks the schedule gives no successor to, and why.
    ///
    /// Marks the schedule gives no successor to.
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
            let end = rest
                .find(");")
                .map(|e| e + 2)
                .unwrap_or(rest.len().min(400));
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
            assert!(
                text.contains(".after("),
                "mark {index} lost its lower bound"
            );
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
        // Premise guard: without this, marking every index successor-less would make
        // the test above pass vacuously.
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
            assert!(
                index < names.len(),
                "unordered index {index} names no bucket"
            );
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod sim_phase_index_tests {
    use super::*;

    /// `SIM_PHASE_ACTOR_DECISION` is a bare `usize` passed to `mark_sim_phase`.
    /// Inserting a name above it would point that mark at another bucket, and
    /// nothing else would fail.
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
    /// A transposed pair would not fail on its own; it would publish `Targeting`
    /// time under `Decide`'s name.
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

    /// A mark without a census must not stop the game.
    ///
    /// `mark_sim_phase` is `pub`, and a caller in another crate cannot see the
    /// `if enabled` guard in `RuntimeCensusPlugin::build`. The `Option` makes an
    /// unconditional registration safe when `AMBITION_PROFILE_CENSUS` is unset.
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
    /// Without this, an empty `mark_sim_phase` would pass the test above.
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

    /// A one-tick startup window reads 0.000 for every phase, because almost
    /// nothing ran yet. Averaged with real windows, for example
    ///
    /// ```text
    /// (0.000 + 0.341 + 0.332) / 3 = 0.224
    /// ```
    ///
    /// it biases the mean low, most at low populations where runs are short.
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
    /// This makes suppression lossless. The reporter returns before its reset, so
    /// a skipped window keeps accumulating.
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
