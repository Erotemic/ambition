//! Curated imports for games built on the Ambition engine facade.

pub use bevy::app::{PluginGroup, PluginGroupBuilder};
pub use bevy::prelude::{App, Plugin};

pub use crate::engine::{add_headless_foundation, init_engine_states, PlatformerEnginePlugins};
pub use crate::runtime;
pub use crate::windowed_host::PlatformerHostPlugins;

pub use crate::{
    actors, asset_manager, characters, combat, dialog, encounter, engine_core, host, input,
    ldtk_map, menu, persistence, platformer, projectiles, render, sim_view, sprite_sheet, time,
    world,
};
