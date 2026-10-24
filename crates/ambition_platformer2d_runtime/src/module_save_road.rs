//! The session durable save road of the extension host
//! ([`SaveRoad::SessionDurable`]): a module's session record whose schema
//! says `SaveEligibility::Durable` is kept in the durable world save, and
//! comes back when that save loads.
//!
//! Three adapters, installed with the offer in one call, so the host admits
//! a durable schema only where something keeps its records:
//!
//! - the mirror writes the live rows into the save, in the simulation, like
//!   every other durable family (DURABLE-HORIZON-CHECKSUM);
//! - the restore puts the saved rows into the session's store, once, on the
//!   tick the loaded save is applied;
//! - a New Game clears the session's records. A New Game restarts the
//!   session in the same root, and a record it kept would go back into the
//!   wiped save on the next mirror.
//!
//! A death keeps the durable records, as it keeps the durable facts (flags,
//! quests). Which facts a death before any checkpoint takes back is Q164.

use ambition_extension_host::save::{adopt_saved_rows, merged_rows, saved_rows, SavedRow};
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, SaveRoad, SessionRecords};
use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::PersistedModuleRecord;
use ambition_platformer2d_shared_tangle::lifecycle::{DurableHorizonSet, DurableRestoreSet, SaveRestored};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::*;

const ROAD: SaveRoad = SaveRoad::SessionDurable;

/// Offer the road and install its adapters. The composition calls it once,
/// after the extension host and the durable save horizon are in the App.
pub fn install_module_save_road(app: &mut App) {
    let sim = app.sim_schedule();
    app.offer_extension_save_road(ROAD)
        // Its writes are its own family (`module_records`), so its order
        // against the other domains' mirrors does not change the save.
        .add_systems(sim, persist_module_records_to_save.in_set(DurableHorizonSet::DomainMirror))
        .add_systems(sim, restore_module_records_from_save.in_set(DurableRestoreSet::Domains))
        .add_systems(
            ambition_platformer2d_shared_tangle::lifecycle::CheckpointDomainApply,
            start_module_records_fresh,
        );
}

fn to_saved(row: &PersistedModuleRecord) -> SavedRow {
    SavedRow { key: row.key.clone(), schema: row.schema.clone(), record: row.record.clone() }
}

fn to_persisted(row: SavedRow) -> PersistedModuleRecord {
    PersistedModuleRecord { key: row.key, schema: row.schema, record: row.record }
}

/// Mirror the session's durable module records into the save when they
/// differ from the saved rows. (sim)
fn persist_module_records_to_save(
    restored: Res<SaveRestored>,
    admitted: Option<Res<AdmittedExtensions>>,
    sessions: Query<&SessionRecords>,
    mut save: ResMut<AmbitionGameSave>,
) {
    // Before the loaded save is applied, the live store is not the save's
    // value yet: writing it would erase what the load is about to give back.
    if !restored.0 {
        return;
    }
    let (Some(admitted), Ok(records)) = (admitted, sessions.single()) else {
        return;
    };
    let saved: Vec<SavedRow> = save.data().module_records().iter().map(to_saved).collect();
    let live = saved_rows(&admitted.0, ROAD, records);
    let rows = merged_rows(&admitted.0, ROAD, &saved, live);
    if rows == saved {
        return;
    }
    // `data_mut` only past the guard: it marks the save changed.
    save.data_mut().set_module_records(rows.into_iter().map(to_persisted).collect());
}

/// Put the saved module records into the session's store, on the tick the
/// loaded save is applied. (sim)
///
/// ⚠ ONCE, on the tick `complete_durable_restore` raises [`SaveRestored`]
/// (the primary body exists). A body that is not the player can run a
/// module before that, and an adoption on each waiting tick would overwrite
/// what it wrote.
fn restore_module_records_from_save(
    restored: Res<SaveRestored>,
    ready_body: Query<(), ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>,
    admitted: Option<Res<AdmittedExtensions>>,
    save: Res<AmbitionGameSave>,
    mut sessions: Query<&mut SessionRecords>,
) {
    if restored.0 || ready_body.single().is_err() {
        return;
    }
    let (Some(admitted), Ok(mut records)) = (admitted, sessions.single_mut()) else {
        return;
    };
    let saved: Vec<SavedRow> = save.data().module_records().iter().map(to_saved).collect();
    for problem in adopt_saved_rows(&admitted.0, ROAD, &mut records.0, &saved) {
        warn!(target: "ambition_platformer2d::save", "{problem}; the record starts from its live value");
    }
}

/// A New Game starts the session's module records from nothing. (fresh-run
/// reducer, in the checkpoint commit's domain-apply schedule)
fn start_module_records_fresh(
    fresh: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::FreshRunRestore>>,
    mut sessions: Query<&mut SessionRecords>,
) {
    if fresh.is_none() {
        return;
    }
    for mut records in &mut sessions {
        if !records.0.records().is_empty() {
            records.0 = Default::default();
        }
    }
}
