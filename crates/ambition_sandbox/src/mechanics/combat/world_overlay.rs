use super::overlay::FeatureEcsWorldOverlay;
use super::*;

pub fn world_with_sandbox_solids(
    world: &ae::World,
    platforms: &[MovingPlatformState],
    ecs_overlay: &FeatureEcsWorldOverlay,
) -> ae::World {
    let mut collision_world =
        crate::world::platforms::world_with_moving_platforms(world, platforms);
    collision_world
        .blocks
        .extend(ecs_overlay.blocks.iter().cloned());
    // Carve portal apertures out of the host surface so a body can sink into a
    // portal (the "feet in, feet out" transit). Only the solid host kinds are
    // carved; the portal rim and surrounding geometry stay solid.
    if !ecs_overlay.portal_carves.is_empty() {
        carve_portal_apertures(&mut collision_world.blocks, &ecs_overlay.portal_carves);
    }
    collision_world
}

/// The room world with ONLY the portal apertures carved out — no moving-platform
/// or ECS-overlay solids added. Projectiles historically collided against the raw
/// room world (they pass through moving platforms); this preserves that exactly
/// while letting a shot sink into a portal opening and transit.
///
/// Returns `Cow::Borrowed(world)` when there are no active carves — the common
/// case (no body in a portal opening, or no portals at all) — so the per-frame
/// projectile steps don't clone the whole block list every frame for nothing.
pub fn world_with_portal_carves<'w>(
    world: &'w ae::World,
    portal_carves: &[ae::Aabb],
) -> std::borrow::Cow<'w, ae::World> {
    if portal_carves.is_empty() {
        return std::borrow::Cow::Borrowed(world);
    }
    let mut carved = world.clone();
    carve_portal_apertures(&mut carved.blocks, portal_carves);
    std::borrow::Cow::Owned(carved)
}

/// Split every solid host block by the portal aperture holes, leaving a doorway
/// in the surface (and a solid frame around it). Non-host kinds (hazard, pogo,
/// rebound) pass through untouched.
fn carve_portal_apertures(blocks: &mut Vec<ae::Block>, holes: &[ae::Aabb]) {
    let original = std::mem::take(blocks);
    for block in original {
        let carvable = matches!(
            block.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. } | ae::BlockKind::OneWay
        );
        if !carvable {
            blocks.push(block);
            continue;
        }
        // Subtract each hole in turn; a block can be split by more than one
        // portal (rare, but cheap to handle).
        let mut pieces = vec![block.aabb];
        for hole in holes {
            let mut next = Vec::with_capacity(pieces.len());
            for piece in pieces.drain(..) {
                crate::portal::pieces::subtract_aabb(piece, *hole, &mut next);
            }
            pieces = next;
        }
        for aabb in pieces {
            blocks.push(ae::Block {
                name: block.name.clone(),
                aabb,
                kind: block.kind,
            });
        }
    }
}
