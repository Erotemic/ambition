//! Advance the simulation by a COUNT OF TICKS instead of by the wall clock.
//!
//! ⭐⭐ WHY THIS EXISTS. Both simulation hosts advance from elapsed REAL TIME:
//! `Fixed60Hz` through `Time::<Fixed>`, and the rollback host through
//! `bevy_ggrs`, whose driver accumulates `Time::delta()` and steps
//! `while accumulator >= fps_delta`. For a game that is exactly right — the
//! world should run at the speed of the room it is in.
//!
//! For a HEADLESS DRIVER it is neither right nor harmless. A tool that spins
//! `app.update()` in a loop gets ZERO, ONE or SEVERAL sim ticks per call
//! depending on how long the previous iteration happened to take, so:
//!
//! - a recording costs at least as much wall time as the game time it contains
//!   (`moveset_takes` spent 7 minutes on 19 takes of 2.5 seconds each); and
//! - the SAME BINARY on the SAME TREE produces different output run to run.
//!   Measured 2026-08-27: two runs agreed on 6 of 19 takes.
//!
//! ⛔⛔ THE SEAM IS `TimeUpdateStrategy`, WHICH IS NOT A HACK AND THE
//! ALTERNATIVES ARE. Bevy's own answer to *"what does `now` mean"* is
//! `TimeUpdateStrategy::ManualDuration`: `Time<Real>` stops reading the wall
//! clock and advances by exactly that much per `update()`. Everything
//! downstream is untouched — virtual follows real as always, and `bevy_ggrs`
//! reads `Time::delta()` as always — so the accumulator receives precisely one
//! tick's worth of time per frame. The clock is told what `now` means, which is
//! the entire reason the API exists.
//!
//! ⛔ PAUSING `Time<Virtual>` AND ADVANCING IT BY HAND DOES NOT WORK, and it was
//! the obvious first attempt: `TimePlugin` REWRITES the virtual clock from the
//! real one every frame, so the manual advance is discarded before anything
//! reads it. Measured — 120 steps that way produced ZERO sim ticks.
//!
//! What would be a hack, and what this deliberately does not do:
//!
//! - poking `bevy_ggrs`'s own accumulator (private, and it is lying to a clock);
//! - running `GgrsSchedule` directly, which skips save, rollback, prediction and
//!   checksum — a DIFFERENT SIMULATION wearing the same name, which is the worst
//!   possible outcome for a tool whose job is to be evidence;
//! - fabricating `Time<Real>` deltas, which every diagnostic that measures how
//!   long a frame took would then report as a lie.
//!
//! ⭐ SO THE FULL ADVANCE PATH IS KEPT. Save, rollback, prediction and checksum
//! all run exactly as they do in the shipped game; the only thing that changes
//! is WHO DECIDES that a tick's worth of time has passed.
//!
//! ⚠ ADDITIVE. Nothing in production spins `app.update()` — the Bevy runner
//! does — so no existing caller changes behaviour by this file existing. A
//! driver opts in.

use bevy::prelude::*;
use bevy::time::Real;

/// One simulation tick, as a duration.
///
/// Nanoseconds from an integer division rather than `Duration::from_secs_f64`,
/// for the same reason `bevy_ggrs` computes its own period that way: the two
/// have to agree exactly or the accumulator drifts a tick every few seconds.
pub fn tick_period() -> std::time::Duration {
    std::time::Duration::from_nanos(1_000_000_000u64 / crate::SIM_TICK_HZ as u64)
}

/// Put the app's clock under manual control.
///
/// ⛔ CALL ONCE, BEFORE STEPPING. Until this is set, the wall clock decides how
/// many ticks each `update()` is worth.
pub fn take_manual_control(app: &mut App) {
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        tick_period(),
    ));
}

/// Hand the clock back to the wall.
pub fn release_manual_control(app: &mut App) {
    app.insert_resource(bevy::time::TimeUpdateStrategy::Automatic);
}

/// Advance the simulation by exactly `ticks` simulation ticks.
///
/// ⛔⛔ ONE `update()` PER TICK, NOT ONE `update()` FOR ALL OF THEM. Handing the
/// accumulator N ticks' worth of time in one go would run N sim steps inside a
/// SINGLE frame — which is what a stalled host does when it catches up, and it
/// is not what a driver means. Systems outside the sim schedule (presentation,
/// view rebuilds, the input latch) run once per frame, so a driver that wants
/// N ticks wants N frames.
///
/// # Panics
///
/// If the clock is not paused. A silent no-op here would look exactly like a
/// simulation that ran, and the caller would record wall-clock ticks believing
/// they were deterministic ones — the precise failure this module exists to end.
pub fn step_simulation(app: &mut App, ticks: usize) {
    assert!(
        matches!(
            app.world().get_resource::<bevy::time::TimeUpdateStrategy>(),
            Some(bevy::time::TimeUpdateStrategy::ManualDuration(_))
        ),
        "step_simulation requires the clock under manual control — call \
         `take_manual_control` first, or the wall clock is still deciding how \
         many ticks each step is worth"
    );
    for _ in 0..ticks {
        app.update();
    }
}

/// How many simulation ticks this world has run.
///
/// ⭐ THE FACT A DRIVER SHOULD ASSERT ON. "I called `update()` 150 times" is a
/// statement about a loop; this is a statement about the SIMULATION, and until
/// now the two were silently different numbers.
pub fn sim_tick(app: &App) -> u64 {
    app.world()
        .get_resource::<ambition_time::SimTick>()
        .map(|t| t.0)
        .unwrap_or(0)
}

/// Real elapsed time is still real: a driver that pauses the world must not make
/// the diagnostics that measure frame cost report a lie.
pub fn real_elapsed(app: &App) -> std::time::Duration {
    app.world()
        .get_resource::<Time<Real>>()
        .map(|t| t.elapsed())
        .unwrap_or_default()
}
