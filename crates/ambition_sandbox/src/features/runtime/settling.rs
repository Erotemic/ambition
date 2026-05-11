use super::*;

/// Run one fixed-`dt` gravity step on a falling chest, sub-stepped so
/// a fast-falling chest can't tunnel through a thin floor. Clears
/// `falling` (and `vel_y`) on the first solid contact below.
///
/// Pulled out of `FeatureRuntime::update` so the boss-encounter
/// "spawn-and-fast-settle" path can run the same physics in a tight
/// loop (see `sync_mockingbird_treasure_chest`) — guaranteeing a
/// looted chest re-spawns at the *same* settled y the live tick
/// would have produced, instead of dropping again on every room load.
pub fn tick_chest_fall(chest: &mut ChestRuntime, world: &ae::World, dt: f32) {
    chest.vel_y = (chest.vel_y + CHEST_FALL_GRAVITY * dt).min(CHEST_FALL_MAX_SPEED);
    let step = chest.vel_y * dt;
    if step <= 0.0 {
        return;
    }
    let max_substep = (chest.size.y * 0.5).max(2.0);
    let mut remaining = step;
    while remaining > 0.0 {
        let advance = remaining.min(max_substep);
        let try_pos = ae::Vec2::new(chest.pos.x, chest.pos.y + advance);
        let try_aabb = ae::Aabb::new(try_pos, chest.size * 0.5);
        let blocked = world.body_overlaps_any(try_aabb, |block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
            )
        });
        if blocked {
            chest.falling = false;
            chest.vel_y = 0.0;
            break;
        }
        chest.pos = try_pos;
        remaining -= advance;
    }
}
