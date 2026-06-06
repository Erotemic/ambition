//! Portal mechanic facade.
//!
//! The implementation is still largely the original portal vertical-slice code,
//! but the module now has explicit seams for plugin registration, schedule
//! vocabulary, and implementation details. This keeps `app/plugins.rs` from
//! owning portal internals and gives future extraction patches stable files to
//! split further.

mod implementation;
mod plugin;
mod schedule;

pub use crate::platformer_runtime::collision::raycast_solids;
pub use crate::platformer_runtime::orientation::{ensure_actor_roll, update_actor_roll, ActorRoll};
pub use crate::platformer_runtime::transit::rotate_velocity_between_normals as portal_transform_velocity;
pub use implementation::*;
pub use plugin::{PortalPlugin, PortalSimulationPlugin};
pub use schedule::PortalSet;
