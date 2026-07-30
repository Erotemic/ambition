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

/// **Everything needed to author a room, in one import.**
///
/// The domain prelude `docs/sdk/api-prototype.md` §5 specified, and ADR 0031's
/// public module list already names `ambition::world` as semantic surface.
///
/// ⚠ It re-exports the GEOMETRY vocabulary too, and that is the point. The
/// movement-only minimal game had to name `ambition::engine_core` — an
/// implementation crate the facade mirrors — for nothing more than `Vec2`,
/// `World` and `Block` while describing a floor. A consumer reaching into a
/// crate called `engine_core` to place a rectangle is the namespace mirror
/// leaking, and it is a leak only a SECOND consumer surfaced: Outlander names
/// that crate for real simulation vocabulary too, so its appearance there
/// proves nothing.
///
/// Domain prelude, not a root one. `ambition::prelude` re-exports twenty-five
/// crate mirrors; an agent told to import all of them has been told nothing
/// about which four matter.
pub mod prelude {
    pub use crate::rooms::{RoomMetadata, RoomSpec};
    pub use ambition_engine_core::{Block, RoomGeometry, Vec2, World};
}

// The world-IR dependency-purity ratchet moved to the workspace-policy package
// (repository structure, not a crate-local behavioral invariant):
// `engine.world-ir-dependency-allowlist` in
// `tests/ambition_workspace_policy/policies/engine.toml`.
