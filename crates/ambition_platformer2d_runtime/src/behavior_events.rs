//! Typed native facts routed to named methods through the addressed inbox.

use std::collections::{BTreeMap, BTreeSet};
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations,
    InvocationEvent, InvocationReceipt, InvocationScope};
use ambition_extension_sdk::{ObjectRef, Port, PortKey, action::DeliveryProgress, phases::ACTOR_THINK};
use ambition_platformer2d_core::snapshot::{SnapshotState, Reader, put_bytes};
use ambition_platformer2d_shared_tangle::lifecycle::*;
use ambition_world_port::behavior::*;
use bevy::{ecs::{lifecycle::HookContext, world::DeferredWorld}, prelude::*};
use crate::object_behaviors::BehaviorInstance;

type Dispatch = fn(&mut ExtensionInvocations, &str, &str, InvocationScope, &[u8], InvocationReceipt)
    -> Result<(), ambition_extension_sdk::wire::WireError>;
/// A source's events: one fixed selector, or (authored signals) one event per
/// topic, the subscribing entry's selector naming the topic.
#[derive(Clone)]
pub(crate) struct NativeSource { pub port: PortKey, pub selector: String, pub topical: bool, pub dispatch: Dispatch }
impl NativeSource {
    /// The routing key of this source's event at `selector`. A topic is kept
    /// behind `#`, so no topic can spell another source's key.
    fn event(&self, selector: &str) -> String {
        if self.topical { format!("{}#{selector}", self.port) } else { self.port.to_string() }
    }
}
#[derive(Resource, Default)]
pub(crate) struct NativeSources(BTreeMap<String, NativeSource>);
impl NativeSources {
    /// The source of a routed event and the selector its entries carry.
    pub(crate) fn get<'a>(&'a self, event: &'a str) -> Option<(&'a NativeSource, &'a str)> {
        match event.split_once('#') {
            Some((port, topic)) => self.0.get(port).filter(|source| source.topical).map(|source| (source, topic)),
            None => self.0.get(event).filter(|source| !source.topical).map(|source| (source, source.selector.as_str())),
        }
    }
    pub(crate) fn for_entry(&self, entry: &ambition_extension_host::AdmittedEntry) -> Option<&NativeSource> {
        let selector = entry.descriptor.trigger.selector.as_ref();
        self.0.get(&entry.descriptor.trigger.port.to_string()).filter(|source| entry.descriptor.phase == ACTOR_THINK
            && if source.topical { valid_signal_topic(selector) } else { selector == source.selector })
    }
}

/// Install a typed native fact source. The domain owns publication of its values.
/// Facts wait for a later actor read cut. This does not install a physics authority.
pub fn install_native_source<P: Port>(app: &mut App, owner: &'static str, selector: &str) {
    install_source::<P>(app, owner, selector, false);
}
fn install_source<P: Port>(app: &mut App, owner: &'static str, selector: &str, topical: bool) {
    assert_eq!(P::ROLE, ambition_extension_sdk::PortRole::Trigger);
    let source = NativeSource {port: P::KEY, selector: selector.into(), topical, dispatch: dispatch::<P>};
    let mut sources = app.world_mut().get_resource_or_init::<NativeSources>();
    assert!(sources.0.insert(P::KEY.to_string(), source).is_none(), "a native event has one source contract");
    app.install_extension_trigger::<P, _>(ACTOR_THINK, owner,
        crate::behavior_inbox::collect::<EventMailbox>(crate::behavior_inbox::Lane::Fact));
}
fn dispatch<P: Port>(invocations: &mut ExtensionInvocations, entry: &str, selector: &str,
    scope: InvocationScope, bytes: &[u8], receipt: InvocationReceipt)
    -> Result<(), ambition_extension_sdk::wire::WireError> {
    let value = ambition_extension_sdk::wire::decode_all(bytes, P::decode)?;
    invocations.trigger_entry_with_receipt::<P>(&ACTOR_THINK, entry.to_owned(), selector.to_owned(), scope, value, receipt);
    Ok(())
}

#[derive(Clone)]
pub(crate) struct PreparedSubscription { key: String, source: String, method: String, event: String }
#[derive(Clone, Debug, PartialEq, Eq)]
struct Subscription { key: String, source: ObjectRef, method: String, event: String }
ambition_extension_sdk::port_record! { Subscription { key: String, source: ObjectRef, method: String, event: String } }
#[derive(Clone, Debug, PartialEq, Eq)]
struct EventRow { delivery: BehaviorEventDelivery, bytes: Vec<u8>, max_attempts: u32 }
ambition_extension_sdk::port_record! { EventRow { delivery: BehaviorEventDelivery, bytes: Vec<u8>, max_attempts: u32 } }

#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
#[component(on_insert = insert_routes, on_remove = remove_routes)]
pub(crate) struct EventMailbox {
    // Connections are fixed for this occurrence. In-place journal restoration
    // cannot change routing; insertion and retirement update the derived index.
    subscriptions: Vec<Subscription>, rows: Vec<EventRow>, next_sequence: u64, dropped: u64,
    capacity: u32, max_bytes: u32,
}
ambition_extension_sdk::port_record! {
    EventMailbox { subscriptions: Vec<Subscription>, rows: Vec<EventRow>, next_sequence: u64,
        dropped: u64, capacity: u32, max_bytes: u32 }
}
impl SnapshotState for EventMailbox {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut bytes = Vec::new(); ambition_extension_sdk::structure::put(self, &mut bytes); put_bytes(out, &bytes);
    }
    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        ambition_extension_sdk::wire::decode_all(reader.bytes()?, ambition_extension_sdk::structure::read).ok()
    }
}
#[derive(Resource, Default)]
struct EventRoutes {
    routes: BTreeMap<(ObjectRef, String), Vec<(Entity, String)>>,
    reverse: BTreeMap<Entity, BTreeSet<(ObjectRef, String)>>,
}
impl EventRoutes {
    fn remove(&mut self, entity: Entity) {
        for key in self.reverse.remove(&entity).unwrap_or_default() {
            if let Some(receivers) = self.routes.get_mut(&key) {
                receivers.retain(|(receiver, _)| *receiver != entity);
                if receivers.is_empty() { self.routes.remove(&key); }
            }
        }
    }
}
fn insert_routes(mut world: DeferredWorld, context: HookContext) {
    let subscriptions = world.get::<EventMailbox>(context.entity).unwrap().subscriptions.clone();
    let Some(mut routes) = world.get_resource_mut::<EventRoutes>() else { return; };
    routes.remove(context.entity);
    for row in subscriptions {
        let key = (row.source, row.event);
        routes.routes.entry(key.clone()).or_default().push((context.entity, row.key));
        routes.reverse.entry(context.entity).or_default().insert(key);
    }
}
fn remove_routes(mut world: DeferredWorld, context: HookContext) {
    if let Some(mut routes) = world.get_resource_mut::<EventRoutes>() { routes.remove(context.entity); }
}

pub(crate) fn prepare(world: &World, methods: &[BehaviorMethod], references: &[(String, ambition_world_port::object_target::ObjectTarget)],
    subscriptions: &[BehaviorSubscription], max: u32) -> Result<Vec<PreparedSubscription>, String> {
    if subscriptions.len() > max as usize { return Err("The native subscription count exceeds its game bound.".into()); }
    let admitted = &world.resource::<AdmittedExtensions>().0;
    let sources = world.resource::<NativeSources>();
    let mut keys = BTreeSet::new();
    subscriptions.iter().map(|subscription| {
        if subscription.key.is_empty() || subscription.key.len() > 64 || !keys.insert(&subscription.key) {
            return Err("Subscription keys must be unique, nonempty and at most 64 bytes.".into());
        }
        if !references.iter().any(|(key, _)| key == &subscription.source) {
            return Err(format!("Subscription {} has no named source reference {}.", subscription.key, subscription.source));
        }
        let method = methods.iter().find(|method| method.name == subscription.method)
            .ok_or_else(|| format!("Subscription {} has no method {}.", subscription.key, subscription.method))?;
        let entry = admitted.entries.iter().find(|entry| entry.path == method.entry)
            .ok_or_else(|| format!("Subscription {} has no admitted entry {}.", subscription.key, method.entry))?;
        let source = sources.for_entry(entry)
            .ok_or_else(|| format!("Subscription {} method {} is not bound to a native fact source and selector.", subscription.key, method.name))?;
        Ok(PreparedSubscription {key: subscription.key.clone(), source: subscription.source.clone(),
            method: method.name.clone(), event: source.event(entry.descriptor.trigger.selector.as_ref())})
    }).collect()
}
pub(crate) fn initialized(subscriptions: &[PreparedSubscription], references: &[(String, ObjectRef)],
    capacity: u32, max_bytes: u32) -> EventMailbox {
    let subscriptions = subscriptions.iter().map(|row| Subscription {key: row.key.clone(),
        source: references.iter().find(|(key, _)| key == &row.source).expect("preparation checked the source reference").1,
        method: row.method.clone(), event: row.event.clone()}).collect();
    EventMailbox {subscriptions, capacity, max_bytes, ..Default::default()}
}

/// Publication reports delivery limits without changing the native fact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativePublication { pub queued: u32, pub overflow: u32, pub unavailable: u32, pub cross_island: u32 }

/// Publish only after the native domain commits the reported fact. The typed
/// source must be installed. Scripts cannot invoke this function as a request.
pub fn publish_native<P: Port>(world: &mut World, source: ObjectRef, value: &P::Value) -> NativePublication {
    publish_deferred::<P>(&mut world.into(), source, value)
}
fn publish_deferred<P: Port>(world: &mut DeferredWorld<'_>, source: ObjectRef, value: &P::Value) -> NativePublication {
    let event = P::KEY.to_string();
    assert!(world.resource::<NativeSources>().get(&event).is_some(), "the native fact source must be installed");
    if world.resource::<EventRoutes>().routes.get(&(source, event)).is_none_or(Vec::is_empty) {
        return NativePublication::default();
    }
    let objects = ObjectIndex::capture(world);
    let Ok(entity) = objects.resolve(SessionScopeId(source.session), source) else {
        return NativePublication {unavailable: 1, ..Default::default()};
    };
    let Some(authority) = simulation_island(world, entity) else {
        return NativePublication {unavailable: 1, ..Default::default()};
    };
    publish_captured::<P>(world, source, authority, value)
}
fn publish_captured<P: Port>(world: &mut DeferredWorld<'_>, source: ObjectRef, authority: SimulationIsland,
    value: &P::Value) -> NativePublication {
    let event = P::KEY.to_string();
    assert!(world.resource::<NativeSources>().get(&event).is_some(), "the native fact source must be installed");
    let mut bytes = Vec::new(); P::encode(value, &mut bytes);
    let recipients = recipients(world, source, &event);
    publish_encoded(world, source, authority, &event, recipients, &bytes)
}
/// The live mailboxes subscribed to `event` from `source`, in delivery order.
fn recipients(world: &DeferredWorld<'_>, source: ObjectRef, event: &str) -> Vec<(ObjectRef, Entity, String)> {
    let routes = world.resource::<EventRoutes>().routes.get(&(source, event.to_owned())).cloned().unwrap_or_default();
    if routes.is_empty() { return Vec::new(); }
    let objects = ObjectIndex::capture(world);
    let mut recipients: Vec<_> = routes.into_iter().filter_map(|(entity, key)| {
        let reference = object_reference(world, entity)?;
        (objects.resolve(SessionScopeId(source.session), reference) == Ok(entity)).then_some((reference, entity, key))
    }).collect();
    recipients.sort_by(|a, b| (a.0, &a.2).cmp(&(b.0, &b.2)));
    recipients
}
/// Whether `mailbox` takes a row of `len` bytes now, and which settled row it
/// gives up for it.
fn room_for(mailbox: &EventMailbox, len: usize) -> Result<Option<usize>, ()> {
    if len > mailbox.max_bytes as usize || mailbox.capacity == 0 || mailbox.next_sequence == u64::MAX { return Err(()); }
    if mailbox.rows.len() < mailbox.capacity as usize { return Ok(None); }
    mailbox.rows.iter().position(|row| !row.delivery.progress.pending()).map(Some).ok_or(())
}
fn publish_encoded(world: &mut DeferredWorld<'_>, source: ObjectRef, authority: SimulationIsland, event: &str,
    recipients: Vec<(ObjectRef, Entity, String)>, bytes: &[u8]) -> NativePublication {
    let now = world.resource::<ambition_time::SimTick>().0;
    let mut report = NativePublication::default();
    for (_, entity, key) in recipients {
        if simulation_island(world, entity) != Some(authority) {
            report.cross_island += 1;
            continue;
        }
        let mut mailbox = world.get_mut::<EventMailbox>(entity).unwrap();
        match room_for(&mailbox, bytes.len()) {
            Err(()) => { mailbox.dropped = mailbox.dropped.saturating_add(1); report.overflow += 1; continue; }
            Ok(Some(settled)) => { mailbox.rows.remove(settled); }
            Ok(None) => {}
        }
        let sequence = mailbox.next_sequence; mailbox.next_sequence += 1;
        mailbox.rows.push(EventRow {delivery: BehaviorEventDelivery {sequence, subscription: key, source,
            event: event.to_owned(), tick: now, progress: DeliveryProgress::default()},
            bytes: bytes.to_vec(), max_attempts: 1});
        report.queued += 1;
    }
    report
}

/// The most methods one signal reaches. A signal admitted to fewer than all
/// its subscribers would be a partial fact, so a larger set refuses.
pub const MAX_SIGNAL_RECIPIENTS: usize = 16;

/// Admit `signal` from `source` to every method subscribed to it, or to none.
/// Returns the recipients' objects, in delivery order.
pub(crate) fn admit_signal(world: &mut World, source: ObjectRef, authority: SimulationIsland, signal: &ObjectSignal)
    -> Result<Vec<ObjectRef>, &'static str> {
    if !valid_signal_topic(&signal.topic) { return Err("invalid_signal_topic"); }
    let event = world.resource::<NativeSources>().0.get(&ObjectSignalPort::KEY.to_string())
        .filter(|source| source.topical).ok_or("signals_not_installed")?.event(&signal.topic);
    let mut world = DeferredWorld::from(world);
    let recipients = recipients(&world, source, &event);
    if recipients.len() > MAX_SIGNAL_RECIPIENTS { return Err("signal_recipient_bound"); }
    let mut bytes = Vec::new(); ObjectSignalPort::encode(signal, &mut bytes);
    // Every recipient is checked before any changes: a refusal leaves all as they were.
    for (_, entity, _) in &recipients {
        if simulation_island(&world, *entity) != Some(authority) { return Err("cross_island_signal_unsupported"); }
        let mailbox = world.get::<EventMailbox>(*entity).ok_or("signal_recipient_unavailable")?;
        if bytes.len() > mailbox.max_bytes as usize { return Err("signal_data_bound"); }
        room_for(mailbox, bytes.len()).map_err(|()| "signal_queue_full")?;
    }
    // A behavior's mailbox is its own occurrence: the sender hears of the object it runs for.
    let heard: Vec<_> = recipients.iter().map(|(reference, entity, _)|
        world.get::<ObjectScopeOwner>(*entity).map_or(*reference, |owner| owner.0)).collect();
    let report = publish_encoded(&mut world, source, authority, &event, recipients, &bytes);
    debug_assert_eq!(report.queued as usize, heard.len(), "admission checked every recipient");
    Ok(heard)
}

impl EventMailbox {
    pub(crate) fn deliveries<'a>(&'a self, instance: &'a BehaviorInstance)
        -> impl Iterator<Item = (u64, u64, String, &'a str)> + 'a {
        self.rows.iter().filter(|row| row.delivery.progress.pending()).filter_map(|row| {
            let subscription = self.subscriptions.iter().find(|sub| sub.key == row.delivery.subscription)?;
            Some((row.delivery.sequence, row.delivery.tick,
                instance.method_entry(&subscription.method)?.to_owned(), row.delivery.event.as_str()))
        })
    }
    pub(crate) fn queue_delivery(&mut self, sequence: u64, now: u64, bound: bool, attempts: u32) -> Option<Vec<u8>> {
        let row = self.rows.iter_mut().find(|row| row.delivery.sequence == sequence)?;
        if !row.delivery.progress.queue(now, row.delivery.tick, bound, "native_event_method_unbound") { return None; }
        row.max_attempts = attempts;
        Some(row.bytes.clone())
    }
    pub(crate) fn invalid_payload(&mut self, sequence: u64) {
        if let Some(row) = self.rows.iter_mut().find(|row| row.delivery.sequence == sequence) {
            row.delivery.progress.retire("native_event_payload_invalid");
        }
    }
    fn history(&self) -> BehaviorEventHistory {
        BehaviorEventHistory {deliveries: self.rows.iter().map(|row| row.delivery.clone()).collect(), dropped: self.dropped}
    }
}
pub(crate) fn settle(world: &mut World, state: Entity, sequence: u64, event: InvocationEvent) {
    let now = world.resource::<ambition_time::SimTick>().0;
    if let Some(mut mailbox) = world.get_mut::<EventMailbox>(state) {
        if let Some(row) = mailbox.rows.iter_mut().find(|row| row.delivery.sequence == sequence) {
            row.delivery.progress.settle(now, event, row.max_attempts);
        }
    }
}

pub(crate) fn register_rollback_state(registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar) {
    registrar.rollback_component_canonical::<EventMailbox>("ambition_platformer2d_runtime", "script.native_event_mailbox");
    registrar.clear_message_on_rollback::<DefeatCommitted>("ambition_platformer2d_runtime", "message.defeat_committed");
    registrar.clear_message_on_rollback::<BrokenCommitted>("ambition_platformer2d_runtime", "message.broken_committed");
    registrar.clear_message_on_rollback::<CollectedCommitted>("ambition_platformer2d_runtime", "message.collected_committed");
    registrar.declare_rollback_derived_resource::<EventRoutes>("ambition_platformer2d_runtime", "derived.native_event_routes",
        "Subscription insertion and removal update the source index, including canonical snapshot restoration.");
}
pub(crate) fn install(app: &mut App) {
    app.init_resource::<EventRoutes>();
    app.add_message::<ambition_platformer2d_interaction::ActivationCommitted>();
    clear_message_at_session_activation::<ambition_platformer2d_interaction::ActivationCommitted>(app);
    install_native_source::<ObjectActivatedPort>(app, "ambition_platformer2d_interaction", "activated");
    app.install_extension_context_observation::<ObjectActivationPort>(ACTOR_THINK, "ambition_platformer2d_interaction",
        |world, context| Some(world.get::<ambition_platformer2d_interaction::ActivationState>(context.scope).and_then(|state|
            Some(ObjectActivated {object: object_reference(world, context.scope)?, actor: state.actor, revision: state.revision}))));
    use ambition_platformer2d_shared_tangle::schedule::{SimScheduleExt, Platformer2dSimulationPhase};
    let schedule = app.sim_schedule();
    app.add_systems(schedule, publish_activations.in_set(Platformer2dSimulationPhase::GameplayEffects)
        .after(Platformer2dSimulationPhase::FeatureInteraction));
    app.add_message::<DefeatCommitted>();
    clear_message_at_session_activation::<DefeatCommitted>(app);
    install_native_source::<ObjectDefeatedPort>(app, "ambition_platformer2d_actor_monolith", "defeated");
    app.add_systems(schedule, publish_defeats.in_set(Platformer2dSimulationPhase::GameplayEffects)
        .after(ambition_platformer2d_actor_monolith::body::apply_feature_hit_events));
    app.add_message::<BrokenCommitted>();
    clear_message_at_session_activation::<BrokenCommitted>(app);
    install_native_source::<ObjectBrokenPort>(app, "ambition_breakables", "broken");
    app.add_message::<CollectedCommitted>();
    clear_message_at_session_activation::<CollectedCommitted>(app);
    install_native_source::<ObjectCollectedPort>(app, "ambition_platformer2d_items", "collected");
    app.add_systems(schedule, publish_collected.in_set(Platformer2dSimulationPhase::GameplayEffects)
        .after(ambition_platformer2d_actor_monolith::body::apply_feature_hit_events));
    app.add_systems(schedule, publish_broken.in_set(Platformer2dSimulationPhase::GameplayEffects)
        .after(ambition_platformer2d_actor_monolith::body::apply_feature_hit_events));
    install_native_source::<ObjectBornPort>(app, "ambition_platformer2d_runtime", "born");
    install_source::<ObjectSignalPort>(app, "ambition_platformer2d_runtime", "", true);
    ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, "objects.emit_signal",
        ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    app.install_extension_submitted_request::<EmitSignalPort, _>(ACTOR_THINK, "ambition_platformer2d_runtime", emit_signals);
    app.install_extension_context_observation::<BehaviorEventsPort>(ACTOR_THINK, "ambition_platformer2d_runtime",
        |world, context| Some(world.get::<EventMailbox>(context.state_scope).map_or_else(BehaviorEventHistory::default, EventMailbox::history)));
    app.add_observer(|notice: On<ObjectPublished>, mut world: DeferredWorld| {
        let reference = notice.occurrence.reference(notice.session);
        let report = publish_deferred::<ObjectBornPort>(&mut world, reference, &ObjectBorn {object: reference});
        if report.overflow != 0 || report.cross_island != 0 || report.unavailable != 0 {
            warn!(?reference, ?report, "Native birth subscription delivery was limited.");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::snapshot::{decode_state, encode_state};
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    #[test]
    fn facts_keep_occurrence_identity_and_never_write_an_independent_room() {
        let mut world = World::new();
        world.init_resource::<EventRoutes>();
        world.insert_resource(ambition_time::SimTick(10));
        world.insert_resource(NativeSources(BTreeMap::from([(ObjectBornPort::KEY.to_string(), NativeSource {
            port: ObjectBornPort::KEY, selector: "born".into(), topical: false, dispatch: dispatch::<ObjectBornPort>} )])));
        let session = SessionScopeId(1);
        let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        let bodies = rooms.map(|room| {
            world.spawn((RoomInstanceRoot, room, SessionScopedEntity(session)));
            let body = world.spawn((SessionScopedEntity(session), InRoomInstance(room), SimId::placement("equal"))).id();
            ensure_object_occurrence(&mut world, body).unwrap();
            body
        });
        let source = object_reference(&world, bodies[0]).unwrap();
        let others = bodies.map(|body| object_reference(&world, body).unwrap());
        assert_ne!(others[0], others[1]);
        let receivers: Vec<_> = bodies.iter().enumerate().map(|(index, body)| {
            let mailbox = initialized(&[PreparedSubscription {key: "ready".into(), source: "peer".into(),
                method: "ready".into(), event: ObjectBornPort::KEY.to_string()}], &[("peer".into(), source)], 1, 1024);
            let receiver = world.spawn((SessionScopedEntity(session), ObjectScopeOwner(others[index]),
                SimId::placement("private"), mailbox)).id();
            let room = object_scope_room(&world, *body).unwrap();
            PreparedObjectOccurrences::prepare(&mut world, session, room, 1).unwrap()
                .assign(&mut world, &[receiver]).unwrap();
            receiver
        }).collect();
        let baseline = encode_state(world.get::<EventMailbox>(receivers[0]).unwrap());
        world.insert_resource(SimulationIslands(|_| true));
        assert_eq!(publish_native::<ObjectBornPort>(&mut world, source, &ObjectBorn {object: source}),
            NativePublication {queued: 1, cross_island: 1, ..Default::default()});
        assert!(world.get::<EventMailbox>(receivers[1]).unwrap().rows.is_empty());
        assert_eq!(publish_native::<ObjectBornPort>(&mut world, others[1], &ObjectBorn {object: others[1]}), NativePublication::default());
        world.remove_resource::<SimulationIslands>();
        let report = publish_native::<ObjectBornPort>(&mut world, source, &ObjectBorn {object: source});
        assert_eq!((report.queued, report.overflow), (1, 1));
        assert_eq!(world.get::<EventMailbox>(receivers[0]).unwrap().dropped, 1);
        world.entity_mut(receivers[0]).insert(decode_state::<EventMailbox>(&baseline).unwrap());
        world.insert_resource(SimulationIslands(|_| true));
        world.entity_mut(bodies[1]).insert(InRoomInstance(rooms[0]));
        let report = publish_native::<ObjectBornPort>(&mut world, source, &ObjectBorn {object: source});
        assert_eq!((report.queued, report.overflow, report.cross_island), (1, 1, 0));
        assert_eq!(world.get::<EventMailbox>(receivers[0]).unwrap().rows[0].delivery.source, source);
        let authority = simulation_island(&world, bodies[0]).unwrap();
        world.entity_mut(receivers[1]).insert(decode_state::<EventMailbox>(&baseline).unwrap());
        world.despawn(bodies[0]);
        let report = publish_captured::<ObjectBornPort>(&mut DeferredWorld::from(&mut world), source, authority, &ObjectBorn {object: source});
        assert_eq!(report.queued, 1, "captured birth provenance survives source retirement");
        assert_eq!(world.get::<EventMailbox>(receivers[1]).unwrap().rows[0].delivery.source, source);

    }
}


/// Admit each emitted signal whole or refuse it, and answer its sender. The
/// sender is the object its script runs for; a script cannot name another.
fn emit_signals(world: &mut World) {
    use ambition_platformer2d_shared_tangle::action_results::{begin_action_request, publish_action_result};
    use ambition_extension_sdk::action::{ActionStage, ActionSubject};
    let requests = world.resource_mut::<ambition_extension_host::ExtensionOutbox>().drain::<EmitSignalPort>(&ACTOR_THINK);
    for submitted in requests {
        let request = submitted.value;
        let dispatch = begin_action_request::<EmitSignalPort>(world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute { continue; }
        let result = (|| -> Result<Vec<ObjectRef>, &'static str> {
            if !valid_signal_topic(&request.topic) { return Err("invalid_signal_topic"); }
            ensure_object_occurrence(world, submitted.scope).ok_or("emitter_without_identity")?;
            let from = object_reference(world, submitted.scope).ok_or("emitter_without_identity")?;
            let authority = simulation_island(world, submitted.scope).ok_or("emitter_without_authority")?;
            let bound = crate::object_behaviors::limits(world, submitted.scope).max_data_bytes;
            let data = crate::object_behaviors::checked_data(&request.data, bound).ok_or("signal_data_bound")?;
            admit_signal(world, from, authority, &ObjectSignal {from, topic: request.topic.clone(), data})
        })();
        let (stage, code, subjects) = match result {
            Ok(heard) => (ActionStage::Applied, "signal_admitted", heard.into_iter()
                .map(|object| ActionSubject {key: "recipient".into(), object: Some(object)}).collect()),
            Err(code) => (ActionStage::Rejected, code, Vec::new()),
        };
        publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, "", subjects);
    }
}

type ActivationReads = bevy::ecs::system::SystemState<ambition_platformer2d_core::replay_messages::ReplayMessageReader<'static, 'static, ambition_platformer2d_interaction::ActivationCommitted>>;
fn publish_activations(world: &mut World, mut reader: Local<Option<ActivationReads>>) {
    let reader = reader.get_or_insert_with(|| ActivationReads::new(world));
    let facts: Vec<_> = reader.get_mut(world).expect("activation messages are installed").read().cloned().collect();
    for fact in facts {
        let authority = if world.get_resource::<SimulationIslands>().is_some_and(|policy| (policy.0)(world)) {
            SimulationIsland::Room(SessionScopeId(fact.object.session), fact.room)
        } else { SimulationIsland::Session(SessionScopeId(fact.object.session)) };
        let value = ObjectActivated {object: fact.object, actor: fact.actor, revision: fact.revision};
        let report = publish_captured::<ObjectActivatedPort>(&mut DeferredWorld::from(&mut *world), fact.object, authority, &value);
        if report.overflow != 0 || report.cross_island != 0 || report.unavailable != 0 {
            warn!(?report, object = ?fact.object, "Native activation notification delivery was limited; inspect its activation state to reconcile.");
        }
    }
}

type DefeatReads = bevy::ecs::system::SystemState<ambition_platformer2d_core::replay_messages::ReplayMessageReader<'static, 'static, DefeatCommitted>>;
/// Route each committed defeat to the methods subscribed to the defeated object.
fn publish_defeats(world: &mut World, mut reader: Local<Option<DefeatReads>>) {
    let reader = reader.get_or_insert_with(|| DefeatReads::new(world));
    let facts: Vec<_> = reader.get_mut(world).expect("defeat messages are installed").read().cloned().collect();
    for fact in facts {
        let authority = if world.get_resource::<SimulationIslands>().is_some_and(|policy| (policy.0)(world)) {
            SimulationIsland::Room(SessionScopeId(fact.object.session), fact.room)
        } else { SimulationIsland::Session(SessionScopeId(fact.object.session)) };
        let value = ObjectDefeated {object: fact.object, by: fact.by};
        let report = publish_captured::<ObjectDefeatedPort>(&mut DeferredWorld::from(&mut *world), fact.object, authority, &value);
        if report.overflow != 0 || report.cross_island != 0 || report.unavailable != 0 {
            warn!(?report, object = ?fact.object, "Native defeat notification delivery was limited.");
        }
    }
}

type BrokenReads = bevy::ecs::system::SystemState<ambition_platformer2d_core::replay_messages::ReplayMessageReader<'static, 'static, BrokenCommitted>>;
/// Route each committed break to the methods subscribed to the spent object.
fn publish_broken(world: &mut World, mut reader: Local<Option<BrokenReads>>) {
    let reader = reader.get_or_insert_with(|| BrokenReads::new(world));
    let facts: Vec<_> = reader.get_mut(world).expect("break messages are installed").read().cloned().collect();
    for fact in facts {
        let authority = if world.get_resource::<SimulationIslands>().is_some_and(|policy| (policy.0)(world)) {
            SimulationIsland::Room(SessionScopeId(fact.object.session), fact.room)
        } else { SimulationIsland::Session(SessionScopeId(fact.object.session)) };
        let report = publish_captured::<ObjectBrokenPort>(&mut DeferredWorld::from(&mut *world), fact.object, authority,
            &ObjectBroken {object: fact.object});
        if report.overflow != 0 || report.cross_island != 0 || report.unavailable != 0 {
            warn!(?report, object = ?fact.object, "Native break notification delivery was limited.");
        }
    }
}

type CollectedReads = bevy::ecs::system::SystemState<ambition_platformer2d_core::replay_messages::ReplayMessageReader<'static, 'static, CollectedCommitted>>;
/// Route each committed take to the methods subscribed to the taken pickup.
fn publish_collected(world: &mut World, mut reader: Local<Option<CollectedReads>>) {
    let reader = reader.get_or_insert_with(|| CollectedReads::new(world));
    let facts: Vec<_> = reader.get_mut(world).expect("take messages are installed").read().cloned().collect();
    for fact in facts {
        let authority = if world.get_resource::<SimulationIslands>().is_some_and(|policy| (policy.0)(world)) {
            SimulationIsland::Room(SessionScopeId(fact.object.session), fact.room)
        } else { SimulationIsland::Session(SessionScopeId(fact.object.session)) };
        let report = publish_captured::<ObjectCollectedPort>(&mut DeferredWorld::from(&mut *world), fact.object, authority,
            &ObjectCollected {object: fact.object, by: fact.by});
        if report.overflow != 0 || report.cross_island != 0 || report.unavailable != 0 {
            warn!(?report, object = ?fact.object, "Native take notification delivery was limited.");
        }
    }
}
