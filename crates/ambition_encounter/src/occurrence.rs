//! A live encounter occurrence: one authored encounter in one live room.
//!
//! The authored encounter is its id (`goblin_encounter`), and the save keys
//! its durable outcome by that id. What runs is an OCCURRENCE: the authored
//! encounter in one live room. Two live rooms can instantiate one room, and
//! then each holds its own occurrence of the room's encounter, with its own
//! lifecycle, waves and members (OW1).
//!
//! ⛔ THE ROOM IS THE OCCURRENCE'S STAMP. An occurrence entity carries
//! `InRoomInstance`, and every message about one ([`crate::EncounterCommand`],
//! [`crate::EncounterEventMsg`], [`crate::EncounterGate`]) names the room
//! beside the id. The rollback carrier order already keys an entity by its
//! `SimId` and its live room, so the two occurrences keep the one authored
//! `SimId::encounter(id)`.
//!
//! One rule matches a message to an occurrence, by the rule of
//! `LiveRooms::of`: an entity is in its stamped room, else in the sole live
//! room; a message that names no room is about the sole live room. With one
//! live room, every message reaches every occurrence of its id, as before.

use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms};
use bevy::prelude::Entity;

/// The live room a message that names `room` is about: `room`, or the sole
/// live room when it names none.
pub fn message_room(live: &LiveRooms, room: Option<LiveRoomInstance>) -> Option<LiveRoomInstance> {
    room.or_else(|| live.sole())
}

/// Whether a message about `room` addresses the occurrence `entity`.
pub fn addresses(live: &LiveRooms, room: Option<LiveRoomInstance>, entity: Entity) -> bool {
    message_room(live, room) == live.of(entity)
}
