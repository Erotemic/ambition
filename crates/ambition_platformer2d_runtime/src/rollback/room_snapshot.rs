//! A snapshot of one room's entities, through the registered rollback rows.
//!
//! SIMULATION-ISLANDS ISL-2a: the App-wide `bevy_ggrs` `SaveWorld` runs one system per
//! registered row, whatever the room holds, so a rollback history per room on
//! that mechanism would pay the whole schedule once per room. This keeps, for
//! every registered component row, a type-erased save and restore that touch
//! only the entities they are given; a room snapshot visits only the rows its
//! entities carry. Both registrars record the rows (the schema one on a hosted
//! world, the `bevy_ggrs` one on a rollback host): a hosted world rewinds a
//! room too. It is the candidate for room-level rollback, not yet a rollback
//! host: it does not re-create a despawned entity or despawn a spawned one, it
//! holds no resources, and it carries no checksum. See
//! `docs/planning/engine/island-rollback-and-compact-peer-input.md`.

use std::any::Any;
use std::collections::BTreeSet;

use bevy::ecs::component::{ComponentId, Mutable};
use bevy::prelude::*;

use ambition_platformer2d_core::snapshot::{decode_state, encode_state, SnapshotState};

/// One row's values for a set of entities: `None` where an entity did not
/// carry the component.
struct Slice(Box<dyn Any + Send + Sync>);

/// One registered component row's type-erased save and restore.
struct RoomRow {
    name: &'static str,
    component: fn(&World) -> Option<ComponentId>,
    save: fn(&World, &[Entity]) -> Slice,
    restore: fn(&mut World, &[Entity], &Slice),
}

/// Every registered component row, in registration order.
#[derive(Resource, Default)]
pub struct RoomSnapshotRows {
    rows: Vec<RoomRow>,
}

/// The saved state of some entities: the rows they carried, each with its
/// values.
pub struct RoomSnapshot {
    entities: Vec<Entity>,
    slices: Vec<(usize, Slice)>,
}

impl RoomSnapshot {
    /// How many rows the snapshot holds values for.
    pub fn rows(&self) -> usize {
        self.slices.len()
    }

    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }
}

impl RoomSnapshotRows {
    /// Registered component rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The names of the rows `entities` carry, in registration order.
    pub fn carried(&self, world: &World, entities: &[Entity]) -> Vec<&'static str> {
        self.carried_rows(world, entities).into_iter().map(|index| self.rows[index].name).collect()
    }

    fn carried_rows(&self, world: &World, entities: &[Entity]) -> Vec<usize> {
        let mut present: BTreeSet<ComponentId> = BTreeSet::new();
        for entity in entities {
            if let Ok(entity) = world.get_entity(*entity) {
                present.extend(entity.archetype().components().iter().copied());
            }
        }
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, row)| (row.component)(world).is_some_and(|id| present.contains(&id)))
            .map(|(index, _)| index)
            .collect()
    }

    /// Save the rows `entities` carry.
    pub fn save(&self, world: &World, entities: &[Entity]) -> RoomSnapshot {
        let slices = self
            .carried_rows(world, entities)
            .into_iter()
            .map(|index| (index, (self.rows[index].save)(world, entities)))
            .collect();
        RoomSnapshot { entities: entities.to_vec(), slices }
    }

    /// Put the saved values back on the snapshot's entities that still live.
    /// A row the entities carry now but did not carry then is removed from
    /// them.
    pub fn restore(&self, world: &mut World, snapshot: &RoomSnapshot) {
        let saved: BTreeSet<usize> = snapshot.slices.iter().map(|(index, _)| *index).collect();
        for (index, slice) in &snapshot.slices {
            (self.rows[*index].restore)(world, &snapshot.entities, slice);
        }
        for index in self.carried_rows(world, &snapshot.entities) {
            if !saved.contains(&index) {
                // A slice with no values: every entity loses the row.
                (self.rows[index].restore)(world, &snapshot.entities, &Slice(Box::new(())));
            }
        }
    }
}

fn row_component<T: Component>(world: &World) -> Option<ComponentId> {
    world.component_id::<T>()
}

fn save_clone<T: Component + Clone>(world: &World, entities: &[Entity]) -> Slice {
    let values: Vec<Option<T>> = entities.iter().map(|entity| world.get::<T>(*entity).cloned()).collect();
    Slice(Box::new(values))
}

fn restore_clone<T: Component<Mutability = Mutable> + Clone>(world: &mut World, entities: &[Entity], slice: &Slice) {
    let values = slice.0.downcast_ref::<Vec<Option<T>>>();
    for (position, entity) in entities.iter().enumerate() {
        let Ok(mut entity) = world.get_entity_mut(*entity) else { continue };
        match values.and_then(|values| values[position].as_ref()) {
            Some(value) => match entity.get_mut::<T>() {
                Some(mut live) => *live = value.clone(),
                None => {
                    entity.insert(value.clone());
                }
            },
            None => {
                entity.remove::<T>();
            }
        }
    }
}

fn save_codec<T: Component + SnapshotState>(world: &World, entities: &[Entity]) -> Slice {
    let values: Vec<Option<Vec<u8>>> = entities.iter().map(|entity| world.get::<T>(*entity).map(encode_state)).collect();
    Slice(Box::new(values))
}

/// A codec row is put back by insertion: some are declared without mutable
/// access, and insertion replaces a live value.
fn restore_codec<T: Component + SnapshotState>(world: &mut World, entities: &[Entity], slice: &Slice) {
    let values = slice.0.downcast_ref::<Vec<Option<Vec<u8>>>>();
    for (position, entity) in entities.iter().enumerate() {
        let Ok(mut entity) = world.get_entity_mut(*entity) else { continue };
        match values.and_then(|values| values[position].as_deref()).and_then(decode_state::<T>) {
            Some(value) => {
                entity.insert(value);
            }
            None => {
                entity.remove::<T>();
            }
        }
    }
}

/// Keep `row` unless a row of that name is already kept: a declaration can
/// reach the registrar from two plugins.
fn record(app: &mut App, row: RoomRow) {
    let mut rows = app.world_mut().get_resource_or_insert_with(RoomSnapshotRows::default);
    if !rows.rows.iter().any(|kept| kept.name == row.name) {
        rows.rows.push(row);
    }
}

/// Record a cloned component row.
pub fn record_clone_row<T: Component<Mutability = Mutable> + Clone>(app: &mut App, name: &'static str) {
    record(app, RoomRow { name, component: row_component::<T>, save: save_clone::<T>, restore: restore_clone::<T> });
}

/// Record a row stored through its canonical codec.
pub fn record_codec_row<T: Component + SnapshotState>(app: &mut App, name: &'static str) {
    record(app, RoomRow { name, component: row_component::<T>, save: save_codec::<T>, restore: restore_codec::<T> });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component, Clone, Debug, PartialEq)]
    struct Charge(u32);

    /// A restore puts back a changed value, takes away a component added
    /// since the save, and leaves an entity it was not given alone.
    #[test]
    fn a_restore_puts_back_values_and_takes_away_what_was_added() {
        let mut app = App::new();
        record_clone_row::<Charge>(&mut app, "test.charge");
        let world = app.world_mut();
        let charged = world.spawn(Charge(1)).id();
        let bare = world.spawn_empty().id();
        let elsewhere = world.spawn(Charge(7)).id();
        let rows = world.remove_resource::<RoomSnapshotRows>().expect("the row is recorded");
        let snapshot = rows.save(world, &[charged, bare]);
        assert_eq!(snapshot.rows(), 1);

        world.get_mut::<Charge>(charged).unwrap().0 = 2;
        world.entity_mut(bare).insert(Charge(5));
        world.get_mut::<Charge>(elsewhere).unwrap().0 = 8;
        rows.restore(world, &snapshot);

        assert_eq!(world.get::<Charge>(charged), Some(&Charge(1)));
        assert_eq!(world.get::<Charge>(bare), None, "a component added after the save stayed");
        assert_eq!(world.get::<Charge>(elsewhere), Some(&Charge(8)), "an entity outside the snapshot was restored");
    }

    /// An entity that carried no row when saved, and carries one now, loses it.
    #[test]
    fn a_row_none_of_the_entities_carried_is_taken_away() {
        let mut app = App::new();
        record_clone_row::<Charge>(&mut app, "test.charge");
        let world = app.world_mut();
        let bare = world.spawn_empty().id();
        let rows = world.remove_resource::<RoomSnapshotRows>().expect("the row is recorded");
        let snapshot = rows.save(world, &[bare]);
        assert_eq!(snapshot.rows(), 0);
        world.entity_mut(bare).insert(Charge(3));
        rows.restore(world, &snapshot);
        assert_eq!(world.get::<Charge>(bare), None);
    }
}
