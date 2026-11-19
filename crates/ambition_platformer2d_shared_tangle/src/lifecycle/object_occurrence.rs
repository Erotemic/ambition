//! Live object occurrences retain their identity when they travel.

use bevy::prelude::*;
use ambition_platformer2d_core::snapshot::{Reader, SnapshotState, put_u32, put_u64};
use super::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot, SessionScopedEntity};

/// One object lifetime in its session. The allocation room is a namespace,
/// not the object's current location. Retirement removes this component;
/// rollback reconstruction restores it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct ObjectOccurrence {
    pub(crate) room: u32,
    pub(crate) generation: u64,
}

impl ObjectOccurrence {
    pub fn allocation_room(self) -> u32 { self.room }
    pub fn sequence(self) -> u64 { self.generation }
}

impl SnapshotState for ObjectOccurrence {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.room);
        put_u64(out, self.generation);
    }
    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        Some(Self { room: reader.u32()?, generation: reader.u64()? })
    }
}

#[derive(Component, Clone, Debug, Default)]
pub(crate) struct ObjectOccurrenceSequence(u64);

impl SnapshotState for ObjectOccurrenceSequence {
    fn encode(&self, out: &mut Vec<u8>) { put_u64(out, self.0); }
    fn decode(reader: &mut Reader<'_>) -> Option<Self> { Some(Self(reader.u64()?)) }
}

/// Allocate once before an object receives an occurrence reference. The native
/// owner must retain the stamp until retirement. The room counter and stamp
/// rewind together. Allocation does not use the authored identity or Entity bits.
pub fn ensure_object_occurrence(world: &mut World, entity: Entity) -> Option<ObjectOccurrence> {
    let session = world.get::<SessionScopedEntity>(entity)?.0;
    if let Some(occurrence) = world.get::<ObjectOccurrence>(entity) {
        return Some(*occurrence);
    }
    let room = world.get::<InRoomInstance>(entity).map(|at| at.0)
        .or_else(|| world.get::<RoomInstanceRoot>(entity).and(world.get::<LiveRoomInstance>(entity).copied()))?;
    if world.get::<crate::sim_id::SimId>(entity).is_none() && world.get::<RoomInstanceRoot>(entity).is_none() {
        return None;
    }
    let root = super::live_room_root_for(world, session, room)?;
    let mut root = world.entity_mut(root);
    let mut sequence = root.entry::<ObjectOccurrenceSequence>().or_default().into_mut();
    let next = sequence.0.checked_add(1)?;
    let occurrence = ObjectOccurrence { room: room.ordinal(), generation: sequence.0 };
    sequence.0 = next;
    world.entity_mut(entity).insert(occurrence);
    Some(occurrence)
}
