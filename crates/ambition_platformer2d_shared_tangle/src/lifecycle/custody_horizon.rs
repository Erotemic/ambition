//! The custody domain's share of the reset baseline: what each body was
//! carrying when the checkpoint committed.
//!
//! why this is not a second column in the occurrence ledger. That ledger
//! answers *what became of this authored occurrence* — it has a row per
//! occurrence and its `InCustody` variant says only "somebody has it", which is
//! all reconstruction needs to know to refuse minting a second one. The question
//! here is the other one: *which body carries which occurrence*, keyed by the
//! body. Putting it in the ledger would make every reader of "was this
//! suppressed?" able to reach a body's inventory, and would put the two
//! questions on one lifetime when they have different ones.
//!
//! and it is a SNAPSHOT, never an authority. The live authority on
//! custody is [`InCustodyOf`] on the occurrence, and it stays that way — a
//! second live table of the same relation is a fork, and a fork drifts. What
//! this holds is what that relation *was*, at one instant, expressed in
//! identities rather than in `Entity` handles because an `Entity` does not
//! survive the thing it names.
//!
//! ✔ inventory ownership is settled and this does not reopen it: the BODY owns
//! its inventory and capabilities. Recording custody by the custodian's
//! [`SimId`] is that ownership written down at a horizon boundary; participant
//! entitlement and possession-transfer policy are different facts with different
//! owners, and neither appears here.

use std::collections::BTreeMap;

use bevy::prelude::{MessageReader, Query, ResMut, Resource, With};

use super::{horizon::CheckpointCommitted, InCustodyOf, RoomScopedEntity};
use crate::sim_id::SimId;

/// Which occurrence each body was carrying at the last committed
/// checkpoint, both sides by stable identity.
///
/// a custodian without a [`SimId`] contributes no row, and that is correct
/// for a snapshot. It would be wrong for a live authority — dropping a row
/// there would lose the suppression that keeps a carried object from being
/// duplicated — but this value's only job is to say what a restore should put
/// back, and an unnameable hand is one a restore could not find again anyway.
/// The live ledger keeps suppressing it either way, because that leg reads
/// [`InCustodyOf`] and never asks who.
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

    /// Every remembered hand, occurrence first, in identity order.
    ///
    /// a restore has to ASK THE BASELINE what it is missing, and it cannot
    /// do that by walking the world. [`Self::custodian_of`] answers the
    /// question the live-object side asks — *does the checkpoint agree with
    /// where this object is now* — and it can only be asked about an object
    /// that still exists. An occurrence whose entity was destroyed while its
    /// room unloaded is invisible to every query in the world, so the only
    /// place its row can be found is here, by enumerating them.
    ///
    /// ordered by [`SimId`] and not by an archetype, because the walk drives
    /// spawns.
    pub fn rows(&self) -> impl Iterator<Item = (&SimId, &SimId)> {
        self.held.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// Adopt a set of remembered hands — the one road that writes this
    /// outside a [`CheckpointCommitted`].
    ///
    /// its single caller is a durable LOAD, for the reason
    /// `OccurrenceBaseline::adopt` states: a fresh process has no checkpoint
    /// history, so what the file remembered IS the baseline, and the shipped
    /// restore road then puts those hands back exactly as a death does.
    ///
    /// whole-value, never a row insert. "Nobody was carrying anything"
    /// has to be expressible, and a merge cannot say it.
    pub fn adopt(&mut self, held: BTreeMap<SimId, SimId>) {
        if self.held != held {
            self.held = held;
        }
    }

    /// The desync checksum for this baseline — both sides are identities, so
    /// it is entity-free without having to be made so.
    ///
    /// that is the payoff for keying on [`SimId`] rather than `Entity`: an
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

/// Who is carrying what RIGHT NOW, in the shape a baseline row takes.
///
/// A second hand-written copy of "which body holds which occurrence" would be a fork, and the
/// two would disagree the first time either learned about a new case — a custodian without a
/// `SimId`, say, which this function drops and a naive copy would not.
///
/// a `BTreeMap` and not the query's order. This value reaches roads that
/// despawn and spawn, and Bevy's iteration order is an archetype accident.
pub fn live_custody_rows(
    carried: &Query<(&SimId, &InCustodyOf), With<RoomScopedEntity>>,
    custodians: &Query<&SimId>,
) -> BTreeMap<SimId, SimId> {
    carried
        .iter()
        .filter_map(|(occurrence, custody)| {
            let custodian = custodians.get(custody.0).ok()?;
            Some((occurrence.clone(), custodian.clone()))
        })
        .collect()
}

/// Record custody at checkpoint commit, including an empty custody set.
pub fn capture_custody_baseline(
    mut commits: MessageReader<CheckpointCommitted>,
    carried: Query<(&SimId, &InCustodyOf), With<RoomScopedEntity>>,
    custodians: Query<&SimId>,
    baseline: Option<ResMut<CustodyBaseline>>,
) {
    // Drain commits even when the baseline resource is absent; events are frame-scoped.
    let committed = commits.read().count() > 0;
    let Some(mut baseline) = baseline else {
        return;
    };
    if !committed {
        return;
    }
    let held = live_custody_rows(&carried, &custodians);
    if baseline.held != held {
        baseline.held = held;
    }
}

/// Custody restoration stays in the item domain because it must update both
/// `InCustodyOf` and `HeldItem`, and may need authored item data to materialize a
/// missing object. This layer captures only durable identities and must not depend
/// on item/combat representation.
#[allow(dead_code)]
pub const fn retraction_needs_both_halves_of_a_fork() {}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    /// A world with the capture registered and the resources a host installs.
    fn horizon_world() -> App {
        let mut app = App::new();
        app.add_message::<CheckpointCommitted>()
            .init_resource::<CustodyBaseline>()
            .add_systems(Update, capture_custody_baseline);
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

    /// The capture names the HAND, not merely that somebody had it.
    ///
    /// that distinction is the whole reason this value exists: an
    /// `OccurrenceWhereabouts::InCustody` row already says "somebody has it",
    /// which is enough to stop the room minting a second one and not enough to
    /// put it back.
    #[test]
    fn a_capture_records_which_body_was_carrying_what() {
        let mut app = horizon_world();
        let hand = body(&mut app, 0);
        carried_by(&mut app, "key", hand);

        app.world_mut().write_message(CheckpointCommitted);
        app.update();

        assert_eq!(
            app.world()
                .resource::<CustodyBaseline>()
                .custodian_of(&SimId::placement("key")),
            Some(&SimId::player_slot(0)),
        );
    }

    /// A checkpoint with empty hands is a real baseline, not a missing one.
    #[test]
    fn committing_with_empty_hands_overwrites_the_earlier_checkpoints_hands() {
        let mut app = horizon_world();
        let hand = body(&mut app, 0);
        let early = carried_by(&mut app, "key", hand);

        app.world_mut().write_message(CheckpointCommitted);
        app.update();
        assert!(!app.world().resource::<CustodyBaseline>().is_empty());

        app.world_mut().entity_mut(early).remove::<InCustodyOf>();
        app.world_mut().write_message(CheckpointCommitted);
        app.update();
        assert!(
            app.world().resource::<CustodyBaseline>().is_empty(),
            "the second checkpoint saw empty hands and must say so"
        );
    }

    /// A custodian with no identity contributes no row.
    ///
    /// this documents a real edge rather than defending one: the conservative
    /// direction for a snapshot is to forget, because the alternative is a
    /// baseline claiming a hand a restore could never find. The live suppression
    /// leg is unaffected — it reads `InCustodyOf` and never asks who.
    #[test]
    fn an_unnameable_hand_leaves_no_baseline_row() {
        let mut app = horizon_world();
        let anonymous = app.world_mut().spawn_empty().id();
        carried_by(&mut app, "key", anonymous);

        app.world_mut().write_message(CheckpointCommitted);
        app.update();
        assert!(app.world().resource::<CustodyBaseline>().is_empty());
    }
}
