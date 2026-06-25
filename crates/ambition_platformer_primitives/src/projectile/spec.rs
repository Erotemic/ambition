//! Generic per-shot projectile spec — fully self-describing, content-free.
//!
//! The foundation carries no named projectile vocabulary. A game's content (for
//! Ambition: `ambition_gameplay_core::projectile::kind`) authors named kinds and
//! lowers them into this generic [`ProjectileSpec`]; the primitive body steps it
//! purely from the data fields here (no `match kind` anywhere in the engine).

use ambition_engine_core::Vec2;

/// Authored intent for a single new projectile. The spawner builds an entity
/// carrying this spec plus its current pos / vel; `ProjectileBody` is the
/// per-frame state it advances. Every field is generic data — the engine never
/// branches on a named kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileSpec {
    /// Initial center position.
    pub origin: Vec2,
    /// Unit-length direction vector. (1, 0) fires right.
    pub direction: Vec2,
    /// Damage to apply on hit.
    pub damage: i32,
    /// Initial speed in px/s.
    pub speed: f32,
    /// Maximum lifetime.
    pub max_lifetime: f32,
    /// Half-extent of the hitbox.
    pub half_extent: Vec2,
    /// Vertical acceleration applied each frame (px/s^2). Mario-like /
    /// arcade-style arc: positive value pulls down (recall +Y is down
    /// in the sandbox simulation).
    pub gravity: f32,
    /// How many times the projectile bounces off support faces before it
    /// expires on a solid hit. 0 = expire on first solid contact.
    pub bounces: u8,
    /// Opaque charge tier (0 = light, higher = more charged). Carried for the
    /// trace + visual layer; the engine does not interpret it. A game's charge
    /// mechanic stamps it when it scales `damage` / `half_extent`.
    pub charge_tier: u8,
}

impl ProjectileSpec {
    pub fn initial_velocity(&self) -> Vec2 {
        self.direction * self.speed
    }
}

/// A request to spawn one in-flight projectile (origin / dir / speed / damage /
/// lifetime / size / owner-id / gravity). Substrate-neutral data: the effect
/// vocabulary and both projectile pools build bodies from it. (Named for its
/// historical enemy-pool origin; it is pool-agnostic.)
#[derive(Clone, Debug)]
pub struct EnemyProjectileSpawn {
    pub origin: Vec2,
    pub dir: Vec2,
    pub speed: f32,
    pub damage: i32,
    pub max_lifetime: f32,
    /// Id of the spawning actor — self-friendly-fire ignore lists, sprite
    /// routing in the visuals layer, debug traces.
    pub owner_id: String,
    pub half_extent: Vec2,
    /// Per-second downward acceleration each tick. Zero for hitscan-like
    /// volleys; positive for arcing/falling projectiles (e.g. apple rain).
    pub gravity: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_velocity_scales_direction_by_speed() {
        let s = ProjectileSpec {
            origin: Vec2::ZERO,
            direction: Vec2::new(1.0, 0.0),
            damage: 1,
            speed: 360.0,
            max_lifetime: 1.2,
            half_extent: Vec2::new(12.0, 9.0),
            gravity: 360.0,
            bounces: 2,
            charge_tier: 0,
        };
        let v = s.initial_velocity();
        assert!((v.x - 360.0).abs() < 1e-3);
        assert!(v.y.abs() < 1e-3);
    }
}
