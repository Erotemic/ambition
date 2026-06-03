//! Per-frame projectile state plus the engine-side floor/wall
//! resolution (`ProjectileBody::resolve_solid_hit`).

use crate::engine_core::Vec2;

use super::spec::{ProjectileKind, ProjectileSpec};
use crate::engine_core::{aabb_from_min_size, Aabb, AabbExt};

/// Which side of the combat faction a projectile belongs to.
///
/// `Player` projectiles hit enemies / bosses / breakables; `Enemy`
/// projectiles hit the player. The sandbox-side update loops dispatch
/// damage routing on this so a future unified projectile system
/// (OVERNIGHT-TODO #17.7) does not need separate code paths per
/// faction — friendly-fire policy becomes a function of
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

/// One live projectile: the shared kinematic [`ProjectileBody`] plus
/// optional owner attribution. This is the single in-flight
/// representation for both the per-player projectile state and the
/// global enemy-projectile pool, so the body list, visuals, and
/// collision read the same shape regardless of who fired.
///
/// `owner_id` carries the spawning actor's id for self-friendly-fire
/// ignore lists and sprite routing (GNU-ton's apple rain, the lasersword
/// rider). It is empty for player projectiles, which are attributed via
/// `HitEvent::attacker` instead. The body's `faction` distinguishes
/// player vs enemy routing.
#[derive(Clone, Debug)]
pub struct InFlightProjectile {
    pub body: ProjectileBody,
    pub owner_id: String,
}

/// Per-frame physics state of an in-flight projectile. Sandbox owns
/// the world-collision check; this struct only owns position, velocity,
/// faction, and lifetime.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileBody {
    pub kind: ProjectileKind,
    /// Combat faction (who fired this projectile, which targets it
    /// may damage). Set at spawn time; the engine never mutates it.
    pub faction: ProjectileFaction,
    pub pos: Vec2,
    pub vel: Vec2,
    pub age: f32,
    pub max_lifetime: f32,
    pub gravity: f32,
    pub half_extent: Vec2,
    pub damage: i32,
    pub bounces_remaining: u8,
}

impl ProjectileBody {
    /// Build a player-owned projectile body from `spec`. Convenience
    /// wrapper around [`Self::from_spec_with_faction`] preserved for
    /// callers that haven't migrated to the explicit-faction
    /// constructor yet (player projectiles are the historical
    /// default in the engine API).
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
            kind: spec.kind,
            faction,
            pos: spec.origin,
            vel: spec.initial_velocity(),
            age: 0.0,
            max_lifetime: spec.max_lifetime,
            gravity: spec.gravity,
            half_extent: spec.half_extent,
            damage: spec.damage,
            // Fireballs bounce off the floor a couple of times — a
            // Mario-like / arcade-style behavior, not a literal copy.
            bounces_remaining: matches!(spec.kind, ProjectileKind::Fireball) as u8 * 2,
        }
    }

    pub fn aabb(&self) -> Aabb {
        aabb_from_min_size(
            Vec2::new(
                self.pos.x - self.half_extent.x,
                self.pos.y - self.half_extent.y,
            ),
            Vec2::new(self.half_extent.x * 2.0, self.half_extent.y * 2.0),
        )
    }

    /// Step the projectile forward by `dt`. Returns `true` if the
    /// projectile is still alive after the tick. Collision against
    /// solids / breakables is the caller's responsibility (sandbox).
    ///
    /// `gravity_sign` is the world gravity direction along Y (`+1` down, `-1`
    /// flipped) from `GravityField`, so a gravity flip sends gravity-bearing
    /// projectiles (bombs, apple-rain) up too. Pass `1.0` for normal gravity.
    pub fn tick(&mut self, dt: f32, gravity_sign: f32) -> bool {
        self.age += dt;
        if self.age >= self.max_lifetime {
            return false;
        }
        // Apply gravity along the world's down (positive = downward by default).
        self.vel.y += self.gravity * gravity_sign * dt;
        self.pos += self.vel * dt;
        true
    }

    pub fn is_expired(&self) -> bool {
        self.age >= self.max_lifetime
    }

    /// True when the contact geometry qualifies as a top-of-block
    /// landing: projectile coming from above, moving downward, not
    /// merely grazing the side. Shared by solid- and one-way- hit
    /// resolution so both surfaces use identical bounce geometry.
    fn is_top_landing(&self, block_aabb: Aabb) -> bool {
        let body = self.aabb();
        // Side / ceiling-contact filter: if the projectile's y-range
        // fits inside the block's y-range, the contact is on the
        // block's *side*, not its top. (Mirrors the
        // `body_is_side_contact` predicate that movement.rs uses to
        // skip side walls during the y-sweep.) A 1e-3 epsilon allows
        // an exact-edge-touching projectile that just grazes the
        // floor face.
        const SIDE_EPS: f32 = 1e-3;
        let side_contact = body.top() >= block_aabb.top() - SIDE_EPS
            && body.bottom() <= block_aabb.bottom() + SIDE_EPS;
        if side_contact {
            return false;
        }
        // Floor vs ceiling contact: projectile center above the block
        // center AND moving downward → top-of-block hit.
        let from_above = body.center().y < block_aabb.center().y;
        let going_down = self.vel.y > 0.0;
        from_above && going_down
    }

    /// Reposition the body so its bottom rests on the block's top
    /// edge plus a 1px lift, reflect vy with restitution, and
    /// decrement the bounce budget. Caller has already checked that
    /// `bounces_remaining > 0`.
    fn apply_top_bounce(&mut self, block_aabb: Aabb) {
        // The 1px lift prevents an immediate re-hit on the next tick
        // when gravity hasn't yet reaccelerated downward.
        const RESTITUTION: f32 = 0.65;
        const SETTLE_LIFT: f32 = 1.0;
        self.pos.y = block_aabb.top() - self.half_extent.y - SETTLE_LIFT;
        self.vel.y = -self.vel.y.abs() * RESTITUTION;
        self.bounces_remaining -= 1;
    }

    /// Resolution outcome when this projectile overlaps a solid block.
    /// The caller decides what to do: bounce paths re-position the
    /// body and continue the next tick; expire paths despawn the
    /// projectile (and optionally trigger a hit VFX).
    ///
    /// `Bounced` is reserved for *floor* contacts (top edge of the
    /// block, fireball coming down): the only configuration where a
    /// classic platformer fireball reverses direction. Side and
    /// ceiling contacts always expire so the gameplay is predictable
    /// — a flying horizontal projectile doesn't suddenly retrace its
    /// path back through the player.
    pub fn resolve_solid_hit(&mut self, block_aabb: Aabb) -> ProjectileSolidHit {
        if self.is_top_landing(block_aabb) && self.bounces_remaining > 0 {
            self.apply_top_bounce(block_aabb);
            ProjectileSolidHit::Bounced
        } else {
            ProjectileSolidHit::Expired
        }
    }

    /// Resolution outcome when this projectile overlaps a one-way
    /// platform. A top-of-block landing bounces the same way a solid
    /// floor would — the player expects fireballs to skip across
    /// thin platforms identically to thick floors. Every other
    /// contact (sides, below, top with no bounce budget) returns
    /// `Passthrough` so the projectile keeps flying — the platform
    /// is non-solid from those directions.
    pub fn resolve_one_way_hit(&mut self, block_aabb: Aabb) -> ProjectileSolidHit {
        if self.is_top_landing(block_aabb) && self.bounces_remaining > 0 {
            self.apply_top_bounce(block_aabb);
            ProjectileSolidHit::Bounced
        } else {
            ProjectileSolidHit::Passthrough
        }
    }
}

/// Outcome of [`ProjectileBody::resolve_solid_hit`] /
/// [`ProjectileBody::resolve_one_way_hit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileSolidHit {
    /// Projectile bounced off the block top; `bounces_remaining`
    /// decremented and `vel.y` reflected. Caller keeps the body alive.
    Bounced,
    /// Projectile should be removed (no bounces left on a solid hit,
    /// or contact wasn't a top-of-block landing on a solid).
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
            kind: ProjectileKind::Fireball,
            faction: ProjectileFaction::Player,
            pos,
            vel,
            age: 0.0,
            max_lifetime: 1.0,
            gravity: 0.0,
            half_extent: Vec2::new(4.0, 4.0),
            damage: 2,
            bounces_remaining: bounces,
        }
    }

    #[test]
    fn tick_advances_position_and_applies_gravity() {
        let mut p = fireball(Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0), 0);
        p.gravity = 200.0;
        let alive = p.tick(0.1, 1.0);
        assert!(alive, "still alive well within lifetime");
        // vy gains gravity*dt first, then pos integrates the new velocity.
        assert!((p.vel.y - 20.0).abs() < 1e-3);
        assert!((p.pos.x - 10.0).abs() < 1e-3 && (p.pos.y - 2.0).abs() < 1e-3);
    }

    #[test]
    fn tick_returns_false_and_holds_position_when_expired() {
        let mut p = fireball(Vec2::new(5.0, 5.0), Vec2::new(100.0, 0.0), 0);
        p.max_lifetime = 0.1;
        let alive = p.tick(0.2, 1.0);
        assert!(!alive, "a tick past the lifetime reports dead");
        assert!(p.is_expired());
        assert_eq!(p.pos, Vec2::new(5.0, 5.0), "expiring tick does not move the body");
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
        let block = aabb_from_min_size(Vec2::new(40.0, 54.0), Vec2::new(20.0, 20.0));

        let first = p.resolve_solid_hit(block);
        assert_eq!(first, ProjectileSolidHit::Bounced);
        assert!(p.vel.y < 0.0, "bounce reverses vertical velocity");
        assert_eq!(p.bounces_remaining, 0, "bounce spends the budget");

        // Re-aim downward into the same floor with no budget left → expires.
        p.vel.y = 100.0;
        let second = p.resolve_solid_hit(block);
        assert_eq!(second, ProjectileSolidHit::Expired);
    }

    #[test]
    fn side_contact_expires_instead_of_bouncing() {
        // Body fully inside the block's y-range = a side hit, never a bounce.
        let mut p = fireball(Vec2::new(50.0, 60.0), Vec2::new(100.0, 0.0), 2);
        let block = aabb_from_min_size(Vec2::new(40.0, 40.0), Vec2::new(20.0, 60.0));
        assert_eq!(p.resolve_solid_hit(block), ProjectileSolidHit::Expired);
        assert_eq!(p.bounces_remaining, 2, "a side hit does not spend a bounce");
    }
}
