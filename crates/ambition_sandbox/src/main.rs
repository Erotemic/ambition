//! Ambition Tangent Space Sandbox, Bevy backend.
//!
//! This binary is intentionally assetless: all visible objects are colored
//! Bevy sprites, and all audio is synthesized at startup into in-memory WAV
//! assets. The platformer movement/collision core remains in `ambition_engine`.

mod audio;
mod config;
mod data;
mod dialog;
mod debug_overlay;
mod dev_tools;
mod fx;
mod features;
mod game_mode;
mod input;
mod loading;
mod ldtk_world;
mod platforms;
mod physics;
mod rendering;
mod rooms;
mod windowing;

use ambition_engine as ae;
use audio::{play_ambience, play_sound, SoundBank, SoundCue};
use bevy::audio::AudioSource;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution, WindowResizeConstraints};
use bevy_common_assets::ron::RonAssetPlugin;
use bevy_asset_loader::asset_collection::AssetCollectionApp;
use bevy_inspector_egui::{
    bevy_egui::EguiPlugin,
    quick::{ResourceInspectorPlugin, WorldInspectorPlugin},
};
use config::{world_to_bevy, WINDOW_H, WINDOW_W, WORLD_Z_PLAYER};
use dev_tools::{DeveloperTools, EditableAbilitySet, EditableMovementTuning, SandboxFeelTuning};
use bevy_material_ui::MaterialUiPlugin;
use bevy_ecs_ldtk::LdtkPlugin;

const BULLET_TIME_SCALE: f32 = 0.10;
const BLINK_HOLD_SLOW_SCALE: f32 = 0.35;
const DEBUG_SLOWMO_SCALE: f32 = 0.25;
const TIME_RAMP_DOWN_RATE: f32 = 5.0;
const TIME_RAMP_UP_RATE: f32 = 14.0;
const DOWN_DOUBLE_TAP_WINDOW: f32 = 0.24;
const UP_DOUBLE_TAP_WINDOW: f32 = 0.30;
use fx::{
    spawn_blink_effects, spawn_burst, spawn_dust, spawn_impact, spawn_reset_effects,
    spawn_slash_preview, ParticleKind,
};
use game_mode::GameMode;
use input::{ControlFrame, KeyboardPreset, SandboxAction, GAMEPAD_MAP};
use leafwing_input_manager::prelude::{ActionState, InputManagerPlugin, InputMap};
use rendering::{camera_follow, spawn_room_visuals, sync_visuals, HudText, PlayerVisual, RoomVisual, SceneEntities};

fn main() {
    let mut sandbox_data = data::SandboxDataSpec::load_embedded();
    let ldtk_project = ldtk_world::LdtkProject::load_embedded();
    let ldtk_report = ldtk_project.validate();
    ldtk_report.print_to_stderr();
    sandbox_data.rooms = ldtk_project
        .to_room_manifest()
        .expect("embedded LDtk world should validate and convert into Ambition rooms");
    let editable_abilities = EditableAbilitySet::from(sandbox_data.abilities);
    let editable_tuning = EditableMovementTuning::from(sandbox_data.tuning);
    let room_set = rooms::RoomSet::from_manifest(&sandbox_data.rooms);
    let active_world = room_set.active_world().clone();

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.020, 0.024, 0.035)))
        .insert_resource(GameWorld(active_world))
        .insert_resource(room_set)
        .insert_resource(sandbox_data)
        .insert_resource(DeveloperTools::default())
        .insert_resource(SandboxFeelTuning::default())
        .insert_resource(editable_abilities)
        .insert_resource(editable_tuning)
        .insert_resource(windowing::DisplayModeState::default())
        .register_type::<GameMode>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
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
        }))
        // DefaultPlugins installs StatesPlugin, so initialize GameMode after it.
        .init_state::<GameMode>()
        // The inspector quick plugins require EguiPlugin to be registered first.
        .add_plugins(EguiPlugin::default())
        .add_plugins(RonAssetPlugin::<data::SandboxDataSpec>::new(&["ron"]))
        .init_collection::<loading::SandboxAssetCollection>()
        .add_plugins(InputManagerPlugin::<SandboxAction>::default())
        .add_plugins(ae::AmbitionStateMachinePlugin::default())
        .add_plugins(dialog::yarn_spinner_plugin())
        .add_plugins(MaterialUiPlugin)
        .add_plugins(LdtkPlugin)
        .add_plugins(physics::AmbitionPhysicsPlugin)
        .register_type::<DeveloperTools>()
        .register_type::<EditableAbilitySet>()
        .register_type::<EditableMovementTuning>()
        .register_type::<SandboxFeelTuning>()
        .add_plugins(ResourceInspectorPlugin::<DeveloperTools>::default().run_if(dev_tools::inspector_visible))
        .add_plugins(ResourceInspectorPlugin::<EditableAbilitySet>::default().run_if(dev_tools::inspector_visible))
        .add_plugins(ResourceInspectorPlugin::<EditableMovementTuning>::default().run_if(dev_tools::inspector_visible))
        .add_plugins(ResourceInspectorPlugin::<SandboxFeelTuning>::default().run_if(dev_tools::inspector_visible))
        .add_plugins(WorldInspectorPlugin::new().run_if(dev_tools::world_inspector_visible))
        .add_systems(Startup, (data::load_data_asset_handle, setup).chain())
        .add_systems(
            Update,
            (
                dialog::dialog_input,
                sandbox_update,
                sync_visuals,
                camera_follow,
                debug_overlay::draw_debug_overlay,
                platforms::sync_moving_platform,
                fx::update_particles,
                fx::update_impacts,
                fx::update_slash_previews,
                windowing::window_mode_hotkeys,
                update_hud,
                dialog::sync_dialog_ui,
            )
                .chain(),
        )
        .add_systems(Update, rendering::sync_health_overlays.after(sync_visuals))
        .run();
}

#[derive(Resource, Clone)]
pub struct GameWorld(pub ae::World);

#[derive(Resource)]
pub struct SandboxRuntime {
    pub player: ae::Player,
    pub player_health: ae::Health,
    debug: bool,
    slowmo: bool,
    presets: Vec<KeyboardPreset>,
    preset_index: usize,
    preset_flash: f32,
    pub flash_timer: f32,
    hitstop_timer: f32,
    damage_invuln_timer: f32,
    hitstun_timer: f32,
    last_safe_player_pos: ae::Vec2,
    time_scale: f32,
    down_tap_timer: f32,
    up_tap_timer: f32,
    interact_buffer_timer: f32,
    pub moving_platform: platforms::MovingPlatformState,
    pub features: features::FeatureRuntime,
    pub dialogue: dialog::DialogState,
    physics_settings: physics::PhysicsSandboxSettings,
    pub room_transition_cooldown: f32,
}

impl SandboxRuntime {
    fn new(
        world: &ae::World,
        abilities: ae::AbilitySet,
        tuning: ae::MovementTuning,
        physics_settings: physics::PhysicsSandboxSettings,
    ) -> Self {
        let mut player = ae::Player::new_with_abilities(world.spawn, abilities);
        player.refresh_movement_resources(tuning);
        Self {
            player,
            player_health: ae::Health::new(5),
            debug: true,
            slowmo: false,
            presets: KeyboardPreset::presets().to_vec(),
            preset_index: 0,
            preset_flash: 1.2,
            flash_timer: 0.0,
            hitstop_timer: 0.0,
            damage_invuln_timer: 0.0,
            hitstun_timer: 0.0,
            last_safe_player_pos: world.spawn,
            time_scale: 1.0,
            down_tap_timer: 0.0,
            up_tap_timer: 0.0,
            interact_buffer_timer: 0.0,
            moving_platform: platforms::MovingPlatformState::time_reference(world),
            features: features::FeatureRuntime::from_world(world),
            dialogue: dialog::DialogState::default(),
            physics_settings,
            room_transition_cooldown: 0.0,
        }
    }

    fn reset(&mut self, world: &ae::World, tuning: ae::MovementTuning) {
        self.player.reset_to(world.spawn);
        self.player.refresh_movement_resources(tuning);
        self.player_health.reset();
        self.flash_timer = 0.18;
        self.hitstop_timer = 0.0;
        self.damage_invuln_timer = 0.0;
        self.hitstun_timer = 0.0;
        self.last_safe_player_pos = world.spawn;
        self.time_scale = 1.0;
        self.down_tap_timer = 0.0;
        self.up_tap_timer = 0.0;
        self.interact_buffer_timer = 0.0;
        self.moving_platform = platforms::MovingPlatformState::time_reference(world);
        self.features = features::FeatureRuntime::from_world(world);
        self.dialogue.close();
        self.room_transition_cooldown = 0.0;
    }

    fn register_down_tap(&mut self, down_pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.down_tap_timer = (self.down_tap_timer - frame_dt).max(0.0);
        if !down_pressed {
            return false;
        }
        if self.down_tap_timer > 0.0 {
            self.down_tap_timer = 0.0;
            true
        } else {
            self.down_tap_timer = window;
            false
        }
    }

    fn register_up_tap(&mut self, up_pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.up_tap_timer = (self.up_tap_timer - frame_dt).max(0.0);
        if !up_pressed {
            return false;
        }
        if self.up_tap_timer > 0.0 {
            self.up_tap_timer = 0.0;
            true
        } else {
            self.up_tap_timer = window;
            false
        }
    }

    fn buffered_interact(&mut self, interact_pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.interact_buffer_timer = (self.interact_buffer_timer - frame_dt).max(0.0);
        if interact_pressed {
            self.interact_buffer_timer = window;
        }
        self.interact_buffer_timer > 0.0
    }

    fn clear_interact_buffer(&mut self) {
        self.interact_buffer_timer = 0.0;
    }

    fn remember_safe_player_position(&mut self) {
        if self.player.on_ground {
            self.last_safe_player_pos = self.player.pos;
        }
    }

    fn update_time_scale(&mut self, frame_dt: f32, feel: SandboxFeelTuning) {
        let target = if self.hitstop_timer > 0.0 {
            0.0
        } else if self.player.blink_aiming {
            feel.bullet_time_scale
        } else if self.player.blink_hold_active {
            feel.blink_hold_slow_scale
        } else if self.slowmo {
            feel.debug_slowmo_scale
        } else {
            1.0
        };
        let rate = if target < self.time_scale { feel.time_ramp_down_rate } else { feel.time_ramp_up_rate };
        self.time_scale = move_toward(self.time_scale, target, rate * frame_dt);
    }

    pub(crate) fn preset(&self) -> KeyboardPreset {
        self.presets[self.preset_index]
    }

    pub(crate) fn debug_enabled(&self) -> bool {
        self.debug
    }
}

fn setup(
    mut commands: Commands,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data_asset: Option<Res<data::SandboxDataAsset>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    mut audio_sources: ResMut<Assets<AudioSource>>,
    sandbox_data: Res<data::SandboxDataSpec>,
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
) {
    if let Some(handle) = sandbox_data_asset.as_ref() {
        let _asset_handle_for_async_reload = handle.0.clone();
    }
    if let Some(collection) = sandbox_asset_collection.as_ref() {
        let _loaded_sandbox_data_handle = collection.sandbox_data.clone();
    }
    for warning in room_set.layout_warnings() {
        eprintln!("room layout warning: {warning}");
    }

    // The sandbox uses centered world coordinates that match the default
    // Bevy 2D camera convention. With the window at 1600x900 and the generated
    // room at 1600x900, the default orthographic projection shows the whole
    // room without requiring a Bevy-version-sensitive ScalingMode import.
    commands.spawn((Camera2d, Name::new("Main Camera")));
    let runtime = SandboxRuntime::new(
        &world.0,
        editable_abilities.as_engine(),
        editable_tuning.as_engine(),
        *physics_settings,
    );
    let player_input_map = runtime.preset().input_map();
    commands.insert_resource(runtime);
    let sound_bank = SoundBank::new(&mut audio_sources, &sandbox_data.audio);
    play_ambience(&mut commands, &sound_bank);
    commands.insert_resource(sound_bank);

    spawn_room_visuals(&mut commands, &world.0, room_set.active_loading_zones(), *physics_settings);
    platforms::spawn_moving_platform(&mut commands, &world.0, platforms::MovingPlatformState::time_reference(&world.0));

    let player = commands
        .spawn((
            Sprite::from_color(Color::srgba(0.80, 0.95, 1.0, 1.0), BVec2::new(28.0, 46.0)),
            Transform::from_translation(world_to_bevy(&world.0, world.0.spawn, WORLD_Z_PLAYER)),
            PlayerVisual,
            Name::new("Player"),
            ActionState::<SandboxAction>::default(),
            player_input_map,
        ))
        .id();


    let hud = commands
        .spawn((
            Text::new("Ambition"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgba(0.82, 0.90, 1.0, 0.96)),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(14.0),
                top: Val::Px(10.0),
                max_width: Val::Px(920.0),
                ..default()
            },
            Name::new("Debug HUD"),
            HudText,
        ))
        .id();

    commands.insert_resource(SceneEntities { player, hud });
}

fn sandbox_update(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut world: ResMut<GameWorld>,
    mut room_set: ResMut<rooms::RoomSet>,
    bank: Res<SoundBank>,
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut developer_tools: ResMut<DeveloperTools>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut runtime: ResMut<SandboxRuntime>,
    entities: Res<SceneEntities>,
    mut player_input: Query<(&mut ActionState<SandboxAction>, &mut InputMap<SandboxAction>), With<PlayerVisual>>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
) {
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;
    let physics_settings = runtime.physics_settings;
    dev_tools::sync_live_ability_edits(&mut runtime, editable_abilities.as_engine(), tuning);

    let preset_changed = handle_debug_hotkeys(&keys, &mut runtime, &mut developer_tools);

    let mut controls = ControlFrame::default();
    if let Ok((mut action_state, mut input_map)) = player_input.get_mut(entities.player) {
        if preset_changed {
            *input_map = runtime.preset().input_map();
            action_state.reset_all();
        }
        controls = if mode.get().allows_gameplay() {
            ControlFrame::read_gameplay(&action_state)
        } else {
            ControlFrame::read_menu(&action_state)
        };
    }

    if matches!(mode.get(), GameMode::Dialogue) {
        if let Ok((mut action_state, _)) = player_input.get_mut(entities.player) {
            action_state.reset_all();
        }
        let frame_dt = time.delta_secs();
        runtime.time_scale = 0.0;
        runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
        runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
        return;
    }

    if controls.start_pressed {
        let next = if mode.get().allows_gameplay() { GameMode::Paused } else { GameMode::Playing };
        next_mode.set(next);
        if let Ok((mut action_state, _)) = player_input.get_mut(entities.player) {
            action_state.reset_all();
        }
        runtime.time_scale = if next.allows_gameplay() { 1.0 } else { 0.0 };
        return;
    }

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
    controls.fast_fall_pressed = runtime.register_down_tap(controls.down_pressed, frame_dt, feel.down_double_tap_window);
    let door_double_tap_up = runtime.register_up_tap(controls.up_pressed, frame_dt, feel.up_double_tap_window);
    runtime.hitstop_timer = (runtime.hitstop_timer - frame_dt).max(0.0);

    if controls.reset_pressed {
        reset_sandbox(&mut commands, &world.0, &bank, &mut runtime, tuning, feel);
        return;
    } else {
        // Two-clock update:
        // - control_dt is real time for responsive inputs and precision-blink aim;
        // - sim_dt is scaled game time for gravity, platforms, enemies, particles.
        let control_frame = controls_for_hitstun(controls, feel, runtime.hitstun_timer);
        let input = control_frame.engine_input(frame_dt);
        let control_world = features::world_with_sandbox_solids(&world.0, &runtime.moving_platform, &runtime.features);
        let control_events = ae::update_player_control_with_tuning(&control_world, &mut runtime.player, input, frame_dt, tuning);
        if control_events.reset {
            reset_sandbox(&mut commands, &world.0, &bank, &mut runtime, tuning, feel);
            return;
        }
        handle_player_events(
            &mut commands,
            &world.0,
            &bank,
            &mut runtime,
            control_events,
            None,
        );

        runtime.update_time_scale(frame_dt, feel);
        let sim_dt = sandbox_dt(&runtime, frame_dt);

        let platform_delta = runtime.moving_platform.update(sim_dt);
        if runtime.moving_platform.is_riding(&runtime.player) {
            runtime.player.pos += platform_delta;
        }
        let collision_world = features::world_with_sandbox_solids(&world.0, &runtime.moving_platform, &runtime.features);

        let was_grounded = runtime.player.on_ground;
        let sim_events = ae::update_player_simulation_with_tuning(&collision_world, &mut runtime.player, input, sim_dt, tuning);
        if sim_events.reset {
            reset_sandbox(&mut commands, &world.0, &bank, &mut runtime, tuning, feel);
            return;
        }
        handle_player_events(
            &mut commands,
            &world.0,
            &bank,
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
    controls.interact_pressed = runtime.buffered_interact(
        raw_interact_pressed,
        frame_dt,
        feel.interaction_buffer_time,
    );

    let feature_dt = sandbox_dt(&runtime, frame_dt);
    let feature_world = features::world_with_sandbox_solids(&world.0, &runtime.moving_platform, &runtime.features);
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
    handle_feature_events(&mut commands, &world.0, &bank, &feature_events, physics_settings);
    handle_player_heal_events(&mut runtime, &feature_events);
    handle_player_damage_events(&mut commands, &world.0, &bank, &mut runtime, &feature_events, tuning, feel);
    if !feature_damaged_player {
        runtime.remember_safe_player_position();
    }
    if feature_interaction_consumed {
        runtime.clear_interact_buffer();
    }
    if let Some(request) = &feature_events.dialogue_request {
        runtime.dialogue.start(&request.dialogue_id, &request.npc_name);
        runtime.clear_interact_buffer();
        runtime.hitstop_timer = 0.0;
        next_mode.set(GameMode::Dialogue);
        return;
    }
    if feature_reset {
        reset_sandbox(&mut commands, &world.0, &bank, &mut runtime, tuning, feel);
        return;
    }

    if runtime.room_transition_cooldown <= 0.0 {
        if let Some(zone) = room_set.transition_for_player(&runtime.player, controls.interact_pressed) {
            runtime.clear_interact_buffer();
            load_room(
                &mut commands,
                &bank,
                &mut runtime,
                &mut *world,
                &mut *room_set,
                &room_visuals,
                zone,
                tuning,
                feel,
                physics_settings,
            );
            return;
        }
    }

    if runtime.hitstun_timer <= 0.0 && (controls.attack_pressed || controls.pogo_pressed) {
        process_attack(&mut commands, &world.0, &bank, &mut runtime, controls, tuning, feel, physics_settings);
    }

    runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
    runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
}

fn handle_debug_hotkeys(keys: &ButtonInput<KeyCode>, runtime: &mut SandboxRuntime, tools: &mut DeveloperTools) -> bool {
    let mut preset_changed = false;
    if keys.just_pressed(KeyCode::F1) {
        runtime.debug = !runtime.debug;
    }
    if keys.just_pressed(KeyCode::F9) {
        runtime.preset_index = (runtime.preset_index + runtime.presets.len() - 1) % runtime.presets.len();
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
    preset_changed
}

fn sandbox_dt(runtime: &SandboxRuntime, frame_dt: f32) -> f32 {
    if runtime.hitstop_timer > 0.0 {
        0.0
    } else {
        frame_dt * runtime.time_scale
    }
}

fn move_toward(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}

fn reset_sandbox(
    commands: &mut Commands,
    world: &ae::World,
    bank: &SoundBank,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    let reset_from = runtime.player.pos;
    runtime.reset(world, tuning);
    runtime.flash_timer = feel.reset_flash_time;
    let reset_to = runtime.player.pos;
    play_sound(commands, bank, SoundCue::Reset);
    spawn_reset_effects(commands, world, reset_from, reset_to);
}

fn load_room(
    commands: &mut Commands,
    bank: &SoundBank,
    runtime: &mut SandboxRuntime,
    world: &mut GameWorld,
    room_set: &mut rooms::RoomSet,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    transition: rooms::RoomTransition,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
) {
    let old_velocity = runtime.player.vel;
    let abilities = runtime.player.abilities;
    let fly_enabled = runtime.player.fly_enabled;
    let edge_exit = matches!(transition.zone.activation, rooms::LoadingZoneActivation::EdgeExit);

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
    runtime.flash_timer = if edge_exit { feel.edge_transition_flash } else { feel.door_transition_flash };
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
    runtime.room_transition_cooldown = if edge_exit { feel.edge_transition_cooldown } else { feel.door_transition_cooldown };
    runtime.preset_flash = 1.0;

    spawn_room_visuals(commands, &world.0, &spec.loading_zones, physics_settings);
    platforms::spawn_moving_platform(commands, &world.0, runtime.moving_platform);
    play_sound(commands, bank, SoundCue::Reset);
    if edge_exit {
        // Edge exits should feel like contiguous room scrolling, not a death-like
        // teleport. Only show an arrival puff in the new room because `from` was
        // expressed in the previous room's coordinate space.
        spawn_burst(commands, &world.0, runtime.player.pos, 18, 260.0, [0.35, 0.95, 1.0, 0.75], ParticleKind::Dust);
    } else {
        // Door transitions are discrete interactions, so a teleport-like effect
        // is acceptable; use the destination for both endpoints to avoid mixing
        // coordinate systems from two rooms.
        spawn_reset_effects(commands, &world.0, runtime.player.pos, runtime.player.pos);
    }
}

fn handle_player_events(
    commands: &mut Commands,
    render_world: &ae::World,
    bank: &SoundBank,
    runtime: &mut SandboxRuntime,
    events: ae::FrameEvents,
    was_grounded: Option<bool>,
) {
    for op in &events.operations {
        match op {
            ae::MovementOp::Jump | ae::MovementOp::WallJump => {
                play_sound(commands, bank, SoundCue::Jump);
                spawn_dust(commands, render_world, runtime.player.pos, runtime.player.facing);
            }
            ae::MovementOp::DoubleJump => {
                play_sound(commands, bank, SoundCue::DoubleJump);
                spawn_burst(commands, render_world, runtime.player.pos, 14, 210.0, [0.70, 1.0, 0.86, 0.82], ParticleKind::Dust);
            }
            ae::MovementOp::Dash | ae::MovementOp::DoubleDash => {
                play_sound(commands, bank, SoundCue::Dash);
                spawn_burst(commands, render_world, runtime.player.pos, 10, 330.0, [1.0, 0.86, 0.38, 0.90], ParticleKind::Spark);
            }
            ae::MovementOp::Blink | ae::MovementOp::PrecisionBlink => {
                // Blink visuals use the explicit `events.blinks` endpoint data below.
            }
            ae::MovementOp::FlyToggle => {
                spawn_burst(commands, render_world, runtime.player.pos, 12, 180.0, [0.45, 0.82, 1.0, 0.72], ParticleKind::Dust);
            }
            ae::MovementOp::Pogo | ae::MovementOp::Rebound => {
                play_sound(commands, bank, SoundCue::Pogo);
            }
            ae::MovementOp::WallCling | ae::MovementOp::WallClimb | ae::MovementOp::Slash => {}
            ae::MovementOp::Reset => {
                play_sound(commands, bank, SoundCue::Reset);
            }
        }
    }
    for blink in &events.blinks {
        play_sound(
            commands,
            bank,
            if blink.precision { SoundCue::PrecisionBlink } else { SoundCue::Blink },
        );
        spawn_blink_effects(commands, render_world, blink.from, blink.to, blink.precision);
    }
    if events.hazard || !events.operations.is_empty() {
        runtime.flash_timer = 0.12;
    }
    if let Some(was_grounded) = was_grounded {
        if !was_grounded && runtime.player.on_ground {
            spawn_dust(
                commands,
                render_world,
                runtime.player.pos + ae::Vec2::new(0.0, runtime.player.size.y * 0.5),
                runtime.player.facing,
            );
        }
    }
}

fn handle_feature_events(
    commands: &mut Commands,
    world: &ae::World,
    bank: &SoundBank,
    events: &features::FeatureEvents,
    physics_settings: physics::PhysicsSandboxSettings,
) {
    if events.reset_player {
        play_sound(commands, bank, SoundCue::Reset);
    }
    for physics_burst in &events.physics_bursts {
        let cue = match physics_burst.cue {
            features::FeaturePhysicsCue::Breakable => physics::PhysicsDebrisCue::Breakable,
            features::FeaturePhysicsCue::EnemyRagdoll => physics::PhysicsDebrisCue::EnemyRagdoll,
            features::FeaturePhysicsCue::BossRagdoll => physics::PhysicsDebrisCue::BossRagdoll,
        };
        physics::spawn_debris_burst(commands, world, physics_burst.pos, cue, physics_settings);
    }
    for &pos in &events.impacts {
        spawn_impact(commands, world, pos);
        spawn_burst(commands, world, pos, 14, 300.0, [1.0, 0.34, 0.28, 0.88], ParticleKind::Shard);
        physics::spawn_debris_burst(commands, world, pos, physics::PhysicsDebrisCue::Impact, physics_settings);
    }
    for &pos in &events.bursts {
        spawn_burst(commands, world, pos, 16, 230.0, [0.84, 0.95, 1.0, 0.82], ParticleKind::Spark);
    }
}

fn handle_player_heal_events(runtime: &mut SandboxRuntime, events: &features::FeatureEvents) {
    if events.player_heal > 0 {
        runtime.player_health.heal(events.player_heal);
    }
}

fn death_respawn_player(
    commands: &mut Commands,
    world: &ae::World,
    bank: &SoundBank,
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
    play_sound(commands, bank, SoundCue::Death);
    spawn_reset_effects(commands, world, from, to);
}

fn handle_player_damage_events(
    commands: &mut Commands,
    world: &ae::World,
    bank: &SoundBank,
    runtime: &mut SandboxRuntime,
    events: &features::FeatureEvents,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    let Some(damage) = events.player_damage.first().copied() else {
        return;
    };
    if runtime.player_health.damage(damage.amount.max(1)) {
        death_respawn_player(commands, world, bank, runtime, tuning, feel, damage.impact_pos);
        return;
    }
    match damage.mode {
        features::PlayerDamageMode::SafeRespawn => {
            safe_respawn_player(commands, world, bank, runtime, tuning, feel, damage.impact_pos);
        }
        features::PlayerDamageMode::Knockback => {
            apply_player_knockback(commands, world, bank, runtime, tuning, feel, damage);
        }
    }
}

fn safe_respawn_player(
    commands: &mut Commands,
    world: &ae::World,
    bank: &SoundBank,
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
    play_sound(commands, bank, SoundCue::Reset);
    spawn_reset_effects(commands, world, from, to);
}

fn apply_player_knockback(
    commands: &mut Commands,
    world: &ae::World,
    bank: &SoundBank,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    damage: features::PlayerDamageEvent,
) {
    let _source_pos_for_future_directional_rules = damage.source_pos;
    let boss_hit = matches!(damage.source, features::PlayerDamageSource::BossBody | features::PlayerDamageSource::BossAttack);
    let dir = if damage.knockback_dir.abs() <= 0.001 { runtime.player.facing * -1.0 } else { damage.knockback_dir.signum() };
    let strength = damage.strength.max(0.0);
    let knock_x = if boss_hit { feel.boss_knockback_x } else { feel.enemy_knockback_x };
    let knock_y = if boss_hit { feel.boss_knockback_y } else { feel.enemy_knockback_y };
    runtime.player.vel.x = dir * knock_x * strength;
    runtime.player.vel.y = -knock_y * strength;
    runtime.player.refresh_movement_resources(tuning);
    runtime.hitstun_timer = if boss_hit { feel.boss_hitstun_time } else { feel.enemy_hitstun_time } * strength.max(0.35);
    runtime.damage_invuln_timer = feel.knockback_invulnerability_time;
    runtime.hitstop_timer = feel.player_damage_hitstop_time;
    runtime.flash_timer = 0.20;
    play_sound(commands, bank, SoundCue::Hit);
    spawn_impact(commands, world, damage.impact_pos);
}

fn controls_for_hitstun(mut controls: ControlFrame, feel: SandboxFeelTuning, hitstun_timer: f32) -> ControlFrame {
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
    commands: &mut Commands,
    world: &ae::World,
    bank: &SoundBank,
    runtime: &mut SandboxRuntime,
    controls: ControlFrame,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
) {
    if !runtime.player.abilities.attack { return; }
    play_sound(commands, bank, SoundCue::Slash);
    let attack = ae::slash_hitbox(&runtime.player, controls.axis_y, controls.pogo_pressed);
    spawn_slash_preview(commands, world, attack);
    let mut landed = false;
    let mut killed = false;
    let player_facing = runtime.player.facing;
    let feature_events = runtime.features.apply_player_attack(attack, 1, player_facing * 300.0);
    landed |= !feature_events.impacts.is_empty();
    killed |= feature_events.messages.iter().any(|message| message.contains("defeated"));
    handle_feature_events(commands, world, bank, &feature_events, physics_settings);

    if landed {
        play_sound(commands, bank, SoundCue::Hit);
        runtime.hitstop_timer = feel.attack_hitstop_time;
        runtime.flash_timer = 0.16;
    }
    if killed {
        play_sound(commands, bank, SoundCue::Death);
    }
    if landed && runtime.player.abilities.pogo && (controls.pogo_pressed || controls.axis_y > 0.25) {
        runtime.player.vel.y = -tuning.pogo_speed;
        runtime.player.refresh_movement_resources(tuning);
        play_sound(commands, bank, SoundCue::Pogo);
    }
}

fn update_hud(
    runtime: Res<SandboxRuntime>,
    mode: Res<State<GameMode>>,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    display_mode: Res<windowing::DisplayModeState>,
    developer_tools: Res<DeveloperTools>,
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
        .map(|e| format!("{} hp {}/{} alive {}", e.name, e.health.current.max(0), e.health.max, e.alive))
        .collect::<Vec<_>>()
        .join(" | ");
    let mut gamepad = String::new();
    for (physical, semantic) in GAMEPAD_MAP.iter().take(6) {
        gamepad.push_str(&format!("{} = {}  ", physical, semantic));
    }
    let window_line = windows
        .single()
        .map(|w| format!("window: {:.0}x{:.0} {}", w.width(), w.height(), display_mode.label()))
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
            "{} | {} | room {}/{} | hp {}/{} | vel ({:+.0},{:+.0}) | grounded {} | dash {} | jumps {}\ncombo: {} | hint: {}\n{} | hitstun {:.2} invuln {:.2} hitstop {:.2} | preset {} | F1 debug F3 inspector F4 world F5 overview={}\n{}{}\n",
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
        "{}\nmode: {}  room: {}  active {}/{}  size {:.0}x{:.0}\n{}\nvel: ({:+.1}, {:+.1}) speed {:.1} max {:.1}\ngrounded: {} wall: {} dash_charges: {} air_jumps: {} blink_cd {:.2} blink_aim {} fly {} fastfall {} wall_cling: {} wall_climb: {} coyote {:.2} jump_buf {:.2} dash_buf {:.2} interact_buf {:.2}\ncombo: {}\nhint: {}\npreset: {} | movement: {} | {}\nF9/F10 presets  F1 debug  F2 slowmo={}  F3 inspector={}  F4 world-inspector={}  F5 overview={}  F6 windowed  F7 borderless  F8 fullscreen  Esc mode={}  Delete reset  hitstop {:.2}  hitstun {:.2}  invuln {:.2}  time_scale {:.6}\n{}\nplayer hp: {}/{}\nenemies: {}\n{}\ngamepad target: {}{}{}\n",
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
        mode.get().label(),
        runtime.hitstop_timer,
        runtime.hitstun_timer,
        runtime.damage_invuln_timer,
        runtime.time_scale,
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
