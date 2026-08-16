//! **THE DURABLE SAVE HORIZON — what survives closing the program, for the
//! occurrences the world remembers anything about.**
//!
//! ```text
//! 1  current world truth     AuthoredOccurrences + ItemCustody          ✔
//! 2  checkpoint truth        OccurrenceBaseline + CustodyBaseline +     ✔
//!                            MintedItemBaseline, restored on a death
//! 3  durable save truth      ← THIS MODULE
//! ```
//!
//! # ⭐⭐ THE ON-DISK FORM IS THE CHECKPOINT'S OWN DESCRIPTION, SERIALIZED
//!
//! Not a fourth description of the same facts. The three values a checkpoint
//! copies are exactly the three lists this writes, field for field:
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
//! [`restore_durable_horizon`] installs the three values and writes one
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
//! * a runtime mint that is NOT in a hand — lying in a loaded room, or in flight —
//!   is still undescribed and still lost, because [`MintedItemDescription`]
//!   remembers no position and nothing in the world remembers one for it;
//! * `OwnedItems` is a QUANTITY table persisted by the sibling leg in
//!   `items::persist`, and the two do not coordinate (D132's surviving half);
//! * `OccurrenceWhereabouts::Consumed` round-trips through the file and has no
//!   live producer, so nothing yet WRITES a terminal row.

use std::collections::BTreeMap;

use bevy::prelude::*;

use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::{
    PersistedCustody, PersistedMintedItem, PersistedOccurrence, PersistedWhereabouts,
};
use ambition_platformer2d_shared_tangle::construction::SpawnOrigin;
use ambition_platformer2d_shared_tangle::lifecycle::{
    live_custody_rows, AuthoredOccurrences, CustodyBaseline, InCustodyOf, OccurrenceBaseline,
    OccurrenceWhereabouts, ResetToCheckpoint, RoomScopedEntity,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

use crate::items::pickup::minted_horizon::{
    live_minted_descriptions, MintedItemBaseline, MintedItemDescription,
};
use crate::items::pickup::{GroundItem, ItemCustody};

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

/// **Apply the file's memory of where everything is, then ask for a checkpoint
/// resume.**
///
/// ⚠ **it waits for a primary body, and shares that gate with
/// [`restore_inventory_from_save`](crate::items::persist::restore_inventory_from_save)
/// on purpose.** The resume it requests names a SUBJECT, so a reset written into
/// a bodiless world is read by `resume_at_checkpoint_on_reset`, found subjectless,
/// and dropped — the ledger would be installed and no room would ever be rebuilt
/// from it. The two restores run in one chain behind one latch, so they land
/// together or not at all.
///
/// ⚠ **and it is IDEMPOTENT anyway**, which is what keeps that shared gate from
/// being load-bearing: adopting a value it already holds writes nothing, and a
/// second `ResetToCheckpoint` reconciles against the same baseline and reaches
/// the same world.
pub fn restore_durable_horizon(
    restored: Res<SaveRestored>,
    save: Res<AmbitionGameSave>,
    // The readiness gate, read as a COUNT rather than taken: this system needs a
    // body to exist, not to touch one.
    bodies: Query<(), crate::actor::PrimaryPlayerOnly>,
    occurrences: Option<ResMut<AuthoredOccurrences>>,
    occurrence_baseline: Option<ResMut<OccurrenceBaseline>>,
    custody_baseline: Option<ResMut<CustodyBaseline>>,
    minted_baseline: Option<ResMut<MintedItemBaseline>>,
    mut resets: MessageWriter<ResetToCheckpoint>,
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
                // ⛔ the id is REBUILT, never re-derived: `SimId::from_snapshot`
                // is the one road from a raw string, and the string in the file
                // is the same one the checksum and every snapshot row key on.
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
    let minted: BTreeMap<SimId, MintedItemDescription> = data
        .minted_items
        .iter()
        .map(|row| {
            (
                SimId::from_snapshot(row.occurrence.clone()),
                MintedItemDescription {
                    origin: SpawnOrigin::Dynamic {
                        parent: SimId::from_snapshot(row.parent.clone()),
                        sequence: row.sequence,
                    },
                    held_item: row.held_item.clone(),
                },
            )
        })
        .collect();

    occurrences.adopt_rows(ledger_rows);
    // ⭐ the loaded state is this process's baseline — see the module header.
    if let Some(mut baseline) = occurrence_baseline {
        baseline.adopt(occurrences.clone());
    }
    if let Some(mut baseline) = custody_baseline {
        baseline.adopt(held);
    }
    if let Some(mut baseline) = minted_baseline {
        baseline.adopt(minted);
    }
    // ⭐⭐ AND THE SHIPPED RESUME DOES THE REST. Nothing here spawns, despawns,
    // equips or rebuilds a room: the restore road a death takes reads the three
    // values just installed and reaches the same world from them.
    //
    // ⛔ **only when the file actually remembered something, and that condition
    // is not an optimisation.** A resume rebuilds the active room and puts the
    // body at the checkpoint; a save with nothing to say about any occurrence
    // describes a world already in its authored state, so asking for one would
    // make every session — every demo, every harness, every fresh boot — take a
    // room rebuild and a teleport on its first frame for no fact at all.
    // `restore_checkpoint_on_session_start` is the road that already opens a
    // session at its checkpoint.
    if !data.occurrences.is_empty() || !data.custody.is_empty() || !data.minted_items.is_empty() {
        resets.write(ResetToCheckpoint);
    }
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
pub fn persist_durable_horizon_to_save(
    restored: Res<SaveRestored>,
    occurrences: Option<Res<AuthoredOccurrences>>,
    carried: Query<(&SimId, &InCustodyOf), With<RoomScopedEntity>>,
    custodians: Query<&SimId>,
    minted: Query<(&SimId, &SpawnOrigin, &GroundItem, &ItemCustody), With<RoomScopedEntity>>,
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
    let minted_items: Vec<PersistedMintedItem> = live_minted_descriptions(&minted)
        .into_iter()
        .filter_map(|(occurrence, description)| {
            // ⛔ the population is `SpawnOrigin::Dynamic` by construction — see
            // `live_minted_descriptions` — so this destructure cannot fail. It is
            // written as a match rather than an unwrap because the disk row's
            // shape IS "dynamic", and a describer that ever handed over an
            // authored origin must be dropped rather than flattened into one.
            let SpawnOrigin::Dynamic { parent, sequence } = &description.origin else {
                return None;
            };
            Some(PersistedMintedItem {
                occurrence: occurrence.as_str().to_string(),
                parent: parent.as_str().to_string(),
                sequence: *sequence,
                held_item: description.held_item.clone(),
            })
        })
        .collect();

    let data = save.data();
    if data.occurrences == rows && data.custody == custody && data.minted_items == minted_items {
        return;
    }
    let data = save.data_mut();
    data.occurrences = rows;
    data.custody = custody;
    data.minted_items = minted_items;
}

#[cfg(test)]
mod tests;
