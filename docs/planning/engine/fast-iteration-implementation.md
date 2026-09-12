# Fast iteration implementation packets

**State:** target packet catalog, not a completed feature or a second queue.
[The queue](../queue.md) selects execution. The [extension model](extension-model.md)
owns decisions, the [execution contract](extension-state-and-execution.md) owns
state semantics. [Generation/reload](content-generation-and-reload.md) and
[domain contracts](extension-domain-contracts.md) own their protocol details.
[Acceptance](fast-iteration-acceptance.md) defines FI1-FI10, and
[evidence](extension-iteration-evidence.md) owns M0-M3.
Baseline: `d81a7ae1d2db1fc5caa49efc807a39ea6b1ca266`.

## Start here

Read the model plus the protocol and fixture sections for the selected packet
before changing a runtime seam. Do not ingest every long plan by default. Re-read the
named source on the actual working head. Record the active writer, installer,
caller, lifetime, generation and cheapest existing behavioral test for that seam.
Do not treat an old paragraph about unfinished A11 validation as current source.
Do not regenerate the entire repository index or run the entire workspace suite
just to begin this work.

The paths explicitly marked **proposed** below do not exist at the baseline.
Create them only in the packet that owns them. Exact package/file names may
change after an import-cycle check; the dependency direction and tests may not.
Update these locators in the same commit when an implementation chooses a name.

## Dependency graph and delivery cuts

```text
I0/M0 baseline and measurement helper ---------------------> report gains
    (parallel; no blanket design gate)

I1 pure move authoring -> I2 loadable data artifact -> I3a candidate coordinator
                                                      -> I3b bounded construction (A10)
                                                      -> I3c repeatable reload
                                                        |
I4 procedural SDK + static semantic reference -> I5 state + real rewind
                       |                         |        |
                       +-------------------------+--------+
                                                 v
                                      I6 backend experiment (M1)
                                                 |
                                      I7 selected procedural path
                                                 |
                                      I9 external/flagship acceptance

I5 -> M2 -> I8 measured runtime/snapshot optimizations -> I9 cost evidence
```

**First useful delivery:** I1-I3 plus M0 evidence for a real move edit. Do not
wait for a scripting language, state-store optimization or whole-engine carve.
**Second useful delivery:** I4-I7 with one migrated boss mechanic and a graph-based
extension. I8 is not a prerequisite for a correct prototype, but measured budget
regressions must be resolved before claiming production readiness.

| Existing program | Exact dependency | Not a dependency |
| --- | --- | --- |
| A6 definitions/materialization | Use its field census to place any moved field | Renaming/extracting all characters or the actor SCC |
| A9 minimal profiles | Reuse resolved-closure guards and independent consumer method | Completing every facade capability profile before pure authoring |
| A11/A12 techniques | Preserve current installed admission, reference validation and occurrence rules when loading moves or providing techniques | Rebuilding the already implemented flow interpreter |
| A2/contact and A4/body | Their published request/observation contracts govern corresponding ports | Completing every combat or movement feature before the data artifact |
| A1/checkpoints | Use its durable restore/confirmed lifecycle road for required saved state | A new persistence engine inside the extension host |
| A8/multiple worlds | Two-instance FI9 is a planned requirement; scope portable records consistently with their actual owners | Full concurrent-world execution before I1-I3 |
| A10/bounded candidate construction | I3b needs one safe migrated reconstruction path; verify candidate materialization before retirement | Arbitrary World cloning/undo, all-plugin migration or a full streaming system |

## I0 - record real iteration costs without blocking clean boundaries

**Input:** current source and a configured developer machine. **Class:** MEASURE.
Read `Cargo.toml`, `.cargo/config.toml`, `AGENTS.md`, the B7 build plan and M0.
Use existing dependency/absence instruments before adding a new helper.

1. Record the exact normal move-edit and procedural-edit commands, features,
   host process, assets, cache state and machine. Preserve a representative input
   trace that makes each selected edit observable.
2. Obtain no-op and paired edit results using M0. Separate host relink from
   unchanged dependencies staying fresh. Include codegen, launch/reconstruction
   and observable readiness, not just cargo check.
3. Extend a current measurement helper if suitable. Otherwise add a narrowly
   scoped iteration recorder with the M0 output schema. It must run an explicitly
   selected command, not choose a broad build/test lane implicitly.
4. Save raw evidence under a generated target report directory. Summarize the
   source revision, commands, sample count, result and limitations in the owning
   plan. Do not copy numeric counts into queue/status/tracks.
5. Identify profile/link/feature/cache changes as separate measured experiments.
   Do not combine all flags into an untraceable speed patch.

**Acceptance:** timing rows correlate edited input, actual admitted generation,
observed behavior and build/link events. A missing tool is recorded as unavailable.
**Poison:** skip loading the new artifact or feed an old host acknowledgment;
the helper rejects the run instead of reporting a fast success. Make its metadata
command fail; the closure report must fail rather than report zero dependencies.
**Cheapest check:** helper unit tests using fake command/result fixtures, then the
selected real loops. No workspace test suite. **Stop:** lack of a representative
machine leaves M0 open; it does not hold I1's pure ownership extraction.

## I1 - pure Rust authoring without the facade

**Class:** DO. **Input:** no new runtime required.
Read `crates/ambition_entity_catalog/src/lib.rs`,
`crates/ambition_entity_catalog/src/authoring.rs` (it was
`crates/ambition_characters/src/moveset_authoring.rs` when this packet was written; I1 moved it 2026-09-11), <!-- cite-ok: the path it moved from -->
`crates/ambition_characters/src/moveset_prefabs.rs`, and imports in
`game/ambition_demo_smash/src/moveset.rs`. Follow all helper callers with rg and
inspect the corresponding import bodies. A6 supplies the field responsibility map.

1. Inventory each helper's input/output types, constants and dependencies. Move
   only functions which construct pure MoveSpec values and the pure constants
   they need into the existing entity-catalog authoring module. Do not move live
   character/body preparation, repertoire policy or prefab selection wholesale.
2. Update all helper callers, including other demo providers. Keep one helper
   implementation. Delete the old internal helper road/reexports rather than
   preserving a bridge for a pre-release API.
3. Create an out-of-workspace builder fixture that authors a real multi-window
   move with a technique reference. Its dependencies are the pure value/pipeline
   crates only. The fixture may share an authored input file, not import the
   broad game provider as a library.
4. Add a feature-resolved closure witness through the existing absence-contract
   machinery. Examine normal, build and dev dependencies for the actual build
   and test invocations separately. A proc-macro/build-script edge can defeat a
   claim even when normal imports look clean.
5. Move/grow pure helper tests alongside the owner. Preserve exact emitted move
   values through golden or direct structural comparison. Changes to game feel
   are not bundled with this ownership move.

✅ **STEP 1 COMPLETED 2026-09-11 in two passes, and the first pass was too
narrow.** `moveset_authoring.rs` moved first; the vocabulary shipped tables are
actually built from did not. The second pass moved the twenty `smash_*` modules
(captures, repertoires, counters, tethers, portals, 4,767 lines) after measuring
that every one of them names `bevy` only inside comments, and split
`SmashHoldState` — a rollback-registered `Component`, the single derive pinning
the family — into `ambition_characters::smash_hold_state`. Receipt, numbers and
the poison in [queue.md](../queue.md).

**Proposed output locations:**
`crates/ambition_entity_catalog/src/authoring.rs`;
<!-- cite-ok: proposed file to be created by I1, not source evidence. -->
`fixtures/content_builder/Cargo.toml` and its builder source.
<!-- cite-ok: proposed independent fixture, not a baseline file. -->

**Acceptance:** the external fixture builds and tests without Bevy, runtime,
render, audio, monolith or named game providers in its resolved closure. Move
semantics remain identical. This is an authoring improvement, not yet the claim
that the host never relinks for data edits.
**Poison:** make the fixture import the facade or restore the helper's runtime
edge; the resolved-closure witness fails and names the path. Mutate one emitted
move field; the parity test fails rather than normalizing it away.
**Cheapest existing checks:** `cargo test -p ambition_entity_catalog --lib`,
then the affected character/helper tests. Run the new independent fixture's
unit tests after it exists. Run the relevant demo acceptance once for the moved
consumer, not after every helper edit. **Do not expand:** into a character-domain
rename, global serde redesign or compile-profile tuning.

## I2 - a loadable move artifact through existing preparation

**Class:** DO. **Requires:** I1 for the lightweight Rust frontend; the data
format/host side can be developed in parallel.
Read `crates/ambition_content_pack/src/lib.rs`,
`crates/ambition_content_pack/src/prepared.rs`,
`game/ambition_content/src/pack.rs`,
`crates/ambition_characters/src/prepared.rs`, and
`crates/ambition_combat/src/technique.rs`.

1. Define the envelope and one domain-owned move section under the
   generation/reload contract. Record section dependencies, deletion semantics,
   cache inputs and diagnostic-versus-mechanical provenance. Version envelope and
   section separately. Specify canonical numeric/key encoding, bounded lengths,
   logical references and required versus optional section rules. The first
   implementation may favor clarity over compression.
2. Separate portable section data from Any-valued lowered objects and installed
   callbacks. Keep the current pipeline's diagnostics and validators. Add a
   domain hydration adapter that produces the same prepared move/character
   values used by existing consumers.
3. Have the independent Rust builder emit this artifact. Add a source-data
   frontend for the same section where it fits the existing RON path. Compare
   frontend outputs at the admitted semantic value, not at source formatting.
4. Add a selected-host load path through the current source/resolver policy.
   Publish immutable objects plus a complete manifest; partial writes or watcher
   order cannot select a mixed pack. Inspect actual installed technique support. Unknown keys, unavailable
   capabilities, invalid parameters and unresolved references refuse admission.
5. Remove the migrated move table as a compiled authoritative input of the host.
   A test-only old table may be a temporary parity oracle, not a runtime fallback.
   Other content families can remain compiled until their own migration packet.
6. Reuse the existing character candidate path. The first activation can happen
   at startup or a supported local reconstruction boundary. I3 adds coordinated
   repeated reload; do not build arbitrary live-world replacement here.

**Proposed file:** `crates/ambition_content_pack/src/artifact.rs`.
<!-- cite-ok: proposed I2 output, not a baseline file. -->
The move section codec belongs next to its pure value owner. Host hydration
belongs next to character/preparation integration, not in the pure artifact crate.

**Acceptance:** a prebuilt host plays the edited artifact without invoking Cargo
or its linker. The external compiler's dependency guard remains green. Changing
move timing changes a controlled exchange; alternate frontends admit the same
prepared result. Cross-section bad input is rejected before activation.
**Poison:** restore include_str or a game-crate dependency for the migrated input;
M0/link tracing or closure guard must detect it. Change the artifact while keeping
the old in-process table; the observed-move witness must fail. Remove installed
support; artifact admission must refuse even with a valid offline schema manifest.
**Cheapest checks:** artifact codec/roundtrip tests in content_pack and owning
value crate, affected character preparation tests, one selected host integration
module. No renderer build for codec-only tests. **Stop:** unresolved transport or
codec choice can use a simple versioned baseline; it cannot justify serializing
Any/function pointers or adding a second validator.

## I3 - coordinated generation publication and local reload

**Class:** DO. **Requires:** I2.
Read character stage/activate functions, PreparedContentBuilder and
PreparedContentIdentity in runtime content_identity, session_world, rollback
session authority, and the existing construction/reconstruction coordinator.
Read [immutable construction](immutable-content-and-transactional-construction.md)
and [netcode](netcode.md) before claiming a transaction.

1. Introduce one candidate-bundle coordinator in the existing lifecycle/content
   ownership road. Replace process-global mutable-generation assumptions from
   the OnceLock content route for migrated families with App-scoped selection.
   Keep immutable reusable data shareable, but never share activation authority.
2. Factor candidate construction/validation from the current character activation
   function if needed; return a prepared candidate without publishing it. Do not
   call today's mutating activation early and try to undo it. Preserve initial
   cast-withholding policy separately from all-or-nothing revision replacement.
   Extend the runtime's existing digest sections for the artifact and profile
   requirements. Capture domain revisions in the bundle. Candidate hydration
   must not publish one domain while another still validates.
3. Implement the generation/reload state machine and the execution contract's
   binding obligations. Seal the base epoch/profile. Stale candidates refuse or are re-prepared; no
   activation from whichever current registry happens to be readable.
4. Expose explicit validate, describe-diff, reload-request and activation-status
   operations to developer tools. File watching calls the same request path.
   Coalesce notifications without losing the identity of the chosen candidate.
5. Support local scenario reconstruction, remote-session refusal and
   presentation-only reload under the stated classification. Pin the scenario
   input/checkpoint. Implement one bounded safe candidate path with A10 before
   closing reliable reload. Retain the active scene on supported candidate
   refusals; classify explicit recovery separately from unchanged retention.
   Do not promise undo after arbitrary native plugin failure.
6. Add generation binding to any new state or cache. Ensure pending construction
   plans, handles and observers either stay with their generation or are retired
   before stepping the new one. Preserve same-session unhealthy diagnostics.

⭐⭐ **THE REQUEST PATH EXISTS AND IT IS A MESSAGE — MEASURED 2026-09-11, and it
is what step 4's *"file watching calls the same request path"* is about.** The
road to `prepare_platformer_content`, end to end:

```text
ShellEvent::PreparationRequested(ProviderLoadTransaction)   router.rs:490
        ↓
prepare_requested_sessions                                  provider/lifecycle.rs:219
        ↓
PlatformerPreparation::prepare                              provider/lifecycle.rs:280
        ↓
prepare_platformer_content                                  provider/lifecycle.rs:~819
        ↓  epoch allocated as "the final non-fallible step"
sessions.publish(transaction, …)                            provider/lifecycle.rs:~556
```

⇒ **A RELOAD DOES NOT NEED A NEW PUBLICATION ROAD; IT NEEDS TO ISSUE THAT
REQUEST.** The transaction is three fields (`route_id`, `experience_id`,
`barrier`), and the router emits the event as part of a ROUTE change: it opens a
load barrier, marks a pending route and expects an activation to follow. So
re-preparing a LIVE world is a route-level operation, which is exactly I3 step
2's *"a supported local reconstruction boundary"* — not a reload-specific
lifecycle, and the old generation stays authoritative until the activation
publishes the new one.

✅ **A ROUTE CAN BE REQUESTED TO ITSELF WHILE ACTIVE — MEASURED, `f470b18c3`.**
`start_route` has no same-route guard: the request mints a fresh `LoadId` and the
preparation lifecycle runs again, with the old generation authoritative until the
new one activates. ⇒ **The bounded A10 proof is "re-request the current route",
not a new route kind.** `ambition_content::reload::request_reload` issues it
(`e627a4399`), as `ReplaceWith` rather than `GoTo` so a reload does not push
history.

⛔⛔ **BUT RE-PREPARING IS NOT SUFFICIENT, AND THIS IS THE TRAP IN THE OBVIOUS
READING.** MEASURED 2026-09-11: `register_declared_cast` and
`character_catalog::register` run in `AmbitionContentPlugin::build` — ONCE, at App
construction. A session re-preparation reads registries that were built then, so
it moves the `ContentEpoch`, the content fingerprint and the rollback contract
**and does not change a single move table the live cast plays.**

⇒ **THE COMPLETE TRANSACTION IS BOTH ROADS, PUBLISHED AT ONE BOUNDARY:**

| what | which road | what it moves |
| --- | --- | --- |
| the engine's generation | `PreparationRequested` → `prepare_platformer_content` | `ContentEpoch`, content fingerprint, rollback contract |
| the live cast's moves | `stage_move_section` → `activate_staged_revision` | `CharacterCatalogGeneration`, what bodies play |

They are different mechanisms **by necessity** — you cannot re-run `Plugin::build`
— so "route the reload through the existing preparation" is necessary and NOT
sufficient. Anything that does only the first publishes a new generation of the
same moves; anything that does only the second (today's
`publish_candidate`) changes the moves under an unchanged engine generation.

**⇒ WHAT CLOSES I3:** make the activation boundary apply the staged cast revision,
so the two move together or not at all. That is the remaining P0, and it is where
A10's Prepare/Admit/Draft/Verify/Publish/Retire stages earn their keep.

**Acceptance:** valid generation N+1 becomes visible at one boundary; no system
observes N's definitions with N+1's code/schema. Invalid N+1 leaves N's digest,
character generation, playback references and active timeline unchanged. Two Apps
can select different packs without contamination. Same-session rebase cannot
erase a previously unhealthy rollback diagnosis.
**Poison:** publish the character registry before validating another family;
the cross-family atomic-publication test fails. Increment epoch on refusal; the
retention test fails. Reuse a sealed plan after an installer/profile change;
admission fails. Inject a materialization-draft failure after metadata admission;
the supported path retains the old scene. A legacy stopped-world result is not a
passing retained-scene witness. FI2-FI4 specify the independent assertions.
**Cheapest checks:** extend existing character revision tests, content_identity
unit tests and one lifecycle integration module in the shared app_it binary.
**Subcuts and done boundaries:**

| Cut | Ordered output | Acceptance |
| --- | --- | --- |
| I3a | Factor nonmutating candidate hydration; implement complete-bundle seals, no-op identity and stale-attempt rejection | FI2/FI3; no full scene-retention claim |
| I3b | Inventory the selected constructors and hooks; split typed candidate data from active mutation; validate relationships/resource deltas; connect one bounded publication path with A10 | FI4's candidate refusal and valid reconstruction, not only parser failure |
| I3c | Add scenario pin/replay, changed-section explanation, actual activation status and generation-aware cancellation to existing tools | FI1-FI4 plus M0 measurements on the real edit loop |

I3a is independently useful. I3 is complete only after all three cuts. I1/I2 and
I4 contract work need not wait for I3b; procedural replacement does. Do not turn
I3b into arbitrary ECS undo or all-world concurrent simulation.

## I4 - small procedural SDK and one native semantic reference

**Class:** DO. **Requires:** the execution contract; does not wait for a VM.
Read actual boss special producers, domain request types, combat_schedule,
SimId/session ownership and RollbackRegistrar. Use one EchoFan-like technique as
a small real migration probe. Preserve its current request/provenance semantics.

1. Create the dependency-light SDK with module/entry/schema descriptors,
   semantic handles, bounded observations and explicit own-state access. Keep
   selected domain request schemas in dependency-light domain owners, not one
   engine-wide request enum. Reuse pure identity/schema primitives where they
   have the right owner; avoid a giant prelude reexporting the engine.
2. Create a host executor/registration adapter with no named game algorithm.
   Keep it below the runtime composition root: it may use Bevy and the existing
   neutral registration vocabulary, but must not depend on
   ambition_platformer2d_runtime. The runtime composes it and domain adapters.
   Reject any import-cycle workaround that moves game code into the host.
3. Add only the observation/request ports needed by the fixture. The owning
   domain supplies projections, validation and lowering. Couple each port's
   advertised support to actual installation. Fill the domain-contract card for
   each port, including submit/apply distinction, cancellation, grants and results.
4. Map fixture entry points to current public phase/occurrence guarantees.
   Record input freshness, output consume barrier, Commands flush point and
   rejection behavior. Implement stable serial entry ordering first.
5. Implement staged invocation outputs and deterministic limits. Make the
   native reference invoke the same semantic contract without copying all
   inputs through a serialized buffer unnecessarily. Stage only changed records;
   batch at the declared scope and preserve the read cut. Never deep-clone the
   complete store for each callback to simulate a transaction.
6. Port the selected algorithm while retaining a test-only reference trace.
   Its state initially uses a narrow explicit state interface which I5 backs
   with generic registration. Do not invent a VM-wide event bus.

**Proposed roots:** `crates/ambition_extension_sdk/Cargo.toml` and
<!-- cite-ok: proposed I4 package, not a baseline path. -->
`crates/ambition_extension_host/Cargo.toml`.
<!-- cite-ok: proposed I4 package, not a baseline path. -->

**Acceptance:** an independent module builds against the SDK without Bevy or
engine implementation. The selected real technique emits the same accepted
requests and occurrence credit as the native reference. Unsupported ports and
phase cycles fail admission. Engine-owned state is writable only through its
public request protocols in the supported API.
**Poison:** provide a fake metadata-only port; actual admission fails. Try direct
health/body mutation through the SDK in a compile-fail test; no such API exists.
Attempt foreign schema writes; runtime validation rejects them. Misorder entry
execution past the domain consume barrier; same-tick request acceptance changes
and the fixture fails. **Trust limit:** these API tests do not sandbox unsafe
native code or protect against a malicious shared library.
**Cheapest checks:** SDK unit/compile-fail tests, host adapter tests with a tiny
Bevy app, one domain integration trace. **Not complete:** statically linking this
reference into the host does not satisfy no-relink procedural iteration.

## I5 - generic extension state through the existing rollback host

**Class:** DO. **Requires:** I4; I3 before activation of changed state schemas.
Read core snapshot traits, rollback registry/registrar implementation, current
GGRS participation/identity probes, and
`game/ambition_content/src/bosses/specials/rollback.rs`.

1. Implement versioned bounded schemas, collision rejection and canonical
   encode/decode/hash over logical records. Put schema descriptors in immutable
   generation metadata. Generate typed accessors from that schema for Rust;
   avoid a handwritten codec per migrated mechanic.
2. Implement the safe host-owned state store and register its concrete type
   using the current rollback_resource_clone_checksum method. The reference may
   deep-clone active records; immutable versioned chunks are another safe option.
   Keep dormant durable state outside this active snapshot, with explicit pinned
   inputs/handoffs where it affects simulation. Do not clone metadata each tick. SnapshotState decoding has no schema
   context, so do not route it through a global registry. Explicitly cover value checksums, live
   population, dynamic entity creation, reference mapping and session retirement.
   A metadata row or presence-only checksum is not acceptance.
3. Connect attachment to existing semantic IDs and spawner counters. Implement
   required/optional live references separately from durable unloaded references.
   Restore entity mapping before validating references. Test re-created entities,
   not only existing entities whose numbers stay unchanged.
4. Add a module-defined resource, a component-like record, a graph/list and a
   cross-tick cursor. Populate them during real simulation. Include nondefault
   values on both sides of a rollback and a state field controlling a later spawn.
5. Wire save eligibility through the existing persistence/checkpoint owner for
   a required durable fixture. Refuse unsupported policies instead of treating
   everything as either permanent or discardable.
6. Add registry schema identity and update its current consumers in the same
   implementation change. At this baseline the Rust schema fixture and Python
   absence baseline both exist; follow their current owners and do not update
   only one. Prefer a single generator/source if that correction has landed.

**Acceptance:** an actual SyncTestSession rewinds a populated stateful mechanic,
recreates/remaps entities, and produces identical logical state and accepted
request traces. State may be added by a newly loaded schema without recompiling
host storage code. Independent Apps/sessions retire independently. An approved
world record survives a supported save/load while a transient cursor does not.
**Poison:** omit store registration, omit the only differing field from its hash,
leave one counter in a native static, or skip a dynamic population anchor; each
has a separate failing witness. Swap same-shaped records across semantic entities;
the checksum/reference test fails. A simple encode/decode roundtrip does not
replace FI6/FI7. Measure record visits and copied bytes as well as elapsed cost;
one active write must not force a traversal of unrelated dormant records. **Cheapest checks:** schema unit tests, then one populated
real-GGRS integration module. Schema changes also run the narrow schema baseline
guards. Do not run all game scenarios after every codec field change.

## I6 - compare executable backends behind the same contract

**Class:** MEASURE plus a bounded prototype. **Requires:** I3-I5 for honest reload
and rewind evidence. This packet chooses the deployment, not new semantics.
Read M1 and the primary runtime references. Pin versions and target profiles.

1. Use the I4/I5 fixture unchanged as a static native semantic reference.
   Build a WASM module against the small SDK. Supply only admitted deterministic
   imports. Apply the reset, numeric, allocation and work-budget policies from
   the execution contract. Do not expose WASI/files/clocks as convenient defaults.
2. Build a trusted native shared-library prototype with a versioned C entry
   table, fixed-width wire values, caller-owned buffers and no Rust container
   ABI. Implement generation pinning before trying unload/reload.
3. Use the same captured input traces and compare canonical state and domain
   request output across runs. Test failure, malformed output and budget paths.
   Do not extrapolate a float-equivalence result beyond the tested numeric profile.
4. Measure warm module rebuild/load, first-call and steady-state batch costs,
   state reset, allocation, debugger diagnostics and required target feasibility.
   Confirm no runtime host build/link is invoked during module iteration.
5. Record a short decision choosing one production path or naming the precise
   blocker. Remove throwaway prototype dependencies from normal builds. Retain
   useful conformance fixtures, not two production APIs by accident.

**Acceptance:** the report includes raw observations and a justified deployment
choice, not guessed timings or preference for a language brand. Both prototypes
must be honestly bounded; an unsupported platform is reported, not simulated by
running the Linux version. **Poison:** mutate a guest global across calls, allow
an undeclared nondeterministic import, retain a stale callback during reload or
load a schema-mismatched module; the respective conformance test must fail/refuse.
A host relink forced into the helper must fail the iteration witness.
**Cheapest checks:** module tests and selected host backend conformance fixture;
M1 loops. **Do not expand:** into a public mod marketplace, stable ABI for all
Bevy internals, or several production language bindings.

## I7 - deliver the selected procedural path and retire its old road

**Class:** DO after M1 selection. **Requires:** I6.
Keep the I4 reference as a test oracle where useful, not a second live provider.
Read the selected boss file, its required-components installation and rollback
registration before removing old state.

1. Harden the chosen loader with exact executable/profile identity, inactive
   validation, source-linked diagnostics and the I3 activation coordinator.
   Guarantee local code edits do not rebuild the host.
2. Migrate the real mechanic completely. Remove its old live system, concrete
   state attachment and bespoke rollback registration only after the new
   populated conformance witness passes. Do not attach both state roads.
3. Add a graph-based procedural fixture: maintain an authored actor's trail or
   another custom graph, detect a cycle, update a bounded record graph and emit
   an existing domain request. Add no game-specific opcode to the engine IR.
   Where a required fundamental query is absent, implement one reusable owner
   port and report that engine work separately from subsequent module edits.
4. Add deterministic cancellation and occurrence cleanup. A despawn, interrupted
   move or room retirement must not leak references or repeated requests.
5. Expose module/schema/port inspection and source-map diagnostics in the current
   agent tool road. Add a scripting binding only when a concrete authoring need
   chooses it; distinguish offline artifact generation from live simulation.
   Simulation scripts use the same state/reset/port contract and conformance suite.

**Acceptance:** a new algorithm and new state schema reach playable behavior
through a separately built module. The graph fixture proves more than the finite
TechniqueFlow vocabulary. An agent can list required ports, inspect state, see
which generation is active and diagnose a failed replacement without private
World access. Current boss behavior passes its parity trace.
**Poison:** reintroduce the old live system and detect double emission; remove a
rewound graph edge/cursor and detect changed cycle results; add a central enum
branch specifically for this graph mechanic and fail FI8's independent consumer
requirement. Do not encode semantic expressiveness as a source-string ban. **Cheapest checks:** module/schema tests, selected mechanic
parity, populated rewind, then M0/M1 loop. Existing demos are regression customers,
not a reason to postpone this path until every demo is complete.

## I8 - improve runtime and snapshot costs where measured

**Class:** MEASURE-guided implementation. **Requires:** I5 and M2; selected backend
for meaningful end-to-end costs. Keep canonical logical state unchanged.

1. Identify the dominant cost among world projection, guest crossings, reset,
   copying, hashing, snapshot retention and restore/resimulation. Record workload
   sizes and write density. Do not start with unsafe dynamic components by taste.
2. Optimize one cause: batch a port, cache a generation-pinned pure projection,
   partition records by scope, add typed chunks, or use snapshot COW/deltas.
   State plainly whether the change moves compile, runtime, memory or all three.
3. Preserve a reference full logical snapshot and compare every optimized
   restore/hash against it. Delta history must have a bounded base/chain and
   deterministic eviction. Derived caches must survive being cleared each call.
4. For any unsafe dynamic ECS layout, review allocation/drop/reference lifetimes
   and code unloading. For incremental hashing, exercise every write entry point.
   Keep physical layout out of the SDK and save identity.
5. Rerun paired M2 samples and relevant M0/M1 cases. Keep an optimization only
   with evidence that its gain is worth its complexity and no hidden regression.

**Acceptance:** measured improvement in the named loop under representative
populations, with identical replay and request outcomes. A constant-factor gain
on two records does not demonstrate scalable open-world state handling.
**Poison:** bypass one COW/dirty barrier, expire a needed delta base, or reuse a
cache across generations; full-reference comparison fails for each defect.
**Cheapest checks:** affected store/port tests plus chosen scaling fixture.
**Do not expand:** into a new rollback scheduler or engine-wide numeric rewrite.

## I9 - independent consumers, flagship pressure and packaging

**Class:** DO with M3/M0 evidence. **Requires:** first delivery I1-I3 for data
acceptance; I7 and acceptable M2 costs for procedural production acceptance.
Partial delivery reports must name which half is complete.

1. Run a genuinely independent consumer with its own manifest/lockfile. Author
   data, build a procedural module and install a raw Bevy plugin through their
   respective supported surfaces. No private-path allowlist loopholes.
2. Drive a repeatable two-body exchange, one stateful boss/encounter, a graph
   algorithm and a persistent record crossing a supported room/save boundary.
   Use the same simulation with headless and available presentation profiles.
3. Demonstrate independent N-participant/actor identity, not hard-coded player
   zero or a singleton active boss. Add an active-region/dormant-world workload
   so persistent state is not conflated with every-tick snapshot population.
4. Package exact artifacts/modules for the supported development and read-only
   installed targets. Verify no undeclared source checkout files are needed at
   runtime. Report unsupported targets and actual fallback deployment semantics.
5. Test two peers with mismatched content, code, state schema, host protocol and
   numeric/runtime profiles; reject before speculative play. Local reload remains
   distinct from a future coordinated online migration feature.
6. Update normal iteration commands and their cheapest validation selection in
   existing authoring/build guidance. Remove obsolete runtime tables and deferral
   text at migrated seams. Update queue/status with actual witnesses, not counts
   copied from this document.

**Acceptance:** both no-host-relink loops are measured end to end, behavior is
visible, the module boundary supports non-IR algorithms, full Bevy extension
still works, real rollback is deterministic, and packaging uses the same logical
content model. No claim that these fixtures implement the whole future game.
**Poison:** substitute an old artifact, omit a required packaged dependency,
flatten all actors to player zero, bind a new code digest to old snapshots, or
change a view's asset quality to alter simulation; each relevant test fails.
**Cheapest check:** independent fixtures and selected integration/scaling cases.
Run a broader assembly/regression checkpoint only when the accumulated changes
warrant it under AGENTS.md, not once per scalar content edit.

## Prevent incomplete migrations from becoming the default

Before editing, record one small seam card with the current writer, readers,
installer, preparation input, live scope, rollback registration, retirement owner
and file to delete or simplify. Use the source map and the packet's actual paths.
A field declaration location or crate name alone is not an owner.

For each delivery slice, implement preparation, installation, live behavior,
restoration and cleanup together for one customer. A loader with no consumer, a
schema with no populated rewind, or a port descriptor without a reducer stays
open. Keep the old implementation only as a noninstalled test oracle during the
slice. Remove its production writers and compatibility exports before closure.

Stop and amend the owner plan when a requested port needs a new domain authority,
when construction can mutate outside its candidate, or when a state handoff has
no single writer. Do not add a fallback, global context, extra bridge registry or
second gameplay path to get a green test. Add the missing owned contract instead.

Do not stop merely because a measurement is unavailable: implement independent
DO work and leave that measurement's acceptance open. Conversely, a benchmark win
does not waive single-authority or rollback requirements. [FI1-FI10](fast-iteration-acceptance.md)
are concrete fixture specifications, not ten new compulsory binaries or a demand
to run all ten after every edit.

## Validation routing and completion receipts

Commands naming current packages below exist at the inspection baseline. New
fixture commands are added by their owning packet only after the files exist.
Do not paste a proposed crate command into a completion report as an executed test.

| Edited responsibility | Normal local check | Escalation trigger |
| --- | --- | --- |
| Planning only | Planning Markdown/pointer tests; diff and link review | None to Rust simply because a plan mentions code |
| Pure move/schema helper | Owning unit tests; independent compiler fixture | Shared format or runtime hydration changes |
| Artifact input only | Compiler validation plus selected prebuilt-host observation | Protocol/schema change, not a scalar move value |
| Procedural algorithm only | Module unit tests and trace replay | New state/port/phase or backend change |
| Schema / state storage | Canonical codec, hash sensitivity, populated real rewind | Shared registry/backend change |
| Host/domain adapter | Selected domain and app_it module | Cross-domain lifecycle/profile change |
| Runtime assembly / package boundaries | Relevant profile, workspace-policy and assembly checks | Deliberate integration checkpoint |

Current examples, run only for the responsibility they cover:

```bash
python3 -m pytest -q scripts/tests/test_planning_markdown_structure.py \
  scripts/tests/test_planning_pointers_are_live.py
cargo test -p ambition_entity_catalog --lib
cargo test -p ambition_content_pack --lib
cargo test -p ambition_characters --lib refused_revision
cargo test -p ambition_workspace_policy
```

For app integration, add tests to the existing shared app_it target and use
`cargo test -p ambition_app --test app_it -- <selected_module>` with the actual
module name. Do not add one integration binary per new case. Check target/disk
prerequisites before builds. The architecture merge gate is the applicable
assembly check; it is not every-edit policy.

Every completion receipt names the source revision, changed authority, removed
old road, positive behavior witness, independent poison and its observed failure,
commands actually run, omitted checks with reason, and measured versus unmeasured
cost claims. Restore the poisoned code before committing. A source grep is not
proof of execution; a successful codec roundtrip is not proof of participation;
a dependency count is not a compile-time speedup.
