//! Generic per-shot projectile spec — fully self-describing, content-free.
//!
//! The foundation carries no named projectile vocabulary. A game's content (for
//! Ambition: `ambition_projectiles::kind`) authors named kinds and
//! lowers them into this generic [`ProjectileSpec`]; the primitive body steps it
//! purely from the data fields here (no `match kind` anywhere in the engine).

use ambition_platformer2d_core::Vec2;

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
    /// How this shot interacts with world geometry (bounce/passthrough vs
    /// expire-on-any-contact). A property of the ability, not the firer — so a
    /// shot behaves the same whoever fires it ([`super::WorldHitPolicy`]).
    pub world_hit: super::WorldHitPolicy,
    /// Opaque charge tier (0 = light, higher = more charged). Carried for the
    /// trace + visual layer; the engine does not interpret it. A game's charge
    /// mechanic stamps it when it scales `damage` / `half_extent`.
    pub charge_tier: u8,
    /// Seconds until this shot has stopped and starts coming back, or `None`
    /// for a shot that never turns around — which is every shot but the
    /// ponytail. See [`super::ProjectileGameplay::accel`], which this resolves
    /// into at spawn.
    ///
    /// ⛔ IT IS THE TIME TO THE TURNAROUND, NOT THE TIME TO GET HOME. The return
    /// leg takes the same time as the outbound one, so a shot authored at `0.4`
    /// passes back through the launch point at `0.8` and wants a lifetime a
    /// little past that.
    pub boomerang_return_s: Option<f32>,
}

impl ProjectileSpec {
    pub fn initial_velocity(&self) -> Vec2 {
        self.direction * self.speed
    }
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
            world_hit: crate::projectile::WorldHitPolicy::Bouncing,
            charge_tier: 0,
        };
        let v = s.initial_velocity();
        assert!((v.x - 360.0).abs() < 1e-3);
        assert!(v.y.abs() < 1e-3);
    }
}
