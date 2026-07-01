//! Spawn-request → body mapping for enemy-fired projectiles.
//!
//! The in-flight bodies are ECS entities (`crate::enemy_projectile::entity`),
//! mirroring the player pool. `EnemyProjectileState` is a field-less resource:
//! it owns no in-flight storage, but it keeps the canonical request→body builder
//! (`build`) that both the `SpawnProjectile` message path and the direct test
//! spawns share, and it remains a stable type for the
//! `Res<EnemyProjectileState>` references + room-reset hooks across the codebase
//! (matching how the player pool keeps `PlayerProjectileState` for its
//! controller state).

use ambition_engine_core as ae;
use bevy::prelude::Resource;

// `EnemyProjectileSpawn` (a substrate-neutral projectile spawn request) moved
// down to `ambition_platformer_primitives::projectile` so the foundation
// `ambition_vfx` vocabulary can reference it. Re-exported here at its
// historical path.
pub use ambition_platformer_primitives::projectile::EnemyProjectileSpawn;

/// Bevy resource for the enemy-projectile pool. The in-flight bodies are ECS
/// entities; this type owns no `Vec` — it is a stable resource handle + the home
/// of the [`Self::build`] request→body mapping.
#[derive(Resource, Default)]
pub struct EnemyProjectileState;

impl EnemyProjectileState {
    /// Build (but do not store) the in-flight projectile for `request`.
    /// The single place the spawn-request → body mapping lives;
    /// the fire paths emit it inside a
    /// [`crate::projectile::SpawnProjectile`] message that
    /// `apply_projectile_effects` later spawns as an entity, and
    /// tests build it directly. The mapping is unchanged from the pre-entity
    /// pool.
    pub fn build(request: EnemyProjectileSpawn) -> crate::projectile::InFlightProjectile {
        let speed = request.speed.max(1.0);
        let dir = if request.dir.length() < 1.0e-4 {
            ae::Vec2::new(1.0, 0.0)
        } else {
            request.dir / request.dir.length()
        };
        let spec = crate::projectile::ProjectileSpec {
            origin: request.origin,
            direction: dir,
            damage: request.damage.max(1),
            speed,
            max_lifetime: request.max_lifetime.max(0.2),
            half_extent: request.half_extent,
            gravity: request.gravity.max(0.0),
            // Pool projectiles travel in a straight line and die on contact (a
            // bouncing volley reads as a pinball and confuses the reader about
            // the path). Authored on the spec, firer-agnostic — a per-ability
            // bouncing pool shot is now expressible by setting these differently.
            bounces: 0,
            world_hit: crate::projectile::WorldHitPolicy::ExpireOnContact,
            charge_tier: 0,
        };
        let body = crate::projectile::ProjectileBody::from_spec(spec);
        crate::projectile::InFlightProjectile {
            body,
            owner_id: request.owner_id,
        }
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
            visual_tag: 0,
        }
    }

    #[test]
    fn build_records_owner_id_for_self_filter() {
        let proj = EnemyProjectileState::build(spawn_request(120.0, 1));
        assert_eq!(proj.owner_id, "pirate_1");
    }

    #[test]
    fn build_zeroes_bounces_remaining_on_enemy_projectile() {
        // Enemy projectiles travel in a straight line; the per-frame
        // update treats one-way platforms as solid and expires on
        // first contact. `from_spec` would normally give Fireball
        // two bounces, but `build` zeroes the counter so the engine
        // sees the no-bounce policy.
        let proj = EnemyProjectileState::build(spawn_request(120.0, 1));
        assert_eq!(proj.body.game.bounces_remaining, 0);
    }

    #[test]
    fn build_clamps_zero_direction_to_right_facing() {
        let proj = EnemyProjectileState::build(EnemyProjectileSpawn {
            origin: ae::Vec2::ZERO,
            dir: ae::Vec2::ZERO,
            speed: 120.0,
            damage: 1,
            max_lifetime: 1.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            owner_id: "test".into(),
            gravity: 0.0,
            visual_tag: 0,
        });
        // A zero-length direction would NaN the initial_velocity; build
        // defaults to (1, 0) so the projectile has a sensible direction.
        let vel = proj.body.kin.vel;
        assert!(vel.x > 0.0 && vel.y == 0.0, "got {vel:?}");
    }

    #[test]
    fn build_clamps_zero_speed_and_damage_to_minimums() {
        let proj = EnemyProjectileState::build(spawn_request(0.0, 0));
        let body = &proj.body;
        assert!(body.kin.vel.length() >= 1.0, "speed clamped to >= 1.0");
        assert!(body.game.damage >= 1, "damage clamped to >= 1");
    }
}
