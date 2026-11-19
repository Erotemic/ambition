//! Private object state inherits the simulation scope of its occurrence owner.

use std::collections::{BTreeMap, BTreeSet};
use ambition_extension_sdk::ObjectRef;
use ambition_platformer2d_core::snapshot::{SnapshotState, Reader, put_bytes};
use bevy::prelude::*;
use super::{InRoomInstance, LiveRoomInstance, ObjectIndex, RoomInstanceRoot, SessionScopedEntity, resolve_object};

/// One stable scope owner. A direct room stamp takes precedence over this link.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectScopeOwner(pub ObjectRef);

impl SnapshotState for ObjectScopeOwner {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut bytes = Vec::new(); ambition_extension_sdk::structure::put(&self.0, &mut bytes); put_bytes(out, &bytes);
    }
    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        ambition_extension_sdk::wire::decode_all(reader.bytes()?, ambition_extension_sdk::structure::read).ok().map(Self)
    }
}

/// Stable semantic identity for one named private state instance.
pub fn owned_object_state_id(owner: ObjectRef, key: &str) -> crate::sim_id::SimId {
    crate::sim_id::SimId::singleton("object_state", &format!("{}:{:?}:{}:{key}",
        owner.session, owner.allocation_room, owner.sequence))
}

/// Resolve current residency without treating a session object as a room object.
pub fn object_scope_room(world: &World, mut entity: Entity) -> Option<LiveRoomInstance> {
    let mut seen = BTreeSet::new();
    while seen.insert(entity) {
        if let Some(at) = world.get::<InRoomInstance>(entity) { return Some(at.0); }
        if world.get::<RoomInstanceRoot>(entity).is_some() {
            return world.get::<LiveRoomInstance>(entity).copied();
        }
        let owner = world.get::<ObjectScopeOwner>(entity)?.0;
        let session = world.get::<SessionScopedEntity>(entity)?.0;
        entity = resolve_object(world, session, owner).ok()?;
    }
    None
}

/// The complete room population, including private state with inherited residency.
pub fn room_scope_members(world: &mut World, room: LiveRoomInstance) -> Vec<Entity> {
    let mut entities: Vec<_> = world.query::<(Entity, &InRoomInstance)>().iter(world)
        .filter(|(_, at)| at.0 == room).map(|(entity, _)| entity).collect();
    entities.extend(world.query::<(Entity, &LiveRoomInstance)>().iter(world)
        .filter(|(_, at)| **at == room).map(|(entity, _)| entity));
    include_owned_object_state(world, &mut entities);
    entities.sort();
    entities.dedup();
    entities
}

/// Include owned state in a room snapshot or visibility cut. Each edge is visited once.
pub fn include_owned_object_state(world: &World, entities: &mut Vec<Entity>) {
    let Some(mut owners) = world.try_query::<(Entity, &SessionScopedEntity, &ObjectScopeOwner,
        Option<&InRoomInstance>, Option<&RoomInstanceRoot>)>() else { return; };
    let index = ObjectIndex::capture(world);
    let mut children: BTreeMap<Entity, Vec<Entity>> = BTreeMap::new();
    for (entity, session, owner, at, root) in owners.iter(world) {
        if at.is_some() || root.is_some() { continue; }
        if let Ok(parent) = index.resolve(session.0, owner.0) {
            children.entry(parent).or_default().push(entity);
        }
    }
    let mut seen: BTreeSet<_> = entities.iter().copied().collect();
    let mut pending: Vec<_> = seen.iter().copied().collect();
    while let Some(parent) = pending.pop() {
        if let Some(owned) = children.get(&parent) {
            for child in owned {
                if seen.insert(*child) { pending.push(*child); }
            }
        }
    }
    *entities = seen.into_iter().collect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lifecycle::*, sim_id::SimId};

    #[test]
    fn private_state_follows_one_occurrence_through_travel_and_reconstruction() {
        let mut world = World::new();
        let session = SessionScopeId(1);
        let first = LiveRoomInstance::ACTIVATION;
        let second = first.next();
        for room in [first, second] {
            world.spawn((RoomInstanceRoot, room, SessionScopedEntity(session)));
        }
        let body = world.spawn((SessionScopedEntity(session), InRoomInstance(first), SimId::placement("owner"))).id();
        ensure_object_occurrence(&mut world, body).unwrap();
        let reference = object_reference(&world, body).unwrap();
        let owned = world.spawn((SessionScopedEntity(session), ObjectScopeOwner(reference),
            SimId::singleton("private", "one"), InRoomInstance(first))).id();
        PreparedObjectOccurrences::prepare(&mut world, session, first, 1).unwrap().assign(&mut world, &[owned]).unwrap();
        world.entity_mut(owned).remove::<InRoomInstance>();
        let nested = world.spawn((SessionScopedEntity(session), ObjectScopeOwner(object_reference(&world, owned).unwrap()))).id();
        let foreign = world.spawn((SessionScopedEntity(SessionScopeId(2)), ObjectScopeOwner(reference))).id();
        let mut members = vec![body];
        include_owned_object_state(&world, &mut members);
        assert!(members.contains(&owned) && members.contains(&nested));
        assert!(!members.contains(&foreign));
        assert_eq!(object_scope_room(&world, nested), Some(first));
        world.entity_mut(body).insert(InRoomInstance(second));
        assert_eq!(object_scope_room(&world, nested), Some(second));
        assert!(!room_scope_members(&mut world, first).contains(&owned));
        assert!(room_scope_members(&mut world, second).contains(&owned));
        let occurrence = *world.get::<ObjectOccurrence>(body).unwrap();
        world.despawn(body);
        assert_eq!(object_scope_room(&world, nested), None);
        let rebuilt = world.spawn((SessionScopedEntity(session), InRoomInstance(second), occurrence)).id();
        assert_ne!(rebuilt, body);
        assert_eq!(object_scope_room(&world, nested), Some(second));
        assert!(room_scope_members(&mut world, second).contains(&nested));
    }
}
