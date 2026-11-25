//! One addressed delivery pass for typed outcomes, requests and native facts.

use std::collections::BTreeMap;
use ambition_extension_host::{AdmittedExtensions, ExtensionInvocations, ExtensionSet,
    InvocationEvent, InvocationReceipt, InvocationScope};
use ambition_extension_sdk::{ObjectRef, Port, action::{ActionResultPort, ActionTicket}, phases::ACTOR_THINK};
use ambition_platformer2d_shared_tangle::{action_results::{ActionDeliveryPolicy, ActionMailbox}, lifecycle::*};
use ambition_platformer2d_world::governing_rules::RulesOf;
use ambition_world_port::behavior::ObjectCallPort;
use bevy::{ecs::system::SystemState, prelude::*};
use crate::object_behaviors::{self, BehaviorInstance};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Lane { Outcome, Request, Fact }

/// Collection is transient. The domain journals retain all delivery progress.
#[derive(Resource, Default)]
pub(crate) struct BehaviorInbox { sources: BTreeMap<ObjectRef, (Entity, Vec<Lane>)> }

pub(crate) fn register_rollback_state(registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar) {
    registrar.declare_rollback_derived_resource::<BehaviorInbox>(
        "ambition_platformer2d_runtime", "derived.behavior_inbox",
        "The collection cut rebuilds this map from canonical domain journals; delivery drains it before the phase ends.");
}

pub(crate) fn install(app: &mut App) {
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
    app.init_resource::<BehaviorInbox>();
    let schedule = app.sim_schedule();
    app.add_systems(schedule, (retire_orphans, deliver).chain().in_set(ExtensionSet::Deliver(ACTOR_THINK)));
}

/// Collect the receivers of one lane that have journal work: a pending row,
/// an open call, or a caller watch. An idle receiver costs nothing here; an
/// orphaned one is retired by [`retire_orphans`], not by delivery.
pub(crate) fn collect<T: Component>(lane: Lane, active: fn(&T) -> bool) -> impl Fn(
    Query<(Entity, &SessionScopedEntity, &ObjectOccurrence, &T, Option<&object_behaviors::LocalCallWatches>)>, ResMut<BehaviorInbox>) {
    move |scopes, mut inbox| {
        for (state, session, occurrence, journal, watches) in &scopes {
            if !active(journal) && watches.is_none_or(|watches| watches.is_empty()) { continue; }
            let (_, lanes) = inbox.sources.entry(occurrence.reference(session.0)).or_insert((state, Vec::new()));
            lanes.push(lane);
        }
    }
}

struct Candidate { written: u64, sequence: u64, path: String, origin: Option<ActionTicket>, event: Option<String>, lane: Lane }
impl Candidate {
    fn bound<'a>(&self, admitted: &'a ambition_extension_host::Admitted, sources: &crate::behavior_events::NativeSources) -> Option<&'a ambition_extension_host::AdmittedEntry> {
        if let Some(event) = &self.event {
            let (source, selector) = sources.get(event)?;
            // The entries bound to this trigger, prepared at admission (E3).
            return admitted.dispatch().bound_to(&ACTOR_THINK, &source.port, selector).iter()
                .map(|&index| &admitted.entries[index]).find(|entry| entry.path == self.path);
        }
        match &self.origin {
            Some(ticket) => ambition_extension_host::reply_entry::<ActionResultPort>(admitted, &ticket.entry, &self.path, &ACTOR_THINK),
            None => admitted.dispatch().entries_of(&ACTOR_THINK).iter().map(|&index| &admitted.entries[index])
                .find(|entry| entry.path == self.path && entry.descriptor.trigger.port == ObjectCallPort::KEY),
        }
    }
}

/// The lifecycle facts an owner's resolution reads: object addresses, their
/// session, their enablement, and the state's own owner link and journals.
type OwnerCut = SystemState<(
    RemovedComponents<'static, 'static, ObjectOccurrence>,
    RemovedComponents<'static, 'static, SessionScopedEntity>,
    RemovedComponents<'static, 'static, bevy::ecs::entity_disabling::Disabled>,
    Query<'static, 'static, (), Or<(Changed<ObjectOccurrence>, Changed<SessionScopedEntity>, Changed<ObjectScopeOwner>,
        Added<BehaviorInstance>, Added<ActionMailbox>, Added<crate::behavior_events::EventMailbox>)>>,
    Query<'static, 'static, (), (Added<bevy::ecs::entity_disabling::Disabled>, With<ObjectOccurrence>)>,
)>;

/// Retire a behavior state whose owner no longer resolves, at the owner's
/// lifecycle cut: its open calls fail and the state is despawned. Runs only
/// when an address, enablement or owner link changed, so idle states are not
/// revisited each tick; delivery collects only receivers with work.
fn retire_orphans(world: &mut World, mut cut: Local<Option<OwnerCut>>) {
    let cut = cut.get_or_insert_with(|| SystemState::new(world));
    let (mut lost, mut left, mut enabled, changed, disabled) = cut.get_mut(world).expect("owner lifecycle readers are installed");
    // Drain every reader so each change is seen once.
    let moved = lost.read().count() + left.read().count() + enabled.read().count() > 0;
    let moved = moved | !changed.is_empty() | !disabled.is_empty();
    cut.apply(world);
    if !moved { return; }
    let objects = ObjectIndex::capture(world);
    let orphans: Vec<_> = world.query_filtered::<(Entity, &SessionScopedEntity, &ObjectOccurrence),
        (With<ObjectScopeOwner>, Or<(With<BehaviorInstance>, With<ActionMailbox>, With<crate::behavior_events::EventMailbox>)>)>()
        .iter(world)
        .filter(|(state, session, occurrence)| objects.resolve(session.0, occurrence.reference(session.0)) == Ok(*state))
        .map(|(state, ..)| state)
        .filter(|state| subject(world, *state, &objects).is_none())
        .collect();
    for state in orphans {
        object_behaviors::retire(world, state);
        world.despawn(state);
    }
}

type DeliveryReads = SystemState<(RoomFreeze<'static, 'static>, RulesOf<'static, 'static, ActionDeliveryPolicy>)>;
fn deliver(world: &mut World, mut reads: Local<Option<DeliveryReads>>) {
    let sources = std::mem::take(&mut world.resource_mut::<BehaviorInbox>().sources);
    if sources.is_empty() { return; }
    let objects = ObjectIndex::capture(world);
    let admitted = world.resource::<AdmittedExtensions>().0.clone();
    let now = world.resource::<ambition_time::SimTick>().0;
    let reads = reads.get_or_insert_with(|| SystemState::new(world));
    for (reference, (state, lanes)) in sources {
        // Validate the collected address again before it can receive a payload.
        if objects.resolve(SessionScopeId(reference.session), reference) != Ok(state) { continue; }
        // An orphan was retired at its owner's cut, before this pass.
        let Some(subject) = subject(world, state, &objects) else { continue; };
        let (pace, policies) = reads.get(world).expect("addressed delivery readers are installed");
        if world.resource::<ambition_time::WorldTime>().sim_dt() <= 0.0 || pace.scale_of(subject) <= 0.0 { continue; }
        let policy = policies.of(subject).copied().unwrap_or_default();
        object_behaviors::maintain_watches(world, state, &objects, now);
        if lanes.contains(&Lane::Request) { object_behaviors::maintain(world, state, &objects, now); }
        let mut candidates = Vec::new();
        if let Some(mailbox) = world.get::<ActionMailbox>(state) {
            candidates.extend(mailbox.deliveries().map(|(sequence, written, path, ticket)| Candidate {
                sequence, written, path: path.into(), origin: Some(ticket.clone()), event: None, lane: Lane::Outcome }));
        }
        if let Some(instance) = world.get::<BehaviorInstance>(state) {
            candidates.extend(instance.deliveries(world, state, &objects).map(|(sequence, written, path)| Candidate {
                sequence, written, path: path.into(), origin: None, event: None, lane: Lane::Request }));
        }
        if let (Some(events), Some(instance)) = (world.get::<crate::behavior_events::EventMailbox>(state), world.get::<BehaviorInstance>(state)) {
            candidates.extend(events.deliveries(instance).map(|(sequence, written, path, event)| Candidate {
                sequence, written, path, origin: None, event: Some(event.into()), lane: Lane::Fact }));
        }
        // Admission follows age, then typed lane and local sequence. The host
        // retains its declared handler order when it executes the admitted batch.
        candidates.sort_by_key(|row| (row.written, row.lane, row.sequence));
        let mut allowance = policy.max_per_tick.get();
        for candidate in candidates {
            if allowance == 0 { break; }
            let entry = candidate.bound(&admitted, world.resource::<crate::behavior_events::NativeSources>());
            let scope = InvocationScope {subject, state};
            let queued = match candidate.lane {
                Lane::Outcome => {
                    let value = world.get_mut::<ActionMailbox>(state).unwrap()
                        .queue_delivery(candidate.sequence, now, entry.is_some());
                    if let Some(value) = value {
                        world.resource_mut::<ExtensionInvocations>().trigger_entry_with_receipt::<ActionResultPort>(
                            &ACTOR_THINK, candidate.path, entry.unwrap().descriptor.trigger.selector.clone(), scope, value,
                            InvocationReceipt {sequence: candidate.sequence, notify: settle_outcome});
                        true
                    } else { false }
                }
                Lane::Request => {
                    let value = world.get_mut::<BehaviorInstance>(state).unwrap()
                        .queue_delivery(candidate.sequence, now, entry.is_some(), policy.max_attempts.get());
                    if let Some(value) = value {
                        world.resource_mut::<ExtensionInvocations>().trigger_entry_with_receipt::<ObjectCallPort>(
                            &ACTOR_THINK, candidate.path, entry.unwrap().descriptor.trigger.selector.clone(), scope, value,
                            InvocationReceipt {sequence: candidate.sequence, notify: object_behaviors::settle});
                        true
                    } else { false }
                }
                Lane::Fact => {
                    let source = candidate.event.as_ref().and_then(|event| world.resource::<crate::behavior_events::NativeSources>().get(event)
                        .map(|(source, selector)| (source.dispatch, selector.to_owned())));
                    let value = world.get_mut::<crate::behavior_events::EventMailbox>(state).unwrap()
                        .queue_delivery(candidate.sequence, now, entry.is_some(), policy.max_attempts.get());
                    if let Some(value) = value {
                        let (dispatch, selector) = source.unwrap();
                        if dispatch(&mut world.resource_mut::<ExtensionInvocations>(), &candidate.path, &selector,
                            scope, &value, InvocationReceipt {sequence: candidate.sequence, notify: crate::behavior_events::settle}).is_err() {
                            world.get_mut::<crate::behavior_events::EventMailbox>(state).unwrap().invalid_payload(candidate.sequence);
                        }
                        true
                    } else { false }
                }
            };
            if queued { allowance -= 1; }
        }
    }
}

fn subject(world: &World, state: Entity, objects: &ObjectIndex) -> Option<Entity> {
    match world.get::<ObjectScopeOwner>(state) {
        Some(owner) => objects.resolve(SessionScopeId(owner.0.session), owner.0).ok(),
        None => Some(state),
    }.filter(|entity| world.get_entity(*entity).is_ok())
}

fn settle_outcome(world: &mut World, state: Entity, sequence: u64, event: InvocationEvent) {
    let now = world.resource::<ambition_time::SimTick>().0;
    let attempts = if event == InvocationEvent::Faulted {
        let objects = ObjectIndex::capture(world);
        let subject = subject(world, state, &objects).unwrap_or(state);
        let mut reads = SystemState::<RulesOf<ActionDeliveryPolicy>>::new(world);
        reads.get(world).ok().and_then(|rules| rules.of(subject).copied()).unwrap_or_default().max_attempts.get()
    } else { 1 };
    if let Some(mut mailbox) = world.get_mut::<ActionMailbox>(state) {
        mailbox.settle_delivery(sequence, now, event, attempts);
    }
}
