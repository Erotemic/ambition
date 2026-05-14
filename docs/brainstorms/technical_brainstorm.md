# Technical Brainstorm: Bevy ECS, Event Buses, and Expansion Direction

> Status update (2026-05-14): this document is historical. The active sandbox
> implementation has since migrated feature families out of `FeatureRuntime`
> and into ECS entities/components. Use it for migration context, not as a
> description of current runtime ownership.

Date: 2026-05-13
Context: notes from a design conversation about Ambition's Bevy architecture, event/message buses, ECS/data-driven thinking, and crates that might prevent reinventing wheels.

This is intentionally a brainstorm document, not an ADR. It records ideas to consider, including options that were rejected or should only be revisited later.

## Executive summary

The current design is directionally sound: Ambition already uses a narrow, typed sim-to-presentation message seam (`SfxMessage`, `VfxMessage`, `DebrisBurstMessage`, `PlayerDiedMessage`) and a typed gameplay-effect stream (`GameplayEffect`) instead of letting gameplay code directly call audio, VFX, save, quest, boss, and encounter systems.

The main architectural risk is not that Ambition lacks a more powerful event bus crate. The bigger risk is that `FeatureRuntime` is becoming a custom mini-ECS inside a Bevy resource. That can remain useful for early deterministic prototyping, but as the game expands, more runtime actors should gradually become Bevy entities with components, queries, messages, observers, and per-entity state machines.

Recommended north star:

```text
Keep the current typed, narrow message seam.
Avoid a generic global bus crate for local gameplay.
Move simple feature families from FeatureRuntime Vecs into ECS entities over time.
Use Bevy Message for scheduled many-reader streams.
Use Bevy observers only for immediate/entity-local reactions.
Use seldom_state more as the per-entity state machine wheel.
Use bevy_ecs_ldtk more directly for authored entities/components.
```

## What Rust and Bevy can optimize away

Rust can usually optimize much of the wrapper/abstraction overhead:

- `SystemParam` wrapper structs
- newtype wrappers
- small helper functions
- generic functions after monomorphization
- straightforward iterator/drain loops
- many enum matches

Rust cannot optimize away semantic work that changes program behavior:

- pushing messages into a `Vec`
- growing/clearing queues
- moving/copying event payloads
- draining messages into Bevy `Messages<T>`
- reader bookkeeping
- iterating over messages
- running separate systems

So a Bevy typed message path is much better than a dynamic `HashMap<TypeId, Vec<Box<dyn Any>>>` style global bus, but it still is not literally free. That is fine for low-frequency semantic gameplay events. Avoid using the bus for very hot data like every projectile movement, particle, collision contact, or tile test.

## Current Ambition event/message architecture

### Bevy sim-to-presentation messages

`add_simulation_plugins` registers the core local message channels:

```rust
app.add_message::<SfxMessage>()
    .add_message::<VfxMessage>()
    .add_message::<DebrisBurstMessage>()
    .add_message::<PlayerDiedMessage>();
```

Current flow:

```text
sandbox_update phases
    -> FrameFeedback Vecs
    -> flush_feedback
    -> MessageWriter<T>
    -> presentation/consumer systems with MessageReader<T>
```

This is a good narrow pattern. It keeps deep helper functions mostly pure-ish and testable without giving every helper direct access to `Commands`, audio systems, particle spawners, or physics-debris presentation.

### `FrameFeedback`

Current shape:

```rust
pub(super) struct FrameFeedback {
    pub(super) sfx: Vec<SfxMessage>,
    pub(super) vfx: Vec<VfxMessage>,
    pub(super) debris: Vec<DebrisBurstMessage>,
    pub(super) died: Vec<PlayerDiedMessage>,
}
```

`flush_feedback` writes these batches into Bevy messages at the end of the simulation tick. This is a reasonable compromise while the core update loop is still phase-helper driven.

Keep this unless one of these becomes true:

- helpers become regular Bevy systems;
- message counts become large enough to matter in profiles;
- the same event has to be observed immediately before the end-of-frame flush;
- event ordering becomes hard to reason about.

### `FeatureEventBus`

Current shape:

```rust
#[derive(Resource, Default)]
pub struct FeatureEventBus {
    pub effects: Vec<GameplayEffect>,
}
```

Current `GameplayEffect` variants include:

```rust
SetFlag { id, on }
AdvanceQuest(QuestAdvanceEvent)
ActivateSwitch { payload, pos }
DamageBoss { boss_id, amount }
StrikeNpc { npc_id, pos }
PlaySfx { id, pos }
```

The bus contract is currently:

```text
sandbox_update emits feature effects
update_projectiles can emit projectile-hit effects
drain_feature_event_bus drains the resource
drain_feature_event_bus routes to save / quest / switch / boss / cutscene / music / audio
```

This is useful because it stopped the spread of parallel stringly typed vectors. But as `drain_feature_event_bus` grows, it may become a central god-router.

## Bevy-specific architecture notes

### Bevy `Message` is the built-in local event bus

For normal in-process gameplay pub/sub, prefer Bevy's built-in `Message`, `MessageWriter<T>`, and `MessageReader<T>`. They are typed, scheduled, many-reader, and already integrated with ECS.

Do not add a crate just to get a local bus unless Ambition needs semantics Bevy does not provide, such as persistence, broker transport, delivery acknowledgements, or custom consumption/bubbling.

### Bevy `Event` / observers are different

Use observers for immediate, entity-scoped, or lifecycle-like reactions.

Good candidates:

```text
On chest opened -> spawn sparkle/view update
On component added -> initialize child view entity
On entity damaged -> flash sprite / attach temporary marker
On entity despawned -> cleanup related presentation entity
```

Avoid making observers the main gameplay event stream for deterministic scheduled simulation. `Message` fits the current sim flow better.

### `.add_systems` tuple limits are not an architecture limit

Tuple limits for `.add_systems(Update, (...))` or system params are API/trait convenience limits, not limits on the number of systems in the app.

Use these patterns instead of trying to fit everything into one tuple or one giant system signature:

```rust
app.add_systems(Update, system_a)
    .add_systems(Update, system_b)
    .add_systems(Update, system_c);
```

Group by plugin:

```rust
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlayerDied>()
            .add_systems(Update, apply_damage)
            .add_systems(Update, detect_deaths)
            .add_systems(Update, play_death_sounds)
            .add_systems(Update, show_death_ui);
    }
}
```

Bundle repeated params with `#[derive(SystemParam)]` when it improves readability:

```rust
#[derive(SystemParam)]
pub struct SandboxEventWriters<'w> {
    pub sfx: MessageWriter<'w, SfxMessage>,
    pub vfx: MessageWriter<'w, VfxMessage>,
    pub debris: MessageWriter<'w, DebrisBurstMessage>,
    pub died: MessageWriter<'w, PlayerDiedMessage>,
}
```

This is a good technique, but it should be used to organize related data, not to hide a system that wants to do too much.

## Data-driven vs code-driven Bevy thinking

Ambition is code-first, but Bevy is data-driven. Those are not opposites.

A healthy Bevy data-driven design means:

```text
state lives as components/resources/assets in the World
systems are code that transforms that data
content/tuning lives in LDtk/RON/assets where practical
presentation is separate from reusable model/mechanics
```

It does not mean "no code". It means code should operate over data in ECS-friendly shapes.

Current concern:

```rust
pub struct FeatureRuntime {
    pub hazards: Vec<HazardRuntime>,
    pub enemies: Vec<EnemyRuntime>,
    pub bosses: Vec<BossRuntime>,
    pub breakables: Vec<BreakableRuntime>,
    pub pickups: Vec<PickupRuntime>,
    pub chests: Vec<ChestRuntime>,
    pub npcs: Vec<NpcRuntime>,
    pub switches: Vec<SwitchRuntime>,
    pub banner: String,
    pub banner_timer: f32,
}
```

This is a pragmatic prototype structure, but it means Bevy sees one large resource rather than many gameplay entities. That limits use of:

- per-entity change detection;
- query composition;
- per-entity debug inspection;
- observer hooks;
- entity-local state machines;
- parallel scheduling;
- relationships/ownership;
- despawn-on-room-unload patterns;
- save/load by component groups.

So the likely long-term mistake would be letting `FeatureRuntime` keep absorbing more gameplay systems indefinitely.

## Recommended migration direction

Do not migrate everything at once. Pick one simple feature family and make it ECS-native.

Best first candidates:

```text
Pickups
Chests
Breakables
Switches
```

Avoid starting with:

```text
Player controller
Bosses
Room transitions
Moving platforms
```

Example target shape for pickups:

```rust
#[derive(Component)]
struct Pickup {
    id: String,
    kind: PickupKind,
}

#[derive(Component)]
struct Collectible;

#[derive(Component)]
struct PersistKey(String);
```

Then a Bevy system can own collection:

```rust
fn collect_pickups(
    mut commands: Commands,
    player: Query<&Transform, With<Player>>,
    pickups: Query<(Entity, &Transform, &Pickup), With<Collectible>>,
    mut sfx: MessageWriter<SfxMessage>,
    mut effects: MessageWriter<GameplayEffect>,
) {
    // query entities, test overlap, despawn/mark collected,
    // emit messages/effects
}
```

This makes the game more Bevy-native while preserving the typed message boundary.

## Specific TODO directions

### TODO 1: Keep the narrow event/message seam

Keep the current sim-to-presentation shape:

```text
simulation helpers -> FrameFeedback -> Bevy Messages -> presentation systems
```

This is still the right shape for audio/VFX/debris/death notifications.

Acceptance ideas:

- `sandbox_update` should not directly play audio or spawn presentation-only effects.
- headless/minimal tests should still be able to drive gameplay without rendering/audio plugins.
- event/message assertions should remain part of scripted gameplay tests.

### TODO 2: Consider converting `GameplayEffect` into a Bevy `Message`

Instead of a custom `FeatureEventBus` resource, consider:

```rust
#[derive(Message, Clone, Debug, PartialEq)]
pub enum GameplayEffect {
    SetFlag { id: String, on: bool },
    AdvanceQuest(QuestAdvanceEvent),
    ActivateSwitch { payload: String, pos: Vec2 },
    DamageBoss { boss_id: String, amount: i32 },
    StrikeNpc { npc_id: String, pos: Vec2 },
    PlaySfx { id: SfxId, pos: Vec2 },
}
```

Then split the current router into smaller consumer systems:

```text
apply_flag_effects
apply_quest_effects
apply_switch_effects
apply_boss_damage_effects
apply_npc_strike_effects
apply_gameplay_sfx_effects
```

Potential benefits:

- removes custom bus resource;
- lets each consumer use `MessageReader<GameplayEffect>` independently;
- reduces god-router pressure;
- makes scheduling/order dependencies more explicit with system sets;
- aligns with Bevy's built-in message wheel.

Potential downside:

- each consumer scans the same effect stream and filters variants;
- splitting systems may introduce ordering concerns that are currently centralized;
- current custom bus gives explicit drain timing and can be easier to reason about during prototype phases.

This is a good medium-term refactor, not necessarily urgent.

### TODO 3: If scanning one enum stream gets ugly, split by domain

Alternative to one `GameplayEffect` enum:

```text
SaveFlagEffect
QuestEffect
SwitchActivation
BossDamageEvent
NpcStrikeEvent
GameplaySfxRequest
```

This reduces per-system filtering and makes ownership clearer, but can also make producers more verbose.

Possible rule:

- keep one `GameplayEffect` while the vocabulary is small and cross-cutting;
- split into domain messages when one variant family starts needing multiple consumers or payload-specific ordering.

### TODO 4: Migrate one feature family from `FeatureRuntime` to ECS

Pick a simple family and make it a vertical slice:

```text
LDtk/RON authored data -> Bevy entity/components -> ECS system behavior -> Messages/effects -> save/view/audio
```

Suggested first slice: pickups or chests.

Acceptance ideas:

- authored object spawns as an entity with typed components;
- system queries those components instead of looping through `FeatureRuntime.pickups` or `FeatureRuntime.chests`;
- collection/opening still emits SFX/VFX/progression messages;
- save/persistence key remains explicit;
- old runtime path can coexist until the slice is proven.

### TODO 5: Use `seldom_state` more fully for per-entity state

Ambition already has `seldom_state` as a dependency/foundation. This is likely the most relevant wheel already in the repo.

Use it for things like:

```text
ChestClosed -> ChestOpening -> ChestOpened
BreakableIntact -> BreakableCracking -> BreakableBroken -> BreakableRespawning
EnemyIdle -> EnemyPatrol -> EnemyTelegraph -> EnemyAttack -> EnemyRecover
BossDormant -> BossIntro -> BossPhase1 -> BossPhase2 -> BossStaggered -> BossDefeated
```

Recommended approach:

- do not migrate every enemy at once;
- choose one simple enemy or chest/breakable first;
- keep old runtime/state mirror until tests prove equivalence;
- use existing state vocabulary in `ambition_engine::state_machines`.

Note: checked current docs.rs while writing this note: latest `seldom_state` docs currently show 0.15.0 depending on Bevy 0.17.x, while this repo currently uses `seldom_state = 0.16` and Bevy 0.18.1. Re-check crate compatibility before changing versions.

### TODO 6: Lean harder on `bevy_ecs_ldtk` where practical

The current docs already say LDtk is a first-class adapter target. Continue moving toward:

```text
LDtk entity/layer data
    -> bevy_ecs_ldtk asset/entity path
    -> typed Ambition components/bundles
    -> ECS systems own behavior
```

Avoid treating LDtk only as data that gets copied into one monolithic runtime resource. The more LDtk objects can become actual Bevy entities, the more Bevy can help with queries, visibility, relationships, inspectors, observers, despawns, and per-entity behavior.

### TODO 7: Keep `ambition_engine` Bevy-native but presentation-light

The existing boundary is right:

```text
ambition_engine
  movement, collision semantics, abilities, combat, actors, reusable mechanics,
  geometry, state-machine vocabulary, data types

ambition_sandbox / future story crates
  app setup, LDtk/RON content, rendering, input bindings, HUD/debug UI,
  audio playback, presentation choices, temporary labs
```

Do not regress to "engine must be Bevy-independent" if Bevy-adjacent crates remove bespoke infrastructure. The better boundary is reusable mechanics vs sandbox/story presentation.

### TODO 8: Keep custom player movement for now

Do not replace the bespoke player controller with a general character-controller crate just because one exists. Movement feel is the project's core differentiator.

Possible exception: evaluate external controllers only for non-player actors if enemy/NPC motion becomes physics-heavy and the current code becomes a maintenance burden.

### TODO 9: Use profiling before optimizing bus overhead

The bus/message overhead is probably not the bottleneck for low-frequency semantic events. Measure before changing architecture for performance.

Profile questions:

- message counts per frame for SFX/VFX/debris/gameplay effects;
- allocation/capacity churn in `FrameFeedback` vectors;
- cost of `drain_feature_event_bus` as effects grow;
- cost of VFX/debris spawning relative to message overhead;
- cost of large `FeatureRuntime` loops as entity counts rise.

If message counts get high, consider:

- reserving/reusing vector capacity;
- splitting high-volume streams from semantic streams;
- direct ECS queries for hot paths;
- fixed timestep data-oriented loops for dense interactions.

## Crates and wheels to consider

### Keep / expand current dependencies

#### `seldom_state`

Purpose: component-based state machines for Bevy entities.

Recommendation: use more fully for per-entity state, especially once features become ECS entities.

Risk: confirm version compatibility against Bevy 0.18.1 before upgrading. The repo currently uses `seldom_state = 0.16`; docs.rs latest visible page checked during this brainstorm showed 0.15.0 with Bevy 0.17.x dependencies.

#### `bevy_ecs_ldtk`

Purpose: LDtk-to-Bevy asset/entity workflow.

Recommendation: continue using it as the external level-editor adapter, but push more LDtk-authored things toward typed Bevy bundles/components instead of only mirroring into `FeatureRuntime`.

#### `bevy_asset_loader` and `bevy_common_assets`

Purpose: explicit loading states and serde-backed asset loading.

Recommendation: keep expanding for RON manifests, generated content specs, and loading-state clarity.

#### `leafwing-input-manager`

Purpose: semantic input actions.

Recommendation: keep. It matches the existing action-driven input model and supports code-driven movement feel.

### Add only when the pain is real

#### `bevy_hanabi`

Purpose: GPU particle system for Bevy.

Recommendation: consider if CPU sprite particles become heavy or VFX authoring becomes central. Keep the `VfxMessage` API stable so the backend can change later.

Current docs.rs check: latest visible `bevy_hanabi` is 0.18.0 and depends on Bevy 0.18, so it appears version-aligned with the repo's Bevy 0.18.1.

#### `bevy-persistent`

Purpose: persistent Bevy resources across sessions.

Recommendation: consider for settings, keybindings, window/audio/debug preferences, or simple resource persistence. Use caution for core save-game data if explicit migrations/versioning matter.

Current docs.rs check: latest visible `bevy-persistent` is 0.10.0 and depends on Bevy 0.18.

#### `moonshine_save`

Purpose: selective Bevy world save/load.

Recommendation: consider later, after more gameplay state lives as ECS entities. Its philosophy of separating saved model state from view entities matches Ambition's likely direction, but adopting it too early could add complexity.

Current docs.rs check: latest visible `moonshine-save` is 0.6.1 and depends on Bevy 0.18. Its docs explicitly emphasize selective world saving and model/view separation, and also state it does not provide built-in backwards compatibility/versioning/validation.

#### `bevy_tnua`

Purpose: character controller / floating controller patterns.

Recommendation: probably not for the player. Maybe evaluate later for non-player actors if custom movement becomes too costly.

#### `bevy_seedling`

Purpose: Bevy audio routing/effects based on Firewheel.

Recommendation: only revisit if Kira/FunDSP/audio routing becomes a bottleneck. Not an architecture fix for gameplay.

### Probably reject for current problem

#### `bevy_event_bus`

Purpose: bridge Bevy messages/events to external brokers like Kafka/Redis.

Decision: reject for local gameplay bus needs. It is useful for distributed/external eventing, telemetry, dashboards, tools, or multi-process architecture. It is not a replacement for Bevy's in-process `Message` system.

Current docs.rs check: latest visible `bevy_event_bus` is 1.1.4, depends on Bevy 0.17.3, and describes itself as connecting Bevy events to external message brokers like Kafka.

Possible future use case:

```text
Bevy game/server emits match telemetry -> Kafka/Redis -> analytics/admin/replay systems
```

Not current use case:

```text
Bevy system A -> local gameplay bus -> Bevy system B
```

#### `bevy_eventlistener`

Decision: reject unless there is a very specific need for older DOM-like bubbling/listener semantics. Bevy's own observer system covers much of the need now, and older crates may lag Bevy versions.

#### A custom generic `EventBus<T>` or `Any`/`TypeId` registry

Decision: reject for now.

Reasons:

- Bevy `Message<T>` is already the typed local bus.
- Dynamic type-erased buses add dispatch/downcast/allocation complexity.
- A project-specific registry can become a second ECS.
- Narrow typed messages are easier to test, trace, and optimize.

#### `bevy_rl`

Decision: defer. The current `SandboxSim` / `ControlFrame` / `AgentObservation` seam is probably the right place to keep RL integration until crate compatibility and API fit are clear.

#### `bevy_save`

Decision: probably reject for core game saves unless it clearly handles versioning/migration in a way Ambition needs. The current explicit save schema may be safer for long-lived games.

## Possible implementation plan

### Phase 0: Documentation and metrics

- Keep this document as a brainstorm note.
- Add a small metric/debug report for per-frame counts of `SfxMessage`, `VfxMessage`, `DebrisBurstMessage`, `PlayerDiedMessage`, and `GameplayEffect`.
- Add comments near `FeatureRuntime` explaining that it is a transitional prototype runtime, not the intended final shape for all actors.

### Phase 1: Message refactor experiment

- Convert `GameplayEffect` to `#[derive(Message)]` in a branch.
- Replace `FeatureEventBus` with direct `MessageWriter<GameplayEffect>` at the flush/drain seam.
- Split `drain_feature_event_bus` into several consumer systems.
- Use system sets to make ordering explicit.
- Compare test clarity and code churn.

Success criteria:

- fewer central router responsibilities;
- no loss of same-frame behavior;
- scripted gameplay tests pass;
- ordering is clearer, not more implicit.

Abort criteria:

- ordering becomes difficult to reason about;
- many systems repeatedly filter the same stream with no benefit;
- code becomes more verbose without reducing coupling.

### Phase 2: ECS-native pickup/chest slice

- Pick one simple feature family.
- Spawn from LDtk/RON into actual Bevy entities/components.
- Implement behavior with queries.
- Emit existing SFX/VFX/gameplay messages.
- Preserve save/progression semantics.
- Keep old `FeatureRuntime` path temporarily for other families.

Success criteria:

- entity is inspectable in Bevy world inspector;
- tests can spawn the entity in a minimal App;
- behavior does not require mutating one large `FeatureRuntime` resource;
- the migration pattern is reusable.

### Phase 3: State machine slice

- Pick one stateful entity family, probably chest/breakable or one enemy.
- Use existing `ambition_engine::state_machines` vocabulary.
- Add `seldom_state` state machine components.
- Mirror or replace old runtime state only for that family.

Success criteria:

- state transitions are visible as component changes;
- debug tooling can inspect state;
- state-specific behavior is cleaner than the old enum/timer loop.

### Phase 4: Save/model-view evaluation

After enough gameplay state is ECS-native, evaluate whether `moonshine_save` or an Ambition-specific explicit save mapper is a better fit.

Do not evaluate this too early, because the answer depends on whether saved state is mostly ECS components or still custom resources/specs.

## Rejected / deferred ideas recap

| Idea | Decision | Reason |
|---|---|---|
| Bigger generic local event bus crate | Reject | Bevy `Message<T>` already solves local typed pub/sub. |
| `bevy_event_bus` for local gameplay | Reject | It targets external Kafka/Redis-style brokers. |
| Dynamic `Any`/`TypeId` bus | Reject | Adds type erasure and runtime dispatch while duplicating Bevy. |
| Rewrite all features into ECS now | Reject | Too much churn; migrate one family at a time. |
| Replace player controller with external crate | Reject for now | Movement feel is core to Ambition. |
| Use observers everywhere | Reject | Use observers selectively for immediate/entity-local reactions. |
| Adopt `moonshine_save` immediately | Defer | More useful after ECS-native model state exists. |
| Adopt `bevy_hanabi` immediately | Defer | Keep VFX message API stable; swap backend when needed. |

## Good design heuristics going forward

- If a behavior is a hot local transformation, use direct data/query code.
- If a behavior is a semantic cross-system notification, use a typed Bevy message.
- If a behavior is immediate and entity-local, consider an observer.
- If an actor has local state transitions, consider `seldom_state`.
- If authored content creates gameplay objects, prefer typed ECS components/entities over only copying into a global runtime resource.
- If a resource contains many homogeneous actor vectors, ask whether it is becoming a mini-ECS.
- If a system signature is huge, split by responsibility first; use `SystemParam` grouping second.
- If a crate solves infrastructure cleanly and is Bevy-version compatible, prefer it over bespoke machinery.
- If a crate solves a different problem than Ambition has, reject it even if the name sounds perfect.

## Open questions

- Which feature family should be the first ECS-native migration: pickups, chests, breakables, or switches?
- Should `GameplayEffect` remain one enum stream, or split into domain-specific messages once the router is decomposed?
- How much of `FeatureRuntime` is still valuable as a deterministic/headless adapter after simple entities move into ECS?
- Should generated/audio/VFX assets become more data-authored before or after the ECS migration?
- When does the save system need full ECS-world selective saving versus explicit versioned save structs?
- How much authoring should LDtk own versus RON/code-generated specs?
- Where should the first vertical slice live so architecture work is tested against real game feel rather than isolated labs?

## Concrete next patch candidate

A low-risk next patch would be:

1. Add instrumentation/debug counters for per-frame message/effect counts.
2. Add a source comment near `FeatureRuntime` stating that it is a transitional feature-prototype runtime.
3. Add a small design note or TODO near `FeatureEventBus` saying the medium-term candidate is `GameplayEffect: Message` plus smaller consumer systems.
4. Pick pickups or chests for the first ECS-native migration branch.

That patch would not commit to a large migration, but it would make the intended direction explicit and reduce the chance that future features automatically expand the mini-ECS resource.
