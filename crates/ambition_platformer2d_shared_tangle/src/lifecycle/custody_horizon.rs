//! **The custody domain's share of the reset baseline: what each body was
//! carrying when the checkpoint committed.**
//!
//! ⭐ **why this is not a second column in the occurrence ledger.** That ledger
//! answers *what became of this authored occurrence* — it has a row per
//! occurrence and its `InCustody` variant says only "somebody has it", which is
//! all reconstruction needs to know to refuse minting a second one. The question
//! here is the other one: *which body carries which occurrence*, keyed by the
//! body. Putting it in the ledger would make every reader of "was this
//! suppressed?" able to reach a body's inventory, and would put the two
//! questions on one lifetime when they have different ones.
//!
//! ⭐⭐ **and it is a SNAPSHOT, never an authority.** The live authority on
//! custody is [`InCustodyOf`] on the occurrence, and it stays that way — a
//! second live table of the same relation is a fork, and a fork drifts. What
//! this holds is what that relation *was*, at one instant, expressed in
//! identities rather than in `Entity` handles because an `Entity` does not
//! survive the thing it names.
//!
//! ✔ **inventory ownership is settled and this does not reopen it: the BODY owns
//! its inventory and capabilities.** Recording custody by the custodian's
//! [`SimId`] is that ownership written down at a horizon boundary; participant
//! entitlement and possession-transfer policy are different facts with different
//! owners, and neither appears here.

use std::collections::BTreeMap;

use bevy::prelude::{Commands, Entity, MessageReader, Query, Res, ResMut, Resource, With};

use super::{
    horizon::{CheckpointCommitted, ResetToCheckpoint},
    InCustodyOf, RoomScopedEntity,
};
use crate::sim_id::SimId;

/// **Which occurrence each body was carrying at the last committed
/// checkpoint**, both sides by stable identity.
///
/// ⚠ **a custodian without a [`SimId`] contributes no row, and that is correct
/// for a snapshot.** It would be wrong for a live authority — dropping a row
/// there would lose the suppression that keeps a carried object from being
/// duplicated — but this value's only job is to say what a restore should put
/// back, and an unnameable hand is one a restore could not find again anyway.
/// The live ledger keeps suppressing it either way, because that leg reads
/// [`InCustodyOf`] and never asks who.
///
/// ⛔⛔ **rollback state with a real VALUE, not a derived projection.** Nothing
/// republishes a baseline from live state, and a checkpoint commits mid-frame,
/// so a rewind across the commit must restore this or the world keeps a baseline
/// from a future that was un-happened.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct CustodyBaseline {
    /// occurrence → the body that was carrying it.
    ///
    /// Keyed by the occurrence because that is the direction every question
    /// asks: *should this thing still be in a hand?* A body-keyed map would need
    /// a scan to answer it, and a body carries at most a hand's worth.
    held: BTreeMap<SimId, SimId>,
}

impl CustodyBaseline {
    /// The body that was carrying `occurrence` at the checkpoint, if any.
    pub fn custodian_of(&self, occurrence: &SimId) -> Option<&SimId> {
        self.held.get(occurrence)
    }

    /// Whether the checkpoint remembers this occurrence in somebody's hands.
    pub fn was_carried(&self, occurrence: &SimId) -> bool {
        self.held.contains_key(occurrence)
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// **The desync checksum for this baseline** — both sides are identities, so
    /// it is entity-free without having to be made so.
    ///
    /// ⭐ that is the payoff for keying on [`SimId`] rather than `Entity`: an
    /// `Entity` handle would need a mapping pass to be comparable between peers
    /// at all, and a checksum over one would be noise.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_str, put_u64};
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.held.len() as u64);
        // `BTreeMap`, so this walk is ordered by identity on every peer.
        for (occurrence, custodian) in &self.held {
            put_str(&mut bytes, occurrence.as_str());
            put_str(&mut bytes, custodian.as_str());
        }
        checksum_bytes(&bytes)
    }
}

/// **Record who was carrying what, at the instant a checkpoint commits.**
///
/// ⚠ **an empty capture is a real answer and is written.** "Nothing was being
/// carried at this checkpoint" is exactly what makes a later death take a
/// picked-up object back off the player; skipping the write would leave an
/// older checkpoint's hands in place.
pub fn capture_custody_baseline(
    mut commits: MessageReader<CheckpointCommitted>,
    carried: Query<(&SimId, &InCustodyOf), With<RoomScopedEntity>>,
    custodians: Query<&SimId>,
    baseline: Option<ResMut<CustodyBaseline>>,
) {
    // Drained unconditionally, like every other reader of this channel: a commit
    // seen during a load must not be re-read against a world that has moved on.
    let committed = commits.read().count() > 0;
    let Some(mut baseline) = baseline else {
        return;
    };
    if !committed {
        return;
    }
    // A `BTreeMap` and not the query's order: this value reaches a restore that
    // despawns entities, and Bevy's iteration order is an archetype accident.
    let held: BTreeMap<SimId, SimId> = carried
        .iter()
        .filter_map(|(occurrence, custody)| {
            let custodian = custodians.get(custody.0).ok()?;
            Some((occurrence.clone(), custodian.clone()))
        })
        .collect();
    if baseline.held != held {
        baseline.held = held;
    }
}

/// **Take back what was picked up after the checkpoint.**
///
/// ⭐⭐ **the retraction is a DESPAWN, and that is the point rather than a
/// shortcut.** An authored occurrence's identity lives in the record that minted
/// it, not in the entity: destroying the entity and letting the rebuild author
/// it again from the record produces the *same* `SimId` at the *authored*
/// position, which is precisely "the key went back on its pedestal". Moving the
/// live entity back instead would need this system to know where the record puts
/// it — a second answer to a question `RoomOccurrenceOutlook` already owns.
///
/// ⚠ **and the rebuild reaches the right answer without being told.** The
/// occurrence ledger has been restored to the baseline by then, and a baseline
/// that never saw this object has no row for it — so its disposition is
/// `Authored`, as written. ⛔ this system therefore must NOT also clear the
/// ledger row: two systems retracting one fact is how a retraction survives one
/// of them being deleted and stops working anyway.
///
/// ⚠ **an occurrence the baseline says WAS carried is left strictly alone**,
/// including when it is now in a different body's hands. Moving it back would
/// need a road that mints an occurrence directly into custody, which does not
/// exist; the gap is named at `AuthoredOccurrences::baseline_is_a_copy_of_this`
/// and is unreachable while a held occurrence is resident in no room.
pub fn retract_custody_to_checkpoint(
    mut commands: Commands,
    mut resets: MessageReader<ResetToCheckpoint>,
    baseline: Option<Res<CustodyBaseline>>,
    carried: Query<(Entity, &SimId), (With<InCustodyOf>, With<RoomScopedEntity>)>,
) {
    let requested = resets.read().count() > 0;
    let Some(baseline) = baseline else {
        return;
    };
    if !requested {
        return;
    }
    for (entity, occurrence) in &carried {
        if baseline.was_carried(occurrence) {
            continue;
        }
        super::despawn_scoped_entity(&mut commands, entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    /// A world with the two systems registered and the resources a host installs.
    fn horizon_world() -> App {
        let mut app = App::new();
        app.add_message::<CheckpointCommitted>()
            .add_message::<ResetToCheckpoint>()
            .init_resource::<CustodyBaseline>()
            .add_systems(
                Update,
                (capture_custody_baseline, retract_custody_to_checkpoint).chain(),
            );
        app
    }

    fn body(app: &mut App, slot: u8) -> Entity {
        app.world_mut().spawn(SimId::player_slot(slot)).id()
    }

    fn carried_by(app: &mut App, placement: &str, holder: Entity) -> Entity {
        app.world_mut()
            .spawn((
                SimId::placement(placement),
                InCustodyOf(holder),
                RoomScopedEntity,
            ))
            .id()
    }

    /// **The whole rule in one world: what the checkpoint saw stays, what it did
    /// not see goes back.**
    ///
    /// ⭐ **both halves are asserted from ONE reset**, because either alone
    /// passes for the wrong reason — a system that despawns nothing satisfies
    /// "the committed one survived", and a system that despawns everything
    /// satisfies "the uncommitted one was taken back".
    #[test]
    fn a_reset_keeps_what_the_checkpoint_saw_and_takes_back_what_it_did_not() {
        let mut app = horizon_world();
        let hand = body(&mut app, 0);
        let committed = carried_by(&mut app, "key", hand);

        app.world_mut().write_message(CheckpointCommitted);
        app.update();

        assert_eq!(
            app.world().resource::<CustodyBaseline>().custodian_of(&SimId::placement("key")),
            Some(&SimId::player_slot(0)),
            "the capture must name the hand, not merely that somebody had it"
        );

        // Picked up AFTER the checkpoint: no row, so a reset owes it back.
        let uncommitted = carried_by(&mut app, "torch", hand);
        app.world_mut().write_message(ResetToCheckpoint);
        app.update();

        assert!(
            app.world().get_entity(committed).is_ok(),
            "an occurrence the checkpoint saw in a hand must survive the reset"
        );
        assert!(
            app.world().get_entity(uncommitted).is_err(),
            "an occurrence acquired after the checkpoint must be taken back, so the \
             rebuild can author it at its record's position"
        );
    }

    /// **A checkpoint with empty hands is a real baseline, not a missing one.**
    ///
    /// ⛔ the failure this pins is a capture that skips the write when nothing is
    /// carried: the previous checkpoint's hands stay in the baseline, and a
    /// death then returns an object the player had already legitimately given up.
    #[test]
    fn committing_with_empty_hands_overwrites_the_earlier_checkpoints_hands() {
        let mut app = horizon_world();
        let hand = body(&mut app, 0);
        let early = carried_by(&mut app, "key", hand);

        app.world_mut().write_message(CheckpointCommitted);
        app.update();
        assert!(!app.world().resource::<CustodyBaseline>().is_empty());

        // The object leaves the hand, and a SECOND checkpoint commits.
        app.world_mut().entity_mut(early).remove::<InCustodyOf>();
        app.world_mut().write_message(CheckpointCommitted);
        app.update();
        assert!(
            app.world().resource::<CustodyBaseline>().is_empty(),
            "the second checkpoint saw empty hands and must say so"
        );

        // Picked up again after C1, then a death.
        app.world_mut().entity_mut(early).insert(InCustodyOf(hand));
        app.world_mut().write_message(ResetToCheckpoint);
        app.update();
        assert!(
            app.world().get_entity(early).is_err(),
            "C1 saw empty hands, so the reset owes this object back to the world"
        );
    }

    /// **A custodian with no identity contributes no row — and the reset then
    /// takes the object back rather than silently keeping it.**
    ///
    /// ⚠ this documents a real edge rather than defending one: the conservative
    /// direction for a snapshot is to forget, because the alternative is a
    /// baseline claiming a hand a restore could never find.
    #[test]
    fn an_unnameable_hand_leaves_no_baseline_row() {
        let mut app = horizon_world();
        let anonymous = app.world_mut().spawn_empty().id();
        let held = carried_by(&mut app, "key", anonymous);

        app.world_mut().write_message(CheckpointCommitted);
        app.update();
        assert!(app.world().resource::<CustodyBaseline>().is_empty());

        app.world_mut().write_message(ResetToCheckpoint);
        app.update();
        assert!(app.world().get_entity(held).is_err());
    }
}
