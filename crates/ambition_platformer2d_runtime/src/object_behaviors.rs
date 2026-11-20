//! Private reusable behavior instances and bounded directed object calls.

use std::collections::{BTreeMap, BTreeSet};
use ambition_extension_host::{AdmittedExtensions, BodyRecords, ExtensionAppExt,
    ExtensionOutbox, InvocationEvent, ObservationContext};
use ambition_extension_sdk::{ObjectRef, Port, action::*, phases::ACTOR_THINK, script::ScriptValue};
use ambition_platformer2d_core::snapshot::{SnapshotState, Reader, put_bytes};
use ambition_platformer2d_shared_tangle::{action_results::{self, ActionReply, ActionDeliveryPolicy},
    lifecycle::*};
use ambition_platformer2d_world::{creation::ObjectBehaviorLimits, governing_rules::RulesOf};
use ambition_world_port::behavior::*;
use bevy::{ecs::system::SystemState, prelude::*};

#[derive(Clone, Debug, PartialEq)]
struct CallAuthority { session: u64, room: Option<u32> }
ambition_extension_sdk::port_record! { CallAuthority { session: u64, room: Option<u32> } }
impl CallAuthority {
    fn of(world: &World, entity: Entity) -> Option<Self> {
        match simulation_island(world, entity)? {
            SimulationIsland::Session(session) => Some(Self {session: session.0, room: None}),
            SimulationIsland::Room(session, room) => Some(Self {session: session.0, room: Some(room.ordinal())}),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
struct CallOrigin { recipient: ObjectRef, ticket: ActionTicket, authority: CallAuthority }
ambition_extension_sdk::port_record! { CallOrigin { recipient: ObjectRef, ticket: ActionTicket, authority: CallAuthority } }
#[derive(Clone, Debug, PartialEq)]
struct CallRow {
    call: ObjectCall,
    origin: CallOrigin,
    entry: String,
    written: u64,
    progress: DeliveryProgress,
    terminal: bool,
    remaining: u32,
    max_attempts: u32,
}
ambition_extension_sdk::port_record! {
    CallRow { call: ObjectCall, origin: CallOrigin, entry: String, written: u64,
        progress: DeliveryProgress, terminal: bool, remaining: u32, max_attempts: u32 }
}
#[derive(Component, Clone, Debug, PartialEq)]
pub(crate) struct BehaviorInstance {
    key: String,
    methods: Vec<BehaviorMethod>,
    params: ScriptValue,
    references: Vec<(String, ObjectRef)>,
    next_sequence: u64,
    calls: Vec<CallRow>,
}
ambition_extension_sdk::port_record! {
    BehaviorInstance { key: String, methods: Vec<BehaviorMethod>, params: ScriptValue,
        references: Vec<(String, ObjectRef)> as ambition_extension_sdk::structure::TextMap<ObjectRef>,
        next_sequence: u64, calls: Vec<CallRow> }
}
impl SnapshotState for BehaviorInstance {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut bytes = Vec::new(); ambition_extension_sdk::structure::put(self, &mut bytes); put_bytes(out, &bytes);
    }
    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        ambition_extension_sdk::wire::decode_all(reader.bytes()?, ambition_extension_sdk::structure::read).ok()
    }
}

#[derive(Clone, Debug, PartialEq)]
struct LocalCallWatch { ticket: ActionTicket, receiver: ObjectRef, authority: CallAuthority }
ambition_extension_sdk::port_record! { LocalCallWatch { ticket: ActionTicket, receiver: ObjectRef, authority: CallAuthority } }

/// Caller-owned lifetime metadata. Request payloads stay in the callee journal.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub(crate) struct LocalCallWatches { rows: Vec<LocalCallWatch> }
ambition_extension_sdk::port_record! { LocalCallWatches { rows: Vec<LocalCallWatch> } }
impl SnapshotState for LocalCallWatches {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut bytes = Vec::new(); ambition_extension_sdk::structure::put(self, &mut bytes); put_bytes(out, &bytes);
    }
    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        ambition_extension_sdk::wire::decode_all(reader.bytes()?, ambition_extension_sdk::structure::read).ok()
    }
}

pub(crate) fn maintain_watches(world: &mut World, state: Entity, objects: &ObjectIndex, now: u64) {
    let Some(watches) = world.get::<LocalCallWatches>(state) else { return; };
    if watches.rows.is_empty() { return; }
    let authority = CallAuthority::of(world, state);
    let mailbox = world.get::<action_results::ActionMailbox>(state);
    let mut retained = Vec::new();
    let mut failed = Vec::new();
    for row in watches.rows.clone() {
        if !mailbox.is_some_and(|mailbox| mailbox.is_pending(&row.ticket)) { continue; }
        let receiver = objects.resolve(SessionScopeId(row.receiver.session), row.receiver).ok();
        let local = authority.as_ref() == Some(&row.authority)
            && receiver.is_some_and(|entity| CallAuthority::of(world, entity).as_ref() == Some(&row.authority));
        // A foreign room is hidden during replay. Its presence must not
        // change the caller's result or wait clock.
        if local { retained.push(row); }
        else { failed.push((row.ticket, "local_call_authority_ended")); }
    }
    world.get_mut::<LocalCallWatches>(state).unwrap().rows = retained;
    for (ticket, code) in failed {
        world.get_mut::<action_results::ActionMailbox>(state).unwrap().publish(ActionResult {
            ticket, stage: ActionStage::Failed, terminal: true, tick: now, code: code.into(), message: String::new(), subjects: Vec::new()});
    }
}
fn local_origin(world: &World, state: Entity, origin: &CallOrigin, objects: &ObjectIndex) -> Option<Entity> {
    if CallAuthority::of(world, state).as_ref() != Some(&origin.authority) { return None; }
    let receiver = objects.resolve(SessionScopeId(origin.recipient.session), origin.recipient).ok()?;
    (CallAuthority::of(world, receiver).as_ref() == Some(&origin.authority)).then_some(receiver)
}

#[derive(Component, Clone, Debug, Default, PartialEq)]
pub(crate) struct ObjectCallResults { entries: Vec<ObjectCallResult> }
ambition_extension_sdk::port_record! { ObjectCallResults { entries: Vec<ObjectCallResult> } }
impl SnapshotState for ObjectCallResults {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut bytes = Vec::new(); ambition_extension_sdk::structure::put(self, &mut bytes); put_bytes(out, &bytes);
    }
    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        ambition_extension_sdk::wire::decode_all(reader.bytes()?, ambition_extension_sdk::structure::read).ok()
    }
}
fn call_result(world: &World, context: ObservationContext<'_>) -> Option<Option<ObjectCallResult>> {
    let Some(result) = context.trigger::<ActionResultPort>() else { return Some(None); };
    if !result.terminal { return Some(None); }
    Some(world.get::<ObjectCallResults>(context.state_scope).and_then(|history|
        history.entries.iter().find(|row| row.ticket == result.ticket).cloned()))
}

pub(crate) fn install(app: &mut App) {
    action_results::register_action_domain(app, "objects.resolve_authored", action_results::ActionCancellationPolicy::Unsupported);
    app.install_extension_submitted_request::<ResolveAuthoredObjectsPort, _>(ACTOR_THINK, "ambition_platformer2d_runtime", resolve_authored);
    action_results::register_action_domain(app, "objects.attach_behaviors", action_results::ActionCancellationPolicy::Unsupported);
    action_results::register_action_domain(app, "objects.complete_call", action_results::ActionCancellationPolicy::Unsupported);
    action_results::register_action_domain(app, "objects.call", action_results::ActionCancellationPolicy::Handler(cancel_call));
    app.install_extension_trigger::<ObjectCallPort, _>(ACTOR_THINK, "ambition_platformer2d_runtime",
        crate::behavior_inbox::collect::<BehaviorInstance>(crate::behavior_inbox::Lane::Request));
    app.install_extension_submitted_request::<AttachBehaviorsPort, _>(ACTOR_THINK, "ambition_platformer2d_runtime", attach);
    app.install_extension_submitted_request::<CallObjectPort, _>(ACTOR_THINK, "ambition_platformer2d_runtime", call);
    app.install_extension_submitted_request::<CompleteObjectCallPort, _>(ACTOR_THINK, "ambition_platformer2d_runtime", complete);
    app.install_extension_context_observation::<ObjectCallResultPort>(ACTOR_THINK, "ambition_platformer2d_runtime", call_result);
    app.install_extension_context_observation::<BehaviorContextPort>(ACTOR_THINK, "ambition_platformer2d_runtime", context);
}

fn context(world: &World, context: ObservationContext<'_>) -> Option<Option<BehaviorContext>> {
    Some(world.get::<BehaviorInstance>(context.state_scope).and_then(|instance| Some(BehaviorContext {
        object: world.get::<ObjectScopeOwner>(context.state_scope)?.0,
        binding: instance.key.clone(), params: instance.params.clone(), references: instance.references.clone(),
    })))
}
fn tick(world: &World) -> u64 { world.resource::<ambition_time::SimTick>().0 }
pub(crate) fn limits(world: &mut World, subject: Entity) -> ObjectBehaviorLimits {
    let mut state = SystemState::<RulesOf<ObjectBehaviorLimits>>::new(world);
    state.get(world).ok().and_then(|rules| rules.of(subject).copied()).unwrap_or_default()
}
fn target(world: &World, caller: Entity, reference: ObjectRef) -> Result<Entity, &'static str> {
    let scope = world.get::<SessionScopedEntity>(caller).ok_or("caller_without_session")?.0;
    let target = resolve_object(world, scope, reference).map_err(|_| "object_not_live_in_authority")?;
    if !shares_simulation_island(world, caller, target) { return Err("object_not_live_in_authority"); }
    Ok(target)
}
pub(crate) fn checked_data(data: &ScriptValue, max: u32) -> Option<ScriptValue> {
    use ambition_extension_sdk::structure::Field;
    let data = <ScriptValue as Field<ScriptValue>>::from_script(data).ok()?;
    let mut bytes = Vec::new(); <ScriptValue as Field<ScriptValue>>::put(&data, &mut bytes);
    (bytes.len() <= max as usize).then_some(data)
}
fn method_bound(admitted: &ambition_extension_host::Admitted, path: &str) -> bool {
    admitted.entries.iter().any(|entry| entry.path == path && entry.descriptor.phase == ACTOR_THINK
        && entry.descriptor.trigger.port == ObjectCallPort::KEY)
}

fn behavior_method_bound(world: &World, path: &str) -> bool {
    let admitted = &world.resource::<AdmittedExtensions>().0;
    method_bound(admitted, path) || admitted.entries.iter().find(|entry| entry.path == path)
        .is_some_and(|entry| world.resource::<crate::behavior_events::NativeSources>().for_entry(entry).is_some())
}

#[derive(Clone)]
pub(crate) struct BehaviorInitialization {
    instance: BehaviorInstance,
    records: BodyRecords,
    references: Vec<(String, ambition_world_port::object_target::ObjectTarget)>,
    subscriptions: Vec<crate::behavior_events::PreparedSubscription>,
    event_capacity: u32,
    event_max_bytes: u32,
}
impl ambition_platformer2d_construction::runtime::ObjectInitialization for BehaviorInitialization {
    type Prepared = (BehaviorInstance, BodyRecords, crate::behavior_events::EventMailbox);
    fn prepare(&self, context: &ambition_platformer2d_construction::runtime::ObjectInitializationContext<'_>)
        -> Result<Self::Prepared, String> {
        use ambition_world_port::object_target::ObjectTarget;
        let mut instance = self.instance.clone();
        instance.references = self.references.iter().map(|(key, target)| {
            let reference = match target {
                ObjectTarget::Live(reference) => *reference,
                ObjectTarget::Local(local) => *context.locals.get(local)
                    .ok_or_else(|| format!("Behavior {} reference {key}: no local object {local} exists in this batch.", instance.key))?,
            };
            if reference.session != context.object.session {
                return Err(format!("Behavior {} reference {key} crosses sessions.", instance.key));
            }
            Ok((key.clone(), reference))
        }).collect::<Result<_, String>>()?;
        let events = crate::behavior_events::initialized(&self.subscriptions, &instance.references, self.event_capacity, self.event_max_bytes);
        Ok((instance, self.records.clone(), events))
    }
}
pub(crate) type PreparedBehaviorState = ambition_platformer2d_construction::runtime::OwnedObjectState<BehaviorInitialization>;
fn prepare_bindings(world: &World, bindings: &[BehaviorBinding],
    limits: ObjectBehaviorLimits) -> Result<Vec<PreparedBehaviorState>, String> {
    let admitted = &world.resource::<AdmittedExtensions>().0;
    let mut keys = BTreeSet::new();
    let mut bytes = 0usize;
    bindings.iter().map(|binding| {
        let params = checked_data(&binding.params, limits.max_data_bytes)
            .ok_or("Behavior parameters exceed their bound or contain invalid data.")?;
        if binding.key.is_empty() || binding.key.len() > 64 || !keys.insert(&binding.key)
            || binding.methods.is_empty() || binding.methods.len() > limits.max_methods as usize
            || binding.state.len() > limits.max_state_records as usize {
            return Err("A behavior key, method count, state count or parameter value exceeds its bound.".into());
        }
        if binding.references.len() > limits.max_references as usize {
            return Err("The behavior reference count exceeds its game bound.".into());
        }
        let mut references = binding.references.clone();
        references.sort_by(|a, b| a.0.cmp(&b.0));
        let mut reference_names = BTreeSet::new();
        for (key, target) in &references {
            if key.is_empty() || key.len() > 64 || !reference_names.insert(key) {
                return Err("Behavior reference names must be unique, nonempty and at most 64 bytes.".into());
            }
            match target {
                ambition_world_port::object_target::ObjectTarget::Local(key) if key.is_empty() || key.len() > 128 => {
                    return Err("Local object keys must be nonempty and at most 128 bytes.".into());
                }
                ambition_world_port::object_target::ObjectTarget::Live(reference) if !reference.is_bound() => {
                    return Err(format!("Behavior reference {key} is unbound."));
                }
                _ => {}
            }
        }
        let mut names = BTreeSet::new();
        let mut writes = BTreeSet::new();
        for method in &binding.methods {
            if method.name.is_empty() || method.name.len() > 64 || !names.insert(&method.name)
                || !behavior_method_bound(world, &method.entry) {
                return Err(format!("Behavior {} has an invalid or unavailable method {} at {}.", binding.key, method.name, method.entry));
            }
            let entry = admitted.entries.iter().find(|entry| entry.path == method.entry).unwrap();
            writes.extend(entry.descriptor.writes.iter().cloned());
        }
        let subscriptions = crate::behavior_events::prepare(world, &binding.methods, &references,
            &binding.subscriptions, limits.max_subscriptions)?;
        let mut records = BodyRecords::default();
        for (key, shape, record) in crate::record_seeds::prepare_record_seeds(admitted, &writes, &binding.state,
            &mut bytes, limits.max_state_bytes, &format!("Behavior {}", binding.key))? {
            records.put(key, shape, record);
        }
        Ok(PreparedBehaviorState {key: binding.key.clone(), bundle: BehaviorInitialization {
            instance: BehaviorInstance {key: binding.key.clone(), methods: binding.methods.clone(), params,
                references: Vec::new(), next_sequence: 0, calls: Vec::new()}, records, references, subscriptions,
                event_capacity: limits.max_event_history, event_max_bytes: limits.max_data_bytes}})
    }).collect()
}
/// Validate the complete creation batch before the native owner mints any root.
pub(crate) fn prepare_creation(world: &mut World, caller: Entity, groups: &[&[BehaviorBinding]])
    -> Result<Vec<Vec<PreparedBehaviorState>>, String> {
    let session = world.get::<SessionScopedEntity>(caller).ok_or("The creator has no session.")?.0;
    let limits = limits(world, caller);
    let count = groups.iter().try_fold(0usize, |count, group| count.checked_add(group.len()))
        .ok_or("Too many requested behavior instances.")?;
    if count == 0 { return Ok(groups.iter().map(|_| Vec::new()).collect()); }
    let population = world.query::<(&SessionScopedEntity, &BehaviorInstance)>().iter(world)
        .filter(|(owner, _)| owner.0 == session).count();
    if population.saturating_add(count) > limits.max_bindings_per_session as usize
        || groups.iter().any(|group| group.len() > limits.max_bindings_per_object as usize) {
        return Err("The object or session behavior bound cannot admit this creation batch.".into());
    }
    groups.iter().map(|group| prepare_bindings(world, group, limits)).collect()
}

fn attach(world: &mut World) {
    let requests = world.resource_mut::<ExtensionOutbox>().drain::<AttachBehaviorsPort>(&ACTOR_THINK);
    for submitted in requests {
        let request = submitted.value;
        let dispatch = action_results::begin_action_request::<AttachBehaviorsPort>(world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute { continue; }
        let result = (|| -> Result<(), String> {
            let entity = target(world, submitted.scope, request.target)?;
            let session = world.get::<SessionScopedEntity>(entity).unwrap().0;
            let limits = limits(world, entity);
            let mut instances = world.query::<(&SessionScopedEntity, &ObjectScopeOwner, &BehaviorInstance)>();
            let mut population = 0usize;
            let mut existing = BTreeSet::new();
            for (_, owner, instance) in instances.iter(world).filter(|(session_owner, _, _)| session_owner.0 == session) {
                population += 1;
                if owner.0 == request.target { existing.insert(instance.key.clone()); }
            }
            if request.bindings.is_empty() || existing.len().saturating_add(request.bindings.len()) > limits.max_bindings_per_object as usize
                || population.saturating_add(request.bindings.len()) > limits.max_bindings_per_session as usize
                || request.bindings.iter().any(|binding| existing.contains(&binding.key)) {
                return Err("Behavior keys must be new and the object and session bounds must permit the batch.".into());
            }
            let prepared = prepare_bindings(world, &request.bindings, limits)?;
            use ambition_platformer2d_construction::runtime::{ObjectInitialization, ObjectInitializationContext};
            let locals = BTreeMap::new();
            let context = ObjectInitializationContext {object: request.target, locals: &locals};
            let prepared = prepared.into_iter().map(|binding| Ok((binding.key, binding.bundle.prepare(&context)?)))
                .collect::<Result<Vec<_>, String>>()?;
            let room = object_scope_room(world, entity);
            let allocation = match room {
                Some(room) => PreparedObjectOccurrences::prepare(world, session, room, prepared.len())?,
                None => PreparedObjectOccurrences::prepare_session(world, session, prepared.len())?,
            };
            let mut entities = Vec::new();
            for (key, bundle) in prepared {
                let id = owned_object_state_id(request.target, &key);
                entities.push(world.spawn((SessionScopedEntity(session), id,
                    ObjectScopeOwner(request.target), bundle)).id());
            }
            if let Err(problem) = allocation.assign(world, &entities) {
                for entity in entities { world.despawn(entity); }
                return Err(problem.into());
            }
            Ok(())
        })();
        let (stage, code, message) = match result {
            Ok(()) => (ActionStage::Applied, "behaviors_attached", String::new()),
            Err(error) => (ActionStage::Rejected, "behavior_attachment_refused", error),
        };
        action_results::publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, message, Vec::new());
    }
}

fn call(world: &mut World) {
    let requests = world.resource_mut::<ExtensionOutbox>().drain::<CallObjectPort>(&ACTOR_THINK);
    let mut query = world.query::<(Entity, &ObjectScopeOwner, &BehaviorInstance)>();
    let mut bindings = BTreeMap::new();
    for (entity, owner, instance) in query.iter(world) {
        bindings.entry((owner.0, instance.key.clone())).and_modify(|found| *found = None).or_insert(Some(entity));
    }
    for submitted in requests {
        let request = submitted.value;
        let dispatch = action_results::begin_action_request::<CallObjectPort>(world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute { continue; }
        let result = (|| -> Result<(), &'static str> {
            let entity = target(world, submitted.scope, request.target)?;
            ensure_object_occurrence(world, submitted.scope).ok_or("caller_without_identity")?;
            let caller = object_reference(world, submitted.scope).ok_or("caller_without_identity")?;
            let limits = limits(world, entity);
            let data = checked_data(&request.data, limits.max_data_bytes).ok_or("call_data_bound")?;
            let state = bindings.get(&(request.target, request.binding.clone())).copied().flatten()
                .ok_or("behavior_not_bound")?;
            let instance = world.get::<BehaviorInstance>(state).unwrap();
            let method = instance.methods.iter().find(|method| method.name == request.method).ok_or("method_not_bound")?;
            if !method_bound(&world.resource::<AdmittedExtensions>().0, &method.entry) { return Err("method_unavailable"); }
            if instance.calls.iter().filter(|row| !row.terminal).count() >= limits.max_calls_per_binding as usize {
                return Err("too_many_object_calls");
            }
            let next = instance.next_sequence.checked_add(1).ok_or("object_call_sequence_exhausted")?;
            let entry = method.entry.clone();
            let written = tick(world);
            let origin = dispatch.reply.as_ref().unwrap();
            let authority = CallAuthority::of(world, submitted.state_scope).ok_or("caller_without_authority")?;
            let watch = LocalCallWatch {ticket: origin.ticket.clone(), receiver: request.target,
                authority: authority.clone()};
            let mut instance = world.get_mut::<BehaviorInstance>(state).unwrap();
            if instance.calls.len() >= limits.max_calls_per_binding as usize {
                instance.calls.retain(|row| !row.terminal);
            }
            let sequence = instance.next_sequence;
            instance.next_sequence = next;
            instance.calls.push(CallRow {call: ObjectCall {sequence, caller, method: request.method, data},
                origin: CallOrigin {recipient: origin.recipient.reference(), ticket: origin.ticket.clone(),
                    authority}, entry,
                written, progress: DeliveryProgress::default(), terminal: false, remaining: limits.max_active_wait_ticks, max_attempts: ActionDeliveryPolicy::default().max_attempts.get()});
            let mut caller = world.entity_mut(submitted.state_scope);
            caller.entry::<LocalCallWatches>().or_default().into_mut().rows.push(watch);
            Ok(())
        })();
        let (stage, terminal, code) = match result {
            Ok(()) => (ActionStage::Admitted, false, "object_call_admitted"),
            Err(code) => (ActionStage::Rejected, true, code),
        };
        action_results::publish_action_result(world, dispatch.reply.as_ref(), stage, terminal, code, "", Vec::new());
    }
}
fn publish(world: &mut World, producer: Entity, origin: &CallOrigin, stage: ActionStage, code: &str, data: Option<ScriptValue>) {
    let now = tick(world);
    let Some(entity) = local_origin(world, producer, origin, &ObjectIndex::capture(world)) else { return; };
    let published = world.get_mut::<action_results::ActionMailbox>(entity).is_some_and(|mut mailbox|
        mailbox.publish(ActionResult {ticket: origin.ticket.clone(), stage, terminal: true, tick: now,
            code: code.into(), message: String::new(), subjects: Vec::new()}));
    if let Some(data) = data.filter(|_| published) {
        let tickets: BTreeSet<_> = world.get::<action_results::ActionMailbox>(entity).unwrap()
            .payload_tickets().cloned().collect();
        let mut entity = world.entity_mut(entity);
        let mut history = entity.entry::<ObjectCallResults>().or_default().into_mut();
        history.entries.retain(|row| tickets.contains(&row.ticket));
        history.entries.push(ObjectCallResult {ticket: origin.ticket.clone(), data});
    }
}
fn complete(world: &mut World) {
    let requests = world.resource_mut::<ExtensionOutbox>().drain::<CompleteObjectCallPort>(&ACTOR_THINK);
    for submitted in requests {
        let request = submitted.value;
        let dispatch = action_results::begin_action_request::<CompleteObjectCallPort>(world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute { continue; }
        let result = (|| -> Result<(CallOrigin, ScriptValue), &'static str> {
            if request.code.is_empty() || request.code.len() > 128 { return Err("invalid_completion_code"); }
            let cap = limits(world, submitted.scope).max_data_bytes;
            let data = checked_data(&request.data, cap).ok_or("completion_data_bound")?;
            let instance = world.get::<BehaviorInstance>(submitted.state_scope).ok_or("not_a_behavior_instance")?;
            let row = instance.calls.iter().find(|row| row.call.sequence == request.sequence
                && !row.terminal && row.progress.status == ActionDelivery::Acknowledged).ok_or("call_not_pending_completion")?;
            if row.entry.rsplit_once('/').map(|(module, _)| module) != submitted.entry.rsplit_once('/').map(|(module, _)| module) {
                return Err("completion_from_foreign_module");
            }
            let origin = row.origin.clone();
            let caller = local_origin(world, submitted.state_scope, &origin, &ObjectIndex::capture(world))
                .ok_or("cross_island_behavior_unsupported")?;
            if !world.get::<action_results::ActionMailbox>(caller).is_some_and(|mailbox| mailbox.is_pending(&origin.ticket)) {
                return Err("call_source_not_pending");
            }
            world.get_mut::<BehaviorInstance>(submitted.state_scope).unwrap().calls.iter_mut()
                .find(|row| row.call.sequence == request.sequence).unwrap().terminal = true;
            Ok((origin, data))
        })();
        let (stage, code) = match result {
            Ok((origin, data)) => {
                publish(world, submitted.state_scope, &origin, if request.accepted { ActionStage::Completed } else { ActionStage::Rejected }, &request.code, Some(data));
                (ActionStage::Applied, "object_call_completed")
            }
            Err(code) => (ActionStage::Rejected, code),
        };
        action_results::publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, "", Vec::new());
    }
}
fn cancel_call(world: &mut World, reply: &ActionReply) -> bool {
    let objects = ObjectIndex::capture(world);
    let candidates: Vec<_> = world.query::<(Entity, &BehaviorInstance)>().iter(world)
        .filter_map(|(entity, instance)| instance.calls.iter().find(|row|
            row.origin.recipient == reply.recipient.reference() && row.origin.ticket == reply.ticket && !row.terminal
            && local_origin(world, entity, &row.origin, &objects).is_some()).map(|_| entity)).collect();
    for entity in candidates {
        let mut instance = world.get_mut::<BehaviorInstance>(entity).unwrap();
        let row = instance.calls.iter_mut().find(|row| row.origin.recipient == reply.recipient.reference()
            && row.origin.ticket == reply.ticket && !row.terminal).unwrap();
        if matches!(row.progress.status, ActionDelivery::Invoked | ActionDelivery::Acknowledged) { return false; }
        row.terminal = true;
        row.progress.retire("object_call_canceled");
        return true;
    }
    false
}
pub(crate) fn settle(world: &mut World, state: Entity, sequence: u64, event: InvocationEvent) {
    let now = tick(world);
    let Some(mut instance) = world.get_mut::<BehaviorInstance>(state) else { return; };
    let Some(row) = instance.calls.iter_mut().find(|row| row.call.sequence == sequence && !row.terminal) else { return; };
    row.progress.settle(now, event, row.max_attempts);
    if row.progress.status == ActionDelivery::Quarantined {
        row.terminal = true;
        let origin = row.origin.clone();
        publish(world, state, &origin, ActionStage::Failed, "object_handler_fault", None);
    }
}

pub(crate) fn retire(world: &mut World, state: Entity) {
    let origins = world.get::<BehaviorInstance>(state).into_iter().flat_map(|instance|
        instance.calls.iter().filter(|row| !row.terminal).map(|row| row.origin.clone())).collect::<Vec<_>>();
    for origin in origins { publish(world, state, &origin, ActionStage::Failed, "object_retired", None); }
}

/// Call completion and caller lifetime remain with the request owner.
pub(crate) fn maintain(world: &mut World, state: Entity, objects: &ObjectIndex, now: u64) {
    let Some(instance) = world.get::<BehaviorInstance>(state) else { return; };
    let live_origins: BTreeSet<_> = instance.calls.iter().filter(|row| !row.terminal).filter(|row| {
        local_origin(world, state, &row.origin, objects)
            .and_then(|entity| world.get::<action_results::ActionMailbox>(entity))
            .is_some_and(|mailbox| mailbox.is_pending(&row.origin.ticket))
    }).map(|row| row.call.sequence).collect();
    let mut expired = Vec::new();
    let mut instance = world.get_mut::<BehaviorInstance>(state).unwrap();
    for row in instance.calls.iter_mut().filter(|row| !row.terminal) {
        if !live_origins.contains(&row.call.sequence) {
            row.terminal = true;
            row.progress.retire("object_caller_retired");
        } else if row.written < now {
            if row.remaining == 0 {
                row.terminal = true;
                row.progress.retire("object_call_expired");
                expired.push(row.origin.clone());
            } else { row.remaining -= 1; }
        }
    }
    for origin in expired { publish(world, state, &origin, ActionStage::Failed, "object_call_expired", None); }
}
impl BehaviorInstance {
    pub(crate) fn method_entry(&self, name: &str) -> Option<&str> {
        self.methods.iter().find(|method| method.name == name).map(|method| method.entry.as_str())
    }
    pub(crate) fn deliveries<'a>(&'a self, world: &'a World, state: Entity, objects: &'a ObjectIndex)
        -> impl Iterator<Item = (u64, u64, &'a str)> + 'a {
        self.calls.iter().filter(move |row| !row.terminal && row.progress.pending()
            && local_origin(world, state, &row.origin, objects).is_some())
            .map(|row| (row.call.sequence, row.written, row.entry.as_str()))
    }
    pub(crate) fn queue_delivery(&mut self, sequence: u64, now: u64, bound: bool, attempts: u32) -> Option<ObjectCall> {
        let row = self.calls.iter_mut().find(|row| row.call.sequence == sequence && !row.terminal)?;
        if !row.progress.queue(now, row.written, bound, "object_method_unbound") { return None; }
        row.max_attempts = attempts;
        Some(row.call.clone())
    }
}

#[cfg(test)]
mod local_authority_tests {
    use super::*;
    use bevy::ecs::entity_disabling::Disabled;
    use ambition_platformer2d_core::snapshot::{encode_state, decode_state};
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    #[test]
    fn calls_refuse_foreign_authority_and_scope_exit_fails_only_the_local_wait() {
        let mut world = World::new();
        world.insert_resource(ambition_time::SimTick(10));
        world.insert_resource(SimulationIslands(|_| true));
        let session = SessionScopeId(1);
        let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        for room in rooms { world.spawn((RoomInstanceRoot, room, SessionScopedEntity(session))); }
        let caller = world.spawn((SessionScopedEntity(session), InRoomInstance(rooms[0]), SimId::placement("caller"))).id();
        let receiver = world.spawn((SessionScopedEntity(session), InRoomInstance(rooms[1]), SimId::placement("receiver"))).id();
        for entity in [caller, receiver] { ensure_object_occurrence(&mut world, entity).unwrap(); }
        let receiver_ref = object_reference(&world, receiver).unwrap();
        assert_eq!(target(&world, caller, receiver_ref), Err("object_not_live_in_authority"));
        world.entity_mut(receiver).insert(Disabled);
        assert_eq!(target(&world, caller, receiver_ref), Err("object_not_live_in_authority"));
        world.entity_mut(receiver).remove::<Disabled>();
        world.remove_resource::<SimulationIslands>();
        assert_eq!(target(&world, caller, receiver_ref), Ok(receiver), "whole-app authority can span rooms");
        world.insert_resource(SimulationIslands(|_| true));
        world.entity_mut(receiver).insert(InRoomInstance(rooms[0]));
        let dispatch = action_results::begin_action(&mut world, caller, "probe::local/start", "objects.call",
            Some(&ReplyTo {request: 1, selector: "probe::local/done".into()}), 0);
        let reply = dispatch.reply.unwrap();
        let authority = CallAuthority::of(&world, caller).unwrap();
        let origin = CallOrigin {recipient: reply.recipient.reference(), ticket: reply.ticket.clone(), authority: authority.clone()};
        let state = world.spawn((SessionScopedEntity(session), ObjectScopeOwner(receiver_ref), BehaviorInstance {
            key: "logic".into(), methods: Vec::new(), params: ScriptValue::Unit, references: Vec::new(), next_sequence: 1,
            calls: vec![CallRow {call: ObjectCall {sequence: 0, caller: object_reference(&world, caller).unwrap(), method: "use".into(), data: ScriptValue::Unit},
                origin: origin.clone(), entry: "probe::local/use".into(), written: 10, progress: DeliveryProgress::default(), terminal: false,
                remaining: 10, max_attempts: 3}],
        })).id();
        world.entity_mut(caller).insert(LocalCallWatches {rows: vec![LocalCallWatch {
            ticket: reply.ticket.clone(), receiver: receiver_ref, authority}]});
        let caller_before = world.get::<action_results::ActionMailbox>(caller).unwrap().clone();
        let watches_before = world.get::<LocalCallWatches>(caller).unwrap().clone();
        let callee_before = encode_state(world.get::<BehaviorInstance>(state).unwrap());
        world.entity_mut(caller).insert(InRoomInstance(rooms[1]));
        let objects = ObjectIndex::capture(&world);
        maintain_watches(&mut world, caller, &objects, 11);
        let history = world.get::<action_results::ActionMailbox>(caller).unwrap().history();
        let terminal = history.entries.last().unwrap();
        assert_eq!(terminal.result.stage, ActionStage::Failed);
        assert_eq!(terminal.result.code, "local_call_authority_ended");
        assert_eq!(encode_state(world.get::<BehaviorInstance>(state).unwrap()), callee_before);
        let failed = encode_state(world.get::<action_results::ActionMailbox>(caller).unwrap());
        publish(&mut world, state, &origin, ActionStage::Completed, "late", Some(ScriptValue::Unit));
        assert_eq!(encode_state(world.get::<action_results::ActionMailbox>(caller).unwrap()), failed,
            "a foreign callee cannot overwrite the caller's outcome");
        assert!(!cancel_call(&mut world, &reply));
        assert_eq!(encode_state(world.get::<BehaviorInstance>(state).unwrap()), callee_before);
        world.entity_mut(caller).insert((InRoomInstance(rooms[0]), caller_before.clone(), watches_before.clone()));
        world.entity_mut(receiver).insert(InRoomInstance(rooms[1]));
        let objects = ObjectIndex::capture(&world);
        maintain_watches(&mut world, caller, &objects, 12);
        assert_eq!(world.get::<action_results::ActionMailbox>(caller).unwrap().history().entries.last().unwrap().result.code,
            "local_call_authority_ended", "receiver departure ends the local wait too");
        assert_eq!(encode_state(world.get::<BehaviorInstance>(state).unwrap()), callee_before);
        world.entity_mut(caller).insert((caller_before.clone(), watches_before.clone()));
        world.entity_mut(receiver).insert(Disabled);
        let objects = ObjectIndex::capture(&world);
        maintain_watches(&mut world, caller, &objects, 12);
        assert_eq!(world.get::<action_results::ActionMailbox>(caller).unwrap().history().entries.last().unwrap().result.code,
            "local_call_authority_ended", "hidden and visible foreign endpoints have the same local outcome");
        world.entity_mut(receiver).remove::<Disabled>();
        world.entity_mut(receiver).insert(InRoomInstance(rooms[0]));
        world.entity_mut(caller).insert((caller_before, watches_before));
        world.entity_mut(state).insert(decode_state::<BehaviorInstance>(&callee_before).unwrap());
        publish(&mut world, state, &origin, ActionStage::Completed, "local", Some(ScriptValue::Unit));
        assert_eq!(world.get::<action_results::ActionMailbox>(caller).unwrap().history().entries.last().unwrap().result.code, "local");
        assert_eq!(target(&world, caller, receiver_ref), Ok(receiver));
    }
}

fn resolve_authored(world: &mut World) {
    let requests = world.resource_mut::<ExtensionOutbox>().drain::<ResolveAuthoredObjectsPort>(&ACTOR_THINK);
    if requests.is_empty() { return; }
    let objects = AuthoredObjectIndex::capture(world);
    for submitted in requests {
        let request = submitted.value;
        let dispatch = action_results::begin_action_request::<ResolveAuthoredObjectsPort>(world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute { continue; }
        let cap = limits(world, submitted.scope).max_authored_lookups;
        let result = (|| -> Result<Vec<ActionSubject>, (&str, String)> {
            if request.placements.is_empty() || request.placements.len() > cap as usize
                || request.placements.iter().any(|key| key.is_empty() || key.len() > 256) {
                return Err(("authored_lookup_bound", "Use nonempty placement keys within the room lookup bound.".into()));
            }
            let session = world.get::<SessionScopedEntity>(submitted.scope)
                .ok_or(("caller_without_session", String::new()))?.0;
            let room = object_scope_room(world, submitted.scope).ok_or(("caller_without_room", String::new()))?;
            let root = live_room_root_for(world, session, room).ok_or(("room_not_live", String::new()))?;
            let definition = world.get::<ambition_platformer2d_world::rooms::LiveRoomDefinition>(root)
                .ok_or(("room_without_definition", String::new()))?;
            let room_set = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<ambition_platformer2d_world::rooms::RoomSet>(world)
                .ok_or(("room_without_definition", String::new()))?;
            let source = &room_set.spec(*definition).id;
            request.placements.iter().map(|key| {
                objects.resolve(session, room, source, key).map(|object| ActionSubject {key: key.clone(), object: Some(object)})
                    .map_err(|problem| (match problem {
                        AuthoredLookupError::Absent => "authored_object_absent",
                        AuthoredLookupError::Ambiguous => "authored_object_ambiguous",
                        AuthoredLookupError::Unpublished => "authored_object_unpublished",
                    }, format!("Placement {key} in live room {} ({source}): {problem:?}.", room.ordinal())))
            }).collect()
        })();
        let (stage, code, message, subjects) = match result {
            Ok(subjects) => (ActionStage::Applied, "authored_objects_resolved", String::new(), subjects),
            Err((code, message)) => (ActionStage::Rejected, code, message, Vec::new()),
        };
        action_results::publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, message, subjects);
    }
}
