# Architecture reconnaissance and decomposition plan

## Audit scope

I reviewed the archive at commit `0d45956`, including:

* Workspace manifests and the crate dependency graph.
* Engine, game, demo, and content source.
* Plugin installation and schedule construction.
* Session preparation, activation, reset, teardown, and provider switching.
* Room construction and placement lowering.
* Human control, possession, AI intent, body simulation, combat, ranged actions, and held items.
* `SimView`, rendering, RL observation, and headless execution.
* Snapshot registration, restore, stable identity, and known restore debt.
* Architecture documentation, module maps, policy tests, poison tests, migration matrices, and the generated ECS inventory.
* Recent history available in the archive, especially convergence work around movement, melee, capabilities, portals, and controlled subjects.

This is a static audit. Cargo and Rust were unavailable in the analysis environment, so I could not perform a fresh compile, test run, feature-matrix build, or runtime trace. The recommendations should therefore be validated against CI and behavioral oracles as they are implemented.

## Executive judgment

The repository is not suffering primarily from a lack of decomposition. It already has a substantial number of crates—49 workspace packages—and a relatively sophisticated architecture.

Its real problem is that several strong abstractions are **nearly authoritative but still have bypasses**:

* Provider-owned preparation exists, but its implementation lives inside the umbrella facade and providers repeat lifecycle glue.
* Session-root-owned world state exists, but some important live state remains process-global.
* App-local placement registries exist, but initial construction and reset can bypass them.
* A unified action timeline exists, but ranged and held-item mechanics still have parallel execution paths.
* `SimView` exists as a one-way observation boundary, but RL and some infrastructure still read authoritative ECS state directly.
* Domain crates exist, but the runtime still installs many subsystem leaf systems instead of mostly ordering domain-owned sets.
* Feature names imply modular builds, but the actual Cargo graph often compiles broad platform, portal, LDtk, audio, and presentation machinery unconditionally.
* Exact snapshot infrastructure exists, but dynamic reconstruction and room/session restoration are not yet unified with normal construction.

The highest-value architectural work is therefore not another wave of size-driven crate splitting. It is to make the intended paths exclusive:

```text
one provider lifecycle
one session authority
one room-lowering path
one body movement law
one action execution law
one observation boundary
one restore/construction path
one global schedule graph composed from domain-owned sets
```

That is what will make the engine elegant, extensible, and resistant to misuse.

## The architecture that is already emerging

The codebase has a coherent architecture underneath its recent growth:

```text
Provider-owned authored content and catalog fragments
                         │
                         ▼
       Backend-independent world and entity IR
                         │
                         ▼
        Prepared, identity-checked game session
                         │
                         ▼
        Session-root-owned live world state
                         │
                         ▼
     Actor/body simulation and domain mechanics
                         │
                         ▼
        Canonical facts and SimView projection
                         │
              ┌──────────┴──────────┐
              ▼                     ▼
    Headless/RL/test consumers   Rendering/UI/audio
```

The host and shell sit around this pipeline:

```text
Host selects an experience
            │
            ▼
Provider receives an exact preparation request
            │
            ▼
Provider builds and validates a prepared world
            │
            ▼
Shell activates that exact prepared identity
            │
            ▼
Session is constructed, scoped, and later cleaned up
```

This is a good model for an engine that must support:

* Conventional authored games.
* Code-authored and LDtk-authored worlds.
* Headless simulation.
* RL and evaluation.
* Replaceable presentation.
* Portals and nontrivial reference frames.
* Proper-time mechanics.
* Future networking and rollback.
* Multiple independently supplied games or experiences.

The goal should be to finish this architecture, not replace it with a more generic but less grounded framework.

---

# What should be protected

## The shared body simulation law

Player-controlled bodies and AI-controlled bodies already converge on the same underlying movement machinery.

The essential flow is:

```text
device or host input
        │
        ▼
ControlFrame at the input boundary
        │
        ├──────────────┐
        ▼              ▼
human control      brain-produced intent
        │              │
        └──────┬───────┘
               ▼
          ActorControl
               │
               ▼
resolved frame + selected MotionModel
               │
               ▼
      integrate_actor_body
               │
               ▼
 ambition_engine_core::step_motion
```

Brains do not directly move bodies. Controlled-subject resolution is separate from the home-player identity. Bodies select motion behavior rather than implicitly being divided into player and enemy physics classes.

That is exactly the kind of unification the engine needs.

`ambition_actors` is large, but much of its size is the orchestration surrounding this shared law:

* Perception preparation.
* Brain intent production.
* Mount and steering relationships.
* Moving platforms.
* Boss ordering.
* Body integration.
* Contact consequences.
* Read-model publication.
* Feature-specific adapters.

Splitting these by vertical game mechanic could recreate parallel movement and lifecycle systems. `ambition_actors` should be reorganized and purified where needed, but it should not be divided merely because it is large.

## The action timeline

`MoveSpec`, `MovePlayback`, action windows, proper-time progression, authored events, impulses, gates, and cancellation form a real action execution architecture.

Ordinary melee is already moving toward one route:

```text
intent
   ↓
moveset lookup
   ↓
MoveSpec
   ↓
MovePlayback
   ↓
timed semantic events
   ↓
hit volumes, motion, effects, audio, presentation
```

This should become the universal action law rather than being wrapped in a new action abstraction.

## App-local ordered registries

The catalog and placement registries are a strong extension model.

They provide:

* Deterministic ordering.
* App-local registration instead of process-global mutable registries.
* A place for providers to add authored concepts.
* Typed lowering rather than arbitrary runtime reflection.
* A viable path for content packages and plugins.

These registries should become more authoritative, particularly during initial session construction and restore.

## `SimView`

The intended one-way boundary from simulation truth to presentation facts is excellent.

It creates a natural home for:

* Rendering.
* Audio and UI projection.
* Headless observation.
* RL.
* Replay inspection.
* Netcode.
* Observer-relative and delayed views.

The concept should be expanded and enforced rather than bypassed.

## Exact preparation identity and session scopes

Prepared sessions are not simply selected by a loose name or “whatever was most recently loaded.” The code has typed prepared-session storage and exact activation identity.

Session scopes also provide a real basis for teardown and provider switching.

This is important architectural work and should remain central.

## Architecture enforcement

The repository’s policy tests are unusually valuable. They check things such as:

* Forbidden dependency directions.
* Direct access to input state.
* Ambient randomness and wall-clock use.
* Nondeterministic hash iteration.
* Entity ordering.
* Migration debt.
* Session authority.
* Module size and organization.
* Cross-domain scheduling assumptions.

These tests should increasingly complement compiler-enforced boundaries rather than compensate for missing ones, but the overall architecture-governance strategy is strong.

---

# Highest-value crate split

## Extract `ambition_platformer_provider`

The clearest subsystem currently hidden in the wrong crate is `crates/ambition/src/provider.rs`.

It is not merely facade glue. It owns substantive engine behavior, including:

* `AuthoredCatalogFragments`.
* `PlatformerAuthoredCatalogRegistry`.
* `PlatformerExperienceAuthoring`.
* Preparation validation.
* Asset and audio readiness.
* Typed `PreparedPlatformerSessions<M>`.
* Prepared-world cleanup.
* `PlatformerSessionBuilder`.
* Exact session activation.
* Generic platformer world assembly.

This belongs in a crate such as:

```text
ambition_platformer_provider
```

The public SDK can preserve the ergonomic path:

```rust
ambition::provider::PlatformerExperienceAuthoring
```

by reexporting the new crate.

This split has unusually high value because it improves several properties simultaneously:

* The umbrella facade becomes honest: mostly curation and reexports.
* Provider and content crates can depend directly on the subsystem they use.
* The dependency graph reveals how platformer experiences enter the engine.
* Provider lifecycle tests gain a clear home.
* Third-party developers can find the extension boundary by name.
* LLM agents do not need to infer that substantive session machinery is hidden inside a facade.
* The batteries-included one-crate user experience remains intact.

### Consolidate repeated provider lifecycle code

The current providers repeat a common protocol:

1. Install typed prepared-session storage.
2. Receive a preparation request.
3. Construct and validate authored catalogs and world state.
4. Store the exact prepared identity.
5. Receive the corresponding gameplay activation.
6. Consume the matching prepared world.
7. Build the session.
8. Install cleanup behavior.

That repetition indicates that the provider concept is sound but not completely lowered into a reusable plugin.

The extracted crate should offer a typed common plugin or builder. A provider should mainly supply:

* Authored catalog fragments.
* A world-preparation function.
* Optional provider-specific validation.
* Optional activation hooks.
* Custom placement, action, presentation, and content registrations.

Avoid a trait-object service locator. Ordinary Rust generics, Bevy plugins, typed resources, and typed messages are sufficient.

---

# Session lifecycle needs one authority

## The session root is the correct model

`PlatformerSessionWorld` correctly groups exact-session state such as:

* Catalogs.
* Room sets and geometry.
* Active-room metadata.
* Starting-character information.
* LDtk runtime indexing.
* Music and encounter requests.
* Other world-level session state.

The architecture documentation is also explicit that this is session-owned authority rather than process-global state.

The code has not yet fully completed that transition.

## Remove `SceneEntities`

`SceneEntities` is a process-global resource containing raw entity handles such as:

* Player.
* HUD.
* Quest panel or related presentation entities.

The setup path inserts placeholders, and later presentation code replaces some of those handles.

This is fragile for several reasons:

* Entity handles can outlive the session that created them.
* Session identity is implicit.
* Correctness depends on setup and overwrite order.
* Multiple simultaneous sessions or observers become difficult.
* Exact restore must reconstruct global handles.
* Possession and controlled-subject semantics are weakened by a process-global “player entity.”
* Relativistic or observer-relative games should not assume one universal player viewpoint.

The replacement should use relationships and scoped queries:

* The human-controlled body is discovered through `ControlledSubject`.
* Presentation entities carry session scope and semantic role markers.
* HUD systems observe the active session and controlled subject.
* Session-owned presentation roots are children of or explicitly related to the session root.
* A root-local handle cache is used only when a direct handle is genuinely necessary.

The objective should be deletion of `SceneEntities`, not merely renaming it.

## Give moving-platform state explicit session ownership

`MovingPlatformSet` is process-global while its contents are derived from the active room and session.

It participates in:

* Collision queries.
* Portal and frame behavior.
* `SimView`.
* Debug presentation.
* Initial setup.
* Room transitions.
* Hot reload.
* Provider-specific activation.

This produces two competing ownership stories:

```text
Room/session root owns authored and active world state

but

process-global MovingPlatformSet owns important live collision state
```

Hot collision state may reasonably use a specialized cache rather than a tree of ordinary ECS components. The problem is not that it is cached. The problem is that the cache’s session identity and invalidation law are implicit.

A better model would be one of:

* A session-root-owned `SessionCollisionState`.
* A scoped moving-platform cache keyed by `SessionScopeId`.
* A deterministic derivation rebuilt by one room-materialization owner.
* A runtime cache with explicit source generation and invalidation epoch.

The exact representation should preserve collision performance. What must disappear is ambient process-global authority.

## Construct the session root first

Current setup still divides construction among:

* Process resources.
* Placeholder scene handles.
* Player and room feature spawning.
* Development-editable configuration.
* Session-root attachment.
* Provider-specific preparation.
* Presentation attachment.

The lifecycle should become:

```text
create exact session root
        │
        ▼
attach authoritative PlatformerSessionWorld
        │
        ▼
resolve session configuration
        │
        ▼
materialize active room through installed registries
        │
        ▼
spawn controlled body and scoped entities
        │
        ▼
publish session-activation facts
        │
        ▼
presentation observes and attaches
```

This ordering makes the session root authoritative from the first spawned entity onward.

Development overrides should be resolved before this assembly step. Generic session construction should not directly depend on editable development-resource types.

---

# Unify every room-materialization path

There is a concrete inconsistency in placement lowering.

The general extension seam is `PlacementLoweringRegistry<C>`. Normal room transitions use the installed App-local registry.

However, `spawn_room_feature_entities` constructs a fresh hard-coded registry containing built-in interpreters such as:

* Hazard.
* Interactable.
* Pickup.
* Chest.
* Breakable.
* Portal where enabled.

Initial setup, resets, and some development paths use this fallback rather than the installed registry.

A provider-defined placement can therefore behave differently depending on how the room became active:

* It may work on an ordinary transition.
* It may be absent on initial activation.
* It may disappear after same-room reset.
* It may not participate in hot reload.
* It may not be reconstructible during restore.

This is exactly the kind of partial extension point that is dangerous for outside developers and coding agents.

## Required correction

All production room construction should call one materializer:

```text
authored room IR
      │
      ▼
installed PlacementLoweringRegistry
      │
      ▼
session-scoped runtime entities and caches
```

The built-in interpreters should be installed by their owning engine plugins. A production fallback that silently creates a different registry should not exist.

Tests can construct an explicit default registry when they need a lightweight fixture.

## Acceptance test

Register a fake provider-specific placement and prove identical behavior during:

1. Initial session activation.
2. Same-room reset.
3. Cross-room transition.
4. Development reload.
5. Snapshot reconstruction, once exact reconstruction is supported.

This should be a core extension-contract test.

---

# Finish one action execution law

## The remaining parallel paths are identifiable

The action architecture is not hypothetical. The remaining bypasses are visible migration seams:

* `ActionRequest::Ranged`.
* `ActionRequest::PlayerProjectileTick`.
* `MovesetRanged`.
* `HeldUseBehavior::UseSystem`.
* Bespoke systems for beams, meteors, sentries, vortexes, dives, volleys, shockwaves, charged weapons, and other named mechanics.
* Concrete held-item definitions inside `ambition_characters::brain::action_set`.
* Capability booleans that overlap with action availability.

These make the answer to “how does an actor perform an action?” depend on the specific mechanic.

That should converge to:

```text
human control / AI / replay / RL
              │
              ▼
       semantic action intent
              │
              ▼
      actor action bindings
              │
              ▼
        MoveSpec selection
              │
              ▼
         MovePlayback
              │
              ▼
       typed semantic effects
              │
  ┌───────────┼───────────┬────────────┐
  ▼           ▼           ▼            ▼
motion      combat    projectiles   audiovisual facts
```

## Extend the existing model rather than adding a new crate

The action runtime needs to express the mechanics currently delegated to bespoke systems:

* Press, hold, and release transitions.
* Charge accumulation.
* Repeated active ticks.
* Channelled execution.
* Aim sampling.
* Gesture or motion-recognizer gating.
* Resource expenditure.
* Cooldowns.
* Persistent or deployed effects.
* Per-action local state.
* Interruption and cancellation.
* Proper-time versus simulation-time progression.

These should be additions to the existing authored action model and playback law.

A universal event bus, arbitrary script callback, or `UseSystem` escape hatch would preserve ambiguity and make the engine easier to misuse.

## Rehome named item definitions

`ambition_characters` should own:

* Actor and control vocabulary.
* Brain policies.
* Intent generation.
* Action-selection interfaces.
* Durable actor relationships.

It should not own the global catalog of concrete named weapons and held items.

Reusable item schemas and behavior contracts belong in `ambition_items`. Named authored items belong in provider or content catalog fragments.

This improves:

* Dependency direction.
* Mod and provider ownership.
* Discoverability.
* Catalog isolation.
* Reuse of brains across different games.
* Testing of actor policy without importing flagship content.

## Migration gate

Action convergence is complete when:

* No gameplay system outside the control/action ingress reads physical attack input.
* `HeldUseBehavior::UseSystem` no longer exists.
* `MovesetRanged` no longer exists.
* `PlayerProjectileTick` is no longer a separate action execution system.
* Named item catalogs are not owned by the character/brain crate.
* Human, AI, possessed, replayed, and RL-controlled bodies use the same action contract.
* Proper-time behavior is declared rather than mechanic-specific.
* Old execution paths are deleted immediately after each mechanic family migrates.

---

# Make `SimView` the real observation boundary

`SimView` is intended to serve:

* Rendering.
* RL.
* Networking.
* AI or fighter observation.
* Slower-light mechanics.
* Debugging and replay.

Rendering generally follows this boundary.

The RL path does not fully do so. `game/ambition_app/src/rl_sim/runtime.rs` constructs observations by directly querying live body clusters, combat state, health, safety state, items, and gravity. It also uses assumptions tied to `PrimaryPlayer`.

This creates two definitions of observable game state:

```text
presentation-visible state through SimView

and

RL-visible state through direct ECS queries
```

That will eventually cause inconsistencies in:

* Possession.
* Hidden information.
* Delayed observation.
* Frame-relative observation.
* Replays.
* Netcode.
* Headless/render parity.
* Relativistic mechanics.

## Separate three concepts

The engine should distinguish:

1. **Authoritative simulation truth**
   Internal ECS and domain state.

2. **Canonical simulation facts**
   Stable, provider-neutral facts exposed through `SimView`.

3. **Observer projection**
   Facts transformed, filtered, delayed, or selected for a particular observer.

A conventional game can make observer projection nearly identical to canonical facts.

A relativistic or slower-light game can publish a delayed observer-specific slice without corrupting authoritative collision or action state.

## Recommended change

Move canonical observation extraction into `ambition_sim_view`. RL observation should adapt from those facts, plus explicitly exposed agent-only facts if necessary.

A research escape hatch for direct world queries can remain, but it should be clearly labeled as advanced and noncanonical. Standard RL, replay, test, rendering, and networking paths should share the same observation model.

This also implies that `SimView` should not depend on global `SceneEntities` to identify the subject. It should use controlled-subject, session, or observer relationships.

---

# Make the compile graph genuinely modular

The source architecture looks more modular than the Cargo feature graph.

## The umbrella facade is broad even with empty default features

The `ambition` crate exposes feature selection, but its internal engine dependencies are largely unconditional. A consumer that expects a minimal facade configuration may still compile:

* Rendering.
* Audio.
* Portal presentation.
* Touch support.
* LDtk support.
* Menus.
* Other upper-layer systems.

The public API says “modular engine,” while Cargo may still see “compile most of the engine.”

## The runtime is not truly headless-minimal

`ambition_runtime` unconditionally includes portal and LDtk-related actor machinery in places where the named feature suggests optional behavior.

Similarly, `ambition_actors` has a broad `desktop_dev` default feature and persona-style features combining:

* Visibility.
* Platform integration.
* Development tools.
* Input devices.
* Portal/LDtk support.
* Mobile or web behavior.
* RL support.

Library crates should not select application personas by default.

## Better feature organization

Separate **domain capabilities** from **application personas**.

Domain features might include:

* `portal_sim`.
* `portal_presentation`.
* `world_ldtk`.
* `audio_kira`.
* `touch_input`.
* `inspector`.
* `material_ui`.
* `rl`.
* `netcode`.

Application-level persona bundles might include:

* `desktop_dev`.
* `desktop_release`.
* `web`.
* `android`.
* `headless_eval`.

The persona bundles should live in the facade, host, or application layer. Lower machinery crates should have minimal defaults and narrowly named domain features.

## Required build matrix

Add CI configurations that prove the graph is truly modular:

1. Core headless, code-authored world, no LDtk.
2. Headless with no rendering or audio.
3. LDtk-authored world.
4. Portal simulation without portal presentation.
5. Full visible desktop.
6. Web.
7. Android or touch.
8. RL/headless evaluation.

Use `cargo tree` assertions or dependency-denial checks to verify, for example, that the minimal headless configuration does not pull rendering, Kira, windowing, touch, portal presentation, or LDtk.

This is high-value for build times and for third-party confidence.

---

# Runtime should order domains, not own all their leaf systems

Central schedule ownership is correct. The engine needs one place that can state:

```text
input resolution
    before brain intent

brain intent
    before motion

motion
    before contacts

actions
    before hit resolution

simulation facts
    before SimView publication

SimView publication
    before presentation
```

The problem is not central ordering. The problem is that runtime code frequently registers individual implementation systems from many domain crates.

For example, combat contains its messages, components, and implementation systems, while runtime assembles a large tuple of combat, actor, projectile, effect, and adapter functions.

This means understanding or extending a domain often requires editing the runtime’s leaf-system list.

## Better division

Each domain crate should own:

* Its resources.
* Its messages.
* Local system registration.
* Local ordering constraints.
* Public schedule sets.
* Its plugin or plugin family.

Runtime should own:

* Cross-domain set ordering.
* Global simulation phase definitions.
* Feature-level plugin composition.
* Headless versus visible composition policy.

The runtime graph would then read conceptually:

```text
InputResolveSet
    → BrainIntentSet
    → MotionSet
    → ContactSet
    → ActionAdvanceSet
    → CombatResolutionSet
    → WorldConsequenceSet
    → SimViewPublishSet
```

Runtime should not need to know every projectile recycler, boss hit handler, effect request consumer, and read-model updater by function name.

This suggests adding or strengthening plugins such as:

* Combat core plugin.
* Projectile simulation plugin.
* Actor-combat adapter plugin.
* Encounter runtime plugin.
* Domain-owned read-model publication plugins.

These are organizational plugins inside existing crates, not necessarily new crates.

---

# Model resource authority explicitly

Writer count alone does not determine whether a resource is well designed.

Different resources have different mutation algebra.

## Noncommutative state machines

Resources such as the current gameplay banner or active quest state should generally have:

```text
many typed requests
        │
        ▼
one reducer / state owner
        │
        ▼
published current state
```

`GameplayBannerRequested` already exists, but several callers still mutate banner state directly. The message path should become exclusive.

## Inventory state

Inventory has many legitimate producers:

* Pickups.
* Rewards.
* Consumption.
* Equipment.
* Throwing.
* Restore.
* Debug tools.

They should not all receive unrestricted mutable access to the representation.

Expose typed operations:

* `GrantItem`.
* `ConsumeItem`.
* `EquipItem`.
* `UnequipItem`.
* `TransferItem`.
* `DropItem`.
* `RestoreInventory`.

One owner applies these operations and protects invariants.

## Persistence

Broad mutable access such as `SandboxSave::data_mut` is a misuse magnet.

Persistence should not become the authoritative gameplay database. Domain state should remain authoritative, and persistence should:

* Project domain state into stable save representations.
* Checkpoint it.
* Resolve stable content references.
* Reconstruct domain state through normal construction paths.
* Apply explicit migrations.

A gameplay mechanic should not casually reach into the save blob and mutate unrelated fields.

## Append-only or commutative resources

Some registries, diagnostic logs, and event accumulators legitimately have many writers. These should expose APIs that make their algebra clear:

* Append-only.
* Idempotent registration.
* Ordered registration.
* Commutative accumulation.
* Scoped replacement.

The architecture rule should be:

> Give every durable resource an explicit mutation model, not necessarily one writer.

That is more rigorous and more useful to agents than a blanket ownership rule.

---

# Snapshot and restore should converge with construction

The snapshot system is already sophisticated. It includes:

* Opt-in domain registration.
* Stable deterministic ordering.
* `SimId` reconciliation.
* Content-reference resolution.
* Restore preflight.
* Honest reporting of whether restore was lossless.
* Tests around unregistered state debt.

Its documented limitations are also significant:

* Cross-room rollback is not generally supported.
* Dynamic entities cannot always be reconstructed without spawn recipes.
* Some resources remain known restore debt.
* Derived caches and presentation relationships require rebuilding.

This should remain close to runtime composition. A separate snapshot crate would not currently reduce conceptual coupling.

## Main architectural requirement

Restore should call the same authoritative mechanisms as normal construction:

```text
stable session identity
       │
       ▼
provider/catalog resolution
       │
       ▼
session-root construction
       │
       ▼
room materialization through installed registry
       │
       ▼
dynamic spawn recipes
       │
       ▼
component/resource restoration
       │
       ▼
derived-cache reconstruction
```

Do not create a second restore-only world builder.

## Dynamic spawn recipes

Dynamic entities need typed recipes that identify:

* What authored or runtime category they belong to.
* Which stable content identifier constructs them.
* Their session and room ownership.
* Which components are authoritative.
* Which components are derived.
* Which relationships must be resolved after creation.

This same mechanism can support:

* Rollback reconstruction.
* Save loading.
* Netcode reconciliation.
* Replay seeking.
* Development hot reload.

## “Lossless” should be a tested contract

For explicitly supported scenarios, CI should require a round trip:

```text
snapshot
→ mutate or destroy world
→ restore
→ canonical deterministic state matches
```

Unsupported state should fail preflight or report debt; it should not silently produce an approximate world.

---

# Refine `ambition_engine_core`; do not split it

The core owns a cohesive set of trusted concepts:

* Geometry.
* Casts.
* Contacts.
* Motion frames.
* Kinematics.
* Motion models.
* Body integration.
* Surface interaction.
* Gravity and acceleration behavior.

Splitting this into spatial and motion crates now would likely scatter invariants while giving most consumers two ubiquitous dependencies instead of one.

There are, however, vocabulary issues to address.

## Separate physical law from named mechanics

`BlockKind` includes both foundational collision behavior and named platformer mechanics:

* Solid.
* One-way.
* Hazard.
* Blink wall.
* Pogo orb.
* Rebound.

A motion kernel needs to know physical surface behavior. It should not need a closed enumeration of every game verb that can react to a surface.

A better typed decomposition is:

```text
Physical collision response
    solid / one-way / passable
    support velocity
    contact normal behavior

Mechanic overlays or capabilities
    damage on contact
    blink permeability
    pogo eligibility
    rebound impulse
    provider-defined contact reaction
```

Do not replace the enum with an untyped string property map. That would make validation and agent behavior worse.

The extensibility point should remain typed and constrained.

## Split capability ownership conceptually

`AbilitySet` combines several categories:

* Movement permissions.
* Combat/action permissions.
* Directional primary and special behavior.
* Shield or dodge behavior.
* Interaction behavior.
* Potentially host/session operations.

These should become clearer typed groups, for example:

```text
MovementCapabilities
ActionBindings or ActionCapabilities
InteractionCapabilities
SessionPermissions
```

The movement kernel should only receive the movement capabilities it needs.

Attack availability should increasingly be represented by the body’s action kit or action bindings rather than duplicated as broad capability booleans.

## Lower-priority cleanup

Window size, rendering Z configuration, and presentation conversion helpers are conceptually outside a movement core. They can move when a clear presentation-primitives owner exists.

This is not urgent enough to justify destabilizing the foundation before session and action convergence.

---

# Complete encounter convergence

`ambition_encounter` already contains generic encounter vocabulary and lifecycle machinery, but actor-side boss and wave coordinators still overlap with it.

The target ownership should be:

```text
ambition_encounter
    participant relationships
    objectives
    lifecycle
    phase/progression truth
    completion, failure, reset
    stable snapshot relationships
    generic encounter commands and facts

actors / characters / content
    boss brain policy
    named attacks
    phase-specific action selection
    authored scripts

presentation
    boss UI
    encounter banners
    music
    visual staging
```

A boss should be a specialized participant, not a parallel class of simulation object.

The generic encounter model should be validated with at least one non-boss case:

* Survival wave.
* Escort.
* Timed escape.
* Multi-switch objective.
* Arena control.
* Puzzle encounter.

Otherwise, the generic API may only disguise a boss-specific lifecycle.

As with actions, convergence should end by deleting the competing boss or wave lifecycle path.

---

# Crate decisions

| Candidate                       | Recommendation        | Reason                                                                                                             |
| ------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `ambition_platformer_provider`  | **Create now**        | A real preparation/session subsystem is hidden inside the SDK facade                                               |
| `ambition_sim_harness`          | **Possible later**    | Could own deterministic stepping, input streams, observations, snapshots, and replay after session/SimView cleanup |
| `ambition_spatial`              | **Do not create now** | Geometry, frame, contact, and motion invariants are cohesive                                                       |
| `ambition_brain`                | **Do not create**     | Actor, control, perception, intent, and brain vocabulary form a meaningful domain                                  |
| `ambition_action`               | **Do not create**     | Existing `MoveSpec` and `MovePlayback` should become authoritative                                                 |
| `ambition_snapshot`             | **Do not create now** | Restore is cross-domain runtime assembly and not independently cohesive yet                                        |
| Per-feature actor crates        | **Reject**            | Would likely split authority-woven orchestration and recreate mechanic forks                                       |
| More tiny presentation crates   | **Defer**             | First establish plugin/projection boundaries inside existing presentation crates                                   |
| `ambition_inventory_ui` removal | **Not important**     | Small but coherent navigation state is not architectural bloat                                                     |

## `ambition_actors`

Do not split it by LOC.

High-value work within it is:

* Complete the semantic transition from “features” terminology to actors or simulation entities where appropriate.
* Remove compatibility modules and aliases when migrations finish.
* Move named content definitions to content or item owners.
* Remove application-persona feature bundles.
* Clarify internal schedule sets and ownership.
* Keep tightly woven actor/body adapters together.

## `ambition_app`

Do not require the flagship product app to be tiny.

It legitimately owns:

* Product-specific menus.
* Development overlays.
* Capture and evaluation tooling.
* Flagship presentation.
* RL product integration.
* Debugging and authoring affordances.
* Title/frontend behavior.

Code should leave the app when another game would otherwise need to copy it:

* Generic provider lifecycle.
* Session creation.
* Room materialization.
* Reset/transition/restore construction.
* Standard headless/windowed simulation assembly.

The correct measure is not app LOC. It is whether a fifth independent game can be built without copying engine infrastructure or editing engine crates.

---

# Architecture for ordinary and relativistic games

The repository already has the beginnings of a strict generalization:

* Motion and acceleration frames.
* Frame-relative gravity behavior.
* Per-body motion models.
* Portal transforms.
* Proper-time scale.
* Proper-time action playback.
* Explicit control relationships.
* A simulation-to-observation boundary.

The next work should deepen those concepts through concrete mechanics.

## Every timer needs a clock domain

Each timer, cooldown, action phase, status effect, encounter phase, and perception delay should state whether it advances in:

* Global simulation time.
* Entity proper time.
* World or region time.
* Observer time.
* Host wall-clock time.

Ordinary games use identity mappings. Relativistic mechanics can alter them without inventing separate action or status systems.

Wall-clock time should remain outside deterministic simulation.

## Important vectors need frame semantics

It is not necessary to wrap every `Vec2` in heavy type machinery.

It is necessary to clarify vectors crossing trusted boundaries:

* Body-local control intent.
* World-frame velocity.
* Surface-frame contact velocity.
* Portal-transformed position and velocity.
* Observer-relative direction.
* Environmental acceleration.

Use typed wrappers or explicit structures where ambiguity can change behavior.

## Environmental assumptions should become fields

Gravity, flow, orientation, and acceleration should be queryable at a spacetime sample:

```text
field.sample(position, simulation_time, frame)
```

A normal game uses a constant field and pays almost no conceptual cost.

More unusual games can define:

* Rotating rooms.
* Accelerating frames.
* Local gravity wells.
* Time-varying forces.
* Region-specific clock behavior.

## Observation is distinct from truth

The engine should explicitly support:

```text
authoritative simulation state
           │
           ▼
canonical facts
           │
           ▼
observer-specific projection
```

A slower-light game can delay or transform what an observer sees while collision and action resolution continue against current authoritative state.

This is one of the strongest reasons to make `SimView` canonical and eliminate global “current player entity” assumptions.

## Acceptance mechanics

Build the mathematical architecture around real mechanics:

1. Two actors with different proper-time rates execute the same authored move.
2. A portal attached to a moving frame transforms position and velocity according to an explicit law.
3. A rotating or accelerating room supplies a nontrivial environmental field.
4. A slower-light observer sees delayed state without changing authoritative collision.
5. Possession changes the observer/control subject without changing the home-body identity.

Property tests should cover:

* Identity transforms.
* Inverses.
* Composition.
* Portal round trips.
* Collision-query covariance under supported transforms.
* Proper-time action consistency.
* Deterministic replay under the same frame and clock configuration.

Do not create a generalized “spacetime framework” before these examples expose a stable missing abstraction.

---

# Make the engine easy for humans and agents to explore

## Give every crate the same architectural header

Each major crate should answer, near its root:

* What does this crate own?
* What does it explicitly not own?
* Which lower-level crates may it depend on?
* Which higher-level crates must it never know about?
* Which commands or messages does it accept?
* Which facts does it publish?
* Which resources are authoritative?
* Which plugin installs it?
* Which schedule sets does it expose?
* What is the smallest composition example?
* Where should a developer add a new implementation?

The existing module maps are useful, but generated maps must remain synchronized with source and manifests.

## Make the correct path the shortest path

Weak agents will use whatever API is easiest.

Therefore:

* Do not expose broad mutable save internals.
* Do not permit direct input reads from arbitrary gameplay systems.
* Do not keep fallback room-lowering paths.
* Do not keep a generic `UseSystem` escape hatch.
* Do not expose public runtime components when commands or bundles are sufficient.
* Do not make a facade prelude include every internal type.
* Do not keep compatibility aliases indefinitely.
* Do not require editing a giant runtime tuple to add a local domain system.

Compiler privacy, crate topology, typed commands, and narrow plugin APIs are stronger than prose.

## Curate the public SDK

The `ambition` facade should provide a batteries-included experience, but its prelude should contain stable extension concepts rather than every internal implementation detail.

Useful public surfaces include:

* Provider authoring.
* World and room IR.
* Entity and action catalog authoring.
* Placement registration.
* Actor and control contracts.
* Session and lifecycle facts.
* Presentation projection interfaces.
* Deterministic test harness APIs.

Advanced engine internals can remain accessible through explicitly named modules without being placed in the common prelude.

## Build an external sample game as an architectural oracle

A fifth, deliberately small provider should exercise:

* One custom character.
* One custom action.
* One custom held item.
* One custom placement interpreter.
* One custom movement or frame policy.
* One encounter.
* Save and restore.
* Headless simulation.
* Replaceable presentation.

It should not edit any engine crate.

Every piece of copied boilerplate or required internal import is evidence of a missing engine surface.

## Editor and authoring architecture

Rivaling Unity or Godot requires more than runtime elegance. The current catalog/IR/lowering design can support good tools, but the tooling must use the same compiler path as runtime.

Priorities include:

* Schema-aware diagnostics.
* Stable authored identifiers.
* Content versioning and migration.
* Live reload through the normal lowering pipeline.
* Inspection of the lowered runtime form.
* Validation before session activation.
* Clear source locations in errors.
* Content packages that register catalogs and placements without modifying engine code.

Do not build a separate editor-only world compiler. That would create another authoritative path.

---

# Staged execution plan

## Phase 0: establish behavioral oracles

Before moving architecture, encode the intended equivalences.

Add tests for:

* Provider preparation identity isolation.
* Provider switching without stale scoped entities.
* Initial room activation versus transition equivalence.
* Custom placement behavior across every room lifecycle.
* Shell-driven versus direct/headless session construction.
* Controlled subject versus home player behavior.
* Current action-path census.
* Snapshot round trips for the scenarios currently claimed to be lossless.
* Minimal feature configurations.

Produce or update generated documentation for:

* Crate graph.
* Plugin ownership.
* Public schedule-set graph.
* Resource authority.
* Session lifecycle.
* Room construction.
* Action ingress.
* Observation production.
* Snapshot registration.

### Gate

No major move begins until a test would detect accidental preservation or creation of a second path.

## Phase 1: extract provider infrastructure mechanically

Create `ambition_platformer_provider`.

Move provider preparation and session-builder implementation without changing behavior. Reexport it from `ambition`.

Then introduce a reusable typed provider lifecycle plugin to remove repeated preparation/activation/cleanup glue.

### Gate

* All existing providers still activate through exact prepared identities.
* Headless and visible hosts can use the provider subsystem.
* No host-side provider match statement is introduced.
* The facade contains curation, not hidden provider implementation.
* Provider catalogs remain App-local and isolated.
* No long-lived compatibility implementation remains in the facade.

## Phase 2: make session and room construction authoritative

* Create the session root first.
* Resolve a session configuration independent of dev-edit resources.
* Materialize the active room through the installed registry.
* Spawn all room and actor entities with explicit session ownership.
* Remove `SceneEntities`.
* Give moving-platform and collision-overlay caches explicit scope and invalidation.
* Route initial activation, reset, transition, hot reload, and restore through one room materializer.
* Remove hard-coded production lowering fallbacks.

### Gate

* Custom placements work during every lifecycle.
* Relaunching or switching a provider leaves no stale handles or collision state.
* Headless and visible modes use the same simulation construction path.
* Every room-derived cache has explicit source session and invalidation ownership.
* No session-owned truth has an undocumented process-global mirror.

## Phase 3: complete action convergence

* Move concrete held-item catalogs out of the character crate.
* Extend `MoveSpec`/`MovePlayback` for hold, release, charge, channel, repeated ticks, aiming, and persistent effects.
* Migrate ranged and held-item mechanic families.
* Delete each bypass immediately after migration.
* Align capability ownership with action bindings.

### Gate

* One action execution law exists.
* No arbitrary gameplay `UseSystem` callback.
* No separate player-projectile action state machine.
* No migration marker that merely skips an old execution path.
* AI, human, possessed, replay, and RL action initiation converge.
* Action clock ownership is explicit.

## Phase 4: localize domain registration and resource mutation

* Give combat, projectiles, encounters, actor adapters, and related domains proper owner plugins.
* Move local resource/message initialization into those plugins.
* Expose public domain schedule sets.
* Keep global set ordering in runtime.
* Introduce typed reducers or operations for banner, inventory, quest, and save-facing state.
* Eliminate runtime reexport tunnels where an explicit adapter belongs.

### Gate

* Runtime schedule code mostly names architectural sets rather than leaf functions.
* Every durable resource names its owner and mutation algebra.
* Direct mutable access is restricted to the owner where appropriate.
* Cross-domain operations use typed messages or commands.
* Global phase ordering remains visible in one place.

## Phase 5: make Cargo modularity match conceptual modularity

* Make heavy facade dependencies optional.
* Use minimal library defaults.
* Move persona bundles upward.
* Separate portal simulation from presentation.
* Separate code-authored world support from LDtk support.
* Audit `ambition_actors` and runtime feature gates.
* Add the full feature-matrix CI.

### Gate

The minimal headless build does not compile or link:

* Renderer.
* Audio backend.
* Windowing.
* Touch input.
* LDtk.
* Portal presentation.
* Inspector or dev UI.

Optional domains compile in combinations that the public feature model claims to support.

## Phase 6: finish restore and observer architecture

* Introduce typed dynamic spawn recipes.
* Reconstruct sessions and rooms through the normal provider and lowering paths.
* Move standard RL observation onto canonical `SimView` facts.
* Add observer identity and projection.
* Reduce known snapshot resource debt.
* Make supported lossless restore scenarios contractual.

### Gate

* Save, rollback, replay, and reconstruction do not maintain independent world builders.
* Canonical RL and rendering observe the same fact model.
* Observer-specific delay or transformation can be added without changing authoritative simulation.
* Lossless restore is proven for defined scenarios.

## Phase 7: refine the mathematical kernel through concrete mechanics

* Distinguish physical surface law from mechanic overlays.
* Split capability ownership into typed domains.
* Audit timers by clock domain.
* Clarify frame semantics at subsystem boundaries.
* Add environmental field sampling.
* Implement the concrete proper-time, moving-frame, rotating-room, and delayed-observer examples.

### Gate

* Ordinary games remain simple identity-frame and identity-clock cases.
* New mechanics compose from existing primitives.
* No stringly property bags or general-purpose service registries are introduced.
* Mathematical laws have executable property tests.
* The trusted movement kernel remains understandable in one place.

## Phase 8: validate third-party usability

Build the independent sample provider/game and use it as a release gate.

Measure:

* Number of engine internals imported.
* Amount of copied setup code.
* Number of engine crates edited.
* Build size and compile time.
* Quality of diagnostics.
* Whether headless and visible composition differ.
* Whether a new placement, action, or actor follows one obvious path.

---

# Priority ranking

| Priority | Work                                                          | Architectural value |        Risk |
| -------: | ------------------------------------------------------------- | ------------------: | ----------: |
|        1 | Unify all room-materialization paths                          |           Very high |  Low–medium |
|        2 | Finish session-root authority and remove global scene handles |           Very high |      Medium |
|        3 | Extract `ambition_platformer_provider`                        |           Very high |  Low–medium |
|        4 | Consolidate provider lifecycle boilerplate                    |                High |      Medium |
|        5 | Complete action convergence                                   |           Very high |        High |
|        6 | Make `SimView` canonical for RL and observers                 |           Very high |      Medium |
|        7 | Move local system ownership into domain plugins               |                High |      Medium |
|        8 | Make Cargo features genuinely modular                         |                High | Medium–high |
|        9 | Complete encounter convergence                                |                High |      Medium |
|       10 | Converge restore with normal construction                     |           Very high |        High |
|       11 | Refine surface and capability vocabulary                      |                High |      Medium |
|       12 | Deepen frame, time, and observer semantics                    |      Transformative |        High |
|       13 | Build the external sample game and tool-facing workflow       |           Very high |      Medium |

# Final assessment

The codebase already contains many of the ingredients of a serious engine:

* A common body simulation law.
* Explicit control relationships.
* Data-driven action playback.
* Backend-independent world IR.
* App-local extension registries.
* Exact prepared-session identities.
* Session scopes.
* Determinism policies.
* A one-way simulation-view boundary.
* Portal and proper-time foundations.
* Headless execution and evaluation support.
* Strong architecture tests.

The repository’s rapid growth is dangerous mainly because several migrations are incomplete at the same time. Each incomplete migration leaves two plausible answers to an architectural question:

* Which state owns the session?
* Which registry constructs a placement?
* Which path executes an action?
* Which view defines observation?
* Which system owns a resource?
* Which builder reconstructs the world?
* Which Cargo feature actually excludes a subsystem?

The most elegant next stage is not maximal decomposition. It is **architectural exclusivity**.

Once the engine has one mechanically true answer to each of those questions, the crate graph will become easier to navigate, the runtime will become easier to debug, and coding agents will have fewer incorrect paths available to them. The resulting primitives—session-scoped worlds, body motion, typed actions, fields, frames, clocks, observer projections, and deterministic lowering—are also the right basis for both conventional platformers and the more mathematical, relativistic games the engine is intended to support.
