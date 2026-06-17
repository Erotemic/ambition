//! Developer-facing tooling that stays in the machinery lib: the dev
//! STATE (`dev_tools`: DeveloperTools + editable profiles, read by
//! persistence + presentation), the gameplay `trace` recorder (written
//! by sim code), and the startup `profiling` marks (read by audio).

pub mod dev_tools;
pub mod profiling;
pub mod trace;
