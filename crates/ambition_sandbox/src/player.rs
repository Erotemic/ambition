//! ECS player components.
//!
//! The ECS player entity is the frame-to-frame authority for player movement,
//! health, combat timers, and interaction buffering. All player state lives on
//! ECS components; do not reintroduce a god-object runtime resource.

use ambition_engine as ae;
use bevy::prelude::*;

/// Marker for the single local player entity.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerEntity;

/// Frame-to-frame authoritative player movement state.
///
/// This is the single source of truth for `ae::Player` within the Bevy world.
/// All sandbox systems that read or write player movement/ability state must
/// go through this component.
#[derive(Component, Clone)]
pub struct PlayerMovementAuthority {
    pub player: ae::Player,
}

impl PlayerMovementAuthority {
    pub fn new(player: ae::Player) -> Self {
        Self { player }
    }

    pub fn body(&self) -> PlayerBody {
        PlayerBody::from_player(&self.player)
    }
}

/// ECS-visible player body.
///
/// The full engine `ae::Player` state lives on `PlayerMovementAuthority`; this
/// compact component is the query-friendly body/read model for systems that do
/// not need every movement-internal field.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerBody {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub size: ae::Vec2,
    pub base_size: ae::Vec2,
    pub facing: f32,
    pub on_ground: bool,
    pub fly_enabled: bool,
    pub dash_charges_available: u8,
    pub air_jumps_available: u8,
    pub mana_current: f32,
    pub body_mode: ae::BodyMode,
    pub invincible: bool,
    pub dodge_rolling: bool,
    /// True while the shield ability is active (button held, not dashing).
    /// Used by the sandbox to show the bubble visual.
    pub shielding: bool,
    /// True during the parry window: shield is active AND `parry_window_timer > 0`.
    /// Damage checks gate contact damage behind `!parrying`.
    pub parrying: bool,
}

impl PlayerBody {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            pos: player.pos,
            vel: player.vel,
            size: player.size,
            base_size: player.base_size,
            facing: player.facing,
            on_ground: player.on_ground,
            fly_enabled: player.fly_enabled,
            dash_charges_available: player.dash_charges_available,
            air_jumps_available: player.air_jumps_available,
            mana_current: player.mana.current,
            body_mode: player.body_mode,
            invincible: player.invincible,
            dodge_rolling: player.dodge_roll_timer > 0.0,
            shielding: player.shield_active,
            parrying: player.shield_active && player.parry_window_timer > 0.0,
        }
    }

    pub fn aabb(self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}

/// ECS-owned player health.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerHealth {
    pub health: ae::Health,
}

impl PlayerHealth {
    pub fn new(health: ae::Health) -> Self {
        Self { health }
    }

    pub fn current(self) -> i32 {
        self.health.current
    }

    pub fn max(self) -> i32 {
        self.health.max
    }

    pub fn heal(&mut self, amount: i32) {
        self.health.heal(amount);
    }

    pub fn damage(&mut self, amount: i32) -> bool {
        self.health.damage(amount)
    }

    pub fn reset(&mut self) {
        self.health.reset();
    }
}

/// ECS-authoritative player combat/timer state.
///
/// The four timer fields are written directly by the phase helpers and
/// `world_flow` functions that produce damage/hit/respawn events.
/// `write_player_ecs_components` no longer touches them; it only syncs the
/// `attacking` flag from `CurrentPlayerAttack` so rendering
/// systems can check attack state without querying the runtime.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct PlayerCombatState {
    /// Presentation flash (damage hit-blink). Decays in `cleanup_timers_phase`.
    pub flash_timer: f32,
    /// Hitstop: freezes `time_scale` to 0 while positive. Decays in `input_timer_phase`.
    pub hitstop_timer: f32,
    /// Invulnerability window after taking damage. Decays in `input_timer_phase`.
    pub damage_invuln_timer: f32,
    /// Partial-control penalty after knockback. Decays in `input_timer_phase`.
    pub hitstun_timer: f32,
    /// Mirrored each frame from `CurrentPlayerAttack::is_some()`.
    pub attacking: bool,
}

impl PlayerCombatState {
    pub fn vulnerable(&self) -> bool {
        self.damage_invuln_timer <= 0.0
    }

    pub fn reset(&mut self) {
        self.flash_timer = 0.0;
        self.hitstop_timer = 0.0;
        self.damage_invuln_timer = 0.0;
        self.hitstun_timer = 0.0;
        self.attacking = false;
    }
}

/// ECS-owned player animation signal timers.
///
/// All fields are presentation-only: they gate which sprite row plays and
/// decay independent of gameplay timers like hitstop or invulnerability.
/// Written directly by `cleanup_timers_phase` / `start_attack` /
/// `advance_attack`; `animate_player` reads them via `pick_player_anim`.
/// This is the authoritative source — `write_player_ecs_components` does
/// not touch it.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct PlayerAnimState {
    /// Time remaining for the slash animation row.
    pub slash_anim_timer: f32,
    /// Time remaining for the post-touchdown landing pose.
    pub land_anim_timer: f32,
    /// True when the landing was fast enough for the hard-impact row.
    pub land_anim_hard: bool,
    /// Time remaining for the brief dash pre-roll pose.
    pub dash_startup_timer: f32,
    /// Previous frame's `on_ground`; used to detect the touchdown edge.
    pub anim_prev_on_ground: bool,
    /// Previous frame's pre-landing downward velocity; used to grade
    /// hard vs. soft landings.
    pub anim_prev_vel_y: f32,
    /// Previous frame's `dash_timer`; used to detect the dash rising edge.
    pub anim_prev_dash_timer: f32,
}

impl PlayerAnimState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// ECS-visible player interaction buffer state.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerInteractionState {
    /// Counts down after a double-tap-down edge; non-zero means morph-ball
    /// entry is pending for the body-mode driver.
    pub down_tap_timer: f32,
    /// Counts down after a double-tap-up edge; drives door/NPC triggers.
    pub up_tap_timer: f32,
    /// Counts down after `interact_pressed`; keeps the interact signal alive
    /// across frames so the player doesn't need to hold the button until the
    /// door animation completes.
    pub interact_buffer_timer: f32,
    /// Set true by the input-timer phase when a double-tap-down is detected;
    /// consumed by the body-mode driver after `sandbox_update`.
    pub double_tap_down_pending: bool,
    /// Set true by `input_timer_system` when a double-tap-up gesture is
    /// detected; consumed by `interaction_input_phase` (or its future
    /// extracted system) to activate door/NPC triggers. Cleared after use.
    pub double_tap_up_pending: bool,
}

impl PlayerInteractionState {
    /// Advance timers and detect a double-tap-down edge. Returns `true` when
    /// two taps arrive within `window` seconds.
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

    /// Advance timers and detect a double-tap-up edge. Returns `true` when
    /// two taps arrive within `window` seconds.
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

    /// Update the interact buffer and return whether the buffer is live.
    pub fn buffered_interact(&mut self, pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.interact_buffer_timer = (self.interact_buffer_timer - frame_dt).max(0.0);
        if pressed {
            self.interact_buffer_timer = window;
        }
        self.interact_buffer_timer > 0.0
    }

    pub fn buffered(self) -> bool {
        self.interact_buffer_timer > 0.0
    }

    pub fn clear(&mut self) {
        self.interact_buffer_timer = 0.0;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Camera easing and blink-in presentation state. Authoritative ECS component;
/// written by `cleanup_timers_phase`, `load_room`, and `handle_player_events`
/// (blink path). Read by the camera follow system and the sprite animator.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerBlinkCameraState {
    /// Counts down from `blink_in_duration` to 0 after a blink; the camera
    /// and animator use this to play the arrival ease-in.
    pub blink_in_timer: f32,
    /// Set to `BLINK_IN_ANIM_TIME` when a blink fires; used to normalise
    /// `blink_in_timer` into a 0..1 progress value.
    pub blink_in_duration: f32,
    /// World-space camera position at the moment the blink fired; the camera
    /// eases from here toward the new player position.
    pub blink_camera_from: ambition_engine::Vec2,
    /// Blink destination in world space (set alongside `blink_camera_from`
    /// for future use; not yet consumed by the camera easing path).
    pub blink_camera_to: ambition_engine::Vec2,
    /// Positive while the camera should snap (not ease) to the player position.
    /// Set on door transitions; zero on edge exits to allow scroll effects.
    pub camera_snap_timer: f32,
}

impl Default for PlayerBlinkCameraState {
    fn default() -> Self {
        Self {
            blink_in_timer: 0.0,
            blink_in_duration: 0.0,
            blink_camera_from: ambition_engine::Vec2::ZERO,
            blink_camera_to: ambition_engine::Vec2::ZERO,
            camera_snap_timer: 0.0,
        }
    }
}

impl PlayerBlinkCameraState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Typed heal request message for gameplay heal events.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerHealRequested {
    pub amount: i32,
}

impl PlayerHealRequested {
    pub fn new(amount: i32) -> Self {
        Self { amount }
    }
}

/// Damage already travels through the feature-domain rich message. This alias
/// documents that the same message is the player damage request seam for the
/// first player ECS migration chunk.
pub type PlayerDamageRequested = crate::features::PlayerDamageEvent;

/// Write `PlayerBody` and `PlayerCombatState::attacking` from the authoritative
/// sources each frame.
///
/// `PlayerBody` is a snapshot of `PlayerMovementAuthority::player`.
/// `attacking` mirrors whether `CurrentPlayerAttack` has an active swing.
pub fn write_player_ecs_components(
    attack_res: Res<crate::CurrentPlayerAttack>,
    mut players: Query<
        (
            &PlayerMovementAuthority,
            &mut PlayerBody,
            &mut PlayerCombatState,
        ),
        With<PlayerEntity>,
    >,
) {
    let Ok((authority, mut body, mut combat)) = players.single_mut() else {
        return;
    };
    *body = PlayerBody::from_player(&authority.player);
    combat.attacking = attack_res.0.is_some();
}

/// Apply heal messages to the authoritative `PlayerHealth` ECS component.
pub fn apply_player_heal_requests(
    mut heals: MessageReader<PlayerHealRequested>,
    mut players: Query<&mut PlayerHealth, With<PlayerEntity>>,
) {
    let Ok(mut health) = players.single_mut() else {
        // No player entity yet (startup or headless): drain the queue.
        for _ in heals.read() {}
        return;
    };
    for heal in heals.read() {
        if heal.amount > 0 {
            health.heal(heal.amount);
        }
    }
}
