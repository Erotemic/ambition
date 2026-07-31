//! Public facade for Ambition-derived platformer games.
//!
//! This crate is the E9 umbrella surface: a downstream game should depend on
//! `ambition` plus its own content crate instead of copying the app shell's wall
//! of lower `ambition_*` dependencies. It deliberately re-exports the engine,
//! host, renderer, model, and vocabulary crates without depending on any named
//! game content or the `ambition_app` shell.

pub mod app;
/// Asset install for a game that DRAWS — rides the `ambition_render` capability
/// edge, because its Startup ordering anchor is `ambition_render`'s.
#[cfg(feature = "ambition_render")]
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
#[cfg(feature = "ambition_cutscene")]
pub use ambition_cutscene as cutscene;
pub use ambition_dev_tools as dev_tools;
#[cfg(feature = "ambition_dialog")]
pub use ambition_dialog as dialog;
#[cfg(feature = "ambition_encounter")]
pub use ambition_encounter as encounter;
pub use ambition_engine_core as engine_core;
pub use ambition_entity_catalog as entity_catalog;
pub use ambition_game_shell as game_shell;
pub use ambition_host as host;
pub use ambition_input as input;
pub use ambition_interaction as interaction;
#[cfg(feature = "ambition_inventory_ui")]
pub use ambition_inventory_ui as inventory_ui;
#[cfg(feature = "ambition_items")]
pub use ambition_items as items;
#[cfg(feature = "ambition_ldtk_map")]
pub use ambition_ldtk_map as ldtk_map;
pub use ambition_load as load;
pub use ambition_load_presentation as load_presentation;
#[cfg(feature = "ambition_menu")]
pub use ambition_menu as menu;
#[cfg(feature = "ambition_persistence")]
pub use ambition_persistence as persistence;
pub use ambition_platformer_primitives as platformer;
#[cfg(feature = "ambition_portal")]
pub use ambition_portal as portal;
#[cfg(feature = "ambition_portal_presentation")]
pub use ambition_portal_presentation as portal_presentation;
#[cfg(feature = "ambition_projectiles")]
pub use ambition_projectiles as projectiles;
#[cfg(feature = "ambition_render")]
pub use ambition_render as render;
pub use ambition_runtime as runtime;
#[cfg(feature = "ambition_settings_menu")]
pub use ambition_settings_menu as settings_menu;
#[cfg(feature = "ambition_sfx")]
pub use ambition_sfx as sfx;
#[cfg(feature = "ambition_sfx_bank")]
pub use ambition_sfx_bank as sfx_bank;
pub use ambition_sim_view as sim_view;
pub use ambition_sprite_sheet as sprite_sheet;
pub use ambition_time as time;
#[cfg(feature = "ambition_touch_input")]
pub use ambition_touch_input as touch_input;
#[cfg(feature = "ambition_ui_nav")]
pub use ambition_ui_nav as ui_nav;
#[cfg(feature = "ambition_vfx")]
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

    /// **Which seat of the match a body is.**
    ///
    /// The answer to "who is player two?", and the only correct one — the
    /// engine's own docs say every other way to identify a fighter is a guess:
    /// a brain slot misses the CPU seat, the worn character id collides in a
    /// mirror match, and entity order is not an order.
    ///
    /// ⚠ Exposed for blind run 7's finding (g), which is the campaign's own
    /// Smash proof caught short. `RollbackSession::participants()` reports the
    /// count a composition DECLARED; the run declared 1, 2 and 4 through the
    /// public builder and got one body every time, and no consumer could see
    /// the difference. Seats come from DEVICES — a composition with two
    /// gamepads seats two — so a consumer needs this to check that the match
    /// it asked for is the match it got.
    ///
    /// ⚠ **the input half is CLOSED as of 2026-07-31**:
    /// `ambition::sim::drive_seat_frame` drives a named seat, beside
    /// `drive_control_frame` for the primary. The seam had existed in
    /// `ambition_runtime` since queue Y1 and was simply never re-exported — so
    /// the finding described the FACADE, not the engine, which is the more
    /// embarrassing of the two and the harder one to notice.
    pub use ambition_actors::character_runtime::MatchSeat;

    /// **Declaring a MATCH: who is in it, who drives them, and what it costs to
    /// lose.**
    ///
    /// ⚠ Exposed 2026-07-31 because the smash demo could not be written without
    /// them, and what it had to write instead was
    /// `ambition::actors::character_runtime::MatchParticipantRoster` — reaching
    /// through the crate re-export into an implementation module. That is the
    /// leak `minimal-game-names-only-the-public-sdk` exists to catch, and the
    /// only reason it went uncaught is that no consumer had ever declared a
    /// match: the shipped versus stage lives in `ambition_app`, which is allowed
    /// to name anything.
    ///
    /// A second consumer is the only instrument that finds this class, which is
    /// the entire argument for keeping one.
    pub use ambition_actors::character_runtime::{
        ControllerBinding, MatchParticipant, MatchParticipantRoster, RosterProblem,
    };

    /// **The stocks economy**: the count on a body, the fact that it is out, and
    /// the two messages a ruleset acts on.
    ///
    /// The engine owns the COUNT — spend one, decide whether it was the last,
    /// clear the meter of a fighter coming back. It refuses to place the body or
    /// to say what a match ending means, because those need a stage and a
    /// scoreboard. These are the seam between the two halves.
    pub use ambition_combat::components::FighterStocks;
    pub use ambition_combat::stocks::{
        BodyKnockedOut, FighterEliminated, FighterStockSpent, StocksMatchDecided,
    };

    /// How a body came to exist — ADR 0030's construction provenance.
    ///
    /// ⚠ A consumer used to reach this through `ambition::runtime::demo_fixture`,
    /// which is a mirror of an implementation crate wearing a name that says
    /// the opposite of "supported". A module called `demo_fixture` in a shipped
    /// game's imports was the namespace confessing; construction is an ACTOR
    /// concept and this is where a consumer looks for it (LEAK CLOSED, slice F).
    pub use ambition_actors::construction::ActorConstructionRegistry;

    pub use ambition_engine_core::BodyClusterQueryData;
    /// Where the body is, and how it moves.
    pub use ambition_engine_core::movement::{TransitVelocity, transit_body};
    pub use ambition_platformer_primitives::body::BodyKinematics;

    /// What a game spawns and configures.
    pub use ambition_actors::features::{
        ActorConfig, ActorFaction, CharacterRosterFragment, MotionModel, SpawnActorKind,
        SpawnActorRequest,
    };

    /// What a room stages when it opens.
    pub use ambition_actors::features::RoomContentStagingRegistry;

    /// What a body can reach for.
    ///
    /// A capability MASK, not a promise of combat: an actor with an empty
    /// ability set is an ordinary actor, and a game should be able to check
    /// that its cast is not secretly armed.
    pub use ambition_engine_core::abilities::AbilitySet;

    /// The body's INTRINSIC kit as authored, and the EFFECTIVE set the movement
    /// kernel reads. `effective = base ∩ session_mask`, so a game checking what
    /// its cast can actually do wants the base.
    pub use ambition_engine_core::body_clusters::{AbilityBase, BodyAbilities};
}

/// **Characters: the cast, its art, and what it can do.**
///
/// The fourth curated domain module, and the one that absorbs the largest
/// remaining spread. Before this a consumer authoring a single character had to
/// name FOUR mirrored crates — `ambition::characters` for the catalog,
/// `ambition::actors` for its runtime load state, `ambition::sprite_sheet` for
/// what it looks like, and `ambition::entity_catalog` for how it thinks —
/// which is the namespace mirror at its most legible: the cast of one game
/// spread across the engine's internal crate boundaries because those
/// boundaries are what the facade published.
///
/// Closed list.
pub mod character {
    /// Registering a roster archetype (the enemy half of a cast).
    pub use ambition_actors::features::CharacterRosterAppExt;
    /// The cast, as authored content.
    pub use ambition_characters::actor::character_catalog::{
        CharacterCatalog, CharacterCatalogAppExt, CharacterCatalogFragment, parse_catalog,
    };

    /// What providers have authored, for a game that wants to inspect its own
    /// content before a session exists.
    pub use ambition_platformer_provider::PlatformerAuthoredCatalogRegistry;

    /// Declaring a character in Rust, for the cases ADR 0032 keeps in Rust:
    /// tests, procedural generation, unrepresentable schemas, and a cast whose
    /// behavior is supplied by host code as a deliberate authoring choice.
    pub use ambition_actors::character_runtime::{CharacterDefinition, CharacterDefinitionAppExt};
    pub use ambition_characters::actor::WornCharacter;

    /// What a character can do, and how it decides.
    pub use ambition_characters::brain::ActionSet;
    pub use ambition_entity_catalog::placements::CharacterBrain;

    /// What a character looks like, and whether its art has arrived.
    pub use ambition_actors::character_runtime::CharacterLoadStates;
    pub use ambition_actors::character_sprites::sheet_for_declared_character;
    pub use ambition_sprite_sheet::AuthoredSheetAppExt;
    pub use ambition_sprite_sheet::character::CharacterSheetState;
    pub use ambition_sprite_sheet::character::sheets::AuthoredSheets;
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

    /// Simulation time. Not wall time — a game reads the clock the sim advances.
    pub use ambition_time::WorldTime;

    /// One frame of input, and the one seam that delivers it.
    ///
    /// ⚠ `drive_control_frame` lives in `ambition_runtime::rollback` and that is
    /// where a consumer used to reach for it — which meant driving INPUT
    /// required naming the ROLLBACK module even on a fixed-tick host. It is
    /// re-exported here because it is a simulation seam, not a rollback one:
    /// its whole purpose (LEAK CLOSED 2026-07-27) is that a consumer no longer
    /// has to know which host it is on.
    pub use ambition_input::ControlFrame;
    pub use ambition_runtime::rollback::drive_control_frame;

    /// **Drive input to a NAMED seat** — the twin of
    /// [`drive_control_frame`](ambition_runtime::rollback::drive_control_frame),
    /// and the half blind run 7's finding (g) recorded as missing.
    ///
    /// ⚠ the seam has existed in `ambition_runtime::rollback` since queue Y1; it
    /// was never re-exported, so from a consumer's side "no public seam drives
    /// input to a named seat" was true of the SDK while being false of the
    /// engine. That is a worse shape than a missing feature: the capability was
    /// built, tested, and unreachable, and the finding it produced described the
    /// facade rather than the code.
    ///
    /// A driver that needs two independent streams — couch versus, a character
    /// select where four people press their own buttons, a two-peer replay —
    /// writes seat 0 through `drive_control_frame` and every other seat through
    /// this. Slot 0 is REFUSED here rather than silently redirected: it belongs
    /// to the other seam, and a driver that meant the primary seat should say so.
    ///
    /// Under a latching host it folds into that seat's latch, so a sub-tick press
    /// survives exactly as the primary seat's does; without one it writes the
    /// pending seat input directly, which is what a headless or replay driver
    /// wants.
    pub use ambition_characters::brain::PlayerSlot;
    pub use ambition_runtime::rollback::drive_seat_frame;
}

/// **What is drawn, as a game observes it.**
///
/// Deliberately thin. A consumer reads the presented world; it does not own the
/// render path.
pub mod view {
    pub use ambition_platformer_primitives::lifecycle::RoomVisual;

    /// The decoded art the presentation draws from.
    pub use ambition_sprite_sheet::game_assets::GameAssets;

    /// Where the art comes from: every asset path/source policy the
    /// presentation reads.
    pub use ambition_asset_manager::sandbox_assets::{SandboxAssetCatalog, ids};
}

/// **Rollback, as a supported session mode.**
///
/// The six properties ADR 0031 required before this could be a promise, and a
/// test for each — see the module docs.
pub mod rollback;

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
        PlatformerEnginePlugins, SandboxSetsPlugin, SimCoreResourcesPlugin, SimulationHost,
        SimulationHostAppExt, add_headless_foundation, init_engine_states,
    };
}

/// Windowed host plugin groups and host-facing seams.
pub mod windowed_host {
    #[cfg(feature = "input")]
    pub use ambition_host::HostInputBindingsPlugin;
    pub use ambition_host::{HostCameraPlugin, PlatformerHostPlugins};
}

/// Default renderer facade.
#[cfg(feature = "ambition_render")]
pub mod renderer {
    pub use ambition_render::*;
}

/// The generic platformer PRESENTATION face: a camera, the room's static visuals,
/// and the sprite/animation chain. A demo adds this beside the engine and host
/// groups; a game layers its own HUD/menus/dev stack on top (oracle-violation OV1).
#[cfg(feature = "ambition_render")]
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
