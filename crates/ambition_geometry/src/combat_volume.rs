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
    shape::{Ball, ConvexPolygon, Cuboid, Shape},
};

use crate::{Aabb, AabbExt, AccelerationFrame, Vec2};

/// A combat hit/hurt volume. Construct via [`CombatVolume::aabb`],
/// [`CombatVolume::obb`], or [`CombatVolume::convex`]; test overlap with
/// [`CombatVolume::intersects`]. World-space.
#[derive(Clone, Debug, PartialEq)]
pub enum CombatVolume {
    /// Axis-aligned box — the common, cheapest case.
    Aabb(Aabb),
    /// Box rotated `rotation` radians about `center` (CCW, screen axes).
    Obb {
        center: Vec2,
        half: Vec2,
        rotation: f32,
    },
    /// Circle / disc — first-class (Parry `Ball`), exact and cheap. The natural
    /// shape for explosions and radial AoE.
    Circle { center: Vec2, radius: f32 },
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

    pub fn circle(center: Vec2, radius: f32) -> Self {
        CombatVolume::Circle {
            center,
            radius: radius.max(0.0),
        }
    }

    /// Place a BODY-LOCAL volume into the world.
    ///
    /// Body-local means what [`crate::volume_shape::VolumeShape::place_at`]
    /// means: origin at the body centre, `+x` toward the body's committed
    /// facing, `+y` toward its feet. This is the same transform, for a volume
    /// that already carries its own offset — an authored blade whose shape and
    /// position are both in the manifest, where `place_at`'s shapes are
    /// centred on the origin by construction.
    ///
    /// Mirroring belongs HERE and only here: the volume arrives with the art's
    /// handedness already resolved (see `SheetRecord::art_forward_x`), so
    /// `facing` is the only mirror left to apply. A caller that mirrors before
    /// calling this applies it twice.
    pub fn place_body_local(&self, origin: Vec2, facing: f32, frame_down: Vec2) -> Self {
        let frame = AccelerationFrame::new(frame_down);
        let face = if facing < 0.0 { -1.0 } else { 1.0 };
        let theta = frame.side.y.atan2(frame.side.x);
        let to_world = |local: Vec2| origin + frame.side * (local.x * face) + frame.down * local.y;
        match self {
            CombatVolume::Aabb(a) => {
                let (center, half) = (to_world(a.center()), a.half_size());
                if theta.abs() < 1.0e-5 {
                    CombatVolume::Aabb(Aabb::new(center, half))
                } else {
                    CombatVolume::obb(center, half, theta)
                }
            }
            CombatVolume::Obb {
                center,
                half,
                rotation,
            } => CombatVolume::obb(to_world(*center), *half, theta + rotation * face),
            CombatVolume::Circle { center, radius } => {
                CombatVolume::circle(to_world(*center), *radius)
            }
            CombatVolume::Convex { points, .. } => {
                CombatVolume::convex(points.iter().map(|p| to_world(*p)).collect())
            }
        }
    }

    /// Build a convex volume from world-space points. The points need not be
    /// pre-ordered — the Parry shape is built from their convex hull.
    /// The region an axis-aligned box covers while travelling from `from` to
    /// `to` — the exact swept shape, as a convex hull of the box at both ends.
    ///
    /// ⭐ EXACT, WHICH IS THE POINT. The cheap alternative is the UNION of the
    /// two bounding boxes, and that is right only for axis-aligned travel: on a
    /// diagonal it covers corners the box never passed through, so a swept test
    /// built on it invents hits on bodies the mover visually missed. A hull of
    /// the eight corners is the same cost to test and answers what actually
    /// happened.
    ///
    /// ⚠ A STRAIGHT leg. Curved motion must be split into segments by the
    /// caller; sweeping an arc as one hull would cover the inside of the curve.
    pub fn swept_aabb(from: Vec2, to: Vec2, half: Vec2) -> Self {
        let corners = |c: Vec2| {
            [
                Vec2::new(c.x - half.x, c.y - half.y),
                Vec2::new(c.x + half.x, c.y - half.y),
                Vec2::new(c.x + half.x, c.y + half.y),
                Vec2::new(c.x - half.x, c.y + half.y),
            ]
        };
        let mut points = Vec::with_capacity(8);
        points.extend_from_slice(&corners(from));
        points.extend_from_slice(&corners(to));
        Self::convex(convex_hull(&points))
    }

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
            CombatVolume::Circle { center, radius } => Aabb::new(*center, Vec2::splat(*radius)),
            CombatVolume::Convex { bounds, .. } => *bounds,
        }
    }

    pub fn center(&self) -> Vec2 {
        match self {
            CombatVolume::Aabb(a) => a.center(),
            CombatVolume::Obb { center, .. } => *center,
            CombatVolume::Circle { center, .. } => *center,
            CombatVolume::Convex { bounds, .. } => bounds.center(),
        }
    }

    /// Reflect the volume across the vertical line `axis_x`, leaving its size
    /// unchanged.
    ///
    /// A sprite that mirrors to face the other way takes its authored hit and
    /// hurt geometry with it. For a box that is a centre flip; for a hull every
    /// point moves, which is exactly why this belongs here rather than being
    /// re-derived by each consumer that thought it only had boxes.
    pub fn mirrored_x(&self, axis_x: f32) -> Self {
        let flip = |x: f32| 2.0 * axis_x - x;
        match self {
            CombatVolume::Aabb(a) => {
                let (c, h) = (a.center(), a.half_size());
                CombatVolume::Aabb(Aabb::new(Vec2::new(flip(c.x), c.y), h))
            }
            CombatVolume::Obb {
                center,
                half,
                rotation,
            } => CombatVolume::Obb {
                center: Vec2::new(flip(center.x), center.y),
                half: *half,
                // Mirroring negates the sense of rotation, not its magnitude.
                rotation: -*rotation,
            },
            CombatVolume::Circle { center, radius } => CombatVolume::Circle {
                center: Vec2::new(flip(center.x), center.y),
                radius: *radius,
            },
            CombatVolume::Convex { points, .. } => {
                CombatVolume::convex(points.iter().map(|p| Vec2::new(flip(p.x), p.y)).collect())
            }
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
            CombatVolume::Circle { center, radius } => CombatVolume::Circle {
                center: *center + delta,
                radius: *radius,
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

    /// Convenience: overlap against a plain [`Aabb`] target (the common case —
    /// most hurtboxes are still boxes).
    pub fn intersects_aabb(&self, other: Aabb) -> bool {
        match self {
            // Fast path: box-vs-box keeps the exact strict semantics with no
            // wrapping allocation.
            CombatVolume::Aabb(a) => a.strict_intersects(other),
            _ => self.intersects(&CombatVolume::Aabb(other)),
        }
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
            CombatVolume::Circle { center, radius } => (
                ParryShape::Ball(Ball::new(radius.max(0.0))),
                Pose::translation(center.x, center.y),
            ),
            CombatVolume::Convex { points, bounds } => convex_shape(points, *bounds),
        }
    }
}

/// Owns a Parry shape so a borrow can be handed to `intersection_test`.
enum ParryShape {
    Cuboid(Cuboid),
    Ball(Ball),
    Convex(ConvexPolygon),
}

impl ParryShape {
    fn as_shape(&self) -> &dyn Shape {
        match self {
            ParryShape::Cuboid(c) => c,
            ParryShape::Ball(b) => b,
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

pub(crate) fn obb_corners(center: Vec2, half: Vec2, rotation: f32) -> Vec<Vec2> {
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

/// The convex hull of `points`, counter-clockwise, duplicates dropped.
///
/// Monotone chain. Exists because [`CombatVolume::convex`] stores the points it
/// is GIVEN — it does not hull them — so a caller assembling a shape from raw
/// corners owes the hull, and every overlap test downstream assumes convexity.
///
/// Degenerate input answers honestly: fewer than three distinct points cannot
/// bound an area, so they come back as-is rather than as a polygon nothing can
/// intersect.
pub fn convex_hull(points: &[Vec2]) -> Vec<Vec2> {
    let mut sorted: Vec<Vec2> = points.to_vec();
    sorted.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    sorted.dedup_by(|a, b| a.x == b.x && a.y == b.y);
    if sorted.len() < 3 {
        return sorted;
    }
    // > 0 is a left turn; collinear points are dropped, which keeps the hull
    // minimal and the downstream SAT loops shorter.
    let cross = |o: Vec2, a: Vec2, b: Vec2| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut hull: Vec<Vec2> = Vec::with_capacity(sorted.len() * 2);
    for &p in sorted.iter() {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(p);
    }
    let lower = hull.len() + 1;
    for &p in sorted.iter().rev() {
        while hull.len() >= lower && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(p);
    }
    hull.pop();
    hull
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
    Aabb { min, max }
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
        let obb = CombatVolume::obb(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 2.0),
            std::f32::consts::FRAC_PI_4,
        );
        // Box near the rotated box's far diagonal tip (~ (8.5, 8.5)).
        let near_tip = CombatVolume::from(aabb(8.0, 8.0, 1.5, 1.5));
        assert!(
            obb.intersects(&near_tip),
            "rotated box's diagonal should reach the tip box"
        );
        // Same box position but the UNROTATED footprint (half 10x2) would not
        // reach (8,8): confirm the rotation is what made the hit.
        let flat = CombatVolume::obb(Vec2::new(0.0, 0.0), Vec2::new(10.0, 2.0), 0.0);
        assert!(
            !flat.intersects(&near_tip),
            "flat box must not reach the tip"
        );
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
    fn circle_overlap_is_radial_not_boxy() {
        let circle = CombatVolume::circle(Vec2::new(0.0, 0.0), 10.0);
        // A box overlapping the disc near its edge along an axis: hit.
        assert!(circle.intersects(&aabb(9.0, 0.0, 1.0, 1.0).into()));
        // A box in the bounding-box CORNER the disc doesn't reach (~(8.5,8.5) is
        // outside r=10? dist=12.0>10 → miss), proving it's a disc not its bbox.
        assert!(!circle.intersects(&aabb(8.5, 8.5, 0.2, 0.2).into()));
        // Circle vs circle.
        assert!(circle.intersects(&CombatVolume::circle(Vec2::new(18.0, 0.0), 10.0)));
        assert!(!circle.intersects(&CombatVolume::circle(Vec2::new(30.0, 0.0), 5.0)));
    }

    #[test]
    fn broad_phase_rejects_distant_shapes() {
        let obb = CombatVolume::obb(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0), 0.6);
        let far = CombatVolume::from(aabb(1000.0, 1000.0, 5.0, 5.0));
        assert!(!obb.intersects(&far));
    }

    /// The hull of a square's own corners is the square — four points, no more.
    #[test]
    fn a_hull_of_a_square_is_the_square() {
        let pts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ];
        assert_eq!(super::convex_hull(&pts).len(), 4);
    }

    /// Interior and duplicate points are dropped: a hull is the boundary, and
    /// carrying redundant vertices lengthens every SAT loop downstream.
    #[test]
    fn a_hull_drops_interior_duplicate_and_collinear_points() {
        let pts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
            Vec2::new(1.0, 1.0), // interior
            Vec2::new(2.0, 0.0), // duplicate
            Vec2::new(1.0, 0.0), // collinear on the bottom edge
        ];
        assert_eq!(super::convex_hull(&pts).len(), 4);
    }

    /// ⭐ THE CLAIM `swept_aabb` EXISTS FOR: it is EXACT on a diagonal, where the
    /// cheap union-of-boxes is not.
    ///
    /// A box travelling up-right never visits the bottom-right corner of the
    /// union rectangle. A swept test built on that union would report a hit
    /// against a body sitting there — a body the mover visually missed.
    #[test]
    fn a_diagonal_sweep_excludes_the_corner_the_union_would_include() {
        let half = Vec2::new(1.0, 1.0);
        let swept = CombatVolume::swept_aabb(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0), half);
        // Well inside the union rectangle (x
        let off_path = CombatVolume::from(aabb(9.0, 1.0, 0.4, 0.4));
        assert!(
            !swept.intersects(&off_path),
            "a diagonal sweep must not cover the corner the union rectangle would"
        );
        // And it does cover the path itself.
        let on_path = CombatVolume::from(aabb(5.0, 5.0, 0.4, 0.4));
        assert!(
            swept.intersects(&on_path),
            "the sweep must cover its own path"
        );
    }

    /// Both ends belong to the swept region, not just the middle.
    #[test]
    fn a_sweep_covers_both_of_its_endpoints() {
        let half = Vec2::new(2.0, 2.0);
        let swept = CombatVolume::swept_aabb(Vec2::new(0.0, 0.0), Vec2::new(50.0, 0.0), half);
        assert!(swept.intersects(&CombatVolume::from(aabb(0.0, 0.0, 0.5, 0.5))));
        assert!(swept.intersects(&CombatVolume::from(aabb(50.0, 0.0, 0.5, 0.5))));
        assert!(!swept.intersects(&CombatVolume::from(aabb(60.0, 0.0, 0.5, 0.5))));
    }

    /// A zero-length sweep is just the box: the degenerate case a fast-path
    /// caller hands in when nothing moved.
    #[test]
    fn a_zero_length_sweep_is_the_box_itself() {
        let half = Vec2::new(3.0, 3.0);
        let swept = CombatVolume::swept_aabb(Vec2::new(7.0, 7.0), Vec2::new(7.0, 7.0), half);
        assert!(swept.intersects(&CombatVolume::from(aabb(7.0, 7.0, 0.5, 0.5))));
        assert!(!swept.intersects(&CombatVolume::from(aabb(20.0, 7.0, 0.5, 0.5))));
    }
}
