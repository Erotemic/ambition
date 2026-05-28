//! Bevy-native geometry helpers.
//!
//! Ambition uses Bevy's `Aabb2d` as its public rectangular collision primitive
//! instead of maintaining a bespoke engine AABB type. This module only keeps the
//! Ambition-specific semantics layered on top of that primitive: center/half
//! convenience helpers, strict platformer overlap where edge-touching is not an
//! overlap, and Parry-backed swept-box queries.

use bevy_math::bounding::Aabb2d;
use parry2d::{
    math::{Pose, Vector},
    query::{self, ShapeCastOptions},
    shape::Cuboid,
};

use crate::engine_core::Vec2;

const CONTACT_EPS: f32 = 1.0e-4;

/// Public engine AABB type.
///
/// This is Bevy's battle-tested 2D bounding box, re-exported under the
/// shorter `engine_core::Aabb` name so callers can keep importing
/// `ae::Aabb` (the `ae` prefix is the conventional alias for
/// `crate::engine_core`).
pub type Aabb = Aabb2d;

/// Construct an AABB from a minimum corner and a size.
pub fn aabb_from_min_size(min: Vec2, size: Vec2) -> Aabb {
    Aabb::new(min + size * 0.5, size * 0.5)
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
/// Bevy's own bounding-volume traits intentionally use general-purpose geometry
/// semantics. Ambition needs slightly stricter platformer semantics in a few
/// places, most importantly treating edge-touching boxes as non-overlapping.
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
    /// Ambition historically treated edge-touching boxes as non-overlapping. We
    /// preserve that gameplay contract with a cheap separating-axis guard before
    /// calling Parry's shape intersection routine, which considers touching
    /// shapes intersecting.
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
    /// This is a thin wrapper around Parry's shape cast / swept collision query.
    /// Callers pass a frame or ability delta directly, so `time_of_impact` is a
    /// normalized fraction of that delta rather than seconds.
    ///
    /// Parry deliberately reports some edge-touching starts as an immediate
    /// contact. Ambition's platformer contract is stricter: resting on a floor
    /// must not block horizontal motion, and sliding along a wall must not block
    /// vertical motion. We therefore discard zero-time Parry hits when the boxes
    /// were merely touching and the requested delta is not moving into the
    /// touching face.
    fn sweep_hit(self, delta: Vec2, rhs: Self) -> Option<AabbSweepHit> {
        if delta.length_squared() <= 1.0e-8 {
            return self.strict_intersects(rhs).then_some(AabbSweepHit {
                time_of_impact: 0.0,
                normal1: Vec2::ZERO,
            });
        }

        let moving_shape = parry_cuboid(self);
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
            &parry_pose(self),
            to_parry_vec(delta),
            &moving_shape,
            &parry_pose(rhs),
            Vector::ZERO,
            &static_shape,
            options,
        )
        .ok()
        .flatten()
        .and_then(|hit| {
            let time_of_impact = hit.time_of_impact.clamp(0.0, 1.0);
            if time_of_impact <= CONTACT_EPS
                && !self.strict_intersects(rhs)
                && !moves_into_touching_face(self, delta, rhs)
            {
                None
            } else {
                Some(AabbSweepHit {
                    time_of_impact,
                    normal1: Vec2::new(hit.normal1.x, hit.normal1.y),
                })
            }
        })
    }
}

fn moves_into_touching_face(lhs: Aabb, delta: Vec2, rhs: Aabb) -> bool {
    let y_ranges_overlap =
        lhs.bottom() > rhs.top() + CONTACT_EPS && lhs.top() < rhs.bottom() - CONTACT_EPS;
    let x_ranges_overlap =
        lhs.right() > rhs.left() + CONTACT_EPS && lhs.left() < rhs.right() - CONTACT_EPS;

    let touching_rhs_left = nearly_equal(lhs.right(), rhs.left());
    let touching_rhs_right = nearly_equal(lhs.left(), rhs.right());
    let touching_rhs_top = nearly_equal(lhs.bottom(), rhs.top());
    let touching_rhs_bottom = nearly_equal(lhs.top(), rhs.bottom());

    (touching_rhs_left && y_ranges_overlap && delta.x > CONTACT_EPS)
        || (touching_rhs_right && y_ranges_overlap && delta.x < -CONTACT_EPS)
        || (touching_rhs_top && x_ranges_overlap && delta.y > CONTACT_EPS)
        || (touching_rhs_bottom && x_ranges_overlap && delta.y < -CONTACT_EPS)
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
        // Ambition's contract: edge-touching AABBs do NOT count as
        // intersecting. The `body_overlaps_any` predicate relies on
        // this — a player resting on a floor must not register as
        // overlapping the floor.
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
