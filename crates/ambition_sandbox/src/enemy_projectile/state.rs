//! Live state for enemy-fired projectiles.

use ambition_engine as ae;
use bevy::prelude::Resource;

/// A spawn request emitted by an enemy choreography this frame.
/// Collected by `EnemyRuntime::update` into `EnemyTickOutputs` and
/// flushed into [`EnemyProjectileState::bodies`] by the system.
#[derive(Clone, Debug)]
pub struct EnemyProjectileSpawn {
    pub origin: ae::Vec2,
    pub dir: ae::Vec2,
    pub speed: f32,
    pub damage: i32,
    pub max_lifetime: f32,
    pub half_extent: ae::Vec2,
    /// Id of the spawning enemy. Useful for self-friendly-fire ignore
    /// lists and debug traces.
    pub owner_id: String,
}

/// Wrapper around an in-flight `ae::ProjectileBody` plus enemy
/// faction metadata.
#[derive(Clone, Debug)]
pub struct EnemyProjectile {
    pub body: ae::ProjectileBody,
    pub owner_id: String,
}

/// Bevy resource: every in-flight enemy projectile.
#[derive(Resource, Default)]
pub struct EnemyProjectileState {
    pub bodies: Vec<EnemyProjectile>,
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
        let spec = ae::ProjectileSpec {
            // We reuse the Fireball kind for sprite/lifetime tables;
            // damage/speed/lifetime are overridden below.
            kind: ae::ProjectileKind::Fireball,
            origin: request.origin,
            direction: dir,
            damage: request.damage.max(1),
            speed,
            max_lifetime: request.max_lifetime.max(0.2),
            half_extent: request.half_extent,
            gravity: 0.0,
            charge_tier: 0,
        };
        let mut body = ae::ProjectileBody::from_spec(spec);
        // Enemy projectiles travel in a straight line (no bouncing —
        // a bouncing volley reads as a pinball and confuses the
        // player about the hostile path).
        body.bounces_remaining = 0;
        self.bodies.push(EnemyProjectile {
            body,
            owner_id: request.owner_id,
        });
    }

    /// Clear all in-flight bodies (room transition).
    pub fn clear(&mut self) {
        self.bodies.clear();
    }
}
