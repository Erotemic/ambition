//! Backend-agnostic authored world IR.
//!
//! This crate owns the room graph, authored placement records, room metadata,
//! and moving-platform math. Backend adapters such as LDtk convert into these
//! types; simulation crates interpret them through explicit lowering seams.

pub mod debug_label;
pub mod placements;
pub mod platforms;
pub mod rooms;

pub use debug_label::{DebugLabel, DebugLabelKind};
