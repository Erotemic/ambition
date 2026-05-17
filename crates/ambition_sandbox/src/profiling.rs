//! Lightweight startup profiler.
//!
//! Goal: surface "where did the first 5 seconds go" without any
//! external profiler attached. The pattern is `Instant::now()` snapshots
//! taken at named phase boundaries; a final `PostStartup` report system
//! prints per-phase deltas to stderr.
//!
//! Usage: insert `phase_mark("name")` between Startup chain steps.
//!
//! ```ignore
//! .add_systems(Startup, (
//!     phase_mark("startup_begin"),
//!     load_asset_handles,
//!     phase_mark("after_load_assets"),
//!     setup_simulation,
//!     phase_mark("after_setup_sim"),
//! ).chain())
//! ```
//!
//! Output:
//! ```text
//! [startup] startup_begin → after_load_assets: 312.4ms
//! [startup] after_load_assets → after_setup_sim: 41.2ms
//! [startup] total before first frame: 412.7ms
//! ```
//!
//! For deeper per-system profiling, build with `--features profile`,
//! which enables Bevy's `trace_tracy` integration; see
//! `docs/profiling.md`.
//!
//! ## Wasm32 (browser) note
//!
//! `std::time::Instant::now()` panics with `time not implemented on
//! this platform` under `wasm32-unknown-unknown`. Profiling is therefore
//! a no-op on the browser build: [`StartupProfiler`] carries no time
//! fields, [`phase_mark`] is a no-op system, and
//! [`report_startup_phases`] logs once that profiling is disabled.
//! For wall-clock measurement in the browser the right tool is the
//! browser devtools Performance panel.

use bevy::prelude::*;

// ─────────────────────────────────────────────────────────────────────
// Native (non-wasm) implementation — real Instant-based profiling.
// ─────────────────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource)]
pub struct StartupProfiler {
    /// When the App was constructed. All deltas are computed from
    /// here so a "total" line at the end of Startup represents wall-
    /// clock from App::new() to first PostStartup tick.
    pub app_constructed_at: Instant,
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
        Self {
            app_constructed_at: Instant::now(),
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
    if profiler.marks.is_empty() {
        eprintln!("[startup] total before first frame: {total_ms:.1}ms (no phase marks)");
        return;
    }
    let mut prev = profiler.app_constructed_at;
    for (name, at) in &profiler.marks {
        let delta = at.duration_since(prev).as_secs_f32() * 1000.0;
        eprintln!("[startup] → {name}: +{delta:.1}ms");
        prev = *at;
    }
    eprintln!("[startup] total before first frame: {total_ms:.1}ms");
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
#[cfg(target_arch = "wasm32")]
pub fn report_startup_phases(mut profiler: ResMut<StartupProfiler>) {
    if profiler.reported {
        return;
    }
    profiler.reported = true;
    bevy::log::info!(
        target: "ambition::profiling",
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
