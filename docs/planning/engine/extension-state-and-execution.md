# Procedural extension execution, state and generations

**State:** target protocol; not an implemented API. This page refines the
[extension model](extension-model.md). [Packets](fast-iteration-implementation.md)
name the source edits and tests. Existing domain protocols retain authority over
combat, bodies, items, construction and persistence. Names in schematic examples
below describe proposed protocol fields, not shipped Rust items.

## Contract boundary

The procedural SDK offers immutable definition handles, typed observation
batches, extension-owned state access and typed request builders. It does not
export a reflective view of arbitrary engine components. A native Bevy plugin
can use raw ECS APIs in the separate engine tier, with the normal ownership and
rollback obligations. Native trust is not a security sandbox.

The host adapter owns marshaling, admission, ordering and participation. Each
installed domain owns its read projections, request decoder and reducer. A
module cannot claim a domain port merely by putting its name in a manifest.
Installation provides an actual executable adapter and compatible contract.

A module descriptor contains:

| Field | Meaning and admission rule |
| --- | --- |
| Provider and module key | Namespaced stable identity; duplicates reject unless an explicit replacement policy applies before admission |
| Semantic API version | Exact supported major and declared compatible minor range; do not infer compatibility from Rust crate semver |
| Code identity | Digest of exact executable input plus binding/runtime determinism profile; native identity also binds the selected target/build |
| Entry points | Stable entry keys, public semantic phase, declared reads, state writes, request ports and ordering requirements |
| State declarations | Complete namespaced schema IDs, revisions, canonical shape digests, scope, limits and lifetime policies |
| Requirements | Installed port versions, selected capability support and cross-section references |
| Execution profile | Numeric rules, deterministic work and allocation limits, imported services, target support |
| Diagnostics | Source spans/maps and human-readable field names; not substitutes for identity |

Duplicate keys reject. Do not compare function pointers to certify equivalent
providers. Admission is App-local and seals the selected installed profile.
A candidate prepared against profile A cannot activate after the composition has
changed to profile B without re-admission.

## State schemas and storage

A schema defines tagged fields with fixed-width scalars, finite numeric values,
records, enums, bounded sequences, bounded maps and semantic references. Give
sequences/maps explicit element and byte limits. Never put usize, pointer values,
Rust layout, a process hash seed or a Bevy Entity bit pattern on the wire.
Bounded does not mean an arbitrary low gameplay limit: the module declares its
capacity, the host admits an aggregate resource budget, and overflow has a typed
deterministic result. Records and arrays can express graphs through stable keys.

Each schema declaration specifies these independent policies:

| Concern | Required declaration |
| --- | --- |
| Ownership | Which provider writes these records; foreign access requires an explicitly installed port |
| Attachment | Entity-associated state or a resource scoped to a session/world/domain occurrence |
| Rewind | All future-affecting state participates; there is no authoritative-but-unregistered option |
| Lifetime | Spawn/despawn, room residency, session retirement, encounter end and permanent-world ownership |
| Save eligibility | Transient, checkpoint, or durable-world policy with the persistence owner's adapter |
| References | Live semantic entity reference, durable object identity, definition reference, or local record key; null/missing behavior is explicit |
| Canonical encoding | Ordered tags, widths, lengths, numeric rules and key ordering, plus a shape digest |

A module's schema ID is provider plus local key, not the last segment of a Rust
type name. Two modules may both define a local `State`. Revision changes cannot
reuse an old shape digest. Reject contradictory duplicate declarations. Initial
admission uses exact schema matching; forward compatibility needs an explicit
codec rule. Do not silently default an unknown authoritative field to zero.

### Initial physical implementation

Start with one safe host-owned store type, registered through the existing
RollbackRegistrar resource/component mechanisms. Internally it can hold
schema-indexed bounded records keyed by scope and semantic attachment. The first
correctness implementation may use ordered maps and owned values. The store is
only extension-owned state; it must not mirror engine health, bodies or items.
Its schema registry is immutable generation metadata, not copied per snapshot.

Use the existing rollback_resource_clone_checksum method for the first store:
clone owned mutable records and share only immutable schema metadata. Supply a
canonical logical-value checksum, not a presence-only probe. A shallow Arc clone
of mutable records is not a snapshot. This method exists in the current registrar
and avoids inventing a decoder with hidden access to the live World.

In particular, the current SnapshotState decode function receives a Reader, not
a schema registry or World. Do not implement a schema-dependent decoder by
consulting process globals. Save/artifact decoding can take an explicit frozen
schema context. A future canonical snapshot storage implementation must either
be self-describing within its bounded wire format or add a deliberate backend
context contract, with the same generation checks. It is not required for the
first correct clone-snapshot implementation.

The host supplies one canonical codec and checksum traversal for logical records.
An extension author supplies a schema and values, not a per-mechanic SnapshotState
implementation. Generated native typed accessors and script bindings use this
same logical layout. This design does not require a dynamic Bevy component for
every schema in the first implementation.

The current registrar uses concrete generic types. The current rollback registry
lists metadata; it is not a dynamic serializer. Therefore the host must register
the concrete store, enroll its live population, and test real GGRS restoration.
Adding a descriptor row alone does none of those things.

Bevy 0.19.1 does support dynamically described components. That makes native ECS
column storage a later option, not an impossibility and not an automatic rollback
solution. Its layout/drop/thread-safety requirements belong to the adapter.
Keep that unsafe work out of the default SDK. Compare it with safe typed chunks
under M2 before changing storage. [Primary references](extension-iteration-evidence.md)
document the relevant upstream contracts.

### Reference and population rules

Use the existing SimId semantics and per-spawner counters. References also carry
the owning session/spatial scope where needed to distinguish simultaneous worlds.
Do not replace semantic IDs with an unrelated global integer allocator.
Ephemeral host handles may use generation-checked slots for efficient calls;
those slots are not authoritative identity and cannot be saved or compared by peers.

Restore entity existence and its mapping before resolving live references.
A rollback resurrected body must reconnect to its records. A record that survived
room residency may instead contain a durable identity for an unloaded object;
it must not force that object into the live ECS or become a dangling Entity.
Rebuild lookup indexes after restore. Missing references obey the schema's stated
required/optional policy and produce diagnostics, not whichever object reused a slot.

Spawn requests use the existing domain constructor and identity allocator.
An accepted spawn may reserve a deterministic semantic identity and return an
acknowledgment at a declared boundary. An extension cannot query a not-yet-created
body as if deferred Commands had already run. Allocation counters, pending
requests that span ticks, cursors, cooldowns and event-consumption latches are
all authoritative state if they influence a later tick.

Resource records require the same population, scope and retirement tests as
components. Avoid a process-global store leaking state across Apps or sessions.
If a backend snapshots one composite resource, partition records by lifetime and
account for the cost; do not assume that splitting an internal map automatically
makes snapshots incremental.

## Invocation protocol and scheduling

The conceptual call is:

```text
invoke(entry, generation, tick_context, read_batches, own_state)
    -> staged_own_state_changes + typed_requests + diagnostics
```

This is a semantic contract, not a requirement to serialize every argument for
a static Rust call. Native adapters may borrow safe typed views. Portable calls
use bounded linear buffers or handles with defined lifetimes. No borrowed view
survives the invocation, state commit or world mutation barrier.

Entry points declare data access and public phase requirements. Public phases
state facts, not the names of private functions. For the first combat customer,
map them to the already installed ordering in
`crates/ambition_platformer2d_runtime/src/combat_schedule.rs`:

| Semantic point | Input/output contract |
| --- | --- |
| Control intent contribution | Read the previous settled facts and current admitted input; submit to the existing accepted-control owner before its reduction |
| Technique execution | Run when the owning move occurrence invokes its installed technique; preserve issuer, launch occurrence, target and contact provenance |
| Pre-resolution domain requests | Emit to a specific owner before that owner's existing consume barrier; do not retroactively affect an earlier phase |
| Settled result observation | Read accepted contact/reaction facts after the owner publishes them; follow-up actions run only at the next declared eligible phase |
| Confirmed host effects | Handle irreversible work outside speculative simulation, authorized by the existing session confirmation service |

Do not create every phase eagerly. I4 installs only the phases required by its
concrete fixture and documents their exact mappings, flush barriers and run
conditions. A new body-controller entry must use A4's published contribution
contract, not insert a second velocity writer.

Resolve entry ordering at admission. Explicit before/after dependencies name
public phase/entry keys, not internal SystemSets. Reject cycles and unavailable
required phases. Begin with serial invocation in an admitted stable order. Parallel
execution is an optimization only when declared accesses and deterministic buffer
merging establish identical results; it is not achieved by running opaque callbacks
concurrently and sorting their damage afterward.

### Queries

Domain ports expose deliberate immutable projections: body pose/velocity and
status facts, settled contact results, spatial query results, item/progression
observations and installed definition metadata, as each owner supports them.
They do not expose every private field by name.

Every query specifies scope, simulation phase, ordering, numeric conventions,
absence behavior and bounds. Reuse the owner's state-first selection and stable
identity tie-breaks where behavior depends on order. Canonical snapshot ordering
by semantic ID is a different job. Never replace closest-target ordering with
ID ordering merely because both are deterministic.

Prefer batched projections and batched requests over one FFI call per component.
Start with simple bounded selection and known-owner query ports, not a universal
query-language compiler. A spatial index remains owned by the engine; an
extension's graph may use its results without maintaining a second physics world.
If pagination is needed, its cursor pins the observation phase/generation and
explicit sort order; it cannot continue over a changed World silently.

### Writes, failures and domain acceptance

Invocation writes are staged until the host validates schema bounds, ownership,
phase eligibility and output budgets. A trap or rejected invocation exposes none
of that invocation's writes or requests. Scratch state is discarded. This is a
bounded extension-buffer transaction, not transactional undo of arbitrary Bevy
commands.

Domain requests then pass through their owning reducer. Acceptance, rejection,
contact results and spawn acknowledgment have typed occurrence-linked receipts.
An extension cannot assume a request succeeded merely because it emitted it.
State depending on success changes upon the owner's acknowledgment. A mechanic
that requires atomic mana spending plus another effect needs an explicit
owner-approved compound/reservation protocol; two independent requests are not
silently promised to commit together.

Move-origin requests preserve the existing launch occurrence and verdict credit
rules. No fallback to whichever move is playing now. Observers consume occurrence
IDs/latches, not one bool that can accidentally count a prior event again.

A deterministic work-limit trap faults the speculative session consistently;
do not disable the module on just one peer and continue. A local development host
may then reconstruct under an explicitly chosen previous/next generation. Native
process crashes or allocation failures are not recoverable gameplay results.
Do not claim catch-unwind makes arbitrary native code safe or guarantees recovery.

## Deterministic services and hidden state

Expose logical simulation tick and explicitly named domain/proper-time values.
No wall clock, system entropy, filesystem, network, ambient locale or mutable
process singleton is a simulation service. Model or external service results
enter through the existing admitted input road, never a blocking call in a tick.

RNG either uses an explicit state field registered for rewind, or a documented
counter-based service keyed by stable module/occurrence identity and invocation
counter. A counter is authoritative if its value persists. Neither approach may
change because threads happen to schedule in another order. Define integer
wrapping/overflow, float admission and transcendental behavior per execution
profile. Match existing domain numeric semantics; this project is not permission
for an unmeasured engine-wide fixed-point rewrite.

For normal host-managed state, VM memory is scratch. Reset or reconstruct *all*
mutable instance state before the next independent invocation, including globals,
mutable tables, closure environments and imported state. An instance pool must
restore a validated initial image, not merely rewind an allocation pointer.
Persist continuations as explicit schema state, not an invisible coroutine stack.
Cache only derivable immutable results, keyed by generation and all inputs; tests
must succeed with caches cleared on every call.

For trusted native code this is an audited API contract. Rust unsafe code and
statics cannot be sandboxed by a type alias. The portable runtime must enforce
its import and reset rules. Stateless execution plus host-managed state is the
initial default; do not advertise arbitrary persistent VM heaps as supported.

For a WASM prototype, pin deterministic imports, NaN behavior, SIMD policy,
memory/table growth policy and fuel budget. Exclude thread/shared-memory and
nondeterministic imports initially. Fixed admitted capacities avoid per-tick
host-dependent growth results. Use deterministic work limits, not wall-clock
interrupts that become gameplay outcomes. These choices are part of executable
identity. M1 measures their overhead. WASM alone does not establish deterministic
host calls or equivalent floating-point results across different native backends.

## Snapshot efficiency and alternatives

All designs must meet the same semantic state and generation contract.

| Strategy | Strength | Required proof / cost risk | Decision |
| --- | --- | --- | --- |
| Typed native registered state | Fits current domain registration and typed queries | Concrete types still compile into host; custom codecs duplicate extension work | Preserve for engine plugins; comparison baseline |
| Schema-backed host store | Generic codecs, inspection and binding independence | Maps/copies may be expensive; resource granularity may over-copy | Default logical model; safe first implementation |
| Full VM image | Can preserve richer guest state | Must include memory, globals, tables, imports and continuations; opaque state weakens diagnostics | Not default; separate complete-state proof before support |
| Linear-memory copies | Simple for a strictly restricted guest | Linear memory is not the whole instance; cost scales with copied capacity | M2 comparison under a restricted profile only |
| Dirty-page / chunk COW / deltas | Can reduce copying when writes are sparse | Dirty tracking, retained history, restore/checksum costs and target support; all writes must be tracked | Optimize measured bottlenecks without changing API |
| Deterministic reconstruction | Small stored state for caches or regenerated structures | Resimulation cost and full input/history availability | Use for derived caches; not a blanket omission of authoritative state |
| Hybrid host state plus guest image | Supports select state-heavy algorithms | Two state representations need one snapshot cut and one identity | Only when a concrete customer defeats the default with measured evidence |

A delta chain needs bounded reconstruction depth, a clear base snapshot and
lifetime management across GGRS history eviction. A COW snapshot must detach on
every authoritative mutation. Hash canonical logical values, not shared pointers
or page addresses. An incremental checksum needs a dirty-write barrier witness;
otherwise keep the slower correct full hash. Do not implement another rewind
scheduler to make the store fast.

## Side effects, saves and world scale

Speculative sound/particles may use the existing presentation intent road with
occurrence identity and replay reconciliation. Identity includes session,
logical occurrence, producer and within-occurrence sequence, not raw Entity.
Rewind must withdraw/deduplicate speculative effects as their owner requires.
Save writes, achievements, external network messages and irreversible purchases
wait for session-confirmed authorization. A module never sends them directly.

Durable state is not a synonym for rollback state. A graph attached to a live
boss can be transient. Quest state may survive room unload and save/load. Declare
that policy; do not serialize every temporary cooldown into a forever save file.
Reuse the checkpoint/durable ownership road for approved records, with canonical
schemas as codecs where appropriate. Missing persistence support rejects a
module's required durable capability; do not silently degrade it to session-only.

At scale, dormant world data lives with its existing durable/world owner.
Simulation-affecting reads use the admitted active snapshot or confirmed events.
Background IO cannot mutate the current speculative truth. Promote/demote state
at existing confirmed residency/lifecycle boundaries. Measure an active region
plus dormant records, not only a two-body arena. Do not claim persistent schemas
alone implement a streaming open world or cross-room simulation.

## Identity, activation and retirement sequence

Extend, do not replace, `PreparedContentBuilder` and `PreparedContentIdentity` in
`crates/ambition_platformer2d_runtime/src/content_identity.rs`. Add canonical
sections for portable artifacts, installed port contract versions, module code,
runtime execution profile and extension schema digests. Bind the host's engine
protocol/build identity in the session compatibility manifest as well. Hashing a
module is not a digital signature or a trust decision.

Keep three distinct identities: mechanical content digest shared by peers;
App-local ContentEpoch for stale-plan rejection; rollback timeline generation
for history. The character catalog generation can remain a domain-local revision
but must be captured in the selected bundle, never used as the peer identity.
The content-pack u64 fingerprint is not the exact module/session digest.

Activation algorithm:

1. Resolve immutable candidate bytes and their dependency manifest. Freeze the
   selected profile and source revisions for this attempt.
2. Decode and validate every changed section and its affected references. Load
   candidate code into an inactive executor; inspect required imports and schemas.
3. Hydrate domain candidates without mutating active registries or live state.
   Reuse or factor the current character staging validation so candidate building
   does not call a mutating publication function. Preserve initial cast admission's
   withholding policy separately from all-or-nothing revision replacement. Validate the complete bundle,
   not a sequence of independently published family updates.
4. Seal the candidate with its base epoch, actual installed profile identity,
   content digest and required reconstruction/migration policy. A stale base
   epoch or changed installer set invalidates the seal.
5. Request the supported lifecycle boundary. A remote active session rejects
   replacement. A local session waits for the relevant confirmed boundary and
   cancels outstanding plans that name the retired generation.
6. Publish through the existing lifecycle coordinator. Install new definitions,
   schemas and baseline state as one selected generation; initialize the GGRS
   timeline against that generation before stepping it. Keep the existing
   failure semantics if world construction cannot be undone.
7. Retire old executable objects only after active calls, borrowed views, tasks,
   callbacks, runtime-owned destructors and retained references are gone. Clearing
   rollback history is necessary for this policy, but not sufficient for safe
   native library unloading.

No mixed-generation tick is permitted. Snapshots carry or inherit a checked
immutable generation binding. Schema and code changes both invalidate old
history under the initial policy. A future system retaining old executables for
replay must retain all referenced schemas/content/host-contract compatibility,
not just a library handle. That complexity is not an initial reload requirement.

## ABI boundary and advanced native access

The in-process Rust reference binding may use ordinary Rust types. The shared
library experiment must not. Give it a versioned C entry table, fixed-width
fields, explicit status codes, caller-owned byte buffers and opaque checked
handles. Define alignment, lengths, ownership, thread use and call lifetime.
Keep allocation/deallocation on the owning side. Do not pass Rust Vec, String,
trait objects, World or allocator-owned containers across the ABI. Contain
unwinding inside each side; native abort/crash still terminates that process.

Keep loaded generations pinned while their code or destructors are reachable.
Reloaded builds use distinct immutable paths so a loader never edits the mapped
file. An out-of-process native worker is a possible isolation tradeoff, not the
default low-latency simulation call path. Measure its IPC separately if required.

A small native plugin that genuinely needs bevy_ecs may use the static adapter,
with domain-owned components and the existing registrar. It is paired to the
engine's Rust/Bevy versions and may require host relinking. Report that honestly.
Raw Bevy access is not extended through the portable ABI merely because a
fixture found a missing read port.
