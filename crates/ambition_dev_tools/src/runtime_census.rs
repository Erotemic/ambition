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

use bevy::ecs::archetype::Archetypes;
use bevy::ecs::component::Components;
use bevy::ecs::entity::Entities;
use bevy::prelude::*;

use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;

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
    bodies: Query<(), With<BodyKinematics>>,
    players: Query<(), With<PlayerEntity>>,
) {
    let Some(at) = census.due() else {
        return;
    };
    eprintln!(
        "[census] ecs t={at:.3} entities={} archetypes={} components={} bodies={} players={}",
        entities.len(),
        archetypes.len(),
        components.len(),
        bodies.iter().count(),
        players.iter().count(),
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
    // The phase the campaign cannot yet attribute: 0.95ms of it is not the sim.
    report_schedule_membership(&schedules, "PreUpdate");
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
#[derive(Resource, Default)]
pub struct SimPhaseCensus {
    /// When the previous boundary fired, or `None` before the first.
    last: Option<Instant>,
    /// Accumulated time attributed to each phase, parallel to `names`.
    totals: Vec<f64>,
    names: Vec<&'static str>,
    ticks: u32,
}

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

/// The boundary system for one sim phase.
fn mark_sim_phase(index: usize) -> impl FnMut(ResMut<SimPhaseCensus>) {
    move |mut census: ResMut<SimPhaseCensus>| census.close(index)
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
fn open_sim_phase_window(mut census: ResMut<SimPhaseCensus>) {
    census.open();
}

/// Report the sim-phase split on the census interval, then reset.
///
/// ⭐ REPORTED AS A PER-TICK AVERAGE over the interval, because a single sim
/// tick is microseconds and the interesting quantity is what the tick costs on
/// average, not what one of them did.
pub fn report_sim_phase_census(census: Res<RuntimeCensus>, mut phases: ResMut<SimPhaseCensus>) {
    let Some(at) = census.due() else {
        return;
    };
    if phases.ticks == 0 {
        return;
    }
    let ticks = phases.ticks as f64;
    let mut row = format!("[census] sim_phases t={at:.3} ticks={}", phases.ticks);
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
fn report_schedule_membership(schedules: &Schedules, wanted: &str) {
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
        eprintln!(
            "[census] membership t=0.000 schedule={wanted} systems={} {}",
            names.len(),
            names.join(" ")
        );
        return;
    }
    eprintln!("[census] membership t=0.000 schedule={wanted} unavailable=not_found");
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
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Default)]
pub struct SchedulePhaseCensus {
    /// Phase names in schedule order, from `MainScheduleOrder`, plus a trailing
    /// `outside` for the gap after the last phase.
    names: Vec<String>,
    /// Index of the phase currently open, or `None` before the first mark.
    current: Option<usize>,
    marked_at: Option<Instant>,
    /// Accumulated milliseconds per phase since the last report.
    totals_ms: Vec<f64>,
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
            frames: 0,
        }
    }

    /// Close the open phase and open `phase`.
    fn advance_to(&mut self, phase: usize, now: Instant) {
        if let (Some(previous), Some(open)) = (self.marked_at, self.current) {
            if let Some(slot) = self.totals_ms.get_mut(open) {
                *slot += now.duration_since(previous).as_secs_f64() * 1000.0;
            }
        }
        self.current = Some(phase);
        self.marked_at = Some(now);
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
    for total in &mut phases.totals_ms {
        *total = 0.0;
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
/// ⭐ THE ORDER IS THE CHAIN'S, NOT A LIST I KEEP. `configure_platformer2d_simulation_phases`
/// already `.chain()`s these, so ordering a marker `.after(phase)` puts it
/// exactly at that phase's trailing edge. ⛔ The names below still duplicate the
/// chain's membership, and a phase added there without a line here is simply
/// unattributed — its time lands in whichever neighbour closes next, which is
/// the honest failure mode for a boundary instrument but is a failure mode.
fn install_sim_phase_boundaries(app: &mut App) {
    use ambition_platformer2d_shared_tangle::schedule::{
        Platformer2dSimulationPhaseMonolith as Phase, SimScheduleExt as _,
    };

    let sim = app.sim_schedule();
    // Ordered as the chain runs. `CoreSimulation`'s sub-phases come first
    // because the umbrella closes only once they all have.
    let names = vec![
        "PlayerInput",
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
    ];
    app.insert_resource(SimPhaseCensus::with_names(names));

    // ⛔ THE OPENING MARK COMES FIRST, and it is what makes bucket 0 mean
    // `PlayerInput` rather than `PlayerInput plus the whole preceding frame`.
    app.add_systems(sim, open_sim_phase_window.before(Phase::PlayerInput));
    app.add_systems(sim, mark_sim_phase(0).after(Phase::PlayerInput));
    app.add_systems(sim, mark_sim_phase(1).after(Phase::WorldPrep));
    app.add_systems(sim, mark_sim_phase(2).after(Phase::PlayerSimulation));
    app.add_systems(sim, mark_sim_phase(3).after(Phase::RoomTransition));
    app.add_systems(sim, mark_sim_phase(4).after(Phase::Combat));
    app.add_systems(sim, mark_sim_phase(5).after(Phase::PresentationSync));
    app.add_systems(sim, mark_sim_phase(6).after(Phase::FeatureCollection));
    app.add_systems(sim, mark_sim_phase(7).after(Phase::FeatureInteraction));
    app.add_systems(sim, mark_sim_phase(8).after(Phase::LdtkRuntimeSpine));
    app.add_systems(sim, mark_sim_phase(9).after(Phase::EncounterSimulation));
    app.add_systems(sim, mark_sim_phase(10).after(Phase::Cutscene));
    app.add_systems(sim, mark_sim_phase(11).after(Phase::GameplayEffects));
    app.add_systems(sim, mark_sim_phase(12).after(Phase::Progression));
    app.add_systems(sim, mark_sim_phase(13).after(Phase::ResetProcessing));
    app.add_systems(sim, mark_sim_phase(14).after(Phase::FeatureViewSync));
    app.add_systems(sim, mark_sim_phase(15).after(Phase::PresentationVisualSync));
    app.add_systems(sim, mark_sim_phase(16).after(Phase::Trace));
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
        app.add_systems(
            Last,
            (
                report_frame_interval_census,
                report_ecs_census,
                report_schedule_load_census,
                report_entity_populations,
                report_sim_phase_census,
            ),
        );
        let enabled = app.world().resource::<RuntimeCensus>().enabled();

        // ⛔ THE BOUNDARIES ARE NOT REGISTERED WHEN THE CENSUS IS OFF. They run
        // inside the SIM schedule — the hottest schedule in the app — and an
        // instrument must not join the population it measures when nobody asked.
        if enabled {
            install_sim_phase_boundaries(app);
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
