//! Serial invocation: triggers in, staged state and requests out.
//!
//! For one phase, in one tick:
//!
//! 1. `ExtensionSet::Collect(phase)` — trigger adapters queue
//!    [`PendingInvocation`]s with a read-cut value for each scope.
//! 2. `ExtensionSet::Invoke(phase)` — [`run_phase`] calls each admitted entry,
//!    in admitted order, for each queued trigger it is bound to, in queue
//!    order. It checks the staged output and commits it, or discards all of
//!    it on a fault.
//! 3. `ExtensionSet::Lower(phase)` — request adapters drain
//!    [`ExtensionOutbox`] and give each request to the owning domain.
//!
//! The queue and the outbox are empty at the end of the phase. They hold no
//! state from one tick to the next, so they are not rollback state.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use ambition_extension_sdk::abi::{self, InvocationInput, InvocationOutput};
use ambition_extension_sdk::invoke::{HostParts, Observation, OwnedPayload, Payload, StagedRequest};
use ambition_extension_sdk::{
    encode_erased, DecodeFn, EncodeFn, Fault, IdlePolicy, Invocation, Name, Phase, Port, PortKey,
    Record, SchemaKey,
};
use ambition_time::SimTick;
use bevy::prelude::*;

use crate::admission::{Admitted, AdmittedEntry, EntryRunner};
use crate::store::BodyRecords;
use crate::AdmittedExtensions;

/// One queued trigger.
pub struct PendingInvocation {
    pub phase: Phase,
    pub port: PortKey,
    pub selector: Name,
    pub scope: Entity,
    pub occurrence: Option<u32>,
    pub value: Box<dyn Any + Send + Sync>,
    /// Encodes `value` for a loaded module.
    pub encode: EncodeFn,
    /// The trigger's domain says nothing is happening for this scope this
    /// tick. An entry with `IdlePolicy::ResetState` is not called.
    pub idle: bool,
}

/// The triggers queued in this phase.
#[derive(Resource, Default)]
pub struct ExtensionInvocations {
    pending: Vec<PendingInvocation>,
}

impl ExtensionInvocations {
    /// Queue a trigger of port `P` for one scope. Only the adapter that
    /// installed `P` calls this. `idle` is the port's own statement that
    /// nothing is happening for this scope this tick (its card defines it).
    pub fn trigger<P: Port>(
        &mut self,
        phase: &Phase,
        selector: impl Into<Name>,
        scope: Entity,
        occurrence: Option<u32>,
        idle: bool,
        value: P::Value,
    ) {
        self.pending.push(PendingInvocation {
            phase: phase.clone(),
            port: P::KEY,
            selector: selector.into(),
            scope,
            occurrence,
            value: Box::new(value),
            encode: encode_erased::<P>,
            idle,
        });
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// A request that passed the host's checks. It is SUBMITTED; the domain that
/// owns the port decides if it is APPLIED.
pub struct Submitted<T> {
    /// The body the invocation ran for.
    pub scope: Entity,
    /// The move use that asked for the invocation. The host sets it; the
    /// module cannot.
    pub occurrence: Option<u32>,
    /// `provider::module/entry`, for diagnostics.
    pub entry: Arc<str>,
    pub value: T,
}

struct OutboxItem {
    port: PortKey,
    scope: Entity,
    occurrence: Option<u32>,
    entry: Arc<str>,
    value: Box<dyn Any + Send + Sync>,
}

/// Submitted requests waiting for their domain adapters.
#[derive(Resource, Default)]
pub struct ExtensionOutbox {
    items: Vec<OutboxItem>,
}

impl ExtensionOutbox {
    /// Remove and return every submitted request of port `P`, in submit order.
    pub fn drain<P: Port>(&mut self) -> Vec<Submitted<P::Value>> {
        let mut out = Vec::new();
        let mut keep = Vec::with_capacity(self.items.len());
        for item in self.items.drain(..) {
            if item.port != P::KEY {
                keep.push(item);
                continue;
            }
            match item.value.downcast::<P::Value>() {
                Ok(value) => out.push(Submitted {
                    scope: item.scope,
                    occurrence: item.occurrence,
                    entry: item.entry,
                    value: *value,
                }),
                // The SDK boxes `P::Value` under `P::KEY`, so a key with
                // another type is two ports claiming one key.
                Err(_) => panic!("two port types use the key {}", P::KEY),
            }
        }
        self.items = keep;
        out
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// A recorded fault. Diagnostic only: it is not rollback state.
#[derive(Clone, Debug)]
pub struct FaultRecord {
    pub tick: u64,
    pub entry: Arc<str>,
    pub scope: Entity,
    pub fault: Fault,
}

/// Every fault of this App. `total` is exact; `recent` keeps the last
/// [`ExtensionFaults::RECENT`] records.
#[derive(Resource, Default, Debug)]
pub struct ExtensionFaults {
    pub total: u64,
    pub recent: Vec<FaultRecord>,
}

impl ExtensionFaults {
    pub const RECENT: usize = 32;

    fn record(&mut self, record: FaultRecord) {
        self.total += 1;
        if self.recent.len() == Self::RECENT {
            self.recent.remove(0);
        }
        self.recent.push(record);
    }
}

/// A domain's projection for one observation port: the value for one scope at
/// the read cut, or `None` when the domain has no value for that scope.
pub type Supplier = Arc<dyn Fn(&World, Entity) -> Option<Box<dyn Any + Send + Sync>> + Send + Sync>;

/// What the installed ports need at invocation time, frozen at admission:
/// each observation port's projection and encoder, and each request port's
/// decoder (a loaded module hands its requests back as bytes).
#[derive(Resource, Default, Clone)]
pub struct InstalledPortCodecs {
    pub(crate) suppliers: Arc<HashMap<PortKey, (Supplier, EncodeFn)>>,
    pub(crate) request_decoders: Arc<HashMap<PortKey, DecodeFn>>,
}

/// What an invocation staged, after the host's checks.
type Staged = (Vec<(SchemaKey, Record)>, Vec<(PortKey, Box<dyn Any + Send + Sync>)>);

/// The exclusive system for one phase.
pub fn run_phase(phase: Phase) -> impl FnMut(&mut World) {
    move |world: &mut World| {
        let pending: Vec<PendingInvocation> = {
            let mut queue = world.resource_mut::<ExtensionInvocations>();
            let (mine, rest) = std::mem::take(&mut queue.pending)
                .into_iter()
                .partition(|p| p.phase == phase);
            queue.pending = rest;
            mine
        };
        if pending.is_empty() {
            return;
        }
        let admitted = world.resource::<AdmittedExtensions>().0.clone();
        let codecs = world.resource::<InstalledPortCodecs>().clone();
        // ⛔ A REQUIRED AUTHORITY, NOT AN OPTION. An entry reads the logical
        // tick; a composition with modules and no `SimTick` is broken, and a
        // default of zero would hide that.
        let tick = world.resource::<SimTick>().get();

        for entry in admitted.entries_in(&phase) {
            let descriptor = &entry.descriptor;
            let path: Arc<str> = Arc::from(entry.path.as_str());
            for invocation in pending.iter().filter(|p| {
                p.port == descriptor.trigger.port && p.selector == descriptor.trigger.selector
            }) {
                if world.get_entity(invocation.scope).is_err() {
                    continue;
                }
                if invocation.idle && descriptor.on_idle == IdlePolicy::ResetState {
                    reset_idle(world, &admitted, invocation.scope, &descriptor.writes);
                    continue;
                }
                let current = world.get::<BodyRecords>(invocation.scope);
                let state: Vec<(SchemaKey, Record)> = descriptor
                    .writes
                    .iter()
                    .map(|key| {
                        let record = current
                            .and_then(|r| r.get(key))
                            .cloned()
                            .unwrap_or_else(|| admitted.schemas[key].schema.initial_record());
                        (key.clone(), record)
                    })
                    .collect();

                let result = match &entry.runner {
                    EntryRunner::Native(run) => {
                        run_native(world, entry, *run, invocation, &codecs, tick, state)
                    }
                    EntryRunner::Loaded {
                        backend,
                        module,
                        entry: index,
                    } => run_loaded(
                        world, &admitted, entry, backend.as_ref(), *module, *index, invocation,
                        &codecs, tick, state,
                    ),
                }
                .and_then(|staged| {
                    staged.0.iter().try_for_each(|(key, record)| {
                        admitted.schemas[key]
                            .schema
                            .check(record)
                            .map_err(|error| Fault::Schema {
                                schema: key.clone(),
                                error,
                            })
                    })?;
                    Ok(staged)
                });

                match result {
                    Ok((state, requests)) => {
                        commit(world, &admitted, invocation, state);
                        let mut outbox = world.resource_mut::<ExtensionOutbox>();
                        outbox.items.extend(requests.into_iter().map(|(port, value)| OutboxItem {
                            port,
                            scope: invocation.scope,
                            occurrence: invocation.occurrence,
                            entry: path.clone(),
                            value,
                        }));
                    }
                    Err(fault) => {
                        warn!(
                            "extension entry {path} faulted for {:?} at tick {tick}: {fault}; \
                             its staged state and requests are discarded",
                            invocation.scope
                        );
                        world.resource_mut::<ExtensionFaults>().record(FaultRecord {
                            tick,
                            entry: path.clone(),
                            scope: invocation.scope,
                            fault,
                        });
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_native(
    world: &World,
    entry: &AdmittedEntry,
    run: ambition_extension_sdk::EntryFn,
    invocation: &PendingInvocation,
    codecs: &InstalledPortCodecs,
    tick: u64,
    mut state: Vec<(SchemaKey, Record)>,
) -> Result<Staged, Fault> {
    let observations: Vec<Observation> = entry
        .descriptor
        .reads
        .iter()
        .filter_map(|port| {
            let (supply, _) = codecs.suppliers.get(port)?;
            supply(world, invocation.scope).map(|value| Observation {
                port: port.clone(),
                value: OwnedPayload::Native(value),
            })
        })
        .collect();
    let mut requests: Vec<StagedRequest> = Vec::new();
    let mut call = Invocation::from_host(HostParts {
        entry: &entry.descriptor,
        tick,
        occurrence: invocation.occurrence,
        trigger_port: &invocation.port,
        trigger: Payload::Native(invocation.value.as_ref()),
        observations: &observations,
        state: &mut state,
        requests: &mut requests,
    });
    run(&mut call)?;
    drop(call);
    Ok((state, requests.into_iter().map(|r| (r.port, r.value)).collect()))
}

/// Run an entry of a loaded module: encode the read cut, call the backend,
/// and accept its output only if every record matches a DECLARED write and
/// every request is a declared port that decodes, within the entry's limit.
#[allow(clippy::too_many_arguments)]
fn run_loaded(
    world: &World,
    admitted: &Admitted,
    entry: &AdmittedEntry,
    backend: &dyn crate::ModuleBackend,
    module: u32,
    index: u32,
    invocation: &PendingInvocation,
    codecs: &InstalledPortCodecs,
    tick: u64,
    state: Vec<(SchemaKey, Record)>,
) -> Result<Staged, Fault> {
    let descriptor = &entry.descriptor;
    let mut trigger = Vec::new();
    (invocation.encode)(invocation.value.as_ref(), &mut trigger);
    let observations: Vec<(PortKey, Vec<u8>)> = descriptor
        .reads
        .iter()
        .filter_map(|port| {
            let (supply, encode) = codecs.suppliers.get(port)?;
            let value = supply(world, invocation.scope)?;
            let mut bytes = Vec::new();
            encode(value.as_ref(), &mut bytes);
            Some((port.clone(), bytes))
        })
        .collect();
    let input = InvocationInput {
        tick,
        occurrence: invocation.occurrence,
        trigger_port: &invocation.port,
        trigger: &trigger,
        observations: &observations,
        state: &state,
    }
    .encode();
    let output = backend
        .invoke(module, index, &input)
        .map_err(|e| Fault::Module(format!("the loaded module failed: {e}").into()))?;
    let output = abi::read_output(&output, |key| {
        descriptor
            .writes
            .contains(key)
            .then(|| admitted.schemas.get(key).map(|a| &a.schema))
            .flatten()
    })
    .map_err(|e| Fault::Module(format!("the module's output does not decode: {e}").into()))?;
    let (out_state, out_requests) = match output {
        InvocationOutput::Fault(message) => return Err(Fault::Module(message.into())),
        InvocationOutput::Ok { state, requests } => (state, requests),
    };
    let limit = descriptor.limits.max_requests;
    if out_requests.len() > limit as usize {
        return Err(Fault::RequestLimit { limit });
    }
    // The staged state starts as the read state; the module's records replace
    // the ones it returned. A record it did not return keeps its value.
    let mut staged = state;
    for (key, record) in out_state {
        if let Some(slot) = staged.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = record;
        }
    }
    let mut requests = Vec::with_capacity(out_requests.len());
    for (port, bytes) in out_requests {
        if !descriptor.requests.contains(&port) {
            return Err(Fault::UndeclaredRequest(port));
        }
        let decode = codecs
            .request_decoders
            .get(&port)
            .ok_or_else(|| Fault::UndeclaredRequest(port.clone()))?;
        let value = decode(&bytes).map_err(|e| {
            Fault::Module(format!("request to {port} does not decode: {e}").into())
        })?;
        requests.push((port, value));
    }
    Ok((staged, requests))
}

/// `IdlePolicy::ResetState`: put the entry's records back to their initial
/// values without calling it. A body that has no record yet, or whose record
/// is already initial, is left alone, so an idle tick writes nothing.
fn reset_idle(world: &mut World, admitted: &Admitted, scope: Entity, writes: &[SchemaKey]) {
    let Some(current) = world.get::<BodyRecords>(scope) else {
        return;
    };
    let stale: Vec<SchemaKey> = writes
        .iter()
        .filter(|key| {
            current
                .get(key)
                .is_some_and(|record| *record != admitted.schemas[*key].schema.initial_record())
        })
        .cloned()
        .collect();
    if stale.is_empty() {
        return;
    }
    let mut records = world
        .get_mut::<BodyRecords>(scope)
        .expect("read on the lines above");
    for key in stale {
        let schema = &admitted.schemas[&key];
        records.put(key, schema.shape, schema.schema.initial_record());
    }
}

fn commit(
    world: &mut World,
    admitted: &Admitted,
    invocation: &PendingInvocation,
    state: Vec<(SchemaKey, Record)>,
) {
    if state.is_empty() {
        return;
    }
    let mut entity = world.entity_mut(invocation.scope);
    if !entity.contains::<BodyRecords>() {
        entity.insert(BodyRecords::default());
    }
    let mut records = entity
        .get_mut::<BodyRecords>()
        .expect("inserted on the line above");
    for (key, record) in state {
        let shape = admitted.schemas[&key].shape;
        records.put(key, shape, record);
    }
}
