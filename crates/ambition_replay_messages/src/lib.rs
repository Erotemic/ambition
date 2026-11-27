//! A replay uses separate message buffers and reader cursors. The host's
//! pending messages stay at the live boundary until the replay ends.
//!
//! ```
//! use ambition_replay_messages::{isolate_messages_for_replay, restore_messages_after_replay};
//! use bevy_ecs::{message::{Message, Messages}, world::World};
//!
//! #[derive(Message)]
//! struct Trigger;
//!
//! let mut world = World::new();
//! world.init_resource::<Messages<Trigger>>();
//! world.write_message(Trigger);
//! isolate_messages_for_replay::<Trigger>(&mut world);
//! assert!(world.resource::<Messages<Trigger>>().is_empty());
//! restore_messages_after_replay::<Trigger>(&mut world);
//! assert_eq!(world.resource::<Messages<Trigger>>().len(), 1);
//! ```

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

/// Start a private stream for a replay. If the channel is absent, do nothing.
/// Call this outside the schedules that read, write, or update the stream.
/// Use ReplayMessageReader for readers that also run in the replay.
pub fn isolate_messages_for_replay<M: Message>(world: &mut World) {
    assert!(
        !world.contains_resource::<SuspendedMessages<M>>(),
        "message replays cannot nest"
    );
    if !world.contains_resource::<Messages<M>>() {
        return;
    }
    let mut counter = world.get_resource_or_init::<ReplaySequence>();
    let sequence = counter
        .0
        .checked_add(1)
        .expect("message replay sequence exhausted");
    counter.0 = sequence;

    let original = world
        .remove_resource::<Messages<M>>()
        .expect("the channel exists");
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
        Self {
            live: Default::default(),
            replay: Default::default(),
            sequence: None,
        }
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

/// Read from the private cursor during a replay. Read from the live cursor
/// at all other times. Each replay starts with a new private cursor.
/// Keep ordinary Bevy message readers out of schedules that replay this channel.
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
        self.cursors
            .current(sequence)
            .map_or(self.messages.len(), |cursor| cursor.len(&self.messages))
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

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
        assert_eq!(
            fast.get_mut(&mut world)
                .unwrap()
                .read()
                .map(|n| n.0)
                .collect::<Vec<_>>(),
            [0]
        );
        for value in [9, 10] {
            isolate_messages_for_replay::<Notice>(&mut world);
            world.write_message(Notice(value));
            assert_eq!(
                fast.get_mut(&mut world)
                    .unwrap()
                    .read_with_id()
                    .map(|(n, id)| (n.0, id.id))
                    .collect::<Vec<_>>(),
                [(value, 0)]
            );
            assert_eq!(
                slow.get_mut(&mut world)
                    .unwrap()
                    .read()
                    .map(|n| n.0)
                    .collect::<Vec<_>>(),
                [value]
            );
            restore_messages_after_replay::<Notice>(&mut world);
        }
        world.write_message(Notice(1));
        assert_eq!(
            fast.get_mut(&mut world)
                .unwrap()
                .read_with_id()
                .map(|(n, id)| (n.0, id.id))
                .collect::<Vec<_>>(),
            [(1, 1)]
        );
        assert_eq!(
            slow.get_mut(&mut world)
                .unwrap()
                .read()
                .map(|n| n.0)
                .collect::<Vec<_>>(),
            [0, 1]
        );
    }

    #[test]
    fn a_failed_start_leaves_the_current_stream_intact() {
        use std::panic::{catch_unwind, AssertUnwindSafe};

        let mut world = World::new();
        isolate_messages_for_replay::<Notice>(&mut world);
        restore_messages_after_replay::<Notice>(&mut world);
        assert!(!world.contains_resource::<Messages<Notice>>());

        world.init_resource::<Messages<Notice>>();
        world.write_message(Notice(0));
        isolate_messages_for_replay::<Notice>(&mut world);
        world.write_message(Notice(9));
        let start = |world: &mut World| {
            catch_unwind(AssertUnwindSafe(|| {
                isolate_messages_for_replay::<Notice>(world)
            }))
        };
        assert!(start(&mut world).is_err());
        assert_eq!(world.resource::<Messages<Notice>>().len(), 1);

        restore_messages_after_replay::<Notice>(&mut world);
        world.insert_resource(ReplaySequence(u64::MAX));
        assert!(start(&mut world).is_err());
        let mut reader = SystemState::<ReplayMessageReader<Notice>>::new(&mut world);
        assert_eq!(
            reader
                .get_mut(&mut world)
                .unwrap()
                .read()
                .map(|n| n.0)
                .collect::<Vec<_>>(),
            [0]
        );
    }
}
