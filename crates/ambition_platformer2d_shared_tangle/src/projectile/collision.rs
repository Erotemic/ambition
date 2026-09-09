//! Shared world-block collision resolver for player + enemy projectiles.
//!
//! The world scan is common; callers choose the outcome policy by faction.
//! Spawn/damage routing stays in the consuming game.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_core::BodyKinematics;

use super::body::ProjectileGameplay;

/// How a projectile interacts with world geometry — a property of the
/// projectile (its ability/spec), not of who fired it: the same shot
/// behaves identically whether the player, an enemy, or the player-robot boss
/// fires it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldHitPolicy {
    /// An arcing shot. Bounces off solid + blink-wall surfaces using
    /// `bounces_remaining`; a 0-bounce shot expires on first solid hit. One-way
    /// platforms only block from above when the body would normally bounce —
    /// otherwise the projectile passes through (so a horizontal 0-bounce shot
    /// doesn't get stopped by a thin platform).
    Bouncing,
    /// A straight shot that dies on first world contact: any solid / blink-wall
    /// / one-way contact is expiry, no bouncing (a bouncing volley reads as a
    /// pinball and confuses the reader about the projectile's path).
    ExpireOnContact,
}

/// Outcome of a single per-tick world-block resolution call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldHitOutcome {
    /// Projectile bounced; stays alive. Caller plays the bounce SFX.
    Bounced { pos: ae::Vec2 },
    /// Projectile expired on contact; caller plays impact VFX +
    /// drops the body from the in-flight list.
    Expired { pos: ae::Vec2 },
    /// No contact this frame; body stays in flight unchanged.
    Continue,
}

/// ⛔⛔ **ONE ANSWER TO "DOES THIS BLOCK STOP THIS SHOT" — read by the
/// obstruction sweep that ORDERS contacts and by the physical response that
/// resolves them.**
///
/// There were two answers, and they disagreed on exactly the case a one-way
/// exists for. The caller's obstruction filter excluded EVERY `OneWay` for a
/// `Bouncing` shot, while [`resolve_world_collision`] below checks one-ways and
/// bounces whenever the approach qualifies. So a fireball descending onto a
/// one-way was ORDERED as though nothing were in the way — letting it damage a
/// target standing behind the platform — and then physically bounced off it.
/// The contact protocol requires the same world-hit policy on both sides, and
/// requires it to name the admitted approach direction for a directional
/// one-way.
///
/// ⚠ **ONLY LEG-TRUE FACTS LIVE HERE.** The obstruction side asks this per
/// candidate block across the whole travel leg, so it may use the shot's
/// policy, its bounce budget, the block's kind, and whether the shot travels
/// toward that surface's support face at all — but NOT where the shot ended up.
/// The positional straddle ("is this really a landing, or the top face of a
/// wall I am passing halfway down?") is each side's own, layered on top of this.
pub fn shot_policy_admits(
    policy: WorldHitPolicy,
    bounces_remaining: u8,
    velocity: ae::Vec2,
    gravity_dir: ae::Vec2,
    kind: &ae::BlockKind,
) -> bool {
    match policy {
        WorldHitPolicy::Bouncing => match kind {
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. } => true,
            // A fireball crosses a one-way from below BY DESIGN, and is stopped
            // by one it is DESCENDING ONTO. A shot with no bounce budget left
            // passes through rather than expiring, which is what
            // `resolve_one_way_hit_in_frame` does with it.
            //
            // ⚠ WHICH FACE the shot meets is NOT decided here. Both sides add
            // their own positional test, because they ask at different moments:
            // the sweep at the leg's start ([`block_obstructs_shot`]), the
            // response at the selected contact (`is_support_landing`). Putting
            // either one here breaks the other — measured both ways.
            ae::BlockKind::OneWay => {
                bounces_remaining > 0 && velocity.dot(super::body::projectile_down(gravity_dir)) > 0.0
            }
            _ => false,
        },
        // Any solid / blink-wall / one-way contact is expiry, from any
        // direction: this shot's contract is that it dies on what it touches.
        WorldHitPolicy::ExpireOnContact => matches!(
            kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. } | ae::BlockKind::OneWay
        ),
    }
}

/// Does `block` obstruct this shot ALONG ITS LEG — the question the swept
/// contact ordering asks, once per candidate, before it knows where the shot
/// stops.
///
/// [`shot_policy_admits`] supplies the shared policy; this adds the half that is
/// true of a LEG rather than of a resting position.
///
/// ⛔⛔ **DIRECTION ALONE IS NOT THE RULE, AND A GRAVITY-FED `vy` IS WHY.** A
/// shot fired horizontally still has a downward velocity component within one
/// tick, so a direction-only gate let a TALL, VERTICAL one-way block a sideways
/// shot — which the response never does, because `is_support_landing` requires
/// the body to be AT that surface's support face rather than halfway down its
/// side. `a_one_way_blocks_the_shot_whose_policy_says_it_should_and_no_other`
/// went red on exactly that.
///
/// ⇒ So: was the shot entirely on the SUPPORT SIDE of that face when the leg
/// began? A platform stops what falls onto it, not what passes its edge.
///
/// ⛔ AND THIS TEST IS NOT THE RESPONSE'S. Applying it there — where the shot
/// already straddles the face by construction, that being what a contact IS —
/// stopped `fireball_bounces_off_one_way_platform_in_system` bouncing at all.
/// Two moments, two positional tests, one shared policy.
pub fn block_obstructs_shot(
    policy: WorldHitPolicy,
    bounces_remaining: u8,
    velocity: ae::Vec2,
    body_at_leg_start: ae::Aabb,
    gravity_dir: ae::Vec2,
    block: &ae::Block,
) -> bool {
    if !shot_policy_admits(policy, bounces_remaining, velocity, gravity_dir, &block.kind) {
        return false;
    }
    // ⚠ THE SUPPORT-SIDE TEST IS A `Bouncing` CONCEPT AND ONLY THAT. An
    // `ExpireOnContact` shot dies on a one-way from ANY direction — its
    // contract is that it ends on what it touches — so gating it on approach
    // let one damage a body straight through a platform, which is the original
    // defect this fixture was written for. Caught by the `ExpireOnContact` arm
    // of `a_one_way_blocks_the_shot_whose_policy_says_it_should_and_no_other`.
    if !matches!(
        (policy, &block.kind),
        (WorldHitPolicy::Bouncing, ae::BlockKind::OneWay)
    ) {
        return true;
    }
    let down = super::body::projectile_down(gravity_dir);
    body_at_leg_start.feet_coord(down)
        <= block.aabb.head_coord(down) + super::body::CONTACT_SLOP
}

/// Resolve a projectile against the world's blocks for this tick,
/// dispatching on the per-faction collision policy.
///
/// The halves are mutably borrowed because `Bouncing` may decrement
/// `bounces_remaining` and reposition the body; `ExpireOnContact`
/// only reads.
pub fn resolve_world_collision(
    kin: &mut BodyKinematics,
    game: &mut ProjectileGameplay,
    world: &ae::World,
    policy: WorldHitPolicy,
    gravity_dir: ae::Vec2,
) -> WorldHitOutcome {
    let aabb = kin.aabb();
    // ⭐ Snapshotted before anything below can move or spend them, so the
    // admission this function grants is the same one the obstruction sweep was
    // granted over this leg.
    let bounces = game.bounces_remaining;
    let velocity = kin.vel;
    // ⚠ THE SHARED POLICY, NOT THE SWEEP'S LEG TEST. This side's positional
    // question is `is_support_landing`, asked below at the shot's actual
    // contact; layering the leg-start test here as well refuses the very
    // straddle a contact IS.
    let admits = |block: &ae::Block| {
        shot_policy_admits(policy, bounces, velocity, gravity_dir, &block.kind)
    };
    match policy {
        WorldHitPolicy::Bouncing => {
            // Solids first so a bouncing shot overlapping both kinds in the
            // same frame resolves against the harder surface (matches
            // the priority used by player physics).
            let solid_hit = world.blocks.iter().find(|block| {
                matches!(
                    block.kind,
                    ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
                ) && admits(block)
                    && block.aabb.strict_intersects(aabb)
            });
            if let Some(block) = solid_hit {
                return match game.resolve_solid_hit_in_frame(kin, block.aabb, gravity_dir) {
                    crate::projectile::ProjectileSolidHit::Bounced => {
                        WorldHitOutcome::Bounced { pos: kin.pos }
                    }
                    crate::projectile::ProjectileSolidHit::Expired => {
                        WorldHitOutcome::Expired { pos: kin.pos }
                    }
                    crate::projectile::ProjectileSolidHit::Passthrough => WorldHitOutcome::Continue,
                };
            }
            for block in &world.blocks {
                if !matches!(block.kind, ae::BlockKind::OneWay) {
                    continue;
                }
                // ⭐ THE SAME ADMISSION THE ORDERING USED. Without this the two
                // sides are free to disagree again, which is the whole defect:
                // the sweep must not order a one-way away and then find the
                // response bouncing off it.
                if !admits(block) {
                    continue;
                }
                if !block.aabb.strict_intersects(aabb) {
                    continue;
                }
                let result = game.resolve_one_way_hit_in_frame(kin, block.aabb, gravity_dir);
                if matches!(result, crate::projectile::ProjectileSolidHit::Bounced) {
                    return WorldHitOutcome::Bounced { pos: kin.pos };
                }
                // Passthrough on a one-way: keep scanning in case
                // another one-way overlap qualifies as a top-landing.
                // `Expired` is not produced by `resolve_one_way_hit`.
            }
            WorldHitOutcome::Continue
        }
        WorldHitPolicy::ExpireOnContact => {
            let any_hit = world.blocks.iter().any(|block| {
                admits(block) && block.aabb.strict_intersects(aabb)
            });
            if any_hit {
                WorldHitOutcome::Expired { pos: kin.pos }
            } else {
                WorldHitOutcome::Continue
            }
        }
    }
}

#[cfg(test)]
mod tests;
