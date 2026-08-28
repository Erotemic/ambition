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
    for (_, schedule) in schedules.iter() {
        let count = schedule.systems_len();
        if count > 0 {
            visible += 1;
            total += count;
        }
    }
    eprintln!("[census] schedules t={at:.3} schedules={visible} systems={total}");
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
        app.add_systems(
            Last,
            (
                report_frame_interval_census,
                report_ecs_census,
                report_schedule_load_census,
            ),
        );
        let census = app.world().resource::<RuntimeCensus>();
        if census.enabled() {
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
