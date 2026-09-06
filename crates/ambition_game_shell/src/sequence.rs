//! Neutral ordered presentation-sequence data and runtime.

use std::collections::BTreeMap;
use std::time::Duration;

use bevy::prelude::{Component, Message, Resource};

use crate::{ShellActivationId, ShellExperienceId, ShellSegmentId, ShellSegmentKindId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShellSegmentRole {
    Vanity,
    Notice,
    TitleReveal,
    CreditsSection,
    Custom(String),
}

/// One picture in an image sequence, held for its own duration.
///
/// Per-frame holds rather than a single frame rate: authored card animations are
/// beat-based (a pose held while a caption reads, then a fast run of frames),
/// and a uniform rate can only approximate that by repeating frames. Carrying
/// the hold means a two-second pause costs ONE image, so a sequence ships only
/// its distinct pictures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellSequenceFrame {
    pub asset_path: String,
    pub hold: Duration,
}

impl ShellSequenceFrame {
    pub fn new(asset_path: impl Into<String>, hold: Duration) -> Self {
        Self {
            asset_path: asset_path.into(),
            hold,
        }
    }
}

/// Total time an image sequence occupies — the sum of its frame holds.
///
/// The single authority for a sequence's length: the segment policy derives
/// `auto_advance_after` from this so the animation cannot outlive or undershoot
/// the card that hosts it.
pub fn image_sequence_total(frames: &[ShellSequenceFrame]) -> Duration {
    frames
        .iter()
        .fold(Duration::ZERO, |total, frame| total + frame.hold)
}

/// Index of the frame showing at `elapsed`, played ONCE and holding the last.
///
/// A vanity card is not a loop: past the end it stays on the final picture so
/// the segment's fade-out lands on the punchline rather than a wrapped frame.
pub fn image_sequence_frame_at(frames: &[ShellSequenceFrame], elapsed: Duration) -> usize {
    let mut boundary = Duration::ZERO;
    for (index, frame) in frames.iter().enumerate() {
        boundary += frame.hold;
        if elapsed < boundary {
            return index;
        }
    }
    frames.len().saturating_sub(1)
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShellSegmentPresentation {
    TextCard {
        title: String,
        subtitle: Option<String>,
    },
    StaticImage {
        asset_path: String,
        alt_text: String,
    },
    ImageSequence {
        frames: Vec<ShellSequenceFrame>,
        alt_text: String,
    },
    Registered(ShellSegmentKindId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellSkipPolicy {
    Never,
    Immediate,
    After(Duration),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShellSegmentPolicy {
    pub auto_advance_after: Option<Duration>,
    pub skip_policy: ShellSkipPolicy,
    pub requires_acknowledgement: bool,
}

impl Default for ShellSegmentPolicy {
    fn default() -> Self {
        Self {
            auto_advance_after: Some(Duration::from_secs(2)),
            skip_policy: ShellSkipPolicy::Immediate,
            requires_acknowledgement: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShellSegmentSpec {
    pub id: ShellSegmentId,
    pub role: ShellSegmentRole,
    pub presentation: ShellSegmentPresentation,
    pub policy: ShellSegmentPolicy,
}

impl ShellSegmentSpec {
    pub fn text(id: impl Into<ShellSegmentId>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role: ShellSegmentRole::Vanity,
            presentation: ShellSegmentPresentation::TextCard {
                title: title.into(),
                subtitle: None,
            },
            policy: ShellSegmentPolicy::default(),
        }
    }

    pub fn static_image(
        id: impl Into<ShellSegmentId>,
        asset_path: impl Into<String>,
        alt_text: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            role: ShellSegmentRole::Vanity,
            presentation: ShellSegmentPresentation::StaticImage {
                asset_path: asset_path.into(),
                alt_text: alt_text.into(),
            },
            policy: ShellSegmentPolicy::default(),
        }
    }

    /// A sequence played at one uniform rate — the degenerate case of
    /// [`Self::image_sequence_timed`], which every frame holding equally.
    pub fn image_sequence<I, S>(
        id: impl Into<ShellSegmentId>,
        frames: I,
        frames_per_second: f32,
        alt_text: impl Into<String>,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        assert!(
            frames_per_second.is_finite() && frames_per_second > 0.0,
            "image sequence frame rate must be positive",
        );
        let hold = Duration::from_secs_f32(1.0 / frames_per_second);
        Self::image_sequence_timed(
            id,
            frames
                .into_iter()
                .map(|path| ShellSequenceFrame::new(path, hold)),
            alt_text,
        )
    }

    /// A sequence whose frames each carry their own hold.
    ///
    /// The segment's `auto_advance_after` is DERIVED from the frame holds, so
    /// the card lives exactly as long as its animation.
    pub fn image_sequence_timed<I>(
        id: impl Into<ShellSegmentId>,
        frames: I,
        alt_text: impl Into<String>,
    ) -> Self
    where
        I: IntoIterator<Item = ShellSequenceFrame>,
    {
        let frames: Vec<ShellSequenceFrame> = frames.into_iter().collect();
        let total = image_sequence_total(&frames);
        Self {
            id: id.into(),
            role: ShellSegmentRole::Vanity,
            presentation: ShellSegmentPresentation::ImageSequence {
                frames,
                alt_text: alt_text.into(),
            },
            policy: ShellSegmentPolicy {
                auto_advance_after: Some(total),
                ..ShellSegmentPolicy::default()
            },
        }
    }

    pub fn registered(
        id: impl Into<ShellSegmentId>,
        role: ShellSegmentRole,
        kind: impl Into<ShellSegmentKindId>,
    ) -> Self {
        Self {
            id: id.into(),
            role,
            presentation: ShellSegmentPresentation::Registered(kind.into()),
            policy: ShellSegmentPolicy {
                auto_advance_after: None,
                ..ShellSegmentPolicy::default()
            },
        }
    }

    pub fn with_role(mut self, role: ShellSegmentRole) -> Self {
        self.role = role;
        self
    }

    pub fn with_policy(mut self, policy: ShellSegmentPolicy) -> Self {
        self.policy = policy;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShellSequenceSpec {
    pub segments: Vec<ShellSegmentSpec>,
}

#[derive(Resource, Default)]
pub struct ShellSequenceCatalog {
    sequences: BTreeMap<ShellExperienceId, ShellSequenceSpec>,
}

impl ShellSequenceCatalog {
    pub fn register(&mut self, experience: ShellExperienceId, spec: ShellSequenceSpec) {
        self.sequences.insert(experience, spec);
    }

    pub fn get(&self, experience: &ShellExperienceId) -> Option<&ShellSequenceSpec> {
        self.sequences.get(experience)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShellSequenceRuntime {
    pub spec: ShellSequenceSpec,
    pub segment_index: usize,
    pub elapsed: Duration,
    pub finished: bool,
}

impl ShellSequenceRuntime {
    pub fn new(spec: ShellSequenceSpec) -> Self {
        let finished = spec.segments.is_empty();
        Self {
            spec,
            segment_index: 0,
            elapsed: Duration::ZERO,
            finished,
        }
    }

    pub fn current(&self) -> Option<&ShellSegmentSpec> {
        self.spec.segments.get(self.segment_index)
    }

    pub fn may_skip(&self) -> bool {
        let Some(current) = self.current() else {
            return false;
        };
        match current.policy.skip_policy {
            ShellSkipPolicy::Never => false,
            ShellSkipPolicy::Immediate => true,
            ShellSkipPolicy::After(duration) => self.elapsed >= duration,
        }
    }

    pub fn tick(&mut self, delta: Duration) -> bool {
        if self.finished {
            return false;
        }
        self.elapsed = self.elapsed.saturating_add(delta);
        let auto_advance = self
            .current()
            .and_then(|segment| segment.policy.auto_advance_after)
            .is_some_and(|duration| self.elapsed >= duration);
        if auto_advance
            && !self
                .current()
                .is_some_and(|segment| segment.policy.requires_acknowledgement)
        {
            return self.advance();
        }
        false
    }

    pub fn skip(&mut self) -> bool {
        if self.may_skip() {
            self.advance()
        } else {
            false
        }
    }

    pub fn acknowledge(&mut self) -> bool {
        if self
            .current()
            .is_some_and(|segment| segment.policy.requires_acknowledgement)
        {
            self.advance()
        } else {
            false
        }
    }

    pub fn complete_programmatic_segment(&mut self) -> bool {
        if self.current().is_some_and(|segment| {
            matches!(
                &segment.presentation,
                ShellSegmentPresentation::Registered(_)
            )
        }) {
            self.advance()
        } else {
            false
        }
    }

    fn advance(&mut self) -> bool {
        self.segment_index = self.segment_index.saturating_add(1);
        self.elapsed = Duration::ZERO;
        if self.segment_index >= self.spec.segments.len() {
            self.finished = true;
            true
        } else {
            false
        }
    }
}

#[derive(Resource, Default)]
pub struct ActiveShellSequence {
    pub activation_id: Option<ShellActivationId>,
    pub runtime: Option<ShellSequenceRuntime>,
}

impl ActiveShellSequence {
    /// Return the currently active registered programmatic segment, if any.
    pub fn registered_segment(
        &self,
    ) -> Option<(ShellActivationId, &ShellSegmentId, &ShellSegmentKindId)> {
        let activation_id = self.activation_id?;
        let segment = self.runtime.as_ref()?.current()?;
        let ShellSegmentPresentation::Registered(kind) = &segment.presentation else {
            return None;
        };
        Some((activation_id, &segment.id, kind))
    }
}

/// Attach to entities owned by one programmatic sequence segment.
///
/// The sequence plugin removes these entities as soon as the segment is no
/// longer current, including skip, failure, route replacement, and completion.
#[derive(Component, Clone, Debug, Eq, PartialEq)]
pub struct ShellSegmentScopedEntity {
    pub activation_id: ShellActivationId,
    pub segment_id: ShellSegmentId,
}

#[cfg(test)]
mod image_sequence_tests {
    use super::*;

    fn frames() -> Vec<ShellSequenceFrame> {
        vec![
            ShellSequenceFrame::new("a.png", Duration::from_millis(50)),
            ShellSequenceFrame::new("b.png", Duration::from_millis(650)),
            ShellSequenceFrame::new("c.png", Duration::from_millis(300)),
        ]
    }

    #[test]
    fn total_is_the_sum_of_holds() {
        assert_eq!(image_sequence_total(&frames()), Duration::from_millis(1000));
        assert_eq!(image_sequence_total(&[]), Duration::ZERO);
    }

    #[test]
    fn frame_lookup_respects_each_frames_own_hold() {
        let frames = frames();
        let at = |ms| image_sequence_frame_at(&frames, Duration::from_millis(ms));
        // A short lead-in frame must not swallow the long one after it, which is
        // exactly what a uniform frame rate would get wrong.
        assert_eq!(at(0), 0);
        assert_eq!(at(49), 0);
        assert_eq!(at(50), 1);
        assert_eq!(at(699), 1);
        assert_eq!(at(700), 2);
    }

    #[test]
    fn the_sequence_plays_once_and_holds_the_last_frame() {
        let frames = frames();
        // Past the end it must NOT wrap to frame 0 — the card's fade-out has to
        // land on the punchline.
        assert_eq!(
            image_sequence_frame_at(&frames, Duration::from_millis(1000)),
            2
        );
        assert_eq!(image_sequence_frame_at(&frames, Duration::from_secs(60)), 2);
    }

    #[test]
    fn an_empty_sequence_resolves_to_a_harmless_index() {
        assert_eq!(image_sequence_frame_at(&[], Duration::from_secs(1)), 0);
    }

    #[test]
    fn segment_duration_is_derived_from_the_frames_not_hand_set() {
        // The animation and the card lifetime cannot drift: the policy's
        // auto-advance is the frame total.
        let spec = ShellSegmentSpec::image_sequence_timed("card", frames(), "alt");
        assert_eq!(
            spec.policy.auto_advance_after,
            Some(Duration::from_millis(1000))
        );
    }

    #[test]
    fn a_uniform_rate_is_just_equal_holds() {
        let spec = ShellSegmentSpec::image_sequence("card", ["a.png", "b.png"], 4.0, "alt");
        let ShellSegmentPresentation::ImageSequence { frames, .. } = &spec.presentation else {
            panic!("expected an image sequence");
        };
        assert!(frames
            .iter()
            .all(|frame| frame.hold == Duration::from_millis(250)));
        assert_eq!(
            spec.policy.auto_advance_after,
            Some(Duration::from_millis(500))
        );
    }
}

#[derive(Message, Clone, Debug, Eq, PartialEq)]
pub enum ShellSequenceCommand {
    Skip {
        activation_id: ShellActivationId,
    },
    Acknowledge {
        activation_id: ShellActivationId,
    },
    ProgrammaticSegmentCompleted {
        activation_id: ShellActivationId,
        segment_id: ShellSegmentId,
    },
    ProgrammaticSegmentFailed {
        activation_id: ShellActivationId,
        segment_id: ShellSegmentId,
        message: String,
    },
}
