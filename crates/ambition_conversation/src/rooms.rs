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

use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms, RoomWrite};

use crate::authority::{ActiveConversation, LiveConversation};

/// The conversation's changes rooms made within the rewind horizon, oldest
/// first, each with the conversation it replaced.
///
/// Rollback state with a real value: a talk changes it on a tick.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ConversationsInRooms {
    /// The conversation as the log last saw it.
    seen: Option<LiveConversation>,
    /// Each change a room made, and the conversation it replaced.
    changes: Vec<(RoomWrite, Option<LiveConversation>)>,
}

impl ConversationsInRooms {
    /// The conversation is `now` on tick `tick`: log the change, if it is one,
    /// in the room of its bodies (`room_of`), and drop the changes no rewind
    /// can reach. A conversation no room holds (scripted, with no body) is
    /// seen but not logged: no room's rewind takes it back.
    pub fn observe(
        &mut self,
        now: Option<&LiveConversation>,
        tick: u64,
        room_of: impl Fn(&LiveConversation) -> Option<LiveRoomInstance>,
    ) {
        if self.seen.as_ref() == now {
            return;
        }
        let replaced = std::mem::replace(&mut self.seen, now.cloned());
        if let Some(room) = now.or(replaced.as_ref()).and_then(&room_of) {
            self.changes.push((RoomWrite { room, tick }, replaced));
        }
        self.changes.retain(|(written, _)| !written.beyond_the_horizon(tick));
    }

    /// `room` rewinds to `saved_at`: take out the changes it made after that
    /// tick, newest first, and return the conversation to put back. `None`
    /// when nothing goes back: no change of the room's, or a later change of
    /// another room's still stands (that room's talk is not undone).
    pub fn retract_room(&mut self, room: LiveRoomInstance, saved_at: u64) -> Option<Option<LiveConversation>> {
        let mut put_back = None;
        for at in (0..self.changes.len()).rev() {
            if !self.changes[at].0.retracted_by(room, saved_at) {
                continue;
            }
            let (_, replaced) = self.changes.remove(at);
            if at == self.changes.len() {
                put_back = Some(replaced);
            }
        }
        if let Some(conversation) = &put_back {
            self.seen.clone_from(conversation);
        }
        put_back
    }

    /// Every entity the log names, for the rollback registration's probe.
    pub fn referenced_entities(&self) -> Vec<Entity> {
        self.seen
            .iter()
            .chain(self.changes.iter().filter_map(|(_, replaced)| replaced.as_ref()))
            .flat_map(|live| [live.initiator, live.talker])
            .flatten()
            .collect()
    }

    /// Entity-free value projection.
    pub fn checksum(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.seen.as_ref().map(|live| &live.instance).hash(&mut hasher);
        self.changes.len().hash(&mut hasher);
        for (written, replaced) in &self.changes {
            (written.room.ordinal(), written.tick).hash(&mut hasher);
            replaced.as_ref().map(|live| &live.instance).hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl bevy::ecs::entity::MapEntities for ConversationsInRooms {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        let lives = self.seen.iter_mut().chain(self.changes.iter_mut().filter_map(|(_, replaced)| replaced.as_mut()));
        for live in lives {
            for entity in [live.initiator.as_mut(), live.talker.as_mut()].into_iter().flatten() {
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
    if log.seen.as_ref() == conversation.live() {
        return;
    }
    let tick = tick.map_or(0, |tick| tick.0);
    log.observe(conversation.live(), tick, |live| {
        live.initiator.or(live.talker).and_then(|body| rooms.of(body))
    });
}

/// The room retractor: put back the conversation `room` had at `saved_at`.
pub fn retract_room_conversations(world: &mut World, room: LiveRoomInstance, saved_at: u64) {
    let Some(put_back) = world.get_resource_mut::<ConversationsInRooms>().and_then(|mut log| log.retract_room(room, saved_at))
    else {
        return;
    };
    // `ConversationPlugin` installs both resources, so a log means a conversation.
    let mut conversation = world.resource_mut::<ActiveConversation>();
    match put_back {
        Some(live) => conversation.open(live),
        None => conversation.close(),
    }
}
