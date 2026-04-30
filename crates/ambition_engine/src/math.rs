//! Small math utilities used by the deterministic simulation.
//!
//! This deliberately avoids renderer-specific vector types. Bevy has its own
//! `Vec2`, but the engine should be usable in tests, command-line validation,
//! or a future non-Bevy backend without pulling Bevy into the core crate.

use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

/// Minimal two-dimensional vector for game-space positions and velocities.
///
/// Ambition currently uses a screen-like coordinate system: increasing `x`
/// moves right, and increasing `y` moves downward. That matches the original
/// Macroquad prototype and makes collision/debug reasoning straightforward.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Normalize the vector, or return `fallback` for nearly-zero vectors.
    /// This is used for dash aiming, where neutral input should still dash in
    /// the facing direction rather than producing NaN or a zero dash.
    pub fn normalized_or(self, fallback: Self) -> Self {
        let len = self.length();
        if len > 1.0e-5 {
            self / len
        } else {
            fallback
        }
    }

    pub fn clamp_length_max(self, max_len: f32) -> Self {
        let len = self.length();
        if len > max_len && len > 1.0e-5 {
            self * (max_len / len)
        } else {
            self
        }
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }

    /// Return the left-handed perpendicular vector.
    pub fn perp(self) -> Self {
        Self::new(-self.y, self.x)
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

/// Move `value` toward `target` by at most `delta`.
///
/// This is the workhorse for acceleration, friction, and dummy knockback decay.
/// Keeping it here avoids each module inventing slightly different easing code.
pub fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}
