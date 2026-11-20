//! Live object occurrences retain their identity when they travel.

use bevy::prelude::*;
use ambition_platformer2d_core::snapshot::{Reader, SnapshotState, put_u32, put_u64, put_u8};
use super::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot, SessionScopeId, SessionScopedEntity};

/// One object lifetime in its session. The optional allocation room is a namespace,
/// not the object's current location. Retirement removes this component;
/// rollback reconstruction restores it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct ObjectOccurrence {
    pub(crate) room: Option<u32>,
    pub(crate) generation: u64,
}

impl ObjectOccurrence {
    pub fn allocation_room(self) -> Option<u32> { self.room }
    pub fn sequence(self) -> u64 { self.generation }
    pub fn reference(self, session: SessionScopeId) -> ambition_extension_sdk::ObjectRef {
        ambition_extension_sdk::ObjectRef {session: session.0, allocation_room: self.room, sequence: self.generation}
    }
}

impl SnapshotState for ObjectOccurrence {
    fn encode(&self, out: &mut Vec<u8>) {
        match self.room {
            None => put_u8(out, 0),
            Some(room) => { put_u8(out, 1); put_u32(out, room); }
        }
        put_u64(out, self.generation);
    }
    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        Some(Self { room: match reader.u8()? {0 => None, 1 => Some(reader.u32()?), _ => return None}, generation: reader.u64()? })
    }
}

#[derive(Component, Clone, Debug, Default)]
pub(crate) struct ObjectOccurrenceSequence(u64);

impl SnapshotState for ObjectOccurrenceSequence {
    fn encode(&self, out: &mut Vec<u8>) { put_u64(out, self.0); }
    fn decode(reader: &mut Reader<'_>) -> Option<Self> { Some(Self(reader.u64()?)) }
}

/// This notice follows complete initialization and publication of the batch.
#[derive(Event, Clone, Debug)]
pub struct ObjectPublished {
    pub subject: Entity,
    pub session: SessionScopeId,
    pub occurrence: ObjectOccurrence,
}

/// A body's defeat, committed by its damage owner on the tick the hit killed it.
///
/// Provenance is captured at the decision: the defeated object and the
/// attacker's object (when the hit had one and it is an addressed object), and
/// the live room the defeat happened in. A notification built from it needs no
/// living source. Every kill writes one, whichever owner takes the death after
/// (the world's economy, a match ruleset, a fall out of the world).
#[derive(bevy::prelude::Message, Clone, Debug, PartialEq, Eq)]
pub struct DefeatCommitted {
    pub object: ambition_extension_sdk::ObjectRef,
    pub by: Option<ambition_extension_sdk::ObjectRef>,
    pub room: LiveRoomInstance,
}

/// A checked allocation range. Preparation does not change the room counter.
pub struct PreparedObjectOccurrences {
    root: Entity,
    session: SessionScopeId,
    room: Option<LiveRoomInstance>,
    start: u64,
    end: u64,
}

impl PreparedObjectOccurrences {
    pub fn prepare(world: &mut World, session: SessionScopeId, room: LiveRoomInstance,
        count: usize) -> Result<Self, &'static str> {
        let root = super::live_room_root_for(world, session, room)
            .ok_or("The publication room is not live.")?;
        let start = world.get::<ObjectOccurrenceSequence>(root).map_or(0, |sequence| sequence.0);
        let count = u64::try_from(count).map_err(|_| "The occurrence count is too large.")?;
        let end = start.checked_add(count).ok_or("The room's object occurrence range is exhausted.")?;
        Ok(Self {root, session, room: Some(room), start, end})
    }

    pub fn prepare_session(world: &mut World, session: SessionScopeId, count: usize)
        -> Result<Self, &'static str> {
        let root = super::session_root_for_scope(world, session).ok_or("The session is not live.")?;
        let start = world.get::<ObjectOccurrenceSequence>(root).map_or(0, |sequence| sequence.0);
        let count = u64::try_from(count).map_err(|_| "The occurrence count is too large.")?;
        let end = start.checked_add(count).ok_or("The session's occurrence range is exhausted.")?;
        Ok(Self {root, session, room: None, start, end})
    }

    /// Read a reserved address before materialization. It is not live until assignment.
    pub fn reference(&self, index: usize) -> ambition_extension_sdk::ObjectRef {
        let sequence = self.start.checked_add(index as u64).expect("a prepared index fits its range");
        assert!(sequence < self.end);
        ambition_extension_sdk::ObjectRef {session: self.session.0,
            allocation_room: self.room.map(|room| room.ordinal()), sequence}
    }

    /// Validate every root before assigning any stamp or advancing the counter.
    pub fn assign(self, world: &mut World, entities: &[Entity]) -> Result<Vec<ObjectPublished>, &'static str> {
        if entities.len() as u64 != self.end - self.start
            || world.get::<SessionScopedEntity>(self.root).map(|owner| owner.0) != Some(self.session)
            || match self.room {
                Some(room) => world.get::<LiveRoomInstance>(self.root) != Some(&room),
                None => world.get::<super::SessionRoot>(self.root).map(|root| root.0) != Some(self.session),
            }
            || world.get::<ObjectOccurrenceSequence>(self.root).map_or(0, |sequence| sequence.0) != self.start {
            return Err("The publication allocation changed before commit.");
        }
        let planned: std::collections::BTreeMap<_, _> = entities.iter().enumerate()
            .map(|(index, entity)| (self.reference(index), *entity)).collect();
        let live = std::cell::OnceCell::new();
        let mut unique = std::collections::BTreeSet::new();
        for entity in entities {
            if !unique.insert(*entity)
                || world.get::<SessionScopedEntity>(*entity).map(|owner| owner.0) != Some(self.session)
                || self.residency(world, *entity, &planned, &live)? != self.room
                || world.get::<crate::sim_id::SimId>(*entity).is_none()
                || world.get::<ObjectOccurrence>(*entity).is_some() {
                return Err("A publication root has invalid ownership or an existing occurrence.");
            }
        }
        world.entity_mut(self.root).insert(ObjectOccurrenceSequence(self.end));
        Ok(entities.iter().enumerate().map(|(index, subject)| {
            let occurrence = ObjectOccurrence {room: self.room.map(|room| room.ordinal()), generation: self.start + index as u64};
            world.entity_mut(*subject).insert(occurrence);
            ObjectPublished {subject: *subject, session: self.session, occurrence}
        }).collect())
    }

    fn residency(&self, world: &World, mut entity: Entity,
        planned: &std::collections::BTreeMap<ambition_extension_sdk::ObjectRef, Entity>,
        live: &std::cell::OnceCell<super::ObjectIndex>)
        -> Result<Option<LiveRoomInstance>, &'static str> {
        let mut seen = std::collections::BTreeSet::new();
        while seen.insert(entity) {
            if world.get::<SessionScopedEntity>(entity).map(|owner| owner.0) != Some(self.session) {
                return Err("An owned publication state crosses sessions.");
            }
            if let Some(at) = world.get::<InRoomInstance>(entity) { return Ok(Some(at.0)); }
            if world.get::<RoomInstanceRoot>(entity).is_some() {
                return world.get::<LiveRoomInstance>(entity).copied().map(Some)
                    .ok_or("A publication room root has no live room.");
            }
            let Some(owner) = world.get::<super::ObjectScopeOwner>(entity) else { return Ok(None); };
            entity = match planned.get(&owner.0) {
                Some(entity) if owner.0.session == self.session.0 => *entity,
                _ => live.get_or_init(|| super::ObjectIndex::capture(world)).resolve(self.session, owner.0)
                    .map_err(|_| "An owned publication state has no unique live or planned owner.")?,
            };
        }
        Err("Publication state ownership contains a cycle.")
    }
}

/// Allocate once before an object receives an occurrence reference. The native
/// owner must retain the stamp until retirement. The room counter and stamp
/// rewind together. Objects without a room use the session root counter.
/// Allocation does not use the authored identity or Entity bits.
pub fn ensure_object_occurrence(world: &mut World, entity: Entity) -> Option<ObjectOccurrence> {
    let session = world.get::<SessionScopedEntity>(entity)?.0;
    if let Some(occurrence) = world.get::<ObjectOccurrence>(entity) {
        return Some(*occurrence);
    }
    let room = world.get::<InRoomInstance>(entity).map(|at| at.0)
        .or_else(|| world.get::<RoomInstanceRoot>(entity).and(world.get::<LiveRoomInstance>(entity).copied()));
    if world.get::<crate::sim_id::SimId>(entity).is_none() && world.get::<RoomInstanceRoot>(entity).is_none()
        && world.get::<super::SessionRoot>(entity).is_none() {
        return None;
    }
    let root = match room {
        Some(room) => super::live_room_root_for(world, session, room)?,
        None => super::session_root_for_scope(world, session)?,
    };
    let mut root = world.entity_mut(root);
    let mut sequence = root.entry::<ObjectOccurrenceSequence>().or_default().into_mut();
    let next = sequence.0.checked_add(1)?;
    let occurrence = ObjectOccurrence { room: room.map(|room| room.ordinal()), generation: sequence.0 };
    sequence.0 = next;
    world.entity_mut(entity).insert(occurrence);
    Some(occurrence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_ranges_refuse_without_assigning_partial_occurrences() {
        let mut world = World::new();
        let session = SessionScopeId(1);
        let room = LiveRoomInstance::ACTIVATION;
        let root = world.spawn((RoomInstanceRoot, room, SessionScopedEntity(session))).id();
        let body = |world: &mut World, key| world.spawn((SessionScopedEntity(session),
            InRoomInstance(room), crate::sim_id::SimId::placement(key))).id();
        let a = body(&mut world, "a");
        let b = body(&mut world, "b");
        let prepared = PreparedObjectOccurrences::prepare(&mut world, session, room, 2).unwrap();
        assert!(world.get::<ObjectOccurrenceSequence>(root).is_none());
        world.entity_mut(b).insert(SessionScopedEntity(SessionScopeId(2)));
        assert!(prepared.assign(&mut world, &[a, b]).is_err());
        assert!(world.get::<ObjectOccurrence>(a).is_none());
        assert!(world.get::<ObjectOccurrenceSequence>(root).is_none());
        world.entity_mut(b).insert(SessionScopedEntity(session));
        let stale = PreparedObjectOccurrences::prepare(&mut world, session, room, 2).unwrap();
        let source = body(&mut world, "source");
        ensure_object_occurrence(&mut world, source).unwrap();
        assert!(stale.assign(&mut world, &[a, b]).is_err());
        assert!(world.get::<ObjectOccurrence>(a).is_none());
        assert!(world.get::<ObjectOccurrence>(b).is_none());
        let prepared = PreparedObjectOccurrences::prepare(&mut world, session, room, 2).unwrap();
        let notices = prepared.assign(&mut world, &[a, b]).unwrap();
        assert_ne!(notices[0].occurrence, notices[1].occurrence);
        assert_eq!(world.get::<ObjectOccurrence>(a), Some(&notices[0].occurrence));
        let parent = body(&mut world, "composed");
        let prepared = PreparedObjectOccurrences::prepare(&mut world, session, room, 2).unwrap();
        let owner = prepared.reference(0);
        let private = world.spawn((SessionScopedEntity(session), crate::sim_id::SimId::singleton("private", "one"),
            super::super::ObjectScopeOwner(ambition_extension_sdk::ObjectRef::UNBOUND))).id();
        assert!(prepared.assign(&mut world, &[parent, private]).is_err());
        assert!(world.get::<ObjectOccurrence>(parent).is_none());
        assert!(world.get::<ObjectOccurrence>(private).is_none());
        let prepared = PreparedObjectOccurrences::prepare(&mut world, session, room, 2).unwrap();
        assert_eq!(prepared.reference(0), owner);
        world.entity_mut(private).insert(super::super::ObjectScopeOwner(owner));
        prepared.assign(&mut world, &[parent, private]).unwrap();
        assert_eq!(super::super::object_reference(&world, parent), Some(owner));
        assert_eq!(super::super::object_scope_room(&world, private), Some(room));
        assert!(world.get::<InRoomInstance>(private).is_none());
        world.entity_mut(root).insert(ObjectOccurrenceSequence(u64::MAX));
        assert!(PreparedObjectOccurrences::prepare(&mut world, session, room, 1).is_err());
    }
}
