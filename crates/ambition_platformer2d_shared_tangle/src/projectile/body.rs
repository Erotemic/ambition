//! Projectile state uses shared body motion, carried acceleration and gameplay data.
//!
//! Projectiles use the same body component as players/enemies/bosses, which
//! lets generic systems such as portal transit operate on projectile entities.
//! [`ProjectileBody`] composes the split halves for call sites that still want
//! a single value.

use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::Vec2;

use super::spec::ProjectileSpec;
use crate::frame_env::CarriedAcceleration;
use ambition_platformer2d_core::Aabb;

/// Ownership is deliberately absent here. Before ECS materialization the body is
/// just projectile state; `ProjectileSpawnRequest.owner` is the authoritative
/// firing occurrence, and materialization stamps that as `ProjectileOwner`.
#[derive(Clone, Debug)]
pub struct InFlightProjectile {
    pub body: ProjectileBody,
}

/// Projectile gameplay state: lifetime, gravity, damage, and bounce budget.
///
/// This component is also the projectile marker. Actor-generic systems that
/// query [`BodyKinematics`] exclude `ProjectileGameplay` so projectile bodies
/// are never swept into actor behavior such as auto-righting or AI.
#[derive(Clone, Copy, Debug, PartialEq, bevy::prelude::Component)]
#[require(CarriedAcceleration)]
pub struct ProjectileGameplay {
    pub age: f32,
    pub max_lifetime: f32,
    pub gravity: f32,
    pub damage: i32,
    pub bounces_remaining: u8,
    /// How this shot resolves against world geometry — authored on the spec
    /// (a property of the ability, firer-agnostic), NOT derived from `faction`.
    pub world_hit: super::WorldHitPolicy,
    /// WHICH LEG THIS SHOT'S VICTIM LEDGER WAS LAST CLEARED FOR.
    ///
    /// ⭐⭐ A RETURNING SHOT IS TWO CHANCES, AND THE GENRE SAYS SO. A boomerang
    /// hits you going out and hits you coming back; what it must not do is hit
    /// you on every tick it overlaps you. That is the same rule a multi-hit
    /// move's PULSE already states — one continuous stretch owns one per-victim
    /// ledger, and a gap starts a new one — and the leg is this shot's pulse.
    ///
    /// ⛔ THE LEG ITSELF IS DERIVED, NOT STORED. [`Self::leg`] reads it off the
    /// motion the shot is already carrying, so it cannot disagree with the
    /// trajectory. What has to be REMEMBERED is only which leg the ledger in
    /// [`ProjectileHits`] belongs to, because "the leg changed since I last
    /// cleared" is not a question this tick's velocity can answer.
    ///
    /// `0` for every shot that never turns around, which is all of them but the
    /// ponytail — those carry no `accel`, so [`Self::leg`] is always `0` and the
    /// ledger is never cleared. They also despawn on their first body hit, so
    /// they never write to it either.
    pub hits_cleared_on_leg: u8,
    /// Half-extent of the burst dealt where this shot lands; `0.0` for none.
    /// Rollback state: a rewound shot must land the same burst.
    pub splash_half_extent: f32,
    /// How the shot leaves a floor it bounces on ([`Rebound`]).
    pub rebound: Rebound,
}

/// How a bouncing shot leaves the floor: it keeps `restitution` of the speed it
/// landed with, and leaves at `min_speed` at the least. With no restitution
/// each hop leaves at `min_speed` and so rises to one height, whatever height
/// it fell from: the hop of a Mario fireball ([`Rebound::hop`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rebound {
    pub restitution: f32,
    pub min_speed: f32,
}

impl Rebound {
    /// The bounce of a shot that states no rebound: it keeps part of its
    /// fall. ⛔ The floor is a HOP even so: a shot that came down slowly
    /// (born low, at a hand by the knee) rebounded at 0.65 of almost nothing,
    /// landed again the next tick, and spent its whole bounce budget skidding
    /// (2026-10-21, Jon: "the robot's projectile doesn't seem to do damage
    /// anymore"), so it leaves at 240 px/s at the least.
    pub const SPRINGY: Self = Self { restitution: 0.65, min_speed: 240.0 };

    /// Each hop leaves the floor at `speed`: hops of one height.
    pub const fn hop(speed: f32) -> Self {
        Self { restitution: 0.0, min_speed: speed }
    }
}

impl Default for Rebound {
    fn default() -> Self {
        Self::SPRINGY
    }
}

/// EVERY VICTIM THIS SHOT HAS ALREADY HIT ON THE LEG IT IS FLYING.
///
/// ⭐ MODELLED ON `HitboxHits`, and registered through the same registrar path
/// (`rollback_component_clone_entity_set`) for the same reason: an entity set
/// cannot be snapshot by value across a rollback, because bevy_ggrs destroys and
/// recreates the entities it names. The probe reads it through the targets'
/// stable sim identities.
///
/// ⛔ AUTHORITATIVE ROLLBACK STATE, not a cache. A resimulated frame that lost
/// this ledger re-hits everybody the shot had already passed through, which is
/// the machine-gun the leg rule exists to prevent — one desync per overlap.
#[derive(Clone, Debug, Default, PartialEq, bevy::prelude::Component)]
pub struct ProjectileHits {
    pub hit: std::collections::BTreeSet<bevy::prelude::Entity>,
}

impl bevy::ecs::entity::MapEntities for ProjectileHits {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.hit = std::mem::take(&mut self.hit)
            .into_iter()
            .map(|entity| mapper.get_mapped(entity))
            .collect();
    }
}

/// The field's down direction. A zero field uses the authored world down.
pub(crate) fn projectile_down(gravity_dir: Vec2) -> Vec2 {
    let down=gravity_dir.as_dvec2().normalize_or_zero().as_vec2();
    if down == Vec2::ZERO { Vec2::Y } else { down }
}

impl ProjectileGameplay {
    /// Build the gameplay half from `spec`.
    fn from_spec(spec: ProjectileSpec) -> Self {
        Self {
            age: 0.0,
            max_lifetime: spec.max_lifetime,
            gravity: spec.gravity,
            damage: spec.damage,
            // Bounce budget is authored on the spec (e.g. Ambition's fireball
            // bounces twice; straight shots set 0). The engine never names kinds.
            bounces_remaining: spec.bounces,
            // World-hit policy is the spec's (the ability's), firer-agnostic.
            world_hit: spec.world_hit,
            hits_cleared_on_leg: 0,
            splash_half_extent: spec.splash_half_extent,
            rebound: Rebound::default(),
        }
    }

    /// A returning shot has one victim ledger for each flight leg.
    pub fn leg(&self, vel: Vec2, acceleration: CarriedAcceleration) -> u8 {
        u8::from(vel.as_dvec2().dot(acceleration.0.as_dvec2()) > 0.0)
    }

    pub fn returns(&self, acceleration: CarriedAcceleration) -> bool {
        acceleration.0 != Vec2::ZERO
    }

    pub fn is_expired(&self) -> bool {
        self.age >= self.max_lifetime
    }

    /// Advance lifetime + integrate the kinematic `body` forward by
    /// `dt`. Returns `true` if the projectile is still alive after the
    /// tick. Collision against solids / breakables is the caller's
    /// responsibility (sandbox).
    ///
    /// `gravity_dir` is the projectile's local acceleration-frame down. Gravity
    /// bearing projectiles (fireballs, bombs, apple-rain) accelerate toward that
    /// direction instead of assuming world +Y. Normal gravity remains byte-
    /// identical because `gravity_dir == (0, 1)`.
    pub fn tick(&mut self, body: &mut BodyKinematics, acceleration: CarriedAcceleration, dt: f32, gravity_dir: Vec2) -> bool {
        self.age += dt;
        if self.age >= self.max_lifetime {
            return false;
        }
        let down = projectile_down(gravity_dir);
        body.vel += (down * self.gravity + acceleration.0) * dt;
        body.pos += body.vel * dt;
        true
    }


}

/// Per-frame physics state of an in-flight projectile: kinematics plus
/// projectile gameplay state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileBody {
    /// Generic kinematic body (pos / vel / size / facing). Shared shape
    /// with the player / enemy / boss; `facing` is unused for
    /// projectiles (kept `1.0`). `size = half_extent * 2`.
    pub kin: BodyKinematics,
    /// Projectile gameplay state (lifetime / gravity / damage / bounce budget).
    pub game: ProjectileGameplay,
    pub acceleration: CarriedAcceleration,
}

impl ProjectileBody {
    /// Build a projectile body from `spec`. Damage routing is a function
    /// of the firer's real `ActorFaction` (looked up from the projectile's
    /// owner entity), not anything stored on the body.
    pub fn from_spec(spec: ProjectileSpec) -> Self {
        Self {
            kin: BodyKinematics { material: Default::default(),
                pos: spec.origin,
                vel: spec.initial_velocity(),
                // Full size = 2 * half-extent so `kin.aabb()` ==
                // the historical `Aabb::new(pos, half_extent)`.
                canonical_size: spec.half_extent * 2.0,
                // Projectiles don't use facing for orientation; keep it
                // at the player-flavored default so the field is sane.
                facing: 1.0,
            },
            game: ProjectileGameplay::from_spec(spec),
            acceleration: CarriedAcceleration(spec.boomerang_return_s.map_or(Vec2::ZERO, |out_s| {
                if out_s > 0.0 { -spec.initial_velocity() / out_s } else { Vec2::ZERO }
            })),
        }
    }

    /// Build one projectile value from its ECS components.
    pub fn from_parts(kin: BodyKinematics, game: ProjectileGameplay, acceleration: CarriedAcceleration) -> Self {
        Self { kin, game, acceleration }
    }

    pub fn aabb(&self) -> Aabb {
        self.kin.aabb()
    }

    /// Step the projectile forward by `dt`. Returns `true` if the
    /// projectile is still alive after the tick. Delegates to
    /// [`ProjectileGameplay::tick`] with its carried acceleration.
    pub fn tick(&mut self, dt: f32, gravity_dir: Vec2) -> bool {
        self.game.tick(&mut self.kin, self.acceleration, dt, gravity_dir)
    }

    pub fn is_expired(&self) -> bool {
        self.game.is_expired()
    }

    // Field-style accessors forward to the appropriate half.

    pub fn pos(&self) -> Vec2 {
        self.kin.pos
    }
    pub fn vel(&self) -> Vec2 {
        self.kin.vel
    }
    /// Hitbox half-extent (= `kin.size * 0.5`).
    pub fn half_extent(&self) -> Vec2 {
        self.kin.size() * 0.5
    }
    pub fn world_hit(&self) -> super::WorldHitPolicy {
        self.game.world_hit
    }
    pub fn damage(&self) -> i32 {
        self.game.damage
    }
    pub fn bounces_remaining(&self) -> u8 {
        self.game.bounces_remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fireball(pos: Vec2, vel: Vec2, bounces: u8) -> ProjectileBody {
        ProjectileBody {
            kin: BodyKinematics { material: Default::default(),
                pos,
                vel,
                canonical_size: Vec2::new(8.0, 8.0),
                facing: 1.0,
            },
            acceleration: CarriedAcceleration::default(),
            game: ProjectileGameplay {
                age: 0.0,
                max_lifetime: 1.0,
                gravity: 0.0,
                damage: 2,
                bounces_remaining: bounces,
                world_hit: crate::projectile::WorldHitPolicy::Bouncing,
                hits_cleared_on_leg: 0,
                splash_half_extent: 0.0,
                rebound: Rebound::default(),
            },
        }
    }

    #[test]
    fn tick_advances_position_and_applies_gravity() {
        let mut p = fireball(Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0), 0);
        p.game.gravity = 200.0;
        let alive = p.tick(0.1, Vec2::new(0.0, 1.0));
        assert!(alive, "still alive well within lifetime");
        // vy gains gravity*dt first, then pos integrates the new velocity.
        assert!((p.kin.vel.y - 20.0).abs() < 1e-3);
        assert!((p.kin.pos.x - 10.0).abs() < 1e-3 && (p.kin.pos.y - 2.0).abs() < 1e-3);
    }

    #[test]
    fn tick_returns_false_and_holds_position_when_expired() {
        let mut p = fireball(Vec2::new(5.0, 5.0), Vec2::new(100.0, 0.0), 0);
        p.game.max_lifetime = 0.1;
        let alive = p.tick(0.2, Vec2::new(0.0, 1.0));
        assert!(!alive, "a tick past the lifetime reports dead");
        assert!(p.is_expired());
        assert_eq!(
            p.kin.pos,
            Vec2::new(5.0, 5.0),
            "expiring tick does not move the body"
        );
    }

    #[test]
    fn tick_accelerates_along_local_down_for_all_cardinal_frames() {
        let gravity_dirs = [
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(-1.0, 0.0),
        ];
        let center = Vec2::new(200.0, 200.0);
        for gravity_dir in gravity_dirs {
            let frame = ambition_platformer2d_core::AccelerationFrame::new(gravity_dir);
            let mut p = fireball(center, frame.to_world(Vec2::new(10.0, 5.0)), 0);
            p.game.gravity = 200.0;
            assert!(p.tick(0.1, gravity_dir));
            let local_vel = Vec2::new(p.kin.vel.dot(frame.side), p.kin.vel.dot(frame.down));
            let local_delta = Vec2::new(
                (p.kin.pos - center).dot(frame.side),
                (p.kin.pos - center).dot(frame.down),
            );
            assert!(
                (local_vel.x - 10.0).abs() < 1e-3 && (local_vel.y - 25.0).abs() < 1e-3,
                "projectile velocity should be frame-equivalent for gravity {gravity_dir:?}: {local_vel:?}"
            );
            assert!(
                (local_delta.x - 1.0).abs() < 1e-3 && (local_delta.y - 2.5).abs() < 1e-3,
                "projectile motion should be frame-equivalent for gravity {gravity_dir:?}: {local_delta:?}"
            );
        }
    }

    #[test]
    fn aabb_is_centered_on_position() {
        let p = fireball(Vec2::new(50.0, 60.0), Vec2::ZERO, 0);
        let bb = p.aabb();
        assert_eq!(bb.min, Vec2::new(46.0, 56.0));
        assert_eq!(bb.max, Vec2::new(54.0, 64.0));
    }




}
