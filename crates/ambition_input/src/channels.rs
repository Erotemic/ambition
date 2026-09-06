//! Mapping between local input sources and dense session control channels.
//!
//! Device/source ids are sparse lobby identities; rollback channels are dense
//! session identities. [`LocalChannelPlan`] is the explicit mapping between
//! them. New code must not infer a channel or [`PlayerSlot`](crate::PlayerSlot)
//! by arithmetic on a source id or participant id.

use crate::participant::ParticipantId;

/// What a person is playing ON, in a form that survives being written down.
///
///  not [`crate::sources::InputSourceId`], and the difference is lifetime.
/// That type names a LIVE source — a gamepad is an `Entity`, meaningful only
/// while it stays plugged in. This one is what a lobby chose and a session
/// froze, so it has to outlive a disconnect: the Nth pad in arrival order, or
/// the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LocalInputSource {
    /// The keyboard-and-mouse bundle. Exactly one exists, and it is a source
    /// only where a policy says so — under
    /// [`crate::sources::InputAssignmentPolicy::UnifiedPrimary`] it drives the
    /// primary participant and belongs to nobody.
    Keyboard,
    /// The Nth connected pad, in [`crate::LocalDeviceOrder`]'s arrival order.
    Pad(u8),
}

impl LocalInputSource {
    /// The first pad — one controller on a desk, and the couch's player one.
    pub const FIRST_PAD: Self = Self::Pad(0);

    /// This source's index into the frozen device order, if it is a pad.
    pub const fn pad_index(self) -> Option<usize> {
        match self {
            Self::Keyboard => None,
            Self::Pad(index) => Some(index as usize),
        }
    }

    pub const fn is_keyboard(self) -> bool {
        matches!(self, Self::Keyboard)
    }
}

/// Which source drives which channel, decided once for a session.
///
/// Position IS the channel: `sources[0]` is what channel 0 listens to. That is
/// what keeps channels dense without discarding which controller a person is
/// holding.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalChannelPlan {
    sources: Vec<LocalInputSource>,
}

impl LocalChannelPlan {
    /// Build a plan from the sources that will be driven, in channel order.
    ///
    ///  the caller decides the ORDER, and it is part of the decision: seat
    /// order is what every existing roster means, and re-sorting by source
    /// number would silently swap two people's fighters.
    pub fn from_sources(sources: impl IntoIterator<Item = LocalInputSource>) -> Self {
        Self {
            sources: sources.into_iter().collect(),
        }
    }

    /// How many local channels this plan needs — the GGRS handle count, and the
    /// number of local participants to seat.
    pub fn channels(&self) -> usize {
        self.sources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// The sources, in channel order.
    pub fn sources(&self) -> &[LocalInputSource] {
        &self.sources
    }

    /// Which source drives this channel.
    pub fn source_for(&self, channel: ParticipantId) -> Option<LocalInputSource> {
        self.sources.get(channel.slot() as usize).copied()
    }

    /// Which channel this source drives, if it drives one.
    ///
    ///  answers the FIRST channel holding it. A plan with a repeated source is
    /// a composition error — see [`Self::repeated_sources`] — and this cannot
    /// invent an answer for it.
    pub fn channel_for_source(&self, source: LocalInputSource) -> Option<ParticipantId> {
        self.sources
            .iter()
            .position(|held| *held == source)
            .map(|channel| ParticipantId(channel as u8))
    }

    /// The channel playing on the keyboard, if anybody is.
    ///
    ///  the plan is the authority on this once one exists, and
    /// `keyboard_owner_for`'s policy answer is the fallback for a session that
    /// declared none. The difference is visible in the shipped Smash couch:
    /// under `JoinToClaim` the policy hands the keyboard to the PRIMARY
    /// participant unconditionally, so two pad players had player one bound to
    /// `Entity::PLACEHOLDER` — deaf to the controller in their hands.
    pub fn keyboard_channel(&self) -> Option<ParticipantId> {
        self.channel_for_source(LocalInputSource::Keyboard)
    }

    /// Every channel with the source it listens to, in channel order.
    pub fn channels_with_sources(
        &self,
    ) -> impl Iterator<Item = (ParticipantId, LocalInputSource)> + '_ {
        self.sources
            .iter()
            .copied()
            .enumerate()
            .map(|(channel, source)| (ParticipantId(channel as u8), source))
    }

    /// Sources claimed by more than one channel.
    ///
    /// One controller cannot drive two fighters, and a plan that says it does
    /// leaves one of them permanently still. Reported rather than silently
    /// deduplicated: which of the two seats should lose its driver is not a
    /// question this type can answer.
    pub fn repeated_sources(&self) -> Vec<LocalInputSource> {
        let mut seen: Vec<LocalInputSource> = Vec::new();
        let mut repeated: Vec<LocalInputSource> = Vec::new();
        for source in &self.sources {
            if seen.contains(source) {
                if !repeated.contains(source) {
                    repeated.push(*source);
                }
            } else {
                seen.push(*source);
            }
        }
        repeated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lobby seats one CPU and one human, and the human is holding the second
    /// controller. The channel is ZERO — there is one person playing — while the
    /// source stays pad 1, because that is the pad in their hands.
    #[test]
    fn a_sparse_source_still_lands_on_a_dense_channel() {
        let plan = LocalChannelPlan::from_sources([LocalInputSource::Pad(1)]);
        assert_eq!(plan.channels(), 1);
        assert_eq!(
            plan.source_for(ParticipantId::PRIMARY),
            Some(LocalInputSource::Pad(1)),
            "channel 0 must still be told which controller it listens to"
        );
        assert_eq!(
            plan.channel_for_source(LocalInputSource::Pad(1)),
            Some(ParticipantId::PRIMARY)
        );
        assert_eq!(
            plan.source_for(ParticipantId(1)),
            None,
            "there is no seat 1"
        );
    }

    /// The select screen's own case: three people on pads 0, 1 and 3.
    #[test]
    fn a_hole_in_the_sources_is_not_a_hole_in_the_channels() {
        let plan = LocalChannelPlan::from_sources([0, 1, 3].map(LocalInputSource::Pad));
        assert_eq!(plan.channels(), 3);
        assert_eq!(
            plan.channels_with_sources().collect::<Vec<_>>(),
            vec![
                (ParticipantId(0), LocalInputSource::Pad(0)),
                (ParticipantId(1), LocalInputSource::Pad(1)),
                (ParticipantId(2), LocalInputSource::Pad(3)),
            ]
        );
        assert!(plan.repeated_sources().is_empty());
        assert_eq!(plan.keyboard_channel(), None, "nobody is playing on keys");
    }

    /// A keyboard player in a seat that is not the first — inexpressible while
    /// the keyboard was a hole in a numbering.
    #[test]
    fn the_keyboard_is_a_source_a_seat_can_hold() {
        let plan =
            LocalChannelPlan::from_sources([LocalInputSource::Pad(0), LocalInputSource::Keyboard]);
        assert_eq!(plan.keyboard_channel(), Some(ParticipantId(1)));
        assert_eq!(
            plan.source_for(ParticipantId(0))
                .and_then(|s| s.pad_index()),
            Some(0)
        );
        assert_eq!(
            plan.source_for(ParticipantId(1))
                .and_then(|s| s.pad_index()),
            None,
            "a keyboard seat indexes no pad, rather than indexing the wrong one"
        );
    }

    #[test]
    fn one_controller_driving_two_seats_is_reported() {
        let plan = LocalChannelPlan::from_sources([1, 0, 1].map(LocalInputSource::Pad));
        assert_eq!(plan.repeated_sources(), vec![LocalInputSource::Pad(1)]);
    }
}
