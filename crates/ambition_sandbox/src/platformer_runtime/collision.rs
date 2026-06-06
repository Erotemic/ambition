//! Generic collision and world-query helpers for platformer mechanics.
//!
//! This module is the proto-runtime home for geometry queries that are useful to
//! many mechanics. Portal-specific traversal still belongs to the portal mechanic, but
//! plain solid raycasts should not require non-portal abilities to depend on that
//! mechanic.

use bevy::prelude::*;

use crate::engine_core as ae;

/// Nearest solid surface hit by a ray from `origin` along `dir`.
///
/// Returns the hit point and the outward face normal (pointing back toward the
/// ray). `include_one_way` is opt-in because blink/dive/grapple pathing can pass
/// through one-way platforms, while portal placement should adhere to them.
pub fn raycast_solids(
    world: &ae::World,
    origin: Vec2,
    dir: Vec2,
    max_dist: f32,
    include_one_way: bool,
) -> Option<(Vec2, Vec2)> {
    let dir = dir.normalize_or_zero();
    if dir == Vec2::ZERO {
        return None;
    }
    let mut best_t = max_dist;
    let mut best_normal = Vec2::ZERO;
    for block in &world.blocks {
        let hittable = matches!(
            block.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
        ) || (include_one_way && matches!(block.kind, ae::BlockKind::OneWay));
        if !hittable {
            continue;
        }
        if let Some((t, n)) = ray_aabb(origin, dir, block.aabb) {
            if t < best_t {
                best_t = t;
                best_normal = n;
            }
        }
    }
    if best_normal == Vec2::ZERO {
        None
    } else {
        Some((origin + dir * best_t, best_normal))
    }
}

/// Ray-vs-AABB slab query. Returns `(t_near, face_normal)` for a forward hit
/// (`t >= 0`).
pub(crate) fn ray_aabb(origin: Vec2, dir: Vec2, aabb: ae::Aabb) -> Option<(f32, Vec2)> {
    // 1/0 -> +/-inf is the intended slab-method behavior for axis-parallel rays.
    let inv = Vec2::new(1.0 / dir.x, 1.0 / dir.y);
    let tx1 = (aabb.min.x - origin.x) * inv.x;
    let tx2 = (aabb.max.x - origin.x) * inv.x;
    let ty1 = (aabb.min.y - origin.y) * inv.y;
    let ty2 = (aabb.max.y - origin.y) * inv.y;
    let tminx = tx1.min(tx2);
    let tmaxx = tx1.max(tx2);
    let tminy = ty1.min(ty2);
    let tmaxy = ty1.max(ty2);
    let t_near = tminx.max(tminy);
    let t_far = tmaxx.min(tmaxy);
    if t_near > t_far || t_far < 0.0 {
        return None;
    }
    // The axis that produced t_near is the face we hit; its normal opposes
    // the ray's travel on that axis.
    let normal = if tminx > tminy {
        Vec2::new(-dir.x.signum(), 0.0)
    } else {
        Vec2::new(0.0, -dir.y.signum())
    };
    Some((t_near.max(0.0), normal))
}
