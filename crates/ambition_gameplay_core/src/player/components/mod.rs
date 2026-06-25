//! Player ECS components.
//!
//! The player entity carries one of each of these as its frame-to-frame
//! authoritative state. See [`super::bundles::PlayerSimulationBundle`] for
//! the canonical spawn shape.

use crate::engine_core as ae;
use bevy::prelude::*;

use ambition_input::ControlFrame;

// Re-export generic player markers from the platformer runtime.
pub use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};
// Stable facade for the player-slot marker used by brain/player code.
pub use crate::brain::PlayerSlot;

/// Marks a player whose input comes from this machine's input devices
/// (keyboard / gamepad / touch). In single-player today the local
/// player is also the primary player. In a future networked build,
/// remote players would have `PlayerEntity` (+ `PlayerSlot`) but not
/// `LocalPlayer`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalPlayer;

/// Per-player input snapshot. Mirrors the single global
/// [`ambition_input::ControlFrame`] resource onto the local player
/// entity so simulation systems can move toward reading input from a
/// `Query<&PlayerInputFrame>` rather than `Res<ControlFrame>`. That's
/// the architectural seam multiplayer / netcode work needs:
///
/// - the local primary player's frame is filled by
///   `sync_local_player_input_frame` after the input pipeline writes
///   `Res<ControlFrame>`;
/// - future remote / co-op players would have their own
///   `PlayerInputFrame` populated by a network adapter or a second
///   input device, without competing for the single global resource.
///
/// Today exactly one entity (the local primary player) carries this
/// component, and `Res<ControlFrame>` stays the single writer channel.
/// New simulation systems should prefer this component so they're
/// already shaped for multiple input-bearing players.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlayerInputFrame {
    pub frame: ControlFrame,
}

/// ECS-owned player health.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerHealth {
    pub health: crate::actor::Health,
}

impl PlayerHealth {
    pub fn new(health: crate::actor::Health) -> Self {
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

/// Player money — abstract coin/credits balance shown on the HUD and spent at
/// merchants. Fed by `PickupKind::Currency` collection (`collect_ecs_pickups`).
/// Decided (Jon): a coin/credits wallet, not item-as-currency.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerWallet {
    pub balance: i32,
}

impl PlayerWallet {
    /// Credit the wallet (clamped at zero so a negative `amount` can't drive it
    /// below zero).
    pub fn add(&mut self, amount: i32) {
        self.balance = (self.balance + amount).max(0);
    }

    /// Spend `amount` if affordable; returns `true` and debits on success.
    pub fn try_spend(&mut self, amount: i32) -> bool {
        if amount >= 0 && self.balance >= amount {
            self.balance -= amount;
            true
        } else {
            false
        }
    }
}

/// ECS-authoritative player combat/timer state.
///
/// The four timer fields are written directly by the phase helpers and
/// `world_flow` functions that produce damage/hit/respawn events.
/// `write_player_ecs_components` no longer touches them; it only syncs the
/// `attacking` flag from the per-player `ActivePlayerAttack` component so
/// rendering systems can check attack state without querying the runtime.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct PlayerCombatState {
    /// Presentation flash (damage hit-blink). Decays in `cleanup_timers_system`.
    pub flash_timer: f32,
    /// Hitstop: freezes `time_scale` to 0 while positive. Decays in `input_timer_system`.
    pub hitstop_timer: f32,
    /// Invulnerability window after taking damage. Decays in `input_timer_system`.
    pub damage_invuln_timer: f32,
    /// Partial-control penalty after knockback. Decays in `input_timer_system`.
    pub hitstun_timer: f32,
    /// Short HARD control-lock at the start of a knockback (the recoil throw).
    /// While positive, the player has NO input authority — it is the gate that
    /// suppresses movement/flight steering and the attack-start, so the
    /// knockback ejects the player before they can act. Set to
    /// `SandboxFeelTuning::knockback_recoil_lock_time` on a knockback hit and
    /// decayed in `input_timer_system`; once it hits zero the player can swing
    /// again even though `hitstun_timer` / `damage_invuln_timer` are still up.
    pub recoil_lock_timer: f32,
    /// Mirrored each frame from `ActivePlayerAttack::is_active()`.
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
        self.recoil_lock_timer = 0.0;
        self.attacking = false;
    }
}

/// Per-player active melee swing. `None` when no swing is in progress.
///
/// Authoritative source: set/cleared by `start_attack` / `advance_attack`.
/// `write_player_ecs_components` mirrors `is_some()` into
/// `PlayerCombatState::attacking` each frame so rendering can branch on
/// attack state without a separate query.
///
/// Replaces the global `CurrentPlayerAttack` resource (OVERNIGHT-TODO
/// #17.4 / the multiplayer caveat that used to live in `lib.rs`). Each
/// player entity carries its own attack state, so a future co-op /
/// split-screen build can spawn additional players whose swings tick
/// independently.
#[derive(Component, Clone, Debug, Default)]
pub struct ActivePlayerAttack(pub Option<super::super::PlayerAttackState>);

impl ActivePlayerAttack {
    pub fn is_active(&self) -> bool {
        self.0.is_some()
    }

    pub fn clear(&mut self) {
        self.0 = None;
    }
}

/// ECS-owned player animation signal timers.
///
/// All fields are presentation-only: they gate which sprite row plays and
/// decay independent of gameplay timers like hitstop or invulnerability.
/// Written directly by `cleanup_timers_system` / `start_attack` /
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
    /// Time remaining for the projectile-release `Shoot` pose. Armed by
    /// `update_projectiles` whenever a projectile body is spawned (any
    /// kind — Fireball/Hadouken/HadoukenSuper). Single-shot, short.
    pub shoot_anim_timer: f32,
    /// Set each frame by `update_projectiles` to mirror
    /// `PlayerProjectileState.charging.is_some()`. While true the
    /// player is holding a charge and the `Aim` row plays.
    pub aim_anim_active: bool,
    /// Time remaining for the wall-jump push-off pose. Armed by
    /// `handle_player_events` on a `MovementOp::WallJump` op. Distinct
    /// from `Jump` so the wall departure reads as a kick-off rather
    /// than a ground arc.
    pub wall_jump_anim_timer: f32,
    /// Time remaining for the interact-gesture pose. Armed when an
    /// interaction (door, NPC, pickup) consumes
    /// `interact_buffer_timer`.
    pub interact_anim_timer: f32,
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
    /// Set true by `input_timer_system` when a double-tap-down is detected;
    /// consumed by the body-mode driver after the player tick.
    pub double_tap_down_pending: bool,
    /// Set true by `input_timer_system` when a double-tap-up gesture is
    /// detected; consumed (via `mem::take`) by `interaction_input_system`
    /// the same frame to fold it into the hit-stun-gated interact buffer
    /// that drives door / NPC / chest activation.
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
/// written by `cleanup_timers_system`, `load_room`, and `handle_player_events`
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
    pub blink_camera_from: crate::engine_core::Vec2,
    /// Blink destination in world space (set alongside `blink_camera_from`
    /// for future use; not yet consumed by the camera easing path).
    pub blink_camera_to: crate::engine_core::Vec2,
    /// Positive while the camera should snap (not ease) to the player position.
    /// Set on door transitions; zero on edge exits to allow scroll effects.
    pub camera_snap_timer: f32,
}

impl Default for PlayerBlinkCameraState {
    fn default() -> Self {
        Self {
            blink_in_timer: 0.0,
            blink_in_duration: 0.0,
            blink_camera_from: crate::engine_core::Vec2::ZERO,
            blink_camera_to: crate::engine_core::Vec2::ZERO,
            camera_snap_timer: 0.0,
        }
    }
}

impl PlayerBlinkCameraState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Per-player "last known safe spot" used by hazard knockback and debug
/// respawn helpers. Stored on each player so future co-op builds keep safe
/// anchors independent.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerSafetyState {
    /// Last grounded, gameplay-safe position the safety gate
    /// approved (see `crate::remember_safe_player_position`). The
    /// hazard / OOB respawn path warps the player here.
    pub last_safe_pos: ae::Vec2,
}

impl PlayerSafetyState {
    pub fn new(initial: ae::Vec2) -> Self {
        Self {
            last_safe_pos: initial,
        }
    }
}

#[cfg(test)]
mod multiplayer_smoke_tests;
