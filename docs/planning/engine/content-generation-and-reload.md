# Content generations, incremental preparation and development reload

**State:** selected design; not an implemented reload service.
[Extension model](extension-model.md) owns the architecture. This page owns the
artifact-to-activation protocol. [Construction](construction-and-reconstitution.md)
owns materialization. [Packets](fast-iteration-implementation.md) select the
implementation steps; [the queue](../queue.md) selects work.

## The contract

An ordinary content edit must change the running result without rebuilding the
host. A failed edit must not silently select built-in defaults, publish half a
revision, consume a gameplay reset, or erase the developer's reproducible scene.
The system reports the exact artifact and behavior it is using.

These are DO decisions. Compression, cache layout, preparation worker count and
snapshot layout are MEASURE choices. No latency value is established here.

## Representations and owners

| Value | Owner and contents | Excluded responsibility |
| --- | --- | --- |
| Authored source | Existing Rust/data/media authoring tool; editable values and diagnostic spans | Installed runtime support or live ECS handles |
| Portable section | Pure domain schema; canonical values, references, codec version | Domain callbacks, `Any`, Bevy entities, source-loader policy |
| Artifact manifest | Content pack; immutable section digests, module inputs and requirements | A duplicate character catalog or service locator |
| Prepared domain value | Owning validator/hydrator; resolved immutable definition | Current actor health, inventory or mutable playback |
| Candidate generation | Lifecycle/content coordinator; complete selected domain view and executable profile | A partially published set of domain revisions |
| Active selection | Session owner; admitted generation and local activation epoch | Process-global mutable selection |
| Live state | Existing body, combat, item, world and extension owners | A second copy inside a content cache |

The existing `PreparedContentPack::lowered` may remain an in-process hydration
container. Do not make it the wire format. Extract the pure validator used by both
frontends when necessary; do not rewrite its rules independently in the loader.
A compiler cannot certify installed support. The host checks actual offers.

## Dependency graph, not one rebuild switch

A generation consists of independently identified immutable sections. Each domain
adapter declares its inputs and references. The pack provides one graph of these
requirements, not another evaluator for the domain's own rules.

A cache key includes the source/section digest, codec and validator versions,
dependencies used during preparation, and the selected profile facts that affect
the result. Runtime hydration also includes its host contract/build identity.
Caches may reuse immutable results across Apps; activation authority is App-local.

For a move edit, trace the real dependency closure:

```text
move values -> referenced techniques/moves/mechanical assets
            -> affected prepared character/action definitions
            -> selected generation -> dependent live bindings
```

Do not re-prepare unrelated rooms, audio or characters because one scalar changed.
Do not ignore a dependency to achieve that result. A domain with no precise
invalidation description must conservatively re-prepare its own section and
reported dependents, not claim an incremental hit. Instrument why each section
was rebuilt or reused. Add precision where measured invalidation is expensive.

The graph includes deletions and reverse references. Removing a move used by a
character must fail or update that character in the same candidate. A missing
required section cannot fall back to a compiled table. A field with uncertain
mechanical influence is mechanical until its owner establishes otherwise.

**Example:** changing only diagnostic source spans changes diagnostic provenance,
not mechanical identity. Changing attack duration must change the mechanical
digest, invalidate the dependent preparation and alter the observed trace.
Changing a sprite used for authored collision must do the same. Changing a purely
visual texture need not restart simulation if its owner proves that classification.

## Identity and canonical form

Extend `PreparedContentBuilder`; do not replace it with a parallel hash authority.
Separate these meanings in the implementation:

| Identity | Purpose |
| --- | --- |
| Source revision/provenance | Explain which author input produced a value |
| Portable section digest | Reuse and verify exact canonical domain data |
| Mechanical generation digest | Bind selected definitions, code, schemas, ports and execution policy |
| Host compatibility identity | State the engine/build/target policy required for execution |
| ContentEpoch | Reject stale App-local activation plans |
| Gameplay session / rollback timeline | Own live state and the history being replayed |
| Construction attempt | Own candidate work and cleanup, not durable object identity |

Canonical bytes specify tag order, integer width, sequence order, string encoding,
map order, numeric restrictions and unknown-field behavior. Do not hash Rust
memory or default Debug/Hash output. Keep `-0`, NaN and infinity handling explicit
at the owning numeric contract; never normalize away a mechanically meaningful
value. Required unknown fields/sections refuse admission. An optional field is
ignorable only under a versioned rule that excludes simulation influence.

Source timestamps, local paths, temporary IDs and file enumeration order are not
mechanical identity. Diagnostic metadata may have its own digest. A producer must
not claim identical generations merely because the character IDs are unchanged.
Identical input plus the same selected preparation contract is a no-op, not a new
epoch or timeline. Tests compare semantic section bytes and dependent behavior.

Do not make persistent saves use ContentEpoch or executable addresses. Same-build
network compatibility, save-schema compatibility and authoring-format compatibility
are separate contracts. A code digest establishes identity, not trust or safety.

## Producer, source transport and complete candidates

Use the existing source/asset resolver for local, packaged and web transports.
An authoring tool publishes immutable section/module objects and a complete
manifest last. For a local filesystem this can use write-to-new-path plus atomic
manifest replacement. Other transports need an equivalent complete-object rule.
A watcher only requests inspection. It is not ordered simulation input.

The reader verifies lengths, versions, dependency identities and complete bytes
before building a candidate. Missing or still-writing objects report pending or
invalid; they never activate a mixture. A later notification does not mutate bytes
already captured by an earlier preparation task.

A candidate carries its requested source revision, base ContentEpoch, selected
profile identity and an attempt key. Superseded work may finish, but it cannot
publish. Cancellation retires only that attempt's resources. Shared immutable
cache objects remain valid while any generation holds them. Coalescing requests
may discard intermediate edits, but the UI must state which revision won.

## Activation state machine

Names below specify roles, not mandatory new public Rust type names.

```text
Requested -> ReadComplete -> Prepared -> Admitted -> Sealed
          -> BoundaryGranted -> CandidateReady -> Published -> Retired
```

Preparation and admission can occur without stopping active simulation. Candidate
construction may require a pause; the coordinator explicitly owns that pause.
Failure before publication has one terminal result. It does not retry forever
unless a new source revision or explicit retry requests another attempt.

| Transition | Required input and invariant |
| --- | --- |
| ReadComplete | Immutable complete manifest and verified dependencies |
| Prepared | Domain validators produce values; active registries and live state are unchanged |
| Admitted | Actual installed ports, capabilities, schemas and resource limits satisfy requirements |
| Sealed | Complete replacement view plus base epoch/profile and reconstruction policy |
| BoundaryGranted | Local lifecycle owner authorizes the operation; stale or remote-active candidates refuse |
| CandidateReady | Supported materialization and state mapping are verified without publishing partial state |
| Published | One active selection changes; all participating owners and new timeline name it |
| Retired | Old calls, readers, snapshots, effects, tasks and code-owned destructors are no longer reachable |

Publication is a simulation visibility barrier, not necessarily one machine-word
store. No simulation, event reader, observer or presentation extractor may observe
half the committed generation. A Bevy deferred-command flush is not proof of this.
The owner must specify the systems and hooks covered by its barrier.

Snapshots may only be replayed with their bound mechanical generation. A fresh
local timeline does not clear an unhealthy diagnosis for the same gameplay session.
Remote sessions retain their admitted generation; next-session packaging is not
online hot migration.

## Reload policies with explicit limits

Every migrated family declares one supported action. Tooling reports it before
activation; it does not choose reset versus retain from a heuristic.

| Class | Allowed action and proof |
| --- | --- |
| Diagnostic-only | Refresh diagnostics; mechanical generation and timeline do not change |
| Presentation-only | Use existing asset reload; prove no simulation reader depends on the changed bytes |
| Immutable mechanical definitions | Replace at a local barrier only after every affected live binding has a declared retain/rebind/retire policy |
| State/schema or body-construction change | Reconstruct the selected development scenario through the accepted lifecycle and domain restore paths |
| Fundamental host/port change | Rebuild the engine/profile; report this as engine iteration |
| Unsupported migration | Reject or require an explicit scenario restart; never silently discard durable progress |

The first supported mechanical path is **repeatable scenario reconstruction**.
It is not an arbitrary save migration or seamless mid-attack swap. Capture the
scenario seed, input trace, start point and selected checkpoint before requesting
activation. A checkpoint usable with generation N is not automatically usable
with N+1. Each affected owner validates the mapping, including removed definitions,
state schema changes and semantic references.

An unchanged schema does not prove unchanged meaning. Initially reject a changed
schema's state transfer unless a named migration exists. A developer may explicitly
restart that scenario from authored initial state. Persistent game saves remain
untouched by this local operation.

An owner can later add the narrower immutable-definition fast path. It must state
what happens to active MovePlayback references, cooldowns, attached capabilities,
prepared collision facts and derived caches. Either finish/retire the affected
occurrences before publication, or include the retained definitions explicitly in
the admitted generation. Do not keep an unnamed old Arc while advertising a single
new mechanical identity. A globally idle arena is not a required long-term boundary.

## Bounded safe construction, not arbitrary World undo

The existing raw-Commands recipe context does not meet a preserve-old-scene
contract across arbitrary commit failure. A10 now has a concrete customer: I3's
repeated development reconstruction. I1/I2 do not wait for that work. I3 may ship
an explicitly labeled fail-stop prototype, but cannot close the reliable reload
acceptance on that prototype.

For the migrated reload path, prepare a **domain-owned candidate draft** with
resolved definitions, component values, relationship targets, resource deltas and
retention decisions. No callback during this phase receives mutation access to
active World or process state. Do not invent a universal construction opcode enum
or deep-clone an arbitrary Bevy World.

The default design is typed inactive data produced by the existing domain
constructors' preparation halves. The restricted materializer consumes it after
validation. Separate recoverable preparation errors from internal commit defects.
Hooks and observers reached by installation must be inventoried: they cannot
perform fallible IO, publish effects, discover new requirements, or mutate unrelated
live owners inside this transaction. Unsupported engine plugins keep the explicit
fail-stop contract until they implement this bounded path.

A same-World candidate population is acceptable only with a stronger isolation
proof: all relevant queries, observers, hooks, message writers and resource writes
exclude or isolate it. A `Pending` marker alone proves none of those properties.
Do not refactor every Bevy query merely to justify that shortcut. A separate World
is not a drop-in fix either: entity mapping, resources and hooks need explicit
transfer contracts. Select physical staging after the typed boundary is clear.

Before retirement, the candidate must pass domain verification, relationship
resolution and checkpoint policy. The publication section performs only the
specified materialization/selection and timeline establishment. A supported
validation/materialization refusal retains the old selection and scene. An
unexpected native panic, allocator failure or rogue unsafe plugin is an engine
fault, not a recoverable content refusal and not a sandbox guarantee.

When the implementation can only recover by rebuilding the pinned old scenario,
report **recovered**, not **unchanged**. If neither retention nor recovery is
available, report **failed/stopped** and leave normal simulation disabled. Do not
call that successful hot reload. These result distinctions are part of I3 tests.

## Required tool operations

Use existing CLI/workbench routing. Add operations that expose actual owners:
validate candidate; explain affected sections; request activation; inspect status;
inspect active generation; replay the selected scenario. Names are chosen in the
owning tooling packet, not by creating a second general launcher.

A status result includes attempt, source revision, base/active/candidate identity,
policy, state-machine stage, refusal owner/reason, rebuilt/reused sections and
whether the prior scene was unchanged, recovered, or stopped. Successful status
also names the behavior witness that observed the new generation. A file read or
successful build is not playable readiness.

## Implementation and evidence

I2 implements portable sections and complete-object publication. I3a implements
the candidate coordinator and seals. I3b implements one bounded reconstruction
path with A10. I3c integrates repeatable development scenarios and stale-work
handling. This page does not authorize a generic live-state migration framework.

Use fixtures FI1-FI4 in [acceptance](fast-iteration-acceptance.md). Measure source
read, changed-section preparation, candidate construction, activation, scenario
reset and first observed behavior separately under M0. Safe publication and exact
identity are not optional experiments; their physical implementation can change.
