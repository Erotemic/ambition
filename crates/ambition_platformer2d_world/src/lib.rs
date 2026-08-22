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
mod snapshot_impls;
pub mod world_manifest;

pub use debug_label::{DebugLabel, DebugLabelKind};

/// Domain prelude for authored room data, including the geometry vocabulary
/// needed to describe room solids.
pub mod prelude {
    pub use crate::rooms::{RoomMetadata, RoomSpec};
    pub use ambition_platformer2d_core::{Block, RoomGeometry, Vec2};

    /// Authored world IR, renamed to avoid colliding with `bevy::prelude::World`.
    pub use ambition_platformer2d_core::World as AuthoredWorld;
}

// The world-IR dependency-purity ratchet moved to the workspace-policy package
// (repository structure, not a crate-local behavioral invariant):
// `engine.world-ir-dependency-allowlist` in
// `tests/ambition_workspace_policy/policies/engine.toml`.
