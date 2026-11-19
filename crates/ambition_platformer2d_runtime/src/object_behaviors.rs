//! Private reusable behavior instances and bounded directed object calls.

use std::collections::{BTreeMap, BTreeSet};
use ambition_extension_host::{AdmittedExtensions, BodyRecords, ExtensionAppExt, ExtensionInvocations,
    ExtensionOutbox, InvocationEvent, InvocationReceipt, InvocationScope, ObservationContext};
use ambition_extension_sdk::{Attachment, ObjectRef, Port, action::*, phases::ACTOR_THINK, script::ScriptValue};
use ambition_platformer2d_core::snapshot::{SnapshotState, Reader, put_bytes};
use ambition_platformer2d_shared_tangle::{action_results::{self, ActionReply, ActionDeliveryPolicy, DeliveryProgress},
    lifecycle::*, sim_id::SimId};
use ambition_platformer2d_world::{creation::ObjectBehaviorLimits, governing_rules::RulesOf};
use ambition_world_port::behavior::*;
use bevy::{ecs::system::SystemState, prelude::*};

#[derive(Clone, Debug, PartialEq)]
struct CallOrigin { recipient: ObjectRef, ticket: ActionTicket }
ambition_extension_sdk::port_record! { CallOrigin { recipient: ObjectRef, ticket: ActionTicket } }
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
    next_sequence: u64,
    calls: Vec<CallRow>,
}
ambition_extension_sdk::port_record! {
    BehaviorInstance { key: String, methods: Vec<BehaviorMethod>, params: ScriptValue,
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
    action_results::register_action_domain(app, "objects.attach_behaviors", action_results::ActionCancellationPolicy::Unsupported);
    action_results::register_action_domain(app, "objects.complete_call", action_results::ActionCancellationPolicy::Unsupported);
    action_results::register_action_domain(app, "objects.call", action_results::ActionCancellationPolicy::Handler(cancel_call));
    app.install_extension_trigger::<ObjectCallPort, _>(ACTOR_THINK, "ambition_platformer2d_runtime", queue_calls);
    app.install_extension_submitted_request::<AttachBehaviorsPort, _>(ACTOR_THINK, "ambition_platformer2d_runtime", attach);
    app.install_extension_submitted_request::<CallObjectPort, _>(ACTOR_THINK, "ambition_platformer2d_runtime", call);
    app.install_extension_submitted_request::<CompleteObjectCallPort, _>(ACTOR_THINK, "ambition_platformer2d_runtime", complete);
    app.install_extension_context_observation::<ObjectCallResultPort>(ACTOR_THINK, "ambition_platformer2d_runtime", call_result);
    app.install_extension_context_observation::<BehaviorContextPort>(ACTOR_THINK, "ambition_platformer2d_runtime", context);
}

fn context(world: &World, context: ObservationContext<'_>) -> Option<Option<BehaviorContext>> {
    Some(world.get::<BehaviorInstance>(context.state_scope).and_then(|instance| Some(BehaviorContext {
        object: world.get::<ObjectScopeOwner>(context.state_scope)?.0,
        binding: instance.key.clone(), params: instance.params.clone(),
    })))
}
fn tick(world: &World) -> u64 { world.resource::<ambition_time::SimTick>().0 }
fn limits(world: &mut World, subject: Entity) -> ObjectBehaviorLimits {
    let mut state = SystemState::<RulesOf<ObjectBehaviorLimits>>::new(world);
    state.get(world).ok().and_then(|rules| rules.of(subject).copied()).unwrap_or_default()
}
fn target(world: &World, caller: Entity, reference: ObjectRef) -> Result<Entity, &'static str> {
    let scope = world.get::<SessionScopedEntity>(caller).ok_or("caller_without_session")?.0;
    resolve_object(world, scope, reference).map_err(|_| "object_not_live")
}
fn checked_data(data: &ScriptValue, max: u32) -> Option<ScriptValue> {
    use ambition_extension_sdk::structure::Field;
    let data = <ScriptValue as Field<ScriptValue>>::from_script(data).ok()?;
    let mut bytes = Vec::new(); <ScriptValue as Field<ScriptValue>>::put(&data, &mut bytes);
    (bytes.len() <= max as usize).then_some(data)
}
fn method_bound(admitted: &ambition_extension_host::Admitted, path: &str) -> bool {
    admitted.entries.iter().any(|entry| entry.path == path && entry.descriptor.phase == ACTOR_THINK
        && entry.descriptor.trigger.port == ObjectCallPort::KEY)
}

struct PreparedBinding { instance: BehaviorInstance, records: BodyRecords }
fn prepare_bindings(world: &World, request: &AttachBehaviors,
    limits: ObjectBehaviorLimits) -> Result<Vec<PreparedBinding>, String> {
    let admitted = &world.resource::<AdmittedExtensions>().0;
    let mut keys = BTreeSet::new();
    let mut bytes = 0usize;
    request.bindings.iter().map(|binding| {
        let params = checked_data(&binding.params, limits.max_data_bytes)
            .ok_or("Behavior parameters exceed their bound or contain invalid data.")?;
        if binding.key.is_empty() || binding.key.len() > 64 || !keys.insert(&binding.key)
            || binding.methods.is_empty() || binding.methods.len() > limits.max_methods as usize
            || binding.state.len() > limits.max_state_records as usize {
            return Err("A behavior key, method count, state count or parameter value exceeds its bound.".into());
        }
        let mut names = BTreeSet::new();
        let mut writes = BTreeSet::new();
        for method in &binding.methods {
            if method.name.is_empty() || method.name.len() > 64 || !names.insert(&method.name)
                || !method_bound(admitted, &method.entry) {
                return Err(format!("Behavior {} has an invalid or unavailable method {} at {}.", binding.key, method.name, method.entry));
            }
            let entry = admitted.entries.iter().find(|entry| entry.path == method.entry).unwrap();
            writes.extend(entry.descriptor.writes.iter().cloned());
        }
        let mut records = BodyRecords::default();
        let mut seeded = BTreeSet::new();
        for seed in &binding.state {
            let mut matches = writes.iter().filter(|key| key.key == seed.schema.as_str() || key.to_string() == seed.schema);
            let key = matches.next().ok_or_else(|| format!("Behavior {} cannot initialize schema {}.", binding.key, seed.schema))?;
            if matches.next().is_some() || !seeded.insert(key.clone()) {
                return Err("A behavior state seed is ambiguous or repeated.".into());
            }
            let declared = &admitted.schemas[key];
            if declared.schema.attachment != Attachment::Body {
                return Err("Behavior initialization requires private body-attached records.".into());
            }
            let record = ambition_extension_sdk::script::record_from_script(&declared.schema, &seed.value)
                .map_err(|error| format!("Behavior {} state {}: {error}", binding.key, seed.schema))?;
            declared.schema.check(&record).map_err(|error| format!("Behavior {} state {}: {error}", binding.key, seed.schema))?;
            let mut encoded = Vec::new(); record.put_wire(&mut encoded);
            bytes = bytes.saturating_add(encoded.len());
            if bytes > limits.max_state_bytes as usize { return Err("Behavior state exceeds the byte bound.".into()); }
            records.put(key.clone(), declared.shape, record);
        }
        Ok(PreparedBinding {instance: BehaviorInstance {key: binding.key.clone(),
            methods: binding.methods.clone(), params, next_sequence: 0, calls: Vec::new()}, records})
    }).collect()
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
            let prepared = prepare_bindings(world, &request, limits)?;
            let room = object_scope_room(world, entity);
            let allocation = match room {
                Some(room) => PreparedObjectOccurrences::prepare(world, session, room, prepared.len())?,
                None => PreparedObjectOccurrences::prepare_session(world, session, prepared.len())?,
            };
            let mut entities = Vec::new();
            for binding in prepared {
                let id = SimId::singleton("behavior", &format!("{}:{:?}:{}:{}", request.target.session,
                    request.target.allocation_room, request.target.sequence, binding.instance.key));
                let mut state = world.spawn((SessionScopedEntity(session), id,
                    ObjectScopeOwner(request.target), binding.instance, binding.records));
                if let Some(room) = room { state.insert(InRoomInstance(room)); }
                entities.push(state.id());
            }
            if let Err(problem) = allocation.assign(world, &entities) {
                for entity in entities { world.despawn(entity); }
                return Err(problem.into());
            }
            for entity in entities { world.entity_mut(entity).remove::<InRoomInstance>(); }
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
            let mut instance = world.get_mut::<BehaviorInstance>(state).unwrap();
            if instance.calls.len() >= limits.max_calls_per_binding as usize {
                instance.calls.retain(|row| !row.terminal);
            }
            let sequence = instance.next_sequence;
            instance.next_sequence = next;
            instance.calls.push(CallRow {call: ObjectCall {sequence, caller, method: request.method, data},
                origin: CallOrigin {recipient: origin.recipient.reference(), ticket: origin.ticket.clone()}, entry,
                written, progress: DeliveryProgress::default(), terminal: false, remaining: limits.max_active_wait_ticks, max_attempts: ActionDeliveryPolicy::default().max_attempts.get()});
            Ok(())
        })();
        let (stage, terminal, code) = match result {
            Ok(()) => (ActionStage::Admitted, false, "object_call_admitted"),
            Err(code) => (ActionStage::Rejected, true, code),
        };
        action_results::publish_action_result(world, dispatch.reply.as_ref(), stage, terminal, code, "", Vec::new());
    }
}
fn publish(world: &mut World, origin: &CallOrigin, stage: ActionStage, code: &str, data: Option<ScriptValue>) {
    let now = tick(world);
    let Ok(entity) = resolve_object(world, SessionScopeId(origin.recipient.session), origin.recipient) else { return; };
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
            let mut instance = world.get_mut::<BehaviorInstance>(submitted.state_scope).ok_or("not_a_behavior_instance")?;
            let row = instance.calls.iter_mut().find(|row| row.call.sequence == request.sequence
                && !row.terminal && row.progress.status == ActionDelivery::Acknowledged).ok_or("call_not_pending_completion")?;
            if row.entry.rsplit_once('/').map(|(module, _)| module) != submitted.entry.rsplit_once('/').map(|(module, _)| module) {
                return Err("completion_from_foreign_module");
            }
            row.terminal = true;
            Ok((row.origin.clone(), data))
        })();
        let (stage, code) = match result {
            Ok((origin, data)) => {
                publish(world, &origin, if request.accepted { ActionStage::Completed } else { ActionStage::Rejected }, &request.code, Some(data));
                (ActionStage::Applied, "object_call_completed")
            }
            Err(code) => (ActionStage::Rejected, code),
        };
        action_results::publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, "", Vec::new());
    }
}
fn cancel_call(world: &mut World, reply: &ActionReply) -> bool {
    let mut instances = world.query::<&mut BehaviorInstance>();
    for mut instance in instances.iter_mut(world) {
        if let Some(row) = instance.calls.iter_mut().find(|row| row.origin.recipient == reply.recipient.reference()
            && row.origin.ticket == reply.ticket && !row.terminal) {
            if matches!(row.progress.status, ActionDelivery::Invoked | ActionDelivery::Acknowledged) { return false; }
            row.terminal = true;
            row.progress.status = ActionDelivery::Retired;
            return true;
        }
    }
    false
}
fn settle(world: &mut World, state: Entity, sequence: u64, event: InvocationEvent) {
    let now = tick(world);
    let Some(mut instance) = world.get_mut::<BehaviorInstance>(state) else { return; };
    let Some(row) = instance.calls.iter_mut().find(|row| row.call.sequence == sequence && !row.terminal) else { return; };
    match event {
        InvocationEvent::Started => row.progress.started(now),
        InvocationEvent::Committed => row.progress.committed(),
        InvocationEvent::Faulted => row.progress.faulted(row.max_attempts),
        InvocationEvent::Unavailable | InvocationEvent::Retired | InvocationEvent::IdleReset => row.progress.unavailable(),
    }
    if row.progress.status == ActionDelivery::Quarantined {
        row.terminal = true;
        let origin = row.origin.clone();
        publish(world, &origin, ActionStage::Failed, "object_handler_fault", None);
    }
}
type QueueReads = SystemState<(RoomFreeze<'static, 'static>, RulesOf<'static, 'static, ActionDeliveryPolicy>)>;
fn queue_calls(world: &mut World, mut reads: Local<Option<QueueReads>>,
    mut instances: Local<Option<QueryState<(Entity, &'static ObjectScopeOwner, &'static BehaviorInstance)>>>) {
    let query = instances.get_or_insert_with(|| world.query());
    let mut order: Vec<_> = query.iter(world).map(|(entity, owner, instance)| (owner.0, instance.key.clone(), entity)).collect();
    if order.is_empty() { return; }
    let reads = reads.get_or_insert_with(|| SystemState::new(world));
    let objects = ObjectIndex::capture(world);
    let now = tick(world);
    let admitted = world.resource::<AdmittedExtensions>().0.clone();
    order.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    for (reference, _, state) in order {
        let subject = objects.resolve(SessionScopeId(reference.session), reference).ok()
            .filter(|entity| world.get_entity(*entity).is_ok());
        let Some(subject) = subject else {
            let origins = world.get::<BehaviorInstance>(state).unwrap().calls.iter().filter(|row| !row.terminal)
                .map(|row| row.origin.clone()).collect::<Vec<_>>();
            for origin in origins { publish(world, &origin, ActionStage::Failed, "object_retired", None); }
            world.despawn(state);
            continue;
        };
        let (pace, policies) = reads.get(world).expect("object delivery readers are installed");
        if world.resource::<ambition_time::WorldTime>().sim_dt() <= 0.0 || pace.scale_of(subject) <= 0.0 { continue; }
        let policy = policies.of(subject).copied().unwrap_or_default();
        let live_origins: BTreeSet<_> = world.get::<BehaviorInstance>(state).unwrap().calls.iter()
            .filter(|row| !row.terminal).filter(|row| {
                objects.resolve(SessionScopeId(row.origin.recipient.session), row.origin.recipient).ok()
                    .and_then(|entity| world.get::<action_results::ActionMailbox>(entity))
                    .is_some_and(|mailbox| mailbox.is_pending(&row.origin.ticket))
            }).map(|row| row.call.sequence).collect();
        let mut ready = Vec::new();
        let mut expired = Vec::new();
        let mut instance = world.get_mut::<BehaviorInstance>(state).unwrap();
        for row in instance.calls.iter_mut().filter(|row| !row.terminal) {
            if !live_origins.contains(&row.call.sequence) {
                row.terminal = true;
                row.progress.status = ActionDelivery::Retired;
                row.progress.last_error = Some("object_caller_retired".into());
                continue;
            }
            if row.written >= now { continue; }
            if row.remaining == 0 {
                row.terminal = true;
                expired.push(row.origin.clone());
                continue;
            }
            row.remaining -= 1;
            if ready.len() >= policy.max_per_tick.get() as usize { continue; }
            if row.progress.queue(now, row.written, method_bound(&admitted, &row.entry), "object_method_unbound") {
                row.max_attempts = policy.max_attempts.get();
                ready.push((row.call.clone(), row.entry.clone()));
            }
        }
        for origin in expired { publish(world, &origin, ActionStage::Failed, "object_call_expired", None); }
        for (value, path) in ready {
            let sequence = value.sequence;
            let entry = admitted.entries.iter().find(|entry| entry.path == path).unwrap();
            world.resource_mut::<ExtensionInvocations>().trigger_entry_with_receipt::<ObjectCallPort>(
                &ACTOR_THINK, path, entry.descriptor.trigger.selector.clone(), InvocationScope {subject, state}, value,
                InvocationReceipt {sequence, notify: settle});
        }
    }
}
