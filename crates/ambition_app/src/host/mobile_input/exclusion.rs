//! Touch-control exclusion zones for menu drag gestures.
//!
//! Touch UI producers tag the controls that reserve screen space; the
//! menu bridge only asks whether a cursor/touch point falls in any active zone.

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchExclusionAnchor {
    BottomLeft,
    BottomRight,
    TopRight,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TouchExclusionShape {
    Rect { offset: Vec2, size: Vec2 },
    Circle { offset: Vec2, radius: f32 },
}

/// Component marker for touch UI regions that should not become menu drags.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TouchExclusionZone {
    pub anchor: TouchExclusionAnchor,
    pub shape: TouchExclusionShape,
}

impl TouchExclusionZone {
    pub const fn rect(anchor: TouchExclusionAnchor, offset: Vec2, size: Vec2) -> Self {
        Self {
            anchor,
            shape: TouchExclusionShape::Rect { offset, size },
        }
    }

    pub const fn circle(anchor: TouchExclusionAnchor, offset: Vec2, radius: f32) -> Self {
        Self {
            anchor,
            shape: TouchExclusionShape::Circle { offset, radius },
        }
    }

    pub fn contains(self, pos: Vec2, window_size: Vec2) -> bool {
        match self.shape {
            TouchExclusionShape::Rect { offset, size } => {
                let min = self.anchor.resolve_rect_min(offset, size, window_size);
                pos.x >= min.x
                    && pos.x <= min.x + size.x
                    && pos.y >= min.y
                    && pos.y <= min.y + size.y
            }
            TouchExclusionShape::Circle { offset, radius } => {
                let center = self.anchor.resolve_point(offset, window_size);
                pos.distance(center) <= radius
            }
        }
    }
}

impl TouchExclusionAnchor {
    fn resolve_point(self, offset: Vec2, window_size: Vec2) -> Vec2 {
        match self {
            Self::BottomLeft => Vec2::new(offset.x, window_size.y - offset.y),
            Self::BottomRight => Vec2::new(window_size.x - offset.x, window_size.y - offset.y),
            Self::TopRight => Vec2::new(window_size.x - offset.x, offset.y),
        }
    }

    fn resolve_rect_min(self, offset: Vec2, size: Vec2, window_size: Vec2) -> Vec2 {
        match self {
            Self::BottomLeft => Vec2::new(offset.x, window_size.y - offset.y - size.y),
            Self::BottomRight => Vec2::new(
                window_size.x - offset.x - size.x,
                window_size.y - offset.y - size.y,
            ),
            Self::TopRight => Vec2::new(window_size.x - offset.x - size.x, offset.y),
        }
    }
}

pub fn touch_exclusion_contains<'a>(
    zones: impl IntoIterator<Item = &'a TouchExclusionZone>,
    pos: Vec2,
    window_size: Vec2,
) -> bool {
    zones
        .into_iter()
        .any(|zone| zone.contains(pos, window_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Vec2 = Vec2::new(1280.0, 720.0);

    #[test]
    fn bottom_left_rect_resolves_from_screen_edge() {
        let zone = TouchExclusionZone::rect(
            TouchExclusionAnchor::BottomLeft,
            Vec2::ZERO,
            Vec2::new(210.0, 210.0),
        );
        assert!(zone.contains(Vec2::new(20.0, 700.0), WINDOW));
        assert!(!zone.contains(Vec2::new(220.0, 700.0), WINDOW));
        assert!(!zone.contains(Vec2::new(20.0, 500.0), WINDOW));
    }

    #[test]
    fn bottom_right_circle_resolves_from_screen_edge() {
        let zone =
            TouchExclusionZone::circle(TouchExclusionAnchor::BottomRight, Vec2::splat(80.0), 32.0);
        assert!(zone.contains(Vec2::new(1200.0, 640.0), WINDOW));
        assert!(!zone.contains(Vec2::new(1120.0, 640.0), WINDOW));
    }
}
