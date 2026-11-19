# Addressed behavior delivery

## Decision and scope

Action callbacks and directed object calls share one addressed delivery pass.
Native birth and activation subscriptions use this pass for typed facts. Keep typed
payloads and domain journals. Do not add a third dispatcher or copy domain
payloads into a second persistent queue to obtain common delivery.

The runtime [inbox](../../../crates/ambition_platformer2d_runtime/src/behavior_inbox.rs)
collects recipient occurrences from each native owner. It resolves each private
state's physical subject once, applies its room pace, orders eligible deliveries,
and shares one receiver budget between requests, outcomes and notifications. The host runs these
reserved invocations in its declared handler order. Only a successful host commit
acknowledges delivery. Shared `DeliveryProgress` owns retry and retirement
transitions. The native journals remain the canonical rollback state. The
collection map lasts for one phase and contains no undelivered payloads.
Receipt sequences are local to their typed lane. Outcome and request receipts
select their own native journal, even when their sequence numbers are equal.

The current pass runs in `actor_think`. Birth and activation subscriptions select
named methods. Signals, other lifecycle sources and confirmed world-domain
transport remain open.

## Keep the meanings distinct

| Payload | Authority and completion |
| --- | --- |
| Native event | An owning domain reports a fact that occurred. A handler cannot refuse or undo that fact. |
| Object call | A sender asks a behavior to perform work. Admission can refuse. Handler acknowledgement does not complete the call. |
| Action result | A native domain reports submission, admission, application or completion. Handler acknowledgement does not apply the action again. |
| Authored signal | A script asks a native signal owner to admit a typed, bounded message. It cannot claim to be a native fact. |

Action ticket identity, deduplication and cancellation stay with `ActionMailbox`.
Object-call admission, caller lifetime, active wait time and explicit completion
stay with `BehaviorInstance`. Their records use the same progress transitions
and host receipts. This avoids converting native typed outcomes into an opaque
bus and avoids changes to unrelated working gameplay.

## Read cuts and budgets

Each source must declare its first eligible read cut. Calls and outcomes now
wait for a later `actor_think` cut. Deferred notifications remain in `actor_think`. Existing synchronous native
hooks handle decisions that must affect the current physics/combat phase. Separate that eligibility
from the retry rule that prevents a second attempt in the same tick.

The current single-phase pass needs no persistent budget counter. Before adding
other delivery phases, retain the spent tick and count in recipient-owned
rollback state. One `max_per_tick` bound must cover all addressed payloads and
phases. Separate per-lane counters or a non-rewound local counter would break
that contract.

## Native subscriptions and authored signals

Implemented birth and activation subscriptions select an existing named behavior method. Preparation must
resolve that method to its entry and check the entry's native trigger type,
phase and selector against the source contract. A subscription must not add an
independent handler address table. Initial subscriptions are part of native
construction preparation. The set is fixed for an instance lifetime. A future
live replacement must validate the complete new set
before it retires removed pending deliveries.

Native domains publish typed facts after their own application boundary. A
completed birth uses `ObjectPublished`, after all native and private state is
initialized. Snapshot reconstruction and component removal are not birth or
retirement facts. Subscriptions use occurrence references, bounded retention,
explicit overflow diagnostics and deterministic order. Domain journals can keep
typed native values; portable execution still uses the port's checked codec.

Authored signals require native admission. The owner must check the source,
session, topic, data shape and byte bound, recipient bound, destination lifetime
and queue capacity. An atomic fanout must refuse before any recipient changes
if it cannot admit the full set. The sender needs an actual admission result.
Signal names cannot forge native event types. Admission must not expose raw ECS
writes or bypass native collision, damage, inventory or interaction decisions.

## Committed world boundary between islands

A live room is a location. The host declares the actual rollback authority.
Direct behavior calls, results and notifications are local to one authority.
Unsupported foreign calls must refuse before target mutation. Scope departure
ends the local call lifetime; it does not forward the method or its futures.

Cross-island consequences go through a native world domain. This domain admits
confirmed, identified transactions, not arbitrary remote methods. It can own
persistent state or typed transient transactions. Do not encode every operation
as a quest flag. General remote RPC is outside the current completion contract.

The existing confirmation boundary and source-owned room writes supply parts
of this road. A bridge retains an intent until source confirmation, assigns a
stable transaction identity, admits at a deterministic destination input cut
and replays that admission when the destination rewinds. Check idempotence,
source retirement and a traveling recipient. A confirmed `Update` write alone
is insufficient. Local `HostIntentLedger` input is not peer agreement, and
presentation quarantine is not a simulation transaction service.

Existing deferred native action publishers still need their work ownership
and traveling-recipient audit. A stable occurrence reference alone does not
authorize a speculative write into another island.

## State and notifications

Native state and transaction outcomes are the recovery authority. Notifications
wake scripts. Each source declares its retention and reconciliation contract.
A bounded notification can coalesce or drop only when its owning domain still
supplies the state needed for reconciliation. A transient fact needs retained
occurrence evidence or a reliable policy. Capture the source occurrence and
simulation scope at production; receipt must not require a living source.

The current birth mailbox reports overflow and retains delivery diagnostics.
It is a bounded notification path, not a guaranteed history of every birth.
Creation results and native live identity remain separate. Other source
families must establish recovery before they depend on this policy.

## Local construction references

Use a distinct symbolic local reference for nodes in one factory batch. Resolve
symbols inside the existing native construction plan, against its checked root
roster and reserved occurrence range. Validate every relation, method and state
value before materialization. Initialize all roots and private instances before
publishing any occurrence or creation fact. An unresolved symbol must refuse the
whole batch without spending the allocation counter or exposing a partial birth.
Give initializers a read-only map from checked local node keys to reserved
occurrence references. Resolve typed reference values through that map; do not
replace arbitrary strings inside script data. Do not create objects first and
connect them through later script callbacks.

## Next proof

Complete a playable switch/door/actor interaction in Ambition before further
generic delivery infrastructure. Use native interaction arbitration, a scoped
authored locator, native inspections and actions with acceptance/refusal. Check
state changes, duplicate live rooms, hot reload and rollback. Then add signals
and essential lifecycle sources needed by the composition.

The current playable proof is the [toll mechanism](../../sdk/reactive-mechanisms.md)
in `switch_lab`. Its native lever calls a destructible barrier behavior. Native
consumption commits the barrier and the activating body's coin exchange together.
The script completes the original call from that result. Activation evidence
supports reconciliation when notifications drop. Room and session publication
assign stable occurrences before the first scripted read.

A full duplicated-room mechanism test, additional lifecycle facts, authored
signals and a locking-door capability remain open. The toll barrier uses the
existing destructible return policy; it is not a general locking door.

The next cross-island proof is one committed world-domain consequence with
stable identity and replayable destination admission. Arbitrary remote object
calls and mixed actor/prop factories are not prerequisites for the local room.
The [script-first program](script-first-ambition-gameplay.md) remains open.
