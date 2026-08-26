//! Gravity-zone mechanic.
//!
//! The zones/switches that flip the room's ambient gravity, plus their visuals.
//! Extracted out of `ambition_portal2d` (Stage 6 follow-up / ADR 0019): this is a
//! *gravity mechanic*, not a portal helper, so it owns its own registration via
//! [`GravityPlugin`] and must NOT depend on `ambition_portal2d`.
//!
//! The underlying ambient-gravity types/resources — `BaseGravity`,
//! `GravityField`, `GravityZone`, the `GravityZones` snapshot and its
//! `oscillate`/`collect` systems — live in [`ambition_platformer2d_shared_tangle::gravity`],
//! because they are read far more widely than this mechanic. This module
//! owns the gravity-zone *mechanic* layered on top and names that crate
//! directly: there is no `crate::physics` facade to spell them through.

mod lifecycle;
mod plugin;
mod resolve;

pub use lifecycle::{gravity_flip_switch_system, reset_gravity_on_room_reset, GravityFlipSwitch};
pub use plugin::{GravityPlugin, GravitySet};
pub use resolve::resolve_body_motion_frames;
