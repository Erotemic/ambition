//! Visible-binary App-builder helpers and gameplay systems shared between
//! `src/main.rs` (visible) and `src/headless.rs` (run_headless).
//!
//! Slice 5 of ADR 0012's events refactor moved this code out of `main.rs`
//! into the library so the headless binary can drive the same gameplay loop
//! (`sandbox_update` and friends) without InputPlugin / RenderPlugin /
//! Kira audio. The visible binary's `fn main()` is now a thin shim that
//! calls `run_visible`, which composes:
//!
//! * `init_sandbox_resources`: parse + validate the embedded LDtk world,
//!   build the `RoomSet`, and insert sim resources both halves need.
//! * `add_simulation_plugins`: register sim plugins, messages, and the
//!   gameplay schedule. Headless calls this; visible calls this.
//! * `add_presentation_plugins`: register DefaultPlugins-derived rendering,
//!   inspector overlays, audio/VFX/debris subscribers, HUD, debug overlays,
//!   and input-driven systems. Visible calls this; headless does not.

use ambition_engine as ae;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResizeConstraints, WindowResolution};
use bevy_asset_loader::asset_collection::AssetCollectionApp;
use bevy_common_assets::ron::RonAssetPlugin;
use bevy_ecs_ldtk::prelude::{LdtkPlugin, LdtkSettings, LevelBackground};
use bevy_inspector_egui::{
    bevy_egui::EguiPlugin,
    quick::{ResourceInspectorPlugin, WorldInspectorPlugin},
};
use bevy_kira_audio::prelude::{
    AudioApp, AudioPlugin as KiraAudioPlugin, AudioSource as KiraAudioSource,
};
use bevy_material_ui::MaterialUiPlugin;
use leafwing_input_manager::prelude::{ActionState, InputManagerPlugin, InputMap};

use crate::audio::{
    audio_play_sfx_messages, start_default_music, MusicChannel, SfxChannel, SfxMessage,
};
use crate::config::{WINDOW_H, WINDOW_W};
use crate::data;
use crate::debug_overlay;
use crate::dev_tools::{
    self, DeveloperTools, EditableAbilitySet, EditableMovementTuning, SandboxFeelTuning,
};
use crate::dialog;
use crate::features;
use crate::fx::{self, vfx_spawn_messages, ParticleKind, VfxMessage};
use crate::game_assets::{self, GameAssetConfig};
use crate::game_mode::GameMode;
use crate::input::{ControlFrame, SandboxAction, GAMEPAD_MAP};
use crate::inventory;
use crate::ldtk_world;
use crate::loading;
use crate::pause_menu;
use crate::physics::{self, physics_spawn_debris_messages, DebrisBurstMessage};
use crate::platforms;
use crate::rendering::{
    animate_bosses, animate_enemies, animate_player, camera_follow, spawn_room_visuals,
    sync_visuals, upgrade_boss_sprites, upgrade_enemy_sprites, HudText, PlayerVisual, RoomVisual,
    SceneEntities,
};
use crate::rooms;
use crate::setup;
use crate::windowing;
use crate::{GameWorld, SandboxRuntime};

/// Bundled `MessageWriter`s for the sim → presentation event channel.
///
/// `sandbox_update` outgrew Bevy's 16-system-param limit when individual
/// writers were passed; bundling them in a single `SystemParam` keeps the
/// sim system signature within budget while preserving the Vec-collector →
/// drain pattern documented in `docs/events_refactor_plan.md`. Adding new
/// channels to the sim → presentation seam happens here, not on the
/// `sandbox_update` signature.
#[derive(SystemParam)]
pub struct SandboxEventWriters<'w> {
    sfx: MessageWriter<'w, SfxMessage>,
    vfx: MessageWriter<'w, VfxMessage>,
    debris: MessageWriter<'w, DebrisBurstMessage>,
}

/// Build + run the visible Bevy app. The thin `fn main()` shim in
/// `src/main.rs` calls this.
pub fn run_visible() {
    let asset_config = GameAssetConfig::from_args();
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Ambition - Tangent Space Sandbox (Bevy)".into(),
            resolution: WindowResolution::new(WINDOW_W, WINDOW_H),
            resizable: true,
            resize_constraints: WindowResizeConstraints {
                min_width: 640.0,
                min_height: 360.0,
                ..default()
            },
            ..default()
        }),
        ..default()
    }));
    // DefaultPlugins installs StatesPlugin, so initialize GameMode after it.
    app.init_state::<GameMode>();
    app.insert_resource(asset_config);
    init_sandbox_resources(&mut app);
    add_simulation_plugins(&mut app);
    add_ldtk_runtime_plugin(&mut app);
    add_presentation_plugins(&mut app);
    app.run();
}

/// Parse + validate the embedded LDtk world, build the `RoomSet`, and insert
/// the sim-required resources both visible and headless binaries need.
///
/// Both binaries call this after registering Bevy's plugin foundation
/// (DefaultPlugins or MinimalPlugins + AssetPlugin + StatesPlugin +
/// `init_state::<GameMode>`) and before the App-builder helpers.
///
/// Exits with status 2 on LDtk validation errors — invalid sandbox content
/// is a hard error per the LDtk authoring rules (see ADR 0009 + LDtk
/// authoring memory).
pub fn init_sandbox_resources(app: &mut App) {
    let sandbox_data = data::SandboxDataSpec::load_embedded();
    let ldtk_project = ldtk_world::LdtkProject::load_embedded();
    let ldtk_report = ldtk_project.validate();
    ldtk_report.print_to_stderr();
    let editable_abilities = EditableAbilitySet::from(sandbox_data.abilities);
    let editable_tuning = EditableMovementTuning::from(sandbox_data.tuning);
    let room_set = match ldtk_project.to_room_set() {
        Ok(room_set) => room_set,
        Err(errors) => {
            eprintln!("embedded LDtk world failed validation; fix crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk before running:");
            for error in &errors {
                eprintln!("  - {error}");
            }
            std::process::exit(2);
        }
    };
    let ldtk_index = ldtk_world::LdtkRuntimeIndex::from_project(
        &ldtk_project,
        room_set.active_spec().id.clone(),
    );
    let active_world = room_set.active_world().clone();

    app.insert_resource(GameWorld(active_world))
        .insert_resource(room_set)
        .insert_resource(ldtk_index)
        .insert_resource(ldtk_world::LdtkHotReloadState::from_current_file())
        .insert_resource(ldtk_world::LdtkRuntimeSpineStats::default())
        .insert_resource(ldtk_world::LdtkRuntimeSpineIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeSolidIndex::default())
        // PhysicsSandboxSettings is read by setup_simulation_system; on the
        // visible binary AmbitionPhysicsPlugin re-inserts the default value
        // (harmless — same default), but headless does not load that plugin
        // (it depends on ScenePlugin), so the resource must be available
        // up front.
        .insert_resource(physics::PhysicsSandboxSettings::default())
        .insert_resource(LdtkSettings {
            // Ambition still renders runtime rooms for now; let bevy_ecs_ldtk
            // own level/entity lifecycle without also drawing LDtk background
            // rectangles behind every level.
            level_background: LevelBackground::Nonexistent,
            ..default()
        })
        .insert_resource(sandbox_data)
        .insert_resource(DeveloperTools::default())
        .insert_resource(SandboxFeelTuning::default())
        .insert_resource(editable_abilities)
        .insert_resource(editable_tuning);
}

/// Register core simulation plugins, message types, and the gameplay
/// schedule. Headless and visible both call this. Bevy `LdtkPlugin` is
/// **not** included here (see `add_ldtk_runtime_plugin`) because its
/// tile-rendering pipeline requires the `RenderApp` sub-app, which only
/// exists when DefaultPlugins (or RenderPlugin) is installed. Without
/// LdtkPlugin the runtime-spine systems still run as no-ops; sandbox
/// gameplay drives the JSON-derived `GameWorld` collision world that
/// `init_sandbox_resources` already populated.
///
/// Caller is responsible for installing the appropriate Bevy plugin
/// foundation first (DefaultPlugins for visible, MinimalPlugins +
/// AssetPlugin + ImagePlugin + TransformPlugin + StatesPlugin for
/// headless) and calling `init_sandbox_resources` to populate the
/// LDtk-derived resources.
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
        .register_type::<GameMode>()
        .add_plugins(RonAssetPlugin::<data::SandboxDataSpec>::new(&["ron"]))
        .add_plugins(ae::AmbitionStateMachinePlugin::default())
        .add_systems(
            Startup,
            (data::load_data_asset_handle, setup_simulation_system).chain(),
        )
        .add_systems(
            Update,
            (
                ldtk_world::poll_ldtk_file_changes,
                sandbox_update,
                ldtk_world::sync_plugin_spawned_ambition_entities,
                ldtk_world::rebuild_ldtk_runtime_spine_index,
                ldtk_world::rebuild_ldtk_runtime_solid_index,
                platforms::sync_moving_platform,
            )
                .chain(),
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
fn spawn_ldtk_world_root(
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
        .unwrap_or_else(|| asset_server.load(ldtk_world::SANDBOX_LDTK_ASSET));
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
        .add_plugins(KiraAudioPlugin)
        .add_audio_channel::<MusicChannel>()
        .add_audio_channel::<SfxChannel>()
        // The inspector quick plugins require EguiPlugin to be registered first.
        .add_plugins(EguiPlugin::default())
        .add_plugins(InputManagerPlugin::<SandboxAction>::default())
        .add_plugins(dialog::yarn_spinner_plugin())
        .add_plugins(MaterialUiPlugin)
        // Avian2D — secondary physics for debris/ragdoll visuals (ADR 0007).
        // Visible-binary only because Avian's collider backend needs
        // ScenePlugin's SceneSpawner.
        .add_plugins(physics::AmbitionPhysicsPlugin)
        .register_type::<DeveloperTools>()
        .register_type::<EditableAbilitySet>()
        .register_type::<EditableMovementTuning>()
        .register_type::<SandboxFeelTuning>()
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
            ResourceInspectorPlugin::<SandboxFeelTuning>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(WorldInspectorPlugin::new().run_if(dev_tools::world_inspector_visible))
        .insert_resource(pause_menu::PauseMenuState::default())
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
                pause_menu::pause_menu_toggle,
                inventory::inventory_input,
                pause_menu::pause_menu_navigate,
            )
                .chain()
                .before(sandbox_update),
        )
        .add_systems(
            Update,
            (pause_menu::sync_pause_menu, inventory::sync_inventory_panel).after(sandbox_update),
        )
        .add_systems(
            Startup,
            (
                setup_presentation_system.after(setup_simulation_system),
                start_default_music.after(setup_presentation_system),
            ),
        )
        .add_systems(
            Update,
            (
                dialog::dialog_input,
                handle_ldtk_hot_reload,
                handle_debug_hotkeys,
                sync_visuals,
                upgrade_enemy_sprites,
                upgrade_boss_sprites,
                animate_player,
                animate_enemies,
                animate_bosses,
                camera_follow,
                debug_overlay::draw_debug_overlay,
                fx::update_particles,
                fx::update_impacts,
                fx::update_slash_previews,
                windowing::window_mode_hotkeys,
                update_hud,
                dialog::sync_dialog_ui,
            )
                .chain()
                .after(sandbox_update),
        )
        .add_systems(
            Update,
            crate::rendering::sync_health_overlays.after(sync_visuals),
        )
        // Audio + VFX + debris subscribe on the visible binary only.
        // Headless builds omit these so the message queues drain without
        // entity spawns or audio playback. The `.after` constraints pin
        // presentation to the same frame the simulation emitted the message.
        .add_systems(Update, audio_play_sfx_messages.after(sandbox_update))
        .add_systems(Update, vfx_spawn_messages.after(sandbox_update))
        .add_systems(Update, physics_spawn_debris_messages.after(sandbox_update));
}

// `GameWorld`, `SandboxRuntime`, and the time-scale ramp helper `move_toward`
// have moved to `crate::lib` (`ambition_sandbox`) so both binaries can share
// them. They are re-imported above through `use ambition_sandbox::*;`.

/// Sim-only startup. Calls `crate::setup::simulation_world` to spawn the
/// LdtkWorldBundle, build the SandboxRuntime resource, and spawn the player
/// entity (with gameplay-essential components but no Sprite). Inserts
/// SceneEntities with `hud: Entity::PLACEHOLDER`; the presentation startup
/// system later overwrites that with the real HUD entity.
fn setup_simulation_system(
    mut commands: Commands,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data_asset: Option<Res<data::SandboxDataAsset>>,
    ldtk_asset: Option<Res<ldtk_world::SandboxLdtkAsset>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
    asset_server: Res<AssetServer>,
    ldtk_index: Res<ldtk_world::LdtkRuntimeIndex>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    sandbox_data: Res<data::SandboxDataSpec>,
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
) {
    let _player = setup::simulation_world(
        &mut commands,
        setup::SimulationSetup {
            world: &world,
            room_set: &room_set,
            ldtk_index: &ldtk_index,
            sandbox_data: &sandbox_data,
            editable_abilities: &editable_abilities,
            editable_tuning: &editable_tuning,
            physics_settings: *physics_settings,
            sandbox_data_asset: sandbox_data_asset.as_deref(),
            ldtk_asset: ldtk_asset.as_deref(),
            sandbox_asset_collection: sandbox_asset_collection.as_deref(),
            asset_server: &asset_server,
        },
    );
}

/// Presentation startup. Runs after `setup_simulation_system` so the
/// SceneEntities resource (with player Entity) is visible. Adds the
/// player's Sprite, spawns Camera2d, room visuals, HUD text, generated
/// Kira audio library, and overwrites SceneEntities to fill in the HUD
/// entity.
fn setup_presentation_system(
    mut commands: Commands,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data: Res<data::SandboxDataSpec>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    scene_entities: Res<SceneEntities>,
) {
    let game_assets =
        game_assets::load_game_assets(&asset_config, &asset_server, &mut atlas_layouts);
    setup::presentation_world(
        &mut commands,
        &mut audio_sources,
        setup::PresentationSetup {
            world: &world,
            room_set: &room_set,
            sandbox_data: &sandbox_data,
            physics_settings: *physics_settings,
            game_assets: &game_assets,
        },
        scene_entities.player,
    );
    commands.insert_resource(game_assets);
}

fn sandbox_update(
    mut commands: Commands,
    time: Res<Time>,
    mut world: ResMut<GameWorld>,
    mut room_set: ResMut<rooms::RoomSet>,
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    feel_tuning: Res<SandboxFeelTuning>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut runtime: ResMut<SandboxRuntime>,
    entities: Res<SceneEntities>,
    mut event_writers: SandboxEventWriters,
    mut player_input: Query<&mut ActionState<SandboxAction>, With<PlayerVisual>>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    game_assets: Option<Res<crate::game_assets::GameAssets>>,
) {
    // Per-frame Vec collectors for the sim → presentation event channels.
    // Helpers append messages as the gameplay loop runs; we drain into the
    // MessageWriters at every return point so the audio/fx/physics-debris
    // subscribers see them this frame. Bevy 0.18 buffered events use the
    // Message API (see feedback memory + ADR 0012).
    let mut sfx: Vec<SfxMessage> = Vec::new();
    let mut vfx: Vec<VfxMessage> = Vec::new();
    let mut debris: Vec<DebrisBurstMessage> = Vec::new();
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;
    let physics_settings = runtime.physics_settings;
    dev_tools::sync_live_ability_edits(&mut runtime, editable_abilities.as_engine(), tuning);

    // Debug hotkeys (preset cycling, F1/F2/F3/F4/F5 toggles) live in a
    // separate presentation-side system that runs `.before(sandbox_update)`.
    // sandbox_update no longer reads `Res<ButtonInput<KeyCode>>` so it can run
    // on the headless App-builder track without an InputPlugin.

    let mut controls = ControlFrame::default();
    if let Ok(action_state) = player_input.get(entities.player) {
        controls = if mode.get().allows_gameplay() {
            ControlFrame::read_gameplay(action_state)
        } else {
            ControlFrame::read_menu(action_state)
        };
    }

    if matches!(mode.get(), GameMode::Dialogue) {
        if let Ok(mut action_state) = player_input.get_mut(entities.player) {
            action_state.reset_all();
        }
        let frame_dt = time.delta_secs();
        runtime.time_scale = 0.0;
        runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
        runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
        return;
    }

    // Pause/resume toggling has moved to `pause_menu::pause_menu_toggle`,
    // which runs `.before(sandbox_update)`. The `start_pressed` flag is
    // still read here for compile-completeness; the pause logic itself
    // lives in the pause menu so it can drive a real overlay.
    let _ = controls.start_pressed;

    let frame_dt = time.delta_secs();
    if !mode.get().allows_gameplay() {
        // Pause, dialogue, and transition modes intentionally do not consume
        // gameplay inputs or advance simulation timers. Developer hotkeys above
        // and HUD sync below remain responsive because those systems are outside
        // this early return.
        runtime.time_scale = 0.0;
        runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
        runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
        return;
    }

    runtime.room_transition_cooldown = (runtime.room_transition_cooldown - frame_dt).max(0.0);
    runtime.damage_invuln_timer = (runtime.damage_invuln_timer - frame_dt).max(0.0);
    runtime.hitstun_timer = (runtime.hitstun_timer - frame_dt).max(0.0);
    controls.fast_fall_pressed =
        runtime.register_down_tap(controls.down_pressed, frame_dt, feel.down_double_tap_window);
    let door_double_tap_up =
        runtime.register_up_tap(controls.up_pressed, frame_dt, feel.up_double_tap_window);
    runtime.hitstop_timer = (runtime.hitstop_timer - frame_dt).max(0.0);

    if controls.reset_pressed {
        reset_sandbox(&world.0, &mut sfx, &mut vfx, &mut runtime, tuning, feel);
        event_writers.sfx.write_batch(sfx.drain(..));
        event_writers.vfx.write_batch(vfx.drain(..));
        event_writers.debris.write_batch(debris.drain(..));
        return;
    } else {
        // Two-clock update:
        // - control_dt is real time for responsive inputs and precision-blink aim;
        // - sim_dt is scaled game time for gravity, platforms, enemies, particles.
        let control_frame = controls_for_hitstun(controls, feel, runtime.hitstun_timer);
        let input = control_frame.engine_input(frame_dt);
        let control_world = features::world_with_sandbox_solids(
            &world.0,
            &runtime.moving_platform,
            &runtime.features,
        );
        let control_events = ae::update_player_control_with_tuning(
            &control_world,
            &mut runtime.player,
            input,
            frame_dt,
            tuning,
        );
        if control_events.reset {
            reset_sandbox(&world.0, &mut sfx, &mut vfx, &mut runtime, tuning, feel);
            event_writers.sfx.write_batch(sfx.drain(..));
            event_writers.vfx.write_batch(vfx.drain(..));
            event_writers.debris.write_batch(debris.drain(..));
            return;
        }
        // Damage breakable pogo orbs the player just bounced off. The
        // engine reports orb AABBs; the sandbox matches them against
        // breakables flagged `pogo_refresh` and routes hit/break events
        // through the standard feature pipeline.
        for &orb_aabb in &control_events.pogo_hits {
            let feature_events = runtime.features.on_pogo_bounce(orb_aabb, 1);
            handle_feature_events(
                &mut sfx,
                &mut vfx,
                &mut debris,
                &feature_events,
                runtime.player.pos,
            );
        }
        handle_player_events(&mut sfx, &mut vfx, &mut runtime, control_events, None);

        runtime.update_time_scale(frame_dt, feel);
        let sim_dt = sandbox_dt(&runtime, frame_dt);

        let platform_delta = runtime.moving_platform.update(sim_dt);
        if runtime.moving_platform.is_riding(&runtime.player) {
            runtime.player.pos += platform_delta;
        }
        let collision_world = features::world_with_sandbox_solids(
            &world.0,
            &runtime.moving_platform,
            &runtime.features,
        );

        let was_grounded = runtime.player.on_ground;
        let sim_events = ae::update_player_simulation_with_tuning(
            &collision_world,
            &mut runtime.player,
            input,
            sim_dt,
            tuning,
        );
        if sim_events.reset {
            reset_sandbox(&world.0, &mut sfx, &mut vfx, &mut runtime, tuning, feel);
            event_writers.sfx.write_batch(sfx.drain(..));
            event_writers.vfx.write_batch(vfx.drain(..));
            event_writers.debris.write_batch(debris.drain(..));
            return;
        }
        handle_player_events(
            &mut sfx,
            &mut vfx,
            &mut runtime,
            sim_events,
            Some(was_grounded),
        );
    }

    // Context interaction is deliberately separate from raw up movement.
    // Up is too valuable for platforming/flight/aiming to double as a one-tap
    // door or NPC trigger, so doors/NPCs/chests accept either the dedicated
    // Interact action or a deliberate double-tap-up gesture.
    let raw_interact_pressed = if runtime.hitstun_timer > 0.0 {
        false
    } else {
        controls.interact_pressed || door_double_tap_up
    };
    controls.interact_pressed =
        runtime.buffered_interact(raw_interact_pressed, frame_dt, feel.interaction_buffer_time);

    let feature_dt = sandbox_dt(&runtime, frame_dt);
    let feature_world =
        features::world_with_sandbox_solids(&world.0, &runtime.moving_platform, &runtime.features);
    let feature_player = runtime.player.clone();
    let player_vulnerable = runtime.damage_invuln_timer <= 0.0;
    let feature_events = runtime.features.update(
        &feature_world,
        &feature_player,
        controls.interact_pressed,
        player_vulnerable,
        feel.feature_combat_tuning(),
        feature_dt,
    );
    let feature_reset = feature_events.reset_player;
    let feature_interaction_consumed = feature_events.consumed_interaction;
    let feature_damaged_player = !feature_events.player_damage.is_empty();
    handle_feature_events(
        &mut sfx,
        &mut vfx,
        &mut debris,
        &feature_events,
        runtime.player.pos,
    );
    handle_player_heal_events(&mut runtime, &feature_events);
    handle_player_damage_events(
        &world.0,
        &mut sfx,
        &mut vfx,
        &mut runtime,
        &feature_events,
        tuning,
        feel,
    );
    if !feature_damaged_player {
        runtime.remember_safe_player_position();
    }
    if feature_interaction_consumed {
        runtime.clear_interact_buffer();
    }
    if let Some(request) = &feature_events.dialogue_request {
        runtime
            .dialogue
            .start(&request.dialogue_id, &request.npc_name);
        runtime.clear_interact_buffer();
        runtime.hitstop_timer = 0.0;
        next_mode.set(GameMode::Dialogue);
        event_writers.sfx.write_batch(sfx.drain(..));
        event_writers.vfx.write_batch(vfx.drain(..));
        event_writers.debris.write_batch(debris.drain(..));
        return;
    }
    if feature_reset {
        reset_sandbox(&world.0, &mut sfx, &mut vfx, &mut runtime, tuning, feel);
        event_writers.sfx.write_batch(sfx.drain(..));
        event_writers.vfx.write_batch(vfx.drain(..));
        event_writers.debris.write_batch(debris.drain(..));
        return;
    }

    if runtime.room_transition_cooldown <= 0.0 {
        if let Some(zone) =
            room_set.transition_for_player(&runtime.player, controls.interact_pressed)
        {
            runtime.clear_interact_buffer();
            load_room(
                &mut commands,
                &mut sfx,
                &mut vfx,
                &mut runtime,
                &mut *world,
                &mut *room_set,
                &room_visuals,
                zone,
                tuning,
                feel,
                physics_settings,
                game_assets.as_deref(),
            );
            event_writers.sfx.write_batch(sfx.drain(..));
            event_writers.vfx.write_batch(vfx.drain(..));
            event_writers.debris.write_batch(debris.drain(..));
            return;
        }
    }

    if runtime.hitstun_timer <= 0.0 && (controls.attack_pressed || controls.pogo_pressed) {
        process_attack(
            &mut sfx,
            &mut vfx,
            &mut debris,
            &mut runtime,
            controls,
            tuning,
            feel,
        );
    }

    runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
    runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
    runtime.slash_anim_timer = (runtime.slash_anim_timer - frame_dt).max(0.0);
    event_writers.sfx.write_batch(sfx.drain(..));
    event_writers.vfx.write_batch(vfx.drain(..));
    event_writers.debris.write_batch(debris.drain(..));
}

/// Presentation-side debug hotkey reader.
///
/// Slice 5 of the events refactor moved this out of `sandbox_update` so the
/// gameplay loop no longer reads `Res<ButtonInput<KeyCode>>`. That lets
/// `sandbox_update` run on the headless App-builder track (where
/// `InputPlugin` is absent and `ButtonInput<KeyCode>` therefore does not
/// exist as a resource).
///
/// Runs `.before(sandbox_update)` so preset / debug-flag mutations land
/// before the gameplay loop reads them this frame. Updates the player's
/// `InputMap` directly when the preset cycles, which is the same effect
/// the old in-`sandbox_update` `preset_changed` branch had.
fn handle_debug_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut runtime: ResMut<SandboxRuntime>,
    mut tools: ResMut<DeveloperTools>,
    entities: Res<SceneEntities>,
    mut player_input: Query<
        (
            &mut ActionState<SandboxAction>,
            &mut InputMap<SandboxAction>,
        ),
        With<PlayerVisual>,
    >,
) {
    let mut preset_changed = false;
    if keys.just_pressed(KeyCode::F1) {
        runtime.debug = !runtime.debug;
    }
    if keys.just_pressed(KeyCode::F9) {
        runtime.preset_index =
            (runtime.preset_index + runtime.presets.len() - 1) % runtime.presets.len();
        runtime.preset_flash = 1.2;
        preset_changed = true;
    }
    if keys.just_pressed(KeyCode::F10) {
        runtime.preset_index = (runtime.preset_index + 1) % runtime.presets.len();
        runtime.preset_flash = 1.2;
        preset_changed = true;
    }
    if keys.just_pressed(KeyCode::F2) {
        runtime.slowmo = !runtime.slowmo;
    }
    if keys.just_pressed(KeyCode::F3) {
        tools.inspector_visible = !tools.inspector_visible;
    }
    if keys.just_pressed(KeyCode::F4) {
        tools.world_inspector_visible = !tools.world_inspector_visible;
    }
    if keys.just_pressed(KeyCode::F5) {
        tools.overview_camera = !tools.overview_camera;
    }

    if preset_changed {
        if let Ok((mut action_state, mut input_map)) = player_input.get_mut(entities.player) {
            *input_map = runtime.preset().input_map();
            action_state.reset_all();
        }
    }
}

fn handle_ldtk_hot_reload(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut world: ResMut<GameWorld>,
    mut room_set: ResMut<rooms::RoomSet>,
    mut runtime: ResMut<SandboxRuntime>,
    mut ldtk_index: ResMut<ldtk_world::LdtkRuntimeIndex>,
    mut ldtk_reload: ResMut<ldtk_world::LdtkHotReloadState>,
    editable_tuning: Res<EditableMovementTuning>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    game_assets: Option<Res<crate::game_assets::GameAssets>>,
) {
    if keys.just_pressed(KeyCode::F12) {
        ldtk_reload.auto_apply = !ldtk_reload.auto_apply;
        ldtk_reload.last_status = format!(
            "LDtk auto-apply {}",
            if ldtk_reload.auto_apply {
                "enabled"
            } else {
                "disabled"
            }
        );
    }

    let requested = keys.just_pressed(KeyCode::F11);
    let should_apply = requested || (ldtk_reload.pending && ldtk_reload.auto_apply);
    if !should_apply {
        return;
    }

    match reload_ldtk_world_from_disk(
        &mut commands,
        &mut *world,
        &mut *room_set,
        &mut *runtime,
        &mut *ldtk_index,
        editable_tuning.as_engine(),
        &room_visuals,
        game_assets.as_deref(),
    ) {
        Ok(active_room) => {
            ldtk_reload.mark_applied(&active_room);
            eprintln!("LDtk hot reload applied to active room '{active_room}'");
        }
        Err(errors) => {
            for error in &errors {
                eprintln!("LDtk hot reload rejected: {error}");
            }
            ldtk_reload.mark_failed(errors);
        }
    }
}

struct LdtkReloadTransaction {
    project: ldtk_world::LdtkProject,
    next_room_set: rooms::RoomSet,
    next_spec: rooms::RoomSpec,
    safe_player_pos: ae::Vec2,
}

fn prepare_ldtk_reload_transaction(
    current_room_id: &str,
    preserved_pos: ae::Vec2,
    player_size: ae::Vec2,
) -> Result<LdtkReloadTransaction, Vec<String>> {
    let project = ldtk_world::LdtkProject::load_from_disk().map_err(|error| vec![error])?;
    let report = project.validate();
    report.print_to_stderr();
    if !report.is_ok() {
        return Err(report.errors);
    }

    let mut next_room_set = project.to_room_set()?;
    let Some(next_active) = next_room_set
        .rooms
        .iter()
        .position(|room| room.id == current_room_id)
    else {
        return Err(vec![format!(
            "LDtk reload would delete current active area '{current_room_id}'. Move the player elsewhere or restore that activeArea before applying."
        )]);
    };
    next_room_set.active = next_active;
    let next_spec = next_room_set.active_spec().clone();

    let mut hard_errors = Vec::new();
    for warning in next_room_set.layout_warnings() {
        if warning.contains("references missing") {
            hard_errors.push(format!("LDtk reload graph error: {warning}"));
        } else {
            eprintln!("LDtk reload layout warning: {warning}");
        }
    }
    if !hard_errors.is_empty() {
        return Err(hard_errors);
    }

    let safe_player_pos = rooms::validated_spawn(&next_spec.world, preserved_pos, player_size);
    Ok(LdtkReloadTransaction {
        project,
        next_room_set,
        next_spec,
        safe_player_pos,
    })
}

fn reload_ldtk_world_from_disk(
    commands: &mut Commands,
    world: &mut GameWorld,
    room_set: &mut rooms::RoomSet,
    runtime: &mut SandboxRuntime,
    ldtk_index: &mut ldtk_world::LdtkRuntimeIndex,
    tuning: ae::MovementTuning,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    assets: Option<&crate::game_assets::GameAssets>,
) -> Result<String, Vec<String>> {
    let current_room_id = room_set.active_spec().id.clone();
    let preserved_pos = runtime.player.pos;
    let transaction =
        prepare_ldtk_reload_transaction(&current_room_id, preserved_pos, runtime.player.size)?;

    // Everything above this line is non-mutating: invalid edits, deleted active
    // areas, bad graph links, and unsafe player positions are rejected before
    // touching the live world. Only commit after the complete replacement room
    // graph and repaired player position have been built.
    for (entity, physics_entity) in room_visuals.iter() {
        if physics_entity.is_some() {
            physics::retire_physics_entity(commands, entity);
        } else {
            commands.entity(entity).despawn();
        }
    }

    let active_room = transaction.next_spec.id.clone();
    *room_set = transaction.next_room_set;
    world.0 = transaction.next_spec.world.clone();

    runtime.player.pos = transaction.safe_player_pos;
    runtime.player.refresh_movement_resources(tuning);
    runtime.last_safe_player_pos = transaction.safe_player_pos;
    runtime.moving_platform = platforms::MovingPlatformState::time_reference(&world.0);
    runtime.features = features::FeatureRuntime::from_world(&world.0);
    runtime.dialogue.close();
    runtime.hitstop_timer = 0.0;
    runtime.hitstun_timer = 0.0;
    runtime.room_transition_cooldown = 0.10;
    runtime.preset_flash = 1.0;

    ldtk_index.replace_from_project(&transaction.project, active_room.clone());

    spawn_room_visuals(
        commands,
        &world.0,
        &room_set.active_spec().loading_zones,
        runtime.physics_settings,
        assets,
    );
    platforms::spawn_moving_platform(commands, &world.0, runtime.moving_platform);

    Ok(active_room)
}

fn sandbox_dt(runtime: &SandboxRuntime, frame_dt: f32) -> f32 {
    if runtime.hitstop_timer > 0.0 {
        0.0
    } else {
        frame_dt * runtime.time_scale
    }
}

// `move_toward` has moved to `crate::lib` (`ambition_sandbox`) so the
// `SandboxRuntime` impl can use it; it is re-imported via the wildcard above.

fn reset_sandbox(
    world: &ae::World,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    let reset_from = runtime.player.pos;
    runtime.reset(world, tuning);
    runtime.flash_timer = feel.reset_flash_time;
    let reset_to = runtime.player.pos;
    sfx.push(SfxMessage::Reset { pos: reset_to });
    vfx.push(VfxMessage::ResetEffects {
        from: reset_from,
        to: reset_to,
    });
}

fn load_room(
    commands: &mut Commands,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    world: &mut GameWorld,
    room_set: &mut rooms::RoomSet,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    transition: rooms::RoomTransition,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    assets: Option<&crate::game_assets::GameAssets>,
) {
    let old_velocity = runtime.player.vel;
    let abilities = runtime.player.abilities;
    let fly_enabled = runtime.player.fly_enabled;
    let edge_exit = matches!(
        transition.zone.activation,
        rooms::LoadingZoneActivation::EdgeExit
    );

    for (entity, physics_entity) in room_visuals.iter() {
        if physics_entity.is_some() {
            physics::retire_physics_entity(commands, entity);
        } else {
            commands.entity(entity).despawn();
        }
    }
    let spec = room_set.set_active(transition.target_room).clone();
    world.0 = spec.world.clone();

    // Room transitions are not player deaths/resets. Rebuild transient room
    // state, but preserve ability progression and, for edge exits, preserve
    // velocity so side-to-side room changes feel continuous. Door transitions
    // intentionally zero velocity because they are discrete interactions.
    let arrival = rooms::validated_spawn(&world.0, transition.arrival, runtime.player.size);
    runtime.player = ae::Player::new_with_abilities(arrival, abilities);
    runtime.player.refresh_movement_resources(tuning);
    runtime.player.fly_enabled = fly_enabled && runtime.player.abilities.fly;
    if edge_exit {
        runtime.player.vel = old_velocity;
    }
    runtime.flash_timer = if edge_exit {
        feel.edge_transition_flash
    } else {
        feel.door_transition_flash
    };
    runtime.hitstop_timer = 0.0;
    runtime.damage_invuln_timer = 0.0;
    runtime.hitstun_timer = 0.0;
    runtime.last_safe_player_pos = runtime.player.pos;
    runtime.time_scale = 1.0;
    runtime.down_tap_timer = 0.0;
    runtime.moving_platform = platforms::MovingPlatformState::time_reference(&world.0);
    runtime.features = features::FeatureRuntime::from_world(&world.0);
    runtime.dialogue.close();
    // This guard prevents immediate backtracking when arriving inside/near a
    // paired zone. It should not feel like frozen input, so keep it short and
    // rely on validated arrivals to do most of the safety work.
    runtime.room_transition_cooldown = if edge_exit {
        feel.edge_transition_cooldown
    } else {
        feel.door_transition_cooldown
    };
    runtime.preset_flash = 1.0;

    spawn_room_visuals(
        commands,
        &world.0,
        &spec.loading_zones,
        physics_settings,
        assets,
    );
    platforms::spawn_moving_platform(commands, &world.0, runtime.moving_platform);
    sfx.push(SfxMessage::Reset {
        pos: runtime.player.pos,
    });
    if edge_exit {
        // Edge exits should feel like contiguous room scrolling, not a death-like
        // teleport. Only show an arrival puff in the new room because `from` was
        // expressed in the previous room's coordinate space.
        vfx.push(VfxMessage::Burst {
            pos: runtime.player.pos,
            count: 18,
            speed: 260.0,
            color: [0.35, 0.95, 1.0, 0.75],
            kind: ParticleKind::Dust,
        });
    } else {
        // Door transitions are discrete interactions, so a teleport-like effect
        // is acceptable; use the destination for both endpoints to avoid mixing
        // coordinate systems from two rooms.
        vfx.push(VfxMessage::ResetEffects {
            from: runtime.player.pos,
            to: runtime.player.pos,
        });
    }
}

fn handle_player_events(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    events: ae::FrameEvents,
    was_grounded: Option<bool>,
) {
    let pos = runtime.player.pos;
    for op in &events.operations {
        match op {
            ae::MovementOp::Jump | ae::MovementOp::WallJump => {
                sfx.push(SfxMessage::Jump { pos });
                vfx.push(VfxMessage::Dust {
                    pos: runtime.player.pos,
                    facing: runtime.player.facing,
                });
            }
            ae::MovementOp::DoubleJump => {
                sfx.push(SfxMessage::DoubleJump { pos });
                vfx.push(VfxMessage::Burst {
                    pos: runtime.player.pos,
                    count: 14,
                    speed: 210.0,
                    color: [0.70, 1.0, 0.86, 0.82],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Dash | ae::MovementOp::DoubleDash => {
                sfx.push(SfxMessage::Dash { pos });
                vfx.push(VfxMessage::Burst {
                    pos: runtime.player.pos,
                    count: 10,
                    speed: 330.0,
                    color: [1.0, 0.86, 0.38, 0.90],
                    kind: ParticleKind::Spark,
                });
            }
            ae::MovementOp::Blink | ae::MovementOp::PrecisionBlink => {
                // Blink visuals use the explicit `events.blinks` endpoint data below.
            }
            ae::MovementOp::FlyToggle => {
                vfx.push(VfxMessage::Burst {
                    pos: runtime.player.pos,
                    count: 12,
                    speed: 180.0,
                    color: [0.45, 0.82, 1.0, 0.72],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Pogo | ae::MovementOp::Rebound => {
                sfx.push(SfxMessage::Pogo { pos });
            }
            ae::MovementOp::WallCling | ae::MovementOp::WallClimb | ae::MovementOp::Slash => {}
            ae::MovementOp::Reset => {
                sfx.push(SfxMessage::Reset { pos });
            }
        }
    }
    for blink in &events.blinks {
        sfx.push(SfxMessage::Blink {
            pos: blink.from,
            precision: blink.precision,
        });
        vfx.push(VfxMessage::BlinkEffects {
            from: blink.from,
            to: blink.to,
            precision: blink.precision,
        });
    }
    if events.hazard || !events.operations.is_empty() {
        runtime.flash_timer = 0.12;
    }
    if let Some(was_grounded) = was_grounded {
        if !was_grounded && runtime.player.on_ground {
            vfx.push(VfxMessage::Dust {
                pos: runtime.player.pos + ae::Vec2::new(0.0, runtime.player.size.y * 0.5),
                facing: runtime.player.facing,
            });
        }
    }
}

fn handle_feature_events(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    debris: &mut Vec<DebrisBurstMessage>,
    events: &features::FeatureEvents,
    player_pos: ae::Vec2,
) {
    if events.reset_player {
        sfx.push(SfxMessage::Reset { pos: player_pos });
    }
    for physics_burst in &events.physics_bursts {
        let cue = match physics_burst.cue {
            features::FeaturePhysicsCue::Breakable => physics::PhysicsDebrisCue::Breakable,
            features::FeaturePhysicsCue::EnemyRagdoll => physics::PhysicsDebrisCue::EnemyRagdoll,
            features::FeaturePhysicsCue::BossRagdoll => physics::PhysicsDebrisCue::BossRagdoll,
        };
        debris.push(DebrisBurstMessage {
            pos: physics_burst.pos,
            cue,
        });
    }
    for &pos in &events.impacts {
        vfx.push(VfxMessage::Impact { pos });
        vfx.push(VfxMessage::Burst {
            pos,
            count: 14,
            speed: 300.0,
            color: [1.0, 0.34, 0.28, 0.88],
            kind: ParticleKind::Shard,
        });
        debris.push(DebrisBurstMessage {
            pos,
            cue: physics::PhysicsDebrisCue::Impact,
        });
    }
    for &pos in &events.bursts {
        vfx.push(VfxMessage::Burst {
            pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
    }
}

fn handle_player_heal_events(runtime: &mut SandboxRuntime, events: &features::FeatureEvents) {
    if events.player_heal > 0 {
        runtime.player_health.heal(events.player_heal);
    }
}

fn death_respawn_player(
    world: &ae::World,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
) {
    let to = world.spawn;
    runtime.reset(world, tuning);
    runtime.player_health.reset();
    runtime.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    runtime.flash_timer = feel.reset_flash_time.max(0.35);
    runtime.features.banner = "PLAYER DOWN: respawned at room start with full HP".to_string();
    runtime.features.banner_timer = 2.4;
    sfx.push(SfxMessage::Death { pos: from });
    vfx.push(VfxMessage::ResetEffects { from, to });
}

fn handle_player_damage_events(
    world: &ae::World,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    events: &features::FeatureEvents,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    let Some(damage) = events.player_damage.first().copied() else {
        return;
    };
    if runtime.player_health.damage(damage.amount.max(1)) {
        death_respawn_player(world, sfx, vfx, runtime, tuning, feel, damage.impact_pos);
        return;
    }
    match damage.mode {
        features::PlayerDamageMode::SafeRespawn => {
            safe_respawn_player(sfx, vfx, runtime, tuning, feel, damage.impact_pos);
        }
        features::PlayerDamageMode::Knockback => {
            apply_player_knockback(sfx, vfx, runtime, tuning, feel, damage);
        }
    }
}

fn safe_respawn_player(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
) {
    let to = runtime.last_safe_player_pos;
    runtime.player.reset_to(to);
    runtime.player.refresh_movement_resources(tuning);
    runtime.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    runtime.hitstun_timer = 0.0;
    runtime.hitstop_timer = 0.0;
    runtime.flash_timer = feel.reset_flash_time;
    runtime.time_scale = 1.0;
    sfx.push(SfxMessage::Reset { pos: to });
    vfx.push(VfxMessage::ResetEffects { from, to });
}

fn apply_player_knockback(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    damage: features::PlayerDamageEvent,
) {
    let _source_pos_for_future_directional_rules = damage.source_pos;
    let boss_hit = matches!(
        damage.source,
        features::PlayerDamageSource::BossBody | features::PlayerDamageSource::BossAttack
    );
    let dir = if damage.knockback_dir.abs() <= 0.001 {
        runtime.player.facing * -1.0
    } else {
        damage.knockback_dir.signum()
    };
    let strength = damage.strength.max(0.0);
    let knock_x = if boss_hit {
        feel.boss_knockback_x
    } else {
        feel.enemy_knockback_x
    };
    let knock_y = if boss_hit {
        feel.boss_knockback_y
    } else {
        feel.enemy_knockback_y
    };
    runtime.player.vel.x = dir * knock_x * strength;
    runtime.player.vel.y = -knock_y * strength;
    runtime.player.refresh_movement_resources(tuning);
    runtime.hitstun_timer = if boss_hit {
        feel.boss_hitstun_time
    } else {
        feel.enemy_hitstun_time
    } * strength.max(0.35);
    runtime.damage_invuln_timer = feel.knockback_invulnerability_time;
    runtime.hitstop_timer = feel.player_damage_hitstop_time;
    runtime.flash_timer = 0.20;
    sfx.push(SfxMessage::Hit {
        pos: damage.impact_pos,
    });
    vfx.push(VfxMessage::Impact {
        pos: damage.impact_pos,
    });
}

fn controls_for_hitstun(
    mut controls: ControlFrame,
    feel: SandboxFeelTuning,
    hitstun_timer: f32,
) -> ControlFrame {
    if hitstun_timer <= 0.0 {
        return controls;
    }
    let scale = feel.hitstun_control_scale.clamp(0.0, 1.0);
    controls.axis_x *= scale;
    controls.axis_y *= scale;
    controls.jump_pressed = false;
    controls.dash_pressed = false;
    controls.fast_fall_pressed = false;
    controls.blink_pressed = false;
    controls.blink_held = false;
    controls.blink_released = false;
    controls.attack_pressed = false;
    controls.pogo_pressed = false;
    controls.fly_toggle_pressed = false;
    controls.interact_pressed = false;
    controls
}

fn process_attack(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    debris: &mut Vec<DebrisBurstMessage>,
    runtime: &mut SandboxRuntime,
    controls: ControlFrame,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    if !runtime.player.abilities.attack {
        return;
    }
    let player_pos = runtime.player.pos;
    sfx.push(SfxMessage::Slash { pos: player_pos });
    // Roughly the slash sheet's eight 75ms frames; the animation system
    // freezes on the last frame once `clip_held` is set, so this only
    // needs to cover the typical clip duration.
    runtime.slash_anim_timer = 0.60;
    let attack = ae::slash_hitbox(&runtime.player, controls.axis_y, controls.pogo_pressed);
    vfx.push(VfxMessage::SlashPreview { hitbox: attack });
    let mut landed = false;
    let mut killed = false;
    let player_facing = runtime.player.facing;
    let feature_events = runtime
        .features
        .apply_player_attack(attack, 1, player_facing * 300.0);
    landed |= !feature_events.impacts.is_empty();
    killed |= feature_events
        .messages
        .iter()
        .any(|message| message.contains("defeated"));
    handle_feature_events(sfx, vfx, debris, &feature_events, player_pos);

    if landed {
        sfx.push(SfxMessage::Hit { pos: player_pos });
        runtime.hitstop_timer = feel.attack_hitstop_time;
        runtime.flash_timer = 0.16;
    }
    if killed {
        sfx.push(SfxMessage::Death { pos: player_pos });
    }
    if landed && runtime.player.abilities.pogo && (controls.pogo_pressed || controls.axis_y > 0.25)
    {
        runtime.player.vel.y = -tuning.pogo_speed;
        runtime.player.refresh_movement_resources(tuning);
        sfx.push(SfxMessage::Pogo { pos: player_pos });
    }
}

fn update_hud(
    runtime: Res<SandboxRuntime>,
    mode: Res<State<GameMode>>,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    display_mode: Res<windowing::DisplayModeState>,
    developer_tools: Res<DeveloperTools>,
    ldtk_reload: Res<ldtk_world::LdtkHotReloadState>,
    ldtk_spine: Res<ldtk_world::LdtkRuntimeSpineStats>,
    ldtk_spine_index: Res<ldtk_world::LdtkRuntimeSpineIndex>,
    windows: Query<&Window, With<PrimaryWindow>>,
    entities: Res<SceneEntities>,
    mut query: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = query.get_mut(entities.hud) else {
        return;
    };
    if !developer_tools.show_hud {
        **text = String::new();
        return;
    }
    if !runtime.debug {
        **text = "F1 debug | F3 inspector".to_string();
        return;
    }
    let preset = runtime.preset();
    let enemy_health = runtime
        .features
        .enemies
        .iter()
        .map(|e| {
            format!(
                "{} hp {}/{} alive {}",
                e.name,
                e.health.current.max(0),
                e.health.max,
                e.alive
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let mut gamepad = String::new();
    for (physical, semantic) in GAMEPAD_MAP.iter().take(6) {
        gamepad.push_str(&format!("{} = {}  ", physical, semantic));
    }
    let window_line = windows
        .single()
        .map(|w| {
            format!(
                "window: {:.0}x{:.0} {}",
                w.width(),
                w.height(),
                display_mode.label()
            )
        })
        .unwrap_or_else(|_| format!("window: unknown {}", display_mode.label()));
    let zone_hint = {
        let hints = room_set.nearby_zone_hints(&runtime.player, runtime.player.fly_enabled);
        if hints.is_empty() {
            "zones: none".to_string()
        } else {
            format!("zones: {}", hints.join(" | "))
        }
    };
    let feature_banner = if runtime.features.banner_timer > 0.0 {
        format!("\nFEATURE: {}", runtime.features.banner)
    } else {
        String::new()
    };
    if developer_tools.compact_hud {
        **text = format!(
            "{} | {} | room {}/{} | hp {}/{} | vel ({:+.0},{:+.0}) | grounded {} | dash {} | jumps {}\ncombo: {} | hint: {}\n{} | ldtk: {} auto={} pending={} spine={} rev={} promoted={} last={} | hitstun {:.2} invuln {:.2} hitstop {:.2} | preset {} | F1 debug F3 inspector F4 world F5 overview={} F11 reload F12 auto\n{}{}\n",
            world.0.name,
            mode.get().label(),
            room_set.active + 1,
            room_set.rooms.len(),
            runtime.player_health.current.max(0),
            runtime.player_health.max,
            runtime.player.vel.x,
            runtime.player.vel.y,
            runtime.player.on_ground,
            runtime.player.dash_charges_available,
            runtime.player.air_jumps_available,
            runtime.player.combo_symbols(),
            runtime.player.current_combo_hint(),
            zone_hint,
            ldtk_reload.last_status,
            ldtk_reload.auto_apply,
            ldtk_reload.pending,
            ldtk_spine.spawned_entities,
            ldtk_spine_index.revision,
            ldtk_spine_index.promoted_summary(),
            if ldtk_spine.last_entity.is_empty() { "none" } else { &ldtk_spine.last_entity },
            runtime.hitstun_timer,
            runtime.damage_invuln_timer,
            runtime.hitstop_timer,
            preset.name,
            developer_tools.overview_camera,
            runtime.features.feature_summary(),
            feature_banner,
        );
        return;
    }
    let flash_line = if runtime.preset_flash > 0.0 {
        format!("\nPRESET: {}", preset.name)
    } else {
        String::new()
    };
    **text = format!(
        "{}\nmode: {}  room: {}  active {}/{}  size {:.0}x{:.0}\n{}\nvel: ({:+.1}, {:+.1}) speed {:.1} max {:.1}\ngrounded: {} wall: {} dash_charges: {} air_jumps: {} blink_cd {:.2} blink_aim {} fly {} fastfall {} wall_cling: {} wall_climb: {} coyote {:.2} jump_buf {:.2} dash_buf {:.2} interact_buf {:.2}\ncombo: {}\nhint: {}\npreset: {} | movement: {} | {}\nF9/F10 presets  F1 debug  F2 slowmo={}  F3 inspector={}  F4 world-inspector={}  F5 overview={}  F6 windowed  F7 borderless  F8 fullscreen  F11 LDtk reload  F12 LDtk auto={} pending={}  Esc mode={}  Delete reset  hitstop {:.2}  hitstun {:.2}  invuln {:.2}  time_scale {:.6}\nLDtk: {}\nLDtk spine: {} entities, raw rev {}, promoted rev {}, promoted {}, last {}, sample {}\n{}\nplayer hp: {}/{}\nenemies: {}\n{}\ngamepad target: {}{}{}\n",
        world.0.name,
        mode.get().label(),
        "Bevy backend",
        room_set.active + 1,
        room_set.rooms.len(),
        world.0.size.x,
        world.0.size.y,
        zone_hint,
        runtime.player.vel.x,
        runtime.player.vel.y,
        runtime.player.vel.length(),
        runtime.player.max_speed,
        runtime.player.on_ground,
        runtime.player.on_wall,
        runtime.player.dash_charges_available,
        runtime.player.air_jumps_available,
        runtime.player.blink_cooldown,
        runtime.player.blink_aiming,
        runtime.player.fly_enabled,
        runtime.player.fast_falling,
        runtime.player.wall_clinging,
        runtime.player.wall_climbing,
        runtime.player.coyote_timer,
        runtime.player.jump_buffer_timer,
        runtime.player.dash_buffer_timer,
        runtime.interact_buffer_timer,
        runtime.player.combo_symbols(),
        runtime.player.current_combo_hint(),
        preset.name,
        preset.movement_label(),
        preset.action_label(),
        runtime.slowmo,
        developer_tools.inspector_visible,
        developer_tools.world_inspector_visible,
        developer_tools.overview_camera,
        ldtk_reload.auto_apply,
        ldtk_reload.pending,
        mode.get().label(),
        runtime.hitstop_timer,
        runtime.hitstun_timer,
        runtime.damage_invuln_timer,
        runtime.time_scale,
        ldtk_reload.last_status,
        ldtk_spine.spawned_entities,
        ldtk_spine.revision,
        ldtk_spine_index.revision,
        ldtk_spine_index.promoted_summary(),
        if ldtk_spine.last_entity.is_empty() { "none" } else { &ldtk_spine.last_entity },
        if ldtk_spine.sample_entity.is_empty() { "none" } else { &ldtk_spine.sample_entity },
        window_line,
        runtime.player_health.current.max(0),
        runtime.player_health.max,
        enemy_health,
        runtime.features.feature_summary(),
        gamepad,
        flash_line,
        feature_banner,
    );
}
