//! Scoped placement lookup from native construction provenance.

use std::collections::BTreeMap;
use bevy::prelude::*;
use ambition_extension_sdk::ObjectRef;
use crate::construction::SpawnOrigin;
use super::{ObjectIndex, ObjectLookupError, InRoomInstance, LiveRoomInstance, ObjectOccurrence, SessionScopeId, SessionScopedEntity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoredLookupError { Absent, Ambiguous, Unpublished }

/// A read-cut registry. It includes every authored root, regardless of capability.
/// Rebuild it after construction, travel or retirement.
pub struct AuthoredObjectIndex {
    objects: ObjectIndex,
    rows: BTreeMap<(SessionScopeId, LiveRoomInstance, String, String), Vec<Option<ObjectRef>>>,
}
impl AuthoredObjectIndex {
    pub fn capture(world: &World) -> Self {
        let mut rows = BTreeMap::<_, Vec<_>>::new();
        if let Some(mut objects) = world.try_query::<(Entity, &SessionScopedEntity, &InRoomInstance, &SpawnOrigin)>() {
            for (entity, session, room, origin) in objects.iter(world) {
                let SpawnOrigin::Authored {source, instance} = origin else { continue; };
                rows.entry((session.0, room.0, source.clone(), instance.clone())).or_default()
                    .push(world.get::<ObjectOccurrence>(entity).map(|stamp| stamp.reference(session.0)));
            }
        }
        Self {objects: ObjectIndex::capture(world), rows}
    }
    pub fn resolve(&self, session: SessionScopeId, room: LiveRoomInstance, source: &str, placement: &str)
        -> Result<ObjectRef, AuthoredLookupError> {
        match self.rows.get(&(session, room, source.into(), placement.into())).map(Vec::as_slice) {
            None | Some([]) => Err(AuthoredLookupError::Absent),
            Some([Some(reference)]) => self.objects.resolve(session, *reference).map(|_| *reference).map_err(|problem|
                if problem == ObjectLookupError::Ambiguous { AuthoredLookupError::Ambiguous } else { AuthoredLookupError::Unpublished }),
            Some([None]) => Err(AuthoredLookupError::Unpublished),
            Some(_) => Err(AuthoredLookupError::Ambiguous),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lifecycle::*, sim_id::SimId};

    #[test]
    fn placement_lookup_is_scoped_and_does_not_select_an_ambiguous_or_unpublished_root() {
        let mut world = World::new();
        let session = SessionScopeId(1);
        let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        for room in rooms { world.spawn((RoomInstanceRoot, room, SessionScopedEntity(session))); }
        let mut spawn = |room| world.spawn((SessionScopedEntity(session), InRoomInstance(room),
            SimId::placement("lever"), SpawnOrigin::Authored {source: "lab".into(), instance: "lever".into()})).id();
        let a = spawn(rooms[0]);
        let b = spawn(rooms[1]);
        assert_eq!(AuthoredObjectIndex::capture(&world).resolve(session, rooms[0], "lab", "lever"), Err(AuthoredLookupError::Unpublished));
        for entity in [a, b] { ensure_object_occurrence(&mut world, entity).unwrap(); }
        let reference = object_reference(&world, a).unwrap();
        let index = AuthoredObjectIndex::capture(&world);
        assert_eq!(index.resolve(session, rooms[0], "lab", "lever"), Ok(reference));
        assert_ne!(index.resolve(session, rooms[1], "lab", "lever"), Ok(reference));
        let stamp = *world.get::<ObjectOccurrence>(a).unwrap();
        let origin = world.get::<SpawnOrigin>(a).unwrap().clone();
        world.entity_mut(a).insert(InRoomInstance(rooms[1]));
        let index = AuthoredObjectIndex::capture(&world);
        assert_eq!(index.resolve(session, rooms[0], "lab", "lever"), Err(AuthoredLookupError::Absent));
        assert_eq!(index.resolve(session, rooms[1], "lab", "lever"), Err(AuthoredLookupError::Ambiguous));
        world.despawn(a);
        world.spawn((SessionScopedEntity(session), InRoomInstance(rooms[0]), origin, stamp));
        assert_eq!(AuthoredObjectIndex::capture(&world).resolve(session, rooms[0], "lab", "lever"), Ok(reference));
    }
}
