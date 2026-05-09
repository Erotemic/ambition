//! Cooldown + resource-meter gating for spawning new projectiles.

use bevy_math::Vec2;

use super::spec::{ProjectileKind, ProjectileSpec};
use crate::player_state::ResourceMeter;

/// Spawner state. Owns the per-projectile cooldown timer and a
/// [`ResourceMeter`] that mechanics can refill from rooms / pickups.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileSpawner {
    pub meter: ResourceMeter,
    pub cooldown_remaining: f32,
}

impl ProjectileSpawner {
    pub fn new(max_resource: f32, regen_rate: f32) -> Self {
        Self {
            meter: ResourceMeter::new(max_resource, regen_rate, 0.0),
            cooldown_remaining: 0.0,
        }
    }

    /// Tick down the cooldown timer and regen the resource meter.
    pub fn tick(&mut self, dt: f32) {
        self.cooldown_remaining = (self.cooldown_remaining - dt).max(0.0);
        self.meter.tick_regen(dt);
    }

    /// Try to fire a projectile of the given kind from `origin`
    /// pointing in `direction`. Returns the [`ProjectileSpec`] to
    /// spawn on success. Failure modes:
    ///
    /// - `cooldown_remaining > 0.0` → `Err(SpawnFailure::Cooldown)`
    /// - resource meter doesn't have enough for `kind.cost()` →
    ///   `Err(SpawnFailure::OutOfResource)`
    pub fn try_spawn(
        &mut self,
        kind: ProjectileKind,
        origin: Vec2,
        direction: Vec2,
        outgoing_damage_multiplier: f32,
    ) -> Result<ProjectileSpec, SpawnFailure> {
        if self.cooldown_remaining > 0.0 {
            return Err(SpawnFailure::Cooldown);
        }
        if !self.meter.try_spend(kind.cost()) {
            return Err(SpawnFailure::OutOfResource);
        }
        self.cooldown_remaining = kind.cooldown();
        Ok(ProjectileSpec::new(
            kind,
            origin,
            direction,
            outgoing_damage_multiplier,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnFailure {
    Cooldown,
    OutOfResource,
}
