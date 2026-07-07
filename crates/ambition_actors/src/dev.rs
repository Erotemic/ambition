//! Developer-facing tooling.
//!
//! The dev-tool STATE + logic — `DeveloperTools`, the reflected editable
//! player-tuning / ability / stats resources, the profile enums, the startup
//! profiler, `DeveloperTools` disk persistence, and the live-edit sync system —
//! was carved into the foundational `ambition_dev_tools` crate (E1d). They are
//! re-exported here on the historical `crate::dev::*` paths so the wide set of
//! consumers (render, sim_view, runtime, app, menu) need no import edits.
//!
//! What stays sim-side: the gameplay `trace` recorder, which samples live
//! `player`/`features`/`rooms`/`portal`/`game_mode` state and therefore cannot
//! live in a foundational crate.

pub use ambition_dev_tools::{dev_tools, profiling, sync_live_player_dev_edits_system};

/// Sim-side gameplay trace recorder (written by projectile/encounter/etc.
/// systems). The trace FORMAT lives in `ambition_gameplay_trace`; the
/// recording SYSTEMS stay here because they read sim-only state.
pub mod trace;
