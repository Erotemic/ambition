//! Developer-facing tooling that stays in the machinery lib: the dev
//! STATE (`dev_tools`: DeveloperTools + editable profiles, read by
//! persistence + presentation), the gameplay `trace` recorder (written
//! by sim code), and the startup `profiling` marks (read by audio).
//!
//! The F1 debug overlay + F3 FPS counter (pure presentation, no lib
//! consumer) moved up to `ambition_app::dev` (Stage 20 devtools split).
//!
//! These submodules are presentation-only support for development. None
//! of them is on the critical gameplay path; gating the whole umbrella
//! behind a `cfg(feature = "...")` in a future pass is straightforward
//! because nothing inside `engine`/sim depends on it.

pub mod dev_tools;
pub mod profiling;
pub mod trace;
