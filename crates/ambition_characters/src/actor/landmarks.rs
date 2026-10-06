//! Semantic landmarks of a body: where its hands, its head and its feet are,
//! per pose.
//!
//! This is SIMULATION data, as a body rig is ([`super::body_rig`]). It holds
//! no texture, atlas rectangle or track name. A pet gesture, a muzzle or a
//! rider's grip asks for a [`Landmark`] and never for a sprite bound
//! (ruling Q41, `docs/planning/engine/runtime-rigged-sprite-animation.md`,
//! "Semantic landmarks").
//!
//! A body with a rig answers from its solved pose. A body without a rig
//! answers from a [`BodyLandmarkTable`]: the points its art package publishes
//! for each frame of each clip. The table uses the SAME clip names as the
//! character's sheet rows, so there is no second animation vocabulary.
//!
//! Space: a table point is local to the art's FEET, +x is the direction the
//! art faces, +y is DOWN, and the unit is one full-resolution sheet pixel.
//! Multiply by the world size of one pixel of the body to get the rig space a
//! [`super::body_rig`] attachment uses.

use std::collections::BTreeMap;

use bevy::math::Vec2;

use super::body_rig::{FOOT_FAR, FOOT_NEAR, HAND_FAR, HAND_NEAR, HEAD};

/// A named point on a body. The names are the rig attachment names, so a
/// rigged body and a table answer the same question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Landmark {
    /// The hand nearer the viewer.
    HandNear,
    /// The hand behind the body.
    HandFar,
    /// The point the head turns about.
    Head,
    /// The foot nearer the viewer.
    FootNear,
    /// The foot behind the body.
    FootFar,
}

impl Landmark {
    /// Every landmark, in slot order.
    pub const ALL: [Self; 5] = [
        Self::HandNear,
        Self::HandFar,
        Self::Head,
        Self::FootNear,
        Self::FootFar,
    ];

    /// The rig attachment that answers this landmark on a rigged body.
    pub fn attachment(self) -> &'static str {
        match self {
            Self::HandNear => HAND_NEAR,
            Self::HandFar => HAND_FAR,
            Self::Head => HEAD,
            Self::FootNear => FOOT_NEAR,
            Self::FootFar => FOOT_FAR,
        }
    }

    /// This landmark's slot in a [`LandmarkFrame`].
    pub fn slot(self) -> usize {
        self as usize
    }
}

/// The hand a character's art makes a gesture with: a pet, a shot.
///
/// The catalog row authors it, because no art package states it and no rule
/// gives it. Measured on the published flipbooks (2026-10-05): the hand nearer
/// the viewer is the forward hand in the gesture rows of some art, and trails
/// the body in other art, and neither the track names nor the idle stance
/// tells the two apart.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub enum GestureHand {
    /// The hand nearer the viewer. This is the default.
    #[default]
    Near,
    /// The hand behind the body: the art is drawn three-quarter on, and its
    /// far hand is on the side it faces.
    Far,
}

impl GestureHand {
    /// The hand landmarks to ask, first choice first: an art frame that does
    /// not draw the stated hand answers with the other hand.
    pub fn hands(self) -> [Landmark; 2] {
        match self {
            Self::Near => [Landmark::HandNear, Landmark::HandFar],
            Self::Far => [Landmark::HandFar, Landmark::HandNear],
        }
    }
}

/// The landmarks of one frame. A slot is `None` when the art has no such point
/// in this frame (a hidden hand, a body with no hands).
pub type LandmarkFrame = [Option<Vec2>; Landmark::ALL.len()];

/// One clip of a [`BodyLandmarkTable`].
#[derive(Debug, Clone, PartialEq)]
pub struct LandmarkClip {
    /// The art of this row loops. A row that does not loop holds its last
    /// frame, as the sprite and a rig clip do
    /// ([`super::body_rig::RigClip::looping`]). The source of the table states
    /// it: the query that reads the table infers nothing.
    pub looping: bool,
    pub frame_duration_s: f32,
    pub frames: Vec<LandmarkFrame>,
}

impl LandmarkClip {
    /// The frame a body shows `elapsed_s` seconds into this clip on its own
    /// clock. A looping clip wraps; a one-shot holds its last frame: the rule
    /// of [`super::body_rig::RigClip::frame_at_time`].
    pub fn frame_at_time(&self, elapsed_s: f32) -> usize {
        let count = self.frames.len().max(1);
        let index = (elapsed_s.max(0.0) / self.frame_duration_s).floor();
        let index = if index.is_finite() { index as usize } else { 0 };
        if self.looping {
            index % count
        } else {
            index.min(count - 1)
        }
    }

    /// The frame shown at normalized progress `phase` in `[0, 1]`: the rule a
    /// move's clip follows ([`super::body_rig::RigClip::frame_at_phase`]).
    pub fn frame_at_phase(&self, phase: f32) -> usize {
        let count = self.frames.len().max(1);
        let index = (phase.clamp(0.0, 1.0) * count as f32).floor();
        (if index.is_finite() { index as usize } else { 0 }).min(count - 1)
    }
}

/// The landmark points one character's art publishes, per clip and frame.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BodyLandmarkTable {
    /// The height of one sheet frame, in sheet pixels. A body that states its
    /// drawn quad and not its scale gets the scale from this.
    pub frame_height: f32,
    pub clips: BTreeMap<String, LandmarkClip>,
}

impl BodyLandmarkTable {
    pub fn clip(&self, name: &str) -> Option<&LandmarkClip> {
        self.clips.get(name)
    }

    /// The first clip of `chain` this table has, with its name.
    pub fn first_clip<'a>(
        &'a self,
        chain: impl IntoIterator<Item = &'a str>,
    ) -> Option<(&'a str, &'a LandmarkClip)> {
        chain
            .into_iter()
            .find_map(|name| self.clips.get_key_value(name))
            .map(|(name, clip)| (name.as_str(), clip))
    }

    /// `landmark` in frame `frame` of `clip`, in sheet pixels from the feet.
    pub fn point(&self, landmark: Landmark, clip: &str, frame: usize) -> Option<Vec2> {
        self.clip(clip)?.frames.get(frame)?[landmark.slot()]
    }

    /// `landmark` at normalized progress `phase` of `clip`, in sheet pixels
    /// from the feet.
    pub fn point_at_phase(&self, landmark: Landmark, clip: &str, phase: f32) -> Option<Vec2> {
        let frames = self.clip(clip)?;
        frames.frames.get(frames.frame_at_phase(phase))?[landmark.slot()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> BodyLandmarkTable {
        let frame = |hand_x: f32| -> LandmarkFrame {
            let mut frame = LandmarkFrame::default();
            frame[Landmark::HandNear.slot()] = Some(Vec2::new(hand_x, -30.0));
            frame
        };
        BodyLandmarkTable {
            frame_height: 100.0,
            clips: BTreeMap::from([(
                "pet".to_string(),
                LandmarkClip {
                    looping: true,
                    frame_duration_s: 0.5,
                    frames: vec![frame(1.0), frame(2.0), frame(3.0), frame(4.0)],
                },
            )]),
        }
    }

    #[test]
    fn a_point_is_read_by_clip_and_frame_and_an_absent_one_is_none() {
        let table = table();
        assert_eq!(table.point(Landmark::HandNear, "pet", 2), Some(Vec2::new(3.0, -30.0)));
        // The art has no head point: the answer is None, not the origin.
        assert_eq!(table.point(Landmark::Head, "pet", 2), None);
        assert_eq!(table.point(Landmark::HandNear, "walk", 0), None);
        assert_eq!(table.point(Landmark::HandNear, "pet", 4), None);
    }

    #[test]
    fn a_phase_picks_the_frame_a_move_clip_shows() {
        let table = table();
        let x = |phase| table.point_at_phase(Landmark::HandNear, "pet", phase).map(|p| p.x);
        assert_eq!(x(0.0), Some(1.0));
        assert_eq!(x(0.5), Some(3.0));
        // The end of the clip holds the last frame.
        assert_eq!(x(1.0), Some(4.0));
        let clip = table.clip("pet").expect("the clip");
        // A body's own clock wraps a looping clip.
        assert_eq!(clip.frame_at_time(2.25), 0);
        assert_eq!(clip.frame_at_time(0.75), 1);
        // A one-shot holds its last frame, as a rig clip does.
        let held = LandmarkClip {
            looping: false,
            ..clip.clone()
        };
        assert_eq!(held.frame_at_time(0.75), 1);
        assert_eq!(held.frame_at_time(2.25), 3);
        assert_eq!(held.frame_at_time(1.0e9), 3);
    }

    #[test]
    fn the_first_clip_of_a_chain_is_the_first_the_table_has() {
        let table = table();
        assert_eq!(table.first_clip(["run", "pet", "idle"]).map(|(name, _)| name), Some("pet"));
        assert!(table.first_clip(["run", "idle"]).is_none());
    }

    #[test]
    fn a_gesture_asks_the_stated_hand_first_and_the_near_hand_when_none_is_stated() {
        assert_eq!(GestureHand::default().hands(), [Landmark::HandNear, Landmark::HandFar]);
        assert_eq!(GestureHand::Far.hands(), [Landmark::HandFar, Landmark::HandNear]);
    }

    #[test]
    fn each_landmark_names_its_rig_attachment_and_its_own_slot() {
        for (slot, landmark) in Landmark::ALL.into_iter().enumerate() {
            assert_eq!(landmark.slot(), slot);
        }
        assert_eq!(Landmark::HandNear.attachment(), HAND_NEAR);
        assert_eq!(Landmark::Head.attachment(), HEAD);
    }
}
