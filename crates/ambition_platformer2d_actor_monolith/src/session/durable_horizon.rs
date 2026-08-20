//! **THE DURABLE SAVE HORIZON — what survives closing the program, for the
//! occurrences the world remembers anything about.**
//!
//! ```text
//! 1  current world truth     AuthoredOccurrences + ItemCustody          ✔
//! 2  checkpoint truth        occurrence/custody/minted descriptions +    ✔
//!                            OwnedItemsBaseline, restored on a death
//! 3  durable save truth      ← THIS MODULE
//! ```
//!
//! # ⭐⭐ THE ON-DISK FORM IS THE CHECKPOINT'S OWN DESCRIPTION, SERIALIZED
//!
//! Not a fourth description of the same facts. The occurrence half of a
//! checkpoint is exactly the three occurrence lists this writes, field for field;
//! the sibling item leg persists `OwnedItemsBaseline`'s quantity state through
//! `save.items` rather than pretending a quantity is an occurrence:
//!
//! ```text
//! AuthoredOccurrences   → save.occurrences   SimId + InCustody | Placed{room,at} | Consumed
//! CustodyBaseline       → save.custody       occurrence SimId → custodian SimId
//! MintedItemBaseline    → save.minted_items  SimId + SpawnOrigin::Dynamic + spec id
//! ```
//!
//! ⭐ **and that is a measurement rather than a preference.** The minimal durable
//! description of a runtime-minted occurrence was settled by the checkpoint slice
//! as `identity + provenance + definition-REFERENCE` and nothing else; asking the
//! same question of a file produced the same three fields, because the question is
//! the same one — *how would you make this again?* A second on-disk vocabulary
//! would have been a second answer to it.
//!
//! ⛔ **NO COMPONENT SNAPSHOTS.** That is rollback wearing save's clothes: it
//! welds the file format to ECS layout, so a component split renames a player's
//! progress. What reaches disk is what an occurrence IS and WHERE it is; what it
//! is made of comes back from the authored record, or from the item catalog by
//! reference.
//!
//! # ⭐⭐ A LOAD IS A CHECKPOINT RESUME
//!
//! The lifecycle adopter and item-domain restore install their own values. A
//! final completion system then raises [`SaveRestored`] and writes exactly one
//! [`ResetToCheckpoint`]. Everything after that is the road a DEATH already
//! takes, unchanged:
//!
//! ```text
//! restore_occurrence_baseline      puts the ledger back
//! restore_custody_to_checkpoint    puts the hands back, MATERIALIZING what has no entity
//! resume_at_checkpoint_on_reset    rebuilds the room from the restored ledger
//! ```
//!
//! ⭐ **that is why this module is ~two systems rather than a reconstruction
//! engine.** The hard half — suppress the home room, reinstate where the object
//! lies, materialize a hand's occurrence from a record or from a description —
//! was built for horizon 2 and needed nothing added for horizon 3. The durable
//! horizon's whole job turned out to be *stating the same three values in a form
//! that outlives the process*.
//!
//! ⛔ **so a load must NOT also restore occurrences by some second road.** There
//! is exactly one reconstruction authority and it is the ledger; this module
//! writes the ledger and asks the shipped restore to run.
//!
//! # ⚠ WHY THE LOADED STATE BECOMES THE BASELINE
//!
//! A fresh process has no checkpoint history. Leaving the empty default baseline
//! in place would make the FIRST death after a load take back everything the file
//! remembered — the mirror image of the bug the horizon exists to prevent. So the
//! load adopts what it read as the baseline, which is the same degenerate-case
//! reasoning the sandbox reset uses from the other side: a host that never
//! commits restores the empty baseline, and a process that never committed *this
//! session* restores what it was given.
//!
//! ⚠ **stated rather than defended: the body and the objects then resume from
//! different instants.** `PersistedCheckpoint` is written by a shrine, so the
//! BODY comes back at the last shrine, while these rows are current truth at the
//! moment the autosave ran. For a first slice that is strictly better than the
//! alternative (objects evaporating), and it is the same seam `OwnedItems` has
//! sat outside since D132.
//!
//! # ⚠ WHAT THIS STILL DOES NOT COVER
//!
//! * ✔ **a runtime mint NOT in a hand is COVERED as of 2026-08-19**, and the
//!   reason this list gave was wrong twice over. It said the mint "remembers no
//!   position" — it does not need to: an `OccurrenceWhereabouts::Placed { room,
//!   at }` row records exactly where a resting occurrence lies, written for every
//!   in-world item. What actually lost it was `live_minted_descriptions`
//!   refusing anything `InWorld`, so the world knew WHERE and had no way to make
//!   it again. The describer no longer filters, and the room build settles the
//!   reinstatement debt from the checkpoint's description.
//!   ⚠ **in FLIGHT is still not covered, and that half is by design**: the
//!   ledger tracks only occurrences it already `remembers` — things somebody
//!   CARRIED — and `record_placed_ground_items` refuses to become the universal
//!   instance registry that tracking everything would require;
//! * ✔ `OwnedItems` remains a QUANTITY table, but its checkpoint baseline is
//!   now adopted by the same item-domain restore that applies the saved bag,
//!   before the global restore latch rises;
//! * `OccurrenceWhereabouts::Consumed` round-trips through the file and has no
//!   live producer, so nothing yet WRITES a terminal row.

use std::collections::BTreeMap;

use bevy::prelude::*;

use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::{PersistedCustody, PersistedOccurrence, PersistedWhereabouts};
use ambition_platformer2d_shared_tangle::lifecycle::{
    live_custody_rows, AuthoredOccurrences, CustodyBaseline, InCustodyOf, OccurrenceBaseline,
    OccurrenceWhereabouts, ResetToCheckpoint, RoomScopedEntity,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// **Has the loaded save been applied to this world yet?**
///
/// ⭐ **ONE latch for the whole durable restore, and that is the point of the
/// name.** It used to be `InventoryRestored` and gate only the catalog + wallet;
/// the occurrence leg asks the same question — *has the file been applied?* — and
/// a second flag would be a second answer to it, free to disagree the first time
/// one leg's precondition was met and the other's was not.
///
/// ⛔ **ROLLBACK STATE.** This is an "already applied" flag that GATES behaviour,
/// not a cache, and everything it coordinates rewinds: `OwnedItems`,
/// `BodyWallet`, `AmbitionGameSave` and the occurrence ledger are all in the
/// schema. Left un-rewound it survived a rewind that undid the restore,
/// `restore_inventory_from_save` then returned early forever, and
/// `persist_inventory_to_save` — gated on this being TRUE — wrote the starter set
/// over the loaded save.
#[derive(Resource, Default, Clone)]
pub struct SaveRestored(pub bool);

/// Adopt the lifecycle/occurrence domain from a loaded file.
///
/// This is deliberately one domain adapter rather than one parameter per
/// checkpoint baseline in a global census. Item-domain baselines are adopted by
/// `items::persist::restore_inventory_from_save`, beside the item state the file
/// actually restores.
pub fn adopt_occurrence_checkpoint_from_save(
    restored: Res<SaveRestored>,
    save: Res<AmbitionGameSave>,
    bodies: Query<(), crate::actor::PrimaryPlayerOnly>,
    occurrences: Option<ResMut<AuthoredOccurrences>>,
    occurrence_baseline: Option<ResMut<OccurrenceBaseline>>,
    custody_baseline: Option<ResMut<CustodyBaseline>>,
) {
    if restored.0 || bodies.is_empty() {
        return;
    }
    let Some(mut occurrences) = occurrences else {
        return;
    };
    let data = save.data();

    let ledger_rows: BTreeMap<SimId, OccurrenceWhereabouts> = data
        .occurrences
        .iter()
        .map(|row| {
            (
                SimId::from_snapshot(row.id.clone()),
                match &row.whereabouts {
                    PersistedWhereabouts::InCustody => OccurrenceWhereabouts::InCustody,
                    PersistedWhereabouts::Placed { room, x, y } => OccurrenceWhereabouts::Placed {
                        room: room.clone(),
                        at: Vec2::new(*x as f32, *y as f32),
                    },
                    PersistedWhereabouts::Consumed => OccurrenceWhereabouts::Consumed,
                },
            )
        })
        .collect();
    let held: BTreeMap<SimId, SimId> = data
        .custody
        .iter()
        .map(|row| {
            (
                SimId::from_snapshot(row.occurrence.clone()),
                SimId::from_snapshot(row.custodian.clone()),
            )
        })
        .collect();

    occurrences.adopt_rows(ledger_rows);
    if let Some(mut baseline) = occurrence_baseline {
        baseline.adopt(occurrences.clone());
    }
    if let Some(mut baseline) = custody_baseline {
        baseline.adopt(held);
    }
}

/// Mark durable adoption complete and request the ordinary checkpoint resume.
///
/// This is the only global completion point. Domain adopters run before it and
/// touch only their own state; this system states that every adopter in the
/// chain has had its turn. Keeping the request here prevents an item or lifecycle
/// domain from becoming the coordinator for its siblings.
pub fn complete_durable_restore(
    mut restored: ResMut<SaveRestored>,
    save: Res<AmbitionGameSave>,
    ready_body: Query<&ambition_characters::actor::BodyWallet, crate::actor::PrimaryPlayerOnly>,
    mut resets: MessageWriter<ResetToCheckpoint>,
) {
    if restored.0 || ready_body.single().is_err() {
        return;
    }
    restored.0 = true;
    let data = save.data();
    if !data.occurrences.is_empty() || !data.custody.is_empty() || !data.minted_items.is_empty() {
        resets.write(ResetToCheckpoint);
    }
}

/// Install the complete durable-save application/mirroring chain owned by the
/// actor integration layer.
///
/// The generic runtime calls this one domain offer. It no longer enumerates
/// concrete durable systems or checkpoint baselines.
pub fn install_durable_save_horizon(app: &mut App) {
    app.init_resource::<SaveRestored>().add_systems(
        Update,
        (
            // Lifecycle state first: the room/custody baseline must be present
            // before the load asks the ordinary checkpoint-resume road to act.
            adopt_occurrence_checkpoint_from_save,
            // Item state second. This applies the saved bag and adopts BOTH
            // item checkpoint baselines from the post-load values.
            crate::items::persist::restore_inventory_from_save,
            // The host-level completion point comes last: only now is the file
            // fully applied, and only now may a checkpoint resume be requested.
            complete_durable_restore,
            // Mirrors run only after the latch is true, so none can overwrite a
            // file before all domain adopters have consumed it.
            crate::items::persist::persist_inventory_to_save,
            persist_occurrence_horizon_to_save,
            crate::items::pickup::minted_horizon::persist_minted_item_horizon_to_save,
        )
            .chain(),
    );
}

/// **Mirror what the world currently remembers into the save**, for the autosave
/// to commit.
///
/// ⚠ **value-compared before it writes, like every other save mirror.** The
/// autosave's throttle is "does the file still match the value", so a mirror that
/// marked the save changed every tick would rewrite the file forever.
///
/// ⚠ **gated on the restore**, so it cannot run first and write a world that has
/// not yet been told what the file said.
///
/// ⭐ **the CURRENT horizon, not the checkpoint one.** An object carried into
/// another room and put down after the last shrine is where the player left it,
/// and a save that wrote the shrine's memory instead would move it back while
/// they watched.
#[allow(clippy::too_many_arguments)]
pub fn persist_occurrence_horizon_to_save(
    restored: Res<SaveRestored>,
    occurrences: Option<Res<AuthoredOccurrences>>,
    carried: Query<(&SimId, &InCustodyOf), With<RoomScopedEntity>>,
    custodians: Query<&SimId>,
    mut save: ResMut<AmbitionGameSave>,
) {
    if !restored.0 {
        return;
    }
    let Some(occurrences) = occurrences else {
        return;
    };
    let rows: Vec<PersistedOccurrence> = occurrences
        .rows()
        .map(|(sim_id, whereabouts)| {
            PersistedOccurrence::new(
                sim_id.as_str(),
                match whereabouts {
                    OccurrenceWhereabouts::InCustody => PersistedWhereabouts::InCustody,
                    OccurrenceWhereabouts::Placed { room, at } => PersistedWhereabouts::Placed {
                        room: room.clone(),
                        // ⭐ INTEGER pixels — see `PersistedWhereabouts::Placed`.
                        // A float would cost the save's `Eq` and make a NaN
                        // rewrite the file every frame forever.
                        x: at.x.round() as i32,
                        y: at.y.round() as i32,
                    },
                    OccurrenceWhereabouts::Consumed => PersistedWhereabouts::Consumed,
                },
            )
        })
        .collect();
    let custody: Vec<PersistedCustody> = live_custody_rows(&carried, &custodians)
        .into_iter()
        .map(|(occurrence, custodian)| {
            PersistedCustody::new(occurrence.as_str(), custodian.as_str())
        })
        .collect();
    let data = save.data();
    if data.occurrences == rows && data.custody == custody {
        return;
    }
    let data = save.data_mut();
    data.occurrences = rows;
    data.custody = custody;
}

#[cfg(test)]
mod tests;
