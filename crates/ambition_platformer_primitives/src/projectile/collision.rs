//! Shared world-block collision resolver for player + enemy projectiles.
//!
//! The world scan is common; callers choose the outcome policy by faction.
//! Spawn/damage routing stays in the consuming game.

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use ambition_engine_core::BodyKinematics;

use super::body::ProjectileGameplay;

/// How a projectile interacts with world geometry — a property of the
/// projectile (its ability/spec), **not** of who fired it: the same shot
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
    match policy {
        WorldHitPolicy::Bouncing => {
            // Solids first so a bouncing shot overlapping both kinds in the
            // same frame resolves against the harder surface (matches
            // the priority used by player physics).
            let solid_hit = world.blocks.iter().find(|block| {
                matches!(
                    block.kind,
                    ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
                ) && block.aabb.strict_intersects(aabb)
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
                matches!(
                    block.kind,
                    ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. } | ae::BlockKind::OneWay
                ) && block.aabb.strict_intersects(aabb)
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
