//! Developer-facing tooling: inspectors, overlays, the gameplay trace
//! recorder, the mechanics registry, and the startup profiler.
//!
//! These submodules are presentation-only support for development. None
//! of them is on the critical gameplay path; gating the whole umbrella
//! behind a `cfg(feature = "...")` in a future pass is straightforward
//! because nothing inside `engine`/sim depends on it.

pub mod debug_overlay;
pub mod dev_tools;
pub mod fps_overlay;
pub mod mechanics;
pub mod profiling;
pub mod trace;
