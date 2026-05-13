use super::*;

pub fn world_with_sandbox_solids(
    world: &ae::World,
    platforms: &[MovingPlatformState],
    features: &FeatureRuntime,
) -> ae::World {
    let mut collision_world = crate::platforms::world_with_moving_platforms(world, platforms);
    for breakable in &features.breakables {
        if breakable.broken() {
            continue;
        }
        // Breakable pogo orbs contribute a pogo-orb block (no body collision)
        // while intact, so the engine's pogo-bounce logic finds them; the
        // bounce damage is routed back through `FeatureRuntime::on_pogo_bounce`
        // by the gameplay loop.
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
            // Solid breakables behave like a hard blink wall for blink
            // pathing: identical to BlockKind::Solid for ordinary movement
            // (BlinkWall is solid on both axes), but max-tier blink with
            // `blink_through_hard_walls` can teleport through. Lower-tier
            // blink is still blocked, so the breakable still gates progress
            // until the player either earns the upgrade or breaks it.
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
    }
    collision_world
}
