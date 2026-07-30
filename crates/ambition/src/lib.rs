//! Public facade for Ambition-derived platformer games.
//!
//! This crate is the E9 umbrella surface: a downstream game should depend on
//! `ambition` plus its own content crate instead of copying the app shell's wall
//! of lower `ambition_*` dependencies. It deliberately re-exports the engine,
//! host, renderer, model, and vocabulary crates without depending on any named
//! game content or the `ambition_app` shell.

pub mod app;
pub mod game_assets;
pub mod prelude;
pub mod session_world;

/// The platformer experience-provider protocol (authoring identity + the one
/// shared preparation/activation lifecycle). Lives in
/// `ambition_platformer_provider`; re-exported here so provider crates keep
/// the `ambition::provider::…` path.
pub use ambition_platformer_provider as provider;

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
/// **Bodies: what a game queries, moves and transits.**
///
/// A curated domain module, second of the set ADR 0031's decision 1 lists. It
/// exists because the two sentinel consumers had to reach into
/// `ambition::engine_core` — an IMPLEMENTATION crate the facade mirrors — for
/// `transit_body`, `TransitVelocity` and `BodyClusterQueryData`, and into
/// `ambition::platformer` for the marker and kinematics types you need to ask
/// "where is the player".
///
/// Closed list, like [`world`]. Adding a type to a mirrored crate does not
/// silently become public API.
pub mod actor {
    /// Who the body is.
    pub use ambition_platformer_primitives::markers::PrimaryPlayer;

    /// Where the body is, and how it moves.
    pub use ambition_engine_core::movement::{transit_body, TransitVelocity};
    pub use ambition_engine_core::BodyClusterQueryData;
    pub use ambition_platformer_primitives::body::BodyKinematics;
}

/// **The simulation schedule a game joins its own systems to.**
///
/// A game never names a literal Bevy schedule: it asks for the sim schedule and
/// a semantic set, so the same system runs under the fixed tick and a GGRS host
/// alike. That indirection is the engine's rule, and before this the only way
/// to reach it was `ambition::platformer::schedule` — the crate mirror.
pub mod sim {
    pub use ambition_platformer_primitives::schedule::{
        GameMode, SandboxSet, SimSchedule, SimScheduleExt,
    };
}

/// **What is drawn, as a game observes it.**
///
/// Deliberately thin. A consumer reads the presented world; it does not own the
/// render path.
pub mod view {
    pub use ambition_platformer_primitives::lifecycle::RoomVisual;
}

/// **The authored world: rooms, geometry, placements, collision.**
///
/// ⚠ A CURATED MODULE, not a crate mirror — and the difference is the whole
/// point of ADR 0031. `pub use ambition_world as world` made the compatibility
/// surface change whenever the crate did: a new submodule became public API by
/// existing. This list is CLOSED, so adding one to `ambition_world` is an
/// internal change until somebody adds it here on purpose.
///
/// `ron_room` is deliberately absent. It is an authoring backend, nothing
/// outside the engine reaches for it, and the mirror was publishing it anyway.
///
/// This is the first module to get the treatment; the rest of the crate mirrors
/// in this file are still mirrors, and each is a leak the campaign's ratchets
/// still count.
pub mod world {
    /// Everything needed to author a room, in one import.
    pub use ambition_world::prelude;

    pub use ambition_world::{collision, debug_label, placements, platforms, rooms};
}
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
        SimCoreResourcesPlugin, SimulationHost, SimulationHostAppExt,
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
    pub use ambition_render::dialog_ui::{
        DefaultDialogUiPlugin, DialogOverlayRoot, DialogPresentationSet,
    };
    pub use ambition_render::platformer_presentation::{
        PlatformerPresentationPlugin, PlatformerPresentationSetupSet,
    };

    /// How gameplay is framed on the physical display: the four presentation
    /// policy axes, the pure layout resolver, and the tested author presets a
    /// provider declares with
    /// [`with_presentation_profiles`](ambition_platformer_provider::PlatformerExperienceAuthoring::with_presentation_profiles).
    pub use ambition_platformer_primitives::gameplay_presentation;
    pub use ambition_platformer_primitives::gameplay_presentation::profiles;

    /// The DECLARED-HUD seam: a game declares its readout slots with
    /// [`with_hud`](ambition_platformer_provider::PlatformerExperienceAuthoring::with_hud)
    /// and publishes their live values into [`HudReadouts`] each frame. The
    /// engine holds no readout vocabulary — every label is a string the game
    /// writes.
    pub use ambition_platformer_primitives::gameplay_presentation::{
        ActiveHudDeclaration, HudDeclaration, HudReadout, HudReadouts, HudSlotId, HudSlotSpec,
        SurroundRegion,
    };
    pub use ambition_render::hud::declared::{DeclaredHudPlugin, DeclaredHudRoot, DeclaredHudSlot};
}
