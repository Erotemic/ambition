//! Temporary-control state: whether an autonomous actor is currently masked by a
//! transient controller (player possession or a mount), recorded by STABLE
//! [`SimId`] so it survives a snapshot rewind in both directions.
//!
//! The live `Brain` alone cannot answer "who controls this body across time": a
//! `Brain::Player` is restored by no cursor (it is a no-op), and possession /
//! mount relationships were re-derived each frame from live components, so a
//! rollback that crossed a possess/release boundary left the body in the WRONG
//! control mode. This component is the durable fact reconciliation reads to
//! restore the control mode itself — not merely to avoid clobbering one that
//! happens to be live at restore time.
//!
//! It rides on the autonomous body (the possessed actor / the rider), alongside
//! its [`BrainBinding`](ambition_characters::actor::character_catalog::BrainBinding):
//! the binding says which autonomous source resumes when control ends, and this
//! says whether a controller is masking it right now.

use ambition_platformer_primitives::sim_id::SimId;
use bevy::prelude::Component;

/// Which transient controller (if any) is masking an actor's autonomous brain.
///
/// `Default` is [`Autonomous`](Self::Autonomous) — the body runs its own brain.
/// The controller / mount is named by [`SimId`] (never a raw `Entity`), so a
/// restore rebuilds the live relationship from the stable id.
#[derive(Component, Clone, Debug, PartialEq, Eq, Default)]
pub enum TemporaryControl {
    /// No controller — the body runs its autonomous brain (its `BrainBinding`
    /// source).
    #[default]
    Autonomous,
    /// Player-possessed: the controlling home avatar is `controller`, whose
    /// player brain was vacated onto this body.
    Player { controller: SimId },
    /// Mounted: this rider's brain is the mount-cached brain while riding
    /// `mount`.
    Mounted { mount: SimId },
}

impl TemporaryControl {
    /// True iff the body runs its own autonomous brain (no controller masking).
    pub fn is_autonomous(&self) -> bool {
        matches!(self, Self::Autonomous)
    }

    /// True iff a transient controller (player / mount) currently masks the
    /// autonomous brain.
    pub fn is_controlled(&self) -> bool {
        !self.is_autonomous()
    }
}
