// ⛔ **NOT `#[cfg(feature = "ui")]`.** It was, while its three use sites were
// not — so `--no-default-features --features web` had the uses without the
// import and the web build did not compile. `Platformer2dStartupAssets` is an
// asset collection and has nothing to do with the UI feature; the gate was on
// the wrong half of the pair. Found 2026-08-03 by running the web check, which
// only the exhaustive plan runs — the same way this target sat broken for four
// days once before.
use ambition_platformer2d::actors::assets::loading;
use ambition_platformer2d::actors::rooms;
use ambition_platformer2d::actors::time::feel::Platformer2dFeelTuningMonolith;
#[cfg(feature = "physics_debris")]
use ambition_platformer2d::actors::world::physics;
#[cfg(feature = "physics_debris")]
use ambition_platformer2d::actors::world::physics::physics_spawn_debris_messages;
use ambition_platformer2d::dev_tools::dev_tools::{
    DeveloperTools, EditableAbilitySet, EditableMovementTuning, EditablePlayerStats,
    MovementProfile, PlayerBodyProfile,
};
use ambition_platformer2d::inventory_ui;
use ambition_platformer2d::ldtk_map as ldtk_world;
use ambition_platformer2d::platformer::schedule::{
    gameplay_allowed, Platformer2dSimulationPhaseMonolith, PresentationSetupSet, SimScheduleExt,
};
// The rest of `fx` moved to `HostVfxPresentationPlugin` (see
// `install_projectile_and_vfx_systems`); the blink preview ring is the one
// pass still registered here, and only under the `input` persona.
#[cfg(feature = "input")]
use ambition_platformer2d::render::fx;
use ambition_platformer2d::render::rendering::{camera_follow, sync_visuals};
use ambition_platformer2d::render::ui_fonts;
use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollectionApp;
use bevy_ecs_ldtk::prelude::LdtkPlugin;

use crate::dev::debug_overlay;
use crate::host::windowing;

use super::dev_runtime::{
    handle_debug_hotkeys, handle_ldtk_hot_reload, restart_local_ggrs_after_hot_reload,
};
use super::hud::{update_hud, update_quest_panel};
use super::player_tick::sync_player_presentation;
use super::resources::init_sandbox_resources;
use super::setup_systems::{
    reload_visual_quality_assets_on_scale_change, setup_host_presentation_system,
    setup_simulation_system,
};
use super::sim_systems::apply_player_reset_input_system;

/// Register core simulation plugins, message types, and the gameplay
/// schedule. Headless and visible both call this.
///
/// The body is split into per-set helpers below so each section is short
/// enough to read in one screen and stays under Bevy's 20-system tuple
/// arity limit. New simulation systems should go into the matching
/// `register_*_systems` helper rather than back into this orchestrator.
pub fn add_simulation_plugins(app: &mut App) {
    app.add_message::<ambition_platformer2d::platformer::developer_hotkeys::DeveloperAction>();
    // AmbitionPhysicsPlugin (Avian2D) is intentionally NOT here. Per
    // ADR 0007 Avian is secondary physics for debris/ragdoll visuals;
    // the player controller is custom via parry2d in ambition_platformer2d::engine_core.
    // Avian's collider backend needs `SceneSpawner` (from ScenePlugin in
    // DefaultPlugins), which headless doesn't have. Until Avian's debris
    // role is migrated to presentation events end-to-end (or Avian gains
    // a headless-friendly init path), it lives in
    // `add_presentation_plugins`.

    // Room transitions are load transactions in both direct-entry and shell
    // hosts. Install the contributor-neutral coordinator at the simulation
    // boundary; shell composition later adds only its route adapter and
    // presentation. The plugin is idempotent, so this needs no order guard.
    app.add_plugins(ambition_platformer2d::load::AmbitionLoadPlugin);
    // `RoomTransitionLoadState` / `RoomTransitionContentEpoch` are the engine's
    // now — `RoomTransitionComposerPlugin` owns them.

    // The canonical simulation-phase sets + engine resources now live in
    // `ambition_platformer2d::runtime::Platformer2dSimulationFoundationPlugin` (first in the engine group below).
    // Host configuration overrides are consumed before simulation plugins
    // build. Live gameplay-world values are already components on the exact
    // direct/session root; no canonical world value is initialized as a resource.

    // The construction-time host must be chosen before the first content/sim
    // plugin. Missing means the lightweight render-frame host.
    let simulation_host = app
        .world()
        .get_resource::<ambition_platformer2d::runtime::SimulationHost>()
        .copied()
        .unwrap_or_default();

    app.add_plugins(super::sim_resources::AmbitionGameSimulationSetupPlugin);

    // Named Ambition game content: quests, bosses, dialogue/cutscenes, intro
    // hooks, and portal adapters. Installed after simulation resources so content
    // registries land at the expected assembly point.
    app.add_plugins(ambition_content::AmbitionContentPlugin);

    // Yarn dialogue stack: compile `.yarn`, bridge runner events into sandbox
    // state, and register the commands / functions / markup used by content.
    #[cfg(feature = "ui")]
    {
        app.add_plugins(ambition_content::dialogue::yarn_spinner_plugin());
        app.add_plugins(ambition_platformer2d::actors::dialog::YarnBridgePlugin);
        app.add_plugins(ambition_platformer2d::actors::dialog::YarnBindingsPlugin);
    }

    // The content-free engine SIMULATION plugins (E5): the SAME
    // `PlatformerEnginePlugins` group a demo app builds on — the sandbox sets
    // + engine resources, the sim schedule, the universal brain, gravity,
    // traversal abilities, item pickups, encounters/cutscenes, feature
    // collection/interaction/effects/view-sync, room reset, traces,
    // affordances, and the combat-phase chain. Ordering is set-based, so
    // group membership does not change the resolved schedule.
    app.add_plugins(ambition_platformer2d::runtime::PlatformerEnginePlugins::new(simulation_host));

    // App-LOCAL residue the E5 step-5 carve deliberately left behind. The
    // engine group above registers the shared per-frame wiring (player input
    // chain, brains, possession, room-transition detect/reset, portal
    // schedule, progression); these systems wrap app-only concerns
    // (`reset_sandbox`, `load_room` + render spawns, the player clone) and
    // pin themselves into the documented ordering SLOTS between engine
    // systems (see `ambition_platformer2d::runtime::PlayerSchedulePlugin` /
    // `RoomTransitionSchedulePlugin` module docs).
    register_app_local_sim_systems(app);

    // All construction/snapshot registries are now installed. Publish the
    // direct-entry session root from the same immutable prepared-content path
    // used by shell activation.
    // PROBE-K2B-EDIT2: deleted
}

/// The app-LOCAL per-frame systems, pinned into the ordering SLOTS the engine
/// chains leave for the host (E5 step 5). Everything engine-generic that used
/// to be registered here lives in `ambition_platformer2d::runtime::{PlayerSchedulePlugin,
/// RoomTransitionSchedulePlugin, PortalSchedulePlugin,
/// ProgressionSchedulePlugin}`.
fn register_app_local_sim_systems(app: &mut App) {
    let sim = app.sim_schedule();
    // ── The PlayerInput gap: Ambition's reset-INPUT consumer ──────────────
    //
    // Ambition's own "the player pressed Reset" path, slotted after the
    // dev-edit sync and before the input timer (the exact position it held in
    // the old inline chain). It stays app-side because the button binding is
    // Ambition's; the reset it performs is the engine's `reset_sandbox`.
    //
    // Its former chain partner, the `RoomReplayRequested` consumer, is now
    // engine-side in `RoomReplaySchedulePlugin` (tracks §2.5), along with the
    // two content slot edges the app used to anchor. Keep the explicit
    // ordering edge to it: both systems reset the same body, so without one
    // they are ambiguous, and this preserves the exact order they ran in when
    // they were chained here.
    app.add_systems(
        sim,
        apply_player_reset_input_system
            .run_if(gameplay_allowed)
            .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput)
            .after(ambition_platformer2d::dev_tools::DevEditApplySet)
            .before(ambition_platformer2d::actors::control::InputTimersAdvanced)
            .before(ambition_platformer2d::runtime::RoomReplayApplied),
    );

    // ── Brain-driven player clone (press K) ────────────────────────────────
    //
    // A `PlayerEntity` body driven by a PlayerDemo brain through the SAME
    // shared player systems as the human player. Spawn lands in `WorldPrep`
    // (the earliest set) so the new body exists before the PlayerInput/
    // PlayerSimulation phases pick it up the same frame; the brain tick
    // produces its `ActorControl` in `PlayerInput` (before the control phase
    // consumes it); the transform sync runs in `PresentationSync` after the
    // shared simulation has moved it.
    // **Ambition's own death rules** (ADR 0033). Exploration's answer: no
    // interlude, and the room goes back when nobody is left in play. That is
    // exactly what `apply_home_reset_policy` used to do by reflex — a pit fall
    // returns you to spawn with the room's features restored — except that it is
    // now stated rather than assumed, it happens ONCE rather than on every frame
    // the gate re-fires, and it travels through the same road every other host
    // uses.
    app.insert_resource(
        ambition_platformer2d::combat::death_rules::DeathRules::replay_level_after(0.0),
    );
    app.init_resource::<crate::app::player_clone::PlayerCloneClock>()
        .init_resource::<crate::app::player_clone::SpawnPlayerCloneRequest>()
        .add_systems(
            sim,
            (
                crate::app::player_clone::request_player_clone_on_key,
                crate::app::player_clone::spawn_requested_player_clone,
            )
                .chain()
                .in_set(Platformer2dSimulationPhaseMonolith::WorldPrep),
        )
        .add_systems(
            sim,
            crate::app::player_clone::tick_player_clone_brains
                .run_if(gameplay_allowed)
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput),
        )
        .add_systems(
            sim,
            crate::app::player_clone::sync_player_clone_transform
                .in_set(Platformer2dSimulationPhaseMonolith::PresentationSync),
        )
        .add_systems(
            sim,
            crate::app::player_clone::despawn_player_clones_on_reset
                .in_set(Platformer2dSimulationPhaseMonolith::ResetProcessing)
                // AFTER, not before: the processor is the only system that may
                // decline a reset, and this one waits for its commitment.
                .after(ambition_platformer2d::actors::session::reset::NewGameResetDecided),
        );

    // ── The PlayerSimulation gap: home presentation ───────────────────────
    //
    // Slotted between the possession release and the hit-event drain (the
    // exact position it held in the old inline chain).
    //
    // ⛔ **`apply_home_reset_policy` is GONE** (ADR 0033), and this app is why
    // the defect it caused was invisible: it lived HERE and not in the demo
    // binaries, so the hosted app and the standalone Mary-O disagreed about what
    // a pit death does. It turned every frame of `PlayerBodyFrameOutput.reset`
    // into a `ResetRoomFeaturesEvent { PlayerDeath }`, and a death beat that
    // pinned the body outside the world re-flagged that gate for its whole
    // dwell — 192 full room resets per death, while the death music played.
    // What it did is now a CONSEQUENCE the game authors through `DeathRules`,
    // delivered once through the one shared road (`RoomReplayRequested`).
    app.add_systems(
        sim,
        (
            // HOME PRESENTATION — screen shake + landing SFX + the per-op
            // anim/SFX/VFX — reads the movement phase's hand-off. Moves no body.
            sync_player_presentation.run_if(gameplay_allowed),
        )
            .chain()
            // The engine's named slot for exactly this: after control has
            // settled, before the frame's damage lands. It used to be two leaf
            // names a host had to trust stayed true — the position was documented
            // in the engine's module docs and enforced nowhere.
            .in_set(
                ambition_platformer2d::platformer::schedule::PlayerSimulationSet::PostPossession,
            ),
    );

    // The RoomTransition gap — readiness transaction + authorized commit — is
    // filled by the ENGINE now (`ambition_platformer2d::runtime::room_transition`), carried
    // by `PlatformerEnginePlugins` so a demo host gets it too. This app keeps
    // only its two optional CONTRIBUTORS: the asset manifest/readiness half
    // (`world_flow::room_transition_assets`) and the cover
    // (`install_room_transition_presentation`).

    // The player avatar is carried across rooms, not staged as room content, so
    // the room asset barrier never materializes its (deferred) worn sheet. Keep
    // the primary player's current form decoded — its starting id AND any runtime
    // power-form swap — so a content game's hero draws its real sprite instead of
    // the colored-rectangle fallback. Resource-guarded: a no-op in headless.
}

/// Register Bevy's `LdtkPlugin` plus the supporting Ambition glue
/// (entity registrations, asset collection, LdtkWorldBundle spawn,
/// level-set sync, asset handle preload). Visible binary only —
/// `LdtkPlugin` panics in headless because its tile pipeline expects a
/// `RenderApp` sub-app, and `asset_server.load::<LdtkProject>` requires
/// the LDtk asset type to be registered.
///
/// Once the LDtk runtime-spine roadmap finishes promoting LDtk entity
/// categories to direct Ambition components (see
/// `project_ldtk_roadmap` memory), this dependency goes away and
/// headless can spawn the same entity set without bevy_ecs_ldtk's
/// rendering machinery.
pub fn add_ldtk_runtime_plugin(app: &mut App) {
    // `Platformer2dStartupAssets` includes a typed LDtk handle, so the LDtk
    // asset type and loader must be initialized before bevy_asset_loader
    // allocates collection handles. Keep this before `init_collection`.
    app.add_plugins(LdtkPlugin)
        .init_collection::<loading::Platformer2dStartupAssets>()
        .add_plugins(ldtk_world::AmbitionLdtkRegistrationPlugin)
        .add_systems(
            Startup,
            // ⭐ **K2b edit 3: the direct-entry world-spine spawn is GONE.**
            // It ran only when `AmbitionShellHosted` was absent, and every
            // composition inserts it now — so it was dead code that looked
            // live. The shell spawns SESSION-scoped roots per activation
            // (`spawn_ldtk_world_roots_scoped`), which is the only path.
            ldtk_world::load_ldtk_asset_handle,
        )
        .add_systems(
            Update,
            (
                ldtk_world::sync_ldtk_level_set,
                // ADR 0015 §Coordinate-frame reconciliation — keep the
                // LdtkWorldBundle's root transform aligned with the
                // current active area's centered frame. Runs every
                // gameplay frame; cheap and idempotent.
                ldtk_world::sync_ldtk_world_transform,
            )
                .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
        );
}

/// The LdtkWorldBundle spawn shared by direct startup (`UNSCOPED`,
/// process-resident) and the shell host's per-session activation (scoped, so
/// the session sweep retires the visual spine roots with the session).
pub(crate) fn spawn_ldtk_world_roots_scoped(
    commands: &mut Commands,
    scope: ambition_platformer2d::platformer::lifecycle::SessionSpawnScope,
    asset_server: &AssetServer,
    ldtk_index: &ldtk_world::LdtkRuntimeIndex,
    room_set: &rooms::RoomSet,
    world_assets: Option<&ldtk_world::LdtkWorldAssets>,
    sandbox_asset_collection: Option<&loading::Platformer2dStartupAssets>,
    manifest: &ldtk_world::WorldManifest,
) {
    // One LdtkWorldBundle per prepared WorldManifest row. bevy_ecs_ldtk's
    // asset loader is per-file; Ambition's merged JSON loader doesn't
    // propagate into the Bevy asset system, so each .ldtk file needs its
    // own bundle to get its painted tile layers rendered. The shared sync
    // system writes the same LevelSet to every bundle; only the bundle
    // whose loaded asset contains the active level iids spawns any levels
    // (iids are unique per file).
    let initial_level_set = ldtk_index.level_set_for(&room_set.active_spec().id);
    for (index, source) in manifest.worlds.iter().enumerate() {
        let handle = world_assets
            .and_then(|assets| assets.0.get(index).cloned())
            .or_else(|| {
                // Web loading-state preload covers the primary world only.
                (index == 0)
                    .then(|| {
                        sandbox_asset_collection.map(|collection| collection.ldtk_project.clone())
                    })
                    .flatten()
            })
            .unwrap_or_else(|| asset_server.load(ldtk_world::world_bevy_asset_path(source)));
        let mut root = commands.spawn((
            bevy_ecs_ldtk::prelude::LdtkWorldBundle {
                ldtk_handle: handle.into(),
                level_set: initial_level_set.clone(),
                // AMBITION_REVIEW(spatial): migrate each registered marker from
                // adapter-driven semantics to direct Ambition components.
                ..default()
            },
            ldtk_world::LdtkWorldRoot,
            Name::new(format!("LDtk Runtime Spine Root ({})", source.id)),
        ));
        scope.apply_to(&mut root);
    }
}

/// Register presentation-side plugins (input, dialogue, inspector, audio
/// and VFX subscribers, HUD, debug overlays). Visible binary only.
pub fn add_presentation_plugins(app: &mut App) {
    // Generic load presentation is a presentation-tier service, not a shell
    // service. Install it for direct entry, shell-hosted play, and no-window
    // presentation harnesses alike; the shell contributes only its adapter.
    if !app.is_plugin_added::<ambition_platformer2d::load_presentation::AmbitionLoadPresentationPlugin>() {
        app.add_plugins(ambition_platformer2d::load_presentation::MinimalLoadPresentationPlugins);
    }
    super::world_flow::install_room_transition_presentation(app);
    // The windowed-host face (E5 step 5): leafwing input bindings + the
    // camera follow/shake cluster (+ portal camera continuity). The SAME
    // group a windowed demo adds; the app-local presentation below layers
    // Ambition's HUD/menu/dev stack on top.
    app.add_plugins(ambition_platformer2d::host::PlatformerHostPlugins);
    install_presentation_resources_and_subplugins(app);
    app.add_plugins((
        ambition_platformer2d::persistence::PersistenceSchedulePlugin,
        ambition_platformer2d::dev_tools::DeveloperPersistenceSchedulePlugin,
    ));
    install_menu_setup_and_hotkeys(app);
    app.add_plugins(ambition_platformer2d::render::rendering::PresentationVisualAnimationPlugin);
    // Ambition's named presentation passes (puppy-slug deep-dream) compose onto
    // the renderer's public `ActorOverlaySet` seam the plugin above positions.
    app.add_plugins(ambition_content::presentation::AmbitionPresentationPlugin);
    install_camera_and_debug_overlay_systems(app);
    app.add_plugins(ambition_platformer2d::render::rendering::ActorNameplatePresentationPlugin);
    install_fx_and_hud_systems(app);
    install_misc_visual_sync_systems(app);
    app.add_plugins(ambition_platformer2d::render::rendering::PlayerVisualSchedulePlugin);
    install_projectile_and_vfx_systems(app);
}

/// Visible-side resources, registered types, and presentation child
/// plugins (input, audio, dev_tools, physics_debris, ui, mobile touch,
/// FPS overlay, font loader).
fn install_presentation_resources_and_subplugins(app: &mut App) {
    app.insert_resource(ClearColor(Color::srgb(0.020, 0.024, 0.035)))
        .init_resource::<ambition_platformer2d::render::asset_census::ImageCensus>()
        .insert_resource(windowing::DisplayModeState::default())
        .register_type::<DeveloperTools>()
        .register_type::<PlayerBodyProfile>()
        .register_type::<MovementProfile>()
        .register_type::<EditableAbilitySet>()
        .register_type::<EditableMovementTuning>()
        .register_type::<EditablePlayerStats>()
        .register_type::<Platformer2dFeelTuningMonolith>()
        .register_type::<ambition_platformer2d::portal::PortalConvention>()
        .register_type::<ambition_platformer2d::portal::PortalTuning>();

    #[cfg(feature = "portal_render")]
    app.register_type::<ambition_platformer2d::portal_presentation::PortalVisualEffect>()
        .register_type::<ambition_platformer2d::portal_presentation::PortalEffectSelection>()
        .register_type::<ambition_platformer2d::portal_presentation::PortalCameraTransitMode>()
        .register_type::<ambition_platformer2d::portal_presentation::PortalCameraContinuitySelection>()
        .register_type::<ambition_platformer2d::portal_presentation::PortalCameraContinuityConfig>()
        .register_type::<ambition_platformer2d::portal_presentation::PortalCameraContinuityState>()
        .register_type::<ambition_platformer2d::portal_presentation::PortalViewConeConfig>();

    app.add_plugins(crate::host::platform::PlatformPlugin);
    app.add_plugins(ambition_platformer2d::render::screen_effects::ScreenEffectsPlugin);
    // Loads baked `*_spritesheet.ron` manifests for runtime sheet metadata.
    app.add_plugins(ambition_platformer2d::sprite_sheet::SheetRegistryPlugin);
    // ⭐ **THE HALF THE SHEET CRATE CANNOT DO** (ledger D36). It records every
    // target a later manifest took from an earlier one with a different frame
    // grid, and reports the count once; deciding which of those MATTER needs to
    // know which targets something resolves art by, and that is a catalog fact.
    // This is the caller that owns both, so the filter lives here.
    app.add_systems(bevy::prelude::Startup, report_shadowed_character_sheets);
    app.add_plugins(crate::dev::DevToolsPlugin);
    add_physics_debris_plugins(app);
    // No UI-widget-framework plugin is installed here, and there is no
    // `add_ui_plugins` to call. This app's UI is plain Bevy UI plus the
    // typography Ambition owns (`MenuFont`, `MenuTextHeightFraction`,
    // `resolve_menu_text_size`). See the note on the `ui` feature in
    // `Cargo.toml` for what that feature still buys.
    // Input bindings/bridge live in `ambition_platformer2d::host::HostInputBindingsPlugin`
    // (E5 step 5). The app-local preset→InputMap resync that sat here
    // (`sync_preset_input_map`) is deleted: it reached its participant with
    // `single_mut()` — so a second couch seat made a preset change reach
    // nobody — and it shipped only in this app, so no demo composition had a
    // rebuild path at all. The engine owns it now:
    // `sync_primary_recipe_from_settings` +
    // `ambition_input::rebuild_maps_from_recipes` in the host input pipeline
    // (`InputSet::Collect`), for every seat in every composition.
    add_audio_plugins(app);
    add_mobile_touch_plugin(app);
    #[cfg(feature = "falling_sand")]
    app.add_plugins(ambition_content::falling_sand::FallingSandRoomPlugin);
    // Frame pacing / battery saver. Enabled by the normal visible personas so
    // desktop and Android exercise the same pacing behavior by default.
    #[cfg(feature = "frame_pacing")]
    app.add_plugins(crate::host::framepace::FramePacePlugin);

    // ⚠ the PLUGIN, not the bare system: `load_ui_fonts` is engine code, and
    // registering it here alone left every non-app composition with no `UiFonts`
    // and a vacuous `.after(UiFontsLoaded)`. Same defect the note below records
    // for `sync_resolved_visual_quality`.
    ui_fonts::UiFontsPlugin::ensure_installed(app);
    // ⚠ `sync_resolved_visual_quality` left this file on 2026-07-31:
    // `VisualQualityPlugin` owns the resource AND the system that keeps it
    // true, and `SessionRoomVisualsPlugin` installs it. They were apart, and
    // the half that MOVES was registered here alone — so every other
    // composition held a `ResolvedVisualQuality` that never left its default.
    // ⭐ and the OWNER, for the same reason as `UiFontsPlugin` above: this
    // function registers systems that take `Res<ResolvedVisualQuality>`, and the
    // plugin that installs it was reached only along the shipped host's path
    // (`SessionRoomVisualsPlugin`). `capture_scene` composes this function
    // WITHOUT that path, so it panicked on a missing resource before drawing a
    // frame. Idempotent by construction (`is_unique() -> false` plus a marker),
    // so saying it here costs the host nothing.
    app.add_plugins(ambition_platformer2d::render::quality::VisualQualityPlugin);
    app.add_systems(
        Update,
        // ⚠ `refresh_parallax_layers_on_quality_change` left this chain on
        // 2026-07-31: `SessionRoomVisualsPlugin` installs it now, beside the
        // theme load it exists to make visible. Registering it here as well
        // would run the despawn/respawn twice in one frame.
        (
            reload_visual_quality_assets_on_scale_change,
            ambition_platformer2d::render::rendering::refresh_entity_sprite_handles_on_game_assets_change,
        )
            .chain()
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    );
    // ⚠ `sync_portal_quality_budget` left this file on 2026-08-04, for exactly
    // the reason the note above gives for its sibling: it REQUIRES
    // `Res<ResolvedVisualQuality>`, which `VisualQualityPlugin` owns, and being
    // registered apart from it panicked every other composition that installs the
    // render presentation — `capture_scene` among them.
}

/// Pause menu, inventory, map menu, presentation startup, dev/dialog
/// hotkeys.
fn install_menu_setup_and_hotkeys(app: &mut App) {
    // Starter item-ownership roster (the 24-item catalog default set).
    app.add_plugins(ambition_content::items::AmbitionItemRosterPlugin);
    // An open inventory OWNS the seat's input, declared rather than derived:
    // every site that wanted this fact used to spell it `Paused &&
    // inventory.visible`, which is the `GameMode` derivation the participant
    // layer's own header forbids — and which said nothing at all in a
    // composition that never registers `GameMode`.
    app.add_plugins(inventory_ui::InventoryInputContextPlugin);
    app.insert_resource(inventory_ui::InventoryUiState::default())
        .init_resource::<ambition_platformer2d::actors::items::persist::InventoryRestored>()
        // Persist the inventory + wallet across save/load: restore the saved set
        // once the player exists, then mirror live changes back into the save
        // (the existing autosave writes the dirtied save to disk).
        .add_systems(
            Update,
            (
                ambition_platformer2d::actors::items::persist::restore_inventory_from_save,
                ambition_platformer2d::actors::items::persist::persist_inventory_to_save,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (ambition_platformer2d::actors::menu::map::sync_map_menu,)
                .after(Platformer2dSimulationPhaseMonolith::CoreSimulation)
                .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
        )
        .add_systems(
            Startup,
            (
                ambition_platformer2d::dev_tools::profiling::phase_mark(
                    "before_setup_presentation",
                ),
                // `PresentationSetupSet` is the machinery-facing label for
                // this slot: audio init (and any future machinery startup
                // work) orders `.after(the set)` instead of naming this
                // app system.
                // K2b edit 3: the direct-entry presentation startup is gone
                // with its entry path; the host one is UNCONDITIONAL now,
                // because there is no longer a composition without the marker
                // it was testing for.
                setup_host_presentation_system.in_set(PresentationSetupSet),
                ambition_platformer2d::dev_tools::profiling::phase_mark("after_setup_presentation"),
                ambition_platformer2d::actors::menu::map::populate_map_rooms,
                ambition_platformer2d::dev_tools::profiling::phase_mark("after_map_menu_spawn"),
            )
                .chain()
                .after(setup_simulation_system)
                .after(ui_fonts::UiFontsLoaded),
        )
        .add_systems(
            Update,
            (
                // ⚠ `dialog_input` LEFT this chain on 2026-08-02 — it and
                // `dialog_reveal_tick` are now owned by the monolith's
                // `YarnBindingsPlugin`, beside the rest of the feature.
                handle_ldtk_hot_reload,
                handle_debug_hotkeys,
                // ⚠ `sync_developer_body_profile` LEFT this chain on
                // 2026-08-02: it writes rollback state (`BodyBaseSize`), so
                // it belongs in `DevToolsSimPlugin`'s `DevEditApplySet`,
                // which is in the schedule that rewinds.
                ambition_platformer2d::actors::trace::handle_trace_hotkey,
                ambition_platformer2d::actors::menu::map::handle_map_menu_hotkeys,
            )
                .chain()
                .after(Platformer2dSimulationPhaseMonolith::CoreSimulation)
                .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
        )
        .add_systems(PostUpdate, restart_local_ggrs_after_hot_reload);

    // Unified menu (the one menu): install backend-agnostic menu state first,
    // then install each compiled backend independently. The backend features are
    // platform-neutral so desktop and Android stay in sync unless a build profile
    // intentionally opts out of a backend.
    crate::menu::kaleidoscope_app::install_unified_menu_shared(app);
    // The cube backend exists only when its feature is on. The runtime constant
    // below is the SELECTION (a build may compile the cube and still boot on the
    // grid); the cfg is whether there is a cube to select at all. Both are
    // needed, and conflating them is what made the web build fail to compile the
    // FLAT menu.
    #[cfg(feature = "kaleidoscope_menu")]
    if ambition_platformer2d::menu::backend::KALEIDOSCOPE_MENU_BACKEND_ENABLED {
        crate::menu::kaleidoscope_app::install_kaleidoscope_menu_backend(app);
    }
    #[cfg(feature = "bevy_ui_menu")]
    if ambition_platformer2d::menu::backend::BEVY_UI_MENU_BACKEND_ENABLED {
        crate::menu::grid_backend::install_grid_unified_menu(app);
    }
}

fn install_camera_and_debug_overlay_systems(app: &mut App) {
    // The camera cluster itself (viewport publish, shake, follow, portal
    // continuity) is `ambition_platformer2d::host::HostCameraPlugin` (E5 step 5). What
    // stays here is the Ambition DEBUG OVERLAY, drawn once the camera has
    // landed this frame.
    app.init_resource::<debug_overlay::DebugOverlayLabels>();
    // With portals, the continuity camera tag (registered by HostCameraPlugin)
    // must land before the overlay reads it.
    #[cfg(feature = "portal_render")]
    let overlay = (
        debug_overlay::draw_debug_overlay,
        // Materialize the labels the overlay just queued (Text2d). Runs
        // right after so the labels track this frame's boxes.
        debug_overlay::render_debug_overlay_labels,
    )
        .chain()
        .after(camera_follow)
        .after(ambition_platformer2d::host::portal::PortalContinuityCameraTagged);
    #[cfg(not(feature = "portal_render"))]
    let overlay = (
        debug_overlay::draw_debug_overlay,
        debug_overlay::render_debug_overlay_labels,
    )
        .chain()
        .after(camera_follow);
    app.add_systems(
        Update,
        overlay.run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    );
}

fn install_fx_and_hud_systems(app: &mut App) {
    // ⚠ the VFX tick cluster (`update_particles` / `update_explosions` /
    // `update_impacts` / the two speech-bubble passes) left this file on
    // 2026-08-15, together with `vfx_spawn_messages` below:
    // `ambition_platformer2d_host::HostVfxPresentationPlugin` registers them for
    // every composition. Until then this app was the only one that DREW a
    // `VfxMessage` it had written — which is why Mary-O's coin never popped.
    app.add_systems(
        Update,
        (
            update_hud,
            ambition_platformer2d::render::rendering::sync_boss_health_bar_overlay,
            // ⚠ `dialog_reveal_tick` LEFT this chain on 2026-08-02; see the
            // note at `dialog_input` above. It is now CHAINED after the
            // input in one plugin, which those two never were here.
            ambition_platformer2d::render::cutscene::sync_cutscene_ui,
            // Keeps the overlay in the reading rect as the layout moves; the
            // spawn above only gets the first frame right. See
            // `ambition_render::reading_layout`.
            ambition_platformer2d::render::reading_layout::fit_to_reading_rect::<
                ambition_platformer2d::render::cutscene::CutsceneOverlayRoot,
            >,
        )
            .chain()
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    )
    // Always-on *during gameplay* player HUD overlay (health / mana /
    // money bars). The title route owns no gameplay HUD authority.
    .add_systems(
        Update,
        (
            // ⚠ `regen_player_mana` LEFT this chain on 2026-08-02. It mutates
            // `BodyMana`, which is rollback-registered (`body.mana`), and it was
            // running here in `Update` at render rate -- outside the rollback
            // schedule entirely. Its own doc already said "a sim mutator never
            // lives in presentation" (E4); the code had moved out of the render
            // module and the REGISTRATION had not. It now runs in the engine's
            // FeatureCollection phase, so every composition regenerates mana and
            // a rewind resimulates it.
            ambition_platformer2d::render::hud::spawn_player_hud,
            ambition_platformer2d::render::hud::update_player_hud,
            // Ambition's built-in HP/MP/$ row hides whenever the active game
            // declared its OWN HUD (Sanic rings, Mary-O score), so vitals never
            // overlay a game that has no health/mana (Jon bug #36).
            ambition_platformer2d::render::hud::toggle_builtin_hud_for_declared_games,
            // Consumes THIS frame's resolved HUD regions, so a profile that
            // reserves surround for HUD actually gets the HUD put there.
            ambition_platformer2d::render::hud::place_player_hud.after(
                ambition_platformer2d::presentation::gameplay_presentation::GameplayPresentationSet,
            ),
        )
            .chain()
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    );
}

/// Health overlays, portal sprite sync, parallax, dialog redirect,
/// lock-wall visuals, NPC sprite upgrade, map-menu pointer dismiss,
/// quest panel. Each system is its own `add_systems` call because the
/// big presentation tuple is already at Bevy's 20-system arity ceiling.
fn install_misc_visual_sync_systems(app: &mut App) {
    #[cfg(feature = "portal_render")]
    app.add_systems(
        Update,
        ambition_platformer2d::render::rendering::sync_portal_capture_parallax_layers
            .after(ambition_platformer2d::portal_presentation::PortalPresentationSet)
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    );

    app.add_systems(
        Update,
        ambition_platformer2d::render::rendering::sync_health_overlays
            .after(sync_visuals)
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    )
    // Idle barks fire on a 5-10s cadence while the boss is in an
    // attacking phase, so the scholar feels alive between strikes.
    .add_systems(
        Update,
        ambition_content::bosses::tick_boss_idle_barks
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    )
    // Portal presentation: read GatePortalRegistry.phase + apply
    // visibility / animation row / ring-spin to the matching
    // PropVisual-named sprites + hide the redundant debug
    // door-zone visual for portal-mode LoadingZones. Render-side
    // systems (E4 slices 10+20): they consume the sim's phase
    // registry and never live in the sim crate. Runs after
    // sync_visuals so the sprite entities exist this frame.
    .add_systems(
        Update,
        (
            ambition_platformer2d::render::rendering::gate_portal_visuals::sync_portal_sprite_visibility,
            ambition_platformer2d::render::rendering::gate_portal_visuals::sync_portal_sprite_animation,
            ambition_platformer2d::render::rendering::gate_portal_visuals::sync_portal_ring_rotation_system,
            ambition_platformer2d::render::rendering::gate_portal_visuals::hide_portal_loading_zone_visuals,
        )
            .after(sync_visuals)
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    )
    // ⚠ `sync_parallax_layers` left this list on 2026-07-31, with the theme
    // load and the refresh: `SessionRoomVisualsPlugin` registers it
    // `.after(camera_follow)` for every composition, because a backdrop that
    // does not move is not a parallax layer.
    // Encounter / intro LockWall visuals. Reconciles `LockWallVisual`
    // Bevy entities against the collision overlay's `gate_solids` (the
    // lock walls the gate contributors derive each frame in WorldPrep,
    // no longer mutated into the authored base) so the wall is visible
    // when an encounter slams it shut. Pinned after
    // `drive_wave_encounters` so it runs late in the frame, well
    // after the WorldPrep contributor has populated `gate_solids`.
    .add_systems(
        Update,
        ambition_platformer2d::render::rendering::sync_lock_wall_visuals
            .after(ambition_platformer2d::actors::encounter::WaveEncounterDriven)
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    )
    // Dev "hide sprites" / "placeholder sprites" overrides — must run
    // after every other visibility- or sprite-setting system so they
    // win the last-write battle. `sync_morph_ball_visual`,
    // `sync_bubble_shield_visual`, and the projectile rebuild systems
    // all also run `.after(sync_visuals)` and unconditionally set
    // `Visibility` (or despawn-respawn fresh `Inherited` sprites). If
    // the override ran in parallel, Bevy could schedule either order
    // and the player / shield / projectile sprites would sporadically
    // remain visible. Explicit ordering keeps the toggle deterministic.
    .add_systems(
        Update,
        (
            ambition_platformer2d::render::rendering::apply_placeholder_sprites_override,
            ambition_platformer2d::render::rendering::apply_hide_sprites_override,
        )
            .chain()
            // One boundary instead of four names: `SpriteVisualSync` holds
            // every pass that decides a sprite's handle, tint or visibility, so
            // a new one joins it and this override keeps working.
            .after(ambition_platformer2d::render::rendering::SpriteVisualSync)
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    )
    // Mouse / touch dismissal for the map menu.
    .add_systems(
        Update,
        ambition_platformer2d::actors::menu::map::map_menu_pointer_dismiss
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    )
    // Quest panel runs alongside the verbose HUD.
    .add_systems(
        Update,
        update_quest_panel
            .after(ambition_platformer2d::render::dialog_ui::DialogPresentationSet)
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    );
}

/// Projectile sprite ring + VFX/debris message subscribers + (input-
/// feature-gated) blink preview ring.
fn install_projectile_and_vfx_systems(app: &mut App) {
    // Player projectile visuals: rebuild the sprite ring each tick
    // from `PlayerProjectileState::bodies`. Must run after
    // `update_projectiles` so the body list reflects this frame's
    // spawn / tick / collision before the visuals are rebuilt —
    // otherwise newly-fired projectiles would only become visible
    // one frame late.
    // ⚠ the projectile visual passes left this file on 2026-07-31:
    // `ambition_platformer2d_host::HostProjectileVisualsPlugin` registers them, with their
    // two ordering edges intact, for every composition. Until then this app was
    // the only one that DREW a projectile it had fired.
    //
    // ⚠ **and the VFX subscriber followed on 2026-08-15, for the same reason and
    // with the same symptom.** `fx::process_*_requests` + `vfx_spawn_messages`
    // now live in `ambition_platformer2d_host::HostVfxPresentationPlugin`. This
    // app was the only composition that drew a `VfxMessage` it had written, so
    // the Mary-O demo's coin pop, its brick-break burst and every impact spark
    // in the smash demo were spawned into nothing.
    //
    // Live blink-destination preview ring. Reads leafwing action state to
    // know when the blink button is held, so it lives behind the `input`
    // feature alongside the other gameplay-input-driven presentation.
    #[cfg(not(feature = "input"))]
    let _ = app;
    #[cfg(feature = "input")]
    app.add_systems(
        Update,
        fx::update_blink_preview
            .after(Platformer2dSimulationPhaseMonolith::CoreSimulation)
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    );
}

/// Install the Avian2D secondary-physics plugin and its presentation-side
/// debris subscriber. Gated by `physics_debris` so headless / minimal
/// builds drop `avian2d` from the dep graph entirely. Per ADR 0007, this
/// is secondary physics for debris/ragdoll visuals only — the player
/// controller stays kinematic.
#[cfg(feature = "physics_debris")]
pub(super) fn add_physics_debris_plugins(app: &mut App) {
    app.add_plugins(physics::AmbitionPhysicsPlugin).add_systems(
        Update,
        physics_spawn_debris_messages
            .after(Platformer2dSimulationPhaseMonolith::CoreSimulation)
            .run_if(ambition_platformer2d::platformer::lifecycle::session_world_exists),
    );
}

#[cfg(not(feature = "physics_debris"))]
pub(super) fn add_physics_debris_plugins(_app: &mut App) {}

// Ambition's UI is plain Bevy UI, and no widget framework is installed here.
// Typography is owned by `ambition_menu` — `MenuFont`,
// `MenuTextHeightFraction`, `resolve_menu_text_size`.
//
// ⚠ `bevy_material_ui` sat here until 2026-08-08 under a comment calling its
// `DialogPlugin` LOAD-BEARING for menu typography. Measured false: the launcher
// renders the same 29 text entities at the same sizes either way. Do not
// reinstate it on that reasoning — `dev/journals/material-ui-removal-2026-08-08.md`
// has the forensics.
//
// The leafwing input bindings + the device→ControlFrame bridge live in
// `ambition_platformer2d::host::HostInputBindingsPlugin` (E5 step 5); the dev
// preset-input-map sync stays registered app-side (dev_runtime).

/// Register the [`TouchControlsPlugin`](ambition_platformer2d::touch_input::TouchControlsPlugin)
/// (`virtual_joystick` stick + on-screen action buttons). The touch overlay is
/// a VIRTUAL DEVICE: its state is exposed to leafwing as registered input
/// kinds and bound in the persistent participant's `InputMap`, so touch
/// resolves through the same bindings/context pipeline as the keyboard and
/// gamepad — there is no second `ControlFrame` writer. The adapter lives in
/// the sibling `ambition_platformer2d::touch_input` crate (app-thinness); the app's
/// `mobile_touch` feature forwards to `ambition_platformer2d::touch_input/mobile_touch`,
/// which pulls the optional `virtual_joystick` dep. Added UNCONDITIONALLY
/// whenever `mobile_touch` is compiled — no runtime boolean gates it. To rip
/// the touch controls out, remove the single `add_plugins(TouchControlsPlugin)`
/// line below. On builds compiled without `mobile_touch` this is a no-op.
#[cfg(feature = "mobile_touch")]
pub(super) fn add_mobile_touch_plugin(app: &mut App) {
    app.add_plugins(ambition_platformer2d::touch_input::TouchControlsPlugin);
}

#[cfg(not(feature = "mobile_touch"))]
pub(super) fn add_mobile_touch_plugin(_app: &mut App) {}

/// Install the sandbox audio subsystem. Gated by `audio` so headless
/// / RL builds drop `bevy_kira_audio` from the dep graph entirely;
/// the sim still emits `SfxMessage`s and the queue drains harmlessly
/// per the ADR 0012 seam.
#[cfg(feature = "audio")]
pub(super) fn add_audio_plugins(app: &mut App) {
    app.add_plugins(ambition_platformer2d::actors::audio::Platformer2dAudioPlugin);
    // Once the resident SFX bank lands, publish its ids as Ambition's
    // provider-relative SFX authority (bank = storage, selection = permission).
    app.add_systems(
        Update,
        super::setup_systems::publish_resident_sfx_bank_authority,
    );
}

#[cfg(not(feature = "audio"))]
pub(super) fn add_audio_plugins(_app: &mut App) {}

// ── Domain plugin structs ──────────────────────────────────────────────────
//
// These are the public Bevy `Plugin` API for callers that just want to
// `app.add_plugins(…)` without knowing about the internal helper functions.
// The helper functions (`init_sandbox_resources`, `add_simulation_plugins`,
// etc.) stay public so callers that need to inject resources between steps
// (e.g. inserting `StartRoomOverride` before resources are consumed) can
// still call them in sequence.

/// Installs all sandbox simulation resources and systems — the subset
/// that is safe for both visible and headless builds. Calls
/// `init_sandbox_resources` then `add_simulation_plugins`.
pub struct AmbitionGameSimulationPlugin;

impl Plugin for AmbitionGameSimulationPlugin {
    fn build(&self, app: &mut App) {
        // `init_sandbox_resources` composes provider catalogs before building
        // the asset manifest and world/session resources. The later content
        // plugin registration is byte-identical and therefore idempotent.
        init_sandbox_resources(app);
        add_simulation_plugins(app);
    }
}

/// Installs LDtk runtime spine registrations and `LdtkPlugin`. Visible
/// binary only — `LdtkPlugin` panics in headless (no `RenderApp`).
pub struct AmbitionGameLdtkRuntimePlugin;

impl Plugin for AmbitionGameLdtkRuntimePlugin {
    fn build(&self, app: &mut App) {
        add_ldtk_runtime_plugin(app);
    }
}

/// Installs all presentation-side plugins: input, audio, VFX, HUD, debug
/// overlays, and platform plugins. Visible binary only.
pub struct AmbitionGamePresentationPlugin;

impl Plugin for AmbitionGamePresentationPlugin {
    fn build(&self, app: &mut App) {
        add_presentation_plugins(app);
    }
}

/// **Warn only for a shadowed sheet target that is a CHARACTER, because only
/// those are resolved by target** — ledger D36.
///
/// ⛔ **the sheet crate cannot make this call and must not learn to.** Its
/// collision check fired ~30 times per Android boot because 17 characters share
/// the rig target `toon`, 18 share `robot` and 9 share `goblin`, all
/// legitimately and with genuinely different frame sizes — and nothing looks art
/// up by a rig name. What DOES get looked up by target is a character id, and
/// that is the case that cost a day: a stale May manifest won
/// `pirate_heavy_broadside_bess`, so she loaded the right image and cropped it
/// with a dead grid.
///
/// ⚠ **`Option` on both, and that is not defensive padding.** A composition may
/// legitimately reach here with neither — a headless tool, a demo that mounts no
/// Ambition cast — and the correct behaviour there is to say nothing rather than
/// to report every rig target as suspicious.
fn report_shadowed_character_sheets(
    registry: Option<bevy::prelude::Res<ambition_platformer2d::sprite_sheet::SheetRegistry>>,
    catalog: Option<
        bevy::prelude::Res<
            ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog,
        >,
    >,
) {
    let (Some(registry), Some(catalog)) = (registry, catalog) else {
        return;
    };
    for shadowed in registry.shadowed_targets() {
        if catalog.get(&shadowed.target).is_some() {
            bevy::log::warn!("SheetRegistry: {shadowed}");
        }
    }
}
