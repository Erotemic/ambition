//! Ambition Tangent Space Sandbox, Bevy backend.
//!
//! This binary is intentionally assetless: all visible objects are colored
//! Bevy sprites, and all audio is synthesized at startup into in-memory WAV
//! assets. The platformer movement/collision core remains in `ambition_engine`.

use ambition_engine as ae;
use bevy::audio::AudioSource;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use std::f32::consts::TAU;

const WINDOW_W: u32 = 1600;
const WINDOW_H: u32 = 900;
const WORLD_Z_BLOCK: f32 = 0.0;
const WORLD_Z_DUMMY: f32 = 10.0;
const WORLD_Z_PLAYER: f32 = 20.0;
const WORLD_Z_FX: f32 = 30.0;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.020, 0.024, 0.035)))
        .insert_resource(GameWorld(ae::build_endgame_sandbox()))
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
                update_particles,
                update_impacts,
                update_slash_previews,
                update_hud,
            )
                .chain(),
        )
        .run();
}

#[derive(Resource, Clone)]
struct GameWorld(ae::World);

#[derive(Resource)]
struct SandboxRuntime {
    player: ae::Player,
    dummies: Vec<Dummy>,
    debug: bool,
    freeze: bool,
    slowmo: bool,
    presets: Vec<KeyboardPreset>,
    preset_index: usize,
    preset_flash: f32,
    flash_timer: f32,
    hitstop_timer: f32,
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
        }
    }

    fn reset(&mut self, world: &ae::World) {
        self.player.reset_to(world.spawn);
        self.dummies = spawn_dummies(world);
        self.flash_timer = 0.18;
        self.hitstop_timer = 0.0;
    }

    fn preset(&self) -> KeyboardPreset {
        self.presets[self.preset_index]
    }
}

#[derive(Resource)]
struct SceneEntities {
    player: Entity,
    hud: Entity,
}

#[derive(Component)]
struct PlayerVisual;

#[derive(Component)]
struct DummyVisual {
    index: usize,
}

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct ParticleVisual {
    kind: ParticleKind,
    pos: ae::Vec2,
    vel: ae::Vec2,
    age: f32,
    lifetime: f32,
    radius: f32,
    rgba: [f32; 4],
    gravity: f32,
    drag: f32,
}

#[derive(Component)]
struct ImpactVisual {
    pos: ae::Vec2,
    age: f32,
    duration: f32,
    radius: f32,
}

#[derive(Component)]
struct SlashPreviewVisual {
    age: f32,
    duration: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PresetId {
    ArrowsZxc,
    WasdJkl,
    ArrowsQwer,
    WasdUipo,
}

#[derive(Clone, Copy, Debug)]
struct MovementKeys {
    left: KeyCode,
    right: KeyCode,
    up: KeyCode,
    down: KeyCode,
}

#[derive(Clone, Copy, Debug)]
struct ActionKeys {
    jump: KeyCode,
    attack: KeyCode,
    dash: KeyCode,
    secondary: Option<KeyCode>,
    quick_action: Option<KeyCode>,
    modifier: Option<KeyCode>,
    utility: Option<KeyCode>,
    map: Option<KeyCode>,
    inventory: Option<KeyCode>,
    pause: KeyCode,
    select_reset: KeyCode,
    dedicated_pogo: Option<KeyCode>,
}

#[derive(Clone, Copy, Debug)]
struct KeyboardPreset {
    id: PresetId,
    name: &'static str,
    movement: MovementKeys,
    actions: ActionKeys,
}

impl KeyboardPreset {
    fn presets() -> [Self; 4] {
        [
            Self::arrows_zxc(),
            Self::wasd_jkl(),
            Self::arrows_qwer(),
            Self::wasd_uipo(),
        ]
    }

    fn arrows_zxc() -> Self {
        Self {
            id: PresetId::ArrowsZxc,
            name: "classic action: arrows + Z/X/C",
            movement: MovementKeys {
                left: KeyCode::ArrowLeft,
                right: KeyCode::ArrowRight,
                up: KeyCode::ArrowUp,
                down: KeyCode::ArrowDown,
            },
            actions: ActionKeys {
                jump: KeyCode::KeyZ,
                attack: KeyCode::KeyX,
                dash: KeyCode::KeyC,
                secondary: Some(KeyCode::KeyA),
                quick_action: Some(KeyCode::KeyE),
                modifier: Some(KeyCode::KeyS),
                utility: Some(KeyCode::KeyD),
                map: Some(KeyCode::Tab),
                inventory: Some(KeyCode::KeyI),
                pause: KeyCode::Escape,
                select_reset: KeyCode::Delete,
                dedicated_pogo: None,
            },
        }
    }

    fn wasd_jkl() -> Self {
        Self {
            id: PresetId::WasdJkl,
            name: "custom PC: WASD + Space/J/K/L/I/U",
            movement: MovementKeys {
                left: KeyCode::KeyA,
                right: KeyCode::KeyD,
                up: KeyCode::KeyW,
                down: KeyCode::KeyS,
            },
            actions: ActionKeys {
                jump: KeyCode::Space,
                attack: KeyCode::KeyJ,
                dash: KeyCode::KeyK,
                secondary: Some(KeyCode::KeyL),
                quick_action: Some(KeyCode::KeyI),
                modifier: Some(KeyCode::ShiftLeft),
                utility: Some(KeyCode::KeyU),
                map: Some(KeyCode::Tab),
                inventory: Some(KeyCode::KeyV),
                pause: KeyCode::Escape,
                select_reset: KeyCode::Delete,
                dedicated_pogo: None,
            },
        }
    }

    fn arrows_qwer() -> Self {
        Self {
            id: PresetId::ArrowsQwer,
            name: "chirality A: arrows + QWER",
            movement: MovementKeys {
                left: KeyCode::ArrowLeft,
                right: KeyCode::ArrowRight,
                up: KeyCode::ArrowUp,
                down: KeyCode::ArrowDown,
            },
            actions: ActionKeys {
                jump: KeyCode::KeyQ,
                dash: KeyCode::KeyW,
                attack: KeyCode::KeyE,
                secondary: None,
                quick_action: None,
                modifier: None,
                utility: None,
                map: Some(KeyCode::Tab),
                inventory: Some(KeyCode::KeyI),
                pause: KeyCode::Escape,
                select_reset: KeyCode::Delete,
                dedicated_pogo: Some(KeyCode::KeyR),
            },
        }
    }

    fn wasd_uipo() -> Self {
        Self {
            id: PresetId::WasdUipo,
            name: "chirality B: WASD + UIPO",
            movement: MovementKeys {
                left: KeyCode::KeyA,
                right: KeyCode::KeyD,
                up: KeyCode::KeyW,
                down: KeyCode::KeyS,
            },
            actions: ActionKeys {
                jump: KeyCode::KeyU,
                dash: KeyCode::KeyI,
                attack: KeyCode::KeyP,
                secondary: None,
                quick_action: None,
                modifier: None,
                utility: None,
                map: Some(KeyCode::Tab),
                inventory: Some(KeyCode::KeyV),
                pause: KeyCode::Escape,
                select_reset: KeyCode::Delete,
                dedicated_pogo: Some(KeyCode::KeyO),
            },
        }
    }

    fn movement_label(&self) -> &'static str {
        match self.id {
            PresetId::ArrowsZxc | PresetId::ArrowsQwer => "Arrow keys",
            PresetId::WasdJkl | PresetId::WasdUipo => "WASD",
        }
    }

    fn action_label(&self) -> String {
        let mut parts = vec![
            format!("Jump {}", key_name(self.actions.jump)),
            format!("Attack {}", key_name(self.actions.attack)),
            format!("Dash {}", key_name(self.actions.dash)),
        ];
        if let Some(k) = self.actions.dedicated_pogo {
            parts.push(format!("Pogo {}", key_name(k)));
        } else {
            parts.push("Pogo Down+Attack".to_string());
        }
        let optional = [
            ("Secondary", self.actions.secondary),
            ("Quick", self.actions.quick_action),
            ("Modifier", self.actions.modifier),
            ("Utility", self.actions.utility),
            ("Map", self.actions.map),
            ("Inventory", self.actions.inventory),
            ("Select", Some(self.actions.select_reset)),
        ];
        for (label, key) in optional {
            if let Some(k) = key {
                parts.push(format!("{} {}", label, key_name(k)));
            }
        }
        parts.join("  |  ")
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ControlFrame {
    axis_x: f32,
    axis_y: f32,
    jump_pressed: bool,
    jump_held: bool,
    jump_released: bool,
    dash_pressed: bool,
    attack_pressed: bool,
    pogo_pressed: bool,
    reset_pressed: bool,
    start_pressed: bool,
}

impl ControlFrame {
    fn read(keys: &ButtonInput<KeyCode>, preset: KeyboardPreset) -> Self {
        let mut axis_x = 0.0;
        let mut axis_y = 0.0;
        if keys.pressed(preset.movement.left) {
            axis_x -= 1.0;
        }
        if keys.pressed(preset.movement.right) {
            axis_x += 1.0;
        }
        if keys.pressed(preset.movement.up) {
            axis_y -= 1.0;
        }
        if keys.pressed(preset.movement.down) {
            axis_y += 1.0;
        }
        let reset_pressed = keys.just_pressed(preset.actions.select_reset)
            || keys.just_pressed(KeyCode::Delete)
            || keys.just_pressed(KeyCode::Backspace);
        Self {
            axis_x,
            axis_y,
            jump_pressed: keys.just_pressed(preset.actions.jump),
            jump_held: keys.pressed(preset.actions.jump),
            jump_released: keys.just_released(preset.actions.jump),
            dash_pressed: keys.just_pressed(preset.actions.dash),
            attack_pressed: keys.just_pressed(preset.actions.attack),
            pogo_pressed: preset
                .actions
                .dedicated_pogo
                .map(|key| keys.just_pressed(key))
                .unwrap_or(false),
            reset_pressed,
            start_pressed: keys.just_pressed(preset.actions.pause),
        }
    }

    fn engine_input(self) -> ae::InputState {
        ae::InputState {
            axis_x: self.axis_x,
            axis_y: self.axis_y,
            jump_pressed: self.jump_pressed,
            jump_held: self.jump_held,
            jump_released: self.jump_released,
            dash_pressed: self.dash_pressed,
            attack_pressed: self.attack_pressed,
            pogo_pressed: self.pogo_pressed,
            reset_pressed: false,
        }
    }
}

fn key_name(key: KeyCode) -> &'static str {
    match key {
        KeyCode::KeyA => "A",
        KeyCode::KeyB => "B",
        KeyCode::KeyC => "C",
        KeyCode::KeyD => "D",
        KeyCode::KeyE => "E",
        KeyCode::KeyF => "F",
        KeyCode::KeyG => "G",
        KeyCode::KeyH => "H",
        KeyCode::KeyI => "I",
        KeyCode::KeyJ => "J",
        KeyCode::KeyK => "K",
        KeyCode::KeyL => "L",
        KeyCode::KeyM => "M",
        KeyCode::KeyN => "N",
        KeyCode::KeyO => "O",
        KeyCode::KeyP => "P",
        KeyCode::KeyQ => "Q",
        KeyCode::KeyR => "R",
        KeyCode::KeyS => "S",
        KeyCode::KeyT => "T",
        KeyCode::KeyU => "U",
        KeyCode::KeyV => "V",
        KeyCode::KeyW => "W",
        KeyCode::KeyX => "X",
        KeyCode::KeyY => "Y",
        KeyCode::KeyZ => "Z",
        KeyCode::ArrowLeft => "Left",
        KeyCode::ArrowRight => "Right",
        KeyCode::ArrowUp => "Up",
        KeyCode::ArrowDown => "Down",
        KeyCode::Space => "Space",
        KeyCode::ShiftLeft => "LShift",
        KeyCode::Tab => "Tab",
        KeyCode::Escape => "Esc",
        KeyCode::Delete => "Delete",
        KeyCode::Backspace => "Backspace",
        _ => "?",
    }
}

const GAMEPAD_MAP: &[(&str, &str)] = &[
    ("L-stick / D-pad", "movement"),
    ("A / Cross", "jump / confirm"),
    ("X / Square", "primary attack"),
    ("RT / R2", "dash"),
    ("B / Circle", "secondary action placeholder"),
    ("RB / R1", "quick action placeholder"),
    ("LT / L2", "modifier placeholder"),
    ("Y / Triangle", "utility action placeholder"),
    ("LB / L1", "map placeholder"),
    ("Back / Touchpad", "inventory or sandbox reset"),
    ("Start / Options", "pause / menu"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DummyKind {
    InfiniteSandbag,
    FiniteRespawner,
}

#[derive(Clone, Debug)]
struct Dummy {
    name: &'static str,
    kind: DummyKind,
    spawn: ae::Vec2,
    pos: ae::Vec2,
    vel: ae::Vec2,
    size: ae::Vec2,
    hp: i32,
    max_hp: i32,
    alive: bool,
    respawn_timer: f32,
    hit_flash: f32,
    hit_stun: f32,
}

impl Dummy {
    fn infinite(name: &'static str, spawn: ae::Vec2) -> Self {
        Self {
            name,
            kind: DummyKind::InfiniteSandbag,
            spawn,
            pos: spawn,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(38.0, 66.0),
            hp: 9999,
            max_hp: 9999,
            alive: true,
            respawn_timer: 0.0,
            hit_flash: 0.0,
            hit_stun: 0.0,
        }
    }

    fn finite(name: &'static str, spawn: ae::Vec2) -> Self {
        Self {
            name,
            kind: DummyKind::FiniteRespawner,
            spawn,
            pos: spawn,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(34.0, 58.0),
            hp: 6,
            max_hp: 6,
            alive: true,
            respawn_timer: 0.0,
            hit_flash: 0.0,
            hit_stun: 0.0,
        }
    }

    fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    fn apply_hit(&mut self, damage: i32, knock_x: f32) -> bool {
        if !self.alive {
            return false;
        }
        self.hit_flash = 0.18;
        self.hit_stun = 0.075;
        self.vel.x += knock_x;
        self.vel.y = (self.vel.y - 120.0).max(-360.0);
        let mut killed = false;
        if self.kind == DummyKind::FiniteRespawner {
            self.hp -= damage;
            if self.hp <= 0 {
                self.alive = false;
                self.respawn_timer = 0.85;
                killed = true;
            }
        }
        killed
    }

    fn update(&mut self, dt: f32, ground_y: f32) -> bool {
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        if !self.alive {
            self.respawn_timer = (self.respawn_timer - dt).max(0.0);
            if self.respawn_timer <= 0.0 {
                self.alive = true;
                self.hp = self.max_hp;
                self.pos = ae::Vec2::new(self.spawn.x, 88.0);
                self.vel = ae::Vec2::ZERO;
                self.hit_flash = 0.24;
                self.hit_stun = 0.0;
                return true;
            }
            return false;
        }
        self.hit_stun = (self.hit_stun - dt).max(0.0);
        if self.hit_stun > 0.0 {
            return false;
        }
        self.vel.y += 1600.0 * dt;
        self.vel.x = approach(self.vel.x, 0.0, 820.0 * dt);
        self.vel.y = self.vel.y.min(900.0);
        self.pos += self.vel * dt;
        let half_h = self.size.y * 0.5;
        if self.pos.y + half_h >= ground_y {
            self.pos.y = ground_y - half_h;
            self.vel.y = 0.0;
        }
        false
    }
}

fn spawn_dummies(world: &ae::World) -> Vec<Dummy> {
    let ground_y = world.size.y - 48.0;
    vec![
        Dummy::infinite("infinite sandbag", ae::Vec2::new(world.spawn.x + 170.0, ground_y - 33.0)),
        Dummy::finite("finite drop dummy", ae::Vec2::new(world.spawn.x + 300.0, ground_y - 29.0)),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParticleKind {
    Spark,
    Dust,
    Shard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SoundCue {
    Jump,
    DoubleJump,
    Dash,
    Slash,
    Hit,
    Pogo,
    Reset,
    Death,
    Respawn,
}

#[derive(Resource)]
struct SoundBank {
    jump: Handle<AudioSource>,
    double_jump: Handle<AudioSource>,
    dash: Handle<AudioSource>,
    slash: Handle<AudioSource>,
    hit: Handle<AudioSource>,
    pogo: Handle<AudioSource>,
    reset: Handle<AudioSource>,
    death: Handle<AudioSource>,
    respawn: Handle<AudioSource>,
}

impl SoundBank {
    fn new(audio_sources: &mut Assets<AudioSource>) -> Self {
        let mut add = |spec: SynthSpec| audio_sources.add(AudioSource { bytes: synth_wav_bytes(spec, 44_100).into() });
        Self {
            jump: add(SynthSpec::jump()),
            double_jump: add(SynthSpec::double_jump()),
            dash: add(SynthSpec::dash()),
            slash: add(SynthSpec::slash()),
            hit: add(SynthSpec::hit()),
            pogo: add(SynthSpec::pogo()),
            reset: add(SynthSpec::reset()),
            death: add(SynthSpec::death()),
            respawn: add(SynthSpec::respawn()),
        }
    }

    fn get(&self, cue: SoundCue) -> Handle<AudioSource> {
        match cue {
            SoundCue::Jump => self.jump.clone(),
            SoundCue::DoubleJump => self.double_jump.clone(),
            SoundCue::Dash => self.dash.clone(),
            SoundCue::Slash => self.slash.clone(),
            SoundCue::Hit => self.hit.clone(),
            SoundCue::Pogo => self.pogo.clone(),
            SoundCue::Reset => self.reset.clone(),
            SoundCue::Death => self.death.clone(),
            SoundCue::Respawn => self.respawn.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Waveform {
    Sine,
    Square,
    Triangle,
    Saw,
}

#[derive(Clone, Copy, Debug)]
struct SynthSpec {
    waveform: Waveform,
    frequency: f32,
    frequency_end: f32,
    duration: f32,
    volume: f32,
    attack: f32,
    release: f32,
    noise: f32,
}

impl SynthSpec {
    fn jump() -> Self {
        Self::tone(Waveform::Sine, 460.0, 720.0, 0.085, 0.22)
    }
    fn double_jump() -> Self {
        Self::tone(Waveform::Triangle, 520.0, 940.0, 0.115, 0.22)
    }
    fn dash() -> Self {
        Self::tone(Waveform::Saw, 260.0, 110.0, 0.105, 0.18)
    }
    fn slash() -> Self {
        Self::tone(Waveform::Square, 620.0, 340.0, 0.075, 0.16)
    }
    fn hit() -> Self {
        Self {
            noise: 0.44,
            ..Self::tone(Waveform::Triangle, 220.0, 88.0, 0.105, 0.26)
        }
    }
    fn pogo() -> Self {
        Self::tone(Waveform::Sine, 360.0, 880.0, 0.105, 0.22)
    }
    fn reset() -> Self {
        Self::tone(Waveform::Sine, 160.0, 90.0, 0.150, 0.16)
    }
    fn death() -> Self {
        Self {
            noise: 0.18,
            ..Self::tone(Waveform::Saw, 140.0, 48.0, 0.220, 0.24)
        }
    }
    fn respawn() -> Self {
        Self::tone(Waveform::Triangle, 440.0, 660.0, 0.145, 0.20)
    }
    fn tone(waveform: Waveform, frequency: f32, frequency_end: f32, duration: f32, volume: f32) -> Self {
        Self {
            waveform,
            frequency,
            frequency_end,
            duration,
            volume,
            attack: 0.003,
            release: 0.045,
            noise: 0.0,
        }
    }
}

fn setup(
    mut commands: Commands,
    world: Res<GameWorld>,
    mut audio_sources: ResMut<Assets<AudioSource>>,
) {
    // The sandbox uses centered world coordinates that match the default
    // Bevy 2D camera convention. With the window at 1600x900 and the generated
    // room at 1600x900, the default orthographic projection shows the whole
    // room without requiring a Bevy-version-sensitive ScalingMode import.
    commands.spawn(Camera2d);
    commands.insert_resource(SandboxRuntime::new(&world.0));
    commands.insert_resource(SoundBank::new(&mut audio_sources));

    spawn_grid(&mut commands, &world.0);
    for block in &world.0.blocks {
        spawn_block(&mut commands, &world.0, block);
    }

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

    commands.insert_resource(SceneEntities {
        player,
        hud,
    });
}

fn sandbox_update(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<GameWorld>,
    bank: Res<SoundBank>,
    mut runtime: ResMut<SandboxRuntime>,
) {
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

    let preset = runtime.preset();
    let controls = ControlFrame::read(&keys, preset);
    if controls.start_pressed {
        runtime.freeze = !runtime.freeze;
    }

    let frame_dt = time.delta_secs();
    runtime.hitstop_timer = (runtime.hitstop_timer - frame_dt).max(0.0);
    let dt = if runtime.freeze || runtime.hitstop_timer > 0.0 {
        0.0
    } else if runtime.slowmo {
        frame_dt * 0.25
    } else {
        frame_dt
    };

    if controls.reset_pressed {
        let reset_from = runtime.player.pos;
        runtime.reset(&world.0);
        let reset_to = runtime.player.pos;
        play_sound(&mut commands, &bank, SoundCue::Reset);
        spawn_reset_effects(&mut commands, &world.0, reset_from, reset_to);
    } else {
        let was_grounded = runtime.player.on_ground;
        let events = ae::update_player(&world.0, &mut runtime.player, controls.engine_input(), dt);
        if events.reset {
            let reset_from = runtime.player.pos;
            runtime.reset(&world.0);
            let reset_to = runtime.player.pos;
            play_sound(&mut commands, &bank, SoundCue::Reset);
            spawn_reset_effects(&mut commands, &world.0, reset_from, reset_to);
        }
        for op in &events.operations {
            match op {
                ae::MovementOp::Jump | ae::MovementOp::WallJump => {
                    play_sound(&mut commands, &bank, SoundCue::Jump);
                    spawn_dust(&mut commands, &world.0, runtime.player.pos, runtime.player.facing);
                }
                ae::MovementOp::DoubleJump => {
                    play_sound(&mut commands, &bank, SoundCue::DoubleJump);
                    spawn_burst(
                        &mut commands,
                        &world.0,
                        runtime.player.pos,
                        14,
                        210.0,
                        [0.70, 1.0, 0.86, 0.82],
                        ParticleKind::Dust,
                    );
                }
                ae::MovementOp::Dash => {
                    play_sound(&mut commands, &bank, SoundCue::Dash);
                    spawn_burst(
                        &mut commands,
                        &world.0,
                        runtime.player.pos,
                        10,
                        330.0,
                        [1.0, 0.86, 0.38, 0.90],
                        ParticleKind::Spark,
                    );
                }
                ae::MovementOp::Pogo | ae::MovementOp::Rebound => {
                    play_sound(&mut commands, &bank, SoundCue::Pogo);
                }
                ae::MovementOp::Slash => {}
                ae::MovementOp::Reset => {
                    play_sound(&mut commands, &bank, SoundCue::Reset);
                }
            }
        }
        if events.hazard || !events.operations.is_empty() {
            runtime.flash_timer = 0.12;
        }
        if !was_grounded && runtime.player.on_ground {
            spawn_dust(
                &mut commands,
                &world.0,
                runtime.player.pos + ae::Vec2::new(0.0, runtime.player.size.y * 0.5),
                runtime.player.facing,
            );
        }
    }

    if controls.attack_pressed || controls.pogo_pressed {
        play_sound(&mut commands, &bank, SoundCue::Slash);
        let attack = slash_hitbox(&runtime.player, controls.axis_y, controls.pogo_pressed);
        spawn_slash_preview(&mut commands, &world.0, attack);
        let mut landed = false;
        let mut killed = false;
        let player_facing = runtime.player.facing;
        for dummy in &mut runtime.dummies {
            if dummy.alive && attack.intersects(dummy.aabb()) {
                let hit_pos = ae::Vec2::new(
                    (attack.center.x + dummy.pos.x) * 0.5,
                    (attack.center.y + dummy.pos.y) * 0.5,
                );
                spawn_impact(&mut commands, &world.0, hit_pos);
                spawn_burst(
                    &mut commands,
                    &world.0,
                    hit_pos,
                    18,
                    390.0,
                    [1.0, 0.93, 0.44, 0.94],
                    ParticleKind::Shard,
                );
                killed |= dummy.apply_hit(1, player_facing * 300.0);
                landed = true;
            }
        }
        if landed {
            play_sound(&mut commands, &bank, SoundCue::Hit);
            runtime.hitstop_timer = 0.055;
            runtime.flash_timer = 0.16;
        }
        if killed {
            play_sound(&mut commands, &bank, SoundCue::Death);
        }
        if landed && (controls.pogo_pressed || controls.axis_y > 0.25) {
            runtime.player.vel.y = -ae::POGO_SPEED;
            runtime.player.dash_available = true;
            play_sound(&mut commands, &bank, SoundCue::Pogo);
        }
    }

    let ground_y = world.0.size.y - 48.0;
    for dummy in &mut runtime.dummies {
        if dummy.update(dt, ground_y) {
            play_sound(&mut commands, &bank, SoundCue::Respawn);
            spawn_burst(
                &mut commands,
                &world.0,
                dummy.pos,
                16,
                260.0,
                [0.92, 0.48, 0.95, 0.90],
                ParticleKind::Spark,
            );
        }
    }

    runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
    runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
}

fn sync_visuals(
    world: Res<GameWorld>,
    runtime: Res<SandboxRuntime>,
    entities: Res<SceneEntities>,
    mut player_query: Query<(&mut Transform, &mut Sprite), (With<PlayerVisual>, Without<DummyVisual>)>,
    mut dummy_query: Query<(&DummyVisual, &mut Transform, &mut Sprite, &mut Visibility), (With<DummyVisual>, Without<PlayerVisual>)>,
) {
    if let Ok((mut transform, mut sprite)) = player_query.get_mut(entities.player) {
        transform.translation = world_to_bevy(&world.0, runtime.player.pos, WORLD_Z_PLAYER);
        sprite.custom_size = Some(BVec2::new(runtime.player.size.x, runtime.player.size.y));
        let alpha = if runtime.flash_timer > 0.0 { 0.72 } else { 1.0 };
        sprite.color = Color::srgba(0.80, 0.95, 1.0, alpha);
    }

    for (visual, mut transform, mut sprite, mut visibility) in &mut dummy_query {
        let Some(dummy) = runtime.dummies.get(visual.index) else {
            continue;
        };
        transform.translation = world_to_bevy(&world.0, dummy.pos, WORLD_Z_DUMMY);
        sprite.custom_size = Some(BVec2::new(dummy.size.x, dummy.size.y));
        sprite.color = dummy_color(dummy);
        *visibility = if dummy.alive { Visibility::Visible } else { Visibility::Hidden };
    }
}

fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<GameWorld>,
    mut query: Query<(Entity, &mut ParticleVisual, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut p, mut transform, mut sprite) in &mut query {
        p.age += dt;
        if p.age >= p.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        p.vel.y += p.gravity * dt;
        let drag = (1.0 - p.drag * dt).clamp(0.0, 1.0);
        p.vel *= drag;
        let velocity = p.vel;
        p.pos += velocity * dt;
        let t = (p.age / p.lifetime).clamp(0.0, 1.0);
        let alpha = p.rgba[3] * (1.0 - t);
        let size = match p.kind {
            ParticleKind::Spark => p.radius * (1.0 - 0.35 * t),
            ParticleKind::Dust => p.radius * (1.0 + 0.70 * t),
            ParticleKind::Shard => p.radius * (1.0 - 0.15 * t),
        };
        transform.translation = world_to_bevy(&world.0, p.pos, WORLD_Z_FX);
        sprite.custom_size = Some(BVec2::splat(size.max(0.5)));
        sprite.color = rgba(p.rgba[0], p.rgba[1], p.rgba[2], alpha);
    }
}

fn update_impacts(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<GameWorld>,
    mut query: Query<(Entity, &mut ImpactVisual, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut fx, mut transform, mut sprite) in &mut query {
        fx.age += dt;
        if fx.age >= fx.duration {
            commands.entity(entity).despawn();
            continue;
        }
        let t = (fx.age / fx.duration).clamp(0.0, 1.0);
        let radius = fx.radius + 46.0 * t;
        let alpha = 0.82 * (1.0 - t);
        transform.translation = world_to_bevy(&world.0, fx.pos, WORLD_Z_FX + 1.0);
        sprite.custom_size = Some(BVec2::splat(radius));
        sprite.color = Color::srgba(1.0, 1.0, 0.35, alpha);
    }
}

fn update_slash_previews(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut SlashPreviewVisual, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut preview, mut sprite) in &mut query {
        preview.age += dt;
        if preview.age >= preview.duration {
            commands.entity(entity).despawn();
            continue;
        }
        let alpha = 0.80 * (1.0 - preview.age / preview.duration);
        sprite.color = Color::srgba(1.0, 1.0, 0.35, alpha);
    }
}

fn update_hud(
    runtime: Res<SandboxRuntime>,
    world: Res<GameWorld>,
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
        "{}\nroom: {}  size {:.0}x{:.0}\nvel: ({:+.1}, {:+.1}) speed {:.1} max {:.1}\ngrounded: {} wall: {} dash: {} air_jumps: {} coyote {:.2} buffer {:.2}\ncombo: {}\nhint: {}\npreset: {} | movement: {} | {}\nF9/F10 presets  F1 debug  F2 slowmo={}  Esc pause={}  Delete reset  hitstop {:.2}\ndummies: {}\ngamepad target: {}{}",
        world.0.name,
        "Bevy backend",
        world.0.size.x,
        world.0.size.y,
        runtime.player.vel.x,
        runtime.player.vel.y,
        runtime.player.vel.length(),
        runtime.player.max_speed,
        runtime.player.on_ground,
        runtime.player.on_wall,
        runtime.player.dash_available,
        runtime.player.air_jumps_available,
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
        dummies,
        gamepad,
        flash_line,
    );
}

fn spawn_grid(commands: &mut Commands, world: &ae::World) {
    let grid_color = Color::srgba(0.12, 0.15, 0.22, 0.28);
    let step = 80.0;
    let mut x = 0.0;
    while x <= world.size.x {
        let center = ae::Vec2::new(x, world.size.y * 0.5);
        commands.spawn((
            Sprite::from_color(grid_color, BVec2::new(1.0, world.size.y)),
            Transform::from_translation(world_to_bevy(world, center, -20.0)),
        ));
        x += step;
    }
    let mut y = 0.0;
    while y <= world.size.y {
        let center = ae::Vec2::new(world.size.x * 0.5, y);
        commands.spawn((
            Sprite::from_color(grid_color, BVec2::new(world.size.x, 1.0)),
            Transform::from_translation(world_to_bevy(world, center, -20.0)),
        ));
        y += step;
    }
}

fn spawn_block(commands: &mut Commands, world: &ae::World, block: &ae::Block) {
    let size = block.aabb.half * 2.0;
    commands.spawn((
        Sprite::from_color(block_color(block.kind), BVec2::new(size.x, size.y)),
        Transform::from_translation(world_to_bevy(world, block.aabb.center, WORLD_Z_BLOCK)),
    ));
}

fn spawn_slash_preview(commands: &mut Commands, world: &ae::World, hitbox: ae::Aabb) {
    let size = hitbox.half * 2.0;
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 1.0, 0.35, 0.80), BVec2::new(size.x, size.y)),
        Transform::from_translation(world_to_bevy(world, hitbox.center, WORLD_Z_FX + 2.0)),
        SlashPreviewVisual { age: 0.0, duration: 0.10 },
    ));
}

fn spawn_impact(commands: &mut Commands, world: &ae::World, pos: ae::Vec2) {
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 1.0, 0.35, 0.82), BVec2::splat(12.0)),
        Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX + 1.0)),
        ImpactVisual {
            pos,
            age: 0.0,
            duration: 0.24,
            radius: 12.0,
        },
    ));
}

fn spawn_reset_effects(commands: &mut Commands, world: &ae::World, from: ae::Vec2, to: ae::Vec2) {
    // Reset is a teleport-like state transition. Showing both endpoints avoids
    // the ambiguity where a burst at spawn can look like a coordinate bug when
    // the player reset from somewhere else.
    if (from - to).length() > 8.0 {
        spawn_burst(
            commands,
            world,
            from,
            10,
            180.0,
            [0.32, 0.48, 0.70, 0.52],
            ParticleKind::Dust,
        );
    }
    spawn_burst(
        commands,
        world,
        to,
        24,
        280.0,
        [0.55, 0.85, 1.0, 0.90],
        ParticleKind::Spark,
    );
    spawn_impact(commands, world, to);
}

fn spawn_burst(
    commands: &mut Commands,
    world: &ae::World,
    pos: ae::Vec2,
    count: usize,
    speed: f32,
    color_rgba: [f32; 4],
    kind: ParticleKind,
) {
    let count = count.max(1);
    for i in 0..count {
        let t = i as f32 / count as f32;
        let wobble = ((i * 37 + 17) as f32).sin() * 0.22;
        let angle = TAU * t + wobble;
        let strength = speed * (0.45 + 0.55 * ((i * 13 + 5) % 11) as f32 / 10.0);
        let vel = ae::Vec2::new(angle.cos() * strength, angle.sin() * strength);
        let radius = 2.0 + 2.5 * ((i * 5 + 1) % 7) as f32 / 6.0;
        let lifetime = 0.22 + 0.16 * ((i * 7 + 3) % 9) as f32 / 8.0;
        commands.spawn((
            Sprite::from_color(rgba(color_rgba[0], color_rgba[1], color_rgba[2], color_rgba[3]), BVec2::splat(radius)),
            Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX)),
            ParticleVisual {
                kind,
                pos,
                vel,
                age: 0.0,
                lifetime,
                radius,
                rgba: color_rgba,
                gravity: match kind {
                    ParticleKind::Spark => 300.0,
                    ParticleKind::Dust => 120.0,
                    ParticleKind::Shard => 650.0,
                },
                drag: match kind {
                    ParticleKind::Spark => 3.4,
                    ParticleKind::Dust => 4.7,
                    ParticleKind::Shard => 1.8,
                },
            },
        ));
    }
}

fn spawn_dust(commands: &mut Commands, world: &ae::World, pos: ae::Vec2, facing: f32) {
    for i in 0..6 {
        let lateral = -facing * (75.0 + i as f32 * 18.0);
        let upward = -35.0 - i as f32 * 8.0;
        let radius = 3.5 + i as f32 * 0.35;
        commands.spawn((
            Sprite::from_color(Color::srgba(0.58, 0.62, 0.72, 0.75), BVec2::splat(radius)),
            Transform::from_translation(world_to_bevy(world, pos, WORLD_Z_FX)),
            ParticleVisual {
                kind: ParticleKind::Dust,
                pos,
                vel: ae::Vec2::new(lateral, upward),
                age: 0.0,
                lifetime: 0.28 + 0.03 * i as f32,
                radius,
                rgba: [0.58, 0.62, 0.72, 0.75],
                gravity: 80.0,
                drag: 4.4,
            },
        ));
    }
}

fn slash_hitbox(player: &ae::Player, axis_y: f32, forced_pogo: bool) -> ae::Aabb {
    let body = player.aabb();
    if forced_pogo || axis_y > 0.25 {
        ae::Aabb::new(
            ae::Vec2::new(body.center.x, body.bottom() + 24.0),
            ae::Vec2::new(body.half.x * 0.95, 26.0),
        )
    } else if axis_y < -0.25 {
        ae::Aabb::new(
            ae::Vec2::new(body.center.x, body.top() - 22.0),
            ae::Vec2::new(body.half.x * 1.10, 24.0),
        )
    } else {
        ae::Aabb::new(
            ae::Vec2::new(body.center.x + player.facing * (body.half.x + 30.0), body.center.y - 2.0),
            ae::Vec2::new(34.0, 24.0),
        )
    }
}

fn play_sound(commands: &mut Commands, bank: &SoundBank, cue: SoundCue) {
    commands.spawn((
        AudioPlayer::new(bank.get(cue)),
        PlaybackSettings::DESPAWN,
    ));
}

fn block_color(kind: ae::BlockKind) -> Color {
    match kind {
        ae::BlockKind::Solid => Color::srgba(0.25, 0.28, 0.36, 1.0),
        ae::BlockKind::OneWay => Color::srgba(0.36, 0.43, 0.62, 0.92),
        ae::BlockKind::Hazard => Color::srgba(0.96, 0.18, 0.26, 0.92),
        ae::BlockKind::PogoOrb => Color::srgba(0.30, 0.95, 0.64, 0.95),
        ae::BlockKind::Rebound { .. } => Color::srgba(1.0, 0.60, 0.20, 0.95),
    }
}

fn dummy_color(dummy: &Dummy) -> Color {
    if dummy.hit_flash > 0.0 {
        return Color::srgba(1.0, 1.0, 1.0, 1.0);
    }
    match dummy.kind {
        DummyKind::InfiniteSandbag => Color::srgba(0.78, 0.62, 0.42, 1.0),
        DummyKind::FiniteRespawner => Color::srgba(0.86, 0.38, 0.90, 1.0),
    }
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color::srgba(r, g, b, a.clamp(0.0, 1.0))
}

fn world_to_bevy(world: &ae::World, p: ae::Vec2, z: f32) -> Vec3 {
    Vec3::new(p.x - world.size.x * 0.5, world.size.y * 0.5 - p.y, z)
}

fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}

fn sample_wave(phase: f32, waveform: Waveform) -> f32 {
    let p = phase.fract();
    match waveform {
        Waveform::Sine => (p * TAU).sin(),
        Waveform::Square => {
            if p < 0.5 { 1.0 } else { -1.0 }
        }
        Waveform::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
        Waveform::Saw => 2.0 * p - 1.0,
    }
}

fn envelope(index: usize, length: usize, attack: usize, release: usize) -> f32 {
    if attack > 0 && index < attack {
        return index as f32 / attack as f32;
    }
    if release > 0 && index >= length.saturating_sub(release) {
        return (length.saturating_sub(index)) as f32 / release as f32;
    }
    1.0
}

fn synth_wav_bytes(spec: SynthSpec, sample_rate: u32) -> Vec<u8> {
    let sample_count = ((spec.duration * sample_rate as f32).max(1.0)) as usize;
    let attack = (spec.attack * sample_rate as f32) as usize;
    let release = (spec.release * sample_rate as f32) as usize;
    let mut pcm: Vec<i16> = Vec::with_capacity(sample_count * 2);
    let mut phase = 0.0f32;
    let mut noise_state = 0x1234_5678u32;
    for i in 0..sample_count {
        let t = if sample_count > 1 {
            i as f32 / (sample_count - 1) as f32
        } else {
            0.0
        };
        let freq = spec.frequency + (spec.frequency_end - spec.frequency) * t;
        phase += freq / sample_rate as f32;
        let mut sample = sample_wave(phase, spec.waveform);
        if spec.noise > 0.0 {
            noise_state = noise_state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (((noise_state >> 8) as f32 / 0x00ff_ffff as f32) * 2.0) - 1.0;
            sample = sample * (1.0 - spec.noise) + n * spec.noise;
        }
        sample *= envelope(i, sample_count, attack, release) * spec.volume;
        let v = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        pcm.push(v);
        pcm.push(v);
    }
    let data_bytes = (pcm.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_bytes as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2 * 2).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in pcm {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}
