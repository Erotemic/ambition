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
pub mod boss_encounter;
pub mod boss_sprites;
pub mod character_sprites;
pub mod config;
pub mod cutscene;
pub mod data;
pub mod debug_overlay;
pub mod dev_tools;
pub mod dialog;
pub mod encounter;
pub mod features;
pub mod quest;
pub mod feel;
pub mod fx;
pub mod game_assets;
pub mod game_mode;
pub mod input;
pub mod inventory;
pub mod ldtk_world;
pub mod ledge_grab;
pub mod loading;
pub mod map_menu;
pub mod room_builder;
pub mod swim;
pub mod mechanics;
pub mod pause_menu;
pub mod physics;
pub mod platforms;
pub mod projectile;
pub mod rendering;
pub mod rooms;
pub mod save;
pub mod settings;
pub mod trace;
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

/// Per-frame conditions that gate writes to `SandboxRuntime::last_safe_player_pos`.
/// We refuse to record a position as "safe" while any of these flags are
/// set so an in-flight reset / hazard respawn / room transition cannot
/// pollute the safe spawn point. Construct with [`SafePositionContext::frame`].
#[derive(Clone, Copy, Debug)]
pub struct SafePositionContext {
    /// True if the player took damage this frame.
    pub damaged_this_frame: bool,
    /// True if hitstun is active (player has reduced control).
    pub in_hitstun: bool,
    /// True if a feature requested a player reset this frame.
    pub feature_requested_reset: bool,
    /// True if the post-blink grace timer is currently active.
    pub blink_grace_active: bool,
    /// True if a room transition fired or is cooling down this frame.
    pub room_transitioning: bool,
}

impl SafePositionContext {
    /// "All safe": no damage, no hitstun, no reset, no blink grace, no
    /// transition. Useful for tests.
    pub fn ideal() -> Self {
        Self {
            damaged_this_frame: false,
            in_hitstun: false,
            feature_requested_reset: false,
            blink_grace_active: false,
            room_transitioning: false,
        }
    }

    pub fn is_eligible(&self) -> bool {
        !self.damaged_this_frame
            && !self.in_hitstun
            && !self.feature_requested_reset
            && !self.blink_grace_active
            && !self.room_transitioning
    }
}

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
    /// Sandbox-side player mana. Not yet a real engine resource (no
    /// consuming ability); the F3 stats editor reads/writes it so
    /// future abilities can tap a single canonical store. Defaults
    /// to `mana_max` on construction.
    pub mana_current: i32,
    pub mana_max: i32,
    /// Multiplier on outgoing player slash damage. The F3 editor
    /// writes this; `process_attack` reads it. Default 1.
    pub slash_damage: i32,
    /// True → all incoming damage is dropped. Set by the F3 stats
    /// editor's "invincible" toggle.
    pub invincible: bool,
    /// Ledge grab state. `Some` while the player is hanging on a
    /// ledge — gravity is suspended and Up + Jump kicks off the
    /// climb. `None` otherwise. Only mutated by `update_ledge_grab`.
    pub ledge_grab: Option<LedgeGrabState>,
    /// One-shot signal: set true the frame the player died and was
    /// respawned. The encounter system reads this on its next tick
    /// to fail any active encounter (despawn mobs, drop the lock,
    /// re-arm the trigger). Cleared by the encounter system after
    /// it acts.
    pub player_died_pending: bool,
}

/// Sandbox-side ledge grab snapshot. Engine-pure data wrapped with
/// the timer the climb animation drains.
#[derive(Clone, Copy, Debug)]
pub struct LedgeGrabState {
    pub contact: ae::LedgeContact,
    /// Seconds since the cling-snap fired. Used by the climb
    /// animation; doesn't gate anything yet.
    pub elapsed: f32,
    /// True once the climb has been requested (Up + Jump). The next
    /// frame moves the player to `contact.climb_target` and clears
    /// the state.
    pub climbing: bool,
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
            mana_current: 100,
            mana_max: 100,
            slash_damage: 1,
            invincible: false,
            ledge_grab: None,
            player_died_pending: false,
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
        // Refill mana on reset; preserve the editor-tuned slash damage
        // / invincible flag so testers don't lose their inspector
        // settings on every player respawn.
        self.mana_current = self.mana_max;
        self.ledge_grab = None;
        // `player_died_pending` is intentionally NOT cleared here —
        // it's set by `death_respawn_player` (which calls `reset`)
        // so the encounter system can read it on the next tick.
        // Clearing it would defeat the signal.
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

    /// Record the current player position as "the last known safe spot"
    /// when (and only when) every predicate of safety holds. Call sites
    /// pass the same augmented collision world the engine simulated
    /// against this frame so the gate matches reality.
    ///
    /// The flags allow the caller to suppress this write during damage
    /// resolution, hazard respawn, hitstun, post-blink grace, or room
    /// transitions where the player position is intentionally being
    /// teleported and shouldn't be remembered as safe. See
    /// `docs/lessons_learned.md` for the OOB trace where a wall-cling
    /// teleport polluted `last_safe_player_pos` with `(62, -23)`.
    pub fn remember_safe_player_position(&mut self, world: &ae::World, ctx: SafePositionContext) {
        if !self.player.on_ground {
            return;
        }
        if !ctx.is_eligible() {
            return;
        }
        let verdict = ae::classify_player_safety(&self.player, world, 0.0, |block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
            )
        });
        if verdict.is_safe() {
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

/// Live camera scale + ease state. The camera reads target scale from
/// the encounter registry (or developer overview override) every
/// frame; this resource holds the smoothed value so transitions feel
/// like a breath instead of a snap.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CameraEaseState {
    pub live_scale: f32,
}

impl Default for CameraEaseState {
    fn default() -> Self {
        Self { live_scale: 1.0 }
    }
}

/// Scale-units per second when easing camera *into* an encounter
/// (zoom-out). Faster than the recovery rate so the player feels the
/// arena widen quickly when the lock-wall slams.
pub const CAMERA_ZOOM_OUT_RATE: f32 = 1.6;

/// Scale-units per second when easing camera *out of* an encounter
/// (zoom-in). Slower than zoom-out; the post-fight breathing room is
/// the moment to savor.
pub const CAMERA_ZOOM_IN_RATE: f32 = 0.9;

#[cfg(test)]
mod safe_pos_tests {
    use super::*;
    use ambition_engine::Block;

    fn dummy_world() -> ae::World {
        ae::World::new(
            "test",
            ae::Vec2::new(1800.0, 1800.0),
            ae::Vec2::new(170.0, 1695.0),
            vec![Block::solid(
                "left wall",
                ae::Vec2::new(0.0, 0.0),
                ae::Vec2::new(36.0, 1800.0),
            )],
        )
    }

    fn runtime_with_player_at(world: &ae::World, pos: ae::Vec2) -> SandboxRuntime {
        let mut runtime = SandboxRuntime::new(
            world,
            ae::AbilitySet::sandbox_all(),
            ae::DEFAULT_TUNING,
            physics::PhysicsSandboxSettings::default(),
        );
        // Force a known starting "safe pos" we can detect changes from.
        runtime.last_safe_player_pos = ae::Vec2::new(170.0, 1695.0);
        runtime.player.pos = pos;
        runtime.player.on_ground = true;
        runtime
    }

    /// The OOB y=-23 position (above the world envelope) must NOT be
    /// recorded as safe even though `on_ground` is true. This is the
    /// invariant the wall-cling teleport bug violated for two consecutive
    /// reproductions before the fix.
    #[test]
    fn rejects_position_above_world_envelope() {
        let world = dummy_world();
        let mut runtime = runtime_with_player_at(&world, ae::Vec2::new(62.0, -23.0));
        let initial = runtime.last_safe_player_pos;
        runtime.remember_safe_player_position(&world, SafePositionContext::ideal());
        assert_eq!(
            runtime.last_safe_player_pos, initial,
            "above-world position must not become last_safe_player_pos"
        );
    }

    /// The position update should fire when the player is grounded inside
    /// the world envelope and not overlapping a Solid.
    #[test]
    fn accepts_legitimate_grounded_position() {
        let world = dummy_world();
        let mut runtime = runtime_with_player_at(&world, ae::Vec2::new(200.0, 900.0));
        runtime.remember_safe_player_position(&world, SafePositionContext::ideal());
        assert_eq!(
            runtime.last_safe_player_pos,
            ae::Vec2::new(200.0, 900.0),
            "a legal grounded position should be remembered"
        );
    }

    /// Even if the player is grounded somewhere legitimate, an in-flight
    /// reset / damage / hitstun / blink / room transition must veto the
    /// write so the safe pos doesn't drift while the player is being
    /// teleported.
    #[test]
    fn vetoes_write_during_damage_or_reset() {
        let world = dummy_world();
        let mut runtime = runtime_with_player_at(&world, ae::Vec2::new(200.0, 900.0));
        let initial = runtime.last_safe_player_pos;
        let mut ctx = SafePositionContext::ideal();
        ctx.damaged_this_frame = true;
        runtime.remember_safe_player_position(&world, ctx);
        assert_eq!(runtime.last_safe_player_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.feature_requested_reset = true;
        runtime.remember_safe_player_position(&world, ctx);
        assert_eq!(runtime.last_safe_player_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.in_hitstun = true;
        runtime.remember_safe_player_position(&world, ctx);
        assert_eq!(runtime.last_safe_player_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.blink_grace_active = true;
        runtime.remember_safe_player_position(&world, ctx);
        assert_eq!(runtime.last_safe_player_pos, initial);

        let mut ctx = SafePositionContext::ideal();
        ctx.room_transitioning = true;
        runtime.remember_safe_player_position(&world, ctx);
        assert_eq!(runtime.last_safe_player_pos, initial);
    }

    /// A position INSIDE a Solid block is not safe even if `on_ground`
    /// is true. Mirror of `classify_player_safety`'s InsideSolid case.
    #[test]
    fn rejects_position_inside_solid() {
        let world = dummy_world();
        // The left wall's right edge is at x=36; place the player center
        // at x=18 with half-width 14, body covers x=4..32 — fully inside
        // the wall.
        let mut runtime = runtime_with_player_at(&world, ae::Vec2::new(18.0, 900.0));
        let initial = runtime.last_safe_player_pos;
        runtime.remember_safe_player_position(&world, SafePositionContext::ideal());
        assert_eq!(runtime.last_safe_player_pos, initial);
    }
}
