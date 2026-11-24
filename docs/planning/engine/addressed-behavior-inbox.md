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

The current pass runs in `actor_think`. Birth, activation and defeat
subscriptions select named methods. Defeat (`object_defeated`, 2026-11-19) is
committed by the actor damage owner on the killing hit (`DefeatCommitted`) and
routed after the combat phase. A spent breakable (`object_broken`) is committed
by the breakable owner on every road that breaks one (`BrokenCommitted`), and
a taken pickup (`object_collected`) by the pickup owner at the take
(`CollectedCommitted`).
Authored signals (`emit_signal`, 2026-11-19) use
the same mailbox under the `object_signal` port. A post across rollback
islands (`post_world_signal`, 2026-11-19) reaches the same subscriptions
through the committed world road below. Retirement notifications also use this
mailbox through `ObjectRetiredPort`. The end-of-tick
[`publish_retirements`](../../../crates/ambition_platformer2d_runtime/src/behavior_events.rs)
pass finds subscribed source occurrences that no longer resolve and retains
`RetirementsHeard` state per receiver to avoid repeated notification. This
implements a notification path; it does not prove committed cross-island
retirement semantics. The authority boundary below still applies.

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

The source follow-up [AC1/AC2](native-action-completion.md) corrects two
producers, not this delivery pass. Summon completion comes from the native
construction result, not later existence. Synchronous cancellation keeps its
correlated result without reserving another pending slot; a full work pool
must not prevent control of work already admitted. Preserve the shared
delivery budget, journal bound and later-cut callback rules.

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

## Prepared routing and active delivery work

**State:** planned from source review at `aef73be10f3d`; no new runtime or
performance result. This applies the common
[execution specialization contract](execution-specialization-and-work-selection.md).
The [extension host](extension-state-and-execution.md#invocation-protocol-and-scheduling)
owns compiled dispatch. Lifecycle and inbox owners supply address views and
active journal work. Keep the current serial behavior as the reference.

### Compile the fixed handler relations

`AdmittedExtensions` and installed codec maps already use `Arc`; authored move
flows already share their specification. Preserve that work. The remaining
[host path](../../../crates/ambition_extension_host/src/exec.rs) matches phase
entries against pending triggers and resolves entry, method, supplier and schema
keys again during execution.

The extension host prepares ordered trigger spans, entry lookup and the static
supplier/schema/reply relations when it admits the generation. Behavior
preparation binds named methods through that result. Keep source names for
diagnostics and later rebinding; a generation-local index is not a saved identity.

Bucket queued triggers once, then execute in the existing admitted entry order
and original queue order within each entry. Do not batch mutable observations
across ordered commits. Each invocation still reads at its declared cut, stages
its bounded writes, checks dynamic output and settles only after host commit.
Keep the existing pace sampling and request-lowering order.

A module publication replaces these bindings with the admitted generation.
Rebind retained symbolic addresses or report the existing unavailable/unbound
outcome. An old index cannot select a different new entry. Delete repeated path
scans and entry-by-trigger matching as each consumer takes the prepared relation.
This is a host execution projection, not another dispatcher or interpreter.

**First slice (2026-11-22).** `run_phase` now groups the phase's queued
invocations by `(port, selector)` once, keeping queue order inside each
group. Each admitted entry reads only its own trigger's group, so an entry
no longer scans the whole queue; the order is still entry-major, then queue
order.

**Prepared dispatch (2026-11-24).** `Admitted::dispatch()` prepares, once
per admitted set (shared by its `Arc`), each phase's entries in serial order
and the entries bound to each trigger port and selector. `run_phase` walks
the phase's prepared entries instead of filtering `Admitted::entries`, and
the `Unavailable` receipt check asks only the entries bound to the
invocation's trigger. The inbox's `Candidate::bound` reads the same
relation: an event candidate looks up its trigger's entries, and an object
call walks only `ACTOR_THINK`'s entries. The order is unchanged:
entry-major, then queue order. A new admission prepares its own relation,
so an old index cannot select a new entry. Witness:
`the_prepared_dispatch_is_the_serial_order_by_trigger`. Still open:
resolving method, supplier and schema keys at admission (the execution
path still looks up `codecs.suppliers` per read port).

### Share an address view at each lifecycle cut

[`ObjectIndex::capture`](../../../crates/ambition_platformer2d_shared_tangle/src/lifecycle/object_refs.rs)
is a useful reference, but event publication, recipient selection and call
completion rebuild it at separate consumers. `resolve_object` scans occurrences
for one known reference. `object_behaviors::call` also rebuilds every private
behavior binding before it handles its request list.

The lifecycle owner supplies one immutable occurrence view per read cut; the
behavior owner supplies the private binding relation. Build or update these
views after the structural flush that defines that cut. Births, retirement,
reconstruction and address changes invalidate the affected relation before the
next reader. A candidate construction is absent until its publication barrier
commits. `ObjectPublished` observers must see the fully published batch.

Keep occurrence identity, current residency and query visibility distinct.
Ownership, travel and replay visibility changes update the operation's scope
view; they do not by themselves require a new occurrence-address relation.
Apply the room-replay visibility and authority rules, including foreign entities
hidden with `Disabled`. Preserve ambiguous, retired, foreign-session and
unpublished results. Capability absence does not remove an object from the
address population. Private state follows its current owner through travel
and entity reconstruction.

**First shared snapshots (2026-11-24).** Publishing a call's terminal
result adds no occurrence, so `object_behaviors::publish` takes its caller's
occurrence view. `complete` captures one view for its whole request batch
(it captured two for each request); `retire` captures one for all its
origins; and `maintain`, which already received a view, no longer captures
one more for each expired call. Event publication and recipient selection
still capture their own.

Start with shared snapshots at declared cuts, then use lifecycle updates where
they remove repeated builds. The full capture remains a synchronous fallback.
Do not rebuild unconditionally for every consumer, reuse a pre-flush snapshot
after births, or let a cold cache answer that a live object is retired.

The [frozen lookup kernel](data-structure-kernels.md#k2--ambiguity-preserving-frozen-lookup)
shares the repeated missing/unique/ambiguous row reduction in occurrence and
private-binding indexes. Authored placements can retain `Unique(None)` for an
unpublished root. It does not own capture timing or replace the lifecycle view
above. A duplicate-row tombstone cannot support incremental removal without
retained membership or a rebuild. Keep authored alias and registry duplicate
policies separate; they do not use this strict row-cardinality law.

### Derive ready and waiting work from the journals

Native facts already use `EventRoutes` and skip publication work when no route
exists. Preserve those paths. Install the common fact collector once; each new
native source must not add another all-mailbox scan.

**Done (2026-11-22).** Each `install_native_source` used to install its own
`collect::<EventMailbox>(Lane::Fact)` system, so the nine fact ports
(activated, defeated, broken, collected, born, crossed, crossing refused,
retired, signal) ran nine identical scans of every mailbox each tick. Each
scan pushed the same `Lane::Fact` onto the receiver's lane list. The first
source now installs the collector. Later sources call the host's new
`offer_extension_trigger`, which records the trigger offer for admission
with no adapter system. A new fact port adds no scan. Deriving active receiver
work from the journals (below) is still open.

Derive active receiver work from the canonical journals. Keep delivery-ready rows,
retry rows, incomplete calls with active-time deadlines, unbound handlers waiting
for a binding change, and frozen work distinct. A subscription with an empty
mailbox needs no delivery work. An acknowledged method can still own an unfinished
call and must remain in wait/lifetime maintenance.

Queueing, settlement and lifecycle transitions update this derived work set.
Advance existing active wait/retry sets and process due wake conditions; do not
reclassify every mailbox when the simulation tick advances. The set contains
addresses and eligibility, not copied domain payloads. Rollback rebuilds it from
restored journals before delivery. Bevy change ticks alone cannot establish that
it is current. Preserve later-cut eligibility (`written < now`), one attempt per
permitted tick, lane-local sequences, shared receiver budgets and settlement
after commit. Frozen rooms retain work and do not spend active wait time. Unbound
handlers can wake on the relevant generation change; fault retries keep their
existing policy.

No asynchronous worker-ready flag may decide whether a canonical delivery runs.
An invalidated address/work projection is rebuilt synchronously or uses the
reference pass. Preparation may change cost, not observable eligibility.

### Bounded migration and evidence

1. Prepare host dispatch spans and method bindings; delete repeated matching at
   the migrated consumer. Compare mixed broadcast and directed invocations with
   shared state against the current serial order, including a missing handler
   after reload and a fault that discards writes and requests.
2. Share occurrence/binding views across one publication/read cut. Exercise
   same-batch construction, retirement, duplicate identity, reconstruction and
   independent room replay before replacing the reference lookup there.
3. Derive active receiver work. Use the existing private behavior, shared-budget,
   cancellation and replay customers with many unrelated dormant subscriptions.
   Check active-time expiration through freeze and resume.

Record handler comparisons, index builds, visited receivers and allocations as
well as time. Keep native and portable output equal. The
[extension evidence plan](extension-iteration-evidence.md) owns measurement; this
section does not add a new queue, state format, global registry or guard suite.

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

Authored signals (implemented 2026-11-19, `behavior_events::admit_signal`)
are routed by `(sender, "ambition.objects.signal@1#<topic>")`. A subscribing
entry names the topic as its selector on `object_signal`, so a topic can never
spell a native fact's key. The runtime names the sender from the emitting
invocation's subject. A fanout of more than 16, a cross-island subscriber, a
full mailbox or over-bound data refuses the whole signal before any mailbox
changes. The sender's terminal result lists each recipient object.

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

The first road is a posted signal
([`world_posts`](../../../crates/ambition_platformer2d_runtime/src/world_posts.rs),
2026-11-19). `post_world_signal` holds the encoded `object_signal` row with
the sending room and tick (`RoomWrite`). A rewind of that room drops the posts
it made after its save, through a registered room-write retractor, and the
replay posts them again. On a live tick, once `ConfirmedFrameBoundary` passes
the sending tick, the post is admitted. Each subscription of the sender
resolves to its mailbox wherever the subscriber is then, so a recipient that
travelled is found and the source need not be alive. The admission is logged
by tick and recipient. While a room replays, the rows the road admitted to
that room on the replayed tick are queued again into the mailbox its rewind
restored. Each timeline therefore holds the post once. With one rollback
authority (local play, GGRS) a post is delivered at once. The witness
`a_world_post_crosses_rooms_once_and_only_when_confirmed` covers the hosted
host's cases: a speculative post retracted by a late input, a post held for
another guest's input whose sender leaves play before it is admitted, a
recipient that travelled to another room, and a recipient-room rewind across
the admission tick. Named world values now provide a persistent-state road
through `world_values.rs`. Their held-operation outcome and replay-stable
read contract still need the correction in
[causal world commit](causal-world-commit.md).

That plan also brings native shared-item, quest and flag consequences under
the same boundary. A store's undo log is not proof of causal isolation:
another room may already have used the provisional value. Keep local
notifications immediate inside one rollback authority. Use confirmed
domain operations and recorded destination admission across authorities.

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

Local mechanism receipt: `8ee92f38e` adds native placement lookup, activation
state and notifications, and self-consumption with inventory outcomes. The
`a_scripted_lever_spends_its_barrier` app checks cover refused/applied actions,
collision removal, charge deduplication, GGRS replay, lost wakeups and Rhai reload.
The shared locator check covers equal placement IDs in two live room instances.

Defeat is now a subscribable fact (`a_subscribed_method_hears_the_defeat_of_its_source_once`).
A room entered again is a new copy: a subscription made in the old copy hears
nothing from the new one, and a fresh lookup subscribes to the new copy's
object (`a_room_entered_again_is_a_new_copy_and_only_its_own_watch_hears_its_barrier`).
No production road holds two copies of one room live at once (a crossing
into a live room joins it), so that case is the locator check's alone. Body
retirement facts wait for a customer mechanic; a locking-door capability
remains open. The toll barrier uses the
existing destructible return policy; it is not a general locking door.

The first committed world-domain consequence is a posted signal (see
"Committed world boundary between islands"). Arbitrary remote object calls
and mixed actor/prop factories are not prerequisites for the local room.
The [script-first program](script-first-ambition-gameplay.md) remains open.
