//! Backend-agnostic authored world IR.
//!
//! This crate owns the room graph, authored placement records, room metadata,
//! moving-platform math, and the composited [`collision`] world every sweep and
//! raycast reads. Backend adapters such as LDtk convert into these types;
//! simulation crates interpret them through explicit lowering seams.

pub mod collision;
pub mod debug_label;
pub mod placements;
pub mod platforms;
pub mod ron_room;
pub mod rooms;

pub use debug_label::{DebugLabel, DebugLabelKind};

// The world-IR dependency-purity ratchet moved to the workspace-policy package
// (repository structure, not a crate-local behavioral invariant):
// `engine.world-ir-dependency-allowlist` in
// `tests/ambition_workspace_policy/policies/engine.toml`.
