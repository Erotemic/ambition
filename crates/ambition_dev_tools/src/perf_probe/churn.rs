//! Structural churn per source location.
//!
//! Each spawn, despawn, and component add or remove moves an entity to a
//! different archetype. That is framework cost that our code causes. This
//! module counts each such change by its event, its set of components, and the
//! source location that asked for it (Bevy `track_location`).
//!
//! Bevy triggers a lifecycle event only on archetypes whose components have an
//! observer for that event. Thus two observer kinds are necessary:
//!
//! * one global observer per event reads the whole change once and counts it;
//! * "arming" observers watch every registered component and do nothing. They
//!   only make Bevy trigger the event. Components that are registered later get
//!   new arming observers on the next frame.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::panic::Location;

use bevy::ecs::change_detection::MaybeLocation;
use bevy::ecs::component::{ComponentId, Components};
use bevy::ecs::lifecycle::{Add, Despawn, Remove};
use bevy::ecs::observer::Observer;
use bevy::prelude::*;

use super::alloc::ProbeScope;

/// The kind of structural change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChurnEvent {
    /// Components on a new (or empty) entity.
    Spawn,
    /// Components added to an entity that already had components.
    Insert,
    /// Components removed from an entity that stays alive.
    Remove,
    /// An entity and all of its components removed.
    Despawn,
}

impl ChurnEvent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spawn => "spawn",
            Self::Insert => "insert",
            Self::Remove => "remove",
            Self::Despawn => "despawn",
        }
    }
}

/// One row key: what changed, which components together, and who asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChurnKey {
    pub event: ChurnEvent,
    /// Hash of the component set. [`ChurnCounts::components`] gives the ids.
    pub components: u64,
    /// `None` when Bevy did not record a caller.
    pub site: Option<&'static Location<'static>>,
}

/// The churn counts since the process started.
#[derive(Resource, Default)]
pub struct ChurnCounts {
    pub counts: HashMap<ChurnKey, u64>,
    sets: HashMap<u64, Vec<ComponentId>>,
    armed_through: usize,
}

impl ChurnCounts {
    fn record(&mut self, event: ChurnEvent, components: &[ComponentId], caller: MaybeLocation) {
        // Bevy also triggers `Add` for an insert that only replaces values, and
        // `Remove` for a remove of absent components. Neither moves the entity.
        if components.is_empty() {
            return;
        }
        let _probe = ProbeScope::enter();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        components.hash(&mut hasher);
        let set = hasher.finish();
        self.sets
            .entry(set)
            .or_insert_with(|| components.to_vec());
        let key = ChurnKey {
            event,
            components: set,
            site: caller.into_option(),
        };
        *self.counts.entry(key).or_default() += 1;
    }

    /// The component ids of a [`ChurnKey::components`] hash.
    pub fn components(&self, set: u64) -> &[ComponentId] {
        self.sets.get(&set).map_or(&[], Vec::as_slice)
    }
}

fn count_add(on: On<Add>, mut counts: ResMut<ChurnCounts>) {
    let trigger = on.trigger();
    let event = match trigger.old_archetype {
        Some(old) if old.component_count() > 0 => ChurnEvent::Insert,
        _ => ChurnEvent::Spawn,
    };
    counts.record(event, trigger.components, on.caller());
}

fn count_remove(on: On<Remove>, mut counts: ResMut<ChurnCounts>) {
    let trigger = on.trigger();
    // A despawn also triggers `Remove`. `count_despawn` counts it once.
    if trigger.new_archetype.is_some() {
        counts.record(ChurnEvent::Remove, trigger.components, on.caller());
    }
}

fn count_despawn(on: On<Despawn>, mut counts: ResMut<ChurnCounts>) {
    counts.record(ChurnEvent::Despawn, on.trigger().components, on.caller());
}

fn arm_add(_: On<Add>) {}
fn arm_remove(_: On<Remove>) {}
fn arm_despawn(_: On<Despawn>) {}

/// Arm the components that were registered since the last frame.
fn arm_new_components(
    components: &Components,
    mut counts: ResMut<ChurnCounts>,
    mut commands: Commands,
) {
    let registered = components.len();
    if registered <= counts.armed_through {
        return;
    }
    let _probe = ProbeScope::enter();
    let from = counts.armed_through;
    counts.armed_through = registered;
    let ids: Vec<ComponentId> = components
        .iter_registered()
        .map(|info| info.id())
        .filter(|id| id.index() >= from)
        .collect();
    if ids.is_empty() {
        return;
    }
    commands.spawn(Observer::new(arm_add).with_components(ids.iter().copied()));
    commands.spawn(Observer::new(arm_remove).with_components(ids.iter().copied()));
    commands.spawn(Observer::new(arm_despawn).with_components(ids));
}

/// Add the churn counters to `app`.
pub fn install(app: &mut App) {
    app.init_resource::<ChurnCounts>();
    app.add_observer(count_add);
    app.add_observer(count_remove);
    app.add_observer(count_despawn);
    app.add_systems(Last, arm_new_components);
}

/// `A+B+C+3 more`, from short type names.
pub fn component_set_label(components: &Components, ids: &[ComponentId], max: usize) -> String {
    let mut names: Vec<String> = ids
        .iter()
        .take(max)
        .map(|id| {
            components
                .get_name(*id)
                .map_or_else(|| format!("#{}", id.index()), |name| name.shortname().to_string())
        })
        .collect();
    if ids.len() > max {
        names.push(format!("{} more", ids.len() - max));
    }
    names.join("+")
}

/// `file:line`, with the cargo registry prefix removed from dependency paths.
pub fn site_label(site: Option<&'static Location<'static>>) -> String {
    let Some(site) = site else {
        return "(unknown)".to_owned();
    };
    let file = site.file();
    let file = file
        .find("/registry/src/")
        .and_then(|at| {
            let rest = &file[at + "/registry/src/".len()..];
            rest.find('/').map(|slash| &rest[slash + 1..])
        })
        .unwrap_or(file);
    format!("{file}:{}", site.line())
}
