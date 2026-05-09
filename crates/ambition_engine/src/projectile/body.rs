//! Per-frame projectile state plus the engine-side floor/wall
//! resolution (`ProjectileBody::resolve_solid_hit`).

use bevy_math::Vec2;

use super::spec::{ProjectileKind, ProjectileSpec};
use crate::geometry::{aabb_from_min_size, Aabb, AabbExt};

/// Per-frame physics state of an in-flight projectile. Sandbox owns
/// the world-collision check; this struct only owns position, velocity,
/// and lifetime.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileBody {
    pub kind: ProjectileKind,
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
    pub fn from_spec(spec: ProjectileSpec) -> Self {
        Self {
            kind: spec.kind,
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
    pub fn tick(&mut self, dt: f32) -> bool {
        self.age += dt;
        if self.age >= self.max_lifetime {
            return false;
        }
        // Apply gravity (positive = downward in sandbox conventions).
        self.vel.y += self.gravity * dt;
        self.pos += self.vel * dt;
        true
    }

    pub fn is_expired(&self) -> bool {
        self.age >= self.max_lifetime
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
        let body = self.aabb();
        // Side / ceiling-contact filter: if the projectile's y-range
        // fits inside the block's y-range, the contact is on the
        // block's *side*, not its top. (Mirrors the
        // `body_is_side_contact` predicate that movement.rs uses to
        // skip side walls during the y-sweep — same idea, applied
        // here so a horizontal projectile flying past a tall wall
        // doesn't get classified as a floor landing.) A 1e-3 epsilon
        // allows for an exact-edge-touching projectile that just
        // grazes the floor / ceiling face.
        const SIDE_EPS: f32 = 1e-3;
        let side_contact = body.top() >= block_aabb.top() - SIDE_EPS
            && body.bottom() <= block_aabb.bottom() + SIDE_EPS;
        if side_contact {
            return ProjectileSolidHit::Expired;
        }
        // Floor vs ceiling contact: projectile center above the block
        // center AND moving downward → top-of-block hit. Anything else
        // (ceiling, sub-pixel hover-up, bounced-up grazing the floor
        // again) expires.
        let from_above = body.center().y < block_aabb.center().y;
        let going_down = self.vel.y > 0.0;
        if !from_above || !going_down || self.bounces_remaining == 0 {
            return ProjectileSolidHit::Expired;
        }
        // Reposition so the body's bottom rests on the block's top
        // edge plus a 1px lift, then reflect vy with restitution. The
        // 1px lift prevents an immediate re-hit on the next tick when
        // gravity hasn't yet reaccelerated downward.
        const RESTITUTION: f32 = 0.65;
        const SETTLE_LIFT: f32 = 1.0;
        self.pos.y = block_aabb.top() - self.half_extent.y - SETTLE_LIFT;
        self.vel.y = -self.vel.y.abs() * RESTITUTION;
        self.bounces_remaining -= 1;
        ProjectileSolidHit::Bounced
    }
}

/// Outcome of [`ProjectileBody::resolve_solid_hit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileSolidHit {
    /// Projectile bounced off the block top; `bounces_remaining`
    /// decremented and `vel.y` reflected. Caller keeps the body alive.
    Bounced,
    /// Projectile should be removed (no bounces left, or contact wasn't
    /// a top-of-block landing).
    Expired,
}
