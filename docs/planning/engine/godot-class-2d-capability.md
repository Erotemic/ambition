# Godot-class 2D engine capability and expressiveness

**State:** OPEN — cross-program Engine 1.0 acceptance map.

This document defines what it means for Ambition's engine to become credible
competition for a Godot-class 2D engine. It is intentionally **not an editor
parity roadmap**.

The comparison is about what a game can express, how reliably and efficiently it
runs, how cleanly capabilities compose, how well failures can be explained, and
whether another project can use those capabilities through supported engine
surfaces.

Ambition's preferred authoring model is LLM-first and semantic. A visual editor,
asset browser, inspector panel, scene tree UI, visual scripting graph, or
one-click export dialog is not required merely because another engine exposes
one. Human-facing visual tools remain useful where visual/manual manipulation is
the actual task; they should consume the same underlying semantic engine
surfaces.

Durable authoring doctrine:
[`../../concepts/agent-native-authoring.md`](../../concepts/agent-native-authoring.md).

## Competitive target

The Engine 1.0 wedge is **serious 2D games**, especially platformers, action
games, systemic/open-world games, and deterministic multiplayer-capable games.
The architecture should not poison later 3D work, but 3D feature breadth is not a
1.0 parity requirement.

A Godot-class competitor should let a developer or LLM agent build and ship a
substantial 2D game without repeatedly inventing private infrastructure for
ordinary engine concerns.

The target has seven axes.

### 1. Expressiveness

Games can express rich bodies, movement laws, interactions, actions, combat,
world mechanisms, persistent occurrences, UI, cameras, animation, VFX, audio,
multiplayer, AI, and authored orchestration through coherent engine/domain
contracts.

A new game mechanic may add a new domain/plugin or authored vocabulary. It
should not require editing an application-shaped central dispatcher merely to
participate in the engine.

### 2. Simulation integrity

Authoritative state has explicit ownership, deterministic composition, stable
semantic identity where required, inspectable construction/reconstitution, and
rollback/replay semantics where installed.

Headless, visible, replay, AI, and networked hosts consume the same simulation
truth.

This is an area where Ambition should be **stronger** than a general-purpose
engine rather than merely match commodity capability.

### 3. Runtime efficiency

Representative games meet explicit CPU, GPU, memory, loading/hitch, and asset
residency budgets on their supported target profiles.

Efficiency is measured at the actual host/profile. Headless simulation timing is
not used as evidence for rendered performance. System count, crate count, or
abstraction count are not performance proxies.

### 4. Capability completeness

The engine has a supported path for the ordinary 2D game domains listed below.
The path may be:

- an Ambition-owned specialized capability;
- direct Bevy functionality;
- a selected Bevy ecosystem plugin;
- a provider/backend adapter.

"Supported" does not mean Ambition must reimplement Bevy or Godot functionality.
It means an external game has a clear, tested composition story without reaching
through private Ambition internals.

### 5. Extensibility and composability

Rust/Bevy plugins, providers, prepared authored rules, semantic commands, and
public SDK surfaces form an explicit extension ladder. Optional capabilities do
not drag unrelated domains into minimal consumers.

Runtime scripting is not required merely because Godot has GDScript. The
requirement is equivalent **behavioral expressiveness**, not syntax parity.

### 6. Inspectability and diagnosability

An agent or developer can ask the engine what exists, what owns it, why a rule
or action did or did not happen, where a reference came from, what state
participates in rollback/persistence, and where frame/build time went.

Structured inspection is a first-class engine product, not a substitute log
stream for a missing GUI inspector.

### 7. Agent-first development efficiency

A capable LLM agent can discover engine vocabulary, inspect existing content,
make semantic changes, validate them, run representative tests, inspect results,
and produce a build/package without learning repository archaeology or manually
operating an editor.

The competitive question is **time and reliability from intent to validated game
change**, not number of editor panels.

## Explicit non-goals for Engine 1.0

Engine 1.0 does not require:

- a Godot-style integrated scene editor;
- a visual inspector as the primary state-authoring interface;
- a built-in asset browser merely for parity;
- visual scripting;
- a GDScript/Lua/Wasm clone without a demonstrated deployment/modding need;
- one-click export GUI parity when a reliable noninteractive command is better
  for humans, agents, and CI;
- a universal general-purpose rigid-body engine if Bevy/ecosystem physics covers
  real customers adequately;
- a replacement renderer, UI framework, audio engine, asset server, or windowing
  layer when the selected Bevy path meets the requirement;
- general-purpose 3D breadth;
- recreating another engine's scene-tree ontology as Ambition's runtime model.

These exclusions are part of the strategy. Ambition should spend engineering
budget on simulation quality, 2D game expressiveness, composition, agent-native
authoring, and runtime efficiency rather than recreating a human-editor product
whose underlying semantics are already available through better programmatic
surfaces.

## Bevy-native ownership rule

Godot is a useful completeness reference; Bevy is the substrate Ambition
actually runs on.

Use Bevy directly for generic engine mechanisms unless Ambition needs stronger
platformer/systemic semantics. Add an Ambition contract when one of these is
true:

- deterministic simulation requires explicit policy;
- gameplay semantic identity differs from an ECS entity or asset handle;
- lifecycle/reconstitution/persistence requires stronger ownership;
- a provider needs a stable backend-neutral authored contract;
- headless/network/replay behavior requires presentation independence;
- multiple games repeatedly need the same higher-level 2D policy;
- raw Bevy capability is too low-level to be a usable public game contract.

Do not wrap Bevy merely to rename it.

Durable package doctrine:
[`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md).

## Engine 1.0 capability matrix

The matrix is an acceptance map, not an instruction to open every row as a
campaign. A row becomes implementation work when Ambition or another serious
customer demonstrates a concrete gap.

| Capability | Current Ambition posture | Engine 1.0 bar | Current owner |
|---|---|---|---|
| **2D body movement and collision** | Specialized platformer movement, slopes, one-ways, moving surfaces, blink/fast movement, contacts and collision contracts are substantial strengths | One rich reusable body path; high-speed and ordinary platformer customers pass representative conformance scenarios; no player/enemy/demo movement forks | `collision-and-ccd.md`, movement ADRs/concepts, actor kernel |
| **Triggers, areas, surfaces and world interactions** | Portals, zones, gravity/surface semantics, encounters and world mechanisms exist across domains | Typed interaction/overlap semantics compose without one central taxonomy; authored mechanisms can participate through provider/domain seams | spatial architecture, interaction/domain plans |
| **General 2D physics integration** | Ambition owns platformer-specific kinematics rather than a general rigid-body engine | Real rigid-body/joint customers have a supported Bevy/ecosystem path; Ambition does not block integration with unnecessary parallel physics authority | trigger-based; no standing rewrite |
| **World/room composition** | LDtk lowering, prepared world records, rooms, transitions and transactional construction are strong | Backend-neutral prepared world semantics; canonical construct/reconstitute path; reusable authored composition solves real repeated structures without prefab mimicry | `construction-and-reconstitution.md`, `ldtk-authoring-and-world-tools.md`, `reusable-authored-world-composition.md` |
| **Persistent/open-world state** | Session/provenance/persistence foundations exist; residency and occurrence semantics remain active | World existence, residency, simulation and visibility are independent; durable occurrences reconstruct coherently across unload/reload/save | `open-world-runtime-and-residency.md`, persistence/lifetime plans |
| **Actions, abilities and gameplay orchestration** | Semantic actions, movesets, participant actions and authored provider vocabulary exist; general authored verb/relationship composition remains incomplete | Common game behavior can be composed from typed commands/facts/rules; Rust adds vocabulary, content composes it; no app-shaped central behavior switch | `participant-action-system.md`, `authored-gameplay-logic-and-orchestration.md`, `extension-model.md` |
| **2D rendering** | Sprites, atlases/materials, parallax, cameras, VFX and custom presentation paths exist over Bevy | Several materially different games can express their 2D visual language through supported Bevy/Ambition/provider seams; missing lights/shadows/custom-draw/shader features are filled only when a real visual target requires them | `render-animation-and-vfx.md`, renderer/system docs |
| **Animation** | Sprite animation, authored clips, move/portrait animation and domain presentation policy exist | Animation selection is semantic/provider-owned, supports ordinary state/one-shot/loop/transition needs, and remains presentation-only under rollback/headless execution | `render-animation-and-vfx.md`, character authoring |
| **VFX and particles** | Built-in semantic effect requests, authored sprite FX and procedural particles exist | Common effects are expressible without gameplay depending on render entities; richer providers remain optional; persistent emitters reconcile from semantic source state | `render-animation-and-vfx.md` |
| **Cameras and views** | Strong camera/reference-frame concepts and per-view foundations exist | One or many local views can compose tracking, constraints, transitions, shake/impulse and quality policy without conflating camera with control authority | `multiplayer-and-multiview.md`, camera system docs |
| **Audio and music** | Audio/SFX/music systems and generated content pipelines exist | Games can express cues, music routing, spatial/nonspatial playback and product-required mix policy through semantic APIs; confirmed irreversible effects respect netcode boundaries | audio/VFX system docs; product pressure |
| **UI and text** | Shell, menus, settings, dialogue, inventory UI and navigation exist | Participant/view-aware runtime UI supports controller/touch/mouse, adaptive layout, text, settings and required accessibility/localization targets using Bevy UI rather than a second widget engine | `ui-localization-and-accessibility.md`, input/UI systems |
| **Input and control** | Keyboard/gamepad/touch, semantic actions, participants, control authority and prompts exist | N local devices/participants route predictably through one semantic action/control model; rebinding/context/prompt policy is supported and testable | `participant-action-system.md`, multiplayer/control docs |
| **Assets and resource readiness** | Asset manager, provider preparation, generated assets, loading transactions and packaging exist | Stable semantic identity, readiness/preparation, source provenance, render materialization, residency budgets and target packaging are inspectable and bounded | `asset-preparation-and-residency.md`, asset concept |
| **Hot reload / fast content iteration** | LDtk hot reload and generated-content workflows exist in focused areas | Content families that benefit from live iteration have one source-of-truth reload path with validation; no requirement for universal arbitrary-state hot reload | authoring/tools and system docs |
| **Persistence/save compatibility** | Persistence crate and provenance/reconstitution model exist | Durable domain facts are versioned intentionally and restored through canonical construction; saves do not serialize accidental ECS topology as public format | construction/open-world/persistence plans |
| **Navigation and AI** | Platformer reachability, fighter brains and deterministic behavior machinery exist | Navigation understands body capabilities and dynamic platformer mechanics; AI consumes simulation observations/facts through typed action surfaces | `platformer-navigation-and-reachability.md`, world facts/agentic plans |
| **Multiplayer/netcode** | Local-N foundations and GGRS rollback architecture exist | Local multiplayer is ordinary engine usage; online transport/lifecycle can be installed without a multiplayer-only ontology; deterministic state contracts remain the same | `netcode.md`, `multiplayer-and-multiview.md` |
| **Headless simulation/testing** | Strong sim harness, gameplay traces and no-window hosts are differentiators | Public build/step/reset/observe workflows use the same simulation contracts as visible hosts; scenario tests can populate dynamic authoritative state | simulation/headless/test system docs |
| **Diagnostics and profiling** | Tracing, gameplay trace, preparation diagnostics and domain inspection pieces exist | Structured `describe`, provenance, cause/why-not, state/accounting, replay and performance reports are available through supported APIs/CLI; GUI is optional | `inspection-diagnostics-and-workbench.md` |
| **Platform/host support** | Desktop, web, Android/touch, controller, headless and Steam Deck-oriented workflows exist at different maturity | Supported target profiles are explicit, reproducible, feature-composition tested, and have representative artifact/runtime smoke checks | `project-build-and-distribution.md`, platform concept |
| **Packaging/export** | Cargo/project scripts and asset packaging exist; external project packaging remains immature | One supported noninteractive path takes a clean project from configured providers/capabilities to testable release artifacts per supported target | `project-build-and-distribution.md`, public SDK |
| **Public SDK** | Semantic facade is narrowing; external/headless consumer evidence exists | A different 2D game can compose ordinary engine capabilities without importing implementation crates or reading Ambition migration history | `public-sdk-1.0.md` |
| **Extensions/plugins/providers** | Bevy plugins/providers are primary; capability boundaries continue to improve | New behavior/content domains can contribute vocabulary, preparation, systems, diagnostics and optional dependencies through supported extension seams | `extension-model.md`, capability composition |
| **LLM-first authoring** | Strong LDtk, sprite, music, SFX and semantic preparation tooling already exists | Agent can discover, inspect, mutate, validate, test, explain and package common game changes without manual editor operation or raw-format archaeology | `authoring-and-tools.md` |
| **Runtime/build efficiency** | Measurement program has ruled out several generic CPU theories and exposed raster/materialization/build bottlenecks | Representative target profiles have explicit budgets; regressions are attributable; build/test iteration is fast enough for agentic development and bounded in memory/artifact growth | `performance-and-iteration.md`, `project-build-and-distribution.md` |

## Where Ambition should differentiate rather than merely match

A credible competitor does not need to beat Godot on every commodity feature.
It should have several areas where choosing Ambition is technically compelling.

### Deterministic systemic simulation

Rollback, replay, headless stepping, stable semantic identity, explicit lifetime,
and deterministic composition should become an integrated engine strength rather
than a network-mode add-on.

### Platformer/action specialization

Movement, collision, surfaces, one-ways, high-speed traversal, body capabilities,
actions, combat, camera reference frames, and platformer navigation should be
substantially more coherent than assembling them ad hoc from general engine
nodes/components.

### Persistent systemic worlds

World occurrence, residency, reconstitution, body/item custody, navigation and
reactive actors should support a large coherent 2D world without a game inventing
a second persistence/lifecycle framework.

### Agent-native engine operation

An LLM should have a better supported path than screen-driving an editor:
structured vocabulary discovery, semantic queries, transactional changes,
provenance, dry runs, deterministic test scenarios, concise visual review
products, and noninteractive packaging.

### Rust/Bevy composition and static boundaries

Where Rust's type/dependency system can make invalid ownership or composition
harder, use that advantage. A minimal game should not inherit an application-sized
engine graph because the flagship uses many capabilities.

## Competitive authoring model

Godot's integrated editor is evidence that fast feedback and discoverability are
important. Ambition should meet those needs through a different primary surface.

The LLM-first loop is:

```text
intent
  -> discover engine/content vocabulary
  -> inspect current semantic state
  -> plan/dry-run
  -> edit authored source or invoke semantic mutation
  -> validate/prepare
  -> build/run targeted scenario
  -> inspect structured result + concise visual artifact
  -> iterate
  -> package/export noninteractively
```

The performance of this loop is an engine-product concern. Slow compilation,
unclear diagnostics, hidden generated dependencies, giant test gates, or APIs
that require reading internal implementation code are competitive defects even
when runtime frame time is excellent.

A GUI may be added when it improves a real visual/manual task. It should be a
frontend over the same semantics and diagnostics, not a second source of truth.

## External-engine reference baseline

Godot is used here as a completeness reference, not an ontology to copy.
Relevant official references include:

- feature inventory: <https://docs.godotengine.org/en/stable/about/list_of_features.html>
- 2D capability overview: <https://docs.godotengine.org/en/stable/tutorials/2d/index.html>
- noninteractive/headless/build/export workflows:
  <https://docs.godotengine.org/en/stable/tutorials/editor/command_line_tutorial.html>
- native extension model:
  <https://docs.godotengine.org/en/stable/engine_details/engine_api/gdextension/what_is_gdextension.html>

These references should periodically challenge missing engine capability. They
should not create tasks whose only payoff is matching another engine's UI or
internal vocabulary.

## Acceptance customers

Engine capability claims should be pressure-tested by materially different
customers.

- **Ambition** — persistent systemic/open world, body/capability progression,
  agent-authored content, reactive actors, save/reconstitution.
- **Super Smash Siblings** — deterministic combat, local-N input, rollback,
  high-feedback presentation and strict movement/action semantics.
- **Sanic** — very high-speed movement/collision and camera stress.
- **Super Mary-O** — conventional authored platformer level composition and
  classic movement/mechanism expectations.
- **Hollow Lite** — encounters, bosses and richer combat/presentation patterns.
- **TwinTrack** — independent observer/reference frames and multiview.
- **external/minimal consumer** — public SDK, dependency closure, project build,
  diagnostics and packaging outside the flagship workspace topology.

A capability is not proven reusable merely because Ambition can reach an
internal crate that implements it.

## Engine 1.0 competitive gates

Architecture programs may close independently, but Engine 1.0 is not credible as
an engine product until the following statements are true.

### Gate C1 — ordinary 2D game completeness

At least one supported path exists for movement/collision, world composition,
rendering/animation/VFX, audio, UI/text, input, assets/loading, persistence,
cameras, diagnostics, and build/package on the declared target profiles.

The path may be Bevy-native or provider-backed; it need not be Ambition-owned.

### Gate C2 — semantic extension completeness

A different game can add domain vocabulary/content/behavior through the public
plugin/provider/SDK model without editing a central Ambition-specific switch.

### Gate C3 — deterministic/headless integrity

The visible host is not a privileged source of simulation truth. Representative
features can run headlessly, be inspected, and where applicable rewind/replay
without hidden presentation/process state changing outcomes.

### Gate C4 — runtime-quality budgets

Representative target hardware has recorded CPU/GPU/frame-time, hitch/loading
and memory/residency expectations. Known weak profiles have quality controls that
preserve gameplay readability.

### Gate C5 — external project usability

A clean external/minimal game can compose supported capabilities, diagnose
configuration/content failures, run tests, and produce a release artifact
without importing implementation crates.

> ⭐ **MEASURED 2026-09-03 against the three out-of-workspace consumers, and the
> "without importing implementation crates" clause splits them two to one.**
>
> | consumer | `[dependencies]` reach |
> |---|---|
> | `fixtures/minimal_game` | `ambition_platformer2d` only |
> | `fixtures/external_consumer` (outlander) | `ambition_platformer2d` only |
> | `examples/capability_demo` | **four implementation crates** — `ambition_content_pack`, `ambition_causal`, `ambition_input`, `ambition_platformer2d_core` |
>
> ⚠ **The third is not obviously a violation, which is why it is recorded rather
> than filed as a gap.** The capability demo exists to prove that a capability
> can register its own content schema without editing a central enum — its own
> header says *"`ambition_content_pack` never heard of pulses; the schema is
> registered by the capability that owns it"* — and that is a claim about
> EXTENSION, which the facade does not currently surface. ⇒ So C5 has two
> readings and the page should say which it means: if a game extending the
> engine must also go through the facade, this demo is the gap; if extension is
> allowed to name the crates it extends, C5 is about CONSUMPTION and the two
> fixtures already satisfy it.
>
> Either way the fixtures are the evidence for C5 and the demo is not — and it
> is the demo whose lockfile the abilities carve had to update, which is how the
> distinction surfaced.

### Gate C6 — agent-first operability

A capable LLM agent can perform a representative cross-domain game change using
supported discovery/inspection/mutation/validation/test/package surfaces without
manual editor operation and without being taught repository-specific internal
crate topology.

### Gate C7 — differentiated flagship proof

Ambition exercises the persistent-world, deterministic, platformer-specialized
and agent-native capabilities deeply enough that the engine is not only a thin
re-export layer over Bevy.

## How this plan creates work

This is a cross-program acceptance map, not a second queue.

When a real game/customer exposes a gap:

1. confirm whether Bevy or a maintained ecosystem plugin already supplies the
   generic mechanism;
2. identify the missing semantic/public/composition layer, if any;
3. route the work to the focused owner listed in the matrix;
4. define one real customer acceptance case;
5. promote only the executable slice to `queue.md`;
6. update this matrix only if the capability posture changed.

Do not open a campaign because a Godot feature list contains a noun.

## Current highest-value completeness gaps

These are the broad gaps currently most likely to block the competitive target.
Their execution order still comes from [`../roadmap.md`](../roadmap.md) and
[`../queue.md`](../queue.md).

1. **authoritative-state/lifetime/reconstitution correctness** — specialized
   simulation is only a strength if it remains coherent under real lifecycle and
   dynamic-population pressure;
2. **persistent/open-world semantics** — Ambition's flagship differentiator still
   needs complete occurrence/residency/reconstruction behavior;
3. **public SDK and capability closure** — external games need semantic surfaces,
   not implementation-crate reach-ins;
4. **asset materialization/residency and weak-GPU quality** — demonstrated
   user-visible performance pressure;
5. **project build/package/export path** — engine capability must become a usable
   external project, not only a workspace composition;
6. **structured inspection/provenance/why-not diagnostics** — required for both
   LLM-first operation and serious debugging;
7. **authored gameplay orchestration** — the major expressiveness gap between
   strong authored nouns and composing verbs/relationships over time;
8. **remaining multiplayer/multiview/control maturity** — important proof that
   participant/view/world semantics are genuinely general;
9. **presentation/UI/audio feature completeness from real customers** — fill
   demonstrated gaps without starting replacement frameworks.

## What should not become a parity campaign

Do not turn the following into standing Engine 1.0 work without a concrete game
or deployment requirement:

- integrated editor replication;
- visual scene graph editing;
- visual scripting;
- broad 2D light/shadow feature cloning absent an actual visual target;
- arbitrary runtime scripting;
- general rigid-body/joint abstraction owned by Ambition;
- iOS/console support before a real distribution customer and prerequisites;
- plugin marketplace/asset library infrastructure;
- editor-theme/plugin APIs;
- general 3D parity.

The goal is a stronger **engine**, not a look-alike editor product.
