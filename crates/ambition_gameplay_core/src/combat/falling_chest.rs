//! Falling-chest physics for ECS reward chests.
//!
//! Reward chests spawned mid-air by `sync_boss_reward_chests_ecs`
//! carry a [`FallingChest`] until they land on a solid floor. This
//! module owns both the per-frame tick and the headless precomputed
//! settle used when a save says the chest has already been looted
//! (so it can be drawn pre-landed without ever animating).

use super::*;
use super::{CHEST_FALL_GRAVITY, CHEST_FALL_MAX_SPEED};

/// Tick ECS reward chests that are still falling to the floor.
pub fn update_ecs_falling_chests(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    world: Res<ambition_engine_core::RoomGeometry>,
    mut chests: Query<(Entity, &mut CenteredAabb, &mut FallingChest), With<ChestFeature>>,
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
pub(crate) fn settled_chest_center(world: &ae::World, start: ae::Vec2, size: ae::Vec2) -> ae::Vec2 {
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

#[cfg(test)]
mod falling_chest_tests {
    //! settled_chest_center drops a reward chest under gravity until its
    //! body would overlap a solid, then returns the last clear position.
    //! Used to place a chest that was looted before it finished falling.
    use super::*;

    fn world_with_floor() -> ae::World {
        ae::World::new(
            "t",
            ae::Vec2::new(400.0, 400.0),
            ae::Vec2::new(50.0, 50.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 300.0),
                ae::Vec2::new(400.0, 100.0),
            )],
        )
    }

    #[test]
    fn chest_settles_just_above_the_floor() {
        let world = world_with_floor();
        let half = ae::Vec2::new(12.0, 12.0);
        let settled = settled_chest_center(
            &world,
            ae::Vec2::new(200.0, 50.0),
            ae::Vec2::new(24.0, 24.0),
        );
        assert_eq!(settled.x, 200.0, "no horizontal drift");
        assert!(settled.y > 50.0, "the chest fell");
        let body = ae::Aabb::new(settled, half);
        assert!(
            !world.body_overlaps_any(body, |b| matches!(b.kind, ae::BlockKind::Solid)),
            "settled body must not overlap the floor (settled {settled:?})"
        );
        assert!(
            settled.y + half.y <= 300.0,
            "chest bottom stays above the floor top"
        );
        assert!(
            300.0 - (settled.y + half.y) <= 13.0,
            "chest comes to rest within a substep of the floor"
        );
    }

    #[test]
    fn chest_keeps_falling_without_a_floor() {
        let world = ae::World::new(
            "t",
            ae::Vec2::new(400.0, 9999.0),
            ae::Vec2::new(50.0, 50.0),
            Vec::new(),
        );
        let settled = settled_chest_center(
            &world,
            ae::Vec2::new(200.0, 50.0),
            ae::Vec2::new(24.0, 24.0),
        );
        assert!(
            settled.y > 100.0,
            "with no floor the chest keeps falling (settled {settled:?})"
        );
    }
}
