//! Sound-emitter geometry and distance attenuation in world units.

use bevy_ecs::{resource::Resource, schedule::SystemSet};
use bevy_math::Vec2;

/// Distances from the emitter's nearest point, in world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SfxFalloff {
    full: f32,
    silent: f32,
}

impl SfxFalloff {
    pub fn new(full: f32, silent: f32) -> Self {
        assert!(full.is_finite() && silent.is_finite() && full >= 0.0 && silent > full);
        Self { full, silent }
    }

    pub fn gain(self, distance: f32) -> f32 {
        if !distance.is_finite() {
            return 0.0;
        }
        let t = ((distance - self.full) / (self.silent - self.full)).clamp(0.0, 1.0);
        1.0 - t * t * (3.0 - 2.0 * t)
    }
}

impl Default for SfxFalloff {
    fn default() -> Self {
        Self {
            full: 160.0,
            silent: 1600.0,
        }
    }
}

/// A full-volume source region. A point is a region with no extent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SfxShape {
    Point(Vec2),
    Circle { center: Vec2, radius: f32 },
    Rectangle { min: Vec2, max: Vec2 },
}

impl SfxShape {
    pub fn distance(self, listener: Vec2) -> f32 {
        match self {
            Self::Point(position) => listener.distance(position),
            Self::Circle { center, radius } => {
                (listener.distance(center) - radius.max(0.0)).max(0.0)
            }
            Self::Rectangle { min, max } => listener.distance(listener.clamp(min, max)),
        }
    }

    pub fn position(self) -> Vec2 {
        match self {
            Self::Point(position) => position,
            Self::Circle { center, .. } => center,
            Self::Rectangle { min, max } => (min + max) * 0.5,
        }
    }
}

/// How one sound reaches its listeners. UI and announcements use `Global`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SfxEmitter {
    Global,
    Spatial {
        shape: SfxShape,
        falloff: SfxFalloff,
    },
}

impl SfxEmitter {
    pub fn point(position: Vec2) -> Self {
        Self::Spatial {
            shape: SfxShape::Point(position),
            falloff: SfxFalloff::default(),
        }
    }

    pub fn position(self) -> Option<Vec2> {
        match self {
            Self::Global => None,
            Self::Spatial { shape, .. } => Some(shape.position()),
        }
    }

    pub fn gain_at(self, listener: Vec2) -> f32 {
        match self {
            Self::Global => 1.0,
            Self::Spatial { shape, falloff } => falloff.gain(shape.distance(listener)),
        }
    }
}

impl From<Vec2> for SfxEmitter {
    fn from(position: Vec2) -> Self {
        Self::point(position)
    }
}

/// Local listening positions. Several listeners hear one voice once, at its
/// highest audible gain. No listener means that world sounds are silent.
#[derive(Resource, Default, Debug)]
pub struct SfxListeners(pub Vec<Vec2>);

impl SfxListeners {
    pub fn gain(&self, emitter: SfxEmitter) -> f32 {
        if emitter == SfxEmitter::Global {
            return 1.0;
        }
        self.0
            .iter()
            .map(|position| emitter.gain_at(*position))
            .fold(0.0, f32::max)
    }
}

/// Host listener projection precedes playback and voice updates.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SfxPresentationSet {
    Listeners,
    Playback,
    Attenuation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falloff_is_full_inside_silent_outside_and_monotone_between() {
        let falloff = SfxFalloff::new(10.0, 110.0);
        assert_eq!(falloff.gain(0.0), 1.0);
        assert_eq!(falloff.gain(10.0), 1.0);
        assert_eq!(falloff.gain(110.0), 0.0);
        assert_eq!(falloff.gain(200.0), 0.0);
        let mut previous = 1.0;
        for distance in 0..200 {
            let gain = falloff.gain(distance as f32);
            assert!((0.0..=previous).contains(&gain));
            previous = gain;
        }
    }

    #[test]
    fn extended_emitters_measure_distance_to_the_nearest_point() {
        let circle = SfxShape::Circle {
            center: Vec2::new(10.0, 20.0),
            radius: 5.0,
        };
        assert_eq!(circle.distance(Vec2::new(12.0, 20.0)), 0.0);
        assert_eq!(circle.distance(Vec2::new(18.0, 20.0)), 3.0);
        let rectangle = SfxShape::Rectangle {
            min: Vec2::new(10.0, 20.0),
            max: Vec2::new(30.0, 40.0),
        };
        assert_eq!(rectangle.distance(Vec2::new(12.0, 25.0)), 0.0);
        assert_eq!(rectangle.distance(Vec2::new(33.0, 44.0)), 5.0);
    }

    #[test]
    fn listeners_do_not_add_volume_and_global_sounds_need_no_listener() {
        let emitter = SfxEmitter::point(Vec2::ZERO);
        let listeners = SfxListeners(vec![Vec2::ZERO, Vec2::ZERO, Vec2::splat(9000.0)]);
        assert_eq!(listeners.gain(emitter), 1.0);
        assert_eq!(SfxListeners::default().gain(emitter), 0.0);
        assert_eq!(SfxListeners::default().gain(SfxEmitter::Global), 1.0);
    }
}
