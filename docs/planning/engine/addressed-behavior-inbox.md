# Addressed behavior delivery

## Decision and scope

Action callbacks and directed object calls share one addressed delivery pass.
Native event subscriptions must use this pass when they are added. Keep typed
payloads and domain journals. Do not add a third dispatcher or copy domain
payloads into a second persistent queue to obtain common delivery.

The runtime [inbox](../../../crates/ambition_platformer2d_runtime/src/behavior_inbox.rs)
collects recipient occurrences from each native owner. It resolves each private
state's physical subject once, applies its room pace, orders eligible deliveries,
and shares one receiver budget between calls and outcomes. The host runs these
reserved invocations in its declared handler order. Only a successful host commit
acknowledges delivery. Shared `DeliveryProgress` owns retry and retirement
transitions. The native journals remain the canonical rollback state. The
collection map lasts for one phase and contains no undelivered payloads.
Receipt sequences are local to their typed lane. Outcome and request receipts
select their own native journal, even when their sequence numbers are equal.

The current pass runs in `actor_think`. It does not establish subscriptions,
signals, delivery in other phases or safe cross-island transport.

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
wait for a later `actor_think` cut. A damage fact can enter its declared
`hit_reaction` cut after the native damage decision. Separate that eligibility
from the retry rule that prevents a second attempt in the same tick.

The current single-phase pass needs no persistent budget counter. Before adding
other delivery phases, retain the spent tick and count in recipient-owned
rollback state. One `max_per_tick` bound must cover all addressed payloads and
phases. Separate per-lane counters or a non-rewound local counter would break
that contract.

## Native subscriptions and authored signals

A subscription must select an existing named behavior method. Preparation must
resolve that method to its entry and check the entry's native trigger type,
phase and selector against the source contract. A subscription must not add an
independent handler address table. Initial subscriptions are part of native
construction preparation. Live replacement must validate the complete new set
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

## Confirmed transactions between islands

A live room is a location. It is not sufficient evidence of an independent
rollback island. Determine the actual rollback authority before a delivery
crosses an island boundary. The [island plan](island-rollback-and-compact-peer-input.md)
requires confirmed, idempotent world transactions at that boundary.

The existing [confirmation boundary](../../../crates/ambition_platformer2d_core/src/confirmed_frame.rs)
reports confirmation. [Source-owned room writes](../../../crates/ambition_platformer2d_shared_tangle/src/lifecycle/room_writes.rs)
can be retracted during room replay. These mechanisms do not yet form a
complete addressed-message transport. A fully confirmed test in an `Update`
system, followed by a direct destination write, is insufficient: the destination
can restore an earlier snapshot and lose that write.

A transport must retain a source-owned intent until confirmation, give each
transaction a stable idempotence key, admit it at an agreed destination input
cut, and replay that admission when the destination rewinds. It must define
refusal, source retirement, recipient travel, pending-call cancellation and
result delivery. Capture the island that owns the admitted native work; the
occurrence's allocation room is not its current location. Resolve a traveling
recipient by its stable occurrence at destination admission. Peer ingress must
follow agreed frame input or the existing confirmed lifecycle protocol. The local-only `HostIntentLedger` is not peer
agreement. Presentation-only external-effect quarantine is not a simulation
message transport.

Do not let a speculative source write a foreign inbox. Apply this restriction
to calls and action outcomes as well as signals. Existing direct cross-room
calls and traveling replies still need this audit before the general model can
be called safe for independent room replay.

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

Use one relay prop and one actor behavior. Create their mutual references in
one native batch. A native fact activates the relay's declared method. It asks
the actor to perform a native action. The actor observes the actual accepted or
refused result and responds through its declared behavior. Check a competing
request, a handler fault, room duplication, travel, retirement, live replacement
and replay with the same small composition. Add a focused two-island check that
proves speculative work cannot mutate the destination and confirmed delivery
survives destination replay. Keep native authority and portable parity.

Native subscriptions, signal admission, mixed actor/prop factory batches and the
confirmed transport remain implementation work. Actor and prop factories now
resolve typed local references in their own native construction batches. This evaluation does not close the
[script-first program](script-first-ambition-gameplay.md).
