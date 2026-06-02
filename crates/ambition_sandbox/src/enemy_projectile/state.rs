//! Live state for enemy-fired projectiles.

use crate::engine_core as ae;
use bevy::prelude::Resource;

/// A spawn request describing an enemy projectile. Built by the
/// EFFECTS-stage consumer `spawn_enemy_projectiles_from_brain_actions`
/// from an [`crate::brain::ActorActionMessage::Ranged`] and flushed
/// into [`EnemyProjectileState::bodies`] by the same system. Boss
/// projectiles still go through `BossRuntime::update`'s
/// `outputs.projectile_spawns` field until the boss migration
/// lands.
#[derive(Clone, Debug)]
pub struct EnemyProjectileSpawn {
    pub origin: ae::Vec2,
    pub dir: ae::Vec2,
    pub speed: f32,
    pub damage: i32,
    pub max_lifetime: f32,
    pub half_extent: ae::Vec2,
    /// Id of the spawning enemy. Useful for self-friendly-fire ignore
    /// lists, sprite routing in the visuals layer (GNU-ton's apples
    /// stamp `gnu_ton_apple:*` so the visual gets the apple shape
    /// instead of the default bullet rectangle), and debug traces.
    pub owner_id: String,
    /// Per-second downward acceleration applied to the body each tick.
    /// Zero for hitscan-like volleys; positive for arcing/falling
    /// projectiles such as GNU-ton's apple rain.
    pub gravity: f32,
}

/// Bevy resource: every in-flight enemy projectile. Shares the unified
/// [`crate::projectile::InFlightProjectile`] in-flight representation with
/// the per-player projectile state.
#[derive(Resource, Default)]
pub struct EnemyProjectileState {
    pub bodies: Vec<crate::projectile::InFlightProjectile>,
}

impl EnemyProjectileState {
    /// Convert a spawn request into a live projectile body and push
    /// it onto the in-flight list.
    pub fn spawn(&mut self, request: EnemyProjectileSpawn) {
        let speed = request.speed.max(1.0);
        let dir = if request.dir.length() < 1.0e-4 {
            ae::Vec2::new(1.0, 0.0)
        } else {
            request.dir / request.dir.length()
        };
        let spec = crate::projectile::ProjectileSpec {
            // We reuse the Fireball kind for sprite/lifetime tables;
            // damage/speed/lifetime are overridden below.
            kind: crate::projectile::ProjectileKind::Fireball,
            origin: request.origin,
            direction: dir,
            damage: request.damage.max(1),
            speed,
            max_lifetime: request.max_lifetime.max(0.2),
            half_extent: request.half_extent,
            gravity: request.gravity.max(0.0),
            charge_tier: 0,
        };
        let mut body = crate::projectile::ProjectileBody::from_spec_with_faction(
            spec,
            crate::projectile::ProjectileFaction::Enemy,
        );
        // Enemy projectiles travel in a straight line (no bouncing —
        // a bouncing volley reads as a pinball and confuses the
        // player about the hostile path).
        body.bounces_remaining = 0;
        self.bodies.push(crate::projectile::InFlightProjectile {
            body,
            owner_id: request.owner_id,
        });
    }

    /// Clear all in-flight bodies (room transition).
    pub fn clear(&mut self) {
        self.bodies.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_request(speed: f32, damage: i32) -> EnemyProjectileSpawn {
        EnemyProjectileSpawn {
            origin: ae::Vec2::ZERO,
            dir: ae::Vec2::new(1.0, 0.0),
            speed,
            damage,
            max_lifetime: 1.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            owner_id: "pirate_1".into(),
            gravity: 0.0,
        }
    }

    #[test]
    fn spawn_tags_body_with_enemy_faction() {
        let mut state = EnemyProjectileState::default();
        state.spawn(spawn_request(120.0, 1));
        assert_eq!(state.bodies.len(), 1);
        assert_eq!(
            state.bodies[0].body.faction,
            crate::projectile::ProjectileFaction::Enemy
        );
    }

    #[test]
    fn spawn_records_owner_id_for_self_filter() {
        let mut state = EnemyProjectileState::default();
        state.spawn(spawn_request(120.0, 1));
        assert_eq!(state.bodies[0].owner_id, "pirate_1");
    }

    #[test]
    fn spawn_zeroes_bounces_remaining_on_enemy_projectile() {
        let mut state = EnemyProjectileState::default();
        state.spawn(spawn_request(120.0, 1));
        // Enemy projectiles travel in a straight line; the per-frame
        // update treats one-way platforms as solid and expires on
        // first contact. `from_spec` would normally give Fireball
        // two bounces, but `EnemyProjectileState::spawn` zeroes the
        // counter so the engine sees the no-bounce policy.
        assert_eq!(state.bodies[0].body.bounces_remaining, 0);
    }

    #[test]
    fn spawn_clamps_zero_direction_to_right_facing() {
        let mut state = EnemyProjectileState::default();
        state.spawn(EnemyProjectileSpawn {
            origin: ae::Vec2::ZERO,
            dir: ae::Vec2::ZERO,
            speed: 120.0,
            damage: 1,
            max_lifetime: 1.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            owner_id: "test".into(),
            gravity: 0.0,
        });
        // A zero-length direction would NaN the initial_velocity; spawn
        // defaults to (1, 0) so the projectile has a sensible direction.
        let vel = state.bodies[0].body.vel;
        assert!(vel.x > 0.0 && vel.y == 0.0, "got {vel:?}");
    }

    #[test]
    fn spawn_clamps_zero_speed_and_damage_to_minimums() {
        let mut state = EnemyProjectileState::default();
        state.spawn(spawn_request(0.0, 0));
        let body = &state.bodies[0].body;
        assert!(body.vel.length() >= 1.0, "speed clamped to >= 1.0");
        assert!(body.damage >= 1, "damage clamped to >= 1");
    }

    #[test]
    fn clear_drops_all_in_flight_bodies() {
        let mut state = EnemyProjectileState::default();
        state.spawn(spawn_request(120.0, 1));
        state.spawn(spawn_request(120.0, 1));
        assert_eq!(state.bodies.len(), 2);
        state.clear();
        assert!(state.bodies.is_empty());
    }
}
