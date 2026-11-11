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

/// A listening space: a spatial sound is heard only by listeners in its own
/// space, so two spaces may share coordinates without hearing each other. The
/// host names the spaces (a live room each); this crate only compares them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SfxSpace(pub u32);

impl From<u32> for SfxSpace {
    fn from(space: u32) -> Self {
        Self(space)
    }
}

/// How one sound reaches its listeners. UI and announcements use `Global`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SfxEmitter {
    Global,
    Spatial {
        shape: SfxShape,
        falloff: SfxFalloff,
        /// The space it sounds in; `None` reaches a listener in any space.
        space: Option<SfxSpace>,
    },
}

impl SfxEmitter {
    pub fn point(position: Vec2) -> Self {
        Self::Spatial {
            shape: SfxShape::Point(position),
            falloff: SfxFalloff::default(),
            space: None,
        }
    }

    /// This emitter, sounding in `space` when one is named. A global sound
    /// stays global, and an emitter that already names its space keeps it.
    pub fn in_space(self, space: Option<SfxSpace>) -> Self {
        match self {
            Self::Spatial { shape, falloff, space: None } => Self::Spatial { shape, falloff, space },
            other => other,
        }
    }

    /// Whether a listener in `listener` space can hear it at all.
    pub fn reaches(self, listener: Option<SfxSpace>) -> bool {
        match self {
            Self::Global | Self::Spatial { space: None, .. } => true,
            Self::Spatial { space: Some(space), .. } => listener.is_none_or(|listener| listener == space),
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
            Self::Spatial { shape, falloff, .. } => falloff.gain(shape.distance(listener)),
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
///
/// The second list is each listener's space, by index: `.1[i]` is the space
/// of `.0[i]`, and a listener with no entry (or `None`) hears every space.
#[derive(Resource, Default, Debug)]
pub struct SfxListeners(pub Vec<Vec2>, pub Vec<Option<SfxSpace>>);

impl SfxListeners {
    pub fn gain(&self, emitter: SfxEmitter) -> f32 {
        if emitter == SfxEmitter::Global {
            return 1.0;
        }
        self.0
            .iter()
            .enumerate()
            .filter(|(index, _)| emitter.reaches(self.1.get(*index).copied().flatten()))
            .map(|(_, position)| emitter.gain_at(*position))
            .fold(0.0, f32::max)
    }

    /// Listen at `position` in `space`; the same listener twice is one.
    pub fn listen(&mut self, position: Vec2, space: Option<SfxSpace>) {
        self.1.resize(self.0.len(), None);
        if !self.0.iter().zip(&self.1).any(|(at, heard)| *at == position && *heard == space) {
            self.0.push(position);
            self.1.push(space);
        }
    }

    pub fn clear(&mut self) {
        self.0.clear();
        self.1.clear();
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
        let listeners = SfxListeners(vec![Vec2::ZERO, Vec2::ZERO, Vec2::splat(9000.0)], Vec::new());
        assert_eq!(listeners.gain(emitter), 1.0);
        assert_eq!(SfxListeners::default().gain(emitter), 0.0);
        assert_eq!(SfxListeners::default().gain(SfxEmitter::Global), 1.0);
    }

    #[test]
    fn a_sound_in_one_space_is_not_heard_from_another_at_the_same_place() {
        let (here, there) = (Some(SfxSpace(1)), Some(SfxSpace(2)));
        let mut listeners = SfxListeners::default();
        listeners.listen(Vec2::ZERO, there);
        let sound = SfxEmitter::point(Vec2::ZERO);
        assert_eq!(listeners.gain(sound.in_space(here)), 0.0, "another space's listener heard it");
        assert_eq!(listeners.gain(sound.in_space(there)), 1.0);
        assert_eq!(listeners.gain(sound), 1.0, "a sound in no space reaches every listener");
        listeners.listen(Vec2::ZERO, here);
        assert_eq!(listeners.gain(sound.in_space(here)), 1.0);
        assert_eq!(SfxEmitter::Global.in_space(here), SfxEmitter::Global);
        listeners.clear();
        listeners.0.push(Vec2::ZERO);
        assert_eq!(listeners.gain(sound.in_space(here)), 1.0, "a listener in no space hears every space");
    }
}
