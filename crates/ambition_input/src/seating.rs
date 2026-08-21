//! **WHERE A SESSION'S LOCAL SEATS COME FROM, and whose answer that is.**
//!
//! ⛔⛔ **this lived in the GGRS backend until 2026-08-20, and that placement WAS
//! a bug.** How many people are playing is a fact about INPUT — it is decided by
//! a lobby, a roster, or an experience that simply is two-player — and a host
//! merely consumes it. Keeping the declaration inside `ambition_platformer2d_rollback_ggrs`
//! meant only a rollback host could hear one, so a surface that wanted to say
//! *"two seats, always"* had to reach into a backend to say it, and any surface
//! that did not reach in was silently seated from connected DEVICES.
//!
//! Measured: TwinTrack declared two seats and a couch policy through
//! [`crate::DeclaredInputSeats`], got its second `InputParticipant`, got the only
//! pad assigned to it — and the laboratory twin never moved, because the GGRS
//! session had already opened ONE handle from the device count and is never
//! resized. Every headless test passed; the shipped binary is the rollback host.
//! (Jon, 2026-08-20: *"in twin track I still cannot control emmy with the game
//! pad."*)
//!
//! ⚠ **the vocabulary moved with it.** The variants were `RosterPending` and
//! `RosterDecided`, and a roster is one decider among several — TwinTrack's
//! plaza has no roster and is still two-player by construction. What the type
//! always meant is *somebody has claimed local seating and here is their answer*.

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
