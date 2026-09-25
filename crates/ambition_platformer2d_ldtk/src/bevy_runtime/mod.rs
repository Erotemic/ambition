//! Bevy/`bevy_ecs_ldtk` runtime integration over the pure LDtk parser.
//!
//! ⛔ Collision is not read from plugin-spawned entities. The room's collision
//! world is built from the world IR the LDtk backend converts into (ADR 0021),
//! which a RON room and a rollback peer build the same way. The spawned
//! entities feed only the debug overlay and the headless summary.

mod asset;
mod components;
mod indices;
mod plugin;
mod systems;

pub use asset::*;
pub use components::*;
pub use indices::*;
pub use plugin::*;
pub use systems::*;
