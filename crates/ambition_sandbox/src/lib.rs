//! Ambition sandbox library.
//!
//! The sandbox crate exposes both the playable Bevy app (`src/main.rs`) and a
//! headless simulation entry point (`run_headless`, used by `bin/headless.rs`
//! and tests/CI on machines without a display). Both binaries depend on this
//! library; the library owns the module graph and the cross-cutting types
//! (`GameWorld`, `SandboxRuntime`) that submodules reference via `crate::*`.
//!
//! See `docs/headless_simulation.md` for the sim/presentation contract this
//! library is being shaped toward, and `docs/architecture_targets.md` for the
//! longer-term events refactor that will let `sandbox_update` itself run
//! headless.

pub mod audio;
pub mod boss_sprites;
pub mod character_sprites;
pub mod config;
pub mod data;
pub mod debug_overlay;
pub mod dev_tools;
pub mod dialog;
pub mod features;
pub mod feel;
pub mod fx;
pub mod game_assets;
pub mod game_mode;
pub mod input;
pub mod inventory;
pub mod ldtk_world;
pub mod loading;
pub mod pause_menu;
pub mod physics;
pub mod platforms;
pub mod rendering;
pub mod rooms;
pub mod windowing;

pub mod app;
pub mod headless;
pub mod setup;

pub use game_mode::GameMode;
pub use headless::{run_headless, HeadlessReport};

use ambition_engine as ae;
use bevy::prelude::Resource;

use feel::SandboxFeelTuning;
use input::KeyboardPreset;

/// Active room's collision world, exposed as a Bevy resource.
///
/// Sandbox systems read collision through this wrapper so simulation logic
/// stays decoupled from how the world was authored. LDtk hot reload mutates
/// this resource as part of the transactional reload path.
#[derive(Resource, Clone)]
pub struct GameWorld(pub ae::World);

/// Sandbox-side runtime state mirroring per-player gameplay timers and the
/// tools/features it owns.
///
/// AMBITION_REVIEW: this is currently a global resource holding what belongs
/// on a Player entity. Per the architecture targets memory, per-player state
/// should migrate onto a Player component / entity once the events refactor
/// lands; a global `SandboxRuntime` is the SP-only shape that does not extend
/// to multi-player. The headless binary deliberately does not install this
/// resource — Phase 1 headless validates only the asset/world/spine pipeline,
/// not gameplay.
#[derive(Resource)]
pub struct SandboxRuntime {
    pub player: ae::Player,
    pub player_health: ae::Health,
    pub debug: bool,
    pub slowmo: bool,
    pub presets: Vec<KeyboardPreset>,
    pub preset_index: usize,
    pub preset_flash: f32,
    pub flash_timer: f32,
    pub hitstop_timer: f32,
    pub damage_invuln_timer: f32,
    pub hitstun_timer: f32,
    pub last_safe_player_pos: ae::Vec2,
    pub time_scale: f32,
    pub down_tap_timer: f32,
    pub up_tap_timer: f32,
    pub interact_buffer_timer: f32,
    pub moving_platform: platforms::MovingPlatformState,
    pub features: features::FeatureRuntime,
    pub dialogue: dialog::DialogState,
    pub physics_settings: physics::PhysicsSandboxSettings,
    pub room_transition_cooldown: f32,
    /// Time remaining on the player's slash animation. Set when an attack is
    /// triggered so the sprite plays the Slash row even after the brief
    /// hitstop window ends. Decays toward 0 in the gameplay loop.
    pub slash_anim_timer: f32,
}

impl SandboxRuntime {
    pub fn new(
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
            slash_anim_timer: 0.0,
        }
    }

    pub fn reset(&mut self, world: &ae::World, tuning: ae::MovementTuning) {
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
        self.slash_anim_timer = 0.0;
    }

    pub fn register_down_tap(&mut self, down_pressed: bool, frame_dt: f32, window: f32) -> bool {
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

    pub fn register_up_tap(&mut self, up_pressed: bool, frame_dt: f32, window: f32) -> bool {
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

    pub fn buffered_interact(
        &mut self,
        interact_pressed: bool,
        frame_dt: f32,
        window: f32,
    ) -> bool {
        self.interact_buffer_timer = (self.interact_buffer_timer - frame_dt).max(0.0);
        if interact_pressed {
            self.interact_buffer_timer = window;
        }
        self.interact_buffer_timer > 0.0
    }

    pub fn clear_interact_buffer(&mut self) {
        self.interact_buffer_timer = 0.0;
    }

    pub fn remember_safe_player_position(&mut self) {
        if self.player.on_ground {
            self.last_safe_player_pos = self.player.pos;
        }
    }

    pub fn update_time_scale(&mut self, frame_dt: f32, feel: SandboxFeelTuning) {
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
        let rate = if target < self.time_scale {
            feel.time_ramp_down_rate
        } else {
            feel.time_ramp_up_rate
        };
        self.time_scale = move_toward(self.time_scale, target, rate * frame_dt);
    }

    pub fn preset(&self) -> KeyboardPreset {
        self.presets[self.preset_index]
    }

    pub fn debug_enabled(&self) -> bool {
        self.debug
    }
}

/// Approach `target` from `value` by at most `delta`. Used for time-scale
/// ramping in `SandboxRuntime::update_time_scale`.
pub fn move_toward(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}
