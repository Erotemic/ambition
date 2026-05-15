//! ECS player seam.
//!
//! The ECS player entity is the frame-to-frame authority for player movement,
//! health, combat timers, and interaction buffering. `SandboxRuntime` still
//! carries a legacy player scratch copy while the old phase helpers are split
//! into standalone ECS systems.

use ambition_engine as ae;
use bevy::prelude::*;

/// Marker for the single local player entity.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerEntity;

/// Frame-to-frame authoritative player movement state.
///
/// This intentionally wraps the engine player during the authority flip. The
/// legacy `SandboxRuntime::player` field is synchronized from this component at
/// the start of the gameplay chain and synchronized back after the old phase
/// helpers run, making the runtime field a scratch adapter rather than the
/// durable owner.
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
///
/// Movement still mirrors from `SandboxRuntime::player`, but health is the
/// first player domain that can be mutated through ECS systems/messages and
/// mirrored back into the runtime bridge for legacy callers.
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

/// ECS-visible player combat/timer state. Written every frame by
/// `write_player_ecs_components` from `SandboxRuntime` combat timers.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PlayerCombatState {
    pub flash_timer: f32,
    pub hitstop_timer: f32,
    pub damage_invuln_timer: f32,
    pub hitstun_timer: f32,
    pub attacking: bool,
}

impl PlayerCombatState {
    pub fn from_runtime(runtime: &crate::SandboxRuntime) -> Self {
        Self {
            flash_timer: runtime.flash_timer,
            hitstop_timer: runtime.hitstop_timer,
            damage_invuln_timer: runtime.damage_invuln_timer,
            hitstun_timer: runtime.hitstun_timer,
            attacking: runtime.player_attack.is_some(),
        }
    }

    pub fn vulnerable(&self) -> bool {
        self.damage_invuln_timer <= 0.0
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
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerInteractionState {
    pub interact_buffer_timer: f32,
    pub double_tap_down_pending: bool,
}

impl PlayerInteractionState {
    pub fn from_runtime(runtime: &crate::SandboxRuntime) -> Self {
        Self {
            interact_buffer_timer: runtime.interact_buffer_timer,
            double_tap_down_pending: runtime.double_tap_down_pending,
        }
    }

    pub fn apply_to_runtime(self, runtime: &mut crate::SandboxRuntime) {
        runtime.interact_buffer_timer = self.interact_buffer_timer;
        runtime.double_tap_down_pending = self.double_tap_down_pending;
    }

    pub fn buffered(self) -> bool {
        self.interact_buffer_timer > 0.0
    }

    pub fn clear(&mut self) {
        self.interact_buffer_timer = 0.0;
    }
}

/// Typed heal request for producers that should not mutate `SandboxRuntime`
/// directly.
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

/// Write `PlayerBody`, `PlayerCombatState`, and `PlayerInteractionState` from
/// the authoritative sources each frame so rendering, hazard, and interaction
/// systems see current values instead of stale spawn-time data.
pub fn write_player_ecs_components(
    runtime: Res<crate::SandboxRuntime>,
    mut players: Query<
        (
            &PlayerMovementAuthority,
            &mut PlayerBody,
            &mut PlayerCombatState,
            &mut PlayerInteractionState,
        ),
        With<PlayerEntity>,
    >,
) {
    let Ok((authority, mut body, mut combat, mut interaction)) = players.single_mut() else {
        return;
    };
    *body = PlayerBody::from_player(&authority.player);
    *combat = PlayerCombatState::from_runtime(&runtime);
    *interaction = PlayerInteractionState::from_runtime(&runtime);
}

/// Apply heal messages to ECS health and mirror the result into the legacy
/// runtime scratch field for remaining callers.
pub fn apply_player_heal_requests(
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut heals: MessageReader<PlayerHealRequested>,
    mut players: Query<&mut PlayerHealth, With<PlayerEntity>>,
) {
    let Ok(mut health) = players.single_mut() else {
        for heal in heals.read() {
            if heal.amount > 0 {
                runtime.player_health.heal(heal.amount);
            }
        }
        return;
    };
    for heal in heals.read() {
        if heal.amount > 0 {
            health.heal(heal.amount);
        }
    }
    runtime.player_health = health.health;
}
