#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::input_systems::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::update::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;
use bevy::prelude::SystemSet;

/// Coarse simulation ordering for sandbox gameplay systems.
///
/// Keep concrete systems in small chained groups labeled with these sets instead
/// of growing one giant tuple; Bevy only implements system tuples up to a fixed
/// arity, and named sets make the update phases reviewable. Existing `.after`
/// constraints continue to own ordering until the schedule is fully normalized.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum SandboxSet {
    CoreSimulation,
    FeatureCollection,
    FeatureInteraction,
    LdtkRuntimeSpine,
    EncounterSimulation,
    Cutscene,
    GameplayEffects,
    Progression,
    ResetProcessing,
    Trace,
}

/// Register core simulation plugins, message types, and the gameplay
/// schedule. Headless and visible both call this.
pub fn add_simulation_plugins(app: &mut App) {
    // AmbitionPhysicsPlugin (Avian2D) is intentionally NOT here. Per
    // ADR 0007 Avian is secondary physics for debris/ragdoll visuals;
    // the player controller is custom via parry2d in ambition_engine.
    // Avian's collider backend needs `SceneSpawner` (from ScenePlugin in
    // DefaultPlugins), which headless doesn't have. Until Avian's debris
    // role is migrated to presentation events end-to-end (or Avian gains
    // a headless-friendly init path), it lives in
    // `add_presentation_plugins`.
    app.add_message::<SfxMessage>()
        .add_message::<VfxMessage>()
        .add_message::<DebrisBurstMessage>()
        .add_message::<PlayerDiedMessage>()
        .add_message::<crate::features::GameplayEffect>()
        .add_message::<crate::features::PlayerDamageEvent>()
        .add_message::<crate::features::DamageEvent>()
        .add_message::<crate::features::PogoBounceEvent>()
        .add_message::<crate::features::ResetRoomFeaturesEvent>()
        .add_message::<crate::features::GameplayBannerRequested>()
        .add_message::<crate::player::PlayerHealRequested>()
        .register_type::<GameMode>()
        // StartupProfiler captures wall-clock at each marked phase so a
        // PostStartup report prints "where did the first frame's
        // worth of init time go" without needing an external profiler
        // attached. See `crate::profiling` for the helper API and
        // `docs/profiling.md` for Tracy / per-system profiling.
        .insert_resource(crate::profiling::StartupProfiler::default())
        .insert_resource(crate::trace::GameplayTraceBuffer::default())
        .insert_resource(crate::features::GameplayBanner::default())
        .insert_resource(crate::features::FeatureEcsWorldOverlay::default())
        .insert_resource(crate::mechanics::MechanicsRegistry::default())
        .add_plugins(RonAssetPlugin::<data::SandboxDataSpec>::new(&["ron"]))
        .add_plugins(ae::AmbitionStateMachinePlugin)
        .add_systems(
            Startup,
            (
                crate::profiling::phase_mark("startup_begin"),
                data::load_data_asset_handle,
                crate::profiling::phase_mark("after_load_data_handle"),
                setup_simulation_system,
                crate::profiling::phase_mark("after_setup_simulation"),
            )
                .chain(),
        )
        // Final report. Runs once on the first PostStartup tick. The
        // pre-report mark captures the time between the last Startup
        // mark and PostStartup, so any heavy Startup systems we
        // didn't explicitly mark show up as a delta on the
        // "post_startup_begin" line.
        .add_systems(
            PostStartup,
            (
                crate::profiling::phase_mark("post_startup_begin"),
                crate::profiling::report_startup_phases,
            )
                .chain(),
        )
        .insert_resource(crate::projectile::PlayerProjectileState::default())
        // Encounter system. The legacy single-encounter `EncounterState`
        // resource stays for backwards-compat tests; the live
        // multi-encounter store is `EncounterRegistry`, populated
        // from LDtk by `populate_encounter_registry`.
        .insert_resource(crate::encounter::EncounterState::default())
        .insert_resource(crate::encounter::EncounterRegistry::default())
        .insert_resource(crate::encounter::SwitchActivationQueue::default())
        .insert_resource(crate::encounter::EncounterSwitchIndex::default())
        .insert_resource(crate::encounter::EncounterMusicRequest::default())
        .insert_resource(crate::rooms::RoomMusicRequest::default())
        // Sandbox save game (encounter defeat + switch state).
        // Loaded from disk by `load_save_at_startup` in the
        // presentation half so headless / RL drivers don't touch
        // disk; mutated by encounter + switch systems; written by
        // `autosave_sandbox_save` when change-detection fires.
        .insert_resource(crate::save::SandboxSave::default())
        // Quest + cutscene systems. Both are sim-side state machines
        // that read/write the save resource and surface HUD lines via
        // the encounter overlay.
        .insert_resource(crate::quest::QuestRegistry::default())
        .insert_resource(crate::cutscene::default_cutscene_library())
        .insert_resource(crate::cutscene::ActiveCutscene::default())
        .insert_resource(crate::cutscene::CutsceneTriggerQueue::default())
        .insert_resource(crate::cutscene::CutsceneAdvanceRequest::default())
        .insert_resource(crate::cutscene::RoomCutsceneBindings::defaults())
        .insert_resource(crate::boss_encounter::BossEncounterRegistry::default())
        .insert_resource(crate::map_menu::MapMenuState::default())
        .insert_resource(crate::CameraEaseState::default())
        .insert_resource(crate::CameraEaseTuning::default())
        .insert_resource(crate::rendering::CameraViewState::default())
        .insert_resource(crate::reset::SandboxResetRequested::default())
        // Core simulation chain. Keep this split into small chained groups: Bevy
        // only implements tuple system configs up to a fixed arity, and the ECS
        // feature migration added enough systems to exceed that limit if they
        // all live in one tuple. The `.after(...)` links preserve the previous
        // total ordering.
        .add_systems(
            Update,
            (
                ldtk_world::poll_ldtk_file_changes,
                crate::features::rebuild_feature_ecs_world_overlay,
                crate::features::update_ecs_hazards,
                crate::features::update_ecs_actors,
                crate::features::update_ecs_bosses,
                sandbox_update,
                crate::player::sync_player_entity_from_runtime,
                crate::features::reset_ecs_room_features,
                crate::projectile::update_projectiles,
                crate::features::apply_feature_damage_events,
            )
                .chain()
                .in_set(SandboxSet::CoreSimulation),
        )
        .add_systems(
            Update,
            (
                crate::features::collect_ecs_pickups,
                crate::player::apply_player_heal_requests,
            )
                .chain()
                .in_set(SandboxSet::FeatureCollection)
                .after(crate::features::apply_feature_damage_events),
        )
        .add_systems(
            Update,
            (
                crate::features::interact_ecs_actors_and_switches,
                crate::features::open_ecs_chests,
                crate::features::update_ecs_breakables,
                crate::features::update_ecs_falling_chests,
                crate::features::sync_ecs_switches_from_save,
                crate::encounter::rebuild_encounter_switch_index,
            )
                .chain()
                .in_set(SandboxSet::FeatureInteraction)
                .after(crate::features::collect_ecs_pickups),
        )
        .add_systems(
            Update,
            (
                ldtk_world::sync_plugin_spawned_ambition_entities,
                ldtk_world::rebuild_ldtk_runtime_spine_index,
                ldtk_world::rebuild_ldtk_runtime_solid_index,
                ldtk_world::rebuild_ldtk_runtime_one_way_index,
                ldtk_world::rebuild_ldtk_runtime_damage_index,
                ldtk_world::check_ldtk_runtime_spine_parity,
            )
                .chain()
                .in_set(SandboxSet::LdtkRuntimeSpine)
                .after(crate::encounter::rebuild_encounter_switch_index),
        )
        .add_systems(
            Update,
            (
                platforms::sync_moving_platform,
                crate::encounter::update_encounters_from_world,
                crate::encounter::sync_encounter_controller_states,
                crate::features::apply_gameplay_banner_requests,
                crate::features::tick_gameplay_banner,
            )
                .chain()
                .in_set(SandboxSet::EncounterSimulation)
                .after(ldtk_world::check_ldtk_runtime_spine_parity),
        )
        // Progression chain: cutscenes, gameplay-effect routing, boss
        // encounters, quest events, and the F3 stats editor sync. Split into
        // several chained groups so each tuple stays under Bevy's macro
        // arity limit while preserving the old drain-before-progression order.
        .add_systems(
            Update,
            (
                crate::cutscene::auto_trigger_room_cutscenes,
                crate::cutscene::drain_cutscene_triggers,
                crate::cutscene::tick_active_cutscene,
            )
                .chain()
                .in_set(SandboxSet::Cutscene)
                .after(crate::encounter::sync_encounter_controller_states),
        )
        .add_systems(
            Update,
            (
                crate::features::apply_flag_effects,
                crate::features::apply_quest_effects,
                crate::features::apply_switch_effects,
                crate::features::apply_boss_damage_effects,
                crate::features::apply_npc_strike_effects,
                crate::features::apply_gameplay_sfx_effects,
            )
                .chain()
                .in_set(SandboxSet::GameplayEffects)
                .after(crate::cutscene::tick_active_cutscene),
        )
        .add_systems(
            Update,
            (
                crate::boss_encounter::update_boss_encounters,
                crate::features::sync_ecs_actors_with_save,
                crate::features::sync_ecs_bosses_with_save,
                crate::quest::push_room_entered_quest_events,
                crate::quest::apply_quest_advance_events,
                crate::quest::grant_quest_completion_rewards,
                crate::body_mode::update_body_mode,
                crate::rooms::sync_active_room_metadata,
                crate::rooms::sync_room_music_request,
                crate::map_menu::track_room_visits,
                crate::map_menu::sync_map_from_save,
                dev_tools::sync_player_stats_with_inspector,
            )
                .chain()
                .in_set(SandboxSet::Progression)
                .after(crate::features::apply_gameplay_sfx_effects),
        )
        // Populate the encounter / quest / boss registries from the LDtk
        // project + save. These run on Update (not Startup) with their
        // existing `specs_loaded` / `initialized` short-circuits so:
        //   1. The first Update tick populates them (Startup is done by
        //      the time any Update fires, so SandboxLdtkProject + save
        //      are ready).
        //   2. The "reset sandbox" flow (`process_sandbox_reset_request`)
        //      can flip those flags back to false and the next tick
        //      repopulates from the freshly-cleared save — without us
        //      having to inline the populate logic in two places.
        // The cost when already loaded is one ResMut acquisition + one
        // bool check per registry per frame: negligible.
        .add_systems(
            Update,
            (
                crate::quest::populate_quest_registry,
                crate::boss_encounter::populate_boss_encounter_registry,
                crate::encounter::populate_encounter_registry,
            )
                .in_set(SandboxSet::Progression),
        )
        // Sandbox reset processor: consumes pending reset requests
        // (set by the pause-menu "Reset Sandbox" item or any other
        // caller). Runs after `sandbox_update` so it can't race with
        // in-flight gameplay mutations, and before the populate
        // systems on the next frame so they see the cleared
        // registries when re-running.
        .add_systems(
            Update,
            crate::reset::process_sandbox_reset_request
                .in_set(SandboxSet::ResetProcessing)
                .after(sandbox_update),
        )
        // Trace recorder lives at the simulation seam: `record_frame_system`
        // captures one frame per Update tick after `sandbox_update` has
        // resolved player state; `flush_pending_dump` writes any pending
        // dump to disk on the same tick. Both run on the simulation half
        // so headless and visible builds share trace output.
        .add_systems(
            Update,
            (
                crate::trace::record_frame_system.after(sandbox_update),
                crate::trace::flush_pending_dump.after(crate::trace::record_frame_system),
            )
                .in_set(SandboxSet::Trace),
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
        .add_systems(Update, ldtk_world::sync_ldtk_level_set);
}

/// Spawn the `LdtkWorldBundle` entity. Runs in `add_ldtk_runtime_plugin`
/// (visible binary only) after `setup_simulation_system` so the
/// `LdtkRuntimeIndex` resource is available.
pub(super) fn spawn_ldtk_world_root(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ldtk_index: Res<ldtk_world::LdtkRuntimeIndex>,
    room_set: Res<rooms::RoomSet>,
    ldtk_asset: Option<Res<ldtk_world::SandboxLdtkAsset>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
) {
    let ldtk_handle = ldtk_asset
        .as_ref()
        .map(|asset| asset.0.clone())
        .or_else(|| {
            sandbox_asset_collection
                .as_ref()
                .map(|collection| collection.ldtk_project.clone())
        })
        .unwrap_or_else(|| asset_server.load(ldtk_world::sandbox_ldtk_asset_path()));
    commands.spawn((
        bevy_ecs_ldtk::prelude::LdtkWorldBundle {
            ldtk_handle: ldtk_handle.into(),
            level_set: ldtk_index.level_set_for(&room_set.active_spec().id),
            // AMBITION_REVIEW(spatial): migrate each registered marker from
            // adapter-driven semantics to direct Ambition components.
            ..default()
        },
        ldtk_world::SandboxLdtkWorldRoot,
        Name::new("LDtk Runtime Spine Root"),
    ));
}

/// Register presentation-side plugins (input, dialogue, inspector, audio
/// and VFX subscribers, HUD, debug overlays). Visible binary only.
pub fn add_presentation_plugins(app: &mut App) {
    app.insert_resource(ClearColor(Color::srgb(0.020, 0.024, 0.035)))
        .insert_resource(windowing::DisplayModeState::default())
        .register_type::<DeveloperTools>()
        .register_type::<PlayerBodyProfile>()
        .register_type::<MovementProfile>()
        .register_type::<EditableAbilitySet>()
        .register_type::<EditableMovementTuning>()
        .register_type::<EditablePlayerStats>()
        .register_type::<SandboxFeelTuning>();

    app.add_plugins(crate::platform::PlatformPlugin);
    add_dev_tools_plugins(app);
    add_physics_debris_plugins(app);
    add_ui_plugins(app);
    add_input_plugins(app);
    add_audio_plugins(app);
    add_mobile_touch_plugin(app);

    app.add_systems(Startup, ui_fonts::load_ui_fonts);

    // Settings + sandbox-save persistence. Both load on startup and
    // autosave when the relevant resource changes (`Res::is_changed`
    // throttle). Headless drivers do not register these systems, so
    // a `cargo run --bin headless` never reads or writes user files.
    app.add_systems(
        Startup,
        (
            crate::settings::persistence::load_settings_at_startup,
            crate::save::load_save_at_startup,
        ),
    )
    .add_systems(
        Update,
        (
            crate::settings::persistence::save_settings_on_change,
            crate::save::autosave_sandbox_save,
        ),
    );

    app.insert_resource(pause_menu::PauseMenuState::default())
        .insert_resource(inventory::InventoryUiState::default())
        .insert_resource(inventory::PlayerInventory::starter())
        .add_systems(
            Startup,
            (
                pause_menu::spawn_pause_menu,
                inventory::spawn_inventory_panel,
            )
                .after(setup_simulation_system),
        )
        .add_systems(
            Update,
            (
                pause_menu::sync_pause_menu,
                inventory::sync_inventory_panel,
                crate::map_menu::sync_map_menu,
            )
                .after(sandbox_update),
        )
        .add_systems(
            Startup,
            (
                crate::profiling::phase_mark("before_setup_presentation"),
                setup_presentation_system,
                crate::profiling::phase_mark("after_setup_presentation"),
                crate::map_menu::populate_map_rooms,
                crate::map_menu::spawn_map_menu,
                crate::profiling::phase_mark("after_map_menu_spawn"),
            )
                .chain()
                .after(setup_simulation_system)
                .after(ui_fonts::load_ui_fonts),
        )
        .add_systems(
            Update,
            (
                dialog::dialog_input,
                handle_ldtk_hot_reload,
                handle_debug_hotkeys,
                dev_tools::sync_developer_body_profile,
                crate::trace::handle_trace_hotkey,
                crate::map_menu::handle_map_menu_hotkeys,
            )
                .chain()
                .after(sandbox_update),
        )
        .add_systems(
            Update,
            (
                // Spawn visual entities for encounter-spawned enemies
                // BEFORE sync_visuals reads positions for them.
                crate::rendering::spawn_dynamic_feature_visuals,
                sync_visuals,
                upgrade_enemy_sprites,
                upgrade_boss_sprites,
                animate_player,
                animate_characters,
                animate_bosses,
            )
                .chain()
                .after(crate::map_menu::handle_map_menu_hotkeys),
        )
        .add_systems(
            Update,
            (camera_follow, debug_overlay::draw_debug_overlay)
                .chain()
                .after(animate_bosses),
        )
        .add_systems(
            Update,
            (
                fx::update_particles,
                fx::update_impacts,
                fx::update_slash_previews,
                fx::update_speech_bubbles,
                windowing::window_mode_hotkeys,
            )
                .chain()
                .after(debug_overlay::draw_debug_overlay),
        )
        .add_systems(
            Update,
            (update_hud, dialog::sync_dialog_ui)
                .chain()
                .after(windowing::window_mode_hotkeys),
        )
        .add_systems(
            Update,
            crate::rendering::sync_health_overlays.after(sync_visuals),
        )
        .add_systems(
            Update,
            crate::rendering::sync_parallax_layers.after(camera_follow),
        )
        // Quest-state-driven dialog redirect: flips the live dialog
        // branch the moment the underlying world state advances past
        // the conversation's prompt (e.g. mockingbird is now dead).
        // Must run AFTER `sandbox_update` (which is where dialog
        // start happens) and BEFORE `sync_dialog_ui` (which renders
        // the chosen branch) so the redirected mode is the one drawn.
        .add_systems(
            Update,
            dialog::redirect_post_quest_dialog
                .after(sandbox_update)
                .before(dialog::sync_dialog_ui),
        )
        // Encounter-driven LockWall visuals. Reconciles `LockWallVisual`
        // Bevy entities against `world.blocks` so the wall is visible
        // for the player when an encounter slams it shut. Must run
        // after `update_encounters_from_world` (which inserts /
        // removes the backing `lockwall:*` blocks) so we observe the
        // current frame's world state, not last frame's.
        .add_systems(
            Update,
            crate::rendering::sync_lock_wall_visuals
                .after(crate::encounter::update_encounters_from_world),
        )
        // NPC spritesheet upgrade. Lives outside the big presentation
        // tuple because that tuple is already at Bevy's 20-system
        // `IntoSystemConfigs::chain()` cap. `.after(sync_visuals)`
        // preserves the ordering guarantee the chain otherwise
        // provided (FeatureVisuals must exist before we look them up).
        .add_systems(
            Update,
            crate::rendering::upgrade_npc_sprites.after(sync_visuals),
        )
        // Mouse / touch dismissal for the map menu — separate from the
        // big presentation tuple to keep that under Bevy's 20-system
        // tuple budget.
        .add_systems(Update, crate::map_menu::map_menu_pointer_dismiss)
        // Quest panel runs alongside the verbose HUD; placed in its
        // own `add_systems` so the main presentation tuple doesn't
        // overflow Bevy's 20-system tuple budget.
        .add_systems(Update, update_quest_panel.after(dialog::sync_dialog_ui))
        // Procedural morph-ball visual: build the texture once at
        // startup, spawn the sibling sprite as soon as the player
        // entity exists, and toggle visibility / position each frame
        // based on `Player::body_mode`. After `sync_visuals` so the
        // player's transform is already mirrored when we read it.
        .add_systems(Startup, crate::body_mode::build_morph_ball_sprite)
        .add_systems(
            Update,
            (
                crate::body_mode::spawn_morph_ball_visual,
                crate::body_mode::sync_morph_ball_visual,
            )
                .chain()
                .after(sync_visuals),
        )
        // Player projectile visuals: rebuild the sprite ring each tick
        // from `PlayerProjectileState::bodies`. Lives in its own
        // `add_systems` because the main visible-only chain is at the
        // tuple-arity ceiling. Must run after `update_projectiles` so
        // the body list reflects this frame's spawn / tick / collision
        // before the visuals are rebuilt — otherwise newly-fired
        // projectiles would only become visible one frame late.
        .add_systems(
            Update,
            crate::projectile::sync_projectile_visuals.after(crate::projectile::update_projectiles),
        )
        // VFX + debris subscribe on the visible binary only. Audio's
        // subscriber lives in `add_audio_plugins` so the entire kira
        // chain stays behind the `audio` feature. Headless builds omit
        // these so the message queues drain without entity spawns or
        // audio playback.
        .add_systems(Update, vfx_spawn_messages.after(sandbox_update));
    // Live blink-destination preview ring. Reads leafwing action state to
    // know when the blink button is held, so it lives behind the `input`
    // feature alongside the other gameplay-input-driven presentation.
    #[cfg(feature = "input")]
    app.add_systems(Update, fx::update_blink_preview.after(sandbox_update));
}

/// Install the egui inspector plugins. Gated by the `dev_tools` feature so
/// shipping/headless builds don't pay for `bevy-inspector-egui` /
/// `bevy_egui` in the dep graph. The inspector quick plugins require
/// EguiPlugin first; that's why both live behind the same gate.
#[cfg(feature = "dev_tools")]
pub(super) fn add_dev_tools_plugins(app: &mut App) {
    app.add_plugins(EguiPlugin::default())
        .add_plugins(
            ResourceInspectorPlugin::<DeveloperTools>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditableAbilitySet>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditableMovementTuning>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditablePlayerStats>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<SandboxFeelTuning>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(WorldInspectorPlugin::new().run_if(dev_tools::world_inspector_visible));
}

#[cfg(not(feature = "dev_tools"))]
pub(super) fn add_dev_tools_plugins(_app: &mut App) {}

/// Install the Avian2D secondary-physics plugin and its presentation-side
/// debris subscriber. Gated by `physics_debris` so headless / minimal
/// builds drop `avian2d` from the dep graph entirely. Per ADR 0007, this
/// is secondary physics for debris/ragdoll visuals only — the player
/// controller stays kinematic.
#[cfg(feature = "physics_debris")]
pub(super) fn add_physics_debris_plugins(app: &mut App) {
    app.add_plugins(physics::AmbitionPhysicsPlugin)
        .add_systems(Update, physics_spawn_debris_messages.after(sandbox_update));
}

#[cfg(not(feature = "physics_debris"))]
pub(super) fn add_physics_debris_plugins(_app: &mut App) {}

/// Install UI-shell plugins: Yarn Spinner runtime and bevy_material_ui's
/// styling layer. The dialogue overlay (`dialog::sync_dialog_ui`) draws
/// with Bevy's core UI primitives and stays installed unconditionally;
/// only the optional plugins live behind `ui`.
#[cfg(feature = "ui")]
pub(super) fn add_ui_plugins(app: &mut App) {
    app.add_plugins(dialog::yarn_spinner_plugin())
        .add_plugins(MaterialUiPlugin);
}

#[cfg(not(feature = "ui"))]
pub(super) fn add_ui_plugins(_app: &mut App) {}

/// Install the leafwing-input-manager plugin, the player-input attach
/// startup system, and the bridge that keeps `Res<ControlFrame>` in sync
/// with leafwing's `ActionState`. Gated behind `input` so headless /
/// minimal builds can drop `leafwing-input-manager` from the dep graph;
/// the sim itself reads `Res<ControlFrame>` (always-available) and is
/// agnostic to where the frame came from.
#[cfg(feature = "input")]
pub(super) fn add_input_plugins(app: &mut App) {
    app.init_resource::<MenuInputState>()
        .init_resource::<MenuControlFrame>()
        .init_resource::<PlayerDashTriggerState>()
        .add_plugins(InputManagerPlugin::<SandboxAction>::default())
        .add_systems(
            Startup,
            attach_player_input_components.after(setup_simulation_system),
        )
        // Collect semantic menu intent before gameplay input is suppressed.
        // `populate_control_frame_from_actions` may zero the sim-side
        // `ControlFrame` in UI modes, but it must not mutate leafwing's
        // `ActionState`; held keyboard/menu buttons should not become
        // `just_pressed` again on every dialog frame.
        //
        // Therefore the order is:
        // 1. read keyboard/gamepad menu actions into `MenuControlFrame`,
        // 2. read/suppress gameplay into `ControlFrame`,
        // 3. let touch folds merge into both seams before the consumers below.
        //
        // Touch fold (mobile_input plugin) runs
        // `.after(populate_control_frame_from_actions)` for gameplay and
        // `.after(populate_menu_control_frame_from_actions)` for menus, then
        // `.before(pause_menu_toggle)`, so pause / inventory / navigation see
        // keyboard, gamepad, and touch contributions in one frame.
        .add_systems(
            Update,
            (
                populate_menu_control_frame_from_actions,
                populate_control_frame_from_actions,
                apply_menu_frame_to_cutscene_request,
                dialog::dialog_pointer_input,
                pause_menu::pause_menu_toggle,
                inventory::inventory_input,
                pause_menu::pause_menu_pointer_input,
                inventory::inventory_pointer_input,
                pause_menu::pause_menu_navigate,
            )
                .chain()
                .before(sandbox_update),
        )
        .add_systems(Update, sync_preset_input_map.before(sandbox_update));
}

#[cfg(not(feature = "input"))]
pub(super) fn add_input_plugins(_app: &mut App) {}

/// Register the mobile-touch input plugin (`virtual_joystick` sticks
/// + on-screen action buttons that fold into ControlFrame). Gated
/// behind the `mobile_touch` feature; on desktop builds without the
/// feature this is a no-op.
///
/// The mobile plugin runs ALONGSIDE the desktop input pipeline --
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
    app.add_plugins(crate::mobile_input::bevy_plugin::MobileTouchPlugin);
}

#[cfg(not(feature = "mobile_touch"))]
pub(super) fn add_mobile_touch_plugin(_app: &mut App) {}

/// Install the kira audio backend, channel resources, default music
/// startup, and the SFX subscriber. Gated by `audio` so headless / RL
/// builds drop `bevy_kira_audio` and `fundsp` from the dep graph
/// entirely. The sim still emits `SfxMessage`s; without this plugin the
/// message queue just drains harmlessly per the ADR 0012 seam.
#[cfg(feature = "audio")]
pub(super) fn add_audio_plugins(app: &mut App) {
    app.add_plugins(KiraAudioPlugin)
        .init_resource::<crate::audio::RadioStationState>()
        .init_resource::<crate::audio::SfxBankHandleCache>()
        .add_audio_channel::<MusicChannel>()
        .add_audio_channel::<SfxChannel>()
        .add_audio_channel::<crate::music::MusicLayer0AChannel>()
        .add_audio_channel::<crate::music::MusicLayer1AChannel>()
        .add_audio_channel::<crate::music::MusicLayer2AChannel>()
        .add_audio_channel::<crate::music::MusicLayer3AChannel>()
        .add_audio_channel::<crate::music::MusicLayer4AChannel>()
        .add_audio_channel::<crate::music::MusicLayer5AChannel>()
        .add_audio_channel::<crate::music::MusicLayer0BChannel>()
        .add_audio_channel::<crate::music::MusicLayer1BChannel>()
        .add_audio_channel::<crate::music::MusicLayer2BChannel>()
        .add_audio_channel::<crate::music::MusicLayer3BChannel>()
        .add_audio_channel::<crate::music::MusicLayer4BChannel>()
        .add_audio_channel::<crate::music::MusicLayer5BChannel>()
        .add_systems(
            Startup,
            (
                crate::profiling::phase_mark("before_audio_init"),
                start_default_music,
                crate::music::load_music_cues,
                crate::profiling::phase_mark("after_audio_init"),
            )
                .chain()
                .after(setup_presentation_system),
        )
        .add_systems(Update, audio_play_sfx_messages.after(sandbox_update))
        // Push UserSettings.audio (master/music/sfx/mute) into the
        // Kira channels whenever the user changes the menu sliders.
        // Cheap; the system early-returns when settings are unchanged.
        .add_systems(Update, apply_audio_settings.after(sandbox_update))
        // Unified director: resolves room/encounter simple tracks and
        // adaptive cue states behind one music intent layer.
        .add_systems(
            Update,
            crate::music::drive_music_director.after(sandbox_update),
        );
}

#[cfg(not(feature = "audio"))]
pub(super) fn add_audio_plugins(_app: &mut App) {}
