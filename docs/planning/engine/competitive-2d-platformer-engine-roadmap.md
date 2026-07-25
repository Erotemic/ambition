# Competitive 2D platformer engine master plan

**Status:** master plan for engine/runtime competitiveness
**Scope:** 2D platformer and action-platformer engine capability on Bevy; not an editor roadmap
**Authority:** expands [`../vision.md`](../vision.md) into a cross-track capability and sequencing plan. [`../tracks.md`](../tracks.md) remains the live executable queue, and focused plans own implementation-level design once a campaign opens.

---

## 1. The goal, in Jon's words

Ambition should become a game engine on the level of Unity, Godot, and Unreal
**for 2D platformers**, built on Bevy and Rust. It should be ECS-native,
composition-first, plugin-oriented, elegant, and beautiful. The goal is engine
quality, not reproduction of those engines' editors, visual scripting systems,
asset browsers, or general-purpose 3D breadth.

Ambition should support materially different platformers without turning each
new mechanic or game into a new core code path. The reusable engine should own
platformer semantics that ordinary games otherwise have to reinvent:

- deterministic character motion and collision;
- actions, abilities, combat, and temporal ownership;
- contacts, triggers, surfaces, and moving-world interaction;
- rollback-safe simulation and headless execution;
- deterministic construction, lifecycle, and persistence boundaries;
- platformer-specific observation, replay, diagnosis, and testing.

At the same time, Ambition must not fight Bevy. Bevy is already a modular game
engine with ECS, schedules, plugins, assets, scenes, rendering, cameras, input,
audio, UI, diagnostics, and cross-platform hosts. Ambition should compose with
those facilities, extend them where platformer-specific policy is needed, and
replace them only when a demonstrated engine requirement cannot be satisfied by
Bevy or a suitable Bevy ecosystem component.

The target is:

> A Bevy-native, deterministic, composable 2D platformer engine whose movement,
> simulation integrity, headless execution, rollback, and platformer-specific
> diagnostics are stronger than the general engines, while its ordinary runtime
> surface is complete enough to build and ship several substantially different
> platformers without constructing a second private engine inside each game.

This plan is intentionally a master plan. It describes the complete destination,
the capability gaps that matter, and the dependency order between campaigns. It
does not make every later campaign active today.

---

## 2. What "compete" means

"Competitive" has three levels. Keeping them separate prevents the plan from
making every desirable shipping feature a prerequisite for declaring the core
architecture successful.

### Level 1 — competitive platformer engine core

The simulation and extension architecture is competitive when:

- platformer bodies, actions, contacts, time, damage, and lifecycle have coherent
  reusable contracts;
- authoritative state is mechanically accounted for under rollback, checksums,
  pause, hitstop, and reconstruction;
- human, AI, replay, RL, and network-controlled participants converge at stable
  authoritative boundaries;
- content and game plugins compose through explicit engine seams rather than
  private scheduler conventions;
- headless and visible execution use the same simulation truth;
- the engine can explain action, movement, contact, damage, and rollback outcomes.

This is the first target. It is mostly about Ambition's own platformer semantics.

### Level 2 — competitive shippable runtime

The runtime is competitive when games can also rely on dependable:

- resource identity, readiness, failure reporting, and target packaging;
- participant/device input mapping and rebinding;
- provider-specific animation and camera policy;
- rendering, audio, UI, settings, and persistence integration;
- supported desktop, web, Android/mobile, touch, controller, and headless host
  profiles;
- performance budgets and artifact-level smoke tests.

Most of these capabilities should be compositions over Bevy, not replacements
for Bevy.

### Level 3 — mature reusable engine

The engine is mature when:

- several substantially different games exercise the same contracts;
- the public provider surface is coherent enough to document and version;
- debugging and scenario tooling make engine failures inspectable rather than
  mysterious;
- advanced optional capabilities, such as local-N fighters, online transport,
  unusual gravity, exploration persistence, or imported authoring backends, can
  be added without deforming the core;
- engine upgrades do not require every game to understand internal implementation
  details.

Level 3 is evidence of maturity, not the definition of every core design choice.

### Explicit non-goals

This plan does not require:

- a Unity-, Godot-, or Unreal-style editor;
- a general 3D engine;
- a universal rigid-body simulator;
- a general gameplay scripting language;
- a replacement renderer, audio engine, UI framework, asset server, or windowing
  layer when Bevy already provides the needed substrate;
- one universal event type for every kind of collision, overlap, combat hit, and
  proximity query;
- online multiplayer as a requirement for every provider.

---

## 3. Bevy-native composition doctrine

Ambition is a specialized engine built **on** Bevy, not a competing general
engine embedded beside Bevy.

Bevy already provides a modular ECS engine, schedules and plugins, typed assets
and asynchronous loading, scenes, cameras, sprite and 2D rendering, UI, audio,
input, diagnostics, and cross-platform application support. Ambition should keep
those concepts visible and usable rather than hiding them behind equivalent
Ambition-owned abstractions.

### 3.1 Ownership split

| Domain | Bevy should own | Ambition should own |
|---|---|---|
| ECS and scheduling | entities, components, resources, queries, schedules, plugins, states | semantic simulation phases, deterministic ordering constraints, authoritative-state rules |
| Assets | `AssetServer`, typed assets, loaders, handles, hot reload, load state | stable gameplay/content identifiers, prepared-content validation, readiness policy, target packaging checks |
| Rendering | cameras, sprites, atlases, materials, render graph, UI rendering | platformer read models, sorting policy where needed, presentation facts, platformer effects and visual-quality policy |
| Input and hosts | keyboard, gamepad, touch, windows, platform events | participant ownership, semantic actions, contexts, deterministic `ControlFrame` production |
| Audio | asset playback and Bevy audio entities, or a selected Bevy audio plugin | semantic cues, confirmed-frame policy, routing conventions, provider music policy |
| UI | Bevy UI layout/rendering and suitable ecosystem tooling | game shell semantics, navigation policy, prompts, settings bindings, simulation separation |
| Scenes and hierarchy | Bevy entities, parent/child relationships, scenes where useful | immutable prepared content, deterministic construction plans, stable simulation identity, lifecycle ownership |
| Diagnostics | Bevy diagnostics, tracing, schedule inspection, ecosystem inspectors | platformer-specific scenario traces, movement/contact/action/rollback explanation |

### 3.2 Decision rules

1. **Use Bevy directly when the problem is generic.** Do not create an Ambition
   wrapper merely to rename a Bevy component or service.
2. **Add an Ambition contract when platformer semantics require one.** Stable
   simulation identity, rollback authority, action timing, movement laws, contact
   attribution, room construction, and confirmed effects are legitimate engine
   responsibilities.
3. **Prefer adapters over replacements.** For example, Ambition may publish a
   camera policy into Bevy camera components; it should not create a parallel
   camera/render world.
4. **Evaluate ecosystem plugins before building generic infrastructure.** The
   choice must still satisfy determinism, host compatibility, maintenance, and
   dependency-quality requirements.
5. **Keep simulation independent of presentation implementation.** Bevy rendering,
   audio, and UI may be replaced or omitted by a headless host without changing
   authoritative gameplay.
6. **Avoid lowest-common-denominator abstraction.** Bevy-native composition means
   accepting Bevy's ECS and plugin model rather than inventing a backend-neutral
   framework above it.

---

## 4. Competitive capability matrix

The general engines establish expectations for reusable character bodies,
collision and trigger objects, action-based input, animation policy, cameras,
resource loading, reusable object composition, debugging, and broad host support.
They do not dictate Ambition's internal object model.

Official baseline references:

- Godot `CharacterBody2D`, `Area2D`, `AnimationTree`, `InputMap`, and `Camera2D`:
  <https://docs.godotengine.org/en/stable/classes/class_characterbody2d.html>
- Unity Input System and Cinemachine:
  <https://docs.unity3d.com/Packages/com.unity.inputsystem@latest/>
  and <https://docs.unity3d.com/Packages/com.unity.cinemachine@latest/>
- Unreal Paper 2D and Enhanced Input:
  <https://dev.epicgames.com/documentation/unreal-engine/paper-2d-overview-in-unreal-engine>
  and <https://dev.epicgames.com/documentation/unreal-engine/enhanced-input-in-unreal-engine>
- Bevy engine capabilities and examples:
  <https://bevy.org/>, <https://bevy.org/examples/>, and
  <https://docs.rs/bevy/latest/bevy/asset/struct.AssetServer.html>

| Capability | General-engine baseline | Bevy foundation | Ambition today | Required outcome | Level |
|---|---|---|---|---|---|
| Platformer character motion | configurable character bodies and collision queries | ECS, transforms, time, schedules | strong specialized movement kernels, slopes, one-ways, moving platforms, multiple movement identities | preserve and generalize one rich body path without player/actor forks | 1 |
| Deterministic simulation and rollback | not normally the organizing center | deterministic-friendly ECS and fixed schedules, but no automatic game contract | strong GGRS/headless/stable-ID foundation; participation remains manually fallible | authoritative state and systems are mechanically accounted and resimulation-safe | 1 |
| Actions and abilities | input actions plus game/animation state machines | input events, states, ECS systems | the slot-to-action scheme, shared resolver, prompts, and `MovePlayback` are substantially landed; participant ownership and temporal action state still have parallel residue | finish participant/context routing and add shared temporal ownership without replacing the landed action seam | 1 |
| Contacts, areas, and surfaces | bodies, layers/masks, areas/triggers, contact results | ECS and third-party physics/spatial options; Ambition has custom solver | canonical `Contact`, `SweepSample`, cast registry, geometry identity, and swept-trigger doctrine exist; enforcement and some consumers remain incomplete | finish and enforce the existing doctrine, migrate ad hoc readers, and share causal/filtering conventions without one universal implementation | 1 |
| Actor model and combat | reusable components and game-specific scripts | ECS composition | unusually strong unified-body and combat primitives | finish one-body enforcement and remove remaining controller/content forks | 1 |
| Construction and lifecycle | scenes/prefabs and instantiated hierarchies | entities, hierarchy, scenes, assets | immutable prepared content and transactional room planning substantially landed | one canonical deterministic construction/lifecycle boundary for supported origins | 1 |
| Provider/plugin composition | packages, plugins, scenes, components | first-class plugins and modular crates | substantial provider/runtime/host split; some leaf registration remains centralized | games compose capabilities through supported plugins and data seams | 1–2 |
| Resources and packaging | asset databases, dependency tracking, target builds | typed assets, async loading, handles, hot reload | substantial asset manager/load infrastructure; cross-target failures have occurred | use Bevy assets while adding exact content identity, readiness, failure, and artifact validation | 2 |
| Participant input | action maps, devices, rebinding, local players | keyboard, gamepad, touch, input events | deterministic `ControlFrame`, participant plans, touch/controller work | device/context mapping produces participant-local semantic frames through one route | 2 |
| Animation policy | state machines, blend trees, flipbooks | sprite sheets, animation facilities, ECS state | substantial animation playback with centralized hardcoded selection pressure | provider-owned presentation policy driven by authoritative facts | 2 |
| Camera policy | target following, constraints, blending, shot composition | `Camera2d`, transforms, render layers | strong camera snapshot/resolver concepts | thin policy selection, target grouping, transition, and impulse composition over Bevy cameras | 2 |
| Rendering | sprites, tilemaps, particles, lights, postprocessing | capable 2D renderer, sprites, atlases, materials, UI, render graph | substantial renderer and VFX crates | audit actual game needs; fill platformer-specific integration gaps rather than rebuild Bevy | 2 |
| Audio | playback, mixers/buses, spatial sound, music systems | core audio plus ecosystem options | multiple Ambition audio/SFX crates and confirmed-effect requirements | semantic cue contract and selected Bevy-native routing implementation | 2 |
| UI/settings | runtime UI, navigation, localization, accessibility settings | Bevy UI and ecosystem tooling | game shell, menus, UI navigation, settings, dialogue, inventory UI exist | consolidate only shared game-shell policy; continue using Bevy UI primitives | 2 |
| Persistence | game-specific saves, object identity, schema evolution | ECS/scene/serde ecosystem | persistence crate and session reconstruction concepts exist | intentional versioned domain state rebuilt through canonical construction | 2 |
| Diagnostics | inspectors, profilers, collision/debug views | tracing, diagnostics, schedule tooling, inspectors | gameplay trace, headless tests, debug visualization exist | platformer-specific scenario and frame explanation composed with Bevy diagnostics | 1–3 |
| Host/platform support | desktop, web, mobile, controllers | broad cross-platform engine support | desktop, web, Android/mobile, touch, controller, and headless paths exist | explicit supported profiles, artifact smoke tests, and measured budgets | 2 |
| Multiplayer | packages and engine-specific networking options | ecosystem networking; no universal policy | GGRS rollback foundation and local/online plans | local-N as an acceptance need; online transport optional and provider-driven | 3/optional |

The matrix distinguishes **confirmed architectural gaps** from generic categories
that Bevy already substantially supplies. Rendering, audio, and UI are therefore
not invitations to build replacements. They are integration and completeness
reviews whose output may be "use the existing Bevy path and delete Ambition
special cases."

---

## 5. What Ambition already has and must protect

### 5.1 Specialized movement as an engine capability

Primary owners include:

- `crates/ambition_engine_core/src/movement/`
- `crates/ambition_engine_core/src/cast.rs`
- `crates/ambition_platformer_primitives/src/body/`
- [`unified-movement-kernel.md`](unified-movement-kernel.md)
- [`collision-and-ccd.md`](collision-and-ccd.md)

Ambition already treats high-quality platformer motion as reusable engine
machinery rather than per-game controller code. This is a real advantage over
general engines and should not be weakened by a generic rigid-body abstraction.

### 5.2 Deterministic, headless, rollback-oriented execution

Primary owners include:

- `crates/ambition_runtime/src/rollback/`
- `crates/ambition_sim_harness/`
- `crates/ambition_sim_view/`
- `crates/ambition_gameplay_trace/`
- [`netcode.md`](netcode.md)
- [`headless-verification.md`](headless-verification.md)

The foundation is strong. Recent defects show that participation and ownership
are still too easy to get wrong, not that the capability is absent.

### 5.3 Unified actors and reusable combat machinery

The one-body doctrine, movement entry unification, movesets, hit volumes, damage,
knockback, hitstop, health, and actor lifecycle already provide a deeper
platformer/action framework than a general engine supplies by default.

Primary owners include:

- `crates/ambition_actors/`
- `crates/ambition_characters/`
- `crates/ambition_combat/`
- [`combat-model.md`](combat-model.md)
- [`character-actions.md`](character-actions.md)

The slot-to-action scheme, shared resolver, prompt projection, and
`MovePlayback` timeline are already substantial foundations. The next work is to
finish participant ownership and contexts, remove remaining activation residue,
and add temporal action ownership where the existing move/maneuver timelines do
not yet express it. This is consolidation, not a new action or combat engine.

### 5.4 Prepared content and transactional construction

Primary owners include:

- `crates/ambition_platformer_provider/`
- `crates/ambition_world/`
- `crates/ambition_actors/src/world/rooms/`
- `crates/ambition_actors/src/features/ecs/spawn/`
- [`immutable-content-and-transactional-construction.md`](immutable-content-and-transactional-construction.md)

This campaign is substantially advanced. It should be finished and simplified,
not replaced with a Unity-shaped prefab layer.

### 5.5 Existing Bevy-facing runtime systems

Ambition already has meaningful crates for:

- assets and loading;
- rendering and VFX;
- audio and SFX;
- input and touch;
- menus, settings, UI navigation, dialogue, and inventory UI;
- persistence;
- hosts and platform composition.

These areas need targeted audits and consolidation against concrete game and host
requirements. Their existence is evidence against broad greenfield tasks.

---

## 6. Three north stars

### North star A — enforced simulation integrity

A gameplay feature declares its authoritative state, systems, lifecycle, and
facts close to the code that owns them. The engine provides a small explicit
semantic phase graph and mechanically checks participation in rollback,
checksums, pause, hitstop, and reconstruction where applicable.

The desired outcome is not total decentralization. Global semantic ordering
remains an engine responsibility. The desired outcome is that feature authors do
not have to remember several distant registries and informal scheduling rules.

### North star B — one rich actor path from intent to outcome

Human controls, brains, RL, replay, possession, and network input may have
different upstream representations, but they converge at stable authoritative
boundaries. Actions, movement, contact, damage, and lifecycle use one rich actor
path rather than player/enemy/boss/demo forks.

The convergence point should be narrow and semantic. It should not force human
input devices, AI deliberation, and network transport to share irrelevant
upstream details.

### North star C — Bevy-native provider composition, under the design oracle

The standing test is Jon's oracle: *could another platformer be built by ADDING a
provider/content crate, without editing core?* A game provider composes Ambition
platformer capabilities through Bevy plugins, components, resources, assets, and
explicit content seams. Ambition adds platformer contracts while leaving generic
Bevy facilities available.

The oracle judges the end state, so a provider may legitimately reveal a missing
reusable engine capability — that capability is then landed as engine work, in
its own commit, not inlined into the provider's. What must never be required is a
game-named core branch, a private global schedule, a duplicated engine service, or
knowledge of unstable implementation internals.

The oracle has an executable instrument:
[`fixtures/external_consumer/`](../../../fixtures/external_consumer/)
(Outlander) authors a room, character, enemy, recipe, and transition from outside
the workspace through the `ambition` umbrella alone, gated by `external consumer:
outlander` in `scripts/run_tests.py`, with each engine-internal assumption it must
lean on recorded as a named API leak. Tasks 3, 4, 6, and 8 below should each
retire leaks rather than add them.

---

## 7. High-level roadmap

### Phase 0 — close active architecture debt

Finish already-open construction and rollback-registration work before opening a
large parallel redesign. Keep `tracks.md` authoritative for the exact remaining
slices.

### Phase 1 — enforce and prove simulation integrity

Make authoritative participation explicit and add a minimal real-schedule
scenario harness. This creates the safety net required for later semantic
migration.

### Phase 2 — finish participant routing and temporal action ownership

Preserve the landed slot-to-action resolver and `MovePlayback` seams. Complete
participant/context ownership, then converge one bounded action family on shared
temporal ownership only where existing move or maneuver state is insufficient.

### Phase 3 — finish the swept contact and interaction doctrine

Close and enforce the existing `SweepSample` / `Contact` / `cast` design, migrate
remaining ad hoc endpoint readers, and standardize causal identity and filtering
across environmental and combat consumers without forcing one event type.

### Phase 4 — finish provider and construction composition

Complete canonical construction and tighten the provider-facing plugin/content
surface without prematurely freezing a public SDK.

### Phase 5 — complete the shippable runtime by composing with Bevy

Audit resources, input, animation, cameras, rendering, audio, UI, persistence,
and hosts. For each domain:

1. identify what Bevy or a maintained ecosystem plugin already provides;
2. identify the platformer-specific contract Ambition actually needs;
3. delete parallel or redundant Ambition machinery;
4. add only the missing integration, policy, or validation layer;
5. prove the result in representative hosts and games.

### Phase 6 — make the engine inspectable and mature

Add platformer-specific frame explanation, performance budgets, provider-facing
documentation, and acceptance evidence from increasingly different games.

---

## 8. Planning scale and uncertainty

Task estimates use two independent axes.

### Engineering size

- **S — bounded:** one crate or a narrow cross-crate seam; normally a few focused
  changes.
- **M — campaign:** several crates and consumers; requires migration and focused
  acceptance tests.
- **L — major campaign:** changes a central semantic contract across current
  games and engine layers.
- **XL — multi-campaign:** a product area that should be decomposed before active
  execution.

### Design uncertainty

- **Low:** the target contract is already documented and source assumptions are
  well established.
- **Medium:** the direction is clear but consumer pressure may change details.
- **High:** competing concepts exist or the abstraction boundary is not yet
  proven.

These are planning estimates, not duration promises. Focused plans must refine
them when a campaign becomes active.

---

## 9. Task breakdown

### Task 1 — feature-owned simulation authority and rollback participation

**Goal**

Make gameplay-authoritative state and systems difficult to omit from rollback,
checksums, simulation authorization, lifecycle cleanup, or reconstruction.

**Current code to refactor**

- `crates/ambition_runtime/src/rollback/mod.rs::register_engine_rollback_state`
- `crates/ambition_platformer_primitives/src/schedule.rs::SandboxSet`
- runtime plugin assembly in `crates/ambition_runtime/src/lib.rs`
- demo scheduling in `game/ambition_demo_mary_o/` and
  `game/ambition_demo_sanic/`
- `game/ambition_app/tests/rollback_coverage.rs`

**Approach**

1. Move rollback declarations toward owner crates through a neutral registration
   vocabulary that does not invert dependencies into `ambition_runtime`.
2. Require authoritative systems to install into explicit engine semantic phases.
3. Remove gameplay-authoritative `Local<T>` state; use rollback state or derived
   state with a proved reconstruction rule.
4. Extend computed coverage and real resimulation tests to representative demo
   and dynamic-entity populations.
5. Use ordinary plugins, traits, types, and tests before considering a generated
   manifest or macro framework.

**Size:** L
**Design uncertainty:** Medium
**Payoff:** Extremely high. Removes a demonstrated source of rollback, pause,
ordering, and lifecycle defects from every future mechanic.
**Risks:** registration bureaucracy, macro opacity, or a new central manifest
that simply moves synchronization errors.
**Exit criteria:** a feature-owned authoritative component and system are
mechanically accounted, run under the simulation gate, and survive real
rewind/resimulation without edits to a giant runtime list.

---

### Task 2 — minimal deterministic scenario and replay harness

**Goal**

Make full-schedule gameplay failures reproducible under normal stepping and
rollback before building a large debugger vocabulary.

**Current code to extend**

- `crates/ambition_sim_harness/`
- `crates/ambition_gameplay_trace/`
- existing headless/replay tests under `game/ambition_app/tests/`
- `game/ambition_app/tests/rollback_coverage.rs`

**Approach**

1. Add a concise scenario description: composition/provider, room/setup hook,
   participant input frames, tick count, selected observations/facts, optional
   rollback window, and expected invariant/checksum.
2. Run the same scenario in ordinary headless stepping and rewind/resimulation.
3. Allow subsystems to attach typed opaque facts without prematurely defining
   the final action/contact/debug schemas.
4. Persist small failing fixtures where they capture architectural invariants,
   not unpolished feel values.
5. Defer interactive or rendered inspection to Task 12.

**Size:** M
**Design uncertainty:** Medium
**Payoff:** Extremely high. Makes later refactors safer and turns rollback claims
into executable evidence.
**Risks:** brittle golden snapshots, excessive trace volume, or designing the
future debugger before semantic contracts stabilize.
**Exit criteria:** representative damage, transition, action, and contact
scenarios can be stepped, rewound, checksum-compared, and asserted through the
real schedule.

---

### Task 3 — finish participant action routing and temporal action ownership

**Goal**

Complete the action architecture already established by `ControlFrame`,
`ActorActionScheme`, the shared slot resolver, `ControlPrompt`, and
`MovePlayback`. Remove the remaining ownership/activation residue, then add
shared temporal action state only where existing move or maneuver timelines do
not already own it.

**Current code and plans**

- `crates/ambition_characters/src/brain/action_set/mod.rs::ActionSet`
- `crates/ambition_characters/src/action_scheme.rs`
- `crates/ambition_combat/src/moveset/mod.rs::{ActorMoveset, MovePlayback}`
- `crates/ambition_actors/src/ability_cooldown.rs`
- `crates/ambition_actors/src/avatar/starting_character.rs::gate_worn_player_control`
- `crates/ambition_actors/src/affordances/`
- focused plans [`character-actions.md`](character-actions.md),
  [`participant-input.md`](participant-input.md), and
  [`participant-action-system.md`](participant-action-system.md)

**Approach**

1. Treat the landed slot-to-action scheme and shared resolver as the base; do not
   mint a second invocation framework merely to rename it.
2. Complete participant-owned bindings, contexts, device routing, cue ownership,
   and local-N routing through the PA1–PA6 sequence.
3. Delete remaining post-hoc control stripping, duplicated affordance logic, and
   demo-specific edge interception as their consumers move onto the shared seam.
4. Model startup, active, recovery, cancellation, cost commitment, armor/i-frames,
   cooldown, completion, and abort only for actions that need temporal ownership.
   Reuse or extend `MovePlayback` and movement-maneuver state instead of competing
   with them.
5. Keep movement laws, movesets, interactions, and provider effects as specialized
   executors; do not invent a universal effect DSL.
6. Migrate one coherent action family first and delete its superseded activation
   path before expanding. Ordinary walking and passive locomotion state are not
   forced into the action timeline.

**Size:** L
**Design uncertainty:** Medium–high
**Payoff:** Extremely high. Finishes an already-valuable seam, removes
mechanic-specific wiring, and supports local-N and provider-defined actions
without controller forks.
**Risks:** replacing rather than completing the landed action system, duplicating
`MovePlayback`, forcing continuous modes into a discrete lifecycle, or leaving
parallel activation paths indefinitely.
**Exit criteria:** the selected action family uses the existing semantic resolver
from human, brain, replay, and provider sources; prompts and eligibility agree;
participant ownership is explicit; temporal/cooldown state is rollback-safe; the
old activation and affordance path for that family is deleted.

---

### Task 4 — finish the swept contact and interaction doctrine

**Goal**

Complete and enforce the architecture already specified by `SweepSample`, the
canonical `Contact` vocabulary, the `cast` family registry, `GeoId`/`GeoSource`,
and the per-trigger semantics in [`collision-and-ccd.md`](collision-and-ccd.md).
Movement, environmental interaction, and combat should share identity, filtering,
ordering, and frame conventions without being forced into one implementation or
event type.

**Current code and plans**

- `crates/ambition_engine_core/src/cast.rs`
- `crates/ambition_engine_core/src/world.rs`
- `crates/ambition_engine_core/src/body_clusters.rs::{SweepSample, BodyEnvironmentContact}`
- `crates/ambition_interaction/src/lib.rs::Interactable`
- `crates/ambition_combat/src/lib.rs::DamageVolume`
- hazard, rebound, pogo, pipe, loading-zone, portal, and actor-contact consumers
- [`collision-and-ccd.md`](collision-and-ccd.md)
- [`spatial-model.md`](spatial-model.md)

**Approach**

1. Preserve both movement kernels and the existing rule that path-dependent
   mechanics evaluate the continuous swept path.
2. Audit the CC2/CC3 status against source, finish remaining cast-family and
   trigger-reader migrations, and turn the diagnostic illegal-state coverage into
   enforcement only when its documented behavioral preconditions are met.
3. Make remaining consumers use canonical `SweepSample`, `Contact`, cast queries,
   geometry identity, and stable ordering instead of reconstructing proximity or
   contact from endpoints and tolerances.
4. Define enter/stay/exit state only for consumers that genuinely need persistent
   region occupancy; do not create a second generic area engine beside the swept
   trigger doctrine.
5. Standardize causal source/target identity and filtering conventions across
   combat hits and environmental contacts while retaining optimized separate
   pipelines.
6. Peel hazard, rebound, pogo, blink, or similar effects away from `BlockKind`
   only after migrated consumers prove a real independent surface/effect dimension.

**Size:** M–L
**Design uncertainty:** Medium
**Payoff:** Extremely high. Finishes a strong existing design, prevents repeated
false-contact and attribution defects, and avoids a speculative solver rewrite.
**Risks:** ignoring the landed doctrine and building a parallel contact layer,
turning every contact into an allocated event stream, or decomposing geometry
before consumers prove the dimensions.
**Exit criteria:** the existing collision doctrine has source-audited status;
remaining path-dependent readers use canonical swept queries; CC3's chosen gate
is executable; migrated effects preserve stable causal identity without new
solver branches or a second area/contact framework.

---

### Task 5 — finish canonical construction and lifecycle ownership

**Goal**

Complete the existing construction campaign so supported authored, staged,
dynamic, reset, transition, reload, and reconstruction paths use one planned and
validated authority boundary.

**Current code and plans**

- `crates/ambition_actors/src/world/rooms/stage.rs::RoomConstructionPlan`
- `crates/ambition_world/src/placements.rs::PlacementLoweringPlan`
- `crates/ambition_actors/src/features/ecs/spawn/content_staging.rs`
- `crates/ambition_platformer_provider/`
- [`immutable-content-and-transactional-construction.md`](immutable-content-and-transactional-construction.md)
- the current closure items recorded in `../status.md` and `../tracks.md`

**Approach**

1. Finish the active Phase 6 remainder and measurements before broadening scope.
2. Preserve immutable prepared content, explicit provenance, stable identity,
   relation closure, and transactional publication.
3. Make lifecycle ownership explicit enough that temporary transfers can restore
   exact prior ownership.
4. Remove remaining direct live-world spawn paths only where the construction
   doctrine says they belong in the transaction.
5. Do not add a second prefab/object graph above Bevy ECS.

**Size:** M, because the major campaign is already substantially landed
**Design uncertainty:** Low–medium
**Payoff:** Very high. Closes an existing architecture campaign and supports
reliable reset, transition, rollback, and provider composition.
**Risks:** reopening settled design, duplicating ECS state in plan data, or
expanding the campaign into every transient effect.
**Exit criteria:** the named remaining construction closure is complete; supported
lifecycle paths share the canonical plan/commit boundary; ownership transfers
restore exact prior state.

---

### Task 6 — provider plugin surface and engine boundary cleanup

**Goal**

Make game composition explicit and Bevy-native while reducing central runtime
knowledge of leaf systems, without prematurely promising API stability.

**Current code to refactor**

- `crates/ambition_runtime/src/lib.rs`
- `crates/ambition_platformer_provider/`
- `crates/ambition_host/`
- `crates/ambition/src/lib.rs`
- domain owner plugins throughout the workspace
- [`architecture.md`](architecture.md)

**Approach**

1. Keep `ambition_runtime` responsible for the global semantic phase graph and
   common session lifecycle, not installation of every leaf system.
2. Move domain-local initialization, systems, registrations, and presentation
   adapters into owner plugins.
3. Keep Bevy `App`, `Plugin`, components, resources, and schedules visible to
   providers where they are the correct extension mechanism.
4. Curate the `ambition` facade around supported composition conveniences, but
   avoid hiding implementation crates behind indiscriminate re-exports.
5. Delay compatibility promises until multiple current games use the same seam.

**Size:** M–L
**Design uncertainty:** Medium
**Payoff:** High. Improves composition and navigation while avoiding a parallel
framework above Bevy.
**Risks:** plugin fragmentation, opaque ordering, excessive re-export churn, or
stabilizing weak APIs too early.
**Exit criteria:** domain plugins own their local installation; runtime orders
semantic sets rather than naming leaf systems; providers use ordinary Bevy
composition plus documented Ambition contracts.

---

### Task 7 — Bevy asset lifecycle and cross-target content validation

**Goal**

Use Bevy's typed asset system as the loading substrate while making Ambition's
prepared content, stable identities, readiness, failure policy, and artifact
packaging dependable across supported hosts.

**Current code to audit and refactor**

- `crates/ambition_asset_manager/`
- `crates/ambition_load/`
- `crates/ambition_load_presentation/`
- provider preparation and content fingerprints
- platform build and asset publication scripts
- Android, web, desktop, and headless startup paths

**Approach**

1. Inventory where Ambition duplicates `AssetServer`, handle, load-state, or hot
   reload responsibilities and delete unnecessary parallel machinery.
2. Keep stable content IDs and immutable prepared-content fingerprints distinct
   from transient Bevy handles.
3. Define readiness groups and transactional failure behavior at room/session
   boundaries.
4. Make absent payloads produce precise visible/headless diagnostics and valid
   fallback behavior where allowed.
5. Add artifact smoke tests that boot, resolve representative assets, and enter a
   room on each declared host profile.
6. Prefer maintained Bevy loaders/plugins where they satisfy requirements.

**Size:** M
**Design uncertainty:** Medium
**Payoff:** High for shipping reliability and host parity.
**Risks:** building a second asset database, confusing stable content identity
with runtime handles, or trying to solve binary distribution inside the engine.
**Exit criteria:** one Bevy-backed loading path serves supported hosts; prepared
content binds exact dependencies; missing or stale assets fail transactionally
and diagnostically.

---

### Task 8 — participant input composition on Bevy input

**Goal**

Map Bevy keyboard, gamepad, touch, replay, AI, and optional network sources into
participant-owned semantic controls without changing the authoritative body path.

**Current code and plans**

- `crates/ambition_input/`
- `crates/ambition_touch_input/`
- participant/control components in `ambition_actors` and `ambition_characters`
- [`participant-input.md`](participant-input.md)
- [`participant-action-system.md`](participant-action-system.md)

**Approach**

1. Use Bevy input/device events and resources rather than wrapping them behind a
   duplicate hardware layer.
2. Define participant ownership, control schemes, contexts, deadzones, and
   mapping into deterministic `ControlFrame`s.
3. Keep UI/menu input consumption separate from gameplay frame production.
4. Make local-N routing explicit and optional for single-player providers.
5. Add rebinding and persistent profiles only after the semantic action contract
   is stable.

**Size:** M
**Design uncertainty:** Medium
**Payoff:** High for multiple games, controller/touch parity, and local-N play.
**Risks:** conflating device state with authoritative simulation, coupling menus
to gameplay input, or forcing every provider to pay for multiplayer complexity.
**Exit criteria:** keyboard, gamepad, touch, replay, and brain inputs reach the
same authoritative participant/body boundary; device reassignment does not
change simulation semantics.

---

### Task 9 — provider-owned animation and camera policy over Bevy presentation

**Goal**

Let providers map authoritative facts into animation and camera behavior without
extending central hardcoded ladders or creating a parallel rendering world.

**Current code to refactor**

- `crates/ambition_actors/src/character_sprites/anim/`
- sprite-sheet and character-presentation crates
- `crates/ambition_sim_view/src/camera_snapshot.rs`
- camera systems in render/host composition

**Approach**

1. Keep action timing, hit timing, movement, and invulnerability authoritative;
   animation remains a consumer.
2. Replace global animation-priority growth with provider-owned policy or data
   driven from stable body/action facts.
3. Continue using Bevy sprites, atlases, transforms, cameras, and render layers.
4. Add only the camera composition Ambition needs: policy priority, target groups,
   transitions, temporary focus, constraints, and impulses.
5. Avoid a universal animation graph or Cinemachine clone unless concrete games
   demonstrate the need.

**Size:** M
**Design uncertainty:** Medium
**Payoff:** Medium–high. Removes central presentation branching and improves
provider independence without destabilizing simulation.
**Risks:** letting animation drive gameplay, overbuilding graph machinery, or
wrapping Bevy presentation primitives without semantic value.
**Exit criteria:** Mary-O, Sanic, and the main game provide distinct animation and
camera policies without adding named branches to central selectors.

---

### Task 10 — Bevy presentation and game-shell completeness audit

**Goal**

Determine which rendering, audio, UI, settings, localization, and accessibility
capabilities are already adequately supplied by Bevy/Ambition, which should use
an ecosystem plugin, and which small platformer-specific contracts remain.

This is an **audit and consolidation campaign**, not authorization to build a
new renderer, mixer, or UI framework.

**Current code to audit**

- `crates/ambition_render/`
- `crates/ambition_vfx/`
- `crates/ambition_audio/`, `ambition_sfx/`, `ambition_sfx_bank/`
- `crates/ambition_game_shell/`, `ambition_menu/`, `ambition_settings_menu/`
- `crates/ambition_ui_nav/`, `ambition_dialog/`, `ambition_inventory_ui/`
- Bevy renderer, audio, UI, diagnostics, and selected ecosystem plugins

**Approach**

For each domain:

1. identify concrete acceptance-game and host requirements;
2. inventory Bevy and maintained ecosystem support;
3. identify duplicate Ambition machinery or game-specific leakage;
4. retain only platformer-specific semantics, read models, routing, or policy;
5. open a focused implementation plan only for a demonstrated gap.

Likely Ambition-owned seams include:

- confirmed-frame semantic audio/VFX cues;
- platformer sorting and presentation facts where Bevy primitives need policy;
- controller glyph and prompt projection from participant bindings;
- game-shell pause/settings semantics that remain outside authoritative simulation;
- accessibility policy hooks required by shipped games.

**Size:** M for the audit; follow-up tasks may range from S to L
**Design uncertainty:** High until the audit is complete
**Payoff:** High scope control. Prevents Ambition from rebuilding Bevy while
making real shipping gaps visible.
**Risks:** turning the audit into a generic feature wishlist, selecting abandoned
ecosystem dependencies, or retaining duplicate systems because migration is
inconvenient.
**Exit criteria:** every reviewed domain has an explicit ruling: use Bevy core,
use a selected plugin, retain a justified Ambition contract, refactor duplication,
or defer because no current customer requires it.

---

### Task 11 — intentional persistence and checkpoint contracts

**Goal**

Persist versioned domain state and reconstruct through canonical content and
lifecycle seams rather than serializing the live ECS world as an accidental API.

**Current code to refactor or formalize**

- `crates/ambition_persistence/src/save.rs`
- `crates/ambition_persistence/src/save_data.rs`
- settings and progression fragments
- room/checkpoint/respawn/reconstruction systems
- stable content and simulation identifiers

**Approach**

1. Define an engine envelope with provider-owned versioned save fragments.
2. Persist stable content IDs and intentional progression/session facts, never
   Bevy `Entity` handles or presentation caches.
3. Reconstruct through prepared content and construction plans.
4. Add migration and removed-content policy only when actual schema evolution
   requires it.
5. Use Bevy/serde ecosystem support for serialization rather than creating a
   custom serialization framework without need.

**Size:** M
**Design uncertainty:** Medium
**Payoff:** High for exploration games and dependable game updates; moderate for
short level-based demos.
**Risks:** freezing schemas early, creating a second construction authority, or
persisting too much runtime state.
**Exit criteria:** a representative exploration/checkpoint flow saves and
restores intentional state through the canonical construction and lifecycle
paths, including a tested version transition.

---

### Task 12 — platformer frame inspector, budgets, and host validation

**Goal**

Compose Bevy diagnostics with Ambition's semantic traces so movement, action,
contact, damage, lifecycle, rollback, and presentation decisions can be explained
and performance regressions can be measured.

**Current code to extend**

- `crates/ambition_gameplay_trace/`
- `crates/ambition_sim_harness/`
- `crates/ambition_render/src/rendering/debug_viz.rs`
- Bevy diagnostics, tracing, schedule inspection, and suitable inspector plugins
- build/run scripts and host smoke tests

**Approach**

1. Build on Task 2's scenario fixtures and subsystem facts.
2. Provide a textual per-tick explanation first: semantic input, action state,
   velocity contributions, casts/contacts, damage facts, lifecycle changes,
   rollback corrections, and selected presentation decisions.
3. Add Bevy-native visual overlays or inspector integration where it improves
   diagnosis; do not build a separate editor shell by default.
4. Measure fixed-step time, allocations, collision queries, snapshot size,
   rollback cost, construction latency, render cost, startup, and artifact size
   from representative workloads.
5. Define host-specific budgets from measurements rather than arbitrary limits.

**Size:** L in total, decomposable into M campaigns
**Design uncertainty:** Medium
**Payoff:** Extremely high differentiation and engineering leverage.
**Risks:** intrusive tracing, unstable schemas, a custom debugger UI that fights
Bevy tooling, or optimizing without representative workloads.
**Exit criteria:** recorded scenarios can explain the important causal decisions
at a selected tick; supported hosts have artifact smoke tests and measured
budgets; diagnostics do not affect authoritative outcomes.

---

## 10. Dependency and sequencing map

```text
Active construction closure ───────────────> Task 5

Task 1 simulation integrity ─┐
Task 2 scenario proof ───────┼──────────────> Task 3 action lifecycle
                             └──────────────> Task 4 contact conventions

Task 5 construction ─────────┐
Task 6 provider composition ─┼──────────────> Task 7 resource/host validation
                             └──────────────> Task 11 persistence

Task 3 actions ──────────────> Task 8 participant input
Task 3 + Task 4 ─────────────> Task 9 animation/camera policy
Tasks 6–9 ──────────────────> Task 10 presentation/game-shell audit
Tasks 1–4 + 7–10 ───────────> Task 12 inspector, budgets, host proof
```

Recommended strategic order:

1. finish the already-active construction closure;
2. execute Tasks 1 and 2 as the safety foundation;
3. execute Task 3 as one bounded coherent action-family migration;
4. audit and execute Task 4 through a small environmental vertical slice;
5. complete Tasks 5 and 6 as the provider/composition boundary;
6. select Tasks 7–11 based on demonstrated shipping and acceptance-game pressure;
7. build Task 12 incrementally, beginning with textual diagnostics and measured
   workloads rather than an editor.

`tracks.md` may reorder bounded slices when a concrete game or platform blocker
has higher immediate value. It should not violate the dependency logic without
recording why.

---

## 11. Program-level risks and controls

### Risk — second-system architecture

A new action framework, contact framework, asset system, or presentation system
could coexist indefinitely with the current path.

**Control:** migrate one vertical slice, prove it, and delete the superseded path
before broadening.

### Risk — fighting Bevy

Ambition could wrap or replace Bevy facilities until providers must learn two
engines.

**Control:** every generic-runtime task begins with a Bevy/ecosystem inventory and
must justify any Ambition-owned abstraction in platformer-specific terms.

### Risk — copying editor-engine nouns

Godot nodes, Unity GameObjects/prefabs, and Unreal actors are products of their
own composition models.

**Control:** compare capabilities and outcomes, not object names. Preserve ECS,
plugins, components, resources, and explicit data flow.

### Risk — abstraction before pressure

A universal action, surface, material, animation, or save model could be designed
without enough consumers.

**Control:** open with a bounded customer set, record non-goals, and expand only
when the next customer fits without exceptions.

### Risk — manual correctness obligations

The project can continue adding rollback, scheduling, lifecycle, and causal
identity defects even while the broad architecture is correct.

**Control:** prioritize Tasks 1 and 2 before large semantic migrations.

### Risk — broad shipping lists overwhelm the niche

Rendering, audio, UI, accessibility, host, and ecosystem work can consume the
roadmap without strengthening the platformer engine.

**Control:** Task 10 is an audit. Follow-up implementation requires a concrete
provider, host, or shipped-game need and must compose with Bevy.

### Risk — public API stability too early

A polished facade can freeze weak internal concepts.

**Control:** document supported seams and expected churn; version stability comes
after multiple different games exercise the same contract.

### Risk — acceptance games overfit the engine

Demonstrations can encourage special cases or engine code that merely relocates
game policy.

**Control:** engine changes must state the reusable platformer capability being
added and identify the existing or expected second consumer. Game-specific rules
remain provider-owned even when they are sophisticated.

---

## 12. Competitive acceptance levels

### Core engine competitive

- authoritative simulation participation is enforced and tested;
- the rich actor/body path is shared across controller kinds;
- participant routing uses the landed slot-to-action seam, and one action family
  uses shared temporal ownership without a parallel activation path;
- representative environment mechanics use the canonical swept contact/cast
  doctrine rather than endpoint reconstruction;
- canonical construction and lifecycle ownership are complete for supported
  origins;
- headless and rollback scenarios prove the same authoritative outcomes;
- providers compose through Bevy plugins and explicit Ambition semantics rather
  than private global paths, and the out-of-workspace consumer fixture holds the
  oracle with a shrinking API-leak list.

### Shippable runtime competitive

- declared host profiles boot produced artifacts and enter representative rooms;
- assets use Bevy-backed loading with exact Ambition content identity and
  transactional failure policy;
- participant input supports the required device/context profiles;
- animation and camera policy are provider-owned and simulation-independent;
- rendering, audio, UI, settings, and persistence have completed the Bevy-native
  audit and any demonstrated gaps are closed;
- measured performance budgets exist for representative workloads.

### Mature engine competitive

- multiple materially different games use the same core semantics without
  parallel engine paths;
- provider documentation describes stable-enough supported seams and expected
  extension patterns;
- a frame inspector explains platformer-specific causal behavior;
- optional capabilities such as local-N fighters, online rollback transport,
  exploration persistence, unusual gravity, or additional world-authoring
  backends compose without becoming mandatory dependencies for all games.

---

## 13. Deferred or acceptance-driven candidates

These are legitimate future capabilities but are not automatically core roadmap
campaigns:

- production online transport beyond the required GGRS simulation contracts;
- a capability-conditioned traversal graph;
- generalized animation blend trees;
- advanced 2D lighting, occlusion, or postprocessing beyond game needs;
- a standalone Ambition editor or inspector application;
- broad authoring-backend importers beyond active LDtk needs;
- public long-term API compatibility guarantees;
- platform profiles without hardware, a shipping customer, or reproducible CI.

A candidate becomes an active campaign when a focused plan identifies:

1. the concrete consumer;
2. the missing Bevy or Ambition capability;
3. the intended ownership boundary;
4. the smallest proof;
5. the migration and deletion plan;
6. the measured runtime and maintenance cost.

---

## 14. Final direction

Ambition should not attempt to win by matching the total feature count of Unity,
Godot, or Unreal. Bevy already supplies much of the general engine substrate, and
its ecosystem should remain part of the solution.

Ambition should win its niche by making the parts that matter specifically for
2D platformers unusually coherent:

- one rich actor and movement path;
- deterministic actions and contacts;
- enforced rollback and lifecycle integrity;
- canonical content construction;
- headless and visible equivalence;
- inspectable platformer-specific causality;
- Bevy-native composition rather than a second engine layer.

The master-plan sequence is therefore:

> **Protect and enforce the simulation core; unify actions and interactions;
> finish construction and provider composition; compose shipping capabilities
> with Bevy; then productize diagnostics, hosts, and mature provider seams.**
