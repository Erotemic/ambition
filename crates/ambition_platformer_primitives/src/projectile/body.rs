//! Projectile state split into shared kinematics ([`BodyKinematics`]) and
//! projectile gameplay state ([`ProjectileGameplay`]).
//!
//! Projectiles use the same body component as players/enemies/bosses, which
//! lets generic systems such as portal transit operate on projectile entities.
//! [`ProjectileBody`] composes the split halves for call sites that still want
//! a single value.

use ambition_engine_core::BodyKinematics;
use ambition_engine_core::Vec2;

use super::spec::ProjectileSpec;
use ambition_engine_core::{Aabb, AabbExt};

/// Which side of the combat faction a projectile belongs to.
///
/// `Player` projectiles hit enemies / bosses / breakables; `Enemy`
/// projectiles hit the player. Damage routing is a function of
/// `(projectile.faction, target.faction)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProjectileFaction {
    /// Player-owned projectile (fireball, hadouken). Hits hostile
    /// actors; the player's own hurtbox is filtered out.
    #[default]
    Player,
    /// Enemy-owned projectile (pirate volley, future boss shots).
    /// Hits the player; does not damage other enemies.
    Enemy,
}

/// One live projectile: the kinematic [`BodyKinematics`] plus the
/// projectile [`ProjectileGameplay`] state plus optional owner
/// attribution. This is the single in-flight representation for both
/// the per-player projectile state and the global enemy-projectile
/// pool, so the body list, visuals, and collision read the same shape
/// regardless of who fired.
///
/// `owner_id` carries the spawning actor's id for self-friendly-fire
/// ignore lists and sprite routing (GNU-ton's apple rain, the lasersword
/// rider). It is empty for player projectiles, which are attributed via
/// `HitEvent::attacker` instead. The gameplay `faction` distinguishes
/// player vs enemy routing.
#[derive(Clone, Debug)]
pub struct InFlightProjectile {
    pub body: ProjectileBody,
    pub owner_id: String,
}

/// Projectile gameplay state: identity (kind/faction), lifetime, gravity,
/// damage, and bounce budget.
///
/// This component is also the projectile marker. Actor-generic systems that
/// query [`BodyKinematics`] exclude `ProjectileGameplay` so projectile bodies
/// are never swept into actor behavior such as auto-righting or AI.
#[derive(Clone, Copy, Debug, PartialEq, bevy::prelude::Component)]
pub struct ProjectileGameplay {
    /// Combat faction (who fired this projectile, which targets it
    /// may damage). Set at spawn time; the engine never mutates it.
    pub faction: ProjectileFaction,
    pub age: f32,
    pub max_lifetime: f32,
    pub gravity: f32,
    pub damage: i32,
    pub bounces_remaining: u8,
    /// How this shot resolves against world geometry — authored on the spec
    /// (a property of the ability, firer-agnostic), NOT derived from `faction`.
    pub world_hit: super::WorldHitPolicy,
}

fn projectile_down(gravity_dir: Vec2) -> Vec2 {
    if gravity_dir.x.abs() > gravity_dir.y.abs() {
        Vec2::new(gravity_dir.x.signum(), 0.0)
    } else if gravity_dir.y.abs() > 0.0 {
        Vec2::new(0.0, gravity_dir.y.signum())
    } else {
        Vec2::new(0.0, 1.0)
    }
}

fn perpendicular_overlap(body: Aabb, surface: Aabb, gravity_dir: Vec2) -> bool {
    if gravity_dir.x.abs() > gravity_dir.y.abs() {
        body.bottom() > surface.top() && body.top() < surface.bottom()
    } else {
        body.right() > surface.left() && body.left() < surface.right()
    }
}

impl ProjectileGameplay {
    /// Build the gameplay half from `spec` with an explicit faction.
    fn from_spec_with_faction(spec: ProjectileSpec, faction: ProjectileFaction) -> Self {
        Self {
            faction,
            age: 0.0,
            max_lifetime: spec.max_lifetime,
            gravity: spec.gravity,
            damage: spec.damage,
            // Bounce budget is authored on the spec (e.g. Ambition's fireball
            // bounces twice; straight shots set 0). The engine never names kinds.
            bounces_remaining: spec.bounces,
            // World-hit policy is the spec's (the ability's), firer-agnostic.
            world_hit: spec.world_hit,
        }
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
    pub fn tick(&mut self, body: &mut BodyKinematics, dt: f32, gravity_dir: Vec2) -> bool {
        self.age += dt;
        if self.age >= self.max_lifetime {
            return false;
        }
        let down = projectile_down(gravity_dir);
        body.vel += down * (self.gravity * dt);
        body.pos += body.vel * dt;
        true
    }

    /// Compatibility seam for legacy call sites/tests that only model vertical
    /// gravity. New code should call [`Self::tick`] with a full direction.
    pub fn tick_with_gravity_sign(
        &mut self,
        body: &mut BodyKinematics,
        dt: f32,
        gravity_sign: f32,
    ) -> bool {
        self.tick(body, dt, Vec2::new(0.0, gravity_sign.signum()))
    }

    /// True when the contact geometry qualifies as a support-face landing:
    /// projectile moving toward its feet, with overlap on the perpendicular axis,
    /// and with the body straddling the candidate surface's support face. The
    /// face-straddle check is important: a tall side wall also has a top/head
    /// face, but a projectile overlapping the wall halfway down should expire
    /// instead of treating that distant top face as a floor.
    fn is_support_landing(
        &self,
        body: &BodyKinematics,
        block_aabb: Aabb,
        gravity_dir: Vec2,
    ) -> bool {
        const CONTACT_SLOP: f32 = 1.0;
        let down = projectile_down(gravity_dir);
        let body_aabb = body.aabb();
        let moving_toward_feet = body.vel.dot(down) > 0.0;
        let support_face = block_aabb.head_coord(down);
        let body_head = body_aabb.head_coord(down);
        let body_feet = body_aabb.feet_coord(down);
        moving_toward_feet
            && body_head <= support_face + CONTACT_SLOP
            && body_feet >= support_face - CONTACT_SLOP
            && perpendicular_overlap(body_aabb, block_aabb, down)
    }

    /// Reposition the body so its feet rest just outside the support face,
    /// reflect velocity along local down with restitution, and decrement the
    /// bounce budget. Caller has already checked that `bounces_remaining > 0`.
    fn apply_support_bounce(
        &mut self,
        body: &mut BodyKinematics,
        block_aabb: Aabb,
        gravity_dir: Vec2,
    ) {
        // The 1px lift prevents an immediate re-hit on the next tick when gravity
        // has not yet reaccelerated toward the support.
        const RESTITUTION: f32 = 0.65;
        const SETTLE_LIFT: f32 = 1.0;
        let down = projectile_down(gravity_dir);
        let body_aabb = body.aabb();
        body.pos += down * (block_aabb.head_coord(down) - SETTLE_LIFT - body_aabb.feet_coord(down));
        let toward_feet = body.vel.dot(down);
        if toward_feet > 0.0 {
            let side = body.vel - down * toward_feet;
            body.vel = side - down * (toward_feet * RESTITUTION);
        }
        self.bounces_remaining -= 1;
    }

    /// Resolution outcome when this projectile overlaps a solid block.
    /// The caller decides what to do: bounce paths re-position the
    /// body and continue the next tick; expire paths despawn the
    /// projectile (and optionally trigger a hit VFX).
    ///
    /// `Bounced` is reserved for support-face contacts: the projectile is
    /// arriving from the anti-gravity side and moving toward its feet. Side
    /// and ceiling contacts always expire so the gameplay is predictable — a
    /// flying horizontal projectile does not suddenly retrace its path.
    pub fn resolve_solid_hit(
        &mut self,
        body: &mut BodyKinematics,
        block_aabb: Aabb,
    ) -> ProjectileSolidHit {
        self.resolve_solid_hit_in_frame(body, block_aabb, Vec2::new(0.0, 1.0))
    }

    pub fn resolve_solid_hit_in_frame(
        &mut self,
        body: &mut BodyKinematics,
        block_aabb: Aabb,
        gravity_dir: Vec2,
    ) -> ProjectileSolidHit {
        if self.is_support_landing(body, block_aabb, gravity_dir) && self.bounces_remaining > 0 {
            self.apply_support_bounce(body, block_aabb, gravity_dir);
            ProjectileSolidHit::Bounced
        } else {
            ProjectileSolidHit::Expired
        }
    }

    /// Resolution outcome when this projectile overlaps a one-way
    /// platform. A support-face landing bounces the same way a solid
    /// support would — fireballs skip across thin platforms identically to
    /// thick supports. Every other contact (sides, feet-side, support-side
    /// with no bounce budget) returns `Passthrough` so the projectile keeps
    /// flying — the platform is non-solid from those directions.
    pub fn resolve_one_way_hit(
        &mut self,
        body: &mut BodyKinematics,
        block_aabb: Aabb,
    ) -> ProjectileSolidHit {
        self.resolve_one_way_hit_in_frame(body, block_aabb, Vec2::new(0.0, 1.0))
    }

    pub fn resolve_one_way_hit_in_frame(
        &mut self,
        body: &mut BodyKinematics,
        block_aabb: Aabb,
        gravity_dir: Vec2,
    ) -> ProjectileSolidHit {
        if self.is_support_landing(body, block_aabb, gravity_dir) && self.bounces_remaining > 0 {
            self.apply_support_bounce(body, block_aabb, gravity_dir);
            ProjectileSolidHit::Bounced
        } else {
            ProjectileSolidHit::Passthrough
        }
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
    /// Projectile gameplay state (kind / faction / lifetime / gravity /
    /// damage / bounce budget).
    pub game: ProjectileGameplay,
}

impl ProjectileBody {
    /// Build a player-owned projectile body from `spec`.
    pub fn from_spec(spec: ProjectileSpec) -> Self {
        Self::from_spec_with_faction(spec, ProjectileFaction::Player)
    }

    /// Build a projectile body from `spec` with an explicit
    /// [`ProjectileFaction`]. Enemy-fired projectiles (pirate
    /// volleys, future boss shots) pass `ProjectileFaction::Enemy`
    /// so the unified projectile pipeline knows which target side
    /// to test against.
    pub fn from_spec_with_faction(spec: ProjectileSpec, faction: ProjectileFaction) -> Self {
        Self {
            kin: BodyKinematics {
                pos: spec.origin,
                vel: spec.initial_velocity(),
                // Full size = 2 * half-extent so `kin.aabb()` ==
                // the historical `Aabb::new(pos, half_extent)`.
                size: spec.half_extent * 2.0,
                // Projectiles don't use facing for orientation; keep it
                // at the player-flavored default so the field is sane.
                facing: 1.0,
            },
            game: ProjectileGameplay::from_spec_with_faction(spec, faction),
        }
    }

    /// Build a [`ProjectileBody`] from already-split halves.
    pub fn from_parts(kin: BodyKinematics, game: ProjectileGameplay) -> Self {
        Self { kin, game }
    }

    pub fn aabb(&self) -> Aabb {
        self.kin.aabb()
    }

    /// Step the projectile forward by `dt`. Returns `true` if the
    /// projectile is still alive after the tick. Delegates to
    /// [`ProjectileGameplay::tick`] on the split halves.
    pub fn tick(&mut self, dt: f32, gravity_dir: Vec2) -> bool {
        self.game.tick(&mut self.kin, dt, gravity_dir)
    }

    pub fn tick_with_gravity_sign(&mut self, dt: f32, gravity_sign: f32) -> bool {
        self.game
            .tick_with_gravity_sign(&mut self.kin, dt, gravity_sign)
    }

    pub fn is_expired(&self) -> bool {
        self.game.is_expired()
    }

    /// See [`ProjectileGameplay::resolve_solid_hit`].
    pub fn resolve_solid_hit(&mut self, block_aabb: Aabb) -> ProjectileSolidHit {
        self.game.resolve_solid_hit(&mut self.kin, block_aabb)
    }

    /// See [`ProjectileGameplay::resolve_one_way_hit`].
    pub fn resolve_one_way_hit(&mut self, block_aabb: Aabb) -> ProjectileSolidHit {
        self.game.resolve_one_way_hit(&mut self.kin, block_aabb)
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
        self.kin.size * 0.5
    }
    pub fn faction(&self) -> ProjectileFaction {
        self.game.faction
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

/// Outcome of [`ProjectileGameplay::resolve_solid_hit`] /
/// [`ProjectileGameplay::resolve_one_way_hit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileSolidHit {
    /// Projectile bounced off the support face; `bounces_remaining`
    /// decremented and local-down velocity reflected. Caller keeps the body alive.
    Bounced,
    /// Projectile should be removed (no bounces left on a solid hit,
    /// or contact was not a support-face landing on a solid).
    Expired,
    /// Projectile flies through the block unaffected. Only returned
    /// from one-way resolution: the body keeps its position and
    /// velocity and the caller treats the contact as if the block
    /// weren't there.
    Passthrough,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fireball(pos: Vec2, vel: Vec2, bounces: u8) -> ProjectileBody {
        ProjectileBody {
            kin: BodyKinematics {
                pos,
                vel,
                size: Vec2::new(8.0, 8.0),
                facing: 1.0,
            },
            game: ProjectileGameplay {
                faction: ProjectileFaction::Player,
                age: 0.0,
                max_lifetime: 1.0,
                gravity: 0.0,
                damage: 2,
                bounces_remaining: bounces,
                world_hit: crate::projectile::WorldHitPolicy::Bouncing,
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
            let frame = ambition_engine_core::AccelerationFrame::new(gravity_dir);
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

    #[test]
    fn fireball_bounces_off_a_floor_top_then_expires_when_budget_runs_out() {
        // Body just above a block, moving down, one bounce left.
        let mut p = fireball(Vec2::new(50.0, 50.0), Vec2::new(0.0, 100.0), 1);
        let block =
            ambition_engine_core::aabb_from_min_size(Vec2::new(40.0, 54.0), Vec2::new(20.0, 20.0));

        let first = p.resolve_solid_hit(block);
        assert_eq!(first, ProjectileSolidHit::Bounced);
        assert!(p.kin.vel.y < 0.0, "bounce reverses vertical velocity");
        assert_eq!(p.game.bounces_remaining, 0, "bounce spends the budget");

        // Re-aim downward into the same floor with no budget left → expires.
        p.kin.vel.y = 100.0;
        let second = p.resolve_solid_hit(block);
        assert_eq!(second, ProjectileSolidHit::Expired);
    }

    #[test]
    fn side_contact_expires_instead_of_bouncing() {
        // Body fully inside the block's y-range = a side hit, never a bounce.
        let mut p = fireball(Vec2::new(50.0, 60.0), Vec2::new(100.0, 0.0), 2);
        let block =
            ambition_engine_core::aabb_from_min_size(Vec2::new(40.0, 40.0), Vec2::new(20.0, 60.0));
        assert_eq!(p.resolve_solid_hit(block), ProjectileSolidHit::Expired);
        assert_eq!(
            p.game.bounces_remaining, 2,
            "a side hit does not spend a bounce"
        );
    }
}
