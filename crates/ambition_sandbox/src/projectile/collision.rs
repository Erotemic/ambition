//! Shared world-block collision resolver for player + enemy projectiles.
//!
//! Both `crate::projectile::update_projectiles` and
//! `crate::enemy_projectile::update_enemy_projectiles` previously
//! re-implemented the "did this projectile body hit a solid /
//! blink-wall / one-way platform this frame?" scan. They differ only
//! in how the outcome is routed (player bounces off floors, enemy
//! shots expire on any solid contact) — the scan itself is
//! identical and faction-dispatched here so a new projectile family
//! (boss volleys, traps, reflected shots) can pick a policy by tag
//! rather than copying a 40-line loop (OVERNIGHT-TODO #17.7).

use crate::engine_core as ae;
use crate::engine_core::AabbExt;

/// Per-faction world-collision policy for projectile bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldHitPolicy {
    /// Player-fired projectile. Bounces off solid + blink-wall
    /// surfaces using `bounces_remaining` (Fireball arc); Hadouken
    /// spawns with 0 bounces and expires on first solid hit. One-way
    /// platforms only block from above when the body would normally
    /// bounce — otherwise the projectile passes through (so a
    /// horizontal Hadouken doesn't get stopped by a thin platform).
    PlayerBouncing,
    /// Enemy-fired projectile. Treats any solid / blink-wall / one-way
    /// contact as expiry; no bouncing (a bouncing volley reads as a
    /// pinball and confuses the player about the hostile path).
    EnemyExpireOnAnyContact,
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

/// Resolve a projectile body against the world's blocks for this
/// tick, dispatching on the per-faction collision policy.
///
/// The body is mutably borrowed because `PlayerBouncing` may decrement
/// `bounces_remaining` via `body.resolve_solid_hit` / `resolve_one_way_hit`;
/// `EnemyExpireOnAnyContact` only reads.
pub fn resolve_world_collision(
    body: &mut crate::projectile::ProjectileBody,
    world: &ae::World,
    policy: WorldHitPolicy,
) -> WorldHitOutcome {
    let aabb = body.aabb();
    match policy {
        WorldHitPolicy::PlayerBouncing => {
            // Solids first so a fireball overlapping both kinds in the
            // same frame resolves against the harder surface (matches
            // the priority used by player physics).
            let solid_hit = world.blocks.iter().find(|block| {
                matches!(
                    block.kind,
                    ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
                ) && block.aabb.strict_intersects(aabb)
            });
            if let Some(block) = solid_hit {
                return match body.resolve_solid_hit(block.aabb) {
                    crate::projectile::ProjectileSolidHit::Bounced => WorldHitOutcome::Bounced { pos: body.pos },
                    crate::projectile::ProjectileSolidHit::Expired => WorldHitOutcome::Expired { pos: body.pos },
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
                let result = body.resolve_one_way_hit(block.aabb);
                if matches!(result, crate::projectile::ProjectileSolidHit::Bounced) {
                    return WorldHitOutcome::Bounced { pos: body.pos };
                }
                // Passthrough on a one-way: keep scanning in case
                // another one-way overlap qualifies as a top-landing.
                // `Expired` is not produced by `resolve_one_way_hit`.
            }
            WorldHitOutcome::Continue
        }
        WorldHitPolicy::EnemyExpireOnAnyContact => {
            let any_hit = world.blocks.iter().any(|block| {
                matches!(
                    block.kind,
                    ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. } | ae::BlockKind::OneWay
                ) && block.aabb.strict_intersects(aabb)
            });
            if any_hit {
                WorldHitOutcome::Expired { pos: body.pos }
            } else {
                WorldHitOutcome::Continue
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_with_block(kind: ae::BlockKind, center: ae::Vec2, half: ae::Vec2) -> ae::World {
        let mut w = ae::World::new("test", ae::Vec2::new(800.0, 600.0), ae::Vec2::ZERO, vec![]);
        w.blocks.push(ae::Block {
            name: "wall".into(),
            aabb: ae::Aabb::new(center, half),
            kind,
        });
        w
    }

    fn straight_projectile(faction: crate::projectile::ProjectileFaction, pos: ae::Vec2) -> crate::projectile::ProjectileBody {
        let spec = crate::projectile::ProjectileSpec {
            kind: crate::projectile::ProjectileKind::Fireball,
            origin: pos,
            direction: ae::Vec2::new(1.0, 0.0),
            damage: 1,
            speed: 200.0,
            max_lifetime: 1.0,
            half_extent: ae::Vec2::new(6.0, 6.0),
            gravity: 0.0,
            charge_tier: 0,
        };
        let mut body = crate::projectile::ProjectileBody::from_spec_with_faction(spec, faction);
        body.bounces_remaining = 0; // baseline: no bouncing
        body
    }

    #[test]
    fn enemy_policy_expires_on_solid_contact() {
        let world = world_with_block(
            ae::BlockKind::Solid,
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(50.0, 50.0),
        );
        let mut body =
            straight_projectile(crate::projectile::ProjectileFaction::Enemy, ae::Vec2::new(100.0, 100.0));
        let outcome =
            resolve_world_collision(&mut body, &world, WorldHitPolicy::EnemyExpireOnAnyContact);
        assert!(matches!(outcome, WorldHitOutcome::Expired { .. }));
    }

    #[test]
    fn enemy_policy_expires_on_one_way_contact() {
        // The "enemy treats one-way as solid" rule is the whole
        // reason enemy/projectile world-collision can't share the
        // player policy directly. Pin it here.
        let world = world_with_block(
            ae::BlockKind::OneWay,
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(50.0, 4.0),
        );
        let mut body =
            straight_projectile(crate::projectile::ProjectileFaction::Enemy, ae::Vec2::new(100.0, 100.0));
        let outcome =
            resolve_world_collision(&mut body, &world, WorldHitPolicy::EnemyExpireOnAnyContact);
        assert!(matches!(outcome, WorldHitOutcome::Expired { .. }));
    }

    #[test]
    fn player_policy_expires_on_solid_when_out_of_bounces() {
        let world = world_with_block(
            ae::BlockKind::Solid,
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(50.0, 50.0),
        );
        let mut body =
            straight_projectile(crate::projectile::ProjectileFaction::Player, ae::Vec2::new(100.0, 100.0));
        // bounces_remaining = 0 (Hadouken)
        let outcome = resolve_world_collision(&mut body, &world, WorldHitPolicy::PlayerBouncing);
        assert!(matches!(outcome, WorldHitOutcome::Expired { .. }));
    }

    #[test]
    fn player_policy_passes_through_one_way_at_zero_bounces() {
        // Player Hadouken (0 bounces) travelling horizontally past a
        // thin one-way platform should NOT be stopped — pins the
        // asymmetry with enemy policy.
        let world = world_with_block(
            ae::BlockKind::OneWay,
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(50.0, 4.0),
        );
        let mut body =
            straight_projectile(crate::projectile::ProjectileFaction::Player, ae::Vec2::new(100.0, 100.0));
        let outcome = resolve_world_collision(&mut body, &world, WorldHitPolicy::PlayerBouncing);
        assert!(matches!(outcome, WorldHitOutcome::Continue));
    }

    #[test]
    fn no_contact_returns_continue() {
        let world = world_with_block(
            ae::BlockKind::Solid,
            ae::Vec2::new(500.0, 500.0),
            ae::Vec2::new(10.0, 10.0),
        );
        let mut body =
            straight_projectile(crate::projectile::ProjectileFaction::Player, ae::Vec2::new(100.0, 100.0));
        let outcome = resolve_world_collision(&mut body, &world, WorldHitPolicy::PlayerBouncing);
        assert!(matches!(outcome, WorldHitOutcome::Continue));
    }
}
