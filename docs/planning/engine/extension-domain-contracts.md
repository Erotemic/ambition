# Procedural extensions and domain contracts

**State:** selected design. The [extension model](extension-model.md) owns the
architecture. This page owns port, request and observation semantics. The
[state contract](extension-state-and-execution.md) owns extension state. No generic
replacement for Bevy, domain scheduling or combat arbitration is proposed.

## Place the behavior before adding an interface

| Need | Correct owner | Incorrect shortcut |
| --- | --- | --- |
| Compose a known move, encounter or interaction | Existing authored domain values and preparation | New script runtime for data the domain already expresses |
| Compute a graph, strategy, quest predicate or boss algorithm | Procedural module and its registered state | New core opcode for that named mechanic |
| Apply a hit, move a body, transfer an item or establish custody | Existing domain admission and reducer | Extension setter for health, velocity, inventory or relation fields |
| Add a fundamental geometry, movement or rendering service | Bevy-native engine plugin/domain | Reimplement the service privately in every module |
| Observe what an NPC knows | Perception/knowledge projection selected by its authority | Giving every brain an unrestricted world snapshot |
| Inspect authoritative state for tools or a game rule | Explicit read-only diagnostic/rule projection | Treating all observations as belief or all callers as omniscient |

Procedural power means arbitrary computation and new owned state. It does not mean
arbitrary mutation of other owners. A missing reusable service is engine work;
subsequent algorithms using that service remain module-only edits.

## Keep protocol dependencies small

The portable core supplies schema/wire primitives, checked call handles, execution
context and diagnostics. It does not import every domain's payloads. A domain owns
its pure port values in an existing dependency-light owner where possible. Extract
a leaf only when a real module consumer needs those values without the runtime.

```text
pure port values <--- module binding
       ^
       |
owner adapter ---> domain runtime
       |
       +--------> extension host executor

composition installs owner adapters; executor does not import composition
```

An in-process adapter may use normal typed Rust/Bevy APIs. Portable bindings lower
one versioned domain contract to bounded values. Avoid a closed central
`AllEngineRequests` enum, reflection-based `get_component(name)` API, generic
mutable property bag, or SDK feature that imports the entire facade. An adapter
crate is justified only by a dependency direction that needs it; not one per
message or one wrapper per existing function.

## What installing a port proves

Install descriptor, implementation, version, phase, access scope and failure
policy together. A module requirement matches that installed offer. The existing
`InstalledTechniques` / `TechniqueSupport` split is a model of handing plain data
into admission, not permission to move runtime authority into the pure catalog.

For each first port, the implementation packet must fill this contract card:

| Field | Required answer |
| --- | --- |
| Semantic operation | Domain action/fact, not a private component field |
| Owning implementation | Source symbol for validation, reduction and result publication |
| Scope and grant | Who may call, for which subject/instance, and who issued that grant |
| Time | Public phase, input cut, earliest consume point and result availability |
| Read model | Fields, units, coordinate frame, knowledge scope and absence behavior |
| Ordering | Selection/reduction rule and tie-break, including which owner defines it |
| Bounds | Input/output/allocation limits, overflow and pagination behavior |
| Lifetime | Cancellation, owner retirement, result retention and handle invalidation |
| Replay | Which inputs/queues/results/cursors restore; what is derived |
| Evidence | Behavior test plus deliberate faulty implementation that makes it fail |

Do not ship a row with `TBD` as an active callable offer. A missing port is
unavailable, not a no-op. Optional integration is explicit at module admission;
required mechanics cannot disappear behind `Option` or an empty resource.

## The first source routes

These are source locators, not claims that a portable port already exists.

| Customer | Start from existing owners | Required preservation |
| --- | --- | --- |
| Move-invoked technique | `crates/ambition_combat/src/technique.rs`; `crates/ambition_combat/src/strike.rs`; runtime `combat_schedule.rs` | Installed delivery mode, invoking move occurrence, target and contact credit |
| Body-control contribution | A4 and `accepted-control-writer-map.md`; shared `schedule.rs` | Accepted-control owner, body/proper-time units and one execution path |
| Actor creation | `crates/ambition_platformer2d_actor_spawn/src/actor_spawn/mod.rs`, `SpawnActorRequest`; typed construction | Existing identity allocation, admission, scoped construction and actual spawn result |
| Item acquisition/transfer | `item-custody-and-accounting.md`, A7 and the item writer inventory | One occurrence/custody/accounting transaction; no second bag of inventory |
| World observation | `world-facts-observations-and-memory.md`; spatial query owner | Instance/coordinate scope, settled facts versus observer knowledge |
| External effects | Runtime `external_effects.rs` | Replace speculative frame output on replay; release only under matching confirmation |

Read the actual consumer before freezing a port's schedule. Do not infer ordering
from message names. Existing `CombatSet`, `PlayerInputSet`, effect execution and
construction milestones are source vocabulary, not a universal portable schedule.

## Stable invocation identity and batch scope

Each entry is invoked for a declared scope: one technique occurrence, an actor
set, an encounter, or a world service. A phase entry may process a bounded batch;
there is no default requirement for one VM crossing or reset per entity.

An invocation key contains its mechanical generation, session/instance scope,
logical tick and phase, entry key and owner/occurrence identity. Within it, requests
use an explicit sequence. Persist any sequence that spans invocations in registered
state. Do not derive keys from thread order, ECS allocation order or wall time.

Identity is not combat priority. The host uses keys to route, diagnose and suppress
re-delivery of the same accepted operation. It does not sort all gameplay by module
name and thereby replace the domain's resolution rule. Two identical requests
with distinct occurrence keys are two requests unless the domain contract says
otherwise. Resimulating the same occurrence replaces speculative output rather
than accumulating another copy.

Generation identifies executable context; it does not make a canceled effect
legitimate. A stale owner/instance reference is checked when consumed, not only
when a handle was issued. Trusted native code is audited, not sandboxed by this
handle discipline.

## Read cut, staged writes and domain outcomes

The host gives an invocation a declared immutable read cut. The module stages
changes only to records it owns and emits typed domain requests. Reads of its own
staged records have defined read-your-writes behavior; foreign facts remain the
selected cut. Do not expose half-applied changes from another callback.

```text
read cut + registered own state
    -> invoke
    -> validate write set and requests
    -> commit own records and enqueue valid requests
    -> owning domain arbitrates/applies
    -> publish accepted/rejected result
    -> next eligible invocation consumes result
```

There are two different successes: **submitted** means the extension output passed
the adapter checks; **applied** means the domain accepted and performed the action.
A valid request may lose to another request, an expired target or a gameplay rule.
Its result names the original occurrence. A module does not decrement success-based
counters merely because it submitted an action.

Entry failure discards its staged records and requests. That is a local write-set
transaction. It does not roll back domains already executed earlier in the tick.
A deterministic execution fault stops the speculative session under its existing
fault policy; no peer silently disables the faulty module and continues. A native
process crash remains an engine/process failure.

For the initial binding, validate bounded write sets without copying the entire
store for each invocation. A reference implementation can use owned values for
changed records. Later storage optimization must preserve this semantic cut.

### Example: a paid attack

A module asks to spend a resource and launch an attack. Two independent requests
can produce a paid-but-refused attack or a free attack. Choose the correct domain
transaction: the owning action/admission service validates the cost and launch
together, then reports one result. If reservation is needed, its token, expiry and
cancellation are owned and rewound by that service. Do not build a distributed
transaction coordinator in the generic extension host.

### Example: spawn then attach state

A module requests a body and records `SpawnPending(request_key)`. The constructor
returns a result at its declared barrier with the semantic body identity. Only
then may the module transition to `Spawned(identity)` and bind live attached state.
A reserved ID is not proof that an entity exists. On refusal/cancellation, remove
or transition the pending record through the same module state rule. Rewind must
restore the pending request, allocator/counter and result-consumption cursor.

### Example: contact after a reflected projectile

The module preserves the launch occurrence and the contact owner's verdict credit.
It cannot substitute the attack currently playing when a later contact arrives.
Transport envelopes carry the existing provenance; they do not independently
judge whether the contact satisfies `Connected`. A2/A12 own that rule.

## Same-tick versus future-tick behavior

Public phases are guarantees: accepted input available, body facts published,
requests consumed, contact verdicts settled. They are not one phase per private
system. For each port state whether a request can be consumed in the current tick.
If its owner's consume barrier has passed, enqueue for the next declared point or
refuse with a typed timing result. Never silently insert a frame of latency.

Start with serial deterministic entry order. An entry dependency may name another
public entry/phase; admission rejects cycles and unavailable requirements. Within
a shared read cut, stable execution order must not create an undocumented priority.
State or requests whose order matters use the owner's explicit rule. Parallelism
may later exploit disjoint reads/writes, with the same submitted and applied traces.
Sorting only final damage does not prove equivalence.

Queries declare state-first selection and semantic tie-breaks. Canonical serialization
order is a different concern. Pagination pins generation, read cut, sort and bounds;
a stale cursor refuses rather than continuing over a changed population. A quota
must define truncation, continuation or refusal. Returning whichever entities fit
in an allocator-dependent buffer is not a query policy.

## Grants and observation do not imply a new security product

At admission the composition grants the module installed operations and scope.
An actor controller receives contributions for its authorized subject; changing
possession changes that grant through the existing control owner. A game-rule
module can receive broader world reads where its role requires them. A brain
using limited knowledge receives the corresponding observation projection.

These grants enforce architectural scope in checked bindings. They do not claim
to contain unsafe native code, authenticate downloaded modules or implement a
public mod sandbox. Those are distinct deployment/product requirements.

## End-of-life behavior is part of every port

Death, interruption, possession change, room unload, session retirement and
replacement can invalidate an owner. The owning domain defines which submitted
operations survive. Cleanup must run even if the producing entry is never invoked
again. Music, camera and presentation claims release by owner identity; clearing
an entire tier would remove other valid owners.

Cross-tick inboxes, retries, acknowledgement latches and continuations are registered
state. Pure derived indexes are rebuilt after restore. A request queue is not
persistent merely because Bevy stores it as a Resource. A message read is not
rollback registration. Tests must rewind between submission and result, and retire
an owner between them.

## Acceptance

Use FI5-FI8 in [acceptance](fast-iteration-acceptance.md). A port is complete only
when a real consumer reaches its owner, its absence refuses, its result controls
later behavior, and replay preserves both state and outcomes. Keep one source
route per operation; remove migrated direct writers in the same delivery slice.
