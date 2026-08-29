//! Lightweight startup profiler.
//!
//! Records `Instant::now()` marks at named startup boundaries and prints per-phase
//! deltas plus total time before the first frame. Insert `phase_mark("name")`
//! between chained `Startup` systems to delimit windows.
//!
//! For deeper profiling, build with `--features profile` to enable Bevy's
//! `trace_tracy` integration; see `docs/recipes/profiling.md`.
//!
//! On `wasm32-unknown-unknown`, `std::time::Instant` is unavailable, so this module
//! compiles to a one-time disabled notice and no-op mark/report systems. Use browser
//! devtools for wall-clock profiling there.

use bevy::prelude::*;

// ─────────────────────────────────────────────────────────────────────
// Native (non-wasm) implementation — real Instant-based profiling.
// ─────────────────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

/// Wall-clock zero for the whole startup report.
///
/// ⭐ **App CONSTRUCTION is the larger half of startup, and a resource created
/// during plugin build cannot see it.** `StartupProfiler` is initialized by
/// `DevToolsSimPlugin::build`, which runs partway through the simulation plugin
/// tree; anchoring deltas to that moment silently excluded every plugin built
/// before it. A measured headless run reported `total before first frame:
/// 120.4ms` against 2.6s of real pre-frame wall clock -- the report was not
/// wrong about its 120ms, it was answering a much smaller question than the one
/// its own label asked.
///
/// [`note_process_start`] is called from the entry point, before any Bevy work,
/// so the anchor precedes plugin construction. It is a `OnceLock` rather than a
/// resource because the value has to exist before a `World` does.
#[cfg(not(target_arch = "wasm32"))]
static PROCESS_STARTED_AT: OnceLock<Instant> = OnceLock::new();

/// Record the process's startup anchor. Call FIRST in `main()`, before building
/// the `App`. Idempotent: later calls keep the earliest anchor.
///
/// Not calling it is not an error -- the report falls back to the moment the
/// profiler resource was created and says so, rather than quietly attributing
/// app construction to nothing.
#[cfg(not(target_arch = "wasm32"))]
pub fn note_process_start() {
    let _ = PROCESS_STARTED_AT.set(Instant::now());
}

/// No-op on wasm, where `Instant` is unavailable.
#[cfg(target_arch = "wasm32")]
pub fn note_process_start() {}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource)]
pub struct StartupProfiler {
    /// Wall-clock zero: process start when [`note_process_start`] ran, else the
    /// moment this resource was created. `anchor_is_process_start` says which,
    /// because the difference between them is exactly the app-construction time
    /// that the report exists to expose.
    pub app_constructed_at: Instant,
    /// Whether [`app_constructed_at`](Self::app_constructed_at) is a true
    /// process-start anchor rather than a mid-plugin-build fallback.
    pub anchor_is_process_start: bool,
    /// When this resource was created, i.e. a point inside plugin construction.
    /// The span from the anchor to here is app construction.
    pub resource_created_at: Instant,
    /// Ordered list of `(name, instant)` marks. `phase_mark` systems
    /// append to this; the report system reads it.
    pub marks: Vec<(&'static str, Instant)>,
    /// Set true on the first PostStartup tick so the report only
    /// prints once even if the user (somehow) re-runs PostStartup.
    pub reported: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for StartupProfiler {
    fn default() -> Self {
        let now = Instant::now();
        let anchor = PROCESS_STARTED_AT.get().copied();
        Self {
            app_constructed_at: anchor.unwrap_or(now),
            anchor_is_process_start: anchor.is_some(),
            resource_created_at: now,
            marks: Vec::new(),
            reported: false,
        }
    }
}

/// Build a one-shot Startup system that records a phase mark with the
/// given name. Use between chained Startup systems to delimit
/// timing windows. Inserts a `(name, Instant::now())` entry into the
/// `StartupProfiler` resource.
#[cfg(not(target_arch = "wasm32"))]
pub fn phase_mark(name: &'static str) -> impl FnMut(ResMut<StartupProfiler>) {
    move |mut profiler: ResMut<StartupProfiler>| {
        profiler.marks.push((name, Instant::now()));
    }
}

/// PostStartup report. Runs once: prints per-phase deltas + total
/// startup time to stderr. Single fmt block, easy to grep.
#[cfg(not(target_arch = "wasm32"))]
pub fn report_startup_phases(mut profiler: ResMut<StartupProfiler>) {
    if profiler.reported {
        return;
    }
    profiler.reported = true;
    let total_ms = profiler.app_constructed_at.elapsed().as_secs_f32() * 1000.0;

    // App construction first, because it is usually the bigger half and the
    // phase marks below cannot reach it -- they are Startup SYSTEMS, and every
    // plugin has already been built by the time the first one runs.
    if profiler.anchor_is_process_start {
        let build_ms = profiler
            .resource_created_at
            .duration_since(profiler.app_constructed_at)
            .as_secs_f32()
            * 1000.0;
        eprintln!(
            "[startup] → app construction (to dev-tools plugin build): +{build_ms:.1}ms \
             — plugin registration; use Tracy `plugin build` zones to attribute it"
        );
    } else {
        eprintln!(
            "[startup] app construction NOT MEASURED — `note_process_start()` was never \
             called, so this report starts mid-plugin-build and undercounts the total"
        );
    }

    if profiler.marks.is_empty() {
        eprintln!("[startup] total before first frame: {total_ms:.1}ms (no phase marks)");
        return;
    }
    let mut prev = profiler.resource_created_at;
    for (name, at) in &profiler.marks {
        let delta = at.duration_since(prev).as_secs_f32() * 1000.0;
        eprintln!("[startup] → {name}: +{delta:.1}ms");
        prev = *at;
    }
    eprintln!("[startup] total before first frame: {total_ms:.1}ms");
}

/// One-shot census of how many systems each schedule carries. The
/// multithreaded executor pays graph bookkeeping per system per run, so
/// these counts are the denominator for any scheduling-overhead work:
/// capture before gating/merging systems, compare after.
///
/// Runs from inside a schedule, so the schedule currently executing (and
/// its ancestors, e.g. `Main`) are temporarily absent from `Schedules`
/// and are not listed; register it somewhere innocuous like `PostStartup`.
pub fn report_schedule_census(schedules: Res<Schedules>, mut reported: Local<bool>) {
    if *reported {
        return;
    }
    *reported = true;
    let mut counts: Vec<(String, usize)> = schedules
        .iter()
        .map(|(label, schedule)| (format!("{label:?}"), schedule.systems_len()))
        .filter(|(_, count)| *count > 0)
        .collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let total: usize = counts.iter().map(|(_, count)| count).sum();
    for (label, count) in &counts {
        eprintln!("[schedule-census] {label}: {count} systems");
    }
    eprintln!("[schedule-census] total (visible schedules): {total} systems");
}

/// Steady-state frame timing. Emits `[frame-spike]` for frames slower than
/// [`Self::SPIKE_MS`] and periodic `[frame-census]` percentile summaries to
/// stderr for correlation with profile timelines.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource)]
pub struct FrameCensus {
    started_at: Instant,
    last_frame_at: Option<Instant>,
    window_started_at: Instant,
    window_frames_ms: Vec<f64>,
    spikes_logged: usize,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for FrameCensus {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            last_frame_at: None,
            window_started_at: now,
            window_frames_ms: Vec::new(),
            spikes_logged: 0,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl FrameCensus {
    /// A frame this slow dropped below 30fps and is worth a line of its own.
    pub const SPIKE_MS: f64 = 33.4;
    /// Summary cadence. Short enough that a 12-chunk timeline gets several
    /// summaries per chunk, long enough to stay quiet during normal play.
    pub const WINDOW_SECS: f64 = 5.0;
    /// A pathological run must not turn the log into the slow thing. After
    /// this many spikes the per-frame lines stop; the percentile summaries
    /// keep reporting, so nothing is silently lost.
    pub const SPIKE_LOG_CAP: usize = 60;

    /// Nearest-rank percentile over an already-sorted slice.
    ///
    /// Shared with the profiling-only interval census in
    /// [`crate::runtime_census`] so the two readouts of the same frame stream
    /// cannot disagree about what p95 means.
    pub fn percentile(sorted: &[f64], q: f64) -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        let rank = (q * (sorted.len() - 1) as f64).round() as usize;
        sorted[rank.min(sorted.len() - 1)]
    }
}

/// Per-frame tick for [`FrameCensus`]. Register in `Last` so it sees the whole
/// frame, including render extract/queue work.
#[cfg(not(target_arch = "wasm32"))]
pub fn report_frame_census(mut census: ResMut<FrameCensus>) {
    let now = Instant::now();
    // The first observation has no predecessor to subtract, and the interval
    // from app construction to frame one is startup, not a frame time —
    // counting it would report a fake multi-second spike every launch.
    let Some(previous) = census.last_frame_at.replace(now) else {
        return;
    };
    let frame_ms = now.duration_since(previous).as_secs_f64() * 1000.0;
    let at = now.duration_since(census.started_at).as_secs_f64();

    if frame_ms >= FrameCensus::SPIKE_MS && census.spikes_logged < FrameCensus::SPIKE_LOG_CAP {
        census.spikes_logged += 1;
        eprintln!("[frame-spike] {at:8.3}s {frame_ms:7.1}ms");
        if census.spikes_logged == FrameCensus::SPIKE_LOG_CAP {
            eprintln!(
                "[frame-spike] {at:8.3}s reached {} logged spikes; further per-frame lines \
                 suppressed (percentile summaries continue)",
                FrameCensus::SPIKE_LOG_CAP
            );
        }
    }

    census.window_frames_ms.push(frame_ms);
    if now.duration_since(census.window_started_at).as_secs_f64() < FrameCensus::WINDOW_SECS {
        return;
    }

    let window_start = census
        .window_started_at
        .duration_since(census.started_at)
        .as_secs_f64();
    let mut sorted = core::mem::take(&mut census.window_frames_ms);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let count = sorted.len();
    eprintln!(
        "[frame-census] {window_start:8.3}s-{at:.3}s frames={count} \
         p50={:.1}ms p95={:.1}ms p99={:.1}ms max={:.1}ms",
        FrameCensus::percentile(&sorted, 0.50),
        FrameCensus::percentile(&sorted, 0.95),
        FrameCensus::percentile(&sorted, 0.99),
        sorted.last().copied().unwrap_or(0.0),
    );
    census.window_started_at = now;
}

// ─────────────────────────────────────────────────────────────────────
// Wasm (browser) implementation — no Instant::now() calls.
// ─────────────────────────────────────────────────────────────────────
//
// `std::time::Instant::now()` panics on `wasm32-unknown-unknown` with
// "time not implemented on this platform". The shapes below match the
// native API so the call sites in `app::plugins::add_simulation_plugins`,
// `app::setup_systems`, and `setup.rs` compile unchanged.

/// Wasm-side placeholder marker. Kept as a `(&'static str, ())` so the
/// `marks: Vec<(&'static str, _)>` field shape mirrors the native impl
/// (only the timestamp type differs) — call sites that push into
/// `marks` keep compiling.
#[cfg(target_arch = "wasm32")]
pub type Mark = ();

#[cfg(target_arch = "wasm32")]
#[derive(Resource, Default)]
pub struct StartupProfiler {
    /// Always empty on wasm — `phase_mark` does not append. Kept for
    /// API parity with the native `Vec<(&'static str, Instant)>`.
    pub marks: Vec<(&'static str, Mark)>,
    /// Set true on the first `report_startup_phases` call so the
    /// "profiling disabled" message only prints once.
    pub reported: bool,
}

/// No-op `phase_mark` on wasm. The native impl pushes an
/// `Instant::now()` entry; on wasm `Instant::now()` would panic, so we
/// hand back a system that does nothing. The Startup `.chain()` order
/// still works because Bevy ordering is independent of the system body.
#[cfg(target_arch = "wasm32")]
pub fn phase_mark(_name: &'static str) -> impl FnMut(ResMut<StartupProfiler>) {
    move |_profiler: ResMut<StartupProfiler>| {}
}

/// Logs once that startup profiling is disabled on wasm and returns.
/// Pairs with the native [`report_startup_phases`] so the
/// `PostStartup` registration in `add_simulation_plugins` is identical
/// across platforms.
/// Wasm placeholder for [`FrameCensus`]: `Instant::now()` panics there, so the
/// resource exists for API parity and the tick below does nothing.
#[cfg(target_arch = "wasm32")]
#[derive(Resource, Default)]
pub struct FrameCensus;

#[cfg(target_arch = "wasm32")]
pub fn report_frame_census(_census: ResMut<FrameCensus>) {}

#[cfg(target_arch = "wasm32")]
pub fn report_startup_phases(mut profiler: ResMut<StartupProfiler>) {
    if profiler.reported {
        return;
    }
    profiler.reported = true;
    bevy::log::info!(
        target: "ambition_platformer2d::profiling",
        "startup profiling disabled on wasm32 (std::time::Instant::now is unsupported); \
         use the browser devtools Performance panel for wall-clock measurement"
    );
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn phase_mark_appends_to_resource() {
        let mut app = App::new();
        app.insert_resource(StartupProfiler::default());
        app.add_systems(Update, phase_mark("test_phase"));
        app.update();
        let profiler = app.world().resource::<StartupProfiler>();
        assert_eq!(profiler.marks.len(), 1);
        assert_eq!(profiler.marks[0].0, "test_phase");
    }

    #[test]
    fn report_runs_once_even_if_called_twice() {
        let mut app = App::new();
        app.insert_resource(StartupProfiler::default());
        app.add_systems(Update, report_startup_phases);
        app.update();
        // resource flag flipped; second run is a no-op.
        let profiler = app.world().resource::<StartupProfiler>();
        assert!(profiler.reported);
        app.update();
    }
}
