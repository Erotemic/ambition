//! Collision geometry primitives.
//!
//! Ambition starts with axis-aligned boxes because they are easy to debug,
//! deterministic, and sufficient for a first-pass platformer feel sandbox.

use crate::math::Vec2;

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

    pub fn intersects(self, rhs: Self) -> bool {
        self.left() < rhs.right()
            && self.right() > rhs.left()
            && self.top() < rhs.bottom()
            && self.bottom() > rhs.top()
    }

    pub fn translated(self, delta: Vec2) -> Self {
        Self::new(self.center + delta, self.half)
    }
}
