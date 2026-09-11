# Fast content iteration and the engine extension model

**State:** implementation design, not a delivered runtime. Source inspection:
`d81a7ae1d2db1fc5caa49efc807a39ea6b1ca266`, 2026-09-11.
**Authority:** this page owns the extension architecture. The
[execution and state contract](extension-state-and-execution.md) supplies its
protocol details. The [packet catalog](fast-iteration-implementation.md) supplies
implementation steps. [Evidence and experiments](extension-iteration-evidence.md)
separate source facts from outstanding measurements. Only [the queue](../queue.md)
selects work. These pages do not create another queue or rollback backend.

## Intent and decision status

The maintainer needs ordinary game edits to stop paying engine build and link
costs. The target game is a systemic 2D platformer combining Skyrim-like scope,
Smash-like combat, Hollow Knight-like exploration and encounters, and original
mechanics. Those comparisons communicate ambition, not a checklist of copied
features. Current demos are test customers, not the engine's expressiveness limit.

Preserve the original requirements: lightweight Rust authoring, eventual runtime
scripting, sophisticated custom mechanics, rollback-aware state, and direct use
of Bevy where it helps. A small SDK must not mean a small set of possible games.
The present iteration problem is a concrete customer for an executable extension
boundary. Do not defer the whole boundary until public mod distribution exists.

Use these labels in implementation reports:

| Label | Meaning | Agent action |
| --- | --- | --- |
| FACT | Source observation or an executed result with its stated limits | Recheck relevant source on the working head |
| DO | Architecture selected here from ownership, lifecycle or dependency requirements | Implement in the selected packet; no benchmark needed to justify the invariant |
| MEASURE | Implementation choice depends on cost, portability or tool behavior | Run the named experiment; do not block unrelated DO work |
| PRODUCT | User-facing policy not decided by architecture | Use the safe default here; escalate only when shipping that policy |

DO is an engineering decision under the maintainer's requirements. It is not a
claim that every crate split or runtime implementation will be faster. A measured
regression can change a storage or deployment choice without abandoning the
semantic boundary.

## Decisions agents should implement

**D1. Separate content production from the engine executable.** Ordinary
migrated data edits produce a loadable artifact without rebuilding or relinking
the host. A Rust builder may itself compile and link, but only against pure
content/schema libraries. A small crate statically linked into the heavy binary
is an intermediate improvement, not completion of this requirement.

**D2. Reuse the existing preparation road.** Extend `ambition_content_pack` and
the owning domain validators. Hydrate admitted artifacts into existing prepared
definitions. Reuse character staging, installed technique admission and prepared
content identity. Do not add a competing catalog, generic scene constructor,
script-only damage system or second flow interpreter.

**D3. Keep three semantic tiers.** Data composes domain concepts. Procedural
extensions implement arbitrary bounded-per-invocation algorithms and own new
state. Engine plugins add or change fundamental services through ordinary Bevy.
Rust, a script interpreter, and WASM are bindings/deployments of the procedural
tier, not different gameplay authorities. Authoring scripts may instead emit data.

**D4. Make state and ownership explicit.** Every persistent value that can affect
future simulation belongs to registered rollback-visible state. Procedural code
may mutate its own state. It changes engine-owned facts through the same domain
protocols as native systems. Read projections do not transfer mutation authority.

**D5. Keep Bevy in the engine.** Domain plugins, ECS storage, schedules, assets,
rendering, diagnostics and advanced engine extensions remain Bevy-native. The
portable SDK exposes no raw World, Commands, Entity or ComponentId. A separate
static Bevy adapter can offer typed systems and queries without entering the
portable crate's dependency closure. Do not build a second general-purpose ECS.

**D6. Make code, data and schema a generation.** Extend the existing prepared
content identity with executable and schema sections. A generation is immutable.
Rejected preparation or admission leaves the active generation untouched.
Mechanical activation initially uses the supported local session/reconstruction
boundary, not arbitrary live state migration. Remote sessions pin their generation.

**D7. Use one rollback authority.** The existing registrar and GGRS host own
snapshot participation and replay. Normal extensions get generated state codecs
from registered schemas, not a new bespoke rollback implementation per mechanic.
Save/checkpoint eligibility remains an explicit, separate lifetime decision.

**D8. Prove changed cost classes.** Keep content, procedural-module and engine
edit loops distinct. Tests and tools must report the selected loop and actual
build closure. Never call moving files a latency improvement without an
edit-to-observable-result measurement.

## Target data flow and dependency direction

```text
Rust builder / RON / editor / authoring script
       |  pure domain values and shared validation
       v
ambition_content_pack -> portable versioned artifact
                              |
                              v
                 selected host admission + hydration
                              |
                              v
                 immutable prepared generation
                              |
          +-------------------+----------------------+
          |                                          |
  existing domain runtimes                 procedural host adapter
          |                                          ^
          |                               narrow semantic contract
          |                                          |
          |                             Rust / WASM / script module
          +-------------------+----------------------+
                              |
               domain requests and read projections
                              |
                   ordinary Bevy simulation
                              |
                 existing registrar / GGRS host
```

This is not a requirement that every domain depend on one new central crate.
The host adapter depends on selected domain protocol adapters. Those adapters
own lowering. The SDK contains protocol values and schema handles, not private
domain implementation. Installing a new service extends a versioned port set;
it must not edit a central match over all game mechanics.

### Dependency map

| Surface | Initial location / dependency rule | Explicit exclusion |
| --- | --- | --- |
| Move values and pure builders | Existing `ambition_entity_catalog`; move only the pure helper closure from characters | Bevy, live character preparation, combat runtime |
| Generic artifact pipeline | Existing `ambition_content_pack` and `ambition_registry_core` when genuinely reused | Installed handlers, game catalogs, runtime hydration |
| Domain artifact schemas | Owning domain's pure value module; extract a leaf only where a real compiler import proves the need | Runtime/plugin dependency pulled in solely for schema discovery |
| Procedural values/state/ports | Proposed `ambition_extension_sdk`, independently buildable | Bevy umbrella, bevy_ecs, runtime, render, audio, game crates |
| Host adapter | Proposed `ambition_extension_host`; selected domain adapters install into it | Game-specific algorithms or an inventory of every extension's concrete state type |
| Static native integration | Separate Bevy-facing adapter or domain plugin, paired to the engine version | An optional SDK feature that makes portable authors inherit Bevy through feature unification |
| Composition SDK | Existing `ambition_platformer2d` facade and supported profiles | Claim that its broad closure is the normal content author's SDK |

The proposed crate names are design targets, not existing packages. Start with
the existing pure move leaf. Do not extract all of `ambition_characters` or the
actor SCC to obtain a builder. Keep pure implementations in one place; migrate
callers and remove internal compatibility reexports at the moved seam.

A wire-level value can have a native wrapper with familiar Bevy math conversion.
That does not require importing the ECS into the wire layer. Reuse existing
semantic IDs after separating their value definition from their Bevy derive where
needed; do not invent a second identity algebra just to avoid a dependency.

## IR boundary

Keep three representations with distinct responsibilities:

| Representation | Meaning | What it must not contain |
| --- | --- | --- |
| Authored source | Editable documents, builder expressions, provenance, symbolic references | Hidden reliance on the current running World |
| Portable artifact | Canonical domain sections, bounded programs, declared references, schema and protocol versions | Rust function pointers, Any values, vtables, Bevy handles, device resources |
| Admitted runtime generation | Selected profile, resolved services, prepared domain values, runtime handles and immutable executable objects | A fallback parser or unchecked alternate source of the same definition |

These need not become three copies of every struct. A pure serializable MoveSpec
may serve two stages. Runtime-prepared character/body objects need not become
portable wholesale. Follow A6's field census: source, validation, identity,
materialization and live state are separate responsibilities.

The artifact envelope carries format version, required domain section versions,
canonical content sections, logical references, capability/port requirements,
module descriptors, state schemas and optional diagnostic provenance. Required
unknown sections fail closed. Optional sections are ignored only if their contract
says they cannot affect simulation. Debug paths and timestamps are outside the
mechanical digest. Asset references carry stable logical identity and the exact
mechanical content digest where an asset affects gameplay.

Pure validation checks syntax, bounds, finite numbers, reference shapes and
cross-section consistency. A compiler can validate against an exported profile
schema manifest. That manifest is not permission to execute: the host rechecks
that actual selected installers provide every required service and version.
Never serialize installed function-valued TechniqueOffer data. Preserve one
validator implementation; run cheap host admission even for a trusted compiler.

The IR includes existing moves, techniques, flows, encounters and other domain
concepts when their owner defines them. It does not acquire an enum variant for
every new spell or boss algorithm. Add a domain concept when it expresses a
reusable ownership contract; use procedural state/code for the algorithm.
TechniqueFlow remains the bounded move-local interpreter. A procedural technique
provider may implement an installed technique, but global quests, factions and
world algorithms do not become fake moves just to obtain an entry point.

## Two scripting modes, one procedural contract

**Authoring mode** can generate artifacts offline. It does not participate in
rollback. Inputs such as files or model output become explicit build inputs, and
the resulting artifact is validated normally. Runtime play never reruns the
source generator to reconstruct an old tick.

**Simulation mode** runs procedural code under the
[execution contract](extension-state-and-execution.md). It reads published world
facts, updates extension-owned state, and emits authorized domain requests.
Its arbitrary algorithms can operate on graphs, arrays and custom records. It
cannot bypass ownership by writing health, body velocity, custody or progression
components directly. Missing fundamental host functionality is implemented as a
Bevy domain service, not exposed through an unrestricted mutation escape hatch.

A static native reference binding establishes semantics first. It does not meet
the no-host-relink procedural acceptance by itself. Compare a portable WASM
binding with a narrowly specified trusted native ABI using the same fixture.
WASM is the first portable prototype, not a promise that a particular runtime is
fastest or supports every target. Lua and Rhai remain script-binding candidates;
choose a human-facing language after the procedural contract and measurements.
Do not ship several runtimes merely to avoid deciding which one meets the needs.

## Generation, hot reload and packaging

One coordinator combines domain candidates with module code and state schemas.
The current character registry's last-good staging is a building block, not a
reason for each domain to publish its own unrelated global generation. Candidate
hydration is allowed to use in-process prepared objects. Publication selects an
immutable bundle; runtime systems do not independently poll source files.

The initial mechanical reload policy is explicit:

| Context | Policy |
| --- | --- |
| Invalid candidate | Report source-linked diagnostics; keep active definitions, executable generation and timeline unchanged |
| Local development, valid mechanical change | Queue for the supported session/reconstruction boundary; validate before retiring anything; start a fresh timeline bound to the new generation |
| Active remote session | Refuse in-session mechanical replacement; require all peers to admit the same next-session generation |
| Presentation-only change | Use existing asset reload only when no collision, timing, perception or other mechanical fact depends on the changed bytes |
| Arbitrary schema migration or state-preserving code swap | Not an initial guarantee; later migration needs explicit transform, validation and failure policy |

Do not clear history as soon as a file changes. Even identical state schemas do
not make old code and new code interchangeable during replay. A developer rebase
inside the same session must preserve an existing unhealthy rollback diagnosis;
use the existing session/timeline rules, not reload as a way to erase a desync.

Last-good definitions are not an atomic-world-undo promise. The current native
construction road does not undo arbitrary failed Commands. A10 owns that stronger
guarantee. This project can deliver useful reload by validating an inactive
prepared bundle and using a supported reconstruction boundary. A failure after
destructive world commit is a stopped/failed session unless A10 has actually
provided recovery. Never report that failure as a successful retained old world.

Development uses watched loose artifacts and an explicit reload command through
the existing asset/source resolver. Watch events are notifications, not ordered
simulation input. Release packages freeze the same admitted artifact format and
module identities. Web/installed targets use their supported read-only transport,
not a hidden local filesystem fallback. Static native release integration is an
optional backend choice and must preserve the declared contract; a different
binary is a different executable identity until equivalence is demonstrated.

## Flagship expressiveness and scope

Use existing game content for migration and larger fixtures for architectural
acceptance. Fixtures specify capabilities, not new product design commitments.

| Pressure | Required architectural answer |
| --- | --- |
| Smash-like contact, combo, movement and N-participant combat | Existing acceptance, body and contact owners remain sole authorities; request occurrence provenance survives replay |
| Hollow Knight-like traversal, bosses and room progression | Procedural encounter state plus authored moves; lifetime is not tied to a camera or a currently loaded sprite |
| Skyrim-like systemic world, actors, items and reactive quests | Persistent world-owned extension records and confirmed durable transitions; do not rewind all dormant world data every frame |
| Original graph/topology spell or custom strategy algorithm | Add records and code, not a new central IR opcode for the specific algorithm |
| Multiple views and presentation backends | Same strictly 2D simulation result; views and 2.5D staging do not become simulation authorities |

A gameplay extension can ask installed world/query services for obstacle and
spatial facts. It cannot silently ship a contradictory physics solver. Adding a
fundamental spatial service may still be engine work. That is a meaningful tier
boundary, not evidence that all custom gameplay must compile the engine.

## Existing work preserved and work replaced

A9 still measures minimal composition profiles. This program adds a distinct
content/module iteration boundary; a no-render facade is not its completion.
A6 still owns field responsibility. A11/A12 still own technique admission and
move execution. A2/contact and A4/body rules govern corresponding request ports.
A1/checkpoints govern durable restoration. A10/world-undo and A8/multi-instance
world are not blanket prerequisites for pure authoring or artifact loading.

Remove the old requirement to wait for a modding customer before designing
runtime extensions. Retire migrated compile-time content tables as authoritative
inputs, ad hoc script rollback implementations, and whole-host linking for those
content edits. Keep domain compilers, asset tools, LDtk and ordinary Bevy plugins.
Do not combine all authored media into one universal compiler.

## Exit standard

Data edit: a changed authored move reaches the selected running host through an
artifact; no host build/link occurs. Procedural edit: a changed algorithm and new
state schema can be built against the small SDK, admitted and exercised without
host relinking. Populated GGRS rewind produces the same canonical state and
request results. Invalid replacement preserves the active generation. An external
Bevy plugin still uses normal Bevy APIs. Each claim has the independent poison
witness in the [packet catalog](fast-iteration-implementation.md).

Runtime speedup, backend selection, snapshot layout and cross-target execution
remain measured claims. The [experiment table](extension-iteration-evidence.md)
names their evidence and dependencies; none blocks the pure artifact boundary.
