# Roadmap

## Current architecture/product experiments (2026-11-06)

The flagship is being evaluated as a **player-hosted authoritative persistent world**, with client-side prediction for controlled actors and independently simulated/replicated `activeArea` regions. This is **not a completed backend choice**: the [hosted-world vs GGRS experiment](engine/hosted-world-networking-experiment.md) owns the measurement and decision, and [NET-INPUT-WIDTH](queue.md#net-input-width--remove-idle-room-consensus-from-every-frame) records that the GGRS input was compacted (2026-11-10). Smash retains rollback. Do not prematurely build independent GGRS histories for every area while the flagship authority model is under evaluation.

NPC life is a first-class customer: [Radiant-style authored HTN plans and smart-object/offscreen semantics](engine/radiant-npc-planning.md) must work without an engine rebuild, not as a giant Bevy entity hierarchy tied to always-resident bodies. Boss content has a separate selected [roster-wide cinematic intro and GNU2 finale/motion fidelity plan](game/boss-cinematics-gnu2-and-motion.md). These add executable queue tasks; they do not redefine the no-compile G0-G5 acceptance gates.

**2026-10-26 architecture revisit:** This roadmap's older priority and dependency ordering is
reviewable, not a permanent maintainer constraint. The
[reassessment](engine/architecture-reassessment-and-composable-multiplayer.md) connects
no-required-server multiplayer, optional rollback, deterministic fixed-step base, general
persistent objects, optional Avian/ragdolls and evidence-based crate boundaries. It recommends
vertical experiments before selecting a wholesale backend, island, or crate migration. The queue
still selects implementation.
Current facts are in [`status.md`](status.md). Immediate execution is in
[`queue.md`](queue.md). [`tracks.md`](tracks.md) is the standing reservoir.

## North star

Ambition is the flagship game. The product thesis remains a systemic 2D
platforming world: persistent actors and objects, embodied capability/item
progression, open-world traversal, reactive characters, multiplayer residency,
and agent-native authoring.

The engine becomes reusable by making those capabilities ordinary Bevy
plugins/crates and a semantic SDK rather than by exposing Ambition's historical
crate topology.

## Architecture-completion roadmap: no host recompilation for game development

**Goal:** authors can create and iterate on a playable Ambition game through data,
world edits, media and bounded procedural behavior without rebuilding the engine
host. Changing fundamental engine services remains ordinary Rust/Bevy work.
Static release builds *may* embed/recompile selected content. This is a
completion path across existing owners, **not** a replacement execution queue.
Selection and priority remain in [the queue](queue.md); unselected work remains
in [tracks](tracks.md).

### Precisely which edits avoid compilation?

| Edit class | Target loop | Current posture and owner |
| --- | --- | --- |
| Authored gameplay data (moves, characters, rules, items, quests, dialogue, encounters, audio registry) | Save source -> compile/validate pack -> admit -> reconstruct affected session -> observe; **no Cargo or host link** | Implemented for many families; coverage and duplicate Rust readers remain. [Content generation](engine/content-generation-and-reload.md), [I2/I3](engine/fast-iteration-implementation.md), [content-as-data queue](queue.md#content-as-data--content-still-written-as-rust) |
| LDtk rooms and spatial relationships | Edit -> semantic validation -> candidate world rebuild -> observe; **no host link** | World reload and semantic tools exist; field authorability, typed references, diagnostics and integration with other candidate changes remain. [LDtk tools](engine/ldtk-authoring-and-world-tools.md) |
| Images, sprite rigs, music, SFX and shaders | Produce/choose local asset -> select -> restart or safe visual refresh; **no host link** | Published assets and some live restatement exist; selected-file/tool agreement and inexpensive publish/preview remain. [Asset library](engine/asset-library-editor-and-delivery.md), [asset preparation](engine/asset-preparation-and-residency.md) |
| Custom gameplay algorithm with new state | Build **small independent** Rust/WASM module -> admit -> run/rebase -> observe; **no host link**, but the module still compiles | WASM backend and shipped migrated customers exist. Lifecycle cleanup, complete domain coverage, scenario-level reload and scaling proof remain. [Extension model](engine/extension-model.md), [execution](engine/extension-state-and-execution.md) |
| Literal no-compiler custom algorithm | Edit an interpreted script -> admit -> observe; **no module compilation** | Not a shipped guarantee. Lua/Rhai or another interpreter is a conditional binding choice; it must use the existing procedural state/port contract, not a second authority. [Extension model](engine/extension-model.md#two-scripting-modes-one-procedural-contract) |
| New physics/simulation/renderer service or native Bevy plugin | Build/test engine source | Intentionally outside the no-host-rebuild content promise. [Architecture](engine/architecture.md), [SDK](engine/public-sdk-1.0.md) |

**Current coverage:** the core ownership and extension model is specified and
substantially implemented, but the whole *author-to-running-game-to-release*
path is not yet one proven product contract. Do not call a reload family complete
because a watcher fired, a schema parsed, or an artifact was accepted without
observing the changed behavior in the actual host.

### Dependency-ordered completion gates

These are **architectural gates**, not additional scheduled tasks. Complete or
reconcile each existing owner/queue item rather than creating an independent
implementation track.

1. **G0 — One observed iteration contract.** For each supported edit class,
   exercise a real edit, independent preparation, selected admission,
   first observable gameplay/presentation change, rejection and restoration.
   Record changed inputs, selected artifact/generation, stage costs and whether
   Cargo/host linking occurred. Establish cold/no-op/warm numbers on the same
   machine, then choose budgets from measurements. Reuse
   [M0-M3](engine/extension-iteration-evidence.md) and
   [FI1-FI10](engine/fast-iteration-acceptance.md). This gate is partially met;
   do not reschedule successful fixtures.
2. **G1 — Finish the cheap content and asset loop.** Close concrete
   [CONTENT-AS-DATA](queue.md#content-as-data--content-still-written-as-rust)
   readers and [I2/I3](queue.md#i2i3--finish-independent-content-authoring-and-safe-reload)
   customers; make selected-file resolution agree across runtime, editor and
   packager; avoid old-bundle hashing on ordinary publication and release
   auditing during local preview. Retain usable stale artifacts as diagnostics,
   not routine hard stops. Owners: [content generation](engine/content-generation-and-reload.md)
   and [asset library](engine/asset-library-editor-and-delivery.md).
3. **G2 — Complete world and behavior authorability for real game features.**
   Discover supported authoring vocabulary; inspect, mutate, validate and render
   a room through semantic commands. Type the spatial references that currently
   require strings, with the movement-platform path as the first witness.
   Make one real custom algorithm and its state portable across control/actor
   identities through declared domain ports; add a missing *reusable service*
   only when the real algorithm needs it. Reuse
   [LDtk tools](engine/ldtk-authoring-and-world-tools.md),
   [authored orchestration](engine/authored-gameplay-logic-and-orchestration.md),
   [domain ports](engine/extension-domain-contracts.md) and
   [FI8](engine/fast-iteration-acceptance.md#fi8---arbitrary-algorithm-and-actor-symmetry).
   General reusable compositions/prefabs remain customer-gated, not a prerequisite.
4. **G3 — Make multi-domain change admission coherent.** Explicitly describe
   which changes are presentation-only, definition changes, room rebuilds or
   procedural code/schema changes. A candidate must name its exact content,
   module, world and asset selections and affected references; avoid mixed
   mechanical generations. Reject malformed or superseded candidates without
   disturbing the playable old state. A valid mechanical change enters through
   supported reconstruction, preserving any durable save compatibility policy;
   remote peers pin compatible mechanical generations. Existing separate world,
   pack, module and sprite reload roads should be *reconciled*, not replaced with
   a universal file watcher or unsafe arbitrary state migration. Owners:
   [generation/reload](engine/content-generation-and-reload.md),
   [construction](engine/construction-and-reconstitution.md),
   [extension state](engine/extension-state-and-execution.md),
   [netcode](engine/netcode.md).
5. **G4 — One author-facing workflow, not one new source of truth.** From a
   stable CLI and the existing Editor shell, support `discover -> describe ->
   propose -> validate -> prepare/publish -> reload -> inspect/diff -> replay`
   for representative world, character, move, asset and procedural edits. Show
   the active selection, why an edit was refused, and what remains stale or
   unselected. The editor is a client of domain tools and read-only derived
   indexes; the normal runtime never depends on its database. Owners:
   [authoring/tools](engine/authoring-and-tools.md),
   [tooling layout](engine/tooling-layout-and-editor-consolidation.md),
   [inspection](engine/inspection-diagnostics-and-workbench.md).
6. **G5 — Independent-game and shipping proof.** An out-of-workspace game uses
   documented public profiles to author a room, body, action, item/encounter,
   presentation and persistent procedural record. It edits data and an
   independent module without host relinking; tests headlessly; refuses a bad
   candidate; replays/restores deterministically; and packages exactly its
   selected assets, worlds, schema and modules for a supported native target.
   A second platform is separately proved, not inferred. Release auditing is
   explicit; local preview remains cheap. Owners:
   [SDK](engine/public-sdk-1.0.md),
   [build/distribution](engine/project-build-and-distribution.md),
   [asset delivery](engine/asset-library-editor-and-delivery.md),
   [FI9-FI10](engine/fast-iteration-acceptance.md).

### Missing cross-owner agreements to settle as engineering work

- **Selection and version identity:** what exact content, module, world and
  asset selections produced the observed session and the installed artifact?
  Which changes require a fresh mechanical timeline versus a visual rebind?
  Not every media file needs a mechanical digest.
- **Rebuild scope and author feedback:** which dependent domains need
  re-preparation, which active rooms need reconstruction, what was reused, and
  how does the author receive source-qualified refusal and observed activation?
- **Behavior completeness:** which remaining game-authored mechanisms still
  require editing Rust tables or private engine wiring; which can use existing
  data, procedural ports or an intentionally native plugin? Do a real-customer
  inventory, not a file-count campaign.
- **Durable boundaries:** state schema, rollback wire identity, save compatibility,
  multi-room occurrence lifetime and peer admission are separate contracts.
  Do not treat a working local reload as proof of all of them.
- **Portable authoring and release:** can a fresh consumer/editor session
  enumerate installed vocabulary and ship exact artifact closure without
  the flagship source tree, ignored local files or unnecessary recompilation?

**Scope limit:** complete *architecture* means each kind of game change has one
supported authority and a testable path from source to observation and release.
It does not imply all Ambition levels, bosses or content are finished; it does
not require a Godot clone, a language marketplace, no engine compilation ever,
background simulation everywhere, or a new central ECS. Keep the broader game
and runtime programs below active independently when their real customers demand
work.

## Engineering priority order

The priorities below are scored on two axes. The first is authority
decomposition: which crate owns the fact, what may mutate it, one lifecycle,
dependency direction. The second is capability composability: can this capability
be absent, does the rest still form a coherent application, does it declare only
its real prerequisites. The second does not follow from the first. A repository
can satisfy every ownership rule and still ship an effectively indivisible engine.

Authority and capability-closure problems may be investigated in either order; a particular migration should sequence them only when its measured technical prerequisites require it. A slice need not deliver both.
Do not read a run of authority slices as progress on decomposition. A landed slice
says which axis it moved.

The rule, the ordering, the absence criterion and the minimum-host tests live in
[`engine/architecture.md`](engine/architecture.md#decomposition-doctrine) under
"Two dimensions", with the durable statement in
[`../architecture/package-and-capability-boundaries.md`](../architecture/package-and-capability-boundaries.md).
They are not restated here.

### Program order (Jon, 2026-09-28)

Pick work from the programs below in this order. Each slice needs a real
Ambition consumer; a duplicate authority is fixed when a slice finds it, not by
a broad census.

1. Deterministic simulation and lifetime correctness, with canonical
   construction where it is a prerequisite.
2. Runtime-loaded content and prepared generations (fast iteration).
3. Capability composability and the public SDK.
4. The persistent world, with the Alice/Bob delivery quest as its acceptance
   slice.
5. Multiview and multiplayer on the same participant, body and world model.
6. Reactive characters: world facts, observations, memory, goals and typed
   intents.

Measured runtime quality and build iteration are not a program of their own:
take them when a measurement makes them the blocker.

### Program 1 — authoritative-state correctness and lifetime boundaries

The immediate correctness program is broader than rollback registration. An
authoritative population needs the right rewind codec and participation, stable
semantic identity where reconstruction or peer selection depends on it,
deterministic composition when multiple entities affect one result, and the
correct gameplay-session/timeline owner.

Rollback authority is gameplay-session-owned. Remaining work includes
runtime-created populations, deterministic selection/composition, and related
structural tests.

Owner: [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md).

### Program 1 prerequisite — canonical construction and reconstitution

**Converged.** Fresh construction, room transition, same-room replay, new-game
reset and save restore run one constructor. A save load adopts its occurrence
ledger at activation, before initial construction. Each new session or room is a
hidden candidate that is verified before one publication switch; a failed
candidate leaves the live world unchanged. The shape is stated in
[`../architecture/engine-architecture.md`](../architecture/engine-architecture.md#content-and-construction).
New reset or reconstruction roads must use this model, not a separate ledger.

Owner: [`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md).

### Program 2 — fast iteration and prepared generations

The maintainer's edit-to-play latency is a current architecture requirement, not
only the lower-priority profiling work below.

Where it stands: content packs compile through one path
(`ambition_content_pack::compile`) with no Rust rebuild. A development build
reads content off disk, and a running game plays a saved edit to movement
defaults, combat and time feel, boss tuning, dialogue, items, audio registries,
fighter facets and the character catalog. A session reads the content generation
it was prepared against. Named content keeps moving out of Rust tables into pack
schemas (quests, cutscenes, music cues, room bindings).

Remaining work: one prepare/admit/publish contract across every reloadable
registry, and procedural modules with shared rollback state. The owners are the
I2/I3 row in [the queue](queue.md) and the
[extension model](engine/extension-model.md). Do not wait for a mod marketplace,
every demo, or a whole actor-monolith carve.

### Program 4 — persistent systemic world foundation

Build world residency, occurrence lifetime/provenance, item custody, body/item
capability gating, persistent actor population, and platformer reachability on
the P0/P1 ownership and reconstruction model.

Owners:

- [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md)
- [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md)
- [`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md)
- [`engine/platformer-navigation-and-reachability.md`](engine/platformer-navigation-and-reachability.md)

Where P2 stands:

- **Body/item capability gating.** The engine publishes route-facing conditions,
  and a route gate (`gated_by`) can read any of them. Run
  `scripts/measure/authored_route_gates.py` for the current list; do not copy it here.
  What is missing is facts, not predicates: soft systemic pressure,
  social/knowledge state and broken world mechanisms have no durable
  route-facing record. A smashed breakable is session state restored by
  re-authoring, by design (`SpawnOrigin::Dynamic` is not persisted).
  [`engine/world-facts-observations-and-memory.md`](engine/world-facts-observations-and-memory.md)
  owns the durable families. The authored world uses relatively few
  conditions. Q55 is ruled: do not manufacture demo content merely to
  exercise each capability family; sparse usage is not itself an engine deficit.
- **Item custody.** The explored migration paths are documented; remaining
  work is engineering rather than two pending maintainer rulings. The item
  owner still records the atomic durable-horizon writer, orphaned holders,
  match-item provenance and other lifetime residuals. The selected
  `LOADOUT-AUTHORITY` queue row fixes the separate menu-equipment write path.
  See [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md)
  and [`queue.md`](queue.md#loadout-authority--equipment-changes-enter-through-simulation-input).
- **Occurrence lifetime.** An occurrence enters the whereabouts ledger through
  custody, and the ledger enforces it, so a room unload cannot silently erase a
  persistent instance.
- **World residency.** Several rooms can be live at once. Each live room is its
  own root entity, entities carry an `InRoomInstance` stamp, and each view draws
  the room it frames. Separated players each keep their room live. A room that
  is not live does not simulate; a mechanic that must keep time uses the session
  clock. Open work (the OW cuts and the remaining `SoleLiveRoom` readers) is in
  [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md).

### Measured runtime quality and developer iteration

Treat performance as several measured problems rather than one generic ECS
optimization agenda:

- weak-GPU framebuffer/raster cost;
- asset demand, render materialization and residency;
- startup only where measured;
- build/test/profile iteration cost.

Do not revive generic system-count reduction, broad change-driven projection,
parallel `GgrsSchedule`, or capability stripping as CPU work without new
evidence.

Owners:

- [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md)
- [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md)
- [`engine/project-build-and-distribution.md`](engine/project-build-and-distribution.md)

### Program 3 — ownership-based engine composition and supported public profiles

Use [`engine/architecture.md`](engine/architecture.md) for the target authorities
and the [bounded packets](engine/actor-monolith-work-frontier.md) for which
packets have landed and which are open; this page does not copy that list. The
packets are independent work streams, not sequential prerequisites for game
development. The frontier's A12 (flow bounds) is not `queue.md`'s A12
(move-contact attribution). The [queue](queue.md) selects current priority.

The outcome is a set of recognizable state/behavior/lifetime authorities and a
public programmatic engine that can be used without accidental flagship
requirements. Preserve the corrected spawn boundary, coherent internal cycles,
explicit composition and normal downward dependencies. Do not optimize for crate
count, zero foreign installs, zero SCCs or a cosmetically renamed runtime.

### Program 5 — multiplayer and multiview

Apply the same participant, actor, lifetime, world-residency and presentation
semantics to local, online and mixed participants and to shared/fixed/adaptive
split presentation. Local different-room play exists in the engine: separated seats
each keep a live room and each view draws its own room. A second seat joins by
an explicit gesture in an opt-in mode (Q153, ruled 2026-10-19; LOCAL-PLAY-MODES). Online transport waits for a real
customer. The peer input's idle 96-byte preparation verdict became an 8-byte
token on 2026-11-10 (168 to 80 bytes), which moved link traffic by 0.5–4%.
Independent rollback histories per live `activeArea` instance wait on the
authority experiment. See [compact peer input and simulation
islands](engine/island-rollback-and-compact-peer-input.md).

Owners: [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md)
and [`game/multiplayer.md`](game/multiplayer.md).

### Program 6 — reactive world, characters and authored orchestration

Expose deterministic world truth and observations first. Let character
AI/dialogue and authored orchestration consume typed facts/actions without
creating a second source of authoritative state.

Owners:

- [`engine/world-facts-observations-and-memory.md`](engine/world-facts-observations-and-memory.md)
- [`engine/agentic-character-runtime.md`](engine/agentic-character-runtime.md)
- [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md)
- [`game/reactive-characters-and-dialogue.md`](game/reactive-characters-and-dialogue.md)

## Cross-cutting Engine 1.0 competitive capability bar

The priority order above is architecture sequencing. Engine 1.0 also has a
**product completeness bar**: it must be able to support serious 2D games across
ordinary rendering, movement/collision, animation/VFX, audio, UI, input, assets,
persistence, diagnostics, headless testing and project build/package concerns.

That bar is owned by
[`engine/godot-class-2d-capability.md`](engine/godot-class-2d-capability.md).
It is deliberately not an editor roadmap. Ambition competes through engine
capability, runtime/build efficiency, semantic expressiveness, public composition,
inspectability and LLM-first operation.

Use the capability map as a **gap detector**, not as a second queue:

1. a real game/customer exposes a missing ordinary engine capability;
2. check whether Bevy or a maintained ecosystem plugin already supplies the
   generic mechanism;
3. identify the semantic/composition/public layer Ambition actually lacks;
4. route the executable slice to the focused plan and queue;
5. verify it through Ambition plus a materially different customer where the
   boundary is supposed to be reusable.

Do not promote visual-editor parity, visual scripting, a scripting-language
clone, a plugin marketplace, general 3D breadth, or generic rigid-body ownership
merely to make the feature list resemble Godot.

The current highest-value competitive gaps line up with the architecture order:
correct deterministic/lifetime semantics; canonical reconstitution; persistent
world behavior; public SDK/capability closure; asset/raster/runtime quality;
project build/package; structured diagnostics; authored orchestration; and
multiplayer/multiview maturity. Presentation/UI/audio gaps should be filled from
real game pressure rather than replacement-framework campaigns.

## Controlled-character work is no longer a roadmap gate

The first major decision-authority convergence has landed. Remaining
controlled-character work is a bounded residual-kernel/control integration
problem and should proceed when it closes real duplicate authority or supports a
customer. It is not a prerequisite for every open-world or architecture slice.

## Ambition build order

Use [`game/open-world-roadmap.md`](game/open-world-roadmap.md) and
[`game/systemic-progression.md`](game/systemic-progression.md).

The build order remains **world first, story over reality**. Prove a large
persistent world, traversal/capabilities, items/mechanisms, persistent/spawned
actors, and save/load coherence before relying on a linear story spine to provide
meaning. Story should consume the same world facts rather than substitute for
them.

## Bevy package direction

Durable package/decomposition doctrine is in
[`../architecture/package-and-capability-boundaries.md`](../architecture/package-and-capability-boundaries.md).
A reusable domain owns its vocabulary and plugin registration, depends downward,
and is testable in a small host. Extract or publish independently only when the
API has a real game-independent customer.

## Ambiguity policy

Focused plans must distinguish settled direction from open design questions. An
agent may investigate an unresolved question when a concrete slice requires it,
but should not turn an under-specified product choice into architecture merely to
continue execution. Genuine maintainer choices go to
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).
