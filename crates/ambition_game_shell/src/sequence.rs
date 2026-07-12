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
        frames: Vec<String>,
        frames_per_second: f32,
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
        Self {
            id: id.into(),
            role: ShellSegmentRole::Vanity,
            presentation: ShellSegmentPresentation::ImageSequence {
                frames: frames.into_iter().map(Into::into).collect(),
                frames_per_second,
                alt_text: alt_text.into(),
            },
            policy: ShellSegmentPolicy::default(),
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
