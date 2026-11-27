//! Bevy-native geometry helpers.
//!
//! Ambition uses Bevy's `Aabb2d` as its public rectangle collision primitive.
//! This module adds only Ambition-specific semantics on top: center/half
//! helpers, strict platformer overlap (edge-touching is not overlap), and
//! swept-box queries.

use bevy_math::bounding::Aabb2d;
use parry2d::{
    math::{Pose, Vector},
    query::{self, ShapeCastOptions},
    shape::Cuboid,
};

use crate::Vec2;

const CONTACT_EPS: f32 = 1.0e-4;

/// Public engine AABB type.
///
/// Bevy's 2D bounding box, re-exported so callers can import `ae::Aabb`
/// (`ae` is the usual alias for `ambition_platformer2d_core`).
pub type Aabb = Aabb2d;

/// A point with a nonfinite coordinate prevents a finite bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonFinitePoint;

/// Return the coordinate limits of the points. An empty set has no box.
/// A point or line can have a box with zero width or height.
pub fn point_bounds(points: impl IntoIterator<Item = Vec2>) -> Result<Option<Aabb>, NonFinitePoint> {
    let mut points = points.into_iter();
    let Some(first) = points.next() else {
        return Ok(None);
    };
    if !first.is_finite() {
        return Err(NonFinitePoint);
    }
    let mut bounds = Aabb {
        min: first,
        max: first,
    };
    for point in points {
        if !point.is_finite() {
            return Err(NonFinitePoint);
        }
        bounds.min = bounds.min.min(point);
        bounds.max = bounds.max.max(point);
    }
    Ok(Some(bounds))
}

/// Construct an AABB from a minimum corner and a size.
pub fn aabb_from_min_size(min: Vec2, size: Vec2) -> Aabb {
    Aabb::new(min + size * 0.5, size * 0.5)
}

/// Construct an AABB from explicit min/max edges. Degenerate (empty) → `None`.
pub fn aabb_from_min_max(x0: f32, y0: f32, x1: f32, y1: f32) -> Option<Aabb> {
    if x0 >= x1 || y0 >= y1 {
        return None;
    }
    Some(Aabb::new(
        Vec2::new((x0 + x1) * 0.5, (y0 + y1) * 0.5),
        Vec2::new((x1 - x0) * 0.5, (y1 - y0) * 0.5),
    ))
}

/// Set difference of two axis-aligned rectangles: `block` minus `hole`, pushed
/// into `out` as up to four sub-rectangles (the frame around the hole). If
/// they do not overlap, `block` is pushed unchanged; if `hole` covers `block`,
/// nothing is pushed.
///
/// A portal uses this to carve a doorway in its host surface. It is plain
/// rectangle algebra, so it lives here, and `ambition_platformer2d_world` can
/// build a carved collision world without depending on `ambition_portal2d`.
pub fn subtract_aabb(block: Aabb, hole: Aabb, out: &mut Vec<Aabb>) {
    let (bx0, by0, bx1, by1) = (block.min.x, block.min.y, block.max.x, block.max.y);
    // Clamp the hole to the block.
    let hx0 = hole.min.x.max(bx0);
    let hy0 = hole.min.y.max(by0);
    let hx1 = hole.max.x.min(bx1);
    let hy1 = hole.max.y.min(by1);
    if hx0 >= hx1 || hy0 >= hy1 {
        // No real overlap: keep the block whole.
        out.push(block);
        return;
    }
    // Up to four rectangles around the hole (below, above, left-middle,
    // right-middle). `aabb_from_min_max` drops any that are empty.
    out.extend(aabb_from_min_max(bx0, by0, bx1, hy0)); // below the hole
    out.extend(aabb_from_min_max(bx0, hy1, bx1, by1)); // above the hole
    out.extend(aabb_from_min_max(bx0, hy0, hx0, hy1)); // left of the hole
    out.extend(aabb_from_min_max(hx1, hy0, bx1, hy1)); // right of the hole
}

#[cfg(test)]
mod subtract_aabb_tests {
    use super::*;

    #[test]
    fn subtract_carves_a_doorway_leaving_a_frame() {
        // A wide floor block, carve a hole in the middle.
        let block = Aabb::new(Vec2::new(100.0, 300.0), Vec2::new(100.0, 10.0));
        let hole = Aabb::new(Vec2::new(100.0, 300.0), Vec2::new(30.0, 30.0));
        let mut out = Vec::new();
        subtract_aabb(block, hole, &mut out);
        // Left + right segments remain; the middle is open.
        assert_eq!(out.len(), 2, "left + right frame: {out:?}");
        // The opening (x in [70,130]) is not covered by any remaining piece.
        for piece in &out {
            assert!(
                piece.min.x >= 130.0 - 1e-3 || piece.max.x <= 70.0 + 1e-3,
                "piece spans hole: {piece:?}"
            );
        }
    }

    #[test]
    fn subtract_no_overlap_keeps_block() {
        let block = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let hole = Aabb::new(Vec2::new(100.0, 100.0), Vec2::new(5.0, 5.0));
        let mut out = Vec::new();
        subtract_aabb(block, hole, &mut out);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn subtract_a_covering_hole_leaves_nothing() {
        let block = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let hole = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(20.0, 20.0));
        let mut out = Vec::new();
        subtract_aabb(block, hole, &mut out);
        assert!(out.is_empty(), "a hole that covers the block erases it");
    }
}

/// Canonical mutable axis-aligned box, stored as `center` + `half_size`.
///
/// [`Aabb`] (= `Aabb2d`) is the collision-math primitive (min/max corners,
/// sweeps, strict overlap). `CenteredAabb` is its ECS storage twin: a
/// `Component` you move by writing `center` and resize by writing
/// `half_size`. Convert with [`CenteredAabb::aabb`] and
/// [`CenteredAabb::from_aabb`].
///
/// The canonical center+half box for entities that own a footprint (feature
/// geometry, pickups, triggers). Mirrors the `Aabb::new(center, half)`
/// convention.
#[derive(
    bevy_ecs::component::Component,
    Clone,
    Copy,
    Debug,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct CenteredAabb {
    pub center: Vec2,
    pub half_size: Vec2,
}

impl CenteredAabb {
    pub fn new(center: Vec2, half_size: Vec2) -> Self {
        Self { center, half_size }
    }

    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        Self {
            center,
            half_size: size * 0.5,
        }
    }

    pub fn from_aabb(aabb: Aabb) -> Self {
        Self {
            center: aabb.center(),
            half_size: aabb.half_size(),
        }
    }

    /// Full extent (`half_size * 2`).
    pub fn size(self) -> Vec2 {
        self.half_size * 2.0
    }

    /// The collision-math view (`Aabb2d`).
    pub fn aabb(self) -> Aabb {
        Aabb::new(self.center, self.half_size)
    }
}

/// Result of sweeping an AABB by a normalized frame delta.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AabbSweepHit {
    /// Normalized time along the requested delta, in `[0, 1]`.
    pub time_of_impact: f32,
    /// Outward contact normal reported for the moving shape.
    pub normal1: Vec2,
}

/// Ambition-specific helpers layered on Bevy's `Aabb2d`.
///
/// Bevy's bounding-volume traits use general geometry semantics. Ambition
/// needs stricter platformer semantics in places, mainly that edge-touching
/// boxes do not overlap.
pub trait AabbExt {
    fn center(self) -> Vec2;
    fn half_size(self) -> Vec2;
    fn width(self) -> f32;
    fn height(self) -> f32;
    fn top(self) -> f32;
    fn bottom(self) -> f32;
    fn left(self) -> f32;
    fn right(self) -> f32;
    fn translated(self, delta: Vec2) -> Self;
    fn strict_intersects(self, rhs: Self) -> bool;
    fn sweep_hit(self, delta: Vec2, rhs: Self) -> Option<AabbSweepHit>;

    // --- Gravity-relative edges (the one source of truth for "feet") ---
    //
    // Gravity is cardinal (down `(0,1)`, up `(0,-1)`, wall `(±1,0)`). A body's
    // feet are the AABB face in the +gravity direction; its head is the
    // opposite face. Grounding, resize, one-way, and sprite-anchor code reads
    // these instead of `.bottom()`, so all of it flips together with gravity.
    // Under down-gravity `feet == bottom` and `head == top`.

    /// Half-extent along the gravity axis (`half_size.y` for vertical gravity,
    /// `half_size.x` for wall-walking).
    fn gravity_half(self, gravity_dir: Vec2) -> f32
    where
        Self: Sized,
    {
        let h = self.half_size();
        h.x * gravity_dir.x.abs() + h.y * gravity_dir.y.abs()
    }

    /// The feet face center: the face a falling body lands on (+gravity side).
    /// Bottom-center under down-gravity, top-center under up.
    fn feet(self, gravity_dir: Vec2) -> Vec2
    where
        Self: Sized + Copy,
    {
        self.center() + gravity_dir * self.gravity_half(gravity_dir)
    }

    /// The head face center, opposite gravity.
    fn head(self, gravity_dir: Vec2) -> Vec2
    where
        Self: Sized + Copy,
    {
        self.center() - gravity_dir * self.gravity_half(gravity_dir)
    }

    /// The feet edge projected onto the gravity axis. A body rests on a surface
    /// when its `feet_coord` meets the surface's `head_coord`.
    fn feet_coord(self, gravity_dir: Vec2) -> f32
    where
        Self: Sized + Copy,
    {
        self.center().dot(gravity_dir) + self.gravity_half(gravity_dir)
    }

    /// The head edge projected onto the gravity axis. For a platform, the face
    /// a falling body lands on (its anti-gravity face).
    fn head_coord(self, gravity_dir: Vec2) -> f32
    where
        Self: Sized + Copy,
    {
        self.center().dot(gravity_dir) - self.gravity_half(gravity_dir)
    }
    fn sweep_time_of_impact(self, delta: Vec2, rhs: Self) -> Option<f32>
    where
        Self: Sized,
    {
        self.sweep_hit(delta, rhs).map(|hit| hit.time_of_impact)
    }
}

impl AabbExt for Aabb {
    fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    fn half_size(self) -> Vec2 {
        (self.max - self.min) * 0.5
    }

    fn width(self) -> f32 {
        self.max.x - self.min.x
    }

    fn height(self) -> f32 {
        self.max.y - self.min.y
    }

    fn top(self) -> f32 {
        self.min.y
    }

    fn bottom(self) -> f32 {
        self.max.y
    }

    fn left(self) -> f32 {
        self.min.x
    }

    fn right(self) -> f32 {
        self.max.x
    }

    fn translated(self, delta: Vec2) -> Self {
        Self {
            min: self.min + delta,
            max: self.max + delta,
        }
    }

    /// Strict overlap test backed by Parry.
    ///
    /// Edge-touching boxes do not overlap in Ambition. A cheap separating-axis
    /// guard runs before Parry's intersection test, which counts touching
    /// shapes as intersecting.
    fn strict_intersects(self, rhs: Self) -> bool {
        if self.right() <= rhs.left()
            || self.left() >= rhs.right()
            || self.bottom() <= rhs.top()
            || self.top() >= rhs.bottom()
        {
            return false;
        }

        let lhs_shape = parry_cuboid(self);
        let rhs_shape = parry_cuboid(rhs);
        query::intersection_test(&parry_pose(self), &lhs_shape, &parry_pose(rhs), &rhs_shape)
            .unwrap_or(true)
    }

    /// Return the first hit at which this box first touches `rhs` while
    /// moving by `delta`, or `None` if no hit occurs along that segment.
    ///
    /// `time_of_impact` is a fraction of `delta` (a frame or ability delta),
    /// not seconds.
    ///
    /// Parry reports some edge-touching starts as immediate contact. Ambition
    /// is stricter: resting on a floor must not block horizontal motion, and
    /// sliding along a wall must not block vertical motion. So zero-time hits
    /// are discarded when the boxes only touch and `delta` does not move into
    /// the touching face.
    fn sweep_hit(self, delta: Vec2, rhs: Self) -> Option<AabbSweepHit> {
        if delta.length_squared() <= 1.0e-8 {
            return self.strict_intersects(rhs).then_some(AabbSweepHit {
                time_of_impact: 0.0,
                normal1: Vec2::ZERO,
            });
        }

        // Closed-form fast path. Both poses are pure translations, so a
        // general GJK shape cast is not needed for box against box; the shape
        // cast was the largest cost in large-crowd profiles.
        //
        // The fast path declines a penetrating start. An overlapping pair
        // needs Parry's `compute_impact_geometry_on_penetration`
        // (minimum-translation direction). `slab_sweep` returns `None` there
        // and the Parry path below runs.
        if let Some(hit) = slab_sweep(self, delta, rhs) {
            return reject_grazing_contact(self, delta, rhs, hit);
        }
        if !starts_overlapping(self, rhs) {
            // The fast path is exact for separated boxes: no hit means no hit.
            return None;
        }
        parry_sweep(self, delta, rhs).and_then(|hit| reject_grazing_contact(self, delta, rhs, hit))
    }
}

// Test-only count of general-solver calls. Guards the routing: the
// differential test still passes if every call goes to the slow solver.
//
// Thread-local because `cargo test` runs tests in parallel and the
// differential test calls `parry_sweep` many times; a global counter would see
// other tests. Plain `//`: a doc comment on a macro invocation is an unused
// doc comment.
#[cfg(test)]
thread_local! {
    static PARRY_SWEEPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The general solver: used for a penetrating start, and the oracle the fast
/// path is tested against. Keep it. `slab_sweep` answers only for separated
/// boxes; an overlapping pair needs Parry's
/// `compute_impact_geometry_on_penetration` geometry.
fn parry_sweep(lhs: Aabb, delta: Vec2, rhs: Aabb) -> Option<AabbSweepHit> {
    #[cfg(test)]
    PARRY_SWEEPS.with(|count| count.set(count.get() + 1));
    let moving_shape = parry_cuboid(lhs);
    let static_shape = parry_cuboid(rhs);
    let options = ShapeCastOptions {
        max_time_of_impact: 1.0,
        target_distance: 0.0,
        stop_at_penetration: true,
        // Movement consumes `normal1` for t=0 contacts, so ask Parry to
        // compute reliable impact geometry when a cast begins in penetration.
        compute_impact_geometry_on_penetration: true,
    };

    query::cast_shapes(
        &parry_pose(lhs),
        to_parry_vec(delta),
        &moving_shape,
        &parry_pose(rhs),
        Vector::ZERO,
        &static_shape,
        options,
    )
    .ok()
    .flatten()
    .map(|hit| AabbSweepHit {
        time_of_impact: hit.time_of_impact.clamp(0.0, 1.0),
        normal1: Vec2::new(hit.normal1.x, hit.normal1.y),
    })
}

/// Whether the two boxes overlap enough that a sweep begins in penetration.
///
/// Not `strict_intersects`, which treats edge-touching as separate. Parry
/// treats a touching start as contact, and this decides which solver may
/// answer, so it asks Parry's question.
fn starts_overlapping(lhs: Aabb, rhs: Aabb) -> bool {
    lhs.right() >= rhs.left()
        && lhs.left() <= rhs.right()
        && lhs.bottom() >= rhs.top()
        && lhs.top() <= rhs.bottom()
}

/// The strict-touching rejection, shared by both solvers.
///
/// One copy, because this is Ambition's contract, not a property of either
/// solver: resting on a floor must not block horizontal motion, and sliding
/// along a wall must not block vertical motion. A distance epsilon must not
/// discard a positive normalized impact time.
fn reject_grazing_contact(
    lhs: Aabb,
    delta: Vec2,
    rhs: Aabb,
    hit: AabbSweepHit,
) -> Option<AabbSweepHit> {
    if hit.time_of_impact == 0.0
        && !lhs.strict_intersects(rhs)
        && !moves_into_touching_face(lhs, delta, rhs)
    {
        None
    } else {
        Some(hit)
    }
}

/// The closed parameter interval inside an axis-aligned box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AabbLineInterval {
    pub enter: f32,
    pub exit: f32,
    /// The first axis on a tie. None if no axis sets the entry.
    pub entry_axis: Option<usize>,
}

/// Intersect a closed box with `origin + t * direction`. The caller sets the
/// allowed range of `t`. A stationary axis must lie within the slab.
/// Use finite coordinates and ordered box limits.
pub fn aabb_line_interval(origin: Vec2, direction: Vec2, bounds: Aabb) -> Option<AabbLineInterval> {
    let mut interval = AabbLineInterval {
        enter: f32::NEG_INFINITY,
        exit: f32::INFINITY,
        entry_axis: None,
    };
    for axis in 0..2 {
        let (position, motion) = (origin[axis], direction[axis]);
        let (lo, hi) = (bounds.min[axis], bounds.max[axis]);
        if motion == 0.0 {
            if position < lo || position > hi {
                return None;
            }
            continue;
        }
        let inverse = 1.0 / motion;
        let time = |edge| {
            if inverse.is_finite() {
                (edge - position) * inverse
            } else {
                (edge - position) / motion
            }
        };
        let (mut near, mut far) = (time(lo), time(hi));
        if near > far {
            std::mem::swap(&mut near, &mut far);
        }
        if near > interval.enter {
            interval.enter = near;
            interval.entry_axis = Some(axis);
        }
        if far < interval.exit {
            interval.exit = far;
        }
        if interval.enter > interval.exit {
            return None;
        }
    }
    Some(interval)
}

#[cfg(test)]
mod bounds_tests {
    use super::*;

    #[test]
    fn point_bounds_distinguish_empty_invalid_and_degenerate_sets() {
        assert_eq!(point_bounds([]), Ok(None));
        assert_eq!(point_bounds([Vec2::ZERO, Vec2::NAN]), Err(NonFinitePoint));
        let min = Vec2::new(-f32::MAX, -3.0);
        let max = Vec2::new(f32::MAX, -3.0);
        assert_eq!(point_bounds([max, min]), Ok(Some(Aabb { min, max })));
        assert_eq!(point_bounds([min]), Ok(Some(Aabb { min, max: min })));
    }

    #[test]
    fn a_line_parallel_to_and_on_a_box_edge_keeps_the_closed_interval() {
        let bounds = Aabb { min: Vec2::ZERO, max: Vec2::ONE };
        let hit = aabb_line_interval(Vec2::new(0.0, -2.0), Vec2::Y, bounds).unwrap();
        assert_eq!((hit.enter, hit.exit, hit.entry_axis), (2.0, 3.0, Some(1)));
        assert!(aabb_line_interval(Vec2::new(-0.1, -2.0), Vec2::Y, bounds).is_none());
    }
}

/// Expand `rhs` by the half-extents of `lhs` and sweep its center through
/// that box. [`starts_overlapping`] identifies a penetrating start.
fn slab_sweep(lhs: Aabb, delta: Vec2, rhs: Aabb) -> Option<AabbSweepHit> {
    let half = lhs.half_size();
    let expanded = Aabb {
        min: rhs.min - half,
        max: rhs.max + half,
    };
    let sweep_component = |value: f32| if value.abs() <= 1.0e-8 { 0.0 } else { value };
    let motion = Vec2::new(sweep_component(delta.x), sweep_component(delta.y));
    let AabbLineInterval {
        enter,
        exit,
        entry_axis,
    } = aabb_line_interval(lhs.center(), motion, expanded)?;
    if !(0.0..=1.0).contains(&enter) || exit < 0.0 {
        return None;
    }
    let axis = entry_axis?;
    let sign = motion[axis].signum();
    let normal1 = if axis == 0 {
        Vec2::new(sign, 0.0)
    } else {
        Vec2::new(0.0, sign)
    };
    Some(AabbSweepHit {
        time_of_impact: enter.clamp(0.0, 1.0),
        normal1,
    })
}

fn moves_into_touching_face(lhs: Aabb, delta: Vec2, rhs: Aabb) -> bool {
    let y_ranges_overlap =
        lhs.bottom() > rhs.top() && lhs.top() < rhs.bottom();
    let x_ranges_overlap =
        lhs.right() > rhs.left() && lhs.left() < rhs.right();

    let touching_rhs_left = nearly_equal(lhs.right(), rhs.left());
    let touching_rhs_right = nearly_equal(lhs.left(), rhs.right());
    let touching_rhs_top = nearly_equal(lhs.bottom(), rhs.top());
    let touching_rhs_bottom = nearly_equal(lhs.top(), rhs.bottom());

    (touching_rhs_left && y_ranges_overlap && delta.x > 0.0)
        || (touching_rhs_right && y_ranges_overlap && delta.x < 0.0)
        || (touching_rhs_top && x_ranges_overlap && delta.y > 0.0)
        || (touching_rhs_bottom && x_ranges_overlap && delta.y < 0.0)
}

fn parry_pose(aabb: Aabb) -> Pose {
    let center = aabb.center();
    Pose::translation(center.x, center.y)
}

fn parry_cuboid(aabb: Aabb) -> Cuboid {
    let half = aabb.half_size();
    Cuboid::new(to_parry_vec(Vec2::new(half.x.max(0.0), half.y.max(0.0))))
}

fn to_parry_vec(value: Vec2) -> Vector {
    Vector::new(value.x, value.y)
}

fn nearly_equal(a: f32, b: f32) -> bool {
    (a - b).abs() <= CONTACT_EPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_min_size_matches_center_half_constructor() {
        let aabb = aabb_from_min_size(Vec2::new(10.0, 20.0), Vec2::new(30.0, 40.0));
        assert_eq!(aabb.center(), Vec2::new(25.0, 40.0));
        assert_eq!(aabb.half_size(), Vec2::new(15.0, 20.0));
    }

    #[test]
    fn a_positive_impact_time_is_not_a_grazing_contact() {
        let body = Aabb::new(Vec2::ZERO, Vec2::ONE);
        let wall = aabb_from_min_size(Vec2::new(1.0005, -2.0), Vec2::new(1.0, 4.0));
        for distance in [1.0, 10.0, 100.0] {
            let delta = Vec2::new(distance, 0.0);
            let hit = body.sweep_hit(delta, wall).expect("a separated body closes the gap to the wall");
            assert!(hit.time_of_impact > 0.0 && hit.time_of_impact < 1.0);
            let at = body.translated(delta * hit.time_of_impact);
            assert!((at.right() - wall.left()).abs() < 1e-5);
        }
    }

    #[test]
    fn a_tiny_closing_component_still_contacts_a_touching_wall() {
        let body = Aabb::new(Vec2::ZERO, Vec2::ONE);
        let wall = aabb_from_min_size(Vec2::new(1.0, -10.0), Vec2::new(1.0, 20.0));
        let hit = body.sweep_hit(Vec2::new(0.00005, 1.0), wall)
            .expect("motion into the wall remains blocked while the body moves along it");
        assert_eq!(hit.time_of_impact, 0.0);
        assert!(hit.normal1.x > 0.0);
    }

    #[test]
    fn resting_on_floor_does_not_block_horizontal_sweep() {
        let body = Aabb::new(Vec2::new(50.0, 80.0), Vec2::new(10.0, 20.0));
        let floor = Aabb::new(Vec2::new(120.0, 112.0), Vec2::new(180.0, 12.0));
        assert_eq!(body.bottom(), floor.top());
        assert_eq!(body.sweep_time_of_impact(Vec2::new(8.0, 0.0), floor), None);
    }

    #[test]
    fn moving_into_touching_wall_reports_immediate_sweep() {
        let body = Aabb::new(Vec2::new(50.0, 80.0), Vec2::new(10.0, 20.0));
        let wall = Aabb::new(Vec2::new(70.0, 80.0), Vec2::new(10.0, 80.0));
        assert_eq!(body.right(), wall.left());
        assert_eq!(
            body.sweep_time_of_impact(Vec2::new(8.0, 0.0), wall),
            Some(0.0)
        );
    }

    #[test]
    fn moving_into_touching_wall_reports_outward_moving_shape_normal() {
        let body = Aabb::new(Vec2::new(50.0, 80.0), Vec2::new(10.0, 20.0));
        let wall = Aabb::new(Vec2::new(70.0, 80.0), Vec2::new(10.0, 80.0));
        let hit = body
            .sweep_hit(Vec2::new(8.0, 0.0), wall)
            .expect("moving into a touching wall should report an immediate hit");
        assert_eq!(hit.time_of_impact, 0.0);
        assert!(
            hit.normal1.x > 0.9 && hit.normal1.y.abs() < 0.1,
            "expected the outward normal on the moving body to point right, got {:?}",
            hit.normal1
        );
    }

    #[test]
    fn aabb_translated_shifts_min_and_max() {
        let aabb = Aabb::new(Vec2::new(10.0, 10.0), Vec2::new(5.0, 5.0));
        let shifted = aabb.translated(Vec2::new(20.0, -10.0));
        assert_eq!(shifted.center(), Vec2::new(30.0, 0.0));
        assert_eq!(shifted.half_size(), aabb.half_size());
    }

    #[test]
    fn strict_intersects_rejects_edge_touching() {
        // Edge-touching AABBs do not intersect. `body_overlaps_any` relies on
        // this: a player resting on a floor must not overlap it.
        let a = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let b = Aabb::new(Vec2::new(20.0, 0.0), Vec2::new(10.0, 10.0));
        assert_eq!(a.right(), b.left());
        assert!(!a.strict_intersects(b));
    }

    #[test]
    fn strict_intersects_accepts_overlap() {
        let a = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let b = Aabb::new(Vec2::new(15.0, 0.0), Vec2::new(10.0, 10.0));
        assert!(a.strict_intersects(b));
    }

    #[test]
    fn strict_intersects_accepts_full_containment() {
        // Every hit path (`ecs_hit_event_hits_boss/_actor`, `apply_boss_hit`,
        // the projectile) uses `strict_intersects`.
        let hurtbox = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(20.0, 20.0));
        let inside = Aabb::new(Vec2::new(2.0, -3.0), Vec2::new(3.0, 3.0));
        assert!(
            inside.strict_intersects(hurtbox),
            "small box inside the hurtbox hits"
        );
        assert!(
            hurtbox.strict_intersects(inside),
            "and the check is symmetric"
        );
    }

    #[test]
    fn sweep_zero_delta_returns_intersection_state() {
        // Zero delta short-circuits to strict_intersects.
        let a = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let b = Aabb::new(Vec2::new(15.0, 0.0), Vec2::new(10.0, 10.0));
        assert_eq!(a.sweep_time_of_impact(Vec2::ZERO, b), Some(0.0));
        // Disjoint with zero delta returns None.
        let c = Aabb::new(Vec2::new(100.0, 100.0), Vec2::new(5.0, 5.0));
        assert_eq!(a.sweep_time_of_impact(Vec2::ZERO, c), None);
    }
}

#[cfg(test)]
mod slab_sweep_agrees_with_parry {
    use super::*;

    /// Movement is safety-critical. `slab_sweep` answers only for separated
    /// boxes, and Parry is the oracle for them: both must agree on time of
    /// impact and contact normal. The test sweeps a deterministic grid of
    /// placements and directions, including corner grazes, flush starts, and
    /// parallel motion.
    fn xorshift(state: &mut u64) -> f32 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        // [-1, 1)
        ((*state >> 11) as f64 / (1u64 << 52) as f64) as f32 * 2.0 - 1.0
    }

    fn boxes(seed: &mut u64) -> (Aabb, Vec2, Aabb) {
        let ax = xorshift(seed) * 40.0;
        let ay = xorshift(seed) * 40.0;
        let ahw = 2.0 + xorshift(seed).abs() * 14.0;
        let ahh = 2.0 + xorshift(seed).abs() * 14.0;
        let bx = xorshift(seed) * 40.0;
        let by = xorshift(seed) * 40.0;
        let bhw = 2.0 + xorshift(seed).abs() * 14.0;
        let bhh = 2.0 + xorshift(seed).abs() * 14.0;
        let dx = xorshift(seed) * 60.0;
        let dy = xorshift(seed) * 60.0;
        (
            Aabb::new(Vec2::new(ax, ay), Vec2::new(ahw, ahh)),
            Vec2::new(dx, dy),
            Aabb::new(Vec2::new(bx, by), Vec2::new(bhw, bhh)),
        )
    }

    #[test]
    fn the_closed_form_matches_the_general_solver_on_separated_boxes() {
        let mut seed = 0x9E3779B97F4A7C15u64;
        let mut compared = 0usize;
        let mut hits = 0usize;
        for _ in 0..40_000 {
            let (lhs, delta, rhs) = boxes(&mut seed);
            if starts_overlapping(lhs, rhs) || delta.length_squared() <= 1.0e-8 {
                continue;
            }
            compared += 1;
            let fast = slab_sweep(lhs, delta, rhs);
            let oracle = parry_sweep(lhs, delta, rhs);
            match (fast, oracle) {
                (None, None) => {}
                (Some(f), Some(o)) => {
                    hits += 1;
                    assert!(
                        (f.time_of_impact - o.time_of_impact).abs() < 1.0e-3,
                        "toi disagreed: fast {} vs parry {} for {lhs:?} + {delta:?} vs {rhs:?}",
                        f.time_of_impact,
                        o.time_of_impact
                    );
                    // Compare as the movement kernel uses the normal: which
                    // axis is cancelled, and in which direction. A wrong
                    // normal slides a body along the wrong face.
                    //
                    // Not component-wise: on a near-flush contact Parry's GJK
                    // can return e.g. `(-0.0016, 0.9999986)` for `(0, 1)`.
                    // The closed form should not match that noise.
                    let axis_of = |n: Vec2| if n.x.abs() >= n.y.abs() { 0 } else { 1 };
                    assert_eq!(
                        axis_of(f.normal1),
                        axis_of(o.normal1),
                        "cancelled the wrong axis: fast {:?} vs parry {:?} for \
                         {lhs:?} + {delta:?} vs {rhs:?}",
                        f.normal1,
                        o.normal1
                    );
                    assert!(
                        f.normal1.dot(o.normal1) > 0.9,
                        "normal points the other way: fast {:?} vs parry {:?} for \
                         {lhs:?} + {delta:?} vs {rhs:?}",
                        f.normal1,
                        o.normal1
                    );
                }
                (f, o) => panic!(
                    "one solver saw a hit and the other did not: fast {f:?}, parry {o:?}, \
                     for {lhs:?} + {delta:?} vs {rhs:?}"
                ),
            }
        }
        // Premise: the generator produced hits and separated pairs, or the
        // checks above pass vacuously.
        assert!(
            compared > 20_000,
            "only {compared} separated pairs; the generator is not exercising the population"
        );
        assert!(
            hits > 1_000,
            "only {hits} hits out of {compared}; the generator is not aiming the boxes at \
             each other and the agreement above is mostly two Nones"
        );
    }
}

#[cfg(test)]
mod the_fast_path_is_actually_used {
    use super::*;

    fn parry_sweeps() -> usize {
        PARRY_SWEEPS.with(|count| count.get())
    }

    /// The differential test cannot catch a refactor that routes every call
    /// to the slow solver. This checks the route: a separated pair is
    /// answered without calling the general solver.
    #[test]
    fn a_separated_sweep_never_reaches_the_general_solver() {
        let before = parry_sweeps();

        // A plain approach, a clean miss, and a graze along a face: the common
        // movement-kernel queries.
        let mover = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0));
        let ahead = Aabb::new(Vec2::new(40.0, 0.0), Vec2::new(5.0, 5.0));
        let aside = Aabb::new(Vec2::new(0.0, 400.0), Vec2::new(5.0, 5.0));

        assert!(
            mover.sweep_hit(Vec2::new(60.0, 0.0), ahead).is_some(),
            "the premise: this approach must actually hit, or 'no parry call' is \
             trivially true"
        );
        assert!(mover.sweep_hit(Vec2::new(60.0, 0.0), aside).is_none());
        assert!(mover.sweep_hit(Vec2::new(0.0, 60.0), ahead).is_none());

        assert_eq!(
            parry_sweeps(),
            before,
            "a separated sweep reached parry2d's general solver; the closed-form \
             route is bypassed and the 10.7% is back"
        );
    }

    /// Premise guard: the counter must be capable of moving.
    ///
    /// Without this, a counter that is never incremented (or a deleted
    /// `parry_sweep`) would make the test above pass forever.
    #[test]
    fn the_counter_moves_when_the_general_solver_does_run() {
        let before = parry_sweeps();
        let a = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0));
        let b = Aabb::new(Vec2::new(2.0, 0.0), Vec2::new(5.0, 5.0));
        assert!(
            starts_overlapping(a, b),
            "the premise: this pair must be the overlapping case parry owns"
        );
        let _ = a.sweep_hit(Vec2::new(10.0, 0.0), b);
        assert!(
            parry_sweeps() > before,
            "the penetrating start must still route to parry"
        );
    }
}
