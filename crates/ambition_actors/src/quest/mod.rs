//! Gameplay-core adapter for the generic quest runtime.
//!
//! Quest data, events, registry, and save mirroring live in
//! `ambition_persistence::quest`. The only local piece is the room-specific
//! producer that translates the active `RoomSet` into a generic
//! `RoomEntered` quest event.

use bevy::prelude::*;

pub use ambition_persistence::quest::*;

/// Push a `RoomEntered` quest event whenever the active room changes.
/// Idempotent: only fires the frame the room id flips.
pub fn push_room_entered_quest_events(
    room_set: Res<crate::rooms::RoomSet>,
    mut registry: ResMut<ambition_persistence::quest::QuestRegistry>,
    mut last_room: Local<Option<String>>,
) {
    let current = room_set.active_spec().id.clone();
    if last_room.as_deref() == Some(current.as_str()) {
        return;
    }
    *last_room = Some(current.clone());
    registry.push_event(ambition_persistence::quest::QuestAdvanceEvent::RoomEntered(
        current,
    ));
}
