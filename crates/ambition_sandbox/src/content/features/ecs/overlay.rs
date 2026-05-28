//! Sandbox collision-world overlay rebuilt from ECS feature state.
//!
//! The overlay is the bridge between the static ECS world (loaded
//! from LDtk) and the dynamic feature state (broken breakables, alive
//! enemies, boss bodies). Engine code that needs the augmented
//! collision world calls `world_with_sandbox_solids` with this
//! resource; rebuilding it once per frame keeps the augment cheap.

use super::*;

/// Collision contribution from ECS-owned breakables. Rebuilt before the main
/// sandbox tick and consumed by `world_with_sandbox_solids` anywhere the engine
/// needs the augmented collision world.
#[derive(Resource, Default, Clone, Debug)]
pub struct FeatureEcsWorldOverlay {
    pub blocks: Vec<ae::Block>,
}

/// Rebuild the transient collision blocks contributed by ECS-owned breakables.
pub fn rebuild_feature_ecs_world_overlay(
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
    breakables: Query<
        (&FeatureId, &FeatureName, &FeatureAabb, &BreakableFeature),
        With<FeatureSimEntity>,
    >,
    actors: Query<(&FeatureId, &FeatureAabb, &ActorRuntime), With<FeatureSimEntity>>,
    bosses: Query<(&FeatureId, &FeatureAabb, &BossFeature), With<FeatureSimEntity>>,
) {
    overlay.blocks.clear();
    for (id, name, aabb, feature) in &breakables {
        if feature.broken() {
            continue;
        }
        if feature.breakable.pogo_refresh {
            overlay.blocks.push(ae::Block {
                name: format!("ecs-breakable-pogo {}", name.0.as_str()),
                aabb: aabb.aabb(),
                kind: ae::BlockKind::PogoOrb,
            });
            continue;
        }
        let kind = match feature.breakable.collision {
            crate::interaction::BreakableCollision::None => continue,
            crate::interaction::BreakableCollision::Solid => ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Hard,
            },
            crate::interaction::BreakableCollision::OneWayUp => ae::BlockKind::OneWay,
        };
        overlay.blocks.push(ae::Block {
            name: format!("ecs-breakable {}", name.0.as_str()),
            aabb: aabb.aabb(),
            kind,
        });
        if feature.breakable.collision.blocks_movement() && feature.breakable.trigger.allows_stand()
        {
            overlay.blocks.push(ae::Block {
                name: format!("ecs-breakable-pogo-target {}", id.as_str()),
                aabb: aabb.aabb(),
                kind: ae::BlockKind::PogoOrb,
            });
        }
    }

    // Expose alive enemy and boss bodies as PogoOrb ghost-blocks so the
    // pogo-attack advance code can bounce off them without requiring the
    // damage queue to resolve first. PogoOrb blocks do not block player
    // movement or blink traversal, so this cannot cause collision regressions.
    for (id, aabb, actor) in &actors {
        let ActorRuntime::Hostile(enemy) = actor else {
            continue;
        };
        if !enemy.alive {
            continue;
        }
        overlay.blocks.push(ae::Block {
            name: format!("ecs-enemy-body {}", id.as_str()),
            aabb: aabb.aabb(),
            kind: ae::BlockKind::PogoOrb,
        });
    }
    for (id, _aabb, feature) in &bosses {
        let boss = &feature.boss;
        if !boss.alive {
            continue;
        }
        // Use the boss's *combat* AABB (sprite-derived
        // `body_pixel_bbox` × spawn size) rather than the
        // `FeatureAabb` (which is the full spawn / render
        // envelope). The render envelope can extend well past
        // the visible body — at 128×160 spawn the FeatureAabb
        // includes ~22 px of empty space around the sprite
        // body, and pogo would "land" in that empty strip with
        // no visual feedback. The combat AABB matches the
        // orange debug box and the body-contact damage zone
        // exactly, so the player gets predictable pogo targets
        // aligned with what they can see.
        overlay.blocks.push(ae::Block {
            name: format!("ecs-boss-body {}", id.as_str()),
            aabb: boss.aabb(),
            kind: ae::BlockKind::PogoOrb,
        });
    }
}
