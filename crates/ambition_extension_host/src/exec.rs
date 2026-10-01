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

use ambition_extension_sdk::invoke::{HostParts, Observation, StagedRequest};
use ambition_extension_sdk::{Fault, Invocation, Name, Phase, Port, PortKey, Record, SchemaKey};
use ambition_time::SimTick;
use bevy::prelude::*;

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
}

/// The triggers queued in this phase.
#[derive(Resource, Default)]
pub struct ExtensionInvocations {
    pending: Vec<PendingInvocation>,
}

impl ExtensionInvocations {
    /// Queue a trigger of port `P` for one scope. Only the adapter that
    /// installed `P` calls this.
    pub fn trigger<P: Port>(
        &mut self,
        phase: &Phase,
        selector: impl Into<Name>,
        scope: Entity,
        occurrence: Option<u32>,
        value: P::Value,
    ) {
        self.pending.push(PendingInvocation {
            phase: phase.clone(),
            port: P::KEY,
            selector: selector.into(),
            scope,
            occurrence,
            value: Box::new(value),
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

/// The installed observation suppliers, by port.
#[derive(Resource, Default, Clone)]
pub struct ObservationSuppliers(pub(crate) Arc<HashMap<PortKey, Supplier>>);

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
        let suppliers = world.resource::<ObservationSuppliers>().0.clone();
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
                let observations: Vec<Observation> = descriptor
                    .reads
                    .iter()
                    .filter_map(|port| {
                        let supply = suppliers.get(port)?;
                        supply(world, invocation.scope).map(|value| Observation {
                            port: port.clone(),
                            value,
                        })
                    })
                    .collect();
                let current = world.get::<BodyRecords>(invocation.scope);
                let mut state: Vec<(SchemaKey, Record)> = descriptor
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
                let mut requests: Vec<StagedRequest> = Vec::new();

                let mut call = Invocation::from_host(HostParts {
                    entry: descriptor,
                    tick,
                    occurrence: invocation.occurrence,
                    trigger_port: &invocation.port,
                    trigger: invocation.value.as_ref(),
                    observations: &observations,
                    state: &mut state,
                    requests: &mut requests,
                });
                let result = (descriptor.run)(&mut call).and_then(|()| {
                    state.iter().try_for_each(|(key, record)| {
                        admitted.schemas[key]
                            .schema
                            .check(record)
                            .map_err(|error| Fault::Schema {
                                schema: key.clone(),
                                error,
                            })
                    })
                });

                match result {
                    Ok(()) => {
                        commit(world, &admitted, invocation, state);
                        let mut outbox = world.resource_mut::<ExtensionOutbox>();
                        outbox.items.extend(requests.into_iter().map(|r| OutboxItem {
                            port: r.port,
                            scope: invocation.scope,
                            occurrence: invocation.occurrence,
                            entry: path.clone(),
                            value: r.value,
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

fn commit(
    world: &mut World,
    admitted: &crate::Admitted,
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
