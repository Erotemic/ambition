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
pub use implementation::*;
pub use plugin::{PortalPlugin, PortalSimulationPlugin};
pub use schedule::PortalSet;
