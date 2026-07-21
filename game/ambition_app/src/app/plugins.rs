use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollectionApp;
use bevy_ecs_ldtk::prelude::LdtkPlugin;
#[cfg(feature = "ui")]
use bevy_material_ui::MaterialUiPlugin;

use ambition::actors::assets::loading;
use ambition::actors::ldtk_world;
use ambition::actors::rooms;
use ambition::actors::time::feel::SandboxFeelTuning;
#[cfg(feature = "physics_debris")]
use ambition::actors::world::physics;
#[cfg(feature = "physics_debris")]
use ambition::actors::world::physics::physics_spawn_debris_messages;
use ambition::dev_tools::dev_tools::{
    self, DeveloperTools, EditableAbilitySet, EditableMovementTuning, EditablePlayerStats,
    MovementProfile, PlayerBodyProfile,
};
use ambition::inventory_ui;
use ambition::platformer::schedule::{
    gameplay_allowed, PresentationSetupSet, SandboxSet, SimScheduleExt,
};
use ambition::render::fx::{self, vfx_spawn_messages};
use ambition::render::rendering::{camera_follow, sync_visuals};
use ambition::render::ui_fonts;

use crate::dev::debug_overlay;
use crate::host::windowing;

use super::dev_runtime::{
    handle_debug_hotkeys, handle_ldtk_hot_reload, restart_local_ggrs_after_hot_reload,
    sync_preset_input_map,
};
use super::hud::{update_hud, update_quest_panel};
use super::player_tick::{apply_home_reset_policy, sync_player_presentation};
use super::resources::init_sandbox_resources;
use super::setup_systems::{
    reload_visual_quality_assets_on_scale_change, setup_host_presentation_system,
    setup_presentation_system, setup_simulation_system,
};
use super::sim_systems::{apply_player_reset_input_system, apply_room_replay_request_system};
use super::world_flow::{
    advance_room_transition_content_epoch_system, authorize_ready_room_transition_system,
    begin_room_transition_load_system, commit_ready_room_transition_system,
    finalize_unpresented_room_transition_failure_system, RoomTransitionContentEpoch,
    RoomTransitionLoadState,
};

/// Register core simulation plugins, message types, and the gameplay
/// schedule. Headless and visible both call this.
///
/// The body is split into per-set helpers below so each section is short
/// enough to read in one screen and stays under Bevy's 20-system tuple
/// arity limit. New simulation systems should go into the matching
/// `register_*_systems` helper rather than back into this orchestrator.
pub fn add_simulation_plugins(app: &mut App) {
    app.add_message::<ambition::platformer::developer_hotkeys::DeveloperAction>();
    // AmbitionPhysicsPlugin (Avian2D) is intentionally NOT here. Per
    // ADR 0007 Avian is secondary physics for debris/ragdoll visuals;
    // the player controller is custom via parry2d in ambition::engine_core.
    // Avian's collider backend needs `SceneSpawner` (from ScenePlugin in
    // DefaultPlugins), which headless doesn't have. Until Avian's debris
    // role is migrated to presentation events end-to-end (or Avian gains
    // a headless-friendly init path), it lives in
    // `add_presentation_plugins`.

    // Room transitions are load transactions in both direct-entry and shell
    // hosts. Install the contributor-neutral coordinator at the simulation
    // boundary; shell composition later adds only its route adapter and
    // presentation. Avoid a duplicate when an alternate host installed it
    // before this plugin.
    if !app.is_plugin_added::<ambition::load::AmbitionLoadPlugin>() {
        app.add_plugins(ambition::load::AmbitionLoadPlugin);
    }
    app.init_resource::<RoomTransitionLoadState>()
        .init_resource::<RoomTransitionContentEpoch>();

    // The canonical simulation-phase sets + engine resources now live in
    // `ambition::runtime::SandboxSetsPlugin` (first in the engine group below).
    // Host configuration overrides are consumed before simulation plugins
    // build. Live gameplay-world values are already components on the exact
    // direct/session root; no canonical world value is initialized as a resource.

    // The construction-time host must be chosen before the first content/sim
    // plugin. Missing means the lightweight render-frame host.
    let simulation_host = app
        .world()
        .get_resource::<ambition::runtime::SimulationHost>()
        .copied()
        .unwrap_or_default();

    app.add_plugins(super::sim_resources::SandboxSimulationResourcesPlugin);

    // Named Ambition game content: quests, bosses, dialogue/cutscenes, intro
    // hooks, and portal adapters. Installed after simulation resources so content
    // registries land at the expected assembly point.
    app.add_plugins(ambition_content::AmbitionContentPlugin);

    // Yarn dialogue stack: compile `.yarn`, bridge runner events into sandbox
    // state, and register the commands / functions / markup used by content.
    #[cfg(feature = "ui")]
    {
        app.add_plugins(ambition_content::dialogue::yarn_spinner_plugin());
        app.add_plugins(ambition::actors::dialog::YarnBridgePlugin);
        app.add_plugins(ambition::actors::dialog::YarnBindingsPlugin);
    }

    // The content-free engine SIMULATION plugins (E5): the SAME
    // `PlatformerEnginePlugins` group a demo app builds on — the sandbox sets
    // + engine resources, the sim schedule, the universal brain, gravity,
    // traversal abilities, item pickups, encounters/cutscenes, feature
    // collection/interaction/effects/view-sync, room reset, traces,
    // affordances, and the combat-phase chain. Ordering is set-based, so
    // group membership does not change the resolved schedule.
    app.add_plugins(ambition::runtime::PlatformerEnginePlugins::new(
        simulation_host,
    ));

    // App-LOCAL residue the E5 step-5 carve deliberately left behind. The
    // engine group above registers the shared per-frame wiring (player input
    // chain, brains, possession, room-transition detect/reset, portal
    // schedule, progression); these systems wrap app-only concerns
    // (`reset_sandbox`, `load_room` + render spawns, the player clone) and
    // pin themselves into the documented ordering SLOTS between engine
    // systems (see `ambition::runtime::PlayerSchedulePlugin` /
    // `RoomTransitionSchedulePlugin` module docs).
    register_app_local_sim_systems(app);

    // All construction/snapshot registries are now installed. Publish the
    // direct-entry session root from the same immutable prepared-content path
    // used by shell activation.
    super::resources::publish_direct_prepared_session_root(app);
}

/// The app-LOCAL per-frame systems, pinned into the ordering SLOTS the engine
/// chains leave for the host (E5 step 5). Everything engine-generic that used
/// to be registered here lives in `ambition::runtime::{PlayerSchedulePlugin,
/// RoomTransitionSchedulePlugin, PortalSchedulePlugin,
/// ProgressionSchedulePlugin}`.
fn register_app_local_sim_systems(app: &mut App) {
    let sim = app.sim_schedule();
    // ── The PlayerInput gap: the Ambition reset/replay consumers ──────────
    //
    // Both call the app-only `world_flow::reset_sandbox` — so they stay
    // app-side, slotted after the dev-edit sync and before the input timer
    // (the exact position they held in the old inline chain). The replay
    // consumer is now host-generic (it no longer names content): the cut-rope
    // per-attempt reset moved to content's `ContentRoomReplayResetSet`.
    app.add_systems(
        sim,
        (
            apply_player_reset_input_system.run_if(gameplay_allowed),
            apply_room_replay_request_system,
        )
            .chain()
            .in_set(SandboxSet::PlayerInput)
            .after(ambition::dev_tools::DevEditApplySet)
            .before(ambition::actors::control::input_timer_system),
    );
    // Content dialogue-followup emitters (e.g. cut-rope "try again") run
    // before the replay consumer that drains their requests the same frame;
    // content's replay-reset systems run before it too, so a named boss's
    // per-attempt state is cleared the same frame the room replays. The engine
    // anchors each slot's PHASE (PlayerInput); the consumer edge is ours
    // because the consumer is ours.
    app.configure_sets(
        sim,
        (
            ambition::actors::session::reset::ContentDialogueFollowupSet,
            ambition::actors::session::reset::ContentRoomReplayResetSet,
        )
            .before(apply_room_replay_request_system),
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
    app.init_resource::<crate::app::player_clone::PlayerCloneClock>()
        .init_resource::<crate::app::player_clone::SpawnPlayerCloneRequest>()
        .add_systems(
            sim,
            (
                crate::app::player_clone::request_player_clone_on_key,
                crate::app::player_clone::spawn_requested_player_clone,
            )
                .chain()
                .in_set(SandboxSet::WorldPrep),
        )
        .add_systems(
            sim,
            crate::app::player_clone::tick_player_clone_brains
                .run_if(gameplay_allowed)
                .in_set(SandboxSet::PlayerInput),
        )
        .add_systems(
            sim,
            crate::app::player_clone::sync_player_clone_transform
                .in_set(SandboxSet::PresentationSync),
        )
        .add_systems(
            sim,
            crate::app::player_clone::despawn_player_clones_on_reset
                .in_set(SandboxSet::ResetProcessing)
                .before(ambition::actors::session::reset::process_sandbox_reset_request),
        );

    // ── The PlayerSimulation gap: home reset policy + home presentation ───
    //
    // Slotted between the possession release and the hit-event drain (the
    // exact position they held in the old inline chain).
    app.add_systems(
        sim,
        (
            // HOME RESET POLICY. Movement already integrated the home body in
            // `WorldPrep` and flagged any reset in `PlayerBodyFrameOutput`;
            // this owns the home-only sandbox + room reset on that flag (an
            // actor never teleports to the player spawn). Moves no body.
            apply_home_reset_policy.run_if(gameplay_allowed),
            // HOME PRESENTATION — screen shake + landing SFX + the per-op
            // anim/SFX/VFX — reads the same hand-off. Moves no body.
            sync_player_presentation.run_if(gameplay_allowed),
        )
            .chain()
            .in_set(SandboxSet::PlayerSimulation)
            .after(
                ambition::actors::abilities::traversal::possession::release_possession_if_target_lost,
            )
            .before(ambition::actors::features::ecs::damage_apply::apply_player_hit_events),
    );

    // ── The RoomTransition gap: readiness transaction + authorized commit ──
    //
    // Detection emits `RoomTransitionRequested`. The app turns it into an
    // exact `ambition_load` plan, preflights the target while the source room
    // remains authoritative, obtains one-shot authorization on a later sim
    // tick, and only then performs the existing synchronous construction and
    // render commit. Visible hosts additionally require a cover that has
    // survived a presentation frame before authorization can succeed.
    app.add_systems(
        sim,
        (
            advance_room_transition_content_epoch_system,
            begin_room_transition_load_system,
            authorize_ready_room_transition_system,
            finalize_unpresented_room_transition_failure_system,
            commit_ready_room_transition_system,
        )
            .chain()
            .in_set(SandboxSet::RoomTransition)
            .after(ambition::actors::rooms::detect_room_transition_system)
            .before(ambition::actors::features::reset_ecs_room_features),
    );
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
    // `SandboxAssetCollection` includes a typed LDtk handle, so the LDtk
    // asset type and loader must be initialized before bevy_asset_loader
    // allocates collection handles. Keep this before `init_collection`.
    app.add_plugins(LdtkPlugin)
        .init_collection::<loading::SandboxAssetCollection>()
        .add_plugins(ldtk_world::AmbitionLdtkRegistrationPlugin)
        .add_systems(
            Startup,
            (
                ldtk_world::load_ldtk_asset_handle,
                // Direct entry constructs the world spine at boot; the shell
                // host spawns SESSION-scoped roots per activation instead.
                spawn_ldtk_world_root
                    .after(setup_simulation_system)
                    .run_if(super::shell_host::direct_entry),
            ),
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
                .run_if(ambition::platformer::lifecycle::session_world_exists),
        );
}

/// Spawn the `LdtkWorldBundle` entity. Runs in `add_ldtk_runtime_plugin`
/// (visible binary only) after `setup_simulation_system` so the
/// `LdtkRuntimeIndex` session component is available.
pub(super) fn spawn_ldtk_world_root(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ldtk_index: ambition::platformer::lifecycle::SessionWorldRef<ldtk_world::LdtkRuntimeIndex>,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<rooms::RoomSet>,
    world_assets: Option<Res<ldtk_world::LdtkWorldAssets>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
    world_manifest: Res<ldtk_world::WorldManifest>,
) {
    spawn_ldtk_world_roots_scoped(
        &mut commands,
        ambition::platformer::lifecycle::SessionSpawnScope::UNSCOPED,
        &asset_server,
        &ldtk_index,
        &room_set,
        world_assets.as_deref(),
        sandbox_asset_collection.as_deref(),
        &world_manifest,
    );
}

/// The LdtkWorldBundle spawn shared by direct startup (`UNSCOPED`,
/// process-resident) and the shell host's per-session activation (scoped, so
/// the session sweep retires the visual spine roots with the session).
pub(crate) fn spawn_ldtk_world_roots_scoped(
    commands: &mut Commands,
    scope: ambition::platformer::lifecycle::SessionSpawnScope,
    asset_server: &AssetServer,
    ldtk_index: &ldtk_world::LdtkRuntimeIndex,
    room_set: &rooms::RoomSet,
    world_assets: Option<&ldtk_world::LdtkWorldAssets>,
    sandbox_asset_collection: Option<&loading::SandboxAssetCollection>,
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
    if !app.is_plugin_added::<ambition::load_presentation::AmbitionLoadPresentationPlugin>() {
        app.add_plugins(ambition::load_presentation::MinimalLoadPresentationPlugins);
    }
    super::world_flow::install_room_transition_presentation(app);
    // The windowed-host face (E5 step 5): leafwing input bindings + the
    // camera follow/shake cluster (+ portal camera continuity). The SAME
    // group a windowed demo adds; the app-local presentation below layers
    // Ambition's HUD/menu/dev stack on top.
    app.add_plugins(ambition::host::PlatformerHostPlugins);
    install_presentation_resources_and_subplugins(app);
    app.add_plugins((
        ambition::persistence::PersistenceSchedulePlugin,
        ambition::dev_tools::DeveloperPersistenceSchedulePlugin,
    ));
    install_menu_setup_and_hotkeys(app);
    app.add_plugins(ambition::render::rendering::PresentationVisualAnimationPlugin);
    // Ambition's named presentation passes (puppy-slug deep-dream) compose onto
    // the renderer's public `ActorOverlaySet` seam the plugin above positions.
    app.add_plugins(ambition_content::presentation::AmbitionPresentationPlugin);
    install_camera_and_debug_overlay_systems(app);
    app.add_plugins(ambition::render::rendering::ActorNameplatePresentationPlugin);
    install_fx_and_hud_systems(app);
    install_misc_visual_sync_systems(app);
    app.add_plugins(ambition::render::rendering::PlayerVisualSchedulePlugin);
    install_projectile_and_vfx_systems(app);
}

/// Visible-side resources, registered types, and presentation child
/// plugins (input, audio, dev_tools, physics_debris, ui, mobile touch,
/// FPS overlay, font loader).
fn install_presentation_resources_and_subplugins(app: &mut App) {
    app.insert_resource(ClearColor(Color::srgb(0.020, 0.024, 0.035)))
        .init_resource::<ambition::render::quality::ResolvedVisualQuality>()
        .insert_resource(windowing::DisplayModeState::default())
        .register_type::<DeveloperTools>()
        .register_type::<PlayerBodyProfile>()
        .register_type::<MovementProfile>()
        .register_type::<EditableAbilitySet>()
        .register_type::<EditableMovementTuning>()
        .register_type::<EditablePlayerStats>()
        .register_type::<SandboxFeelTuning>()
        .register_type::<ambition::portal::PortalConvention>()
        .register_type::<ambition::portal::PortalTuning>();

    #[cfg(feature = "portal_render")]
    app.register_type::<ambition::portal_presentation::PortalVisualEffect>()
        .register_type::<ambition::portal_presentation::PortalEffectSelection>()
        .register_type::<ambition::portal_presentation::PortalCameraTransitMode>()
        .register_type::<ambition::portal_presentation::PortalCameraContinuitySelection>()
        .register_type::<ambition::portal_presentation::PortalCameraContinuityConfig>()
        .register_type::<ambition::portal_presentation::PortalCameraContinuityState>()
        .register_type::<ambition::portal_presentation::PortalViewConeConfig>();

    app.add_plugins(crate::host::platform::PlatformPlugin);
    app.add_plugins(ambition::render::screen_effects::ScreenEffectsPlugin);
    // Loads baked `*_spritesheet.ron` manifests for runtime sheet metadata.
    app.add_plugins(ambition::sprite_sheet::SheetRegistryPlugin);
    app.add_plugins(crate::dev::DevToolsPlugin);
    add_physics_debris_plugins(app);
    add_ui_plugins(app);
    // Input bindings/bridge live in `ambition::host::HostInputBindingsPlugin`
    // (E5 step 5). The app-local residue: the dev preset-input-map sync.
    #[cfg(feature = "input")]
    app.add_systems(
        Update,
        sync_preset_input_map.before(SandboxSet::CoreSimulation),
    );
    add_audio_plugins(app);
    add_mobile_touch_plugin(app);
    #[cfg(feature = "falling_sand")]
    app.add_plugins(ambition_content::falling_sand::FallingSandRoomPlugin);
    // Frame pacing / battery saver. Enabled by the normal visible personas so
    // desktop and Android exercise the same pacing behavior by default.
    #[cfg(feature = "frame_pacing")]
    app.add_plugins(crate::host::framepace::FramePacePlugin);

    app.add_systems(Startup, ui_fonts::load_ui_fonts);
    app.add_systems(
        Update,
        ambition::render::quality::sync_resolved_visual_quality,
    );
    app.add_systems(
        Update,
        (
            reload_visual_quality_assets_on_scale_change,
            ambition::render::rendering::refresh_entity_sprite_handles_on_game_assets_change,
            ambition::render::rendering::refresh_parallax_layers_on_quality_change,
        )
            .chain()
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    );
    #[cfg(feature = "portal_render")]
    app.add_systems(
        Update,
        ambition::render::quality::sync_portal_quality_budget,
    );
}

/// Pause menu, inventory, map menu, presentation startup, dev/dialog
/// hotkeys.
fn install_menu_setup_and_hotkeys(app: &mut App) {
    // Starter item-ownership roster (the 24-item catalog default set).
    app.add_plugins(ambition_content::items::AmbitionItemRosterPlugin);
    app.insert_resource(inventory_ui::InventoryUiState::default())
        .init_resource::<ambition::actors::items::persist::InventoryRestored>()
        // Persist the inventory + wallet across save/load: restore the saved set
        // once the player exists, then mirror live changes back into the save
        // (the existing autosave writes the dirtied save to disk).
        .add_systems(
            Update,
            (
                ambition::actors::items::persist::restore_inventory_from_save,
                ambition::actors::items::persist::persist_inventory_to_save,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (ambition::actors::menu::map::sync_map_menu,)
                .after(SandboxSet::CoreSimulation)
                .run_if(ambition::platformer::lifecycle::session_world_exists),
        )
        .add_systems(
            Startup,
            (
                ambition::dev_tools::profiling::phase_mark("before_setup_presentation"),
                // `PresentationSetupSet` is the machinery-facing label for
                // this slot: audio init (and any future machinery startup
                // work) orders `.after(the set)` instead of naming this
                // app system.
                setup_presentation_system
                    .in_set(PresentationSetupSet)
                    .run_if(super::shell_host::direct_entry),
                setup_host_presentation_system
                    .in_set(PresentationSetupSet)
                    .run_if(
                        bevy::ecs::schedule::common_conditions::resource_exists::<
                            super::shell_host::AmbitionShellHosted,
                        >,
                    ),
                ambition::dev_tools::profiling::phase_mark("after_setup_presentation"),
                ambition::actors::menu::map::populate_map_rooms,
                ambition::actors::menu::map::spawn_map_menu.run_if(super::shell_host::direct_entry),
                ambition::dev_tools::profiling::phase_mark("after_map_menu_spawn"),
            )
                .chain()
                .after(setup_simulation_system)
                .after(ui_fonts::load_ui_fonts),
        )
        .add_systems(
            Update,
            (
                ambition::dialog::dialog_input,
                handle_ldtk_hot_reload,
                handle_debug_hotkeys,
                dev_tools::sync_developer_body_profile,
                ambition::actors::trace::handle_trace_hotkey,
                ambition::actors::menu::map::handle_map_menu_hotkeys,
            )
                .chain()
                .after(SandboxSet::CoreSimulation)
                .run_if(ambition::platformer::lifecycle::session_world_exists),
        )
        .add_systems(PostUpdate, restart_local_ggrs_after_hot_reload);

    // Unified menu (the one menu): install backend-agnostic menu state first,
    // then install each compiled backend independently. The backend features are
    // platform-neutral so desktop and Android stay in sync unless a build profile
    // intentionally opts out of a backend.
    crate::menu::kaleidoscope_app::install_unified_menu_shared(app);
    if ambition::menu::backend::KALEIDOSCOPE_MENU_BACKEND_ENABLED {
        crate::menu::kaleidoscope_app::install_kaleidoscope_menu_backend(app);
    }
    #[cfg(feature = "bevy_ui_menu")]
    if ambition::menu::backend::BEVY_UI_MENU_BACKEND_ENABLED {
        crate::menu::grid_backend::install_grid_unified_menu(app);
    }
}

fn install_camera_and_debug_overlay_systems(app: &mut App) {
    // The camera cluster itself (viewport publish, shake, follow, portal
    // continuity) is `ambition::host::HostCameraPlugin` (E5 step 5). What
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
        .after(ambition::host::portal::tag_portal_camera_continuity_camera);
    #[cfg(not(feature = "portal_render"))]
    let overlay = (
        debug_overlay::draw_debug_overlay,
        debug_overlay::render_debug_overlay_labels,
    )
        .chain()
        .after(camera_follow);
    app.add_systems(
        Update,
        overlay.run_if(ambition::platformer::lifecycle::session_world_exists),
    );
}

fn install_fx_and_hud_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            fx::update_particles,
            fx::update_explosions,
            fx::update_impacts,
            fx::update_speech_bubbles,
            fx::update_speech_bubble_outlines,
        )
            .chain()
            .after(debug_overlay::draw_debug_overlay)
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    )
    .add_systems(
        Update,
        (
            update_hud,
            ambition::render::rendering::sync_boss_health_bar_overlay,
            ambition::dialog::dialog_reveal_tick,
            ambition::render::cutscene::sync_cutscene_ui,
        )
            .chain()
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    )
    // Always-on *during gameplay* player HUD overlay (health / mana /
    // money bars). The title route owns no gameplay HUD authority.
    .add_systems(
        Update,
        (
            ambition::actors::avatar::regen_player_mana,
            ambition::render::hud::spawn_player_hud,
            ambition::render::hud::update_player_hud,
            // Consumes THIS frame's resolved HUD regions, so a profile that
            // reserves surround for HUD actually gets the HUD put there.
            ambition::render::hud::place_player_hud
                .after(ambition::presentation::gameplay_presentation::GameplayPresentationSet),
        )
            .chain()
            .run_if(ambition::platformer::lifecycle::session_world_exists),
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
        ambition::render::rendering::sync_portal_capture_parallax_layers
            .after(ambition::portal_presentation::PortalPresentationSet)
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    );

    app.add_systems(
        Update,
        ambition::render::rendering::sync_health_overlays
            .after(sync_visuals)
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    )
    // Idle barks fire on a 5-10s cadence while the boss is in an
    // attacking phase, so the scholar feels alive between strikes.
    .add_systems(
        Update,
        ambition_content::bosses::tick_boss_idle_barks
            .run_if(ambition::platformer::lifecycle::session_world_exists),
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
            ambition::render::rendering::gate_portal_visuals::sync_portal_sprite_visibility,
            ambition::render::rendering::gate_portal_visuals::sync_portal_sprite_animation,
            ambition::render::rendering::gate_portal_visuals::sync_portal_ring_rotation_system,
            ambition::render::rendering::gate_portal_visuals::hide_portal_loading_zone_visuals,
        )
            .after(sync_visuals)
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    )
    .add_systems(
        Update,
        ambition::render::rendering::sync_parallax_layers
            .after(camera_follow)
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    )
    // Encounter / intro LockWall visuals. Reconciles `LockWallVisual`
    // Bevy entities against the collision overlay's `gate_solids` (the
    // lock walls the gate contributors derive each frame in WorldPrep,
    // no longer mutated into the authored base) so the wall is visible
    // when an encounter slams it shut. Pinned after
    // `drive_wave_encounters` so it runs late in the frame, well
    // after the WorldPrep contributor has populated `gate_solids`.
    .add_systems(
        Update,
        ambition::render::rendering::sync_lock_wall_visuals
            .after(ambition::actors::encounter::drive_wave_encounters)
            .run_if(ambition::platformer::lifecycle::session_world_exists),
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
            ambition::render::rendering::apply_placeholder_sprites_override,
            ambition::render::rendering::apply_hide_sprites_override,
        )
            .chain()
            .after(sync_visuals)
            .after(ambition::render::rendering::morph_ball::sync_morph_ball_visual)
            .after(ambition::render::rendering::bubble_shield::sync_bubble_shield_visual)
            .after(ambition::render::rendering::projectile_visuals::sync_projectile_visuals)
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    )
    // Mouse / touch dismissal for the map menu.
    .add_systems(
        Update,
        ambition::actors::menu::map::map_menu_pointer_dismiss
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    )
    // Quest panel runs alongside the verbose HUD.
    .add_systems(
        Update,
        update_quest_panel
            .after(ambition::render::dialog_ui::DialogPresentationSet)
            .run_if(ambition::platformer::lifecycle::session_world_exists),
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
    app.add_systems(
        Update,
        (
            // One unified, kind-driven visual pass for ALL projectiles (player +
            // enemy); the charge indicator is its own player-only pass.
            ambition::render::rendering::projectile_visuals::sync_projectile_visuals
                .after(ambition::runtime::projectile_schedule::step_projectiles),
            ambition::render::rendering::projectile_visuals::sync_projectile_charge_visuals
                .after(ambition::runtime::projectile_schedule::step_projectiles),
        )
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    )
    // VFX + debris subscribe on the visible binary only. Audio's
    // subscriber lives in `add_audio_plugins` so the entire kira
    // chain stays behind the `audio` feature. Headless builds omit
    // these so the message queues drain without entity spawns or
    // audio playback.
    .add_systems(
        Update,
        (
            fx::process_fireworks_requests,
            fx::tick_firework_sequences,
            fx::process_explosion_requests,
        )
            .chain()
            .after(SandboxSet::CoreSimulation)
            .before(vfx_spawn_messages)
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    )
    .add_systems(
        Update,
        vfx_spawn_messages
            .after(fx::process_explosion_requests)
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    );
    // Live blink-destination preview ring. Reads leafwing action state to
    // know when the blink button is held, so it lives behind the `input`
    // feature alongside the other gameplay-input-driven presentation.
    #[cfg(feature = "input")]
    app.add_systems(
        Update,
        fx::update_blink_preview
            .after(SandboxSet::CoreSimulation)
            .run_if(ambition::platformer::lifecycle::session_world_exists),
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
            .after(SandboxSet::CoreSimulation)
            .run_if(ambition::platformer::lifecycle::session_world_exists),
    );
}

#[cfg(not(feature = "physics_debris"))]
pub(super) fn add_physics_debris_plugins(_app: &mut App) {}

/// Install UI-shell plugins: bevy_material_ui's styling layer. Ambition's
/// dialogue presenter is mounted by `AmbitionPresentationPlugin` and draws with Bevy's
/// core UI primitives and stays installed unconditionally; only
/// the optional plugins live behind `ui`.
///
/// `YarnSpinnerPlugin` is mounted alongside the bridge in
/// `build_sandbox_simulation_plugins` so the yarn runtime,
/// the bridge observers, and the binding registrations all spawn
/// in one place. Don't re-mount it here.
#[cfg(feature = "ui")]
pub(super) fn add_ui_plugins(app: &mut App) {
    app.add_plugins(MaterialUiPlugin);
}

#[cfg(not(feature = "ui"))]
pub(super) fn add_ui_plugins(_app: &mut App) {}

// The leafwing input bindings + the device→ControlFrame bridge live in
// `ambition::host::HostInputBindingsPlugin` (E5 step 5); the dev
// preset-input-map sync stays registered app-side (dev_runtime).

/// Register the [`TouchControlsPlugin`](ambition::touch_input::TouchControlsPlugin)
/// (`virtual_joystick` stick + on-screen action buttons). The touch overlay is
/// a VIRTUAL DEVICE: its state is exposed to leafwing as registered input
/// kinds and bound in the persistent participant's `InputMap`, so touch
/// resolves through the same bindings/context pipeline as the keyboard and
/// gamepad — there is no second `ControlFrame` writer. The adapter lives in
/// the sibling `ambition::touch_input` crate (app-thinness); the app's
/// `mobile_touch` feature forwards to `ambition::touch_input/mobile_touch`,
/// which pulls the optional `virtual_joystick` dep. Added UNCONDITIONALLY
/// whenever `mobile_touch` is compiled — no runtime boolean gates it. To rip
/// the touch controls out, remove the single `add_plugins(TouchControlsPlugin)`
/// line below. On builds compiled without `mobile_touch` this is a no-op.
#[cfg(feature = "mobile_touch")]
pub(super) fn add_mobile_touch_plugin(app: &mut App) {
    app.add_plugins(ambition::touch_input::TouchControlsPlugin);
}

#[cfg(not(feature = "mobile_touch"))]
pub(super) fn add_mobile_touch_plugin(_app: &mut App) {}

/// Install the sandbox audio subsystem. Gated by `audio` so headless
/// / RL builds drop `bevy_kira_audio` from the dep graph entirely;
/// the sim still emits `SfxMessage`s and the queue drains harmlessly
/// per the ADR 0012 seam.
#[cfg(feature = "audio")]
pub(super) fn add_audio_plugins(app: &mut App) {
    app.add_plugins(ambition::actors::audio::SandboxAudioPlugin);
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
pub struct SandboxSimulationPlugin;

impl Plugin for SandboxSimulationPlugin {
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
pub struct SandboxLdtkPlugin;

impl Plugin for SandboxLdtkPlugin {
    fn build(&self, app: &mut App) {
        add_ldtk_runtime_plugin(app);
    }
}

/// Installs all presentation-side plugins: input, audio, VFX, HUD, debug
/// overlays, and platform plugins. Visible binary only.
pub struct SandboxPresentationPlugin;

impl Plugin for SandboxPresentationPlugin {
    fn build(&self, app: &mut App) {
        add_presentation_plugins(app);
    }
}
