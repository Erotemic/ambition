//! Public facade for Ambition-derived platformer games.
//!
//! This crate is the E9 umbrella surface: a downstream game should depend on
//! `ambition` plus its own content crate instead of copying the app shell's wall
//! of lower `ambition_*` dependencies. It deliberately re-exports the engine,
//! host, renderer, model, and vocabulary crates without depending on any named
//! game content or the `ambition_app` shell.

pub mod prelude;
pub mod provider;
pub mod session_world;

pub use ambition_actors as actors;
pub use ambition_asset_manager as asset_manager;
pub use ambition_audio as audio;
pub use ambition_characters as characters;
pub use ambition_combat as combat;
pub use ambition_cutscene as cutscene;
pub use ambition_dev_tools as dev_tools;
pub use ambition_dialog as dialog;
pub use ambition_encounter as encounter;
pub use ambition_engine_core as engine_core;
pub use ambition_entity_catalog as entity_catalog;
pub use ambition_game_shell as game_shell;
pub use ambition_host as host;
pub use ambition_input as input;
pub use ambition_interaction as interaction;
pub use ambition_inventory_ui as inventory_ui;
pub use ambition_items as items;
pub use ambition_ldtk_map as ldtk_map;
pub use ambition_load as load;
pub use ambition_load_presentation as load_presentation;
pub use ambition_menu as menu;
pub use ambition_persistence as persistence;
pub use ambition_platformer_primitives as platformer;
pub use ambition_portal as portal;
pub use ambition_portal_presentation as portal_presentation;
pub use ambition_projectiles as projectiles;
pub use ambition_render as render;
pub use ambition_runtime as runtime;
pub use ambition_settings_menu as settings_menu;
pub use ambition_sfx as sfx;
pub use ambition_sfx_bank as sfx_bank;
pub use ambition_sim_view as sim_view;
pub use ambition_sprite_sheet as sprite_sheet;
pub use ambition_time as time;
pub use ambition_touch_input as touch_input;
pub use ambition_ui_nav as ui_nav;
pub use ambition_vfx as vfx;
pub use ambition_world as world;
// Re-exported so a game can name bevy TYPES through `ambition::bevy::…`. NOTE:
// this does NOT let a crate `#[derive(Component)]`/`#[derive(Resource)]` through
// the umbrella alone — bevy's derive macros resolve `::bevy_ecs` via the
// CONSUMER's own Cargo.toml (`BevyManifest`), which a re-export does not satisfy.
// A content crate that defines its own components/resources must ALSO list `bevy`
// in its manifest (one line, version pinned by the workspace). See
// docs/planning/demos/README.md.
pub use bevy;

/// Engine assembly helpers most games need first.
pub mod engine {
    pub use ambition_runtime::{
        add_headless_foundation, init_engine_states, PlatformerEnginePlugins, SandboxSetsPlugin,
        SimCoreResourcesPlugin,
    };
}

/// Windowed host plugin groups and host-facing seams.
pub mod windowed_host {
    #[cfg(feature = "input")]
    pub use ambition_host::HostInputBindingsPlugin;
    pub use ambition_host::{HostCameraPlugin, PlatformerHostPlugins};
}

/// Default renderer facade.
pub mod renderer {
    pub use ambition_render::*;
}

/// The generic platformer PRESENTATION face: a camera, the room's static visuals,
/// and the sprite/animation chain. A demo adds this beside the engine and host
/// groups; a game layers its own HUD/menus/dev stack on top (oracle-violation OV1).
pub mod presentation {
    pub use ambition_render::platformer_presentation::{
        PlatformerPresentationPlugin, PlatformerPresentationSetupSet,
    };
}
