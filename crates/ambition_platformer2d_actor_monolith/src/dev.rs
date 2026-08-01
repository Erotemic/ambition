//! Sim-side developer tooling that still samples actor-domain state.
//!
//! The reusable developer-tool state, profiles, startup profiler, persistence,
//! and live-edit sync system live in `ambition_dev_tools`. Actor internals and
//! external consumers name that crate directly now; this module keeps only the
//! gameplay trace recorder because it reads live actor/world/portal/session
//! state.

/// Sim-side gameplay trace recorder (written by projectile/encounter/etc.
/// systems). The trace FORMAT lives in `ambition_gameplay_trace`; the
/// recording SYSTEMS stay here because they read sim-only state.
pub mod trace;
