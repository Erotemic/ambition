//! Temporary-control claims: which transient controllers (player possession, a
//! mount) currently hold an actor's control, recorded by STABLE [`SimId`] so the
//! claims survive a snapshot rewind in both directions.
//!
//! The claims ride on the autonomous body beside its `BrainBinding`: the binding
//! says which autonomous source resumes when control ends, and these say who is
//! masking it right now. There is no stored winner — a reader asks the claim it
//! cares about, so no projection can lag or disagree with the claims.

use crate::sim_id::SimId;
use bevy::prelude::Component;

/// Who is asking to control a body. A ROLE, not an entity and not a crate.
///
/// Possession outranks a ride: a player who takes a mounted NPC is driving it,
/// and the ride continues underneath.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ControlClaimant {
    /// A player has possessed this body.
    Possession,
    /// This body is riding a mount that grants control.
    Mount,
}

/// Every live claim on one body's control.
///
/// Claims are kept side by side rather than as one winning mode because two
/// domains file them independently: a single slot let a dying mount overwrite a
/// live possession, so the possessed body stopped qualifying as a pickup
/// collector while the player kept driving it. Named fields rather than a
/// collection: two claimants, clonable for rollback with no allocation.
#[derive(Component, Clone, Debug, PartialEq, Eq, Default)]
pub struct ControlClaims {
    possession: Option<SimId>,
    mount: Option<SimId>,
}

impl ControlClaims {
    /// File a claim, replacing any previous claim by the same claimant.
    pub fn claim(&mut self, claimant: ControlClaimant, subject: SimId) {
        match claimant {
            ControlClaimant::Possession => self.possession = Some(subject),
            ControlClaimant::Mount => self.mount = Some(subject),
        }
    }

    /// Drop one claimant's claim. ⭐ THE WHOLE POINT: whatever else still holds
    /// the body becomes the winner again, because it was never overwritten.
    pub fn release(&mut self, claimant: ControlClaimant) {
        match claimant {
            ControlClaimant::Possession => self.possession = None,
            ControlClaimant::Mount => self.mount = None,
        }
    }

    /// Whether this claimant currently holds a claim — live or shadowed.
    pub fn holds(&self, claimant: ControlClaimant) -> bool {
        match claimant {
            ControlClaimant::Possession => self.possession.is_some(),
            ControlClaimant::Mount => self.mount.is_some(),
        }
    }

    /// Build from parts — the snapshot decoder's entry point.
    pub fn from_parts(possession: Option<SimId>, mount: Option<SimId>) -> Self {
        Self { possession, mount }
    }

    /// The possessing controller, if any — live or shadowed.
    pub fn possession(&self) -> Option<&SimId> {
        self.possession.as_ref()
    }

    /// The mount being ridden, if any — live or shadowed.
    pub fn mount(&self) -> Option<&SimId> {
        self.mount.as_ref()
    }

    /// True when nobody claims this body.
    pub fn is_empty(&self) -> bool {
        self.possession.is_none() && self.mount.is_none()
    }
}

/// File a claim on `body` from a command context, creating [`ControlClaims`] if
/// this is the first claim.
///
/// Both domains file claims through this one statement, so neither can
/// overwrite the other's.
pub fn file_claim(
    commands: &mut bevy::prelude::Commands,
    body: bevy::prelude::Entity,
    claimant: ControlClaimant,
    subject: SimId,
) {
    if let Ok(mut entity) = commands.get_entity(body) {
        entity
            .entry::<ControlClaims>()
            .or_default()
            .and_modify(move |mut claims| claims.claim(claimant, subject.clone()));
    }
}

/// Drop one claimant's claim on `body`. A body with no [`ControlClaims`] has
/// nothing to release, which is the correct no-op rather than an error.
pub fn drop_claim(
    commands: &mut bevy::prelude::Commands,
    body: bevy::prelude::Entity,
    claimant: ControlClaimant,
) {
    if let Ok(mut entity) = commands.get_entity(body) {
        entity
            .entry::<ControlClaims>()
            .and_modify(move |mut claims| claims.release(claimant));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player() -> SimId {
        SimId::player_slot(0)
    }

    fn shark() -> SimId {
        SimId::from_snapshot("mount.shark".to_string())
    }

    #[test]
    fn possession_and_a_ride_are_held_at_once() {
        let mut claims = ControlClaims::default();
        claims.claim(ControlClaimant::Mount, shark());
        claims.claim(ControlClaimant::Possession, player());
        assert!(claims.holds(ControlClaimant::Possession));
        assert!(
            claims.holds(ControlClaimant::Mount),
            "the ride is shadowed, not erased"
        );
    }

    #[test]
    fn releasing_possession_leaves_the_ride_underneath() {
        let mut claims = ControlClaims::default();
        claims.claim(ControlClaimant::Mount, shark());
        claims.claim(ControlClaimant::Possession, player());
        claims.release(ControlClaimant::Possession);
        assert_eq!(claims, ControlClaims::from_parts(None, Some(shark())));
    }

    /// A dying mount must not drop the possessed body out of
    /// [`crate::markers::body_collects_on_touch`].
    #[test]
    fn a_dying_mount_leaves_the_player_in_control() {
        let mut claims = ControlClaims::default();
        claims.claim(ControlClaimant::Mount, shark());
        claims.claim(ControlClaimant::Possession, player());
        claims.release(ControlClaimant::Mount);
        assert!(
            crate::markers::body_collects_on_touch(false, Some(&claims)),
            "a possessed body must still qualify as a pickup collector after its \
             mount dies"
        );
    }

    /// The two domains file from different schedules; the claim set must not
    /// depend on which landed first, or it desyncs.
    #[test]
    fn the_claims_do_not_depend_on_which_arrived_first() {
        let mut mount_first = ControlClaims::default();
        mount_first.claim(ControlClaimant::Mount, shark());
        mount_first.claim(ControlClaimant::Possession, player());

        let mut possession_first = ControlClaims::default();
        possession_first.claim(ControlClaimant::Possession, player());
        possession_first.claim(ControlClaimant::Mount, shark());

        assert_eq!(mount_first, possession_first);
    }

    #[test]
    fn releasing_an_unheld_claim_is_a_no_op() {
        let mut claims = ControlClaims::default();
        assert!(claims.is_empty());
        claims.release(ControlClaimant::Mount);
        assert!(claims.is_empty());
    }

    /// A shadowed claim must survive a rewind.
    #[test]
    fn a_shadowed_claim_survives_the_snapshot_codec() {
        use ambition_platformer2d_core::snapshot::{Reader, SnapshotState};
        let mut claims = ControlClaims::default();
        claims.claim(ControlClaimant::Mount, shark());
        claims.claim(ControlClaimant::Possession, player());
        let mut bytes = Vec::new();
        claims.encode(&mut bytes);
        let mut reader = Reader::new(&bytes);
        let restored = ControlClaims::decode(&mut reader).expect("round-trips");
        assert_eq!(restored, claims);
        assert!(
            restored.holds(ControlClaimant::Mount),
            "the shadowed ride came back"
        );
    }
}
