//! `CombatVolume` — a hit/hurt shape that can be an axis-aligned box, a rotated
//! box (OBB), or a general convex polygon.
//!
//! The common case is and will stay [`Aabb`]: box-vs-box overlap fast-paths to
//! the existing strict [`AabbExt::strict_intersects`] (cheap separating-axis +
//! Parry tie-break, edge-touching = miss). Rotated and convex shapes route
//! through Parry's `intersection_test` — the same engine the AABB path already
//! uses — so the SAT/GJK math is reused, not reimplemented. A conservative
//! bounding [`Aabb`] gives every variant an O(1) broad-phase reject before any
//! Parry shape is built, so non-overlapping pairs never pay polygon cost.
//!
//! OBB and convex shapes are both lowered to world-space corner points and a
//! Parry [`ConvexPolygon`] with an identity pose, so we never touch the isometry
//! rotation API — the rotation lives in the corner positions.

use parry2d::{
    math::{Pose, Vector},
    query,
    shape::{ConvexPolygon, Cuboid, Shape},
};

use crate::{Aabb, AabbExt, Vec2};

/// A combat hit/hurt volume. Construct via [`CombatVolume::aabb`],
/// [`CombatVolume::obb`], or [`CombatVolume::convex`]; test overlap with
/// [`CombatVolume::intersects`]. World-space.
#[derive(Clone, Debug)]
pub enum CombatVolume {
    /// Axis-aligned box — the common, cheapest case.
    Aabb(Aabb),
    /// Box rotated `rotation` radians about `center` (CCW, screen axes).
    Obb {
        center: Vec2,
        half: Vec2,
        rotation: f32,
    },
    /// Arbitrary convex polygon (world-space points). `bounds` is the cached
    /// broad-phase AABB so we never recompute it per test.
    Convex { bounds: Aabb, points: Vec<Vec2> },
}

impl From<Aabb> for CombatVolume {
    fn from(a: Aabb) -> Self {
        CombatVolume::Aabb(a)
    }
}

impl CombatVolume {
    pub fn aabb(a: Aabb) -> Self {
        CombatVolume::Aabb(a)
    }

    pub fn obb(center: Vec2, half: Vec2, rotation: f32) -> Self {
        CombatVolume::Obb {
            center,
            half,
            rotation,
        }
    }

    /// Build a convex volume from world-space points. The points need not be
    /// pre-ordered — the Parry shape is built from their convex hull.
    pub fn convex(points: Vec<Vec2>) -> Self {
        CombatVolume::Convex {
            bounds: bounds_of_points(&points),
            points,
        }
    }

    /// Conservative axis-aligned bounds — the broad-phase box.
    pub fn bounds(&self) -> Aabb {
        match self {
            CombatVolume::Aabb(a) => *a,
            CombatVolume::Obb {
                center,
                half,
                rotation,
            } => bounds_of_points(&obb_corners(*center, *half, *rotation)),
            CombatVolume::Convex { bounds, .. } => *bounds,
        }
    }

    pub fn center(&self) -> Vec2 {
        match self {
            CombatVolume::Aabb(a) => a.center(),
            CombatVolume::Obb { center, .. } => *center,
            CombatVolume::Convex { bounds, .. } => bounds.center(),
        }
    }

    /// Translate the whole volume by `delta`.
    pub fn translated(&self, delta: Vec2) -> Self {
        match self {
            CombatVolume::Aabb(a) => CombatVolume::Aabb(a.translated(delta)),
            CombatVolume::Obb {
                center,
                half,
                rotation,
            } => CombatVolume::Obb {
                center: *center + delta,
                half: *half,
                rotation: *rotation,
            },
            CombatVolume::Convex { bounds, points } => CombatVolume::Convex {
                bounds: bounds.translated(delta),
                points: points.iter().map(|p| *p + delta).collect(),
            },
        }
    }

    /// True when this volume overlaps `other`. Box-vs-box preserves the strict
    /// platformer contract (edge-touching is NOT an overlap); any rotated/convex
    /// pair is resolved by Parry after a cheap bounds reject.
    pub fn intersects(&self, other: &CombatVolume) -> bool {
        // Broad-phase: bounding boxes must strictly overlap. Because each
        // volume is contained in its bounds, a bounds miss is a true miss, and
        // this keeps the touching-is-not-overlap contract for the box case.
        if !self.bounds().strict_intersects(other.bounds()) {
            return false;
        }
        // Box vs box: exact existing semantics, no Parry polygon machinery.
        if let (CombatVolume::Aabb(a), CombatVolume::Aabb(b)) = (self, other) {
            return a.strict_intersects(*b);
        }
        // Narrow-phase via Parry for anything rotated/convex.
        let (lhs, lhs_pose) = self.parry_shape();
        let (rhs, rhs_pose) = other.parry_shape();
        query::intersection_test(&lhs_pose, lhs.as_shape(), &rhs_pose, rhs.as_shape())
            .unwrap_or(true)
    }

    /// Lower to a Parry shape + pose. AABB → translated `Cuboid`; OBB/convex →
    /// `ConvexPolygon` of world corner points with an identity pose (the
    /// rotation is baked into the points). A degenerate convex hull falls back
    /// to the bounds box so a test never silently drops.
    fn parry_shape(&self) -> (ParryShape, Pose) {
        match self {
            CombatVolume::Aabb(a) => {
                let h = a.half_size();
                let c = a.center();
                (
                    ParryShape::Cuboid(Cuboid::new(pv(h.x.max(0.0), h.y.max(0.0)))),
                    Pose::translation(c.x, c.y),
                )
            }
            CombatVolume::Obb {
                center,
                half,
                rotation,
            } => convex_shape(&obb_corners(*center, *half, *rotation), self.bounds()),
            CombatVolume::Convex { points, bounds } => convex_shape(points, *bounds),
        }
    }
}

/// Owns a Parry shape so a borrow can be handed to `intersection_test`.
enum ParryShape {
    Cuboid(Cuboid),
    Convex(ConvexPolygon),
}

impl ParryShape {
    fn as_shape(&self) -> &dyn Shape {
        match self {
            ParryShape::Cuboid(c) => c,
            ParryShape::Convex(p) => p,
        }
    }
}

/// Build a convex Parry shape from world points (identity pose). Falls back to a
/// `Cuboid` over `bounds` when the hull is degenerate (< 3 distinct points).
fn convex_shape(points: &[Vec2], bounds: Aabb) -> (ParryShape, Pose) {
    let parry_points: Vec<Vector> = points.iter().map(|p| pv(p.x, p.y)).collect();
    if let Some(poly) = ConvexPolygon::from_convex_hull(&parry_points) {
        (ParryShape::Convex(poly), Pose::translation(0.0, 0.0))
    } else {
        let h = bounds.half_size();
        let c = bounds.center();
        (
            ParryShape::Cuboid(Cuboid::new(pv(h.x.max(0.0), h.y.max(0.0)))),
            Pose::translation(c.x, c.y),
        )
    }
}

fn obb_corners(center: Vec2, half: Vec2, rotation: f32) -> Vec<Vec2> {
    let (sin, cos) = rotation.sin_cos();
    // Local corner offsets rotated into world space.
    [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        Vec2::new(half.x, half.y),
        Vec2::new(-half.x, half.y),
    ]
    .iter()
    .map(|o| center + Vec2::new(o.x * cos - o.y * sin, o.x * sin + o.y * cos))
    .collect()
}

fn bounds_of_points(points: &[Vec2]) -> Aabb {
    if points.is_empty() {
        return Aabb::new(Vec2::ZERO, Vec2::ZERO);
    }
    let mut min = points[0];
    let mut max = points[0];
    for p in &points[1..] {
        min = min.min(*p);
        max = max.max(*p);
    }
    Aabb {
        min,
        max,
    }
}

fn pv(x: f32, y: f32) -> Vector {
    Vector::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aabb(cx: f32, cy: f32, hx: f32, hy: f32) -> Aabb {
        Aabb::new(Vec2::new(cx, cy), Vec2::new(hx, hy))
    }

    #[test]
    fn box_vs_box_matches_strict_intersects() {
        let a = aabb(0.0, 0.0, 10.0, 10.0);
        let b = aabb(15.0, 0.0, 10.0, 10.0); // overlaps
        let c = aabb(30.0, 0.0, 5.0, 5.0); // disjoint
        assert!(CombatVolume::from(a).intersects(&b.into()));
        assert!(!CombatVolume::from(a).intersects(&c.into()));
        // Edge-touching is NOT an overlap (platformer contract).
        let touching = aabb(20.0, 0.0, 10.0, 10.0); // a.right()=10, touching.left()=10
        assert!(!CombatVolume::from(a).intersects(&touching.into()));
    }

    #[test]
    fn rotated_box_overlaps_a_corner_an_axis_box_misses() {
        // A 45°-rotated box whose diagonal pokes into an axis box that the
        // rotated box's own AABB would also overlap — but here we check a case
        // where rotation matters: the OBB corner reaches a point the unrotated
        // footprint shares, confirming the polygon path runs.
        let obb = CombatVolume::obb(Vec2::new(0.0, 0.0), Vec2::new(10.0, 2.0), std::f32::consts::FRAC_PI_4);
        // Box near the rotated box's far diagonal tip (~ (8.5, 8.5)).
        let near_tip = CombatVolume::from(aabb(8.0, 8.0, 1.5, 1.5));
        assert!(obb.intersects(&near_tip), "rotated box's diagonal should reach the tip box");
        // Same box position but the UNROTATED footprint (half 10x2) would not
        // reach (8,8): confirm the rotation is what made the hit.
        let flat = CombatVolume::obb(Vec2::new(0.0, 0.0), Vec2::new(10.0, 2.0), 0.0);
        assert!(!flat.intersects(&near_tip), "flat box must not reach the tip");
    }

    #[test]
    fn convex_triangle_overlap() {
        let tri = CombatVolume::convex(vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(0.0, 20.0),
        ]);
        assert!(tri.intersects(&aabb(2.0, 2.0, 2.0, 2.0).into()));
        // Far corner of the bounding box that the triangle doesn't cover.
        assert!(!tri.intersects(&aabb(18.0, 18.0, 1.0, 1.0).into()));
    }

    #[test]
    fn broad_phase_rejects_distant_shapes() {
        let obb = CombatVolume::obb(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0), 0.6);
        let far = CombatVolume::from(aabb(1000.0, 1000.0, 5.0, 5.0));
        assert!(!obb.intersects(&far));
    }
}
