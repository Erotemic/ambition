//! Temporary-control state: whether an autonomous actor is currently masked by a
//! transient controller (player possession or a mount), recorded by STABLE
//! [`SimId`] so it survives a snapshot rewind in both directions.
//!
//! The live `Brain` alone cannot answer "who controls this body across time": a
//! the seat is restored by no cursor (it is a no-op), and possession /
//! mount relationships were re-derived each frame from live components, so a
//! rollback that crossed a possess/release boundary left the body in the WRONG
//! control mode. This component is the durable fact reconciliation reads to
//! restore the control mode itself — not merely to avoid clobbering one that
//! happens to be live at restore time.
//!
//! It rides on the autonomous body (the possessed actor / the rider), alongside
//! its `BrainBinding` (`ambition_platformer2d::characters::actor::character_catalog`): the
//! binding says which autonomous source resumes when control ends, and this says
//! whether a controller is masking it right now. ⚠ Named rather than LINKED —
//! this crate does not depend on `ambition_characters`, and it should not start
//! doing so to satisfy a doc link.

use crate::sim_id::SimId;
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

/// Who is asking to control a body. A ROLE, not an entity and not a crate.
///
/// ⭐⭐ THE PRECEDENCE IS THIS ENUM'S ORDER AND NOWHERE ELSE, so "who wins" is one
/// readable line rather than a rule spread across the two domains that ask.
/// Possession outranks a ride: a player who takes a mounted NPC is driving it,
/// and the ride continues underneath.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ControlClaimant {
    /// A player has possessed this body.
    Possession,
    /// This body is riding a mount that grants control.
    Mount,
}

/// Every live claim on one body's control, and the winner they project to.
///
/// ⛔⛔ THIS EXISTS BECAUSE AN ENUM CANNOT REMEMBER WHAT IT OVERWROTE. Possession
/// wrote [`TemporaryControl::Player`] and `ambition_mount` wrote
/// [`TemporaryControl::Mounted`] / [`TemporaryControl::Autonomous`] into the same
/// field, independently, with no arbiter — so the second writer erased the first
/// claim rather than shadowing it, and neither release arm could ask whether the
/// other still wanted the body.
///
/// ⇒ The failure was not hypothetical and not latent. `pirate_sky_lookout` ships a
/// mounted rider and `carried_item_crosses_rooms.rs` possesses one. Let the mount
/// die under a live possession and the mount's death arm wrote `Autonomous` while
/// `PossessionState` still named that rider — after which
/// [`crate::markers::body_collects_on_touch`], which qualifies a possessed body as
/// a pickup collector by matching `Player`, quietly stopped qualifying it. The
/// player kept driving a body that had stopped picking things up.
///
/// ⚠ NAMED FIELDS RATHER THAN A COLLECTION, deliberately. A `Vec` of claims would
/// be the general answer and it buys nothing here: two claimants exist, the
/// precedence is total and known, and a fixed struct is trivially clonable for
/// rollback with no allocation and no ordering ambiguity. Adding a third claimant
/// is adding a field and an arm — which is a place the compiler makes you visit,
/// unlike a runtime list.
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

    /// The winning claim as a control mode. Deterministic: [`ControlClaimant`]'s
    /// order is the priority, and a claim never depends on the order the claims
    /// arrived in.
    pub fn effective(&self) -> TemporaryControl {
        if let Some(controller) = &self.possession {
            TemporaryControl::Player {
                controller: controller.clone(),
            }
        } else if let Some(mount) = &self.mount {
            TemporaryControl::Mounted {
                mount: mount.clone(),
            }
        } else {
            TemporaryControl::Autonomous
        }
    }
}

/// The tick's control modes have been projected from their claims.
///
/// Published so a domain that READS `TemporaryControl` can order after the
/// projection without naming it, and so the two domains that WRITE claims can
/// order before it without naming each other.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControlClaimsProjected;

/// Write each claimed body's winning control mode into [`TemporaryControl`].
///
/// ⚠ UNFILTERED BY `Changed`, ON PURPOSE. A rollback restores `ControlClaims` and
/// change detection does not necessarily fire for the restore, so a `Changed`
/// filter would leave the projection holding whatever the mispredicted frame
/// wrote. Recomputing every tick from the restored claims is what makes the
/// projection agree with the claims after a rewind — and the population is bodies
/// that somebody is claiming, which is a handful.
pub fn project_control_claims(
    mut commands: bevy::prelude::Commands,
    mut claimed: bevy::prelude::Query<(
        bevy::prelude::Entity,
        &ControlClaims,
        Option<&mut TemporaryControl>,
    )>,
) {
    for (entity, claims, current) in &mut claimed {
        let effective = claims.effective();
        match current {
            Some(mut control) => {
                // Compared before writing so an unchanged mode does not spend a
                // change-detection tick every frame on every claimed body.
                if *control != effective {
                    *control = effective;
                }
            }
            None => {
                commands.entity(entity).insert(effective);
            }
        }
    }
}

/// File a claim on `body` from a command context, creating [`ControlClaims`] if
/// this is the first claim.
///
/// ⭐ THE POINT OF THE HELPER is that both domains file claims with the SAME
/// statement. The old call sites each wrote a different `TemporaryControl` variant
/// with `.insert(...)`, which is exactly what made them look like two independent
/// facts rather than two claims on one.
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

    /// ⭐ THE CASE THE OLD CODE COULD NOT REPRESENT: both claims live at once.
    #[test]
    fn possession_wins_while_the_ride_is_still_claimed() {
        let mut claims = ControlClaims::default();
        claims.claim(ControlClaimant::Mount, shark());
        claims.claim(ControlClaimant::Possession, player());
        assert_eq!(
            claims.effective(),
            TemporaryControl::Player { controller: player() },
            "a player driving a mounted body is driving it"
        );
        assert!(
            claims.holds(ControlClaimant::Mount),
            "the ride is shadowed, not erased — this is the whole fix"
        );
    }

    /// ⛔⛔ THE PRODUCTION BUG, IN ONE ASSERTION. `possession.rs:305` used to write
    /// `Autonomous` here, announcing an autonomy the still-live ride had not
    /// agreed to.
    #[test]
    fn releasing_possession_reveals_the_ride_underneath() {
        let mut claims = ControlClaims::default();
        claims.claim(ControlClaimant::Mount, shark());
        claims.claim(ControlClaimant::Possession, player());
        claims.release(ControlClaimant::Possession);
        assert_eq!(
            claims.effective(),
            TemporaryControl::Mounted { mount: shark() },
            "the body is still riding; releasing the player does not dismount it"
        );
    }

    /// ⛔⛔ THE OTHER HALF, AND THE ONE WITH A PLAYER-VISIBLE CONSEQUENCE.
    /// `ambition_mount:1220` used to write `Autonomous` when the mount died, which
    /// dropped the possessed body out of
    /// [`crate::markers::body_collects_on_touch`] — the player kept driving a body
    /// that had silently stopped picking things up.
    #[test]
    fn a_dying_mount_leaves_the_player_in_control() {
        let mut claims = ControlClaims::default();
        claims.claim(ControlClaimant::Mount, shark());
        claims.claim(ControlClaimant::Possession, player());
        claims.release(ControlClaimant::Mount);
        let effective = claims.effective();
        assert_eq!(
            effective,
            TemporaryControl::Player { controller: player() },
            "the mount died; the player did not stop driving"
        );
        assert!(
            crate::markers::body_collects_on_touch(false, Some(&effective)),
            "a possessed body must still qualify as a pickup collector after its \
             mount dies — this is the exact consequence the old code lost"
        );
    }

    /// ⚠ ORDER-INDEPENDENCE, because the two domains file their claims from
    /// different schedules and nothing fixes which lands first. If the answer
    /// depended on arrival order the projection would be non-deterministic, and a
    /// non-deterministic control mode is a rollback desync.
    #[test]
    fn the_winner_does_not_depend_on_which_claim_arrived_first() {
        let mut mount_first = ControlClaims::default();
        mount_first.claim(ControlClaimant::Mount, shark());
        mount_first.claim(ControlClaimant::Possession, player());

        let mut possession_first = ControlClaims::default();
        possession_first.claim(ControlClaimant::Possession, player());
        possession_first.claim(ControlClaimant::Mount, shark());

        assert_eq!(mount_first, possession_first);
        assert_eq!(mount_first.effective(), possession_first.effective());
    }

    #[test]
    fn no_claims_is_autonomous_and_releasing_an_unheld_claim_is_a_no_op() {
        let mut claims = ControlClaims::default();
        assert!(claims.is_empty());
        assert_eq!(claims.effective(), TemporaryControl::Autonomous);
        claims.release(ControlClaimant::Mount);
        assert_eq!(claims.effective(), TemporaryControl::Autonomous);
    }

    /// ⭐ THE SHADOWED CLAIM MUST SURVIVE A REWIND. Restoring only the projection
    /// would come back with the ride forgotten.
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
