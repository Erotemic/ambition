//! **A local input SOURCE is not a control CHANNEL, and one integer meant
//! both.**
//!
//! Three concepts share the shape of a small number, and the engine had one
//! word for all of them:
//!
//! ```text
//! LocalInputSource  the thing a person picked up  — sparse, chosen in a lobby
//! ParticipantId     the seat at the machine       — dense, one per person here
//! PlayerSlot        the seat the simulation reads — dense, == the GGRS handle
//! ```
//!
//! ⛔ **the sparse one leaked into the dense ones, and a match stopped
//! responding.** A character-select screen deliberately keeps SOURCE numbers:
//! its own test says renumbering them would hand somebody the wrong controller.
//! A roster therefore legitimately says *"the human in this seat holds source
//! 3"*. That number was then used as the rollback channel — `PlayerSlot(3)` —
//! while the session was sized by COUNTING the humans, so GGRS created handles
//! `0..n` and nothing ever wrote handle 3. Reported by review (GPT 5.6,
//! 2026-08-07) and reachable three ways:
//!
//! * two humans on sources 0 and 3 get a two-handle session, and the second
//!   fighter is deaf for the whole match;
//! * pick two seats and set the FIRST to CPU — the remaining human is on source
//!   1 in a one-handle session, so nobody can move;
//! * the shipped Smash couch, where the keyboard is source 0 and two pad players
//!   are sources 1 and 2 — three participants for two channels.
//!
//! ⭐ **the plan is what makes the two numbers stop pretending to be one.** A
//! session decides, once, which source drives which channel. Channels stay dense
//! because the rollback host requires it; sources stay whatever the lobby said,
//! because that is who is holding what.
//!
//! ```text
//! lobby     KEYBOARD ─┐          ┌─ channel 0 ─ GGRS handle 0 ─ PlayerSlot(0)
//!           pad 0    ─┼ the plan ┤
//!           pad 2    ─┘          └─ channel 1 ─ GGRS handle 1 ─ PlayerSlot(1)
//! ```
//!
//! ⚠ **channel is spelled [`ParticipantId`] on purpose.** It is already the
//! dense seat identity that `SlotControls`, the per-seat latches and the GGRS
//! handle order all key on, and `ambition_platformer2d_actor_monolith`'s
//! `participant_seat` owns its one correspondence with `PlayerSlot`. A third
//! newtype for the same integer would be a third thing to keep in step; what was
//! missing is the SOURCE, and the map.
//!
//! ## ⛔ …and that spelling is a LEAK, not the model
//!
//! The behavioural defect above is fixed. The identity conflation under it is
//! not, and new code must not harden it. The chain this engine is heading for:
//!
//! ```text
//! LocalInputSource / InputSourceId   what somebody picked up
//!   → ParticipantId                  the PERSON — survives relaunch, seat
//!                                    reassignment, possession, a dead body
//!   → SessionSeatId                  a seat in THIS session's topology
//!   → ControlChannelId               a deterministic input channel
//!   → PlayerSlot                     what the simulation reads
//!   → the controlled actor
//! ```
//!
//! Two of those do not exist, and this type currently uses [`ParticipantId`] for
//! the fourth. The lifetimes are genuinely different: a participant outlives the
//! session, a channel belongs to one session's topology and dies with it.
//! `participant_seat`'s own docs already say the two "only currently share a
//! number and should eventually become a data mapping".
//!
//! **The standing rule until they are separated:** do not add new ARITHMETIC
//! equality between `ParticipantId` and `PlayerSlot` or a GGRS handle. Route
//! through [`LocalChannelPlan`], which is the map, and let a future
//! `ControlChannelId` replace the spelling in one place instead of in every
//! caller that did the arithmetic itself. Tracked in
//! `docs/planning/tracks.md`.

use crate::participant::ParticipantId;

/// **What a person is playing ON**, in a form that survives being written down.
///
/// ⚠ **not [`crate::sources::InputSourceId`], and the difference is lifetime.**
/// That type names a LIVE source — a gamepad is an `Entity`, meaningful only
/// while it stays plugged in. This one is what a lobby chose and a session
/// froze, so it has to outlive a disconnect: the Nth pad in arrival order, or
/// the keyboard.
///
/// ⭐ **the keyboard is a VARIANT, not a hole in the numbering.** It used to be
/// expressed by absence: a seat that owned no device row, compensated for by
/// subtracting one from every seat above it (`slot.saturating_sub(1)`) at two
/// call sites. That arithmetic silently produced `None` when it was wrong — a
/// player who is simply inert — and it could not express a keyboard player in
/// any seat but the first.
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

/// **Which source drives which channel**, decided once for a session.
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
    /// ⚠ the caller decides the ORDER, and it is part of the decision: seat
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
    /// ⚠ answers the FIRST channel holding it. A plan with a repeated source is
    /// a composition error — see [`Self::repeated_sources`] — and this cannot
    /// invent an answer for it.
    pub fn channel_for_source(&self, source: LocalInputSource) -> Option<ParticipantId> {
        self.sources
            .iter()
            .position(|held| *held == source)
            .map(|channel| ParticipantId(channel as u8))
    }

    /// **The channel playing on the keyboard, if anybody is.**
    ///
    /// ⭐ the plan is the authority on this once one exists, and
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

    /// **Sources claimed by more than one channel.**
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

    /// **The defect this type exists for**, in its smallest reachable form.
    ///
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
