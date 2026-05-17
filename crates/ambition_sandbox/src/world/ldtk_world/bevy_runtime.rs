//! Bevy + bevy_ecs_ldtk plugin glue and runtime-spine indexing for the
//! sandbox's LDtk integration.
//!
//! This submodule isolates everything that needs `bevy_ecs_ldtk` types
//! (PluginEntityInstance, LevelSet, LdtkEntity, asset Handle<LdtkProject>)
//! from the pure-Rust LDtk JSON parser / validator / surface compiler in
//! `super`. Step C of `docs/path_forward.md` calls for splitting
//! `ldtk_world.rs`; this is the bevy_ecs_ldtk-using half of that split.
//! Once the runtime-spine roadmap (`memory project_ldtk_roadmap`) is
//! complete enough for the JSON adapter to retire, this becomes the
//! collision authority too.

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
