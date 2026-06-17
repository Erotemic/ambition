//! Generic solid-world raycasting for platformer mechanics.
//!
//! Geometry queries useful to many mechanics (blink / dive / grapple pathing,
//! portal placement). The world access is captured by the narrow
//! [`SolidWorldQuery`] trait so the raycast logic is content-free: the host
//! decides which blocks count as solid (and provides the
//! `impl SolidWorldQuery for engine_core::World` adapter), while the raycast
//! just consumes their AABBs.

use bevy::prelude::*;

use ambition_engine_core as ae;

/// The minimal world access [`raycast_solids`] needs: a way to visit every solid
/// AABB the ray could hit.
///
/// `raycast_solids` only ever reads, per block, whether the block is hittable
/// (given the `include_one_way` policy) and its [`ae::Aabb`]. This trait
/// captures exactly that — the world decides which blocks count as solid; the
/// raycast just consumes their AABBs. The host (`ambition_gameplay_core`) supplies the
/// `impl SolidWorldQuery for ae::World` adapter, keeping this module
/// content-free.
pub trait SolidWorldQuery {
    /// Invoke `visit` once for each solid AABB the ray should test.
    ///
    /// When `include_one_way` is true, one-way platforms are visited too;
    /// otherwise they are skipped (blink/dive/grapple pass through them, while
    /// portal placement adheres to them).
    fn for_each_solid_aabb(&self, include_one_way: bool, visit: &mut dyn FnMut(ae::Aabb));
}

/// The engine_core world's solid-block policy: `Solid` and `BlinkWall` blocks
/// are always hittable; one-way platforms only when `include_one_way` is set
/// (portal placement adheres to them; blink/dive/grapple pass through).
///
/// `World` and `BlockKind` are both content-free foundation types
/// (`ambition_engine_core`), so this adapter is sandbox-free and lives in-crate
/// with the trait. (The orphan rule precludes implementing this foreign trait
/// for the foreign `World` type sandbox-side anyway.)
impl SolidWorldQuery for ae::World {
    fn for_each_solid_aabb(&self, include_one_way: bool, visit: &mut dyn FnMut(ae::Aabb)) {
        for block in &self.blocks {
            let hittable = matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
            ) || (include_one_way && matches!(block.kind, ae::BlockKind::OneWay));
            if hittable {
                visit(block.aabb);
            }
        }
    }
}

/// Nearest solid surface hit by a ray from `origin` along `dir`.
///
/// Returns the hit point and the outward face normal (pointing back toward the
/// ray). `include_one_way` is opt-in because blink/dive/grapple pathing can pass
/// through one-way platforms, while portal placement should adhere to them.
pub fn raycast_solids<W: SolidWorldQuery + ?Sized>(
    world: &W,
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
    world.for_each_solid_aabb(include_one_way, &mut |aabb| {
        if let Some((t, n)) = ray_aabb(origin, dir, aabb) {
            if t < best_t {
                best_t = t;
                best_normal = n;
            }
        }
    });
    if best_normal == Vec2::ZERO {
        None
    } else {
        Some((origin + dir * best_t, best_normal))
    }
}

/// Ray-vs-AABB slab query. Returns `(t_near, face_normal)` for a forward hit
/// (`t >= 0`).
pub fn ray_aabb(origin: Vec2, dir: Vec2, aabb: ae::Aabb) -> Option<(f32, Vec2)> {
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
