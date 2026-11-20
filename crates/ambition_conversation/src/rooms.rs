//! The conversation's changes, dated in the room that made them
//! (SIMULATION-ISLANDS ISL-1b, `docs/planning/engine/island-rollback-and-compact-peer-input.md`).
//!
//! The live conversation is one world value ([`ActiveConversation`]), and a
//! room rewinds alone: a talk opened in the rewound window stayed open, and a
//! conversation that ended in the window stayed ended though the replay kept
//! talking. A conversation is two bodies of one room, so each change is that
//! room's act: logged with the room and the tick and the value it replaced,
//! and put back by the room's retractor, as the save's flag writes are.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms, RoomWrite, take_rewound_superseded};

use crate::authority::{ActiveConversation, LiveConversation};
use ambition_input::InputOwner;

/// The conversations' changes rooms made within the rewind horizon, oldest
/// first, each with the seat whose conversation changed and the conversation
/// it replaced.
///
/// Rollback state with a real value: a talk changes it on a tick.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ConversationsInRooms {
    /// The conversations as the log last saw them.
    seen: Vec<LiveConversation>,
    /// Each change a room made to one seat's conversation, and what it replaced.
    changes: Vec<(RoomWrite, InputOwner, Option<LiveConversation>)>,
}

impl ConversationsInRooms {
    /// The conversations are `now` on tick `tick`: log each seat's change, if
    /// it is one, in the room of its bodies (`room_of`), and drop the changes
    /// no rewind can reach. A conversation no room holds (scripted, with no
    /// body) is seen but not logged: no room's rewind takes it back.
    pub fn observe<'a>(
        &mut self,
        now: impl Iterator<Item = &'a LiveConversation>,
        tick: u64,
        room_of: impl Fn(&LiveConversation) -> Option<LiveRoomInstance>,
    ) {
        let now: Vec<LiveConversation> = now.cloned().collect();
        if self.seen == now {
            return;
        }
        let mut owners: Vec<InputOwner> = Vec::new();
        for live in self.seen.iter().chain(now.iter()) {
            if !owners.contains(&live.input_owner) {
                owners.push(live.input_owner);
            }
        }
        for owner in owners {
            let before = self.seen.iter().find(|live| live.input_owner == owner);
            let after = now.iter().find(|live| live.input_owner == owner);
            if before == after {
                continue;
            }
            if let Some(room) = after.or(before).and_then(&room_of) {
                self.changes.push((RoomWrite { room, tick }, owner, before.cloned()));
            }
        }
        self.seen = now;
        self.changes.retain(|(written, ..)| !written.beyond_the_horizon(tick));
    }

    /// `room` rewinds to `saved_at`: take out the changes it made after that
    /// tick, newest first, and return each seat's conversation to put back.
    /// A seat is left out when a later change of its conversation in another
    /// room still stands (that room's talk is not undone).
    pub fn retract_room(&mut self, room: LiveRoomInstance, saved_at: u64) -> Vec<(InputOwner, Option<LiveConversation>)> {
        let taken = take_rewound_superseded(
            &mut self.changes,
            room,
            saved_at,
            |(written, ..)| *written,
            |later, taken| later.1 == taken.1,
        );
        // Newest first, so the last of a seat's unsuperseded entries is its
        // oldest: what the seat had at the save.
        let mut put_back: Vec<(InputOwner, Option<LiveConversation>)> = Vec::new();
        for ((_, owner, replaced), superseded) in taken {
            if superseded {
                continue;
            }
            put_back.retain(|(seat, _)| *seat != owner);
            put_back.push((owner, replaced));
        }
        for (owner, replaced) in &put_back {
            self.seen.retain(|live| live.input_owner != *owner);
            self.seen.extend(replaced.iter().cloned());
        }
        put_back
    }

    /// Every entity the log names, for the rollback registration's probe.
    pub fn referenced_entities(&self) -> Vec<Entity> {
        self.seen
            .iter()
            .chain(self.changes.iter().filter_map(|(.., replaced)| replaced.as_ref()))
            .flat_map(super::authority::LiveConversation::bodies)
            .collect()
    }

    /// Entity-free value projection.
    pub fn checksum(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.seen.len().hash(&mut hasher);
        for live in &self.seen {
            live.instance.hash(&mut hasher);
        }
        self.changes.len().hash(&mut hasher);
        for (written, owner, replaced) in &self.changes {
            (written.room.ordinal(), written.tick).hash(&mut hasher);
            format!("{owner:?}").hash(&mut hasher);
            replaced.as_ref().map(|live| &live.instance).hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl bevy::ecs::entity::MapEntities for ConversationsInRooms {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        let lives = self.seen.iter_mut().chain(self.changes.iter_mut().filter_map(|(.., replaced)| replaced.as_mut()));
        for live in lives {
            for entity in live.bodies_mut() {
                *entity = mapper.get_mapped(*entity);
            }
        }
    }
}

/// Log this tick's change to the conversation, at the tail of the tick, after
/// every writer (an opening, an end, a break, a room crossing).
pub fn log_conversation_changes_in_rooms(
    conversation: Res<ActiveConversation>,
    mut log: ResMut<ConversationsInRooms>,
    tick: Option<Res<ambition_time::SimTick>>,
    rooms: LiveRooms,
) {
    if log.seen.iter().eq(conversation.conversations()) {
        return;
    }
    let tick = tick.map_or(0, |tick| tick.0);
    log.observe(conversation.conversations(), tick, |live| {
        live.initiator.or(live.talker).and_then(|body| rooms.of(body))
    });
}

/// The room retractor: put back the conversation `room` had at `saved_at`.
pub fn retract_room_conversations(world: &mut World, room: LiveRoomInstance, saved_at: u64) {
    let Some(put_back) = world.get_resource_mut::<ConversationsInRooms>().map(|mut log| log.retract_room(room, saved_at))
    else {
        return;
    };
    // `ConversationPlugin` installs both resources, so a log means a conversation.
    let mut conversation = world.resource_mut::<ActiveConversation>();
    for (owner, live) in put_back {
        conversation.close_owned_by(owner);
        if let Some(live) = live {
            conversation.open(live);
        }
    }
}
