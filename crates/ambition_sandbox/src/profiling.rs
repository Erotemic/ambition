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

use std::time::Instant;

use bevy::prelude::*;

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
pub fn phase_mark(name: &'static str) -> impl FnMut(ResMut<StartupProfiler>) {
    move |mut profiler: ResMut<StartupProfiler>| {
        profiler.marks.push((name, Instant::now()));
    }
}

/// PostStartup report. Runs once: prints per-phase deltas + total
/// startup time to stderr. Single fmt block, easy to grep.
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

#[cfg(test)]
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
