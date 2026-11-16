//! A replay uses separate message buffers and reader cursors. The host's
//! pending messages stay at the live boundary until the replay ends.

use bevy_ecs::message::{Message, MessageCursor, MessageIterator, MessageIteratorWithId, Messages};
use bevy_ecs::prelude::{Local, Res, Resource, World};
use bevy_ecs::system::SystemParam;

#[derive(Resource, Default)]
struct ReplaySequence(u64);

#[derive(Resource)]
struct SuspendedMessages<M: Message> {
    original: Messages<M>,
    sequence: u64,
}

/// Start a private stream for a replay. Session channel declarations call this.
pub fn isolate_messages_for_replay<M: Message>(world: &mut World) {
    let Some(original) = world.remove_resource::<Messages<M>>() else { return };
    assert!(!world.contains_resource::<SuspendedMessages<M>>(), "message replays cannot nest");
    let mut sequence = world.get_resource_or_init::<ReplaySequence>();
    sequence.0 = sequence.0.checked_add(1).expect("message replay sequence exhausted");
    let sequence = sequence.0;
    world.insert_resource(SuspendedMessages { original, sequence });
    world.insert_resource(Messages::<M>::default());
}

/// Restore the live stream, including its message ids and pending payloads.
pub fn restore_messages_after_replay<M: Message>(world: &mut World) {
    if let Some(suspended) = world.remove_resource::<SuspendedMessages<M>>() {
        world.insert_resource(suspended.original);
    }
}

struct Cursors<M: Message> {
    live: MessageCursor<M>,
    replay: MessageCursor<M>,
    sequence: Option<u64>,
}

impl<M: Message> Default for Cursors<M> {
    fn default() -> Self {
        Self { live: Default::default(), replay: Default::default(), sequence: None }
    }
}

impl<M: Message> Cursors<M> {
    fn selected(&mut self, sequence: Option<u64>) -> &mut MessageCursor<M> {
        if sequence.is_some() {
            if self.sequence != sequence {
                self.replay = MessageCursor::default();
                self.sequence = sequence;
            }
            &mut self.replay
        } else {
            &mut self.live
        }
    }

    fn current(&self, sequence: Option<u64>) -> Option<&MessageCursor<M>> {
        match sequence {
            None => Some(&self.live),
            Some(_) if sequence == self.sequence => Some(&self.replay),
            _ => None,
        }
    }
}

/// A simulation reader whose live cursor does not advance during a room
/// replay. Use this reader for session channels that a replay isolates.
#[derive(SystemParam)]
pub struct ReplayMessageReader<'w, 's, M: Message> {
    cursors: Local<'s, Cursors<M>>,
    messages: Res<'w, Messages<M>>,
    suspended: Option<Res<'w, SuspendedMessages<M>>>,
}

impl<M: Message> ReplayMessageReader<'_, '_, M> {
    pub fn read(&mut self) -> MessageIterator<'_, M> {
        self.read_with_id().without_id()
    }

    pub fn read_with_id(&mut self) -> MessageIteratorWithId<'_, M> {
        let sequence = self.suspended.as_ref().map(|suspended| suspended.sequence);
        self.cursors.selected(sequence).read_with_id(&self.messages)
    }

    pub fn len(&self) -> usize {
        let sequence = self.suspended.as_ref().map(|suspended| suspended.sequence);
        self.cursors.current(sequence).map_or(self.messages.len(), |cursor| cursor.len(&self.messages))
    }

    pub fn is_empty(&self) -> bool { self.len() == 0 }

    pub fn clear(&mut self) {
        let sequence = self.suspended.as_ref().map(|suspended| suspended.sequence);
        self.cursors.selected(sequence).clear(&self.messages);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::system::SystemState;

    #[derive(Message)]
    struct Notice(u8);

    #[test]
    fn a_replay_preserves_different_live_reader_positions_and_message_ids() {
        let mut world = World::new();
        world.init_resource::<Messages<Notice>>();
        let mut fast = SystemState::<ReplayMessageReader<Notice>>::new(&mut world);
        let mut slow = SystemState::<ReplayMessageReader<Notice>>::new(&mut world);
        world.write_message(Notice(0));
        assert_eq!(fast.get_mut(&mut world).unwrap().read().map(|n| n.0).collect::<Vec<_>>(), [0]);
        isolate_messages_for_replay::<Notice>(&mut world);
        world.write_message(Notice(9));
        assert_eq!(fast.get_mut(&mut world).unwrap().read().map(|n| n.0).collect::<Vec<_>>(), [9]);
        assert_eq!(slow.get_mut(&mut world).unwrap().read().map(|n| n.0).collect::<Vec<_>>(), [9]);
        restore_messages_after_replay::<Notice>(&mut world);
        world.write_message(Notice(1));
        assert_eq!(fast.get_mut(&mut world).unwrap().read().map(|n| n.0).collect::<Vec<_>>(), [1]);
        assert_eq!(slow.get_mut(&mut world).unwrap().read().map(|n| n.0).collect::<Vec<_>>(), [0, 1]);
    }
}
