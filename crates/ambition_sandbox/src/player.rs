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

/// ECS-visible player health read model.
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
    mut players: Query<(&mut PlayerBody, &mut PlayerHealth), With<PlayerEntity>>,
) {
    let Ok((mut body, mut health)) = players.single_mut() else {
        return;
    };
    *body = PlayerBody::from_player(&runtime.player);
    *health = PlayerHealth::new(runtime.player_health);
}

/// Apply heal messages to the current runtime authority and immediately mirror
/// the resulting health onto the ECS player entity.
pub fn apply_player_heal_requests(
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut heals: MessageReader<PlayerHealRequested>,
    mut players: Query<&mut PlayerHealth, With<PlayerEntity>>,
) {
    let mut changed = false;
    for heal in heals.read() {
        if heal.amount > 0 {
            runtime.player_health.heal(heal.amount);
            changed = true;
        }
    }
    if changed {
        if let Ok(mut health) = players.single_mut() {
            *health = PlayerHealth::new(runtime.player_health);
        }
    }
}
