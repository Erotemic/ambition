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
//! host: it holds no resources (nor maps the entity references a resource
//! holds), and it carries no checksum. See
//! `docs/planning/engine/island-rollback-and-compact-peer-input.md`.

use std::any::Any;
use std::collections::BTreeSet;

use bevy::ecs::component::{ComponentId, Mutable};
use bevy::ecs::entity::{EntityHashMap, EntityMapper, MapEntities};
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

/// One registered entity-mapping row: rewrite the references a component
/// holds after a restore re-created the entities they name.
struct MapRow {
    name: &'static str,
    apply: fn(&mut World, &[Entity], &EntityHashMap<Entity>),
}

/// Every registered component row and entity-mapping row, in registration
/// order.
#[derive(Resource, Default)]
pub struct RoomSnapshotRows {
    rows: Vec<RoomRow>,
    maps: Vec<MapRow>,
}

/// What a restore did to the entities, beside putting values back.
#[derive(Debug, Default)]
pub struct RoomRestore {
    /// Entities spawned after the save, despawned by the restore.
    pub despawned: Vec<Entity>,
    /// Entities gone since the save, re-created: the saved entity, and the
    /// entity that stands for it now.
    pub recreated: Vec<(Entity, Entity)>,
}

/// The saved state of some entities: the rows they carried, each with its
/// values.
///
/// The values name entities by the ids they had at the save, and those ids
/// never change here. A restore that has to re-create an entity records its
/// stand-in in `live`, beside the saved id, and every restore maps the saved
/// ids to their stand-ins again: the values it puts back are the saved ones,
/// which still name the saved ids.
pub struct RoomSnapshot {
    /// The entities as saved.
    saved: Vec<Entity>,
    /// The entity that stands for each saved one now (the saved one itself
    /// until a restore re-creates it).
    live: Vec<Entity>,
    slices: Vec<(usize, Slice)>,
}

impl RoomSnapshot {
    /// How many rows the snapshot holds values for.
    pub fn rows(&self) -> usize {
        self.slices.len()
    }

    /// The entities that stand for the saved ones now.
    pub fn entities(&self) -> &[Entity] {
        &self.live
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
        RoomSnapshot { saved: entities.to_vec(), live: entities.to_vec(), slices }
    }

    /// Put the room back as it was saved. `now` is the room's entities at
    /// the restore: one the snapshot does not hold was spawned after the save
    /// and is despawned. A saved entity that no longer lives is spawned again
    /// (as a new entity) and every reference the room's components hold to it
    /// is rewritten by the registered entity-mapping rows. Then each saved row
    /// is put back; a row the entities carry now but did not carry then is
    /// removed from them.
    ///
    /// The snapshot records the stand-ins, so it can be restored again: the
    /// next restore reuses a stand-in that still lives, re-creates one that
    /// does not, and maps every saved id to its stand-in each time.
    pub fn restore(&self, world: &mut World, snapshot: &mut RoomSnapshot, now: &[Entity]) -> RoomRestore {
        let mut report = RoomRestore::default();
        for entity in now.iter().filter(|entity| !snapshot.live.contains(entity)) {
            if world.despawn(*entity) {
                report.despawned.push(*entity);
            }
        }
        for (saved, live) in snapshot.saved.iter().zip(snapshot.live.iter_mut()) {
            if world.get_entity(*live).is_err() {
                let again = world.spawn_empty().id();
                report.recreated.push((*saved, again));
                *live = again;
            }
        }
        // Saved id -> the entity standing for it, for every saved entity that
        // has a stand-in: the values put back below name the saved ids.
        let remap: EntityHashMap<Entity> = snapshot
            .saved
            .iter()
            .zip(&snapshot.live)
            .filter(|(saved, live)| saved != live)
            .map(|(saved, live)| (*saved, *live))
            .collect();
        let saved: BTreeSet<usize> = snapshot.slices.iter().map(|(index, _)| *index).collect();
        for (index, slice) in &snapshot.slices {
            (self.rows[*index].restore)(world, &snapshot.live, slice);
        }
        for index in self.carried_rows(world, &snapshot.live) {
            if !saved.contains(&index) {
                // A slice with no values: every entity loses the row.
                (self.rows[index].restore)(world, &snapshot.live, &Slice(Box::new(())));
            }
        }
        if !remap.is_empty() {
            for map in &self.maps {
                (map.apply)(world, &snapshot.live, &remap);
            }
        }
        report
    }
}

/// Maps a re-created entity to its stand-in, and every other entity to itself.
struct Remap<'a>(&'a EntityHashMap<Entity>);

impl EntityMapper for Remap<'_> {
    fn get_mapped(&mut self, source: Entity) -> Entity {
        self.0.get(&source).copied().unwrap_or(source)
    }

    fn set_mapped(&mut self, _source: Entity, _target: Entity) {}
}

fn apply_map<T: Component<Mutability = Mutable> + MapEntities>(world: &mut World, entities: &[Entity], remap: &EntityHashMap<Entity>) {
    for entity in entities {
        if let Some(mut value) = world.get_mut::<T>(*entity) {
            value.map_entities(&mut Remap(remap));
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

/// Record an entity-mapping row.
pub fn record_map_row<T: Component<Mutability = Mutable> + MapEntities>(app: &mut App, name: &'static str) {
    let mut rows = app.world_mut().get_resource_or_insert_with(RoomSnapshotRows::default);
    if !rows.maps.iter().any(|kept| kept.name == name) {
        rows.maps.push(MapRow { name, apply: apply_map::<T> });
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
        let mut snapshot = rows.save(world, &[charged, bare]);
        assert_eq!(snapshot.rows(), 1);

        world.get_mut::<Charge>(charged).unwrap().0 = 2;
        world.entity_mut(bare).insert(Charge(5));
        world.get_mut::<Charge>(elsewhere).unwrap().0 = 8;
        rows.restore(world, &mut snapshot, &[charged, bare]);

        assert_eq!(world.get::<Charge>(charged), Some(&Charge(1)));
        assert_eq!(world.get::<Charge>(bare), None, "a component added after the save stayed");
        assert_eq!(world.get::<Charge>(elsewhere), Some(&Charge(8)), "an entity outside the snapshot was restored");
    }

    #[derive(Component, Clone, Debug, PartialEq)]
    struct Points(Entity);

    impl MapEntities for Points {
        fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
            self.0 = mapper.get_mapped(self.0);
        }
    }

    /// An entity gone since the save comes back, with its values, and a
    /// reference to it points at the entity that stands for it now; an entity
    /// spawned since the save is despawned.
    #[test]
    fn a_restore_brings_back_the_gone_and_takes_away_the_new() {
        let mut app = App::new();
        record_clone_row::<Charge>(&mut app, "test.charge");
        record_clone_row::<Points>(&mut app, "test.points");
        record_map_row::<Points>(&mut app, "map.test.points");
        let world = app.world_mut();
        let target = world.spawn(Charge(4)).id();
        let pointer = world.spawn(Points(target)).id();
        let rows = world.remove_resource::<RoomSnapshotRows>().expect("the rows are recorded");
        let mut snapshot = rows.save(world, &[target, pointer]);

        world.despawn(target);
        let newcomer = world.spawn(Charge(9)).id();
        let report = rows.restore(world, &mut snapshot, &[pointer, newcomer]);

        assert_eq!(report.despawned, vec![newcomer]);
        let [(gone, again)] = report.recreated[..] else { panic!("one entity re-created: {report:?}") };
        assert_eq!(gone, target);
        assert!(world.get_entity(newcomer).is_err(), "the newcomer still lives");
        assert_eq!(world.get::<Charge>(again), Some(&Charge(4)), "the re-created entity lost its value");
        assert_eq!(world.get::<Points>(pointer), Some(&Points(again)), "the reference names the gone entity");
        assert_eq!(snapshot.entities(), &[again, pointer], "the snapshot does not name the stand-in");
    }

    /// Review 2026-11-10: the snapshot used to swap a re-created entity's id
    /// for its stand-in while the saved values kept the old id, so a second
    /// restore put back a reference to the dead entity (the stand-in still
    /// lived, so nothing was mapped). Restored twice, and three times after
    /// the stand-in is gone again, the reference names the live stand-in.
    #[test]
    fn a_snapshot_restored_again_keeps_its_references_live() {
        let mut app = App::new();
        record_clone_row::<Charge>(&mut app, "test.charge");
        record_clone_row::<Points>(&mut app, "test.points");
        record_map_row::<Points>(&mut app, "map.test.points");
        let world = app.world_mut();
        let target = world.spawn(Charge(4)).id();
        let pointer = world.spawn(Points(target)).id();
        let rows = world.remove_resource::<RoomSnapshotRows>().expect("the rows are recorded");
        let mut snapshot = rows.save(world, &[target, pointer]);

        world.despawn(target);
        rows.restore(world, &mut snapshot, &[pointer]);
        let first = snapshot.entities()[0];
        assert_ne!(first, target);

        // Played again: the pointer is changed and restored once more, while
        // the stand-in lives.
        world.get_mut::<Points>(pointer).unwrap().0 = pointer;
        let report = rows.restore(world, &mut snapshot, &[first, pointer]);
        assert!(report.recreated.is_empty(), "a living stand-in is reused: {report:?}");
        assert_eq!(world.get::<Points>(pointer), Some(&Points(first)), "the second restore names the dead entity");

        // The stand-in is gone too: the third restore maps from the saved id.
        world.despawn(first);
        let report = rows.restore(world, &mut snapshot, &[pointer]);
        let [(saved, second)] = report.recreated[..] else { panic!("one entity re-created: {report:?}") };
        assert_eq!(saved, target, "the report names the saved entity");
        assert_eq!(world.get::<Points>(pointer), Some(&Points(second)), "the third restore names a dead entity");
        assert_eq!(world.get::<Charge>(second), Some(&Charge(4)));
    }

    /// An entity that carried no row when saved, and carries one now, loses it.
    #[test]
    fn a_row_none_of_the_entities_carried_is_taken_away() {
        let mut app = App::new();
        record_clone_row::<Charge>(&mut app, "test.charge");
        let world = app.world_mut();
        let bare = world.spawn_empty().id();
        let rows = world.remove_resource::<RoomSnapshotRows>().expect("the row is recorded");
        let mut snapshot = rows.save(world, &[bare]);
        assert_eq!(snapshot.rows(), 0);
        world.entity_mut(bare).insert(Charge(3));
        rows.restore(world, &mut snapshot, &[bare]);
        assert_eq!(world.get::<Charge>(bare), None);
    }
}
