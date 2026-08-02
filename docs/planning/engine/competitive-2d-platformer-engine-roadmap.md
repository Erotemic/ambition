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

- `crates/ambition_platformer2d_core/src/movement/`
- `crates/ambition_platformer2d_core/src/cast.rs`
- `crates/ambition_platformer2d_shared_tangle/src/body/`
- [`unified-movement-kernel.md`](unified-movement-kernel.md)
- [`collision-and-ccd.md`](collision-and-ccd.md)

Ambition already treats high-quality platformer motion as reusable engine
machinery rather than per-game controller code. This is a real advantage over
general engines and should not be weakened by a generic rigid-body abstraction.

### 5.2 Deterministic, headless, rollback-oriented execution

Primary owners include:

- `crates/ambition_platformer2d_runtime/src/rollback/`
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

- `crates/ambition_platformer2d_actor_monolith/`
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

- `crates/ambition_platformer2d_provider/`
- `crates/ambition_platformer2d_world/`
- `crates/ambition_platformer2d_actor_monolith/src/world/rooms/`
- `crates/ambition_platformer2d_actor_monolith/src/features/ecs/spawn/`
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
the workspace through the `ambition_platformer2d` umbrella alone, gated by `external consumer:
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

- `crates/ambition_platformer2d_runtime/src/rollback/mod.rs::register_engine_rollback_state`
- `crates/ambition_platformer2d_shared_tangle/src/schedule.rs::Platformer2dSimulationPhaseMonolith`
- runtime plugin assembly in `crates/ambition_platformer2d_runtime/src/lib.rs`
- demo scheduling in `game/ambition_demo_mary_o/` and
  `game/ambition_demo_sanic/`
- `game/ambition_app/tests/rollback_coverage.rs`

**Approach**

1. Move rollback declarations toward owner crates through a neutral registration
   vocabulary that does not invert dependencies into `ambition_platformer2d_runtime`.
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

**Status 2026-07-27 — exit criteria MET; the internal cleanup is not.**

The criterion is answered, and deliberately from outside the workspace, in
`fixtures/external_consumer` (Outlander):
`consumer_owned_authoritative_state_survives_real_resimulation`. Outlander
declares `BeaconCharge`, writes its own snapshot codec, registers it with
`app.rollback_component_canonical::<BeaconCharge>(..)` through the public
`ambition_platformer2d::runtime::rollback` vocabulary, and installs its systems into
`Platformer2dSimulationPhaseMonolith::PlayerSimulation` via `app.sim_schedule()`. No engine file names the
component; nothing in `ambition_platformer2d` could, because nothing in `ambition_platformer2d` has heard
of it. The rewind is real — a GGRS sync-test session, ~900 loads and ~4500
resimulated advances in the test's own run — and the ridge gate is GATED on the
charge, so the state is authoritative rather than decorative. Verified RED:
delete the one registration line and the rollback host reports `ticks: 150`
against the fixed-tick host's `32`.

That settles "can a feature own its authority". It does NOT settle the
housekeeping the task also lists: the engine's own ~246 registrations still live
in `register_engine_rollback_state`, and item 3 (removing gameplay-authoritative
`Local<T>`) is untouched. Those are real, but they are internal tidiness — the
capability question is the one that decides whether another game can be built on
this, and it is now answered by an executable test rather than by inspection.

Two findings the proof produced, both recorded rather than papered over:

- **Starting a rollback session before construction finishes is a guaranteed
  divergence, and nothing said so.** The first draft started the sync test on
  update #1; GGRS reported a checksum mismatch on frames 2–4 forever, because
  the shell's preparation and the session-world commit build the room through
  `Commands` a rollback cannot undo. The engine's own harness avoids this by
  construction (`with_start_room` builds first), so no in-repo test could find
  it. A consumer gets a raw checksum mismatch and no hint.
- **A consumer that wants both hosts must know TWO input seams** — the
  `ControlFrame` resource under fixed tick, `PendingLocalInput` under GGRS.
  Writing the wrong one is silently ignored: the walk runs, the body never
  moves. Recorded as Phase-6 leak #4.

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

**Status 2026-07-27 — all four families rewind; the DECLARATIVE scenario half
does not exist.**

`Platformer2dSimHarness` steps the real schedule and `with_sync_test_rollback_settings`
turns any scenario into a rewound, checksum-compared one. By family:

- damage — `rollback_exit_oracle::a_player_taking_hp_damage_survives_rollback`,
  `enemy_death_and_inplace_revive_survive_rollback`, and the four-objective
  `combat_equipment_switch_and_breakable_survive_forced_rollback_identically`.
- transition — `rollback_room_transition::a_room_transition_survives_the_rollback_window`,
  plus the intent-committed-exactly-once and momentum-preserved cases.
- action — `desync_canary::sync_test_session_performs_real_rewinds_and_resimulation`
  drives a scripted stream containing attack, dash, jump and projectile, and
  `two_ggrs_harnesses_match_under_the_same_input_stream` compares two hosts.
- contact — **added the same day this status was written.** The gap was real:
  `collision_invariant_oracle` is the contact instrument and every sim in it is a
  plain fixed-tick one, so contact was asserted through the real schedule and
  never through a rewind. `rollback_contact::contact_state_survives_real_rewind_and_resimulation`
  now walks a contact-dense traversal under a sync-test session (236 loads, 1185
  advances over 240 frames), with vacuity guards on grounded, airborne and wall
  contact separately — a body that never leaves the ground has a constant flag,
  and any restore preserves a constant.

Approach items 3–5 (typed opaque facts, persisted failing fixtures, defer
interactive inspection) are satisfied by `ambition_gameplay_trace` and
`replay_fixture_regression`. Item 1 — a concise DECLARATIVE scenario description
— is not: a scenario today is Rust in a test file, and `Platformer2dSimHarnessOptions` +
`AgentAction` is the closest thing to a vocabulary.

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
- `crates/ambition_platformer2d_actor_monolith/src/ability_cooldown.rs`
- `crates/ambition_platformer2d_actor_monolith/src/avatar/starting_character.rs::gate_worn_player_control`
- `crates/ambition_platformer2d_actor_monolith/src/affordances/`
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

**Status 2026-07-27 — MET for melee, which is the selected family.**

Clause by clause, since a five-clause criterion is exactly the kind that gets
called done on two of them:

- one resolver from every source — `trigger_moveset_moves` reads `ActorControl`,
  which is written by a human slot, a brain, or a replayed `InputStream`
  identically; the moveset itself is provider-authored.
- prompts and eligibility agree — `control_prompt.rs` rebuilds the prompt from
  the same gate, and `a_same_tick_kit_swap_cannot_drift_the_prompt_from_the_gate`
  is the test that says so on the tick where drift is possible.
- participant ownership explicit — `ControlledSubject` + `SlotControls`, and
  `prompt_follows_the_controlled_subject_on_possession`.
- temporal state rollback-safe — `MovePlayback` is registered
  `rollback_component_resolved` with `rollback_map_entities`, so its live hitbox
  handles are remapped rather than dangling.
- old path deleted — the flat `BodyMelee` driver is GONE; `BodyMelee` survives
  only as a read-model projected from the live `MovePlayback`, so there is one
  strike path and the other consumers did not have to change.

What remains is scope the criterion does not name: only melee has been taken
through this. Ranged and special still ride the same `ActionSet` but have no
equivalent statement, and "shared temporal action state where existing timelines
do not already provide it" was deferred, correctly, until a second family needs
it.

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

- `crates/ambition_platformer2d_core/src/cast.rs`
- `crates/ambition_platformer2d_core/src/world.rs`
- `crates/ambition_platformer2d_core/src/body_clusters.rs::{SweepSample, BodyEnvironmentContact}`
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

**Status 2026-07-27 — MET. All four clauses, with the CC3 gate enforcing in both
the default suite (cheap tier) and the full guarded sweep.**

The doctrine is source-audited and the audit is written down, per reader, in
[`collision-and-ccd.md`](collision-and-ccd.md)'s CC2-completion table: hazard
touch, Door zones, EdgeExit/Walk zones, water/climbable regions and ledge grab
each carry a verdict and the date it was checked, and the ones that stayed
discrete carry `discrete_ok` at the reader with the reason. That is exactly what
"source-audited status" asked for, and it is rarer than it sounds.

Path-dependent readers use the canonical queries: one `transition_for_player`
sweep subsumes Door and Walk zones through `cast::aabb_path_contacts`, ledge grab
derives from resolved kernel contacts rather than a trigger overlap. Causal
identity is standardized — `GeoId`/`GeoSource` reaches 18 crates, and reactive
blocks carry `GeoId` on `ContactSource::Block` — with no second area engine: no
generic enter/stay/exit layer was built, which was the named risk.

**The third clause was taken on 2026-07-27 and this paragraph is half stale, so
here is what is actually true.** `collision_oracle_full_sweep` ASSERTS: every
invariant class CC3 claims to exclude is a failure with a seeded repro, and the
one exclusion is an out-of-bounds that left through an open edge (§6.1 already
rules those legal). Promoting it from measurement to enforcement found and fixed
a real level defect on the way — `stagger_steps` spawned a body 15px inside a
step — and the grounded-and-settled `EmbeddedAtRest` invariant was added and
measured at zero.

What is NOT true is that it gates ordinary work. It stays `#[ignore]`d because a
full sweep is minutes, not seconds, so `cargo test --workspace` does not run it;
the goal guard runs it explicitly with `--ignored`. So a regression is caught by
a guarded run and NOT by a developer's ordinary test loop.

✔ **The wiring gap is closed too.** `collision_oracle_smoke` already ran in the
default suite and asserted nothing about violations — its comment still called
embed/teleport/OOB "the deferred bugs" long after the full sweep stopped
deferring them. It now applies the SAME exclusion rule at a cost the default
suite can afford (two seeds, cold-launch room, 0.74s), so a break that affects
the launch room fails in an ordinary `cargo test` instead of waiting for a
guarded run. The full sweep remains the authority on COVERAGE; the cheap tier is
the authority on "did somebody just break it".

---

### Task 5 — finish canonical construction and lifecycle ownership

**Goal**

Complete the existing construction campaign so supported authored, staged,
dynamic, reset, transition, reload, and reconstruction paths use one planned and
validated authority boundary.

**Current code and plans**

- `crates/ambition_platformer2d_actor_monolith/src/world/rooms/stage.rs::RoomConstructionPlan`
- `crates/ambition_platformer2d_world/src/placements.rs::PlacementLoweringPlan`
- `crates/ambition_platformer2d_actor_monolith/src/features/ecs/spawn/content_staging.rs`
- `crates/ambition_platformer2d_provider/`
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

**Status 2026-07-27 — MET on ownership; the transactionality verdict below was
CORRECTED after a source check (the boundary exists; the failure HANDLING is
what is uneven).**

`../status.md` carries the evidence row and it is unusually complete. Phase 4
landed 2026-07-23: every authored family is a plan row, the outer roster is
exactly `planned_ids()`, all five lifecycle paths share one transaction, and
stale content bindings are refused at the boundary (`ActiveContentBinding` +
fatal `ContentBindingMismatch`). Ownership transfer restores exact prior state —
that is N3.1 take/restore, where restore PATCHES survivors rather than rebuilding
them, and N3.2b atomic room restore. Phase 5 landed the coverage forcing
functions and the behavioral restore proofs, which caught two demo mode owners
registered-but-unanchored.

**⚠ THE PARAGRAPH THAT USED TO BE HERE WAS WRONG, and wrong in the expensive
direction: it would have sent somebody to build a staging world for a problem
this path does not have.** It said "there is no staging world, so a construction
that fails its boundary check has already mutated the live world". Source-checked
2026-07-27:

- `RoomConstructionPlan::prepare_from_parts` returns
  `Result<Self, RoomConstructionError>` and every variant is detected BEFORE any
  live-room mutation — the type says so and the error enum's own doc comment
  claims it.
- `apply_to_world` documents itself as having "no fallible lookup" after it
  returns, i.e. the commit half is infallible BY CONSTRUCTION.

So plan → validate → commit is real for room construction. Verification does
PREVENT here.

✔ **The actual gap was failure HANDLING, and it is closed.** Of the callers,
`lifecycle_commit.rs` already handled a failed prepare gracefully (logs, returns
`CommitOutcome::Retry`, world untouched), while `session/setup.rs` and
`session/reset/mod.rs` both did
`.unwrap_or_else(|error| panic!("… failed: {error}"))` — a preflight that
correctly refused killed the process on two paths instead of declining.
`process_new_game_reset_request` declines now; `session/setup.rs` still panics
DELIBERATELY, because no game exists yet and a silent partial start is worse
than a loud stop.

✔ **And a second half nobody had looked for.** Declining is not the same as
costing nothing: `clear_transient_on_sandbox_reset` was chained BEFORE the
processor and keyed on the REQUEST, so a refused reset had already emptied the
player's hands, despawned the portals and stripped the portal gun (GPT 5.6,
2026-07-27). `despawn_player_clones_on_reset` in the app crate did the same.
Both now wait for a `NewGameResetCommitted` message the processor writes only
once the preflight has agreed — a request is what somebody asked for, a
commitment is what the preflight allowed, and only the second may authorise a
teardown. `sandbox_reset_clears_portals_held_items_and_summons` drives the
refusal shape.

✔ **The residue is answered:** whether `spawn_contents`' individual COMMANDS can
fail after the infallible boundary. Audited rather than assumed — almost
everything queued is a SPAWN, which cannot fail; three commands touch existing
entities, two of them the arms of one `if` in `retire_outgoing`, and retiring
something already gone is the outcome both arms want, so both are `try_`. The
third is left alone deliberately: its entities come from its own query.
`deferred_write_safety` is the harness that answers this class by running a real
pass against a real teardown.

Approach item 1 named "the active Phase 6 remainder": the second slice landed
(Outlander launches and walks its ridge gate under a real routed shell, gated in
`run_tests.py`), and 2026-07-27 added consumer-owned rollback state to it — see
Task 1. What is left of Phase 6 is the visible-shell half and the Task 7
measurements, neither of which is a construction question.

---

### Task 6 — provider plugin surface and engine boundary cleanup

**Goal**

Make game composition explicit and Bevy-native while reducing central runtime
knowledge of leaf systems, without prematurely promising API stability.

**Current code to refactor**

- `crates/ambition_platformer2d_runtime/src/lib.rs`
- `crates/ambition_platformer2d_provider/`
- `crates/ambition_platformer2d_host/`
- `crates/ambition_platformer2d/src/lib.rs`
- domain owner plugins throughout the workspace
- [`architecture.md`](architecture.md)

**Approach**

1. Keep `ambition_platformer2d_runtime` responsible for the global semantic phase graph and
   common session lifecycle, not installation of every leaf system.
2. Move domain-local initialization, systems, registrations, and presentation
   adapters into owner plugins.
3. Keep Bevy `App`, `Plugin`, components, resources, and schedules visible to
   providers where they are the correct extension mechanism.
4. Curate the `ambition_platformer2d` facade around supported composition conveniences, but
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

**Status 2026-07-27 — third clause MET and proved externally; SECOND clause is
plainly false.**

Providers use ordinary Bevy composition: Outlander is a `Plugin` that calls
`app.sim_schedule()` and `.in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)`, never a literal
schedule, and it assembles a whole game from outside the workspace through the
`ambition_platformer2d` umbrella. Four recorded API leaks is the honest cost, and they are
listed rather than hidden. Domain plugins largely own their installation — the
2026-07-27 addition of consumer-owned rollback registration is the strongest case
of that, since the engine cannot even name the type.

~~The second clause is not met and the file says so at a glance…~~
◐ **RE-MEASURED 2026-08-02: the named next row was DONE, and the debt MOVED.**

The old text said `player_schedule.rs` "names `ambition_platformer2d_actor_monolith::`
leaf systems THIRTY times in one chain" and called turning it into ordered
semantic sub-sets "the concrete next row". That row landed. `PlayerInputSet`
exists in `ambition_platformer2d_shared_tangle::schedule` as an ORDERED phase
vocabulary, the file's own comment records the change (*"now placed by PHASE …
each one states which phase it belongs to, so a caller elsewhere can order
against the phase instead of against a name"*), and an external consumer already
does exactly that — `action_scheme.rs:106` orders `.after(PlayerInputSet::Persona)`.

⚠ **the count is still 31, and that is not the defect.** Those are PLACEMENTS
(`.in_set(PlayerInputSet::…)`) — somebody has to say which phase each system is
in, and the file that composes the schedule is the right somebody. The defect the
clause names is ORDERING against a leaf, which is what made a set's membership
undiscoverable and produced the `GgrsSchedule` cycle.

**Ordering constraints against a leaf system (snake_case target), whole runtime:**

```text
  progression_schedule.rs   8   ← where the debt actually lives now
  portal_schedule.rs        3
  player_schedule.rs        2   ← the file this status singles out
  combat_schedule.rs        2
  sandbox_reset.rs          2
  mode_scope.rs             1
  rollback/session.rs       1
  ────────────────────────────
                           19
```

✔ **and that row is DONE, same day.** `ProgressionSet` now exists beside
`PlayerInputSet` in `ambition_platformer2d_shared_tangle::schedule`, with six
phases: `BossAdvance · BossHazards · SaveMirror · Quest · WorldSync · Map`.
`progression_schedule.rs` went from **8 leaf orderings to 0**, and the runtime
total from 19 to **11**.

⭐ **the phase boundaries were DERIVED from the pins, not invented.** Two slots
(`ContentEncounterScriptSet` and `ambition_encounter::EncounterLifecycleSet`)
anchored INSIDE the boss group, both against `update_encounter_progress` — which
is exactly why the boss work is two phases rather than one. A vocabulary that
could not express an anchor that already existed would have forced that anchor to
stay a leaf, and the leaf is the thing being removed.

⚠ **order preserved byte-for-byte, checked rather than asserted**: the 17 systems
diff identically before and after, and the original block carried no `run_if` to
drop. That mattered more than usual — this chain runs under rollback, where a
reordering is a desync rather than a bug report.

✔ **one more converted, and it was a CRATE that owed a set rather than a
schedule that owed a phase.** `combat_schedule.rs` wrote
`CombatSet::ContentSpecials.before(ambition_vfx::apply_effects)` — a cross-crate
ordering against a bare `pub fn`, because `ambition_vfx` exposed no set to order
against. It does now: `ambition_vfx::EffectExecutionSet`.
⚠ that crate deliberately has NO `Plugin` — it is an effect vocabulary plus one
executor, and the host decides when to run it. A set is the smaller thing that
makes the host's decision expressible: it says WHERE the executor sits without
claiming when the host should install it. Runtime total 11 → **10**.

▢ **and the remaining ten are NOT mechanical — that is the finding.** Three of
them pin INSIDE an existing phase, so converting them means inventing a boundary
and choosing a position that is currently implicit:
* `PortalSet::InputWarp` sits between `interaction_input_system` and
  `sync_local_player_input_frame`, both inside `PlayerInputSet::Device`, with no
  constraint against the three systems chained between them;
* `player_schedule.rs:238` orders a Brain member after `tick_player_brains`,
  another Brain member — an INTRA-phase edge that `.after(Brain)` would make
  circular;
* `DevEditApplySet` pins to "part A's tail", a sub-group with no name.

⚠ **and the ambiguity those imply is not a determinism bug**, which is worth
stating because it looks like one: `rollback/mod.rs` runs `GgrsSchedule` with
`ExecutorKind::SingleThreaded` and `ambiguity_detection: LogLevel::Ignore`, on
the stated reasoning that *"GGRS is a managed same-build contract: every peer
runs the same plugin graph"*. An unconstrained pair resolves the same way on
every peer. What it costs is not correctness but LEGIBILITY — the position is
decided by a topological sort rather than by anything written down.

✔ **and one more was exactly equivalent rather than merely stricter**, which is
the distinction that decides whether a conversion is safe to make blind.
`sandbox_reset.rs` pinned `.before(control::input_timer_system)`; that system is
the FIRST element of the tuple carrying `.chain().in_set(PlayerInputSet::Device)`,
so being before it IS being before all of Device. Converted. Runtime 10 → **9**.

⛔ **and the neighbouring one is NOT, which is why they are being told apart.**
`portal_schedule.rs` pins `.after(physics::collect_gravity_zones)`, and that
system is the MIDDLE of three chained inside `GravitySet::ZoneSnapshot`
(`oscillate_gravity_zones · collect_gravity_zones · collect_force_zones`).
`.after(ZoneSnapshot)` would additionally wait for `collect_force_zones` — a
STRICTER constraint that removes an ambiguity by choosing a side. Probably
harmless (portal carves and force zones are semantically unrelated) and that is
exactly the word that should stop a blind edit in a rollback schedule.

⭐ **and swept WORKSPACE-WIDE, the clause turns out to be scoped to the wrong
place.** This status has only ever counted the runtime. Counting every
`.after`/`.before` against a snake_case target across 945 non-test source files:

```text
  98  own-crate      a plugin ordering its OWN systems — legitimate, not this
  49  CROSS-CRATE    a crate reaching for another crate's leaf function
       ├── ambition_app          10
       ├── ambition_content      10
       ├── ambition_demo_sanic   10
       ├── ambition_platformer2d_runtime   7   ← all this clause has measured
       ├── ambition_demo_mary_o   4
       ├── ambition_platformer2d_host  3
       ├── ambition_touch_input   3
       └── monolith 1 · demo_smash 1
```

⚠ **35 of the 49 are in GAMES** (`app`, `content`, `sanic`, `mary_o`, `smash`),
and a game ordering against an engine leaf is the more serious version of this
defect, not the milder one: it is a consumer depending on engine internals, which
is the same thing the facade and SDK work exists to prevent. The runtime — the
only site this clause has ever named — now holds 7 of 49.
⚠ `ambition_render/rendering/mod.rs` looked like the worst offender at 16 until
the own/cross split: **all 16 order its own systems**, which is a plugin doing
its job. The raw count was measuring the wrong thing, which is why the split is
recorded rather than the total.

✔ **and the biggest single case is converted: 49 → 45.**
`rebuild_feature_ecs_world_overlay` was pinned by SIX consumers — four in
`ambition_content` (`bosses`, `falling_sand`, `falling_sand_sim`, `intro`), one in
the monolith's own `encounter`, all naming an engine leaf across a crate boundary.
It now has `FeatureWorldOverlaySet` and every one of them orders against that.

⚠ **deliberately a ONE-MEMBER set.** The obvious alternative — spanning this
system and `update_ecs_hazards` beside it in the chain — would have made
`.after(set)` STRICTER than the pin it replaces, because consumers would newly
wait for hazards. One member makes the swap exactly equivalent, which is what
lets it be made without a scheduling judgement in a rollback-critical chain.

⭐ **and the measurement that made it findable is worth copying**: for each
cross-crate pin, ask whether the TARGET already belongs to a set. Eighteen of the
twenty in `app`/`content` did not — so the work is mostly "an engine crate owes a
set", not "a consumer is misbehaving", and that is a much cheaper fix than it
looked.

✔ **two more clusters converted the same way — 49 → 39.**
* `rebuild_feature_ecs_world_overlay` finished at EIGHT consumers, not six: two
  more pinned it through the FACADE path
  (`ambition_platformer2d::actors::features::…`) and the first grep, keyed on the
  monolith's own path, missed them. A re-measure by TARGET NAME rather than by
  import path found them.
* `apply_player_hit_events` had four game pins (mary_o ×1, sanic ×3), each
  spelling `actors::features::ecs::damage_apply::…` through the facade. That path
  is itself the tell: a consumer spelling four module levels to place its own
  system is depending on engine internals. Now `PlayerHitResolutionSet`.

⚠ both are ONE-MEMBER sets, and the second has a sharper reason than the first:
`publish_kernel_reset_death` sits beside it in the tuple and is deliberately NOT
gated on `gameplay_allowed` while this one is. A set spanning both would hand
consumers an ordering against a system with different run conditions.

✔ `update_ecs_hazards` too — it lives in `ambition_combat`, is scheduled by the
monolith beside the overlay system, and two `ambition_content` plugins ordered
against it by name. Now `ambition_combat::hazards::HazardTickSet`. **49 → 37.**
⚠ four re-export hops to make it reachable (`hazards` → `features::ecs` →
`features` → the facade), which is itself the argument: a name a consumer must
order against has to travel the same path the function does, and the function was
already re-exported at every level. A set that stops one level short is a set
nobody can use.

▢ what is left, by consumer count: `tick_player_brains` 4 · `step_projectiles` 2 ·
`sync_local_player_input_frame` 2 · `gate_worn_player_control` 2, then singletons.
⚠ `tick_player_brains` is the awkward one: it ALREADY has a set
(`PlayerInputSet::Brain`) with two members, so converting is the STRICTER case,
not the equivalent one.

✔ **a fifth: `ProjectileStepSet`, and this one crosses the PRESENTATION
boundary.** `ambition_platformer2d_host` ordered two render passes
(`sync_projectile_visuals`, `sync_projectile_charge_visuals`) against
`projectile_schedule::step_projectiles` — presentation reaching through the
runtime's re-export into a monolith leaf to place itself. The edge is real and
its comment says why (*"both after the step so a projectile fired this frame is
visible this frame rather than one frame late"*); what was missing was a name to
hang it on. **37 → 35.**
⚠ ONE member again, and here the neighbours make the case: `charge_projectile_input`
and `apply_player_spawn_projectile_messages` sit after the step DELIBERATELY (*"so
the new body first ticks next frame"*), so a set spanning them would push
presentation past a spawn it is not waiting for.

⭐ **the running tally for this whole thread: 49 → 23 cross-crate leaf pins, via
twelve sets** — eleven single-member (`EffectExecutionSet`, `FeatureWorldOverlaySet`,
`PlayerHitResolutionSet`, `HazardTickSet`, `ProjectileStepSet`,
`WornControlGateSet`, `MenuFrameCutsceneSkip`, `LocalInputFrameCommit`,
`PortalLinkResolution`, `PortalPickupArming`, `PlayerBrainTick`) and one with two
(`MenuFramePopulate`) — plus one phase vocabulary (`ProgressionSet`, six phases).
Every conversion is EXACTLY equivalent to the pins it replaced — including the
four that were parked for being "stricter", which they were not; see below.

⛔ **do not maintain the tally here. `scripts/check_cross_crate_leaf_pins.py`
owns it now**, and the numbers above are frozen prose about what happened, not a
worklist. This paragraph used to carry the live count and a list of what
remained, and both went wrong: the list recorded `tick_player_brains` as an
ordinary row when it is the one that already has a two-member set, and the
`git grep NAME | head -8` reached for to re-check the list truncated two
cross-crate pins off the bottom of its own output, which was then published as
"it is intra-crate". A hand tally drifts silently and the ad-hoc command used to
audit it has its own silent limit.

`--list` prints every remaining row grouped by target crate, from the tree, in
under a second. Start there.

⭐ **the "stricter" category was a mistake and is retired.** It said: when the
pinned system already sits in a multi-member set, pinning that set moves every
consumer later, so the conversion needs a decision rather than an edit.
`tick_player_brains` sat under it twice, unconverted, as the single most-pinned
function left.

The error is in the word *that*. Reusing the ENCLOSING set would indeed be
stricter — but a **nested single-member set** is always available, and it is
exactly equivalent to the leaf pin it replaces, by construction. All four
`tick_player_brains` pins converted with no ordering change.

⚠ and the nested set reaches a case the parent never could: an intra-set edge
between two members owned by DIFFERENT crates. `record_player_movement_intent`
is itself in `PlayerInputSet::Brain`, so `.after(PlayerInputSet::Brain)` is a
cycle; `.after(PlayerBrainTick)` is fine. Those edges look irreducible and are
not.

▢ **so there are two kinds left**, and the guard's `--list` tells you where:
* **convert** — nearly everything. Give the target a set beside its definition,
  nested inside whatever set it already has, and pin that. State at the
  definition why the set holds the members it holds; every one of the ten so far
  has had a different reason to stay single-member, and the next reader can only
  widen one safely if that reason is written down.
* **intra-phase** — the pin is between two members of one set AND the consumer
  is in that set too, so the parent is a cycle and the sub-boundary has to be
  invented rather than named. `PortalLinkResolution` and `PortalPickupArming`
  are both this shape; it is still an edit, just a design one.

▢ **so the remaining nine in the runtime sort into three kinds**, and only the
first is mechanical:
* **equivalent** — the pinned system is the head of a chained set. Convert.
* **stricter** — the pinned system is inside a set. Needs someone to decide the
  new position is acceptable (`portal_schedule` × 1).
* **intra-phase** — the pin is between members of one set. Needs a new boundary
  invented (`portal_schedule` × 2, `player_schedule` × 2).

---

### Task 6.5 — the binding resolution boundary — **LANDED 2026-07-25**

Cross-layer references (anim row, item art, brain key, patrol path, room link)
resolve ONCE at construction into typed handles, and what does not resolve lands
in one report naming the declarer and the available ids. The silent
`row_index_of -> unwrap_or(0)` / `HashMap::get -> placeholder` paths are deleted.

This was not on the original task list, and it should have been: it is the
readiness-and-failure half of Task 7, the precondition for Task 9 (a provider
cannot own animation policy until a misnamed row fails loudly), and the first
inspectable-causality surface Task 12 wants — as a value, not a UI.

See [`binding-resolution-boundary.md`](binding-resolution-boundary.md).

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

**Status 2026-07-27 — two clauses met; "fail TRANSACTIONALLY" is deliberately
not, and 2026-07-27 showed what that costs.**

One loading path: `AmbitionLoadPlugin` is supplied by the engine group, every
host gets the same room-transition-as-load-plan, and the game's own content is a
registered `game://` asset source so worlds load without the engine's asset root
containing one. Prepared content binds exact dependencies — that is Task 6.5, and
the prepared definition additionally carries derived cue and vfx inventories.

Failures are DIAGNOSTIC and not transactional, on purpose: an unresolved
reference is reported and the registration publishes anyway, because a character
that draws a placeholder and says why beats a session that refuses to boot. The
cost showed up on 2026-07-27: four shipped characters named the sheet FILE where
the registry is keyed by its `target:`, drew placeholders, printed a 400-id ERROR
on every boot, and `checked_namespaces()` still called the sheet namespace
verified — because it recorded that a resolver RAN, not what it answered. Now
`unresolved_references()` rides on the published value and
`registered_character_art_resolves` fails the build for the shipped composition.
So the criterion is met in spirit by a test rather than by a transaction, and
that substitution should be an explicit ruling here rather than an accident.

Not covered at all: "supported hosts" currently means desktop. There is no
android or wasm crate, so cross-target content validation is untested rather than
passing.

---

### Task 8 — participant input composition on Bevy input

**Goal**

Map Bevy keyboard, gamepad, touch, replay, AI, and optional network sources into
participant-owned semantic controls without changing the authoritative body path.

**Current code and plans**

- `crates/ambition_input/`
- `crates/ambition_touch_input/`
- participant/control components in `ambition_platformer2d_actor_monolith` and `ambition_characters`
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

**Status 2026-07-27 — MET for the five sources; a SIXTH boundary exists that the
criterion did not anticipate.**

All five arrive at the same place and one test drives all of them:
`participant_input.rs` taps keys, sets gamepad buttons and axes, and moves a
touch stick against the real shell composition, while `input_stream_replay`
proves a recorded stream replays a fresh sim with zero divergence and brains
write the same `ActorControl` a human slot does. Device reassignment is the
participant slice's subject — `SlotControls[PRIMARY]` is keyed by slot, not by
device, and possession transfers the brain rather than the body.

The unanticipated part, found on 2026-07-27 while giving Outlander a rollback
host: there are TWO seams depending on host. Fixed tick consumes the
`ControlFrame` resource; GGRS consumes `PendingLocalInput`, because the frame it
simulates is the one the session confirmed. Writing the wrong one is silently
ignored — the walk runs, the body never moves, nothing says why. Every in-repo
caller happens to be on the right side of it, so no test could notice; a consumer
outside hits it immediately. Recorded as Phase-6 leak #4 and queued as D2b(b).
The criterion says inputs reach "the same authoritative boundary" and they do —
but which resource carries them is host-dependent, and that is the same class of
defect one level up.

---

### Task 9 — provider-owned animation and camera policy over Bevy presentation

**Goal**

Let providers map authoritative facts into animation and camera behavior without
extending central hardcoded ladders or creating a parallel rendering world.

**Current code to refactor**

- `crates/ambition_platformer2d_actor_monolith/src/character_sprites/anim/`
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

**Status 2026-07-27 — MET. (This row's first draft said PARTIAL and was wrong on
both clauses; the correction is more interesting than the verdict.)**

All three providers declare distinct camera framing and get distinct animation,
and `grep` finds no game name in any central selector — the only occurrence of
"Sanic" or "Mary O" in the host presentation layer is a doc line forbidding it.

- **camera** — `SubjectFramingPolicy::SoftSafeRegion(SoftFramingProfile)` rides on
  the route-declared `GameplayPresentationProfile`, per environment. The flagship
  takes `adaptive_platformer` (full bleed, occlusion-aware on touch), Sanic takes
  `high_speed_full_bleed` (velocity-aware soft framing everywhere), Mary-O takes
  `fixed_four_by_three` (fixed viewport, reserved surround, top-pinned on touch).
  Three genuinely different cameras, declared, no branches.
- **animation** — `pick_body_anim` is one priority ladder driven by body state and
  per-body thresholds (`idle_below`, `run_above`, `fly_above`), and the sheet rows
  it names are per-character. Distinct animation comes from data, which is the
  form the criterion asks for.

⚠ **The correction, and why it is written down.** The first draft asserted "there
is no `CameraPolicy` and no `AnimationPolicy` type" — true, and irrelevant. Both
capabilities exist under names I did not grep for. That is the SECOND time in one
audit that grepping for a type name produced a false "not started" (the other was
Task 11's save versioning, which lives in `save_data.rs` and not `save.rs`). A
capability is not a type name; checking for one by searching for the other finds
absence reliably and presence never.

What is genuinely NOT declarable, stated narrowly this time: `CameraEaseTuning`
(zoom-in / zoom-out rates and the snap epsilon) and the shake amplitude cap are
single global resources, so two games in one host cannot ease differently. No
provider has asked yet, and it is a small addition to the same route-keyed
catalog when one does.

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

**Status 2026-07-27 — MET. See
[`presentation-and-shell-audit.md`](presentation-and-shell-audit.md).**

The criterion is about COVERAGE — "EVERY reviewed domain has an explicit ruling" —
so individually correct decisions scattered across crate docs could never satisfy
it: nothing could say whether a domain had been skipped. Thirteen domains are now
enumerated with a ruling each, written against the code rather than against crate
names (the method correction recorded at Task 9 applies here, and this audit was
the one remaining "not started" it had to be applied to).

Eleven resolve to "Bevy core", "selected plugin", or a justified Ambition contract.
The two that do not say what would have to be true to change them:

- **localization — DEFER.** No i18n dependency and no string table; every
  user-facing string is a Rust literal. The trigger is the first non-English
  target, and the cost grows with every month of authored content, so this one
  should be re-examined deliberately rather than by drift.
- **accessibility — PARTIAL.** Colorblind mode, flash intensity (a real
  photosensitivity control, clamped), shader strength and camera framing all ship.
  Missing: input remapping surfaced in the menu, text scaling, `bevy_a11y` /
  screen-reader integration, subtitles. No customer yet; platform certification
  mandating remapping is the likeliest first one.

Two gaps found while writing it are tracked rather than absorbed: diagnostics are
measured and never enforced (D13), and who owes `AmbitionLoadPlugin` is an
undocumented composition rule whose violation is a hard panic — which is how the
external fixture sat red until somebody read it.

The audit's own instruction was that it does not authorize building a renderer, a
mixer, or a UI framework. The finding is that nobody has.

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

**Status 2026-07-27 — MET, on the third attempt, after the third correction.**

⚠ This row claimed "met" twice before it was true, in two different ways, and
both corrections are worth keeping visible.

The first draft said versioning did not exist; that came from grepping `save.rs`
alone. The second draft then called the flow representative on the strength of
the shrine — and the shrine was not a checkpoint at all. It called
`save.set_changed()` on a value it never modified, which the value-comparing
autosave correctly ignores, and there was no checkpoint field to write into
anyway. It healed, logged "healed to full + saved", and persisted nothing (GPT
5.6, 2026-07-27). "Marks the resource changed" reads like persistence and is not.

Both halves now exist. `PersistedCheckpoint { room_id, x, y }` is schema v3 —
which is what finally exercised the migration chain on a real change rather than
a hypothetical one — recorded by the shrine and applied by
`restore_checkpoint_on_session_start` through `transit_body`, the one transit
authority, so arrival is at rest with contacts reconciled. Cutscene progress and
the encounter/switch state ride the same save, so "intentional state" is more
than a position.

⚠ **THIRD correction, and this row is NOT met (GPT 5.6, 2026-07-27, correct).**
The saved `room_id` is never used to CHOOSE the opening room — it is only
compared against whatever room the session already opened, and a mismatch
returns. So a player who rests in room B, quits, and starts a session that opens
in room A does not resume at their checkpoint; they start in A. Worse, the
generation latch is set BEFORE the room comparison, so walking into B later in
that same session does not apply it either.

The previous draft called the mismatch "deliberately not applied; that is what
the room id is for", and that sentence is true of the COMPARISON and false as a
description of the feature. Refusing to teleport a body into another room's
coordinates is right. Refusing to OPEN the checkpoint's room is the gap, and
naming the first does not discharge the second.

✔ **Closed the same day it was corrected.** `restore_checkpoint_on_session_start`
now ROUTES: a checkpoint naming another room of this world emits an ordinary
`RoomTransitionRequested` at the checkpoint's coordinates, so the session opens
where the player rested. It requests once per session (a transition takes several
frames to commit, and re-requesting every frame restarts it forever), and a
checkpoint naming a room this world does NOT contain warns and keeps the
session's own room — which is the case the earlier draft was really describing.

Routing through the ordinary transition rather than repointing the room set is
deliberate: staging a room is a transaction with content, geometry and
authorization in it, and "one place stages a room" is worth more than saving a
message.

**Correction to this row's first draft**, which said versioning did not exist:
that was grepped from `save.rs` alone, and the version lives in `save_data.rs`.
`version: u32` and `CURRENT_SAVE_VERSION = 2` had existed for a while. The real
defect was worse than absence and less visible — the tag was WRITTEN and never
READ. No migration, no compatibility check, no consumer of `CURRENT_SAVE_VERSION`
anywhere in the workspace, and `default_save_version()` returned CURRENT, so
every pre-versioning file claimed to be the current shape. A tag nothing reads is
not a tag; it is a comment that costs a field.

Closed 2026-07-27: `AmbitionGameSaveData::migrate()` runs the version chain and
returns a `SaveCompatibility` verdict, a missing field now means v1 (what it
actually is), and `load_save` returns `LoadedSave { data, writable }`. The
`writable` half is the part that protects a player: a save from a NEWER build, or
bytes that will not parse, are read as a fresh sandbox and the file is LEFT
ALONE, so an older build launched once cannot overwrite progress it could not
understand. Seven tests, including two verified RED — remove the gate and both
report the original file destroyed.

The v1 → v2 step is deliberately empty and deliberately present: the wire change
was additive, so there is nothing to do, but the mechanism has to exist before
the first migration that does something real — otherwise that migration is also
the one that has to invent the mechanism, under pressure, with player data at
stake.

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

**Status 2026-07-27 — third clause MET and enforced; the first two are partial.**

"Diagnostics do not affect authoritative outcomes" is the strongest one and it is
architectural rather than promised: the confirmed-frame external-effect
quarantine (`ambition_platformer2d_runtime::external_effects`) means a speculating host defers
an effect instead of suppressing it, and the emit-time gate that used to drop
sounds during resimulation was deleted precisely because suppressing at emit time
destroys the corrected outcome.

~~Causal explanation at a tick is partly there and not assembled.~~ ⛔ **STALE —
the selection step exists and has been load-bearing since 2026-08-01.** This
paragraph ended *"What is missing is the selection step: nothing takes a tick and
returns the decisions at it."* That is `ambition_causal::CausalLog::explain(tick,
&SubjectKey) -> Explanation` (`log.rs:234`), with `Explanation::render()`
(`log.rs:421`) printing every fact for that subject on that tick.

It is not a paper API. On 2026-08-02 it answered a multi-day question — the
fighter's ladder self-KO — by carrying `MovementOp::Slash` on exactly the ticks a
velocity ramp appeared, and the whole `[unclaimed]` velocity detector
(`ambition_causal::UnclaimedStepDetector`) is built on `explain` plus
`subjects_on`.

⚠ **and the paragraph's own list of pieces is what made it read as unfinished**:
`ambition_gameplay_trace` and the rollback localizer are DIFFERENT instruments
answering different questions. The selection step was built beside them, in
`ambition_causal`, rather than assembled out of them — so a status looking for it
among the named pieces would not have found it.

⚠ what IS still missing on this clause, measured 2026-08-02: an explanation is
TICK-SCOPED, so a change at tick N caused by an event at tick N−k cannot be
joined to its cause. That is a real limitation and a different sentence from the
one above.

Budgets are measured but not ENFORCED, and only on one host. The always-on
censuses print `[schedule-census]`, `[frame-spike]` and `[image]` lines every
boot — that is how the 627MP/2.5GB decode was found — but nothing fails when a
number regresses, and "supported hosts" is desktop only (see Task 7: no android
or wasm crate exists). Artifact smoke tests exist for the shipping entrypoint
(`shell_host_headless_entrypoint`, plus the heavy `run_game.sh` acceptance
cycles), so that half is real.

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
