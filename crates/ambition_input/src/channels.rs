//! Mapping between local input sources and dense session control channels.
//!
//! Device/source ids are sparse lobby identities; rollback channels are dense
//! session identities. [`LocalChannelPlan`] is the explicit mapping between
//! them. New code must not infer a channel or [`PlayerSlot`](crate::PlayerSlot)
//! by arithmetic on a source id or participant id.

use crate::participant::ParticipantId;

/// The device a person plays on, in a form that survives serialization.
///
/// This is not [`crate::sources::InputSourceId`]. That type names a live
/// source (a gamepad `Entity`, valid only while connected). This type is what
/// a lobby chose and a session froze, so it outlives a disconnect: the Nth pad
/// in arrival order, or the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LocalInputSource {
    /// The keyboard-and-mouse pair. There is one. It is a source only where a
    /// policy says so: under
    /// [`crate::sources::InputAssignmentPolicy::UnifiedPrimary`] it drives the
    /// primary participant and belongs to no seat.
    Keyboard,
    /// The Nth connected pad, in [`crate::LocalDeviceOrder`]'s arrival order.
    Pad(u8),
}

impl LocalInputSource {
    /// The first pad: a single desk controller, or couch player one.
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
/// Position is the channel: `sources[0]` drives channel 0. This keeps channels
/// dense and still records which controller each person holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalChannelPlan {
    sources: Vec<LocalInputSource>,
}

impl LocalChannelPlan {
    /// Build a plan from the sources that will be driven, in channel order.
    ///
    /// The caller decides the order. Seat order is what rosters mean;
    /// sorting by source number would swap two people's fighters.
    pub fn from_sources(sources: impl IntoIterator<Item = LocalInputSource>) -> Self {
        Self {
            sources: sources.into_iter().collect(),
        }
    }

    /// How many local channels this plan needs: the GGRS handle count and the
    /// number of local participants.
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
    /// Returns the first channel that holds it. A repeated source is a
    /// composition error; see [`Self::repeated_sources`].
    pub fn channel_for_source(&self, source: LocalInputSource) -> Option<ParticipantId> {
        self.sources
            .iter()
            .position(|held| *held == source)
            .map(|channel| ParticipantId(channel as u8))
    }

    /// The channel playing on the keyboard, if anybody is.
    ///
    /// When a plan exists, it is the authority. `keyboard_owner_for`'s policy
    /// answer is only the fallback for sessions without a plan. Under
    /// `JoinToClaim` that policy gives the keyboard to the primary participant
    /// always, so with two pad players player one was bound to
    /// `Entity::PLACEHOLDER` and ignored their pad.
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
    /// One controller cannot drive two fighters; one would never move. This
    /// reports the fault and does not deduplicate, because this type cannot
    /// choose which seat loses its driver.
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

    /// A lobby seats one CPU and one human on the second pad. The channel is 0
    /// (one person plays); the source stays pad 1.
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

    /// Three people on pads 0, 1, and 3.
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

    /// A keyboard player in a seat that is not the first.
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
