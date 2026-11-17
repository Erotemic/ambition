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
//!
//! The body durable road ([`SaveRoad::BodyDurable`]) keeps a module's body
//! record under the body's durable identity: the `SimId` of an authored or
//! provider-staged occurrence, the identity the save already names
//! occurrences by. The records outlive the live body (its room unloaded, the
//! body destroyed) and come back when a body with that identity is built
//! again, in this session or after a load. A body the simulation minted
//! (`SpawnOrigin::Dynamic`) has no durable identity, so its records retire
//! with it, as a transient body record does.
//!
//! ⛔ A body takes its saved records ONCE, when it is built: the observer
//! gives it a `BodyRecords` at once, and from then on that component is the
//! record and rolls back with the body. Adopting "whatever is not stored yet"
//! on each tick instead would hand a body rewound to before its first write
//! the value the save took from the future. The sweep catches a body that
//! was built before the modules were admitted.
//!
//! The mirror skips a room replay: the save is a world store, and the replay
//! ends where the live tick will write it again (`docs/systems/room-replay.md`).

use ambition_extension_host::save::{
    adopt_saved_rows, forget_kept_records, merged_rows, road_keeps_any, saved_rows, SavedRow,
};
use ambition_extension_host::{
    AdmittedExtensions, BodyRecords, ExtensionAppExt, ExtensionSet, RecordSet, SaveRoad, SessionRecords,
};
use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::{PersistedBodyModuleRecord, PersistedModuleRecord};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::construction::SpawnOrigin;
use ambition_platformer2d_shared_tangle::lifecycle::RoomReplay;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use std::collections::BTreeMap;
use ambition_platformer2d_shared_tangle::lifecycle::{DurableHorizonSet, DurableRestoreSet, SaveRestored};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::*;

const ROAD: SaveRoad = SaveRoad::SessionDurable;
const BODY_ROAD: SaveRoad = SaveRoad::BodyDurable;

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
    app.offer_extension_save_road(BODY_ROAD)
        // A body's identity, provenance and body arrive in whatever inserts
        // its construction makes: the last of the three adopts.
        .add_observer(|added: On<Add, SpawnOrigin>, bodies: AdoptionBodies, admitted: AdoptionAdmitted, save: AdoptionSave, commands: Commands| {
            adopt_body_records_when_built(added.entity, bodies, admitted, save, commands)
        })
        .add_observer(|added: On<Add, SimId>, bodies: AdoptionBodies, admitted: AdoptionAdmitted, save: AdoptionSave, commands: Commands| {
            adopt_body_records_when_built(added.entity, bodies, admitted, save, commands)
        })
        .add_observer(|added: On<Add, BodyKinematics>, bodies: AdoptionBodies, admitted: AdoptionAdmitted, save: AdoptionSave, commands: Commands| {
            adopt_body_records_when_built(added.entity, bodies, admitted, save, commands)
        })
        // Before every module phase: each phase runs after `Admit`.
        .add_systems(sim, adopt_body_records_of_bodies_built_earlier.in_set(ExtensionSet::Admit))
        .add_systems(sim, persist_body_records_to_save.in_set(DurableHorizonSet::DomainMirror))
        .add_systems(sim, restore_body_records_from_save.in_set(DurableRestoreSet::Domains))
        .add_systems(
            ambition_platformer2d_shared_tangle::lifecycle::CheckpointDomainApply,
            start_body_records_fresh,
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

/// Whether `origin` outlives its entity: an authored or provider-staged
/// occurrence is built again under the same identity; a minted one is not.
fn durable_identity(origin: &SpawnOrigin) -> bool {
    !matches!(origin, SpawnOrigin::Dynamic { .. })
}

/// The save's rows of the body `body`.
fn saved_body_rows(save: &AmbitionGameSave, body: &SimId) -> Vec<SavedRow> {
    save.data()
        .body_module_records()
        .iter()
        .filter(|row| row.body == body.as_str())
        .map(|row| SavedRow { key: row.key.clone(), schema: row.schema.clone(), record: row.record.clone() })
        .collect()
}

/// A new store for the body `body`, holding its saved records.
fn adopted_store(admitted: &AdmittedExtensions, save: Option<&AmbitionGameSave>, body: &SimId) -> BodyRecords {
    let mut records = RecordSet::default();
    if let Some(save) = save {
        for problem in adopt_saved_rows(&admitted.0, BODY_ROAD, &mut records, &saved_body_rows(save, body)) {
            warn!(target: "ambition_platformer2d::save", "{body}: {problem}; the record starts from its initial value");
        }
    }
    BodyRecords(records)
}

type AdoptionBodies<'w, 's> =
    Query<'w, 's, (&'static SimId, &'static SpawnOrigin), (With<BodyKinematics>, Without<BodyRecords>)>;
type AdoptionAdmitted<'w> = Option<Res<'w, AdmittedExtensions>>;
type AdoptionSave<'w> = Option<Res<'w, AmbitionGameSave>>;

/// A durable body takes its saved records the moment it is built, before any
/// module runs for it. (observers)
fn adopt_body_records_when_built(
    entity: Entity,
    bodies: AdoptionBodies,
    admitted: AdoptionAdmitted,
    save: AdoptionSave,
    mut commands: Commands,
) {
    let Some(admitted) = admitted.filter(|admitted| road_keeps_any(&admitted.0, BODY_ROAD)) else {
        return;
    };
    let Ok((id, origin)) = bodies.get(entity) else {
        return;
    };
    if durable_identity(origin) {
        let store = adopted_store(&admitted, save.as_deref(), id);
        commands.entity(entity).insert(store);
    }
}

/// The same for a durable body that was built before the modules were
/// admitted. (sim, before every module phase)
fn adopt_body_records_of_bodies_built_earlier(
    bodies: Query<(Entity, &SimId, &SpawnOrigin), (With<BodyKinematics>, Without<BodyRecords>)>,
    admitted: Option<Res<AdmittedExtensions>>,
    save: Option<Res<AmbitionGameSave>>,
    mut commands: Commands,
) {
    let Some(admitted) = admitted.filter(|admitted| road_keeps_any(&admitted.0, BODY_ROAD)) else {
        return;
    };
    for (entity, id, origin) in &bodies {
        if durable_identity(origin) {
            commands.entity(entity).insert(adopted_store(&admitted, save.as_deref(), id));
        }
    }
}

/// Mirror the durable bodies' kept records into the save when they differ
/// from the saved rows. A body that is not live keeps its rows. (sim)
fn persist_body_records_to_save(
    restored: Res<SaveRestored>,
    replay: Option<Res<RoomReplay>>,
    admitted: Option<Res<AdmittedExtensions>>,
    bodies: Query<(&SimId, Ref<BodyRecords>, &SpawnOrigin)>,
    mut save: ResMut<AmbitionGameSave>,
) {
    // Before the loaded save is applied, a live body's store is not the
    // save's value yet; during a room replay, the live tick writes after it.
    if !restored.0 || replay.is_some() {
        return;
    }
    let Some(admitted) = admitted.filter(|admitted| road_keeps_any(&admitted.0, BODY_ROAD)) else {
        return;
    };
    // A body leaving changes no row, so only a store that changed (or the
    // save just applied) can.
    if !restored.is_changed() && !bodies.iter().any(|(_, records, _)| records.is_changed()) {
        return;
    }
    let mut by_body: BTreeMap<String, Vec<SavedRow>> = BTreeMap::new();
    for row in save.data().body_module_records() {
        by_body.entry(row.body.clone()).or_default().push(SavedRow {
            key: row.key.clone(),
            schema: row.schema.clone(),
            record: row.record.clone(),
        });
    }
    for (id, records, origin) in &bodies {
        if !durable_identity(origin) {
            continue;
        }
        let live = saved_rows(&admitted.0, BODY_ROAD, &records);
        let saved = by_body.remove(id.as_str()).unwrap_or_default();
        let rows = merged_rows(&admitted.0, BODY_ROAD, &saved, live);
        if !rows.is_empty() {
            by_body.insert(id.as_str().to_owned(), rows);
        }
    }
    let rows: Vec<PersistedBodyModuleRecord> = by_body
        .into_iter()
        .flat_map(|(body, rows)| {
            rows.into_iter().map(move |row| PersistedBodyModuleRecord {
                body: body.clone(),
                key: row.key,
                schema: row.schema,
                record: row.record,
            })
        })
        .collect();
    if rows.as_slice() == save.data().body_module_records() {
        return;
    }
    // `data_mut` only past the guard: it marks the save changed.
    save.data_mut().set_body_module_records(rows);
}

/// Give each live durable body its saved records on the tick the loaded save
/// is applied: a body built before the restore would otherwise keep what it
/// wrote and mirror it over the saved row. (sim, the durable restore)
fn restore_body_records_from_save(
    restored: Res<SaveRestored>,
    ready_body: Query<(), ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly>,
    admitted: Option<Res<AdmittedExtensions>>,
    save: Res<AmbitionGameSave>,
    mut bodies: Query<(&SimId, &SpawnOrigin, &mut BodyRecords)>,
) {
    if restored.0 || ready_body.single().is_err() {
        return;
    }
    let Some(admitted) = admitted.filter(|admitted| road_keeps_any(&admitted.0, BODY_ROAD)) else {
        return;
    };
    for (id, origin, mut records) in &mut bodies {
        if !durable_identity(origin) {
            continue;
        }
        let saved = saved_body_rows(&save, id);
        let mut store = records.0.clone();
        forget_kept_records(&admitted.0, BODY_ROAD, &mut store);
        for problem in adopt_saved_rows(&admitted.0, BODY_ROAD, &mut store, &saved) {
            warn!(target: "ambition_platformer2d::save", "{id}: {problem}; the record starts from its initial value");
        }
        if store != records.0 {
            records.0 = store;
        }
    }
}

/// A New Game starts every live body's kept records from nothing, so the
/// next mirror does not put them back into the wiped save. (fresh-run
/// reducer, in the checkpoint commit's domain-apply schedule)
fn start_body_records_fresh(
    fresh: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::FreshRunRestore>>,
    admitted: Option<Res<AdmittedExtensions>>,
    mut bodies: Query<&mut BodyRecords>,
) {
    let (Some(_), Some(admitted)) = (fresh, admitted) else {
        return;
    };
    if !road_keeps_any(&admitted.0, BODY_ROAD) {
        return;
    }
    for mut records in &mut bodies {
        let mut store = records.0.clone();
        forget_kept_records(&admitted.0, BODY_ROAD, &mut store);
        if store != records.0 {
            records.0 = store;
        }
    }
}
