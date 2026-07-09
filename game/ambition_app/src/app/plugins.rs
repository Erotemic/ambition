use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollectionApp;
use bevy_ecs_ldtk::prelude::LdtkPlugin;
#[cfg(feature = "ui")]
use bevy_material_ui::MaterialUiPlugin;

use ambition_actors::assets::loading;
use ambition_dev_tools::dev_tools::{
    self, DeveloperTools, EditableAbilitySet, EditableMovementTuning, EditablePlayerStats,
    MovementProfile, PlayerBodyProfile,
};
use ambition_actors::ldtk_world;
use ambition_actors::rooms;
use ambition_platformer_primitives::schedule::{gameplay_allowed, PresentationSetupSet, SandboxSet};
use ambition_actors::time::feel::SandboxFeelTuning;
#[cfg(feature = "physics_debris")]
use ambition_actors::world::physics;
#[cfg(feature = "physics_debris")]
use ambition_actors::world::physics::physics_spawn_debris_messages;
use ambition_inventory_ui as inventory_ui;
use ambition_render::fx::{self, vfx_spawn_messages};
use ambition_render::rendering::{camera_follow, sync_visuals};
use ambition_render::ui_fonts;

use crate::dev::debug_overlay;
use crate::host::windowing;

use super::dev_runtime::{handle_debug_hotkeys, handle_ldtk_hot_reload, sync_preset_input_map};
use super::hud::{update_hud, update_quest_panel};
use super::player_tick::{apply_home_reset_policy, sync_player_presentation};
use super::resources::init_sandbox_resources;
use super::setup_systems::{
    reload_visual_quality_assets_on_scale_change, setup_presentation_system,
    setup_simulation_system,
};
use super::sim_systems::{apply_player_reset_input_system, apply_room_replay_request_system};
use super::world_flow::{apply_room_transition_system, ensure_requested_room_parallax_system};

/// Register core simulation plugins, message types, and the gameplay
/// schedule. Headless and visible both call this.
///
/// The body is split into per-set helpers below so each section is short
/// enough to read in one screen and stays under Bevy's 20-system tuple
/// arity limit. New simulation systems should go into the matching
/// `register_*_systems` helper rather than back into this orchestrator.
pub fn add_simulation_plugins(app: &mut App) {
    // AmbitionPhysicsPlugin (Avian2D) is intentionally NOT here. Per
    // ADR 0007 Avian is secondary physics for debris/ragdoll visuals;
    // the player controller is custom via parry2d in ambition_engine_core.
    // Avian's collider backend needs `SceneSpawner` (from ScenePlugin in
    // DefaultPlugins), which headless doesn't have. Until Avian's debris
    // role is migrated to presentation events end-to-end (or Avian gains
    // a headless-friendly init path), it lives in
    // `add_presentation_plugins`.

    // The canonical simulation-phase sets + engine resources now live in
    // `ambition_runtime::SandboxSetsPlugin` (first in the engine group below).
    // Hosts still override StartingCharacter etc. by inserting BEFORE
    // `add_simulation_plugins` runs — init_resource never clobbers.

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
        app.add_plugins(ambition_actors::dialog::YarnBridgePlugin);
        app.add_plugins(ambition_actors::dialog::YarnBindingsPlugin);
    }

    // The content-free engine SIMULATION plugins (E5): the SAME
    // `PlatformerEnginePlugins` group a demo app builds on — the sandbox sets
    // + engine resources, the sim schedule, the universal brain, gravity,
    // traversal abilities, item pickups, encounters/cutscenes, feature
    // collection/interaction/effects/view-sync, room reset, traces,
    // affordances, and the combat-phase chain. Ordering is set-based, so
    // group membership does not change the resolved schedule.
    app.add_plugins(ambition_runtime::PlatformerEnginePlugins);

    // App-LOCAL residue the E5 step-5 carve deliberately left behind. The
    // engine group above registers the shared per-frame wiring (player input
    // chain, brains, possession, room-transition detect/reset, portal
    // schedule, progression); these systems wrap app-only concerns
    // (`reset_sandbox`, `load_room` + render spawns, the player clone) and
    // pin themselves into the documented ordering SLOTS between engine
    // systems (see `ambition_runtime::PlayerSchedulePlugin` /
    // `RoomTransitionSchedulePlugin` module docs).
    register_app_local_sim_systems(app);
}

/// The app-LOCAL per-frame systems, pinned into the ordering SLOTS the engine
/// chains leave for the host (E5 step 5). Everything engine-generic that used
/// to be registered here lives in `ambition_runtime::{PlayerSchedulePlugin,
/// RoomTransitionSchedulePlugin, PortalSchedulePlugin,
/// ProgressionSchedulePlugin}`.
fn register_app_local_sim_systems(app: &mut App) {
    // ── The PlayerInput gap: the Ambition reset/replay consumers ──────────
    //
    // Both call the app-only `world_flow::reset_sandbox`, and the replay
    // consumer names content (the cut-rope attempt reset) — so they stay
    // app-side, slotted after the dev-edit sync and before the input timer
    // (the exact position they held in the old inline chain).
    app.add_systems(
        Update,
        (
            apply_player_reset_input_system.run_if(gameplay_allowed),
            apply_room_replay_request_system,
        )
            .chain()
            .in_set(SandboxSet::PlayerInput)
            .after(ambition_dev_tools::sync_live_player_dev_edits_system)
            .before(ambition_actors::player::input_timer_system),
    );
    // Content dialogue-followup emitters (e.g. cut-rope "try again") run
    // before the replay consumer that drains their requests the same frame.
    // The engine anchors the slot's PHASE (PlayerInput); the consumer edge is
    // ours because the consumer is ours.
    app.configure_sets(
        Update,
        ambition_actors::session::reset::ContentDialogueFollowupSet
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
            Update,
            (
                crate::app::player_clone::request_player_clone_on_key,
                crate::app::player_clone::spawn_requested_player_clone,
            )
                .chain()
                .in_set(SandboxSet::WorldPrep),
        )
        .add_systems(
            Update,
            crate::app::player_clone::tick_player_clone_brains
                .run_if(gameplay_allowed)
                .in_set(SandboxSet::PlayerInput),
        )
        .add_systems(
            Update,
            crate::app::player_clone::sync_player_clone_transform
                .in_set(SandboxSet::PresentationSync),
        )
        .add_systems(
            Update,
            crate::app::player_clone::despawn_player_clones_on_reset
                .in_set(SandboxSet::ResetProcessing)
                .before(ambition_actors::session::reset::process_sandbox_reset_request),
        );

    // ── The PlayerSimulation gap: home reset policy + home presentation ───
    //
    // Slotted between the possession release and the hit-event drain (the
    // exact position they held in the old inline chain).
    app.add_systems(
        Update,
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
                ambition_actors::abilities::traversal::possession::release_possession_if_target_lost,
            )
            .before(ambition_actors::features::ecs::damage_apply::apply_player_hit_events),
    );

    // ── The RoomTransition gap: the transition APPLY composer ─────────────
    //
    // Detection (engine) emits `RoomTransitionRequested`; this pair consumes
    // it, runs `load_room` + the render spawns, and applies the cross-domain
    // arrival resets (the W1 composition tier); the engine's
    // `reset_ecs_room_features` then tears down per-room ECS state.
    app.add_systems(
        Update,
        (
            ensure_requested_room_parallax_system,
            apply_room_transition_system,
        )
            .chain()
            .in_set(SandboxSet::RoomTransition)
            .after(ambition_actors::rooms::detect_room_transition_system)
            .before(ambition_actors::features::reset_ecs_room_features),
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
                spawn_ldtk_world_root.after(setup_simulation_system),
            ),
        )
        .add_systems(
            Update,
            (
                ldtk_world::sync_ldtk_level_set,
                // ADR 0015 §Coordinate-frame reconciliation — keep the
                // LdtkWorldBundle's root transform aligned with the
                // current active area's centered frame. Runs every
                // frame; cheap and idempotent.
                ldtk_world::sync_ldtk_world_transform,
            ),
        );
}

/// Spawn the `LdtkWorldBundle` entity. Runs in `add_ldtk_runtime_plugin`
/// (visible binary only) after `setup_simulation_system` so the
/// `LdtkRuntimeIndex` resource is available.
pub(super) fn spawn_ldtk_world_root(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ldtk_index: Res<ldtk_world::LdtkRuntimeIndex>,
    room_set: Res<rooms::RoomSet>,
    world_assets: Option<Res<ldtk_world::LdtkWorldAssets>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
) {
    // One LdtkWorldBundle per installed WorldManifest row. bevy_ecs_ldtk's
    // asset loader is per-file; Ambition's merged JSON loader doesn't
    // propagate into the Bevy asset system, so each .ldtk file needs its
    // own bundle to get its painted tile layers rendered. The shared sync
    // system writes the same LevelSet to every bundle; only the bundle
    // whose loaded asset contains the active level iids spawns any levels
    // (iids are unique per file).
    let initial_level_set = ldtk_index.level_set_for(&room_set.active_spec().id);
    let manifest = ldtk_world::world_manifest();
    for (index, source) in manifest.worlds.iter().enumerate() {
        let handle = world_assets
            .as_ref()
            .and_then(|assets| assets.0.get(index).cloned())
            .or_else(|| {
                // Web loading-state preload covers the primary world only.
                (index == 0)
                    .then(|| {
                        sandbox_asset_collection
                            .as_ref()
                            .map(|collection| collection.ldtk_project.clone())
                    })
                    .flatten()
            })
            .unwrap_or_else(|| asset_server.load(ldtk_world::world_bevy_asset_path(source)));
        commands.spawn((
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
    }
}

/// Register presentation-side plugins (input, dialogue, inspector, audio
/// and VFX subscribers, HUD, debug overlays). Visible binary only.
pub fn add_presentation_plugins(app: &mut App) {
    // The windowed-host face (E5 step 5): leafwing input bindings + the
    // camera follow/shake cluster (+ portal camera continuity). The SAME
    // group a windowed demo adds; the app-local presentation below layers
    // Ambition's HUD/menu/dev stack on top.
    app.add_plugins(ambition_host::PlatformerHostPlugins);
    install_presentation_resources_and_subplugins(app);
    app.add_plugins((
        ambition_persistence::PersistenceSchedulePlugin,
        ambition_dev_tools::DeveloperPersistenceSchedulePlugin,
    ));
    install_menu_setup_and_hotkeys(app);
    app.add_plugins(ambition_render::rendering::PresentationVisualAnimationPlugin);
    install_camera_and_debug_overlay_systems(app);
    app.add_plugins(ambition_render::rendering::ActorNameplatePresentationPlugin);
    install_fx_and_hud_systems(app);
    install_misc_visual_sync_systems(app);
    app.add_plugins(ambition_render::rendering::PlayerVisualSchedulePlugin);
    install_projectile_and_vfx_systems(app);
}

/// Visible-side resources, registered types, and presentation child
/// plugins (input, audio, dev_tools, physics_debris, ui, mobile touch,
/// FPS overlay, font loader).
fn install_presentation_resources_and_subplugins(app: &mut App) {
    app.insert_resource(ClearColor(Color::srgb(0.020, 0.024, 0.035)))
        .init_resource::<ambition_render::quality::ResolvedVisualQuality>()
        .insert_resource(windowing::DisplayModeState::default())
        .register_type::<DeveloperTools>()
        .register_type::<PlayerBodyProfile>()
        .register_type::<MovementProfile>()
        .register_type::<EditableAbilitySet>()
        .register_type::<EditableMovementTuning>()
        .register_type::<EditablePlayerStats>()
        .register_type::<SandboxFeelTuning>()
        .register_type::<ambition_portal::PortalConvention>()
        .register_type::<ambition_portal::PortalTuning>();

    #[cfg(feature = "portal_render")]
    app.register_type::<ambition_portal_presentation::PortalVisualEffect>()
        .register_type::<ambition_portal_presentation::PortalEffectSelection>()
        .register_type::<ambition_portal_presentation::PortalCameraTransitMode>()
        .register_type::<ambition_portal_presentation::PortalCameraContinuitySelection>()
        .register_type::<ambition_portal_presentation::PortalCameraContinuityConfig>()
        .register_type::<ambition_portal_presentation::PortalCameraContinuityState>()
        .register_type::<ambition_portal_presentation::PortalViewConeConfig>();

    app.add_plugins(crate::host::platform::PlatformPlugin);
    app.add_plugins(ambition_render::screen_effects::ScreenEffectsPlugin);
    // Loads baked `*_spritesheet.ron` manifests for runtime sheet metadata.
    app.add_plugins(ambition_sprite_sheet::SheetRegistryPlugin);
    app.add_plugins(crate::dev::DevToolsPlugin);
    add_physics_debris_plugins(app);
    add_ui_plugins(app);
    // Input bindings/bridge live in `ambition_host::HostInputBindingsPlugin`
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
        (
            ambition_render::quality::sync_resolved_visual_quality,
            reload_visual_quality_assets_on_scale_change,
            ambition_render::rendering::refresh_entity_sprite_handles_on_game_assets_change,
            ambition_render::rendering::refresh_parallax_layers_on_quality_change,
        )
            .chain(),
    );
    #[cfg(feature = "portal_render")]
    app.add_systems(Update, ambition_render::quality::sync_portal_quality_budget);
}

/// Pause menu, inventory, map menu, presentation startup, dev/dialog
/// hotkeys.
fn install_menu_setup_and_hotkeys(app: &mut App) {
    // Starter item-ownership roster (the 24-item catalog default set).
    app.add_plugins(ambition_content::items::AmbitionItemRosterPlugin);
    app.insert_resource(inventory_ui::InventoryUiState::default())
        .init_resource::<ambition_actors::items::persist::InventoryRestored>()
        // Persist the inventory + wallet across save/load: restore the saved set
        // once the player exists, then mirror live changes back into the save
        // (the existing autosave writes the dirtied save to disk).
        .add_systems(
            Update,
            (
                ambition_actors::items::persist::restore_inventory_from_save,
                ambition_actors::items::persist::persist_inventory_to_save,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (ambition_actors::menu::map::sync_map_menu,).after(SandboxSet::CoreSimulation),
        )
        .add_systems(
            Startup,
            (
                ambition_dev_tools::profiling::phase_mark("before_setup_presentation"),
                // `PresentationSetupSet` is the machinery-facing label for
                // this slot: audio init (and any future machinery startup
                // work) orders `.after(the set)` instead of naming this
                // app system.
                setup_presentation_system.in_set(PresentationSetupSet),
                ambition_dev_tools::profiling::phase_mark("after_setup_presentation"),
                ambition_actors::menu::map::populate_map_rooms,
                ambition_actors::menu::map::spawn_map_menu,
                ambition_dev_tools::profiling::phase_mark("after_map_menu_spawn"),
            )
                .chain()
                .after(setup_simulation_system)
                .after(ui_fonts::load_ui_fonts),
        )
        .add_systems(
            Update,
            (
                ambition_dialog::dialog_input,
                handle_ldtk_hot_reload,
                handle_debug_hotkeys,
                dev_tools::sync_developer_body_profile,
                ambition_actors::trace::handle_trace_hotkey,
                ambition_actors::menu::map::handle_map_menu_hotkeys,
            )
                .chain()
                .after(SandboxSet::CoreSimulation),
        );

    // Unified menu (the one menu): install backend-agnostic menu state first,
    // then install each compiled backend independently. The backend features are
    // platform-neutral so desktop and Android stay in sync unless a build profile
    // intentionally opts out of a backend.
    crate::menu::kaleidoscope_app::install_unified_menu_shared(app);
    if ambition_menu::backend::KALEIDOSCOPE_MENU_BACKEND_ENABLED {
        crate::menu::kaleidoscope_app::install_kaleidoscope_menu_backend(app);
    }
    #[cfg(feature = "bevy_ui_menu")]
    if ambition_menu::backend::BEVY_UI_MENU_BACKEND_ENABLED {
        crate::menu::grid_backend::install_grid_unified_menu(app);
    }
}

fn install_camera_and_debug_overlay_systems(app: &mut App) {
    // The camera cluster itself (viewport publish, shake, follow, portal
    // continuity) is `ambition_host::HostCameraPlugin` (E5 step 5). What
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
        .after(ambition_host::portal::tag_portal_camera_continuity_camera);
    #[cfg(not(feature = "portal_render"))]
    let overlay = (
        debug_overlay::draw_debug_overlay,
        debug_overlay::render_debug_overlay_labels,
    )
        .chain()
        .after(camera_follow);
    app.add_systems(Update, overlay);
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
            windowing::window_mode_hotkeys,
        )
            .chain()
            .after(debug_overlay::draw_debug_overlay),
    )
    .add_systems(
        Update,
        (
            update_hud,
            ambition_render::rendering::sync_boss_health_bar_overlay,
            ambition_dialog::dialog_reveal_tick,
            ambition_render::dialog_ui::sync_dialog_ui,
            ambition_render::cutscene::sync_cutscene_ui,
        )
            .chain()
            .after(windowing::window_mode_hotkeys),
    )
    // Always-on player HUD overlay (health / mana / money bars). Spawns once
    // a player exists, then mirrors the sim-built `PlayerHudFacts` each frame.
    // Mana regen is a gameplay system (sim dt) — it lives SIM-side now
    // (E4: no sim mutator in presentation), scheduled here at its old slot.
    .add_systems(
        Update,
        (
            ambition_actors::player::regen_player_mana,
            ambition_render::hud::spawn_player_hud,
            ambition_render::hud::update_player_hud,
        )
            .chain(),
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
        ambition_render::rendering::sync_portal_capture_parallax_layers
            .after(ambition_portal_presentation::PortalPresentationSet),
    );

    app.add_systems(
        Update,
        ambition_render::rendering::sync_health_overlays.after(sync_visuals),
    )
    // Idle barks fire on a 5-10s cadence while the boss is in an
    // attacking phase, so the scholar feels alive between strikes.
    .add_systems(Update, ambition_content::bosses::tick_boss_idle_barks)
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
            ambition_render::rendering::gate_portal_visuals::sync_portal_sprite_visibility,
            ambition_render::rendering::gate_portal_visuals::sync_portal_sprite_animation,
            ambition_render::rendering::gate_portal_visuals::sync_portal_ring_rotation_system,
            ambition_render::rendering::gate_portal_visuals::hide_portal_loading_zone_visuals,
        )
            .after(sync_visuals),
    )
    .add_systems(
        Update,
        ambition_render::rendering::sync_parallax_layers.after(camera_follow),
    )
    // Encounter / intro LockWall visuals. Reconciles `LockWallVisual`
    // Bevy entities against the collision overlay's `gate_solids` (the
    // lock walls the gate contributors derive each frame in WorldPrep,
    // no longer mutated into the authored base) so the wall is visible
    // when an encounter slams it shut. Pinned after
    // `update_encounters_from_world` so it runs late in the frame, well
    // after the WorldPrep contributor has populated `gate_solids`.
    .add_systems(
        Update,
        ambition_render::rendering::sync_lock_wall_visuals
            .after(ambition_actors::encounter::update_encounters_from_world),
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
            ambition_render::rendering::apply_placeholder_sprites_override,
            ambition_render::rendering::apply_hide_sprites_override,
        )
            .chain()
            .after(sync_visuals)
            .after(ambition_render::rendering::morph_ball::sync_morph_ball_visual)
            .after(ambition_render::rendering::bubble_shield::sync_bubble_shield_visual)
            .after(ambition_render::rendering::projectile_visuals::sync_projectile_visuals),
    )
    // Mouse / touch dismissal for the map menu.
    .add_systems(Update, ambition_actors::menu::map::map_menu_pointer_dismiss)
    // Quest panel runs alongside the verbose HUD.
    .add_systems(
        Update,
        update_quest_panel.after(ambition_render::dialog_ui::sync_dialog_ui),
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
            ambition_render::rendering::projectile_visuals::sync_projectile_visuals
                .after(ambition_runtime::projectile_schedule::step_projectiles),
            ambition_render::rendering::projectile_visuals::sync_projectile_charge_visuals
                .after(ambition_runtime::projectile_schedule::step_projectiles),
        ),
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
            .before(vfx_spawn_messages),
    )
    .add_systems(
        Update,
        vfx_spawn_messages.after(fx::process_explosion_requests),
    );
    // Live blink-destination preview ring. Reads leafwing action state to
    // know when the blink button is held, so it lives behind the `input`
    // feature alongside the other gameplay-input-driven presentation.
    #[cfg(feature = "input")]
    app.add_systems(
        Update,
        fx::update_blink_preview.after(SandboxSet::CoreSimulation),
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
        physics_spawn_debris_messages.after(SandboxSet::CoreSimulation),
    );
}

#[cfg(not(feature = "physics_debris"))]
pub(super) fn add_physics_debris_plugins(_app: &mut App) {}

/// Install UI-shell plugins: bevy_material_ui's styling layer. The
/// dialogue overlay (`ambition_render::dialog_ui::sync_dialog_ui`) draws with Bevy's
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

// The leafwing input bindings + the device→ControlFrame bridge moved to
// `ambition_host::HostInputBindingsPlugin` (E5 step 5). The touch fold below
// still runs ALONGSIDE it (both write the same `ControlFrame` seam), and the
// dev preset-input-map sync stays registered app-side (dev_runtime).

/// Register the [`TouchControlsPlugin`](ambition_touch_input::TouchControlsPlugin)
/// (`virtual_joystick` sticks + on-screen action buttons that fold into
/// ControlFrame). The touch adapter lives in the sibling `ambition_touch_input`
/// crate now (app-thinness); the app's `mobile_touch` feature forwards to
/// `ambition_touch_input/mobile_touch`, which pulls the optional
/// `virtual_joystick` dep. Added UNCONDITIONALLY whenever `mobile_touch` is
/// compiled — no runtime boolean gates it. To rip the touch controls out, remove
/// the single `add_plugins(TouchControlsPlugin)` line below. On builds compiled
/// without `mobile_touch` this is a no-op.
///
/// The touch plugin runs ALONGSIDE the desktop input pipeline --
/// both write into the same `ControlFrame` resource, with the
/// mobile-side write happening after the desktop one in this
/// session's chain. On a phone, the desktop pipeline produces
/// neutral output (no keyboard / gamepad); on desktop, the mobile
/// stick UI is invisible without touch input, so neither path
/// stomps the other in practice. A future polish pass can detect
/// the active input source (touch vs keyboard) and skip the
/// inactive folder.
#[cfg(feature = "mobile_touch")]
pub(super) fn add_mobile_touch_plugin(app: &mut App) {
    app.add_plugins(ambition_touch_input::TouchControlsPlugin);
}

#[cfg(not(feature = "mobile_touch"))]
pub(super) fn add_mobile_touch_plugin(_app: &mut App) {}

/// Install the sandbox audio subsystem. Gated by `audio` so headless
/// / RL builds drop `bevy_kira_audio` from the dep graph entirely;
/// the sim still emits `SfxMessage`s and the queue drains harmlessly
/// per the ADR 0012 seam.
#[cfg(feature = "audio")]
pub(super) fn add_audio_plugins(app: &mut App) {
    app.add_plugins(ambition_actors::audio::SandboxAudioPlugin);
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
        // `init_sandbox_resources` installs the named boss roster first (it
        // resolves `BossBehaviorProfile::from_data` while populating the boss
        // encounter registry).
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
