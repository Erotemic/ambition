//! Falling-chest physics for ECS reward chests.
//!
//! Reward chests spawned mid-air by `sync_boss_reward_chests_ecs`
//! carry a [`FallingChest`] until they land on a solid floor. This
//! module owns both the per-frame tick and the headless precomputed
//! settle used when a save says the chest has already been looted
//! (so it can be drawn pre-landed without ever animating).

use super::*;
use crate::features::{CHEST_FALL_GRAVITY, CHEST_FALL_MAX_SPEED};

/// Tick ECS reward chests that are still falling to the floor.
pub fn update_ecs_falling_chests(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    world: Res<crate::GameWorld>,
    mut chests: Query<(Entity, &mut FeatureAabb, &mut FallingChest), With<ChestFeature>>,
) {
    // Sim clock: bullet-time / pause / hitstop must freeze a falling
    // chest mid-arc the same way they freeze the player. ADR 0010.
    let dt = world_time.sim_dt();
    for (entity, mut aabb, mut falling) in &mut chests {
        falling.vel_y = (falling.vel_y + CHEST_FALL_GRAVITY * dt).min(CHEST_FALL_MAX_SPEED);
        let step = falling.vel_y * dt;
        if step <= 0.0 {
            continue;
        }
        let max_substep = aabb.half_size.y.max(2.0);
        let mut remaining = step;
        while remaining > 0.0 {
            let advance = remaining.min(max_substep);
            let try_center = ae::Vec2::new(aabb.center.x, aabb.center.y + advance);
            let try_aabb = ae::Aabb::new(try_center, aabb.half_size);
            let blocked = world.0.body_overlaps_any(try_aabb, |block| {
                matches!(
                    block.kind,
                    ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
                )
            });
            if blocked {
                commands.entity(entity).remove::<FallingChest>();
                break;
            }
            aabb.center = try_center;
            remaining -= advance;
        }
    }
}

/// Run the falling-chest tick virtually to find the chest's final
/// resting position. Used when a save says the boss reward is already
/// looted — the chest spawns pre-settled so the player doesn't see a
/// reward animation for an encounter they cleared in an earlier run.
pub(super) fn settled_chest_center(world: &ae::World, start: ae::Vec2, size: ae::Vec2) -> ae::Vec2 {
    let mut center = start;
    let half_size = size * 0.5;
    let mut vel_y: f32 = 0.0;
    let virtual_dt = 1.0 / 60.0;
    for _ in 0..240 {
        vel_y = (vel_y + CHEST_FALL_GRAVITY * virtual_dt).min(CHEST_FALL_MAX_SPEED);
        let step = vel_y * virtual_dt;
        if step <= 0.0 {
            continue;
        }
        let max_substep = half_size.y.max(2.0);
        let mut remaining = step;
        while remaining > 0.0 {
            let advance = remaining.min(max_substep);
            let try_center = ae::Vec2::new(center.x, center.y + advance);
            let try_aabb = ae::Aabb::new(try_center, half_size);
            let blocked = world.body_overlaps_any(try_aabb, |block| {
                matches!(
                    block.kind,
                    ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
                )
            });
            if blocked {
                return center;
            }
            center = try_center;
            remaining -= advance;
        }
    }
    center
}
