//! Change-detection churn per component and caller.
//!
//! A `&mut` access marks a component changed, also when it writes the same
//! value. Each change wakes every `Changed<T>` filter and every
//! `resource_changed` condition that reads it. This module counts, once per
//! frame, each component value (resources included) that was changed since the
//! last count and not added. It keys the count by the component and by the
//! source location of the last change (Bevy `track_location`).
//!
//! A component that changes on every entity on every frame is a per-frame value
//! (for example `Transform` of a moving body) or a write that changes nothing.
//! The site tells which.

use std::collections::HashMap;
use std::panic::Location;

use bevy::ecs::change_detection::Tick;
use bevy::ecs::component::ComponentId;
use bevy::prelude::*;

use super::alloc::ProbeScope;

/// One row key: which component, and the caller of its last change.
pub type ChangeKey = (ComponentId, Option<&'static Location<'static>>);

/// The change counts since the process started.
#[derive(Resource, Default)]
pub struct ChangeCounts {
    pub counts: HashMap<ChangeKey, u64>,
    last: Option<Tick>,
}

/// Count the changed component values of the main world since the last count.
fn count_changes(world: &mut World) {
    let this_run = world.change_tick();
    let Some(last_run) = world.resource::<ChangeCounts>().last else {
        world.resource_mut::<ChangeCounts>().last = Some(this_run);
        return;
    };
    let _probe = ProbeScope::enter();
    let newer = |tick: Tick| tick.is_newer_than(last_run, this_run);
    let mut found: Vec<ChangeKey> = Vec::new();
    let storages = world.storages();
    for archetype in world.archetypes().iter() {
        if archetype.is_empty() {
            continue;
        }
        let table = &storages.tables[archetype.table_id()];
        for id in archetype.table_components() {
            let (Some(changed), Some(added)) =
                (table.get_changed_ticks_slice_for(id), table.get_added_ticks_slice_for(id))
            else {
                continue;
            };
            let by = table.get_changed_by_slice_for(id).into_option().flatten();
            for entity in archetype.entities() {
                let row = entity.table_row().index();
                // SAFETY: an exclusive system has the whole world; no other
                // reference to these ticks exists while it runs.
                let (changed, added) = unsafe { (*changed[row].get(), *added[row].get()) };
                if newer(changed) && !newer(added) {
                    // SAFETY: as above.
                    found.push((id, by.map(|by| unsafe { *by[row].get() })));
                }
            }
        }
        for id in archetype.sparse_set_components() {
            let Some(set) = storages.sparse_sets.get(id) else {
                continue;
            };
            for entity in archetype.entities() {
                let Some(ticks) = set.get_ticks(entity.id()) else {
                    continue;
                };
                if newer(ticks.changed) && !newer(ticks.added) {
                    let by = set.get_changed_by(entity.id()).into_option().flatten();
                    // SAFETY: as above.
                    found.push((id, by.map(|by| unsafe { *by.get() })));
                }
            }
        }
    }
    let mut counts = world.resource_mut::<ChangeCounts>();
    for key in found {
        *counts.counts.entry(key).or_default() += 1;
    }
    counts.last = Some(this_run);
}

/// Add the change counter to `app`. It runs last in `Last`, so it sees the
/// changes of the whole frame.
pub fn install(app: &mut App) {
    app.init_resource::<ChangeCounts>();
    app.add_systems(Last, count_changes.after(super::report_perf_probe));
}
