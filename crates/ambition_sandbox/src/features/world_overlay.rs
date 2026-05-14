use super::*;

pub fn world_with_sandbox_solids(
    world: &ae::World,
    platforms: &[MovingPlatformState],
    features: &FeatureRuntime,
    ecs_overlay: &FeatureEcsWorldOverlay,
) -> ae::World {
    let mut collision_world = crate::platforms::world_with_moving_platforms(world, platforms);
    collision_world.blocks.extend(ecs_overlay.blocks.iter().cloned());
    for breakable in &features.breakables {
        if breakable.broken() {
            continue;
        }
        // Legacy compatibility: dynamic/runtime-owned breakables still
        // contribute until their callers are migrated to ECS feature entities.
        if breakable.breakable.pogo_refresh {
            collision_world.blocks.push(ae::Block {
                name: format!("breakable-pogo {}", breakable.name),
                aabb: breakable.aabb(),
                kind: ae::BlockKind::PogoOrb,
            });
            continue;
        }
        let kind = match breakable.breakable.collision {
            ae::BreakableCollision::None => continue,
            ae::BreakableCollision::Solid => ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Hard,
            },
            ae::BreakableCollision::OneWayUp => ae::BlockKind::OneWay,
        };
        collision_world.blocks.push(ae::Block {
            name: format!("breakable {}", breakable.name),
            aabb: breakable.aabb(),
            kind,
        });
        if breakable.breaks_on_stand() {
            collision_world.blocks.push(ae::Block {
                name: format!("breakable-pogo-target {}", breakable.name),
                aabb: breakable.aabb(),
                kind: ae::BlockKind::PogoOrb,
            });
        }
    }
    collision_world
}
