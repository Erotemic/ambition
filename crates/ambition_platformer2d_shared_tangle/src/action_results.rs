//! Scoped action replies. Domain owners publish the stages they apply.

use ambition_extension_sdk::action::{
    ActionJournalEntry, ActionResult, ActionResults, ActionStage, ActionTicket, ReplyTo,
};
use bevy::prelude::*;

use crate::lifecycle::{SessionScopedEntity, ObjectOccurrence, ensure_object_occurrence};
#[cfg(test)]
use crate::lifecycle::{InRoomInstance, LiveRoomInstance};
use crate::sim_id::SimId;

const HISTORY_LIMIT: usize = 64;
const PENDING_LIMIT: usize = 32;

/// Native owners cancel queued work before the mailbox reports cancellation.
#[derive(Resource, Default)]
pub struct ActionCancellationHandlers(
    std::collections::BTreeMap<String, fn(&mut World, &ActionReply) -> bool>,
);

pub fn register_action_cancellation(
    app: &mut App,
    domain: &str,
    handler: fn(&mut World, &ActionReply) -> bool,
) {
    app.init_resource::<ActionCancellationHandlers>();
    let mut handlers = app.world_mut().resource_mut::<ActionCancellationHandlers>();
    if let Some(old) = handlers.0.insert(domain.into(), handler) {
        assert!(
            std::ptr::fn_addr_eq(old, handler),
            "an action domain has two cancellation owners"
        );
    }
}

/// A domain can refuse cancellation after application has started.
pub fn cancel_action(world: &mut World, scope: Entity, ticket: &ActionTicket) -> bool {
    if !world
        .get::<ActionMailbox>(scope)
        .is_some_and(|mailbox| mailbox.is_pending(ticket))
    {
        return false;
    }
    let Some(recipient) = ActionRecipient::of(world, scope) else {
        return false;
    };
    let reply = ActionReply {
        recipient,
        ticket: ticket.clone(),
    };
    let handler = world
        .get_resource::<ActionCancellationHandlers>()
        .and_then(|handlers| handlers.0.get(&ticket.domain))
        .copied();
    if handler.is_some_and(|handler| !handler(world, &reply)) {
        return false;
    }
    publish_action_result(
        world,
        Some(&reply),
        ActionStage::Canceled,
        true,
        "caller_canceled",
        "",
        Vec::new(),
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingAction {
    ticket: ActionTicket,
    signature: u64,
    selector: String,
    stage: ActionStage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Producer {
    entry: String,
    domain: String,
    high_watermark: Option<u32>,
    pending: Vec<PendingAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct JournalRow {
    sequence: u64,
    result: ActionResult,
    selector: String,
    signature: u64,
    delivered: bool,
}

/// A scope retains bounded history and pending actions. A request number
/// cannot apply again after its retained reply expires.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionMailbox {
    next_occurrence: u64,
    next_sequence: u64,
    dropped: u64,
    producers: Vec<Producer>,
    journal: Vec<JournalRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionStart {
    New(ActionTicket),
    Existing(ActionTicket),
    Rejected,
}

/// Deferred work keeps semantic identities, not native entity handles.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct ActionRecipient {
    pub session: u64,
    pub occurrence: ObjectOccurrence,
    pub subject: Option<String>,
}

impl ActionRecipient {
    pub fn of(world: &World, scope: Entity) -> Option<Self> {
        Some(Self {
            session: world.get::<SessionScopedEntity>(scope)?.0.0,
            occurrence: *world.get::<ObjectOccurrence>(scope)?,
            subject: world.get::<SimId>(scope).map(|id| id.as_str().to_owned()),
        })
    }

    fn matches(&self, session: &SessionScopedEntity, occurrence: &ObjectOccurrence) -> bool {
        session.0.0 == self.session && self.occurrence == *occurrence
    }

    pub fn resolve(&self, world: &World) -> Option<Entity> {
        let mut query = world.try_query_filtered::<(Entity, &SessionScopedEntity, &ObjectOccurrence), With<ActionMailbox>>()?;
        let mut found = query.iter(world)
            .filter(|(_, session, occurrence)| self.matches(session, occurrence))
            .map(|(entity, ..)| entity);
        let only = found.next()?;
        found.next().is_none().then_some(only)
    }
}

/// A domain can complete only the occurrence that its adapter admitted.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActionReply {
    pub recipient: ActionRecipient,
    #[serde(with = "ticket_codec")]
    pub ticket: ActionTicket,
}

mod ticket_codec {
    use super::*;
    use serde::{Deserialize, Serialize};
    pub fn serialize<S: serde::Serializer>(
        ticket: &ActionTicket,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut bytes = Vec::new();
        ambition_extension_sdk::structure::put(ticket, &mut bytes);
        bytes.serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<ActionTicket, D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        ambition_extension_sdk::wire::decode_all(
            &bytes,
            ambition_extension_sdk::structure::read::<ActionTicket>,
        )
        .map_err(serde::de::Error::custom)
    }
}

pub struct ActionDispatch {
    pub execute: bool,
    pub reply: Option<ActionReply>,
}

/// Start tracking before work enters its native queue. Untracked requests
/// keep their normal path and allocate no mailbox.
pub fn begin_action(
    world: &mut World,
    scope: Entity,
    entry: &str,
    domain: &str,
    reply: Option<&ReplyTo>,
    signature: u64,
) -> ActionDispatch {
    let Some(reply) = reply else {
        return ActionDispatch {
            execute: true,
            reply: None,
        };
    };
    let now = world
        .get_resource::<ambition_time::SimTick>()
        .map_or(0, |tick| tick.0);
    if world.get_entity(scope).is_err() {
        return ActionDispatch {
            execute: false,
            reply: None,
        };
    }
    let Some(occurrence) = ensure_object_occurrence(world, scope) else {
        return ActionDispatch { execute: false, reply: None };
    };
    world.entity_mut(scope).entry::<ActionMailbox>().or_default();
    let recipient = ActionRecipient::of(world, scope);
    let started = world.get_mut::<ActionMailbox>(scope).unwrap()
        .begin(occurrence, entry, domain, reply, signature, now);
    let ActionStart::New(ticket) = started else {
        return ActionDispatch {
            execute: false,
            reply: None,
        };
    };
    let Some(recipient) = recipient else {
        world
            .get_mut::<ActionMailbox>(scope)
            .unwrap()
            .publish(ActionResult {
                ticket,
                stage: ActionStage::Rejected,
                terminal: true,
                tick: now,
                code: "no_live_scope".into(),
                message: "The caller has no live session and room identity.".into(),
                subjects: Vec::new(),
            });
        return ActionDispatch {
            execute: false,
            reply: None,
        };
    };
    ActionDispatch {
        execute: true,
        reply: Some(ActionReply { recipient, ticket }),
    }
}

pub fn action_is_pending(world: &World, reply: Option<&ActionReply>) -> bool {
    reply.is_none_or(|reply| {
        reply
            .recipient
            .resolve(world)
            .and_then(|entity| world.get::<ActionMailbox>(entity))
            .is_some_and(|mailbox| mailbox.is_pending(&reply.ticket))
    })
}

pub fn publish_action_result(
    world: &mut World,
    reply: Option<&ActionReply>,
    stage: ActionStage,
    terminal: bool,
    code: impl Into<String>,
    message: impl Into<String>,
    subjects: Vec<String>,
) -> bool {
    let Some(reply) = reply else {
        return true;
    };
    let Some(entity) = reply.recipient.resolve(world) else {
        return false;
    };
    let now = world
        .get_resource::<ambition_time::SimTick>()
        .map_or(0, |tick| tick.0);
    world
        .get_mut::<ActionMailbox>(entity)
        .is_some_and(|mut mailbox| {
            mailbox.publish(ActionResult {
                ticket: reply.ticket.clone(),
                stage,
                terminal,
                tick: now,
                code: code.into(),
                message: message.into(),
                subjects,
            })
        })
}

/// Typed consumers can publish without taking exclusive access to the world.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ActionReplies<'w, 's> {
    scopes: Query<'w, 's, (Entity, &'static SessionScopedEntity, &'static ObjectOccurrence), With<ActionMailbox>>,
    mailboxes: Query<'w, 's, &'static mut ActionMailbox>,
    tick: Option<Res<'w, ambition_time::SimTick>>,
}

impl ActionReplies<'_, '_> {
    fn receiver(&self, reply: &ActionReply) -> Option<Entity> {
        let mut found = self.scopes.iter()
            .filter(|(_, session, occurrence)| reply.recipient.matches(session, occurrence))
            .map(|(entity, ..)| entity);
        let only = found.next()?;
        found.next().is_none().then_some(only)
    }
    pub fn is_pending(&self, reply: Option<&ActionReply>) -> bool {
        reply.is_none_or(|reply| {
            self.receiver(reply)
                .and_then(|entity| self.mailboxes.get(entity).ok())
                .is_some_and(|mailbox| mailbox.is_pending(&reply.ticket))
        })
    }
    pub fn publish(
        &mut self,
        reply: Option<&ActionReply>,
        stage: ActionStage,
        terminal: bool,
        code: impl Into<String>,
        message: impl Into<String>,
        subjects: Vec<String>,
    ) -> bool {
        let Some(reply) = reply else {
            return true;
        };
        let Some(entity) = self.receiver(reply) else {
            return false;
        };
        let now = self.tick.as_deref().map_or(0, |tick| tick.0);
        self.mailboxes.get_mut(entity).is_ok_and(|mut mailbox| {
            mailbox.publish(ActionResult {
                ticket: reply.ticket.clone(),
                stage,
                terminal,
                tick: now,
                code: code.into(),
                message: message.into(),
                subjects,
            })
        })
    }
}

impl ActionMailbox {
    pub const HISTORY_CAPACITY: usize = HISTORY_LIMIT;
    fn producer(&mut self, entry: &str, domain: &str) -> usize {
        match self
            .producers
            .binary_search_by(|p| (p.entry.as_str(), p.domain.as_str()).cmp(&(entry, domain)))
        {
            Ok(index) => index,
            Err(index) => {
                self.producers.insert(
                    index,
                    Producer {
                        entry: entry.into(),
                        domain: domain.into(),
                        high_watermark: None,
                        pending: Vec::new(),
                    },
                );
                index
            }
        }
    }

    fn ticket(&mut self, scope: ObjectOccurrence, entry: &str, domain: &str, request: u32) -> ActionTicket {
        let ticket = ActionTicket {
            entry: entry.into(),
            domain: domain.into(),
            request,
            occurrence: self.next_occurrence,
            room: scope.allocation_room(),
            generation: scope.sequence(),
        };
        self.next_occurrence = self.next_occurrence.saturating_add(1);
        ticket
    }

    fn record(&mut self, result: ActionResult, selector: String, signature: u64, delivered: bool) {
        if self.journal.len() == HISTORY_LIMIT {
            self.journal.remove(0);
            self.dropped = self.dropped.saturating_add(1);
        }
        self.journal.push(JournalRow {
            sequence: self.next_sequence,
            result,
            selector,
            signature,
            delivered,
        });
        self.next_sequence = self.next_sequence.saturating_add(1);
    }

    fn reject(
        &mut self,
        scope: ObjectOccurrence,
        entry: &str,
        domain: &str,
        reply: &ReplyTo,
        signature: u64,
        now: u64,
        code: &str,
    ) {
        let ticket = self.ticket(scope, entry, domain, reply.request);
        self.record(
            ActionResult {
                ticket,
                stage: ActionStage::Rejected,
                terminal: true,
                tick: now,
                code: code.into(),
                message: String::new(),
                subjects: Vec::new(),
            },
            reply.selector.clone(),
            signature,
            false,
        );
    }

    /// Reserve one action before the domain dispatches work. Retries retain
    /// the original occurrence and cannot repeat a mutation.
    pub fn begin(
        &mut self,
        scope: ObjectOccurrence,
        entry: &str,
        domain: &str,
        reply: &ReplyTo,
        signature: u64,
        now: u64,
    ) -> ActionStart {
        let producer = self.producer(entry, domain);
        if let Some(pending) = self.producers[producer]
            .pending
            .iter()
            .find(|p| p.ticket.request == reply.request)
        {
            if pending.signature == signature && pending.selector == reply.selector {
                return ActionStart::Existing(pending.ticket.clone());
            }
            self.reject(scope, entry, domain, reply, signature, now, "request_id_reused");
            return ActionStart::Rejected;
        }
        if self.producers[producer]
            .high_watermark
            .is_some_and(|high| reply.request <= high)
        {
            if let Some(row) = self.journal.iter_mut().rev().find(|row| {
                row.result.terminal
                    && row.result.ticket.entry == entry
                    && row.result.ticket.domain == domain
                    && row.result.ticket.request == reply.request
                    && row.signature == signature
                    && row.selector == reply.selector
            }) {
                row.delivered = false;
                return ActionStart::Existing(row.result.ticket.clone());
            }
            self.reject(scope, entry, domain, reply, signature, now, "stale_request");
            return ActionStart::Rejected;
        }
        self.producers[producer].high_watermark = Some(reply.request);
        if self
            .producers
            .iter()
            .map(|p| p.pending.len())
            .sum::<usize>()
            >= PENDING_LIMIT
        {
            self.reject(
                scope,
                entry,
                domain,
                reply,
                signature,
                now,
                "too_many_pending_actions",
            );
            return ActionStart::Rejected;
        }
        let ticket = self.ticket(scope, entry, domain, reply.request);
        self.producers[producer].pending.push(PendingAction {
            ticket: ticket.clone(),
            signature,
            selector: reply.selector.clone(),
            stage: ActionStage::Submitted,
        });
        self.record(
            ActionResult {
                ticket: ticket.clone(),
                stage: ActionStage::Submitted,
                terminal: false,
                tick: now,
                code: String::new(),
                message: String::new(),
                subjects: Vec::new(),
            },
            reply.selector.clone(),
            signature,
            false,
        );
        ActionStart::New(ticket)
    }

    pub fn is_pending(&self, ticket: &ActionTicket) -> bool {
        self.producers
            .iter()
            .any(|p| p.pending.iter().any(|pending| &pending.ticket == ticket))
    }

    /// Only a pending occurrence can change stage. A canceled or retired
    /// request cannot regain authority when a later callback arrives.
    pub fn publish(&mut self, result: ActionResult) -> bool {
        let Some(producer) = self
            .producers
            .iter_mut()
            .find(|p| p.entry == result.ticket.entry && p.domain == result.ticket.domain)
        else {
            return false;
        };
        let Some(index) = producer
            .pending
            .iter()
            .position(|p| p.ticket == result.ticket)
        else {
            return false;
        };
        let pending = if result.terminal {
            producer.pending.remove(index)
        } else {
            if producer.pending[index].stage == result.stage {
                return true;
            }
            producer.pending[index].stage = result.stage;
            producer.pending[index].clone()
        };
        self.record(result, pending.selector, pending.signature, false);
        true
    }

    /// Results enter a later read cut. An absent subscriber does not consume
    /// a result, and a paused scope does not call this method.
    pub fn take_ready(
        &mut self,
        now: u64,
        mut bound: impl FnMut(&ActionTicket, &str) -> bool,
    ) -> Vec<(String, ActionResult)> {
        let mut ready = Vec::new();
        for row in &mut self.journal {
            if !row.delivered && row.result.tick < now && bound(&row.result.ticket, &row.selector) {
                row.delivered = true;
                ready.push((row.selector.clone(), row.result.clone()));
            }
        }
        ready
    }

    pub fn history(&self) -> ActionResults {
        ActionResults {
            next_sequence: self.next_sequence,
            dropped: self.dropped,
            entries: self
                .journal
                .iter()
                .map(|row| ActionJournalEntry {
                    sequence: row.sequence,
                    result: row.result.clone(),
                })
                .collect(),
        }
    }

    pub fn retire_absent(&mut self, mut present: impl FnMut(&str) -> bool) {
        let retired: std::collections::BTreeSet<_> = self
            .producers
            .iter()
            .filter(|producer| !present(&producer.entry))
            .map(|producer| producer.entry.clone())
            .collect();
        for entry in retired {
            self.retire_entry(&entry);
        }
    }

    /// Entry removal ends reply ownership, not an owning domain's applied work.
    pub fn retire_entry(&mut self, entry: &str) {
        self.producers.retain(|p| p.entry != entry);
        for row in &mut self.journal {
            if row.result.ticket.entry == entry {
                row.delivered = true;
            }
        }
    }
}

ambition_extension_sdk::port_record! {
    PendingAction { ticket: ActionTicket, signature: u64, selector: String, stage: ActionStage }
}
ambition_extension_sdk::port_record! {
    Producer { entry: String, domain: String, high_watermark: Option<u32>, pending: Vec<PendingAction> }
}
ambition_extension_sdk::port_record! {
    JournalRow { sequence: u64, result: ActionResult, selector: String, signature: u64, delivered: bool }
}
ambition_extension_sdk::port_record! {
    ActionMailbox { next_occurrence: u64, next_sequence: u64, dropped: u64, producers: Vec<Producer>, journal: Vec<JournalRow> }
}

impl ambition_platformer2d_core::snapshot::SnapshotState for ActionMailbox {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut bytes = Vec::new();
        ambition_extension_sdk::structure::put(self, &mut bytes);
        ambition_platformer2d_core::snapshot::put_bytes(out, &bytes);
    }
    fn decode(reader: &mut ambition_platformer2d_core::snapshot::Reader<'_>) -> Option<Self> {
        ambition_extension_sdk::wire::decode_all(
            reader.bytes()?,
            ambition_extension_sdk::structure::read::<Self>,
        )
        .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::snapshot::{decode_state, encode_state};

    fn reply(request: u32) -> ReplyTo {
        ReplyTo {
            request,
            selector: "test::actions/result".into(),
        }
    }
    fn start(mailbox: &mut ActionMailbox, request: u32) -> ActionTicket {
        match mailbox.begin(
            ObjectOccurrence { room: 0, generation: 0 },
            "test::actions/ask",
            "test.domain",
            &reply(request),
            u64::from(request),
            10,
        ) {
            ActionStart::New(ticket) => ticket,
            other => panic!("{other:?}"),
        }
    }
    fn applied(ticket: ActionTicket) -> ActionResult {
        ActionResult {
            ticket,
            stage: ActionStage::Applied,
            terminal: true,
            tick: 10,
            code: "applied".into(),
            message: String::new(),
            subjects: vec!["body:recipient".into()],
        }
    }

    #[test]
    fn replies_are_correlated_delivered_later_and_restored_on_rewind() {
        let mut mailbox = ActionMailbox::default();
        let ticket = start(&mut mailbox, 0);
        assert_eq!(
            mailbox.begin(ObjectOccurrence { room: 0, generation: 0 }, "test::actions/ask", "test.domain", &reply(0), 0, 10),
            ActionStart::Existing(ticket.clone())
        );
        assert!(mailbox.publish(applied(ticket.clone())));
        assert!(
            !mailbox.publish(applied(ticket.clone())),
            "a completed occurrence cannot apply again"
        );
        assert!(mailbox.take_ready(10, |_, _| true).is_empty());
        let snapshot = encode_state(&mailbox);
        let expected = mailbox.take_ready(11, |_, _| true);
        assert_eq!(expected.len(), 2);
        assert!(mailbox.take_ready(12, |_, _| true).is_empty());
        let mut restored: ActionMailbox = decode_state(&snapshot).unwrap();
        assert_eq!(restored.take_ready(11, |_, _| true), expected);
        assert_eq!(encode_state(&restored), encode_state(&mailbox));
        assert_eq!(
            mailbox.begin(ObjectOccurrence { room: 0, generation: 0 }, "test::actions/ask", "test.domain", &reply(0), 0, 12),
            ActionStart::Existing(ticket)
        );
    }

    #[test]
    fn an_expired_reply_does_not_let_an_old_request_apply_again() {
        let mut mailbox = ActionMailbox::default();
        for request in 0..HISTORY_LIMIT as u32 {
            let ticket = start(&mut mailbox, request);
            assert!(mailbox.publish(applied(ticket)));
        }
        let history = mailbox.history();
        assert!(history.dropped > 0 && history.entries.len() <= HISTORY_LIMIT);
        assert_eq!(
            mailbox.begin(ObjectOccurrence { room: 0, generation: 0 }, "test::actions/ask", "test.domain", &reply(0), 0, 20),
            ActionStart::Rejected
        );
        assert_eq!(
            mailbox.history().entries.last().unwrap().result.code,
            "stale_request"
        );
    }

    #[test]
    fn a_retired_producer_cannot_finish_work_in_its_replacement() {
        let mut mailbox = ActionMailbox::default();
        let old = start(&mut mailbox, 0);
        mailbox.retire_entry("test::actions/ask");
        assert!(
            mailbox
                .history()
                .entries
                .iter()
                .all(|entry| entry.result.stage != ActionStage::Canceled)
        );
        let new = start(&mut mailbox, 0);
        assert_ne!(old.occurrence, new.occurrence);
        assert!(!mailbox.publish(applied(old)));
        assert!(mailbox.is_pending(&new));
        assert!(mailbox.publish(applied(new)));
    }

    #[test]
    fn pending_work_has_a_bound_and_refusal_preserves_existing_work() {
        let mut mailbox = ActionMailbox::default();
        let tickets: Vec<_> = (0..PENDING_LIMIT as u32)
            .map(|request| start(&mut mailbox, request))
            .collect();
        assert_eq!(
            mailbox.begin(
            ObjectOccurrence { room: 0, generation: 0 },
                "test::actions/ask",
                "test.domain",
                &reply(PENDING_LIMIT as u32),
                100,
                10
            ),
            ActionStart::Rejected
        );
        assert!(tickets.iter().all(|ticket| mailbox.is_pending(ticket)));
        assert_eq!(
            mailbox.history().entries.last().unwrap().result.code,
            "too_many_pending_actions"
        );
    }
    #[test]
    fn deferred_work_cannot_complete_in_a_recreated_body_with_the_same_identity() {
        use crate::lifecycle::{RoomInstanceRoot, SessionScopeId};
        let mut world = World::new();
        let session = SessionScopedEntity(SessionScopeId(1));
        let room = LiveRoomInstance::ACTIVATION;
        world.spawn((RoomInstanceRoot, session, room));
        let identity = SimId::placement("recreated");
        let body = world
            .spawn((session, InRoomInstance(room), identity.clone()))
            .id();
        let old = begin_action(
            &mut world,
            body,
            "test::actions/ask",
            "test.domain",
            Some(&reply(0)),
            0,
        )
        .reply
        .unwrap();
        world.despawn(body);
        let body = world.spawn((session, InRoomInstance(room), identity)).id();
        let new = begin_action(
            &mut world,
            body,
            "test::actions/ask",
            "test.domain",
            Some(&reply(0)),
            0,
        )
        .reply
        .unwrap();
        assert_ne!(old.ticket.generation, new.ticket.generation);
        assert!(!publish_action_result(
            &mut world,
            Some(&old),
            ActionStage::Applied,
            true,
            "",
            "",
            Vec::new()
        ));
        assert!(action_is_pending(&world, Some(&new)));
    }

    #[test]
    fn equal_authored_ids_keep_distinct_recipients_through_travel_and_reconstruction() {
        use crate::lifecycle::{live_room_root, SessionScopeId};
        use bevy::ecs::system::SystemState;
        let mut world = World::new();
        let session = SessionScopedEntity(SessionScopeId(1));
        let rooms = (0..4).map(LiveRoomInstance::from_ordinal).collect::<Vec<_>>();
        for room in &rooms { world.spawn(live_room_root(session.0, *room)); }
        let id = SimId::placement("shared");
        let a = world.spawn((session, InRoomInstance(rooms[0]), id.clone())).id();
        let b = world.spawn((session, InRoomInstance(rooms[1]), id.clone())).id();
        let untracked = world.spawn((session, InRoomInstance(rooms[2]), id.clone())).id();
        let ask = |world: &mut World, entity| begin_action(world, entity,
            "test::actions/ask", "test.domain", Some(&reply(0)), 0).reply.unwrap();
        let first = ask(&mut world, a);
        let second = ask(&mut world, b);
        assert!(world.get::<ActionMailbox>(untracked).is_none());
        assert_ne!(first.recipient.occurrence, second.recipient.occurrence);
        let saved_identity = encode_state(world.get::<ObjectOccurrence>(a).unwrap());
        let saved_mailbox = encode_state(world.get::<ActionMailbox>(a).unwrap());
        world.entity_mut(a).insert(InRoomInstance(rooms[3]));
        assert_eq!(first.recipient.resolve(&world), Some(a));
        assert_eq!(second.recipient.resolve(&world), Some(b));
        let mut state = SystemState::<ActionReplies>::new(&mut world);
        {
            let mut replies = state.get_mut(&mut world).unwrap();
            assert!(replies.is_pending(Some(&first)));
            assert!(replies.is_pending(Some(&second)));
            assert!(replies.publish(Some(&first), ActionStage::Applied, true, "first", "", Vec::new()));
            assert!(replies.publish(Some(&second), ActionStage::Applied, true, "second", "", Vec::new()));
        }
        for (entity, code) in [(a, "first"), (b, "second")] {
            assert_eq!(world.get::<ActionMailbox>(entity).unwrap().history().entries.last().unwrap().result.code, code);
        }
        world.despawn(a);
        let replacement = world.spawn((session, InRoomInstance(rooms[3]), id.clone())).id();
        let replacement_reply = ask(&mut world, replacement);
        assert_ne!(first.recipient.occurrence, replacement_reply.recipient.occurrence);
        assert_eq!(first.recipient.resolve(&world), None);
        assert!(!publish_action_result(&mut world, Some(&first), ActionStage::Applied, true, "stale", "", Vec::new()));
        let restored = world.spawn((session, InRoomInstance(rooms[0]), id,
            decode_state::<ObjectOccurrence>(&saved_identity).unwrap(),
            decode_state::<ActionMailbox>(&saved_mailbox).unwrap())).id();
        assert_eq!(first.recipient.resolve(&world), Some(restored));
        let mut replies = state.get_mut(&mut world).unwrap();
        assert!(replies.publish(Some(&first), ActionStage::Applied, true, "restored", "", Vec::new()));
        assert!(replies.is_pending(Some(&replacement_reply)));
    }
}
