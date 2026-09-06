//! Curated imports for games built on the Ambition engine facade.

pub use bevy::app::{PluginGroup, PluginGroupBuilder};
pub use bevy::prelude::{App, Plugin};

pub use crate::engine::{
    add_headless_foundation, init_engine_states, PlatformerEnginePlugins, SimulationHost,
    SimulationHostAppExt,
};
pub use crate::runtime;
pub use crate::windowed_host::PlatformerHostPlugins;

pub use crate::{
    actors, asset_manager, characters, combat, engine_core, game_shell, host, input, load,
    load_presentation, platformer, sim_view, sprite_sheet, time, world,
};

// Capability edges (slice H): present exactly when their feature is — which
// the default feature set turns on wholesale.
#[cfg(feature = "ambition_dialog")]
pub use crate::dialog;
#[cfg(feature = "ambition_encounter")]
pub use crate::encounter;
#[cfg(feature = "ambition_menu")]
pub use crate::menu;
#[cfg(feature = "ambition_platformer2d_ldtk")]
pub use crate::ldtk_map;
#[cfg(feature = "ambition_persistence")]
pub use crate::persistence;
#[cfg(feature = "ambition_projectiles")]
pub use crate::projectiles;
#[cfg(feature = "ambition_render")]
pub use crate::render;

#[cfg(feature = "relativity")]
pub use crate::{relativity, relativity2d};
