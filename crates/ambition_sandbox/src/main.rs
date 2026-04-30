//! Ambition Tangent Space Sandbox, Bevy backend.
//!
//! This binary is intentionally assetless: all visible objects are colored
//! Bevy sprites, and all audio is synthesized at startup into in-memory WAV
//! assets. The platformer movement/collision core remains in `ambition_engine`.

mod audio;
mod config;
mod debug_overlay;
mod dummies;
mod fx;
mod input;
mod platforms;
mod rendering;
mod rooms;

use ambition_engine as ae;
use audio::{play_sound, SoundBank, SoundCue};
use bevy::audio::AudioSource;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use config::{world_to_bevy, WINDOW_H, WINDOW_W, WORLD_Z_DUMMY, WORLD_Z_PLAYER};

const BULLET_TIME_SCALE: f32 = 0.000035;
const BLINK_HOLD_SLOW_SCALE: f32 = 0.01;
const DEBUG_SLOWMO_SCALE: f32 = 0.25;
const TIME_RAMP_DOWN_RATE: f32 = 5.0;
const TIME_RAMP_UP_RATE: f32 = 14.0;
const DOWN_DOUBLE_TAP_WINDOW: f32 = 0.24;
use dummies::{spawn_dummies, Dummy, DummyKind};
use fx::{
    spawn_blink_effects, spawn_burst, spawn_dust, spawn_impact, spawn_reset_effects,
    spawn_slash_preview, ParticleKind,
};
use input::{ControlFrame, KeyboardPreset, GAMEPAD_MAP};
use rendering::{camera_follow, dummy_color, spawn_room_visuals, sync_visuals, DummyVisual, HudText, PlayerVisual, RoomVisual, SceneEntities};

fn main() {
    let room_set = rooms::RoomSet::new();
    let active_world = room_set.active_world().clone();

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.020, 0.024, 0.035)))
        .insert_resource(GameWorld(active_world))
        .insert_resource(room_set)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ambition - Tangent Space Sandbox (Bevy)".into(),
                resolution: WindowResolution::new(WINDOW_W, WINDOW_H),
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                sandbox_update,
                sync_visuals,
                camera_follow,
                debug_overlay::draw_debug_overlay,
                platforms::sync_moving_platform,
                fx::update_particles,
                fx::update_impacts,
                fx::update_slash_previews,
                update_hud,
            )
                .chain(),
        )
        .run();
}

#[derive(Resource, Clone)]
pub struct GameWorld(pub ae::World);

#[derive(Resource)]
pub struct SandboxRuntime {
    pub player: ae::Player,
    pub dummies: Vec<Dummy>,
    debug: bool,
    freeze: bool,
    slowmo: bool,
    presets: Vec<KeyboardPreset>,
    preset_index: usize,
    preset_flash: f32,
    pub flash_timer: f32,
    hitstop_timer: f32,
    time_scale: f32,
    down_tap_timer: f32,
    pub moving_platform: platforms::MovingPlatformState,
    pub room_transition_cooldown: f32,
}

impl SandboxRuntime {
    fn new(world: &ae::World) -> Self {
        Self {
            player: ae::Player::new(world.spawn),
            dummies: spawn_dummies(world),
            debug: true,
            freeze: false,
            slowmo: false,
            presets: KeyboardPreset::presets().to_vec(),
            preset_index: 0,
            preset_flash: 1.2,
            flash_timer: 0.0,
            hitstop_timer: 0.0,
            time_scale: 1.0,
            down_tap_timer: 0.0,
            moving_platform: platforms::MovingPlatformState::time_reference(world),
            room_transition_cooldown: 0.0,
        }
    }

    fn reset(&mut self, world: &ae::World) {
        self.player.reset_to(world.spawn);
        self.dummies = spawn_dummies(world);
        self.flash_timer = 0.18;
        self.hitstop_timer = 0.0;
        self.time_scale = 1.0;
        self.down_tap_timer = 0.0;
        self.moving_platform = platforms::MovingPlatformState::time_reference(world);
        self.room_transition_cooldown = 0.0;
    }

    fn register_down_tap(&mut self, down_pressed: bool, frame_dt: f32) -> bool {
        self.down_tap_timer = (self.down_tap_timer - frame_dt).max(0.0);
        if !down_pressed {
            return false;
        }
        if self.down_tap_timer > 0.0 {
            self.down_tap_timer = 0.0;
            true
        } else {
            self.down_tap_timer = DOWN_DOUBLE_TAP_WINDOW;
            false
        }
    }

    fn update_time_scale(&mut self, frame_dt: f32) {
        let target = if self.freeze || self.hitstop_timer > 0.0 {
            0.0
        } else if self.player.blink_aiming {
            BULLET_TIME_SCALE
        } else if self.player.blink_hold_active {
            BLINK_HOLD_SLOW_SCALE
        } else if self.slowmo {
            DEBUG_SLOWMO_SCALE
        } else {
            1.0
        };
        let rate = if target < self.time_scale { TIME_RAMP_DOWN_RATE } else { TIME_RAMP_UP_RATE };
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
    mut audio_sources: ResMut<Assets<AudioSource>>,
) {
    // The sandbox uses centered world coordinates that match the default
    // Bevy 2D camera convention. With the window at 1600x900 and the generated
    // room at 1600x900, the default orthographic projection shows the whole
    // room without requiring a Bevy-version-sensitive ScalingMode import.
    commands.spawn(Camera2d);
    commands.insert_resource(SandboxRuntime::new(&world.0));
    commands.insert_resource(SoundBank::new(&mut audio_sources));

    spawn_room_visuals(&mut commands, &world.0, room_set.active_loading_zones());
    platforms::spawn_moving_platform(&mut commands, &world.0, platforms::MovingPlatformState::time_reference(&world.0));

    let player = commands
        .spawn((
            Sprite::from_color(Color::srgba(0.80, 0.95, 1.0, 1.0), BVec2::new(28.0, 46.0)),
            Transform::from_translation(world_to_bevy(&world.0, world.0.spawn, WORLD_Z_PLAYER)),
            PlayerVisual,
        ))
        .id();

    for (index, dummy) in spawn_dummies(&world.0).iter().enumerate() {
        commands.spawn((
            Sprite::from_color(dummy_color(dummy), BVec2::new(dummy.size.x, dummy.size.y)),
            Transform::from_translation(world_to_bevy(&world.0, dummy.pos, WORLD_Z_DUMMY)),
            DummyVisual { index },
        ));
    }

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
    mut runtime: ResMut<SandboxRuntime>,
    room_visuals: Query<Entity, With<RoomVisual>>,
) {
    handle_debug_hotkeys(&keys, &mut runtime);

    let preset = runtime.preset();
    let mut controls = ControlFrame::read(&keys, preset);
    if controls.start_pressed {
        runtime.freeze = !runtime.freeze;
    }

    let frame_dt = time.delta_secs();
    runtime.room_transition_cooldown = (runtime.room_transition_cooldown - frame_dt).max(0.0);
    controls.fast_fall_pressed = runtime.register_down_tap(controls.down_pressed, frame_dt);
    runtime.hitstop_timer = (runtime.hitstop_timer - frame_dt).max(0.0);
    runtime.update_time_scale(frame_dt);
    let dt = sandbox_dt(&runtime, frame_dt);
    let platform_delta = runtime.moving_platform.update(dt);
    if runtime.moving_platform.is_riding(&runtime.player) {
        runtime.player.pos += platform_delta;
    }
    let collision_world = platforms::world_with_moving_platform(&world.0, &runtime.moving_platform);

    if controls.reset_pressed {
        reset_sandbox(&mut commands, &world.0, &bank, &mut runtime);
    } else {
        update_player_and_feedback(
            &mut commands,
            &world.0,
            &collision_world,
            &bank,
            &mut runtime,
            controls,
            frame_dt,
            dt,
        );
    }

    if runtime.room_transition_cooldown <= 0.0 {
        if let Some(zone) = room_set.transition_for_player(&runtime.player) {
            load_room(
            &mut commands,
            &bank,
            &mut runtime,
            &mut *world,
            &mut *room_set,
            &room_visuals,
            zone,
            );
            return;
        }
    }

    if controls.attack_pressed || controls.pogo_pressed {
        process_attack(&mut commands, &world.0, &bank, &mut runtime, controls);
    }

    update_dummies(&mut commands, &collision_world, &bank, &mut runtime, dt);

    runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
    runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
}

fn handle_debug_hotkeys(keys: &ButtonInput<KeyCode>, runtime: &mut SandboxRuntime) {
    if keys.just_pressed(KeyCode::F1) {
        runtime.debug = !runtime.debug;
    }
    if keys.just_pressed(KeyCode::F9) {
        runtime.preset_index = (runtime.preset_index + runtime.presets.len() - 1) % runtime.presets.len();
        runtime.preset_flash = 1.2;
    }
    if keys.just_pressed(KeyCode::F10) {
        runtime.preset_index = (runtime.preset_index + 1) % runtime.presets.len();
        runtime.preset_flash = 1.2;
    }
    if keys.just_pressed(KeyCode::F2) {
        runtime.slowmo = !runtime.slowmo;
    }
}

fn sandbox_dt(runtime: &SandboxRuntime, frame_dt: f32) -> f32 {
    if runtime.freeze || runtime.hitstop_timer > 0.0 {
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
) {
    let reset_from = runtime.player.pos;
    runtime.reset(world);
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
    room_visuals: &Query<Entity, With<RoomVisual>>,
    zone: rooms::LoadingZone,
) {
    let from = runtime.player.pos;
    for entity in room_visuals.iter() {
        commands.entity(entity).despawn();
    }
    let spec = room_set.set_active(zone.target_room).clone();
    world.0 = spec.world.clone();
    runtime.reset(&world.0);
    runtime.player.reset_to(zone.target_spawn);
    runtime.dummies = spawn_dummies(&world.0);
    runtime.moving_platform = platforms::MovingPlatformState::time_reference(&world.0);
    runtime.room_transition_cooldown = 0.35;
    runtime.flash_timer = 0.24;
    runtime.preset_flash = 1.0;
    spawn_room_visuals(commands, &world.0, &spec.loading_zones);
    platforms::spawn_moving_platform(commands, &world.0, runtime.moving_platform);
    play_sound(commands, bank, SoundCue::Reset);
    spawn_reset_effects(commands, &world.0, from, runtime.player.pos);
}

fn update_player_and_feedback(
    commands: &mut Commands,
    render_world: &ae::World,
    collision_world: &ae::World,
    bank: &SoundBank,
    runtime: &mut SandboxRuntime,
    controls: ControlFrame,
    frame_dt: f32,
    dt: f32,
) {
    let was_grounded = runtime.player.on_ground;
    let events = ae::update_player(collision_world, &mut runtime.player, controls.engine_input(frame_dt), dt);
    if events.reset {
        reset_sandbox(commands, render_world, bank, runtime);
        return;
    }
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
    if !was_grounded && runtime.player.on_ground {
        spawn_dust(
            commands,
            render_world,
            runtime.player.pos + ae::Vec2::new(0.0, runtime.player.size.y * 0.5),
            runtime.player.facing,
        );
    }
}

fn process_attack(
    commands: &mut Commands,
    world: &ae::World,
    bank: &SoundBank,
    runtime: &mut SandboxRuntime,
    controls: ControlFrame,
) {
    if !runtime.player.abilities.attack { return; }
    play_sound(commands, bank, SoundCue::Slash);
    let attack = ae::slash_hitbox(&runtime.player, controls.axis_y, controls.pogo_pressed);
    spawn_slash_preview(commands, world, attack);
    let mut landed = false;
    let mut killed = false;
    let player_facing = runtime.player.facing;
    for dummy in &mut runtime.dummies {
        if dummy.alive && attack.intersects(dummy.aabb()) {
            let hit_pos = ae::Vec2::new((attack.center.x + dummy.pos.x) * 0.5, (attack.center.y + dummy.pos.y) * 0.5);
            spawn_impact(commands, world, hit_pos);
            spawn_burst(commands, world, hit_pos, 18, 390.0, [1.0, 0.93, 0.44, 0.94], ParticleKind::Shard);
            killed |= dummy.apply_hit(1, player_facing * 300.0);
            landed = true;
        }
    }
    if landed {
        play_sound(commands, bank, SoundCue::Hit);
        runtime.hitstop_timer = 0.055;
        runtime.flash_timer = 0.16;
    }
    if killed {
        play_sound(commands, bank, SoundCue::Death);
    }
    if landed && runtime.player.abilities.pogo && (controls.pogo_pressed || controls.axis_y > 0.25) {
        runtime.player.vel.y = -ae::POGO_SPEED;
        runtime.player.refresh_movement_resources(ae::DEFAULT_TUNING);
        play_sound(commands, bank, SoundCue::Pogo);
    }
}

fn update_dummies(
    commands: &mut Commands,
    world: &ae::World,
    bank: &SoundBank,
    runtime: &mut SandboxRuntime,
    dt: f32,
) {
    for dummy in &mut runtime.dummies {
        if dummy.update_in_world(dt, world) {
            play_sound(commands, bank, SoundCue::Respawn);
            spawn_burst(commands, world, dummy.pos, 16, 260.0, [0.92, 0.48, 0.95, 0.90], ParticleKind::Spark);
        }
    }
}

fn update_hud(
    runtime: Res<SandboxRuntime>,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    entities: Res<SceneEntities>,
    mut query: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = query.get_mut(entities.hud) else {
        return;
    };
    if !runtime.debug {
        **text = "F1 debug".to_string();
        return;
    }
    let preset = runtime.preset();
    let dummies = runtime
        .dummies
        .iter()
        .map(|d| {
            if d.kind == DummyKind::FiniteRespawner {
                format!("{} hp {}/{} alive {}", d.name, d.hp.max(0), d.max_hp, d.alive)
            } else {
                format!("{} infinite", d.name)
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let mut gamepad = String::new();
    for (physical, semantic) in GAMEPAD_MAP.iter().take(6) {
        gamepad.push_str(&format!("{} = {}  ", physical, semantic));
    }
    let flash_line = if runtime.preset_flash > 0.0 {
        format!("\nPRESET: {}", preset.name)
    } else {
        String::new()
    };
    **text = format!(
        "{}\nroom: {}  active {}/{}  size {:.0}x{:.0}\nvel: ({:+.1}, {:+.1}) speed {:.1} max {:.1}\ngrounded: {} wall: {} dash_charges: {} air_jumps: {} blink_cd {:.2} blink_aim {} fastfall {} wall_cling: {} wall_climb: {} coyote {:.2} buffer {:.2}\ncombo: {}\nhint: {}\npreset: {} | movement: {} | {}\nF9/F10 presets  F1 debug  F2 slowmo={}  Esc pause={}  Delete reset  hitstop {:.2}  time_scale {:.6}\ndummies: {}\ngamepad target: {}{}",
        world.0.name,
        "Bevy backend",
        room_set.active + 1,
        room_set.rooms.len(),
        world.0.size.x,
        world.0.size.y,
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
        runtime.player.fast_falling,
        runtime.player.wall_clinging,
        runtime.player.wall_climbing,
        runtime.player.coyote_timer,
        runtime.player.jump_buffer_timer,
        runtime.player.combo_symbols(),
        runtime.player.current_combo_hint(),
        preset.name,
        preset.movement_label(),
        preset.action_label(),
        runtime.slowmo,
        runtime.freeze,
        runtime.hitstop_timer,
        runtime.time_scale,
        dummies,
        gamepad,
        flash_line,
    );
}
