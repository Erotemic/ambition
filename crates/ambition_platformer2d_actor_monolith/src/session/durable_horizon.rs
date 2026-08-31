//! Durable persistence for occurrence, custody, and minted-item checkpoint state.
//!
//! The on-disk representation stores domain descriptions, not ECS component snapshots:
//! authored occurrence whereabouts, custody links, and dynamic-mint identity/provenance/spec ids.
//! Loading installs those baselines and triggers the ordinary checkpoint-reset path, so the
//! checkpoint restore remains the single reconstruction authority.
//!
//! Placed runtime mints can be recreated from their saved description. In-flight mints that were
//! never admitted to the occurrence ledger are outside this horizon. `OwnedItems` remains a
//! quantity table; consumed occurrences round-trip but currently have no live producer.

use std::collections::BTreeMap;

use bevy::prelude::*;

use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::{
    PersistedCustody, PersistedOccurrence, PersistedWhereabouts,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    live_custody_rows, AuthoredOccurrences, CustodyBaseline, InCustodyOf, OccurrenceBaseline,
    OccurrenceWhereabouts, ResetToCheckpoint, RoomScopedEntity,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// Whether the loaded save has been applied to this world.
///
/// This is the single durable-restore latch across inventory and occurrence
/// domains. It gates behavior and is therefore rollback state, not a cache.
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
    bodies: Query<(), ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>,
    occurrences: Option<ResMut<AuthoredOccurrences>>,
    occurrence_baseline: Option<ResMut<OccurrenceBaseline>>,
    custody_baseline: Option<ResMut<CustodyBaseline>>,
) {
    if restored.0 || bodies.is_empty() {
        return;
    }
    let Some(occurrences) = occurrences else {
        return;
    };
    adopt_the_ledger(
        save.data(),
        occurrences,
        occurrence_baseline,
        custody_baseline,
    );
}

/// Put the file's occurrence ledger in place BEFORE the session builds its
/// first room.
///
/// ⛔⛔ A LOAD USED TO CONSTRUCT ITS FIRST ROOM KNOWING NOTHING, AND CORRECT IT
/// AFTERWARDS. Activation passed `continuity: None`, so a room whose object the
/// file says is lying next door authored it anyway; the durable chain then ran
/// in `Update`, latched, asked for a checkpoint resume, and the room-transition
/// road rebuilt the room several ticks later with the ledger in hand. For that
/// window there were two live things behind one identity, in a world where
/// combat, pickups and encounters all run ungated — and the population that
/// picked one of them up wrote its custody over the very row the correction was
/// about to read.
///
/// ⭐ THE LEDGER LEG NEEDS NO BODY. That is the whole reason this can move: only
/// the item/wallet leg of the durable chain requires a primary body to exist,
/// and it is `restore_inventory_from_save`'s, not this one's. Running the ledger
/// adoption at [`SessionScopeSet::Activate`](ambition_platformer2d_shared_tangle::lifecycle::SessionScopeSet::Activate)
/// — the seam whose whole promise is "before any provider constructs the world
/// these values describe" — means the temporary population is never built at
/// all, rather than built and repaired.
///
/// ⚠ THE `Update` ADOPTER STAYS. A file can also arrive after activation (a
/// mid-session load), and adoption is idempotent: the same rows adopted twice
/// are the same rows.
pub fn adopt_the_occurrence_ledger_at_activation(
    // `Option`: a narrow fixture that never installs the session-scope plugin
    // registers no such message, and "there is no activation channel here" is an
    // ordinary composition rather than a reason to panic the app.
    activated: Option<
        MessageReader<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeActivated>,
    >,
    save: Res<AmbitionGameSave>,
    occurrences: Option<ResMut<AuthoredOccurrences>>,
    occurrence_baseline: Option<ResMut<OccurrenceBaseline>>,
    custody_baseline: Option<ResMut<CustodyBaseline>>,
) {
    let Some(mut activated) = activated else {
        return;
    };
    if activated.read().count() == 0 {
        return;
    }
    let Some(occurrences) = occurrences else {
        return;
    };
    adopt_the_ledger(
        save.data(),
        occurrences,
        occurrence_baseline,
        custody_baseline,
    );
}

/// The adoption itself, so the activation edge and the `Update` chain cannot
/// drift into reading the file two different ways.
fn adopt_the_ledger(
    data: &ambition_persistence::save_data::AmbitionGameSaveData,
    mut occurrences: ResMut<AuthoredOccurrences>,
    occurrence_baseline: Option<ResMut<OccurrenceBaseline>>,
    custody_baseline: Option<ResMut<CustodyBaseline>>,
) {
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
    ready_body: Query<
        &ambition_characters::actor::BodyWallet,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
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
/// The generic runtime calls this one domain offer.
pub fn install_durable_save_horizon(app: &mut App) {
    app.init_resource::<SaveRestored>()
        .add_systems(
            Update,
            adopt_the_occurrence_ledger_at_activation
                .in_set(ambition_platformer2d_shared_tangle::lifecycle::SessionScopeSet::Activate)
                // AFTER the session-scoped reset, which clears `SaveRestored`.
                // Adopting first would be adopting into a world about to have
                // its latches wiped.
                .after(crate::session::teardown::reset_session_scoped_resources_on_activation),
        )
        .add_systems(
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

/// Mirror the current occurrence horizon into the save after restore completes. Writes are
/// value-compared so an unchanged horizon does not retrigger autosave.
///
/// Persist a custody relationship only when its owning domain can reconstruct that custody after
/// process restart. `ItemCustody` qualifies; transient body possession does not. Other occurrence
/// states cross the durable horizon directly.
#[allow(clippy::too_many_arguments)]
pub fn persist_occurrence_horizon_to_save(
    restored: Res<SaveRestored>,
    occurrences: Option<Res<AuthoredOccurrences>>,
    carried: Query<(&SimId, &InCustodyOf), With<RoomScopedEntity>>,
    custodians: Query<&SimId>,
    // The occurrences whose custody survives a process boundary, because the item
    // domain saves `ItemCustody` and applies it again on load.
    durably_held: Query<&SimId, With<crate::items::pickup::ItemCustody>>,
    mut save: ResMut<AmbitionGameSave>,
) {
    if !restored.0 {
        return;
    }
    let Some(occurrences) = occurrences else {
        return;
    };
    let restorable: std::collections::BTreeSet<&str> =
        durably_held.iter().map(SimId::as_str).collect();
    let rows: Vec<PersistedOccurrence> = occurrences
        .rows()
        // An `InCustody` row is a claim that something is holding this, and the
        // file may only make that claim about a hand it can reconstruct. Every
        // other whereabouts — `Placed`, `Consumed` — is a fact about the world
        // itself and crosses unconditionally.
        .filter(|(sim_id, whereabouts)| {
            !matches!(whereabouts, OccurrenceWhereabouts::InCustody)
                || restorable.contains(sim_id.as_str())
        })
        .map(|(sim_id, whereabouts)| {
            PersistedOccurrence::new(
                sim_id.as_str(),
                match whereabouts {
                    OccurrenceWhereabouts::InCustody => PersistedWhereabouts::InCustody,
                    OccurrenceWhereabouts::Placed { room, at } => PersistedWhereabouts::Placed {
                        room: room.clone(),
                        // INTEGER pixels — see `PersistedWhereabouts::Placed`.
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
        .filter(|(occurrence, _)| restorable.contains(occurrence.as_str()))
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
