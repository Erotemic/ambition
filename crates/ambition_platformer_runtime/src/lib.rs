//! Reusable platformer runtime primitives extracted from `ambition_sandbox`.
//!
//! This is the first real crate carved out of the proto-runtime during the
//! plugin refactor (see `docs/planning/plugin_refactor/14_action_plan.md`,
//! Stage 13 / Task K). It holds the import-clean, Ambition-content-free seams:
//! entity lifecycle vocabulary and generic schedule sets. It depends only on
//! `bevy` (ECS), `glam`/std, and never on `ambition_sandbox`, Ambition content,
//! presentation, app assembly, or devtool modules.
//!
//! Modules that still reach back into the sandbox (`collision`, `orientation`)
//! remain in `ambition_sandbox::platformer_runtime` for now; they are the
//! not-yet-extracted remainder and are tracked by the architecture-boundary
//! guardrail.

pub mod lifecycle;
pub mod math;
pub mod prelude;
pub mod schedule;
pub mod transit;
