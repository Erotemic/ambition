//! Gravity-zone mechanic.
//!
//! The zones/switches that flip the room's ambient gravity, plus their visuals.
//! Extracted out of `ambition_portal` (Stage 6 follow-up / ADR 0019): this is a
//! *gravity mechanic*, not a portal helper, so it owns its own registration via
//! [`GravityPlugin`] and must NOT depend on `ambition_portal`.
//!
//! The underlying ambient-gravity types/resources ([`crate::physics::BaseGravity`],
//! [`crate::physics::GravityField`], [`crate::physics::GravityZone`], the
//! [`crate::physics::GravityZones`] snapshot and its `oscillate`/`collect`
//! systems) stay in [`crate::physics`] because they are read widely; this module
//! owns the gravity-zone *mechanic* layered on top.

mod lifecycle;
mod plugin;

pub use lifecycle::{gravity_flip_switch_system, reset_gravity_on_room_reset, GravityFlipSwitch};
pub use plugin::{GravityPlugin, GravitySet};
