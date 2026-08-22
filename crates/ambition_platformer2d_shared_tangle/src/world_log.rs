//! Coarse `[game-mode]` / `[world-event]` logging.
//!
//! Lines go to stderr with marker prefixes consumed by profiling scripts and
//! Android logcat. Every line includes the Bevy frame number; `mirror_frame_count`
//! publishes the `FrameCount` value early enough for helper functions that cannot
//! receive the resource directly. Marker additions must also be recognized by the
//! profiling timeline parser. `bevy::platform::time::Instant` keeps the same
//! implementation portable to wasm.

use core::fmt::Arguments;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};

use bevy::diagnostic::FrameCount;
use bevy::platform::time::Instant;
use bevy::prelude::*;

use crate::schedule::GameMode;

/// A pathological run must not turn logging into the slow thing. Mirrors
/// `FrameCensus::SPIKE_LOG_CAP`'s intent with a much larger budget, because every
/// line here is edge-triggered: a quiet minute emits nothing, so a run that
/// reaches this cap has a real storm to report and the first few thousand lines
/// already describe it.
pub const WORLD_LOG_CAP: usize = 4000;

/// Origin for the elapsed-seconds stamp. Pinned by [`install`] at App build time
/// so `[game-mode]` seconds line up with `[frame-spike]` / `[frame-census]`, whose
/// origin is their resource's construction.
// AMBITION_REVIEW(determinism): wall clock, and deliberately so — this is the
// stamp on a stderr LOG LINE. Nothing in this module returns a time to a caller
// and no system reads it, so no sim decision can observe it. `SimTick` would be
// the wrong quantity anyway: this instrument exists to be lined up against
// `[frame-spike]` seconds and against an Android logcat timestamp, both of which
// are wall clock. The FRAME index, which is the part a sim question actually
// turns on, comes from `FrameCount` and not from here.
static STARTED_AT: LazyLock<Instant> = LazyLock::new(Instant::now);

/// Projection of [`FrameCount`]. See the module docs — this is what lets a leaf
/// call site stamp a frame without threading a resource through it.
static FRAME: AtomicU32 = AtomicU32::new(0);

/// Lines emitted so far, against [`WORLD_LOG_CAP`].
static LINES: AtomicUsize = AtomicUsize::new(0);

/// The last `(mode, cause)` handed to [`note_game_mode_request`], so a setter that re-asks
/// every frame while its condition holds prints once instead of at 60Hz.
static LAST_REQUEST: Mutex<Option<(GameMode, &'static str)>> = Mutex::new(None);

/// The frame index every marker line stamps. Reads the [`FRAME`] projection, so
/// it is callable from anywhere — including plain helper functions with no ECS
/// access.
pub fn frame() -> u32 {
    FRAME.load(Ordering::Relaxed)
}

/// Seconds since [`install`] pinned the origin.
pub fn elapsed_secs() -> f64 {
    STARTED_AT.elapsed().as_secs_f64()
}

/// Emit one `[marker] {seconds}s f{frame} {body}` line on stderr.
///
/// The single formatter for this family: a caller supplies the marker and the
/// body, never the stamp, so `[game-mode]`, `[sim-clock]`, and `[world-event]`
/// lines stay column-aligned with each other and with `[frame-spike]`.
pub fn log_line(marker: &str, body: Arguments<'_>) {
    let seen = LINES.fetch_add(1, Ordering::Relaxed);
    if seen > WORLD_LOG_CAP {
        return;
    }
    let at = elapsed_secs();
    let frame = frame();
    if seen == WORLD_LOG_CAP {
        eprintln!(
            "[{marker}] {at:8.3}s f{frame:>7} reached {WORLD_LOG_CAP} world-log lines; \
             further lines suppressed"
        );
        return;
    }
    eprintln!("[{marker}] {at:8.3}s f{frame:>7} {body}");
}

/// Emit a `[world-event]` line: a coarse world transition (room change, session
/// start/end, a room reset, a boss phase).
///
/// Call this AT the site that already owns the event, never from a second
/// system that re-derives it. Most of these facts already have exactly one
/// authoritative site — some of which already emit a `tracing` line — and this
/// marker is an additional SINK at that same site, not a parallel emission point.
pub fn world_event(body: Arguments<'_>) {
    log_line("world-event", body);
}

/// Emit a `[sim-clock]` line. Kept here (rather than beside the clock) so the
/// marker family has one formatter; the edge logic lives with the clock's write
/// authority, which is the only place that can say what changed and why.
pub fn sim_clock(body: Arguments<'_>) {
    log_line("sim-clock", body);
}

/// The cause half of `[game-mode]`. Call immediately beside a
/// `NextState<GameMode>::set`, naming the system that asked.
///
/// `Playing -> Paused` on its own does not say who asked for it; this does, and
/// the frame stamp says whether the ask preceded or followed some other system's
/// read. Deliberately a plain function rather than a `set` wrapper: wrapping the
/// setter would force every menu/shell crate that touches `GameMode` to take a
/// dependency it does not otherwise need.
pub fn note_game_mode_request(requested: GameMode, cause: &'static str) {
    let mut last = LAST_REQUEST
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if *last == Some((requested, cause)) {
        return;
    }
    *last = Some((requested, cause));
    drop(last);
    log_line(
        "game-mode",
        format_args!("request {} <- {cause}", requested.label()),
    );
}

/// Project [`FrameCount`] into [`FRAME`]. Registered in `First` so every line
/// emitted during a frame reports that frame's index — `FrameCount` itself is
/// incremented in `Last`, so an `Update` system and a `PostUpdate` census agree.
///
/// `Option<Res<_>>` because `FrameCount` comes from `FrameCountPlugin`; both
/// `DefaultPlugins` and `MinimalPlugins` carry it, but a hand-assembled `App::new`
/// test may not, and an instrument must not be the thing that panics.
pub fn mirror_frame_count(count: Option<Res<FrameCount>>) {
    if let Some(count) = count {
        FRAME.store(count.0, Ordering::Relaxed);
    }
}

/// Observes `State<GameMode>` rather than reading `StateTransitionEvent`, because
/// the question this answers is "what is the mode now", and a `Local` copy makes
/// the very first observation reportable too (`initial`) without a second code
/// path. Registered in `PostUpdate`: `StateTransition` runs before `Update`, so
/// PostUpdate sees this frame's landing under this frame's number.
pub fn report_game_mode_transitions(mode: Res<State<GameMode>>, mut last: Local<Option<GameMode>>) {
    let now = *mode.get();
    match *last {
        Some(previous) if previous == now => return,
        Some(previous) => log_line(
            "game-mode",
            format_args!("{} -> {}", previous.label(), now.label()),
        ),
        None => log_line("game-mode", format_args!("initial {}", now.label())),
    }
    *last = Some(now);
    // A landing invalidates the request memo: the next ask for the same mode is
    // new information, not the same system repeating itself.
    *LAST_REQUEST
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}

/// Pin the elapsed-seconds origin and register the frame projection + the
/// `[game-mode]` census. Called from the engine's core-resources plugin so every
/// composition — visible host, headless sim, demo shell — carries the instrument.
pub fn install(app: &mut App) {
    LazyLock::force(&STARTED_AT);
    app.add_systems(First, mirror_frame_count);
    app.add_systems(PostUpdate, report_game_mode_transitions);
}
