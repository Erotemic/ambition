//! ECS player seam.
//!
//! Movement is still authored by `SandboxRuntime::player`; this module mirrors
//! the authoritative runtime body/health onto a Player entity so readers can
//! migrate to ECS queries before movement authority moves out of the resource.

use ambition_engine as ae;
use bevy::prelude::*;

/// Marker for the single local player entity.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerEntity;

/// ECS-visible player body read model.
///
/// This is intentionally a mirror for now. `SandboxRuntime::player` remains the
/// movement authority until the movement/collision update is migrated.
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

/// ECS-visible player combat/timer state.
///
/// This is authoritative for readers and mirrors back to the legacy runtime
/// bridge while movement authority is still in `SandboxRuntime::player`.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PlayerCombatState {
    pub flash_timer: f32,
    pub hitstop_timer: f32,
    pub damage_invuln_timer: f32,
    pub hitstun_timer: f32,
    pub slash_anim_timer: f32,
    pub attacking: bool,
}

impl PlayerCombatState {
    pub fn from_runtime(runtime: &crate::SandboxRuntime) -> Self {
        Self {
            flash_timer: runtime.flash_timer,
            hitstop_timer: runtime.hitstop_timer,
            damage_invuln_timer: runtime.damage_invuln_timer,
            hitstun_timer: runtime.hitstun_timer,
            slash_anim_timer: runtime.slash_anim_timer,
            attacking: runtime.player_attack.is_some(),
        }
    }

    pub fn apply_to_runtime(&self, runtime: &mut crate::SandboxRuntime) {
        runtime.flash_timer = self.flash_timer;
        runtime.hitstop_timer = self.hitstop_timer;
        runtime.damage_invuln_timer = self.damage_invuln_timer;
        runtime.hitstun_timer = self.hitstun_timer;
        runtime.slash_anim_timer = self.slash_anim_timer;
    }

    pub fn vulnerable(&self) -> bool {
        self.damage_invuln_timer <= 0.0
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

/// Mirror authoritative runtime player state onto the ECS player entity.
pub fn sync_player_entity_from_runtime(
    runtime: Res<crate::SandboxRuntime>,
    mut players: Query<
        (
            &mut PlayerBody,
            &mut PlayerHealth,
            &mut PlayerCombatState,
            &mut PlayerInteractionState,
        ),
        With<PlayerEntity>,
    >,
) {
    let Ok((mut body, mut health, mut combat, mut interaction)) = players.single_mut() else {
        return;
    };
    *body = PlayerBody::from_player(&runtime.player);
    *health = PlayerHealth::new(runtime.player_health);
    *combat = PlayerCombatState::from_runtime(&runtime);
    *interaction = PlayerInteractionState::from_runtime(&runtime);
}

/// Apply heal messages to the current runtime authority and immediately mirror
/// the resulting health onto the ECS player entity.
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
