//! Per-frame projectile state, split into a generic kinematic body
//! (the lower-crate [`BodyKinematics`]) and the projectile *gameplay*
//! state ([`ProjectileGameplay`]) plus the engine-side floor/wall
//! resolution ([`ProjectileGameplay::resolve_solid_hit`]).
//!
//! ## Why the split (Stage 19 Phase 3a)
//!
//! The kinematic half — position, velocity, size — is the SAME shape every
//! controllable body uses ([`ambition_engine_core::BodyKinematics`], re-exported
//! by the runtime). Sharing it means a projectile *entity* can carry the exact
//! component the player / enemy / boss carry, so the portal transit machine
//! (which queries `(&mut BodyKinematics, With<PortalBody>)`) drives projectiles
//! "for free" once they become entities (Phase 3c/3d → Phase 4). The gameplay
//! half — kind, faction, lifetime, gravity, damage, bounce budget — is
//! projectile-specific and stays in [`ProjectileGameplay`].
//!
//! [`ProjectileBody`] composes the two for the still-`Vec`-pooled callers; its
//! field accessors (`.pos`, `.vel`, `.kind`, …) forward to the appropriate half
//! so existing call sites read unchanged until the ECS migration moves the two
//! halves onto separate components.

use ambition_engine_core::BodyKinematics;
use ambition_engine_core::Vec2;

use super::spec::{ProjectileKind, ProjectileSpec};
use ambition_engine_core::{Aabb, AabbExt};

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

/// Projectile *gameplay* state: identity (kind/faction), lifetime,
/// gravity, damage, and bounce budget. The kinematic half (pos / vel /
/// size) lives separately in [`BodyKinematics`] so a projectile entity
/// can carry the same body component the player / enemy / boss carry.
///
/// The collision/lifetime methods take the kinematic body by reference
/// so the two halves can be stored on separate ECS components after the
/// migration without changing this logic.
///
/// As of Stage 19 Phase 3c-i this is an ECS [`Component`]: it is the
/// projectile *marker*. Any actor-generic system that queries the shared
/// [`BodyKinematics`] excludes projectiles with `Without<ProjectileGameplay>`
/// (e.g. [`crate::orientation::ensure_actor_roll`]) so a projectile entity
/// carrying `BodyKinematics` (Phase 3c-ii onward) is never swept into actor
/// behavior (auto-righting, portal transit, AI, …).
#[derive(Clone, Copy, Debug, PartialEq, bevy::prelude::Component)]
pub struct ProjectileGameplay {
    pub kind: ProjectileKind,
    /// Combat faction (who fired this projectile, which targets it
    /// may damage). Set at spawn time; the engine never mutates it.
    pub faction: ProjectileFaction,
    pub age: f32,
    pub max_lifetime: f32,
    pub gravity: f32,
    pub damage: i32,
    pub bounces_remaining: u8,
}

impl ProjectileGameplay {
    /// Build the gameplay half from `spec` with an explicit faction.
    fn from_spec_with_faction(spec: ProjectileSpec, faction: ProjectileFaction) -> Self {
        Self {
            kind: spec.kind,
            faction,
            age: 0.0,
            max_lifetime: spec.max_lifetime,
            gravity: spec.gravity,
            damage: spec.damage,
            // Fireballs bounce off the floor a couple of times — a
            // Mario-like / arcade-style behavior, not a literal copy.
            bounces_remaining: matches!(spec.kind, ProjectileKind::Fireball) as u8 * 2,
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
    /// `gravity_sign` is the world gravity direction along Y (`+1` down,
    /// `-1` flipped) from `GravityField`, so a gravity flip sends
    /// gravity-bearing projectiles (bombs, apple-rain) up too. Pass
    /// `1.0` for normal gravity.
    pub fn tick(&mut self, body: &mut BodyKinematics, dt: f32, gravity_sign: f32) -> bool {
        self.age += dt;
        if self.age >= self.max_lifetime {
            return false;
        }
        // Apply gravity along the world's down (positive = downward by default).
        body.vel.y += self.gravity * gravity_sign * dt;
        body.pos += body.vel * dt;
        true
    }

    /// True when the contact geometry qualifies as a top-of-block
    /// landing: projectile coming from above, moving downward, not
    /// merely grazing the side. Shared by solid- and one-way- hit
    /// resolution so both surfaces use identical bounce geometry.
    fn is_top_landing(&self, body: &BodyKinematics, block_aabb: Aabb) -> bool {
        let body_aabb = body.aabb();
        // Side / ceiling-contact filter: if the projectile's y-range
        // fits inside the block's y-range, the contact is on the
        // block's *side*, not its top. (Mirrors the
        // `body_is_side_contact` predicate that movement.rs uses to
        // skip side walls during the y-sweep.) A 1e-3 epsilon allows
        // an exact-edge-touching projectile that just grazes the
        // floor face.
        const SIDE_EPS: f32 = 1e-3;
        let side_contact = body_aabb.top() >= block_aabb.top() - SIDE_EPS
            && body_aabb.bottom() <= block_aabb.bottom() + SIDE_EPS;
        if side_contact {
            return false;
        }
        // Floor vs ceiling contact: projectile center above the block
        // center AND moving downward → top-of-block hit.
        let from_above = body_aabb.center().y < block_aabb.center().y;
        let going_down = body.vel.y > 0.0;
        from_above && going_down
    }

    /// Reposition the body so its bottom rests on the block's top
    /// edge plus a 1px lift, reflect vy with restitution, and
    /// decrement the bounce budget. Caller has already checked that
    /// `bounces_remaining > 0`.
    fn apply_top_bounce(&mut self, body: &mut BodyKinematics, block_aabb: Aabb) {
        // The 1px lift prevents an immediate re-hit on the next tick
        // when gravity hasn't yet reaccelerated downward.
        const RESTITUTION: f32 = 0.65;
        const SETTLE_LIFT: f32 = 1.0;
        let half_extent_y = body.size.y * 0.5;
        body.pos.y = block_aabb.top() - half_extent_y - SETTLE_LIFT;
        body.vel.y = -body.vel.y.abs() * RESTITUTION;
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
    pub fn resolve_solid_hit(
        &mut self,
        body: &mut BodyKinematics,
        block_aabb: Aabb,
    ) -> ProjectileSolidHit {
        if self.is_top_landing(body, block_aabb) && self.bounces_remaining > 0 {
            self.apply_top_bounce(body, block_aabb);
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
    pub fn resolve_one_way_hit(
        &mut self,
        body: &mut BodyKinematics,
        block_aabb: Aabb,
    ) -> ProjectileSolidHit {
        if self.is_top_landing(body, block_aabb) && self.bounces_remaining > 0 {
            self.apply_top_bounce(body, block_aabb);
            ProjectileSolidHit::Bounced
        } else {
            ProjectileSolidHit::Passthrough
        }
    }
}

/// Per-frame physics state of an in-flight projectile: the kinematic
/// [`BodyKinematics`] plus the projectile [`ProjectileGameplay`]. The
/// two halves are stored as named fields; the convenience field-style
/// accessors below (`.pos`, `.vel`, `.kind`, …) forward to the right
/// half so the still-`Vec`-pooled call sites read unchanged.
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

    /// Build a [`ProjectileBody`] from already-split halves. Used by the
    /// ECS step systems (Phase 3c) to re-compose a body for the shared
    /// collision resolver from the entity's `BodyKinematics` +
    /// `ProjectileGameplay` components.
    pub fn from_parts(kin: BodyKinematics, game: ProjectileGameplay) -> Self {
        Self { kin, game }
    }

    pub fn aabb(&self) -> Aabb {
        self.kin.aabb()
    }

    /// Step the projectile forward by `dt`. Returns `true` if the
    /// projectile is still alive after the tick. Delegates to
    /// [`ProjectileGameplay::tick`] on the split halves.
    pub fn tick(&mut self, dt: f32, gravity_sign: f32) -> bool {
        self.game.tick(&mut self.kin, dt, gravity_sign)
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

    // --- Field-style accessors: forward to the appropriate half so the
    // --- still-Vec-pooled call sites (and tests) read `.pos`, `.kind`,
    // --- … unchanged through the Phase-3a split.

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
    pub fn kind(&self) -> ProjectileKind {
        self.game.kind
    }
    pub fn faction(&self) -> ProjectileFaction {
        self.game.faction
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
            kin: BodyKinematics {
                pos,
                vel,
                size: Vec2::new(8.0, 8.0),
                facing: 1.0,
            },
            game: ProjectileGameplay {
                kind: ProjectileKind::Fireball,
                faction: ProjectileFaction::Player,
                age: 0.0,
                max_lifetime: 1.0,
                gravity: 0.0,
                damage: 2,
                bounces_remaining: bounces,
            },
        }
    }

    #[test]
    fn tick_advances_position_and_applies_gravity() {
        let mut p = fireball(Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0), 0);
        p.game.gravity = 200.0;
        let alive = p.tick(0.1, 1.0);
        assert!(alive, "still alive well within lifetime");
        // vy gains gravity*dt first, then pos integrates the new velocity.
        assert!((p.kin.vel.y - 20.0).abs() < 1e-3);
        assert!((p.kin.pos.x - 10.0).abs() < 1e-3 && (p.kin.pos.y - 2.0).abs() < 1e-3);
    }

    #[test]
    fn tick_returns_false_and_holds_position_when_expired() {
        let mut p = fireball(Vec2::new(5.0, 5.0), Vec2::new(100.0, 0.0), 0);
        p.game.max_lifetime = 0.1;
        let alive = p.tick(0.2, 1.0);
        assert!(!alive, "a tick past the lifetime reports dead");
        assert!(p.is_expired());
        assert_eq!(
            p.kin.pos,
            Vec2::new(5.0, 5.0),
            "expiring tick does not move the body"
        );
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
