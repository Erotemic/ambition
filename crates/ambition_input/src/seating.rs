//! Where a session's local seats come from, and who decides.
//!
//! The number of players is an input fact. A lobby, a roster, or a
//! fixed two-player experience decides it; a backend never does. This
//! declaration lives here, not in the rollback backend, so every surface can
//! read it. If a session opens its handles from the device count, a declared
//! second seat gets a participant and a pad but its body does not move,
//! because the session is never resized.
//!
//! A roster is one possible decider. A plaza with no roster can also be
//! two-player. The type means: a decider claimed local seating, and this is
//! its answer.

use bevy::prelude::Resource;

/// Where this session's seats come from, whether they are decided yet, and
/// whose answer it is.
///
/// One value covers the whole chain: an experience claims local seating, its
/// answer becomes decided, the participant topology freezes from that answer,
/// the session builds from that topology, and the claim is released when the
/// experience ends. A roster is the usual decider but not the only one; a
/// two-observer plaza declares two channels with no lobby.
///
/// [`Self::Devices`] is a real answer. Single-player games, headless oracles,
/// and demos declare nothing and seat from connected devices. Declared seating
/// is opt-in, so compositions that never declare do not stall on the gate.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub enum SessionSeatingSource {
    /// Nobody claimed local seating: freeze from connected devices.
    #[default]
    Devices,
    /// `owner` will publish its answer and has not yet. The session does not
    /// start: a topology frozen from devices now could disagree with the
    /// answer, and the session is never resized.
    Pending { owner: String },
    /// `owner` decided `channels`. The maintainer stamps `frozen_topology` with
    /// the generation it captured, so the roster, the handle count, and the
    /// per-seat latches all cite one number.
    ///
    /// A seat count is not enough (see `LocalSeatTopology`). A count opens the
    /// right number of GGRS handles but does not say which controller feeds
    /// each. Consumers that derived that from the roster's sparse source
    /// numbers put a fighter on an unopened channel when a CPU was seated
    /// before a human.
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

    /// Give the claim back, if it is this owner's to give.
    ///
    /// Returns whether anything was released.
    pub fn release(&mut self, owner: &str) -> bool {
        if !self.is_owned_by(owner) {
            return false;
        }
        *self = Self::Devices;
        true
    }
}

/// Owner-scoped local-seat offer from the currently active surface.
///
/// Unlike [`SessionSeatingSource`], an offer follows surface lifetime and never
/// freezes session topology. Ownership prevents one surface from releasing
/// another surface's offer.
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

    /// How many local seats are on offer. `0` (the default) means no offer,
    /// which is every single-participant route.
    ///
    /// A count only means "seats 0..n, dense". When players are not on the
    /// first n sources (keyboard below a pad), use a
    /// [`crate::LocalChannelPlan`]; do not extend this field.
    pub fn seats(&self) -> u8 {
        self.seats
    }

    /// How local sources become participants while this offer stands. An
    /// unclaimed offer gives the default, which is solo behaviour.
    pub fn policy(&self) -> crate::sources::InputAssignmentPolicy {
        self.policy
    }

    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    pub fn is_owned_by(&self, owner: &str) -> bool {
        self.owner() == Some(owner)
    }

    /// Take the offer over, whatever it currently says and whoever holds it.
    pub fn claim(&mut self, owner: &str, seats: u8, policy: crate::sources::InputAssignmentPolicy) {
        *self = Self::offered(owner, seats, policy);
    }

    /// Withdraw the offer, if it is this owner's to withdraw.
    ///
    /// Returns whether anything was withdrawn.
    pub fn release(&mut self, owner: &str) -> bool {
        if !self.is_owned_by(owner) {
            return false;
        }
        *self = Self::default();
        true
    }
}
