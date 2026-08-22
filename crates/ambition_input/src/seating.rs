//! **WHERE A SESSION'S LOCAL SEATS COME FROM, and whose answer that is.**
//!
//! ⛔⛔ **HOW MANY PEOPLE ARE PLAYING IS A FACT ABOUT INPUT**, decided by a
//! lobby, a roster, or an experience that simply is two-player — never by a
//! backend. This declaration lived inside `ambition_platformer2d_rollback_ggrs`,
//! so only a rollback host could hear one and every other surface was silently
//! seated from connected DEVICES. A composition could then declare two seats,
//! receive its second `InputParticipant`, receive the pad — and still not move
//! the second body, because the session had opened one handle from the device
//! count and is never resized.
//!
//! ⚠ **a ROSTER is one decider among several**, which is why the variants are
//! not named for one: a plaza with no roster can still be two-player by
//! construction. What this type means is *somebody has claimed local seating,
//! and here is their answer*.

use bevy::prelude::Resource;

/// **Where this session's seats come from, whether they are decided yet, and
/// whose answer it is.**
///
/// One value for the whole chain: an experience CLAIMS local seating, its answer
/// becomes DECIDED, the participant topology is frozen from that answer, the
/// session is built from that topology, and the claim is released when the
/// experience ends. A roster is the usual decider and not the only one — a
/// two-observer plaza declares its two channels with no lobby at all.
///
/// ⚠ **[`Self::Devices`] is a real answer, not a missing one.** A single-player
/// game, a headless oracle, a demo with no match — none of them declares seating
/// and all of them are correct to seat from what is plugged in. Declared seating
/// is opt-IN, which is what keeps the gate from stalling every composition that
/// never intended to publish anything.
///
/// ⚠ **every declared state names its OWNER.** A seat count with no owner
/// cannot be released by the experience that decided it: the previous version of
/// this was a bare `DecidedSeatCount(usize)` that the versus route inserted and
/// nothing ever removed, so the next experience's session was sized by a match
/// that had already ended.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub enum SessionSeatingSource {
    /// Nobody claimed local seating: freeze from connected devices.
    #[default]
    Devices,
    /// `owner` will publish its answer and has not yet. **The session does not
    /// start yet** — a topology frozen from devices here is one the answer is
    /// about to contradict, and the session is never resized afterwards.
    Pending { owner: String },
    /// `owner` decided `channels`. `frozen_topology` is stamped by
    /// the maintainer with the generation it actually captured, so the roster,
    /// the handle count and the per-seat latches cite one number rather than
    /// agreeing by coincidence.
    ///
    /// ⛔ **`seat_count: usize` was not enough, and this is the same lesson one
    /// layer up from [`crate::LocalSeatTopology`].** A count opens the
    /// right number of GGRS handles and says nothing about whose controller
    /// feeds each one, so every consumer re-derived the missing half from the
    /// roster's SPARSE source numbers — and a lobby that seats a CPU before a
    /// human produced a fighter on a channel the session never opened (GPT 5.6,
    /// 2026-08-07).
    Decided {
        owner: String,
        channels: crate::LocalChannelPlan,
        frozen_topology: Option<u64>,
    },
}

impl SessionSeatingSource {
    /// `owner` intends to decide local seating and has not yet.
    pub fn pending(owner: impl Into<String>) -> Self {
        Self::Pending {
            owner: owner.into(),
        }
    }

    /// `owner` decided which source drives which channel.
    pub fn decided(owner: impl Into<String>, channels: crate::LocalChannelPlan) -> Self {
        Self::Decided {
            owner: owner.into(),
            channels,
            frozen_topology: None,
        }
    }

    /// Which experience claimed local seating, if any.
    pub fn owner(&self) -> Option<&str> {
        match self {
            Self::Devices => None,
            Self::Pending { owner } | Self::Decided { owner, .. } => Some(owner),
        }
    }

    pub fn is_owned_by(&self, owner: &str) -> bool {
        self.owner() == Some(owner)
    }

    /// The decided channel plan, or `None` while seating is pending or
    /// device-driven.
    pub fn channel_plan(&self) -> Option<&crate::LocalChannelPlan> {
        match self {
            Self::Decided { channels, .. } => Some(channels),
            _ => None,
        }
    }

    /// The decided seat count, or `None` while seating is pending or device-driven.
    pub fn seat_count(&self) -> Option<usize> {
        self.channel_plan().map(|channels| channels.channels())
    }

    /// The topology generation the session was built from, once one was frozen.
    pub fn frozen_topology(&self) -> Option<u64> {
        match self {
            Self::Decided {
                frozen_topology, ..
            } => *frozen_topology,
            _ => None,
        }
    }

    /// **Give the claim back, if it is this owner's to give.**
    ///
    /// Returns whether anything was released. A stranger's claim is left alone:
    /// cleanup that reset the source unconditionally would be one experience
    /// deciding another's seating, which is the failure the owner exists to
    /// prevent.
    pub fn release(&mut self, owner: &str) -> bool {
        if !self.is_owned_by(owner) {
            return false;
        }
        *self = Self::Devices;
        true
    }
}

/// **WHAT A LIVE SURFACE IS OFFERING ABOUT LOCAL INPUT, AND WHOSE OFFER IT IS.**
///
/// ⛔⛔ **this replaces `DeclaredInputSeats` and the `InputAssignmentPolicy`
/// RESOURCE, and it replaces them because releasing them was written as VALUE
/// EQUALITY.** TwinTrack claimed two seats and a couch policy, and gave them
/// back like this:
///
/// ```text
/// if seats == 2      { seats = 0 }
/// if policy == couch { policy = default }
/// ```
///
/// A successor route that independently claims exactly those same values is
/// therefore erased by the previous owner's teardown — a lifecycle bug that no
/// test written against a successor claiming DIFFERENT values can see, and the
/// existing one claimed four seats. ⚠ the shape was already correct one type
/// away: [`SessionSeatingSource::release`] asks *is this claim mine* and leaves
/// a stranger's alone. Two resources that were always claimed together and
/// released together are now one that carries an owner.
///
/// ⚠ **claiming is not conditional on the value differing.** Taking over an
/// offer that already reads the way you want it still makes you the owner —
/// otherwise the previous owner's teardown reaches your claim, which is the same
/// bug from the other end.
///
/// ⭐ **why this is NOT folded into [`SessionSeatingSource`]:** they answer at
/// two different horizons. An offer is what a live SURFACE is showing — a lobby
/// with its panels up, an experience that is two-player by construction — and it
/// comes and goes with that surface. Seating source is what a SESSION was built
/// from, and it freezes. `frozen_topology` is meaningless for an offer, and a
/// surface that raises and drops an offer four times must not be four sessions.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalSeatOffer {
    owner: Option<String>,
    seats: u8,
    policy: crate::sources::InputAssignmentPolicy,
}

impl LocalSeatOffer {
    /// `owner` offers `seats` local seats under `policy`, taking the claim over
    /// from whoever held it.
    pub fn offered(
        owner: impl Into<String>,
        seats: u8,
        policy: crate::sources::InputAssignmentPolicy,
    ) -> Self {
        Self {
            owner: Some(owner.into()),
            seats,
            policy,
        }
    }

    /// **How many local seats are on offer.** `0` — the default — means nothing
    /// is offering any, which is every single-participant route.
    ///
    /// ⚠ **it is a COUNT, and a count can only say "seats 0..n, densely".** A
    /// session whose people are not on the first n sources — somebody on the
    /// keyboard below somebody on a pad — needs a [`crate::LocalChannelPlan`],
    /// which this cannot express and must not be stretched to.
    pub fn seats(&self) -> u8 {
        self.seats
    }

    /// How local sources become participants while this offer stands. An
    /// unclaimed offer answers with the default, which is today's solo
    /// behaviour exactly.
    pub fn policy(&self) -> crate::sources::InputAssignmentPolicy {
        self.policy
    }

    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    pub fn is_owned_by(&self, owner: &str) -> bool {
        self.owner() == Some(owner)
    }

    /// **Take the offer over**, whatever it currently says and whoever holds it.
    pub fn claim(&mut self, owner: &str, seats: u8, policy: crate::sources::InputAssignmentPolicy) {
        *self = Self::offered(owner, seats, policy);
    }

    /// **Withdraw the offer, if it is this owner's to withdraw.**
    ///
    /// Returns whether anything was withdrawn. A stranger's offer is left
    /// standing: cleanup that reset it unconditionally would be one surface
    /// retiring another's seats, which is the failure the owner exists to
    /// prevent.
    pub fn release(&mut self, owner: &str) -> bool {
        if !self.is_owned_by(owner) {
            return false;
        }
        *self = Self::default();
        true
    }
}
