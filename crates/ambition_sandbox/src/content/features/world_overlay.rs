use super::*;

pub fn world_with_sandbox_solids(
    world: &ae::World,
    platforms: &[MovingPlatformState],
    ecs_overlay: &FeatureEcsWorldOverlay,
) -> ae::World {
    let mut collision_world = crate::platforms::world_with_moving_platforms(world, platforms);
    collision_world.blocks.extend(ecs_overlay.blocks.iter().cloned());
    collision_world
}
