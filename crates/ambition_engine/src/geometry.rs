//! Collision geometry primitives.
//!
//! Ambition starts with axis-aligned boxes because they are easy to debug,
//! deterministic, and sufficient for a first-pass platformer feel sandbox. The
//! public engine type remains a tiny `Aabb`, but narrow-phase overlap and swept
//! box queries are delegated to `parry2d` so future collision work can grow from
//! a tested geometry library instead of one-off math helpers.

use parry2d::{
    math::{Pose, Vector},
    query::{self, ShapeCastOptions},
    shape::Cuboid,
};

use crate::Vec2;

const CONTACT_EPS: f32 = 1.0e-4;

/// Axis-aligned bounding box represented by center and half extents.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub center: Vec2,
    pub half: Vec2,
}

impl Aabb {
    pub const fn new(center: Vec2, half: Vec2) -> Self {
        Self { center, half }
    }

    pub fn from_min_size(min: Vec2, size: Vec2) -> Self {
        Self::new(min + size * 0.5, size * 0.5)
    }

    pub fn min(self) -> Vec2 {
        self.center - self.half
    }

    pub fn max(self) -> Vec2 {
        self.center + self.half
    }

    pub fn top(self) -> f32 {
        self.center.y - self.half.y
    }

    pub fn bottom(self) -> f32 {
        self.center.y + self.half.y
    }

    pub fn left(self) -> f32 {
        self.center.x - self.half.x
    }

    pub fn right(self) -> f32 {
        self.center.x + self.half.x
    }

    /// Strict AABB overlap test backed by Parry.
    ///
    /// Ambition historically treated edge-touching boxes as non-overlapping. We
    /// preserve that gameplay contract with a cheap separating-axis guard before
    /// calling Parry's shape intersection routine, which considers touching
    /// shapes intersecting. This gives us Parry-backed geometry without changing
    /// long-standing platformer contact semantics.
    pub fn intersects(self, rhs: Self) -> bool {
        if self.right() <= rhs.left()
            || self.left() >= rhs.right()
            || self.bottom() <= rhs.top()
            || self.top() >= rhs.bottom()
        {
            return false;
        }

        let lhs_shape = self.parry_cuboid();
        let rhs_shape = rhs.parry_cuboid();
        query::intersection_test(
            &self.parry_pose(),
            &lhs_shape,
            &rhs.parry_pose(),
            &rhs_shape,
        )
        .unwrap_or(true)
    }

    pub fn translated(self, delta: Vec2) -> Self {
        Self::new(self.center + delta, self.half)
    }

    /// Return the time in `[0, 1]` at which this box first touches `rhs` while
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
    pub fn sweep_time_of_impact(self, delta: Vec2, rhs: Self) -> Option<f32> {
        if delta.length_squared() <= 1.0e-8 {
            return self.intersects(rhs).then_some(0.0);
        }

        let moving_shape = self.parry_cuboid();
        let static_shape = rhs.parry_cuboid();
        let mut options = ShapeCastOptions::default();
        options.max_time_of_impact = 1.0;
        options.target_distance = 0.0;
        options.stop_at_penetration = true;

        query::cast_shapes(
            &self.parry_pose(),
            to_parry_vec(delta),
            &moving_shape,
            &rhs.parry_pose(),
            Vector::ZERO,
            &static_shape,
            options,
        )
        .ok()
        .flatten()
        .and_then(|hit| {
            let time_of_impact = hit.time_of_impact.clamp(0.0, 1.0);
            if time_of_impact <= CONTACT_EPS
                && !self.intersects(rhs)
                && !self.moves_into_touching_face(delta, rhs)
            {
                None
            } else {
                Some(time_of_impact)
            }
        })
    }

    fn moves_into_touching_face(self, delta: Vec2, rhs: Self) -> bool {
        let y_ranges_overlap = self.bottom() > rhs.top() + CONTACT_EPS
            && self.top() < rhs.bottom() - CONTACT_EPS;
        let x_ranges_overlap = self.right() > rhs.left() + CONTACT_EPS
            && self.left() < rhs.right() - CONTACT_EPS;

        let touching_rhs_left = nearly_equal(self.right(), rhs.left());
        let touching_rhs_right = nearly_equal(self.left(), rhs.right());
        let touching_rhs_top = nearly_equal(self.bottom(), rhs.top());
        let touching_rhs_bottom = nearly_equal(self.top(), rhs.bottom());

        (touching_rhs_left && y_ranges_overlap && delta.x > CONTACT_EPS)
            || (touching_rhs_right && y_ranges_overlap && delta.x < -CONTACT_EPS)
            || (touching_rhs_top && x_ranges_overlap && delta.y > CONTACT_EPS)
            || (touching_rhs_bottom && x_ranges_overlap && delta.y < -CONTACT_EPS)
    }

    fn parry_pose(self) -> Pose {
        Pose::translation(self.center.x, self.center.y)
    }

    fn parry_cuboid(self) -> Cuboid {
        Cuboid::new(to_parry_vec(Vec2::new(
            self.half.x.max(0.0),
            self.half.y.max(0.0),
        )))
    }
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
        assert_eq!(body.sweep_time_of_impact(Vec2::new(8.0, 0.0), wall), Some(0.0));
    }
}
