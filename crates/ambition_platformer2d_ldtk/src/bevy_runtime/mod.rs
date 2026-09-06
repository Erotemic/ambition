//! Bevy/`bevy_ecs_ldtk` runtime integration over the pure LDtk parser.
//!
//! Typed ECS collision entities currently coexist with the JSON-built runtime
//! collision world while parity is checked.
//! TODO(compat-remove): once parity proves the typed LDtk collision indices are
//! authoritative, delete the JSON collision adapter and its duplicate blocks.

mod asset;
mod components;
mod indices;
mod parity;
mod plugin;
mod systems;

pub use asset::*;
pub use components::*;
pub use indices::*;
pub use parity::*;
pub use plugin::*;
pub use systems::*;
