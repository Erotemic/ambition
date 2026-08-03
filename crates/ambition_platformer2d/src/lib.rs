//! Public facade for Ambition-derived platformer games.
//!
//! This crate is the E9 umbrella surface: a downstream game should depend on
//! `ambition_platformer2d` plus its own content crate instead of copying the app shell's wall
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
/// `ambition_platformer2d_provider`; re-exported here so provider crates keep
/// the `ambition_platformer2d::provider::…` path.
pub use ambition_platformer2d_provider as provider;

/// **The causal inspector** — "why did this actor change on this tick".
///
/// Behind the `causal` feature, default-OFF, for the reason every instrument in
/// this engine is: a game that never opens one must not link one. Turning it on
/// gives a consumer the whole vocabulary — facts, domains, subjects, the log,
/// `explain` — plus the host plugin, WITHOUT importing an internal-shaped
/// module. That last clause is a stated goal of the program this belongs to: an
/// agent should be able to inspect why a body moved without reading engine
/// source or naming an engine-private path.
///
/// ```ignore
/// use ambition_platformer2d::causal::{CausalRecording, RecordingPolicy, SubjectKey, domains};
///
/// app.add_plugins(ambition_platformer2d::causal::CausalPlugin);
/// app.world_mut()
///     .resource_mut::<CausalRecording>()
///     .set_policy(RecordingPolicy::only([domains::MOVEMENT]));
/// // … drive the sim …
/// println!("{}", log.explain(tick, &SubjectKey::Seat(1)).render());
/// ```
#[cfg(feature = "causal")]
pub mod causal {
    /// The vocabulary: facts, domains, subjects, the log, `explain`.
    pub use ambition_causal::*;
    /// The host half: the plugin, the frame stamp, the publishers' ordering set.
    ///
    /// ⚠ a game installs `CausalPlugin` and gets recording it can TURN ON. It
    /// never turns itself on: an instrument that is on by default is one
    /// somebody switches off, and then it is not there when it is needed.
    pub use ambition_platformer2d_runtime::causal::{
        assert_no_offthread_loss, record_domains, CausalPlugin, RecordingSet,
    };
}

/// **The content compiler** — author, validate, prepare, without a Rust rebuild.
///
/// Behind the `content_pack` feature, default-OFF, because a game that ships its
/// content embedded and never validates at runtime should not link a compiler.
/// Turning it on gives a consumer the whole pipeline — draft, schema registry,
/// diagnostics, prepared pack, fingerprint — plus [`engine_schemas`], WITHOUT
/// naming an engine-private path.
///
/// ```ignore
/// let mut registry = ambition_platformer2d::content::engine_schemas();
/// registry.register(my_capability::my_schema())?;      // a capability's own
/// let pack = ambition_platformer2d::content::compile_dir(path, &registry, &assets)?;
/// ```
#[cfg(feature = "content_pack")]
pub mod content {
    pub use ambition_content_pack::*;

    /// The schemas the ENGINE itself owns, ready for a consumer to add to.
    ///
    /// ⚠ this is the piece a consumer cannot assemble for itself without
    /// knowing which crates own which schemas — which is exactly the internal
    /// topology the SDK is supposed to hide. A capability's own schema is added
    /// on top; nobody has to know that the character catalog lives in
    /// `ambition_characters`.
    pub fn engine_schemas() -> ambition_content_pack::SchemaRegistry {
        let mut registry = ambition_content_pack::SchemaRegistry::new();
        registry
            .register(crate::characters::actor::character_catalog::character_catalog_schema())
            .expect("the engine's own schemas are registered once");
        registry
            .register(
                crate::characters::brain::boss_pattern::content_schema::boss_seed_library_schema(),
            )
            .expect("the engine's own schemas are registered once");
        registry
            .register(
                crate::characters::brain::boss_pattern::content_schema::boss_validator_bands_schema(
                ),
            )
            .expect("the engine's own schemas are registered once");
        registry
            .register(
                crate::characters::brain::boss_pattern::content_schema::boss_profiles_schema(),
            )
            .expect("the engine's own schemas are registered once");
        registry
            .register(ambition_audio::content_schema::music_registry_schema())
            .expect("the engine's own schemas are registered once");
        registry
            .register(ambition_audio::content_schema::sfx_registry_schema())
            .expect("the engine's own schemas are registered once");
        // ⚠ a capability's schema follows the CAPABILITY. `ambition_items` is an
        // optional facade edge (slice H), so a composition without it must not
        // claim to own `item_catalog` — that is what makes "uninstalled
        // capability" a real refusal rather than a hypothetical one.
        #[cfg(feature = "ambition_items")]
        registry
            .register(ambition_items::content_schema::item_catalog_schema())
            .expect("the engine's own schemas are registered once");
        registry
    }
}

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
pub use ambition_entity_catalog as entity_catalog;
pub use ambition_game_shell as game_shell;
pub use ambition_input as input;
pub use ambition_interaction as interaction;
#[cfg(feature = "ambition_inventory_ui")]
pub use ambition_inventory_ui as inventory_ui;
#[cfg(feature = "ambition_items")]
pub use ambition_items as items;
pub use ambition_load as load;
pub use ambition_load_presentation as load_presentation;
#[cfg(feature = "ambition_menu")]
pub use ambition_menu as menu;
#[cfg(feature = "ambition_persistence")]
pub use ambition_persistence as persistence;
pub use ambition_platformer2d_actor_monolith as actors;
pub use ambition_platformer2d_core as engine_core;
pub use ambition_platformer2d_host as host;
#[cfg(feature = "ambition_platformer2d_ldtk")]
pub use ambition_platformer2d_ldtk as ldtk_map;
pub use ambition_platformer2d_runtime as runtime;
pub use ambition_platformer2d_shared_tangle as platformer;
#[cfg(feature = "ambition_portal2d")]
pub use ambition_portal2d as portal;
#[cfg(feature = "ambition_portal2d_presentation")]
pub use ambition_portal2d_presentation as portal_presentation;
#[cfg(feature = "ambition_projectiles")]
pub use ambition_projectiles as projectiles;
#[cfg(feature = "ambition_render")]
pub use ambition_render as render;
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
/// `ambition_platformer2d::engine_core` — an IMPLEMENTATION crate the facade mirrors — for
/// `transit_body`, `TransitVelocity` and `BodyClusterQueryData`, and into
/// `ambition_platformer2d::platformer` for the marker and kinematics types you need to ask
/// "where is the player".
///
/// Closed list, like [`world`]. Adding a type to a mirrored crate does not
/// silently become public API.
pub mod actor {
    /// Who the body is.
    pub use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;

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
    /// `ambition_platformer2d::sim::drive_seat_frame` drives a named seat, beside
    /// `drive_control_frame` for the primary. The seam had existed in
    /// `ambition_platformer2d_runtime` since queue Y1 and was simply never re-exported — so
    /// the finding described the FACADE, not the engine, which is the more
    /// embarrassing of the two and the harder one to notice.
    pub use ambition_platformer2d_actor_monolith::character_runtime::MatchSeat;

    /// **Declaring a MATCH: who is in it, who drives them, and what it costs to
    /// lose.**
    ///
    /// ⚠ Exposed 2026-07-31 because the smash demo could not be written without
    /// them, and what it had to write instead was
    /// `ambition_platformer2d::actors::character_runtime::MatchParticipantRoster` — reaching
    /// through the crate re-export into an implementation module. That is the
    /// leak `minimal-game-names-only-the-public-sdk` exists to catch, and the
    /// only reason it went uncaught is that no consumer had ever declared a
    /// match: the shipped versus stage lives in `ambition_app`, which is allowed
    /// to name anything.
    ///
    /// A second consumer is the only instrument that finds this class, which is
    /// the entire argument for keeping one.
    pub use ambition_platformer2d_actor_monolith::character_runtime::{
        ControllerBinding, MatchParticipant, MatchParticipantRoster, MatchSeatingRefused,
        RosterProblem,
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
    /// ⚠ A consumer used to reach this through `ambition_platformer2d::runtime::demo_fixture`,
    /// which is a mirror of an implementation crate wearing a name that says
    /// the opposite of "supported". A module called `demo_fixture` in a shipped
    /// game's imports was the namespace confessing; construction is an ACTOR
    /// concept and this is where a consumer looks for it (LEAK CLOSED, slice F).
    pub use ambition_platformer2d_actor_monolith::construction::ActorConstructionRegistry;

    /// Where the body is, and how it moves.
    pub use ambition_platformer2d_core::movement::{transit_body, TransitVelocity};
    pub use ambition_platformer2d_core::BodyClusterQueryData;
    pub use ambition_platformer2d_shared_tangle::body::BodyKinematics;

    /// What a game spawns and configures.
    pub use ambition_platformer2d_actor_monolith::features::{
        ActorConfig, ActorFaction, CharacterRosterFragment, MotionModel, SpawnActorKind,
        SpawnActorRequest,
    };

    /// What a room stages when it opens.
    pub use ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry;

    /// What a body can reach for.
    ///
    /// A capability MASK, not a promise of combat: an actor with an empty
    /// ability set is an ordinary actor, and a game should be able to check
    /// that its cast is not secretly armed.
    pub use ambition_platformer2d_core::abilities::AbilitySet;

    /// The body's INTRINSIC kit as authored, and the EFFECTIVE set the movement
    /// kernel reads. `effective = base ∩ session_mask`, so a game checking what
    /// its cast can actually do wants the base.
    pub use ambition_platformer2d_core::body_clusters::{AbilityBase, BodyAbilities};
}

/// **Characters: the cast, its art, and what it can do.**
///
/// The fourth curated domain module, and the one that absorbs the largest
/// remaining spread. Before this a consumer authoring a single character had to
/// name FOUR mirrored crates — `ambition_platformer2d::characters` for the catalog,
/// `ambition_platformer2d::actors` for its runtime load state, `ambition_platformer2d::sprite_sheet` for
/// what it looks like, and `ambition_platformer2d::entity_catalog` for how it thinks —
/// which is the namespace mirror at its most legible: the cast of one game
/// spread across the engine's internal crate boundaries because those
/// boundaries are what the facade published.
///
/// Closed list.
pub mod character {
    /// The cast, as authored content.
    pub use ambition_characters::actor::character_catalog::{
        parse_catalog, CharacterCatalog, CharacterCatalogAppExt, CharacterCatalogFragment,
    };
    /// Registering a roster archetype (the enemy half of a cast).
    pub use ambition_platformer2d_actor_monolith::features::CharacterRosterAppExt;

    /// What providers have authored, for a game that wants to inspect its own
    /// content before a session exists.
    pub use ambition_platformer2d_provider::PlatformerAuthoredCatalogRegistry;

    pub use ambition_characters::actor::WornCharacter;
    /// Declaring a character in Rust, for the cases ADR 0032 keeps in Rust:
    /// tests, procedural generation, unrepresentable schemas, and a cast whose
    /// behavior is supplied by host code as a deliberate authoring choice.
    pub use ambition_platformer2d_actor_monolith::character_runtime::{
        CharacterDefinition, CharacterDefinitionAppExt,
    };

    /// What a character can do, and how it decides.
    pub use ambition_characters::brain::ActionSet;
    pub use ambition_entity_catalog::placements::CharacterBrain;

    /// What a character looks like, and whether its art has arrived.
    pub use ambition_platformer2d_actor_monolith::character_runtime::CharacterLoadStates;
    pub use ambition_platformer2d_actor_monolith::character_sprites::sheet_for_declared_character;
    pub use ambition_sprite_sheet::character::sheets::AuthoredSheets;
    pub use ambition_sprite_sheet::character::CharacterSheetState;
    pub use ambition_sprite_sheet::AuthoredSheetAppExt;
}

/// **The simulation schedule a game joins its own systems to.**
///
/// A game never names a literal Bevy schedule: it asks for the sim schedule and
/// a semantic set, so the same system runs under the fixed tick and a GGRS host
/// alike. That indirection is the engine's rule, and before this the only way
/// to reach it was `ambition_platformer2d::platformer::schedule` — the crate mirror.
pub mod sim {
    pub use ambition_platformer2d_shared_tangle::schedule::{
        GameMode, Platformer2dSimulationPhaseMonolith, SimSchedule, SimScheduleExt,
    };

    /// Simulation time. Not wall time — a game reads the clock the sim advances.
    pub use ambition_time::WorldTime;

    /// One frame of input, and the one seam that delivers it.
    ///
    /// ⚠ `drive_control_frame` lives in `ambition_platformer2d_runtime::rollback` and that is
    /// where a consumer used to reach for it — which meant driving INPUT
    /// required naming the ROLLBACK module even on a fixed-tick host. It is
    /// re-exported here because it is a simulation seam, not a rollback one:
    /// its whole purpose (LEAK CLOSED 2026-07-27) is that a consumer no longer
    /// has to know which host it is on.
    pub use ambition_input::ControlFrame;
    pub use ambition_platformer2d_runtime::rollback::drive_control_frame;

    /// **Drive input to a NAMED seat** — the twin of
    /// [`drive_control_frame`](ambition_platformer2d_runtime::rollback::drive_control_frame),
    /// and the half blind run 7's finding (g) recorded as missing.
    ///
    /// ⚠ the seam has existed in `ambition_platformer2d_runtime::rollback` since queue Y1; it
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
    pub use ambition_platformer2d_runtime::rollback::drive_seat_frame;
}

/// **What is drawn, as a game observes it.**
///
/// Deliberately thin. A consumer reads the presented world; it does not own the
/// render path.
pub mod view {
    pub use ambition_platformer2d_shared_tangle::lifecycle::RoomVisual;

    /// The decoded art the presentation draws from.
    pub use ambition_sprite_sheet::game_assets::GameAssets;

    /// Where the art comes from: every asset path/source policy the
    /// presentation reads.
    pub use ambition_asset_manager::platformer_assets::{ids, Platformer2dAssetCatalog};

    /// The marker on a generated background layer.
    ///
    /// Exported so a consumer can ask whether its backdrop is DRAWN and whether
    /// it MOVES — two questions `fixtures/external_consumer` now asks, and could
    /// not ask at all while the component sat behind a private module. It went
    /// into `ambition_platformer2d::renderer` first, which is the raw crate re-export, and
    /// `outlander-names-only-the-public-sdk` caught that immediately: a third
    /// party reaching through the facade into an implementation module is the
    /// leak ADR 0031 exists to close, and "the test needed it" is how those get
    /// made.
    #[cfg(feature = "ambition_render")]
    pub use ambition_render::rendering::ParallaxLayerVisual;
}

/// **Rollback, as a supported session mode.**
///
/// The six properties ADR 0031 required before this could be a promise, and a
/// test for each — see the module docs.
pub mod rollback;

/// **The authored world: rooms, geometry, placements, collision.**
///
/// ⚠ A CURATED MODULE, not a crate mirror — and the difference is the whole
/// point of ADR 0031. `pub use ambition_platformer2d_world as world` made the compatibility
/// surface change whenever the crate did: a new submodule became public API by
/// existing. This list is CLOSED, so adding one to `ambition_platformer2d_world` is an
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
    pub use ambition_platformer2d_world::prelude;

    pub use ambition_platformer2d_world::{collision, debug_label, placements, platforms, rooms};
}
// Re-exported so a game can name bevy TYPES through `ambition_platformer2d::bevy::…`. NOTE:
// this does NOT let a crate `#[derive(Component)]`/`#[derive(Resource)]` through
// the umbrella alone — bevy's derive macros resolve `::bevy_ecs` via the
// CONSUMER's own Cargo.toml (`BevyManifest`), which a re-export does not satisfy.
// A content crate that defines its own components/resources must ALSO list `bevy`
// in its manifest (one line, version pinned by the workspace). See
// docs/planning/demos/README.md.
pub use bevy;

/// Engine assembly helpers most games need first.
pub mod engine {
    pub use ambition_platformer2d_runtime::{
        add_headless_foundation, init_engine_states, Platformer2dSimulationFoundationPlugin,
        PlatformerEnginePlugins, SimCoreResourcesPlugin, SimulationHost, SimulationHostAppExt,
    };
}

/// Windowed host plugin groups and host-facing seams.
pub mod windowed_host {
    #[cfg(feature = "input")]
    pub use ambition_platformer2d_host::HostInputBindingsPlugin;
    pub use ambition_platformer2d_host::{HostCameraPlugin, PlatformerHostPlugins};
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
    /// [`with_presentation_profiles`](ambition_platformer2d_provider::PlatformerExperienceAuthoring::with_presentation_profiles).
    pub use ambition_platformer2d_shared_tangle::gameplay_presentation;
    pub use ambition_platformer2d_shared_tangle::gameplay_presentation::profiles;

    /// The DECLARED-HUD seam: a game declares its readout slots with
    /// [`with_hud`](ambition_platformer2d_provider::PlatformerExperienceAuthoring::with_hud)
    /// and publishes their live values into [`HudReadouts`] each frame. The
    /// engine holds no readout vocabulary — every label is a string the game
    /// writes.
    pub use ambition_platformer2d_shared_tangle::gameplay_presentation::{
        ActiveHudDeclaration, HudDeclaration, HudReadout, HudReadouts, HudSlotId, HudSlotSpec,
        SurroundRegion,
    };
    pub use ambition_render::hud::declared::{DeclaredHudPlugin, DeclaredHudRoot, DeclaredHudSlot};
}

/// **The SDK claim about the inspector, tested.**
///
/// The program this belongs to says an agent should be able to *"inspect why the
/// resulting actor moved, attacked, took damage, or changed lifecycle state …
/// without importing internal-shaped engine modules or reading engine source"*.
///
/// So this module uses ONLY `ambition_platformer2d::causal::…` — no `ambition_causal`, no
/// `ambition_platformer2d_runtime`, no `ambition_platformer2d_actor_monolith`. If a consumer would have to reach
/// past the facade to do this, that shows up here as a compile error rather
/// than as a paragraph somebody has to believe.
#[cfg(all(test, feature = "causal"))]
mod causal_sdk_tests {
    use crate::causal::{
        domains, record_domains, CausalFact, CausalPlugin, CausalRecording, FactDetail,
        RecordingPolicy, RecordingSet, SubjectKey,
    };
    use bevy::prelude::*;

    /// A game's own publisher, written the way a consumer would write one.
    fn a_game_publishes_something(mut log: ResMut<CausalRecording>) {
        log.record(
            CausalFact::new(
                domains::MOVEMENT,
                0,
                FactDetail::new("my_game_fact", "the game said something"),
            )
            .about(SubjectKey::Seat(0))
            .field("value", 7_i64),
        );
    }

    #[test]
    fn a_consumer_inspects_a_tick_through_the_facade_alone() {
        let mut app = App::new();
        app.add_plugins(CausalPlugin);
        app.insert_resource(crate::time::SimTick(11));
        record_domains(&mut app, RecordingPolicy::All);
        app.add_systems(
            Update,
            a_game_publishes_something.in_set(RecordingSet::Publish),
        );
        app.update();

        let why = app
            .world()
            .resource::<CausalRecording>()
            .explain(11, &SubjectKey::Seat(0));
        assert_eq!(
            why.first("my_game_fact").and_then(|f| f.get("value")),
            Some(&crate::causal::FactValue::Int(7)),
            "a game's own fact comes back out of the facade's own explainer"
        );
        assert!(
            why.render().contains("seat:0"),
            "and renders for a human: {}",
            why.render()
        );
    }

    /// ⚠ **the tick is the HOST's**, even for a consumer's own fact. A game that
    /// had to stamp its own would be a second clock nothing could join against.
    #[test]
    fn a_consumers_fact_carries_the_hosts_tick_without_being_told_it() {
        let mut app = App::new();
        app.add_plugins(CausalPlugin);
        app.insert_resource(crate::time::SimTick(0));
        record_domains(&mut app, RecordingPolicy::All);
        app.add_systems(
            Update,
            a_game_publishes_something.in_set(RecordingSet::Publish),
        );

        app.world_mut().resource_mut::<crate::time::SimTick>().0 = 40;
        app.update();
        app.world_mut().resource_mut::<crate::time::SimTick>().0 = 41;
        app.update();

        let log = app.world().resource::<CausalRecording>();
        assert_eq!(log.explain(40, &SubjectKey::Seat(0)).facts().len(), 1);
        assert_eq!(log.explain(41, &SubjectKey::Seat(0)).facts().len(), 1);
        assert!(
            log.explain(42, &SubjectKey::Seat(0)).is_empty(),
            "and nothing lands on a tick that never ran"
        );
    }
}

/// **The SDK claim about authoring, tested.**
///
/// The program says an agent should *"add or modify a character and validate it
/// without rebuilding Rust"* and do so *"without importing internal-shaped
/// engine modules"*. This module uses ONLY `ambition_platformer2d::content::…` — no
/// `ambition_content_pack`, no `ambition_characters`. A consumer forced past the
/// facade shows up here as a compile error.
#[cfg(all(test, feature = "content_pack"))]
mod content_sdk_tests {
    use crate::content::{
        compile, engine_schemas, AssetsUnchecked, CompileStage, ContentPackDraft,
        ContentPackManifest, DiagnosticCode, ModuleNamespace, PackId, PackVersion, SchemaId,
        SchemaVersion, SourceDeclaration,
    };

    const CATALOG: &str = r#"(
        brain_presets: { "stand_still": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "newcomer": (
                display_name: "Newcomer",
                spritesheet: "newcomer.png",
                manifest: "newcomer.ron",
                tier: MainHall,
                body_kind: Standard,
                composition: None,
                default_brain: "stand_still",
                default_action_set: "peaceful",
                tags: [],
            ),
        },
    )"#;

    fn pack(name: &str, catalog: &str) -> ContentPackDraft {
        let root = std::env::temp_dir().join(format!("ambition_content_sdk/{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp pack");
        std::fs::write(root.join("cast.ron"), catalog).expect("write");
        ContentPackDraft::read_manifest(
            root,
            ContentPackManifest {
                id: PackId("my_game".into()),
                version: PackVersion("1.0.0".into()),
                namespace: ModuleNamespace("my_game".into()),
                requires: Vec::new(),
                sources: vec![SourceDeclaration {
                    path: "cast.ron".into(),
                    schema: SchemaId::new("character_catalog"),
                    version: SchemaVersion(1),
                }],
            },
        )
        .expect("draft reads")
    }

    /// A consumer composes the engine's schemas and validates its own cast.
    #[test]
    fn a_consumer_validates_its_own_content_through_the_facade_alone() {
        let compiled = compile(&pack("valid", CATALOG), &engine_schemas(), &AssetsUnchecked)
            .expect("a well-formed cast compiles");
        assert_eq!(compiled.namespace.0, "my_game");
        assert!(
            compiled
                .get(&SchemaId::new("character"), "newcomer")
                .is_some(),
            "the consumer's own character is a prepared identity in its OWN namespace"
        );
    }

    /// ⚠ and the refusals are the facade's too — a consumer does not have to
    /// reach into the compiler crate to learn WHY its content was rejected.
    #[test]
    fn a_typo_is_refused_through_the_facade_with_a_code_a_tool_can_branch_on() {
        let failure = compile(
            &pack(
                "typo",
                &CATALOG.replace(
                    r#"default_brain: "stand_still""#,
                    r#"default_brain: "stand_stil""#,
                ),
            ),
            &engine_schemas(),
            &AssetsUnchecked,
        )
        .expect_err("a preset typo must refuse");
        assert_eq!(failure.stage, CompileStage::ReferenceResolution);
        assert!(failure.has(DiagnosticCode::UnknownPreset));
        assert!(
            failure.render().contains("did you mean `stand_still`?"),
            "and it answers the typo: {}",
            failure.render()
        );
    }
}
