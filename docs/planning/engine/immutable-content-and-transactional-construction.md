# Immutable content assembly and transactional construction

> **State:** PLANNED (2026-07-17).
>
> **Priority:** the next major engine-architecture campaign after the active
> character action/control migration in [`character-actions.md`](character-actions.md).
> That migration is a prerequisite but is otherwise out of scope here.
>
> **Strategic role:** this plan builds the substrate beneath reusable entity
> recipes, prefab-like authoring, transactional hot reload, exact reconstruction,
> saves, rollback, content diagnostics, and a stable external SDK.
>
> **Companions:** [`architecture.md`](architecture.md), [`netcode.md`](netcode.md),
> [`spatial-model.md`](spatial-model.md),
> [`room-transition-loading.md`](room-transition-loading.md), and
> [`decisions-2026-07-16.md`](decisions-2026-07-16.md). The room-transition plan
> is the immediate room-lifecycle customer of this campaign: it composes load
> coordination, adaptive presentation, asset readiness, and the future
> construction transaction so ordinary room changes never expose a partial
> target.

## 0. Product objective: compete with Unity, Unreal, and Godot

Ambition's north star is not merely to become a clean bespoke runtime for one
platformer. It is to become a **Unity/Godot-class 2D platformer engine, with the
expressive ceiling and systems ambition associated with Unreal, built the Bevy
and Rust way**. The canonical product vision is in [`../vision.md`](../vision.md).

That does **not** mean copying those engines' editor-first object models. Ambition
is Bevy-native, deterministic where it matters, headless-first, provider-driven,
and willing to use external spatial authoring tools. It should compete on the
outcomes professional engine users depend on:

- reusable and composable game content without editing engine core;
- fast, safe iteration with complete validation before destructive changes;
- inspectable entity construction and actionable source-located diagnostics;
- stable identity across loading, reset, hot reload, saves, replay, and rollback;
- deterministic builds and sessions whose content can be fingerprinted and
  reproduced;
- extension seams strong enough for another game team to author a platformer
  through supported engine surfaces;
- tooling and data models that future visual inspectors, editor backends, and
  agents can understand rather than opaque imperative spawn code;
- architectural advantages over editor engines where Ambition's design permits
  them: exact headless simulation, transactional world replacement, principled
  rollback, and one shared runtime for humans, brains, RL, and replay.

Unity prefabs and ScriptableObjects, Godot scenes and Resources, Unreal assets and
object construction, and all three engines' import databases solve important user
problems even when their implementation models are not appropriate here. Ambition
needs functional answers to those problems. This campaign addresses the common
foundation: **what exact content is active, where it came from, what entities it
will construct, and whether a new world can be proven valid before it replaces the
old one.**

### The goal outranks this proposed mechanism

Future agents should preserve the competitive outcomes and invariants in this
document, not blindly preserve its provisional type names, phase boundaries, or
implementation sketches. A stronger design is welcome when it:

1. moves Ambition more directly toward a professional engine workflow;
2. retains one authoritative construction and content lifecycle;
3. remains deterministic, inspectable, headless-testable, and provider-open;
4. reduces rather than multiplies runtime authorities;
5. demonstrates the improvement through the acceptance tests defined here.

If the code reveals a better route—such as a Bevy-native facility, an ecosystem
crate, or a simpler model that satisfies more of the product objective—revise this
plan and take it. Do not weaken the goal merely because one proposed structure is
awkward. Conversely, do not import an editor engine's object model by analogy when
an ECS-native solution is cleaner.

### Competitive outcomes this campaign must unlock

This campaign is successful only if it materially advances several engine-product
capabilities, not merely if new registries and structs exist.

| Engine-user need | Capability unlocked by this campaign |
|---|---|
| Reusable entity/scene authoring | One validated construction model from which public recipes or prefab-like facilities can later emerge |
| Safe iteration | Candidate content and worlds are prepared off to the side and committed only after validation |
| Inspector/editor support | Construction intent, provenance, relationships, schemas, and diagnostics are data that tools can inspect |
| Reliable hot reload | A new immutable content epoch replaces the old one transactionally instead of mutating live registries piecemeal |
| Saves, replay, and rollback | State is bound to explicit content and schema identity; reconstruction does not depend on naming tricks |
| External game development | Providers register content through deterministic, documented ownership and conflict rules |
| Reproducible builds and bugs | Meaningful content fingerprints identify the exact definitions a session used |
| Agentic development | Headless tools can enumerate planned changes and diagnose invalid content before touching a live world |

A public prefab API is **not** the first milestone. The immediate goal is the
architecture beneath it, proven by real lifecycle customers.

## 1. Executive summary

The engine currently has several successful but independently evolved mechanisms:

- provider content installation;
- character and audio catalogs;
- placement lowering;
- content-staged actor requests;
- direct room-specific spawn loops;
- snapshot registries;
- dynamic-anchor reconstruction;
- hot-reload preparation and commit logic.

These mechanisms do not yet share one lifecycle, one conflict policy, one content
identity, or one construction model. Generalizing the existing projectile restore
or placement-lowering implementation directly into a public prefab API would risk
freezing accidental machinery—such as string-derived identity and imperative
`Commands` mutation—into the engine surface.

The architectural target is:

> **Provider-owned content is collected, validated, assembled, fingerprinted, and
> frozen into an immutable session content epoch. World mutation is driven by a
> pure, preflightable construction plan carrying explicit entity provenance.**

The intended flow is:

```text
provider fragments and authored sources
    → structured validation and conflict detection
    → deterministic PreparedContent assembly
    → ContentFingerprint + immutable ContentEpoch
    → pure ConstructionPlan
    → transactional execution or discard
    → active session/world
```

Normal room activation, reset, transition, hot reload, save loading, and snapshot
reconstruction may supply different source requests, but they should converge on
that one preparation and construction seam.

## 2. Why this is the next architectural keystone

Several desired architecture projects share this prerequisite:

```text
Normalize registration + content assembly
            │
            ├──► typed authoring schemas and diagnostics
            ├──► stable external SDK
            ├──► content fingerprints and epochs
            └──► construction registry
                         │
Explicit identity/provenance ─┤
                         │
                         ▼
              pure ConstructionPlan
                         │
             ┌───────────┼─────────────┐
             ▼           ▼             ▼
       entity recipes  hot reload   snapshot rebuild
                                       │
                                       ▼
                         transactional exact restore
                                       │
                                       ▼
                              rollback driver
```

Without this campaign:

- a recipe registry must invent another duplicate/conflict/finalization policy;
- hot reload can change definitions beneath an active session;
- snapshots may accept provider names while the underlying content differs;
- reconstruction continues inferring origin or family from `SimId` spelling or
  selected component rows;
- visual authoring and inspection have no pure construction representation to
  query;
- a public SDK would expose several incompatible extension lifecycles.

The asset dependency graph/cooker is less entangled and may proceed in parallel,
while sharing source-provenance, hashing, diagnostic, and atomic-publication
vocabulary where that is genuinely useful.

## 3. Relationship to existing canonical decisions

This campaign extends rather than forks the current architecture:

- **M24 remains binding:** activation, reset, transition, and restore use one
  App-installed placement-lowering authority. The construction planner becomes
  the pure/preflightable form of that authority; it does not introduce a second
  placement interpreter or let restore invent a parallel lowering path.
- **The world IR remains authoring-backend neutral:** LDtk is the active spatial
  backend, while the planned construction model consumes canonical authored
  records rather than LDtk node/object semantics.
- **Provider ownership remains explicit:** lower engine crates do not import named
  game content, and opaque plugin discovery is not introduced.
- **One body, one path still governs actors:** construction recipes choose and
  assemble canonical actor data; they do not create player/enemy/boss runtime
  variants.
- **Runtime still owns global schedule ordering:** construction planning does not
  become a new god-system or move domain leaf systems into `ambition_runtime`.
- **The snapshot contract remains same-build unless deliberately expanded:** a
  content fingerprint closes same-name/different-definition holes without
  silently promising cross-version compatibility.

If implementation pressure appears to require a parallel lowering, spawn,
reconstruction, or hot-reload authority, stop and revise the design instead of
adding a bridge. The purpose of this push is convergence.

## 4. Goals

### 4.1 Primary goals

1. Establish a consistent lifecycle for extensibility registries.
2. Assemble session-relevant definitions into immutable `PreparedContent`.
3. Compute a deterministic content fingerprint and assign a session content
   epoch.
4. Stop inferring entity construction semantics from `SimId` string patterns.
5. Introduce explicit provenance for authored, provider-staged, and
   runtime-dynamic entities.
6. Separate construction planning from ECS mutation.
7. Use the same planning and execution seam for normal spawning and
   reconstruction.
8. Migrate room activation, reset, transition, and hot reload toward
   transactional construction.
9. Bind snapshots and reconstruction to compatible content and snapshot-schema
   identities.
10. Produce the evidence needed to design later public entity-recipe and
    prefab-like APIs rather than guessing their final shape.

### 4.2 Secondary goals

- Improve diagnostics for provider conflicts and authored-content failures.
- Make construction behavior inspectable in tests and development tools.
- Reduce app-specific room orchestration without introducing size-driven crate
  decomposition.
- Establish deterministic ordering and ownership conventions across registries.
- Create data surfaces suitable for future inspectors, editor backends, content
  browsers, and agent tooling.
- Make exact world/session reproduction a first-class engine capability.

## 5. Non-goals

This campaign does not:

- implement a general public prefab API;
- implement prefab inheritance or arbitrary instance overrides;
- serialize arbitrary ECS component bags;
- create a universal scene graph;
- create a custom visual editor;
- add a scripting language;
- implement network transport;
- ship a production rollback driver;
- migrate every spawn family immediately;
- require every registry to use one generic Rust container;
- eliminate all direct `Commands` use throughout the codebase;
- guarantee cross-version compatibility for all snapshots;
- abstract away Bevy or make the engine backend-neutral.

Rollback, saves, editor integrations, and a public prefab API are downstream
customers of this work, not excuses to over-generalize its first implementation.

## 6. Binding principles

These outcomes are binding even if the implementation sketches later change.

### 6.1 Collect, validate, assemble, fingerprint, freeze

Provider installation must not mutate a live session's canonical content
piecemeal.

```text
provider fragments
    → local validation
    → cross-provider conflict validation
    → deterministic assembly
    → content and schema fingerprints
    → immutable prepared epoch
    → session activation
```

Once a session starts, its canonical content epoch is immutable. Hot reload
constructs a new candidate epoch rather than mutating the active one in place.

### 6.2 Plan before mutation

Construction has two explicit stages:

```text
prepared content + source request
    → pure validated ConstructionPlan
    → mutation executor
```

Planning must be possible without mutating the ECS world. It must expose enough
information to verify:

- the expected entity roster;
- stable identities and collisions;
- construction recipes;
- ownership and scope;
- relationships and unresolved references;
- dependencies;
- diagnostics;
- content epoch compatibility.

### 6.3 Provenance is data

`SimId` remains a stable simulation lookup identity, but its spelling is not the
authoritative description of how an entity was created or reconstructed.
Authored, provider-staged, and runtime-dynamic origins are represented explicitly.

### 6.4 One construction seam, multiple callers

Normal room loading, reset, transition, hot reload, save loading, and snapshot
reconstruction converge on one planning and execution model. They may apply
different replacement or persistence policies, but they do not maintain separate
constructors for the same entity family.

### 6.5 Transactional failure

Ordinary content and construction failures occur before destructive mutation.
When complete preflight is impossible, execution happens in a disposable world or
staging scope that can be discarded without corrupting the active session.

### 6.6 Internal proof before public commitment

Recipe, registry, provenance, and construction APIs remain internal or explicitly
experimental until normal activation, hot reload, and reconstruction have all
proven the same model. Public API design follows evidence from at least two real
consumers.

### 6.7 Inspectability is part of correctness

A professional engine must explain what it intends to do. Prepared content,
registrations, fingerprints, construction plans, and diagnostics need stable
debug representations that tests, command-line tools, future inspectors, and
agents can consume.

## 7. Current problems this campaign must replace

### 7.1 Construction is coordinated by convention

Room construction currently combines generic placement lowering with direct
family-specific loops, content-staged actor requests, resource resets,
relationship publication, and lifecycle events. These paths can be correct
individually while disagreeing about identity, validation, reconstruction, or
failure timing.

The placement-lowering registry is an execution registry, not yet a declarative
construction model: an interpreter receives mutable `Commands` and may make
construction decisions while mutating deferred ECS state. It cannot independently
report the complete entity roster or prove that execution will succeed.

`RoomContentStagingRegistry` is a better pure/preflightable precedent, but it is
actor-specific and does not describe a complete room transaction.

### 7.2 Reconstruction semantics leak through identity spelling

Existing restore work successfully proved projectile reconstruction, but parts of
reconstruction still recognize authored or dynamic identities through string
shape and nominated component rows. That is an implementation proof, not a sound
general public contract.

Before generalized recipes, the engine must represent explicitly:

- the source and owner of an entity;
- the recipe/family that creates it;
- the stable source instance or dynamic sequence that identifies it;
- whether the snapshot is blob-complete or construction data is required;
- which content epoch makes the recipe meaningful.

### 7.3 Registries disagree about extension semantics

Current registries differ in duplicate policy, override behavior, unknown-key
handling, deterministic ordering, validation timing, and whether failures panic or
return structured errors. Mature provider openness requires one lifecycle
protocol, even if domain-specific registries retain distinct types.

### 7.4 Session identity does not fully identify content

Provider names and room IDs do not distinguish two sessions whose room geometry,
character definitions, moves, recipes, or other behaviorally meaningful content
differ. Hot reload, saves, replay, and rollback need an exact prepared-content
identity, not merely a routing identity.

### 7.5 Supported restore is not yet a general rollback contract

> **Superseded framing (2026-07-19):** this subsection and §7.6/Phase 5 predate
> ADR 0027 — the custom snapshot substrate and its debt ledgers are DELETED;
> GGRS owns rollback. The surviving obligations moved to `tracks.md` #0
> (registration-coverage forcing function) and #5 (provenance/recipes). Read
> Phase 5 as historical; a rewrite is queued in Parallel maintenance.

The snapshot substrate is strong, but known component/resource debt still needs
classification, dynamic families vary in rebuildability, and some decode failures
can occur after mutation begins. This campaign prepares the construction and
content side; exact rollback promotion follows only after the supported state
envelope and transactionality are explicit.

### 7.6 Current evidence map

The first implementation pass should re-audit these sources rather than treating
this document's static review as proof against a later HEAD:

- `crates/ambition_world/src/placements.rs` — imperative placement lowering and
  its current registration/failure semantics;
- `crates/ambition_actors/src/features/ecs/spawn/mod.rs` — the assembled room
  construction path and family-specific loops;
- `crates/ambition_actors/src/features/ecs/spawn/content_staging.rs` — the pure
  actor-staging precedent;
- `crates/ambition_runtime/src/snapshot/{mod.rs,registry.rs,restore.rs}` — snapshot
  identity, registered state, dynamic anchors, preflight, and commit behavior;
- `crates/ambition_entity_catalog/src/lib.rs` and
  `crates/ambition_combat/src/moveset/prefabs.rs` — permissive schema/prefab
  registry precedents;
- `crates/ambition_characters/src/actor/character_catalog/registry.rs` and
  `crates/ambition_audio/src/catalog.rs` — stronger deterministic/transactional
  registry precedents;
- `game/ambition_app/src/app/dev_runtime.rs` — current hot-reload preparation and
  destructive commit orchestration;
- `game/ambition_app/tests/desync_canary.rs` and the known component/resource debt
  ledgers — current reconstruction evidence and unsupported-state pressure.

## 8. Provisional core concepts

The following names and shapes are design sketches. Their responsibilities matter
more than their exact Rust representation.

### 8.1 Provider-owned registration fragments

```rust
struct RegistrationFragment<T> {
    provider: ProviderId,
    source: SourceId,
    key: RegistrationKey,
    value: T,
}
```

Each registry domain defines:

- key namespace;
- provider/source ownership;
- duplicate and idempotency behavior;
- whether deliberate override is legal;
- deterministic assembly order;
- validation rules;
- fingerprint contribution;
- mutability/finalization rules.

One universal generic registry is not required. A common lifecycle and diagnostic
contract is.

### 8.2 Structured content diagnostics

```rust
struct ContentDiagnostic {
    severity: DiagnosticSeverity,
    code: DiagnosticCode,
    message: String,
    provider: Option<ProviderId>,
    source: Option<SourceId>,
    path: Option<ContentPath>,
    related: Vec<RelatedDiagnostic>,
}
```

Diagnostics remain structured until a log, CLI, inspector, or test formats them.
The model must support source-located messages such as:

```text
character_catalog.ron:142:17
error[AMB-CONTENT-0042]: conflicting recipe `enemy/slime`
registered by providers `ambition_base` and `demo_maryo`
```

### 8.3 Prepared content and content epoch

```rust
struct PreparedContent {
    epoch: ContentEpoch,
    fingerprint: ContentFingerprint,
    provider_set: ProviderSetIdentity,

    rooms: Arc<PreparedRoomCatalog>,
    characters: Arc<PreparedCharacterCatalog>,
    moves: Arc<PreparedMoveCatalog>,
    schemas: Arc<PreparedSchemaRegistry>,
    construction: Arc<PreparedConstructionRegistry>,

    snapshot_schema: SnapshotSchemaFingerprint,
}
```

Not every field must land in the first slice. The required property is that all
behaviorally relevant session content has one immutable identity.

The fingerprint must be:

- deterministic;
- insensitive to map insertion or provider discovery order;
- sensitive to behaviorally meaningful content;
- based on canonical representations or explicit hash contributions;
- independent of memory addresses;
- versioned so fingerprint semantics can evolve deliberately.

The first implementation may remain a same-build contract. It must still
distinguish sessions with the same provider names but different meaningful
content.

### 8.4 Explicit spawn provenance

```rust
enum SpawnOrigin {
    Authored {
        source: SourceId,
        instance: AuthoredInstanceId,
    },
    ProviderStaged {
        provider: ProviderId,
        recipe: RecipeId,
        instance: InstanceKey,
    },
    Dynamic {
        recipe: RecipeId,
        parent: Option<SimId>,
        sequence: DynamicSequence,
    },
}
```

The final model must answer:

- who requested the entity;
- which authored declaration or recipe produced it;
- which stable source instance it represents;
- which room/session scope owns it;
- how it can be reconstructed;
- which content epoch defines its recipe.

`SimId` may be generated from provenance. Restore logic does not recover
provenance by parsing the generated string.

### 8.5 Construction registry

A construction registry associates stable recipe identities with validation,
planning, and execution behavior. Conceptually:

```rust
trait ConstructionRecipe {
    fn validate(
        &self,
        request: &ConstructionRequest,
        content: &PreparedContent,
        diagnostics: &mut Vec<ContentDiagnostic>,
    );

    fn plan(
        &self,
        request: &ConstructionRequest,
        content: &PreparedContent,
    ) -> Result<Vec<PlannedEntity>, ConstructionError>;

    fn execute(
        &self,
        entity: &PlannedEntity,
        ctx: &mut ConstructionExecCtx,
    ) -> Result<(), ConstructionError>;
}
```

Typed function registration or another Bevy-native representation may be better
than trait objects. The binding distinction is that planning is pure and
execution consumes the plan instead of rediscovering authoritative decisions.

### 8.6 Construction plan

```rust
struct ConstructionPlan {
    content_epoch: ContentEpoch,
    scope: ConstructionScope,
    source: ConstructionSource,
    entities: Vec<PlannedEntity>,
    relations: Vec<PlannedRelation>,
    resource_ops: Vec<PlannedResourceOp>,
    diagnostics: Vec<ContentDiagnostic>,
}
```

```rust
struct PlannedEntity {
    sim_id: SimId,
    recipe: RecipeId,
    origin: SpawnOrigin,
    room: Option<RoomId>,
    parent: Option<SimId>,
    parameters: PlannedParameters,
}
```

The plan describes construction intent, not a second serialized ECS. Recipes
remain responsible for producing canonical ECS bundles/components. The first
slice models only enough relationships and resource operations to make its chosen
lifecycle transaction complete.

## 9. Work plan

### Phase 0 — architecture inventory and ADR

#### Objective

Make the binding decisions and record current mechanisms before adding APIs.

#### Tasks

1. Inventory registries involved in characters, moves, parameter schemas,
   placement lowering, content staging, audio, snapshots, and dynamic
   reconstruction.
2. Record for each registry:
   - key type and namespace;
   - provider/source ownership;
   - duplicate and override policy;
   - unknown-key policy;
   - deterministic ordering;
   - validation timing;
   - failure behavior;
   - mutation window;
   - fingerprint support.
3. Inventory room/world-construction paths:
   - normal activation;
   - reset;
   - transition;
   - hot reload;
   - snapshot cross-room restoration;
   - direct runtime spawning.
4. Write one ADR settling:
   - registration lifecycle;
   - immutable content epochs;
   - entity provenance;
   - construction planning;
   - transactional execution;
   - snapshot/content compatibility.
5. Evaluate relevant Bevy and ecosystem facilities before committing custom
   infrastructure. Record why adopted facilities are sufficient or why the
   engine needs narrower custom machinery.

#### Exit

- Registration, assembly, preparation, activation, planning, and execution have
  non-overlapping definitions.
- The ADR rejects string parsing as reconstruction authority.
- The ADR defines when content becomes immutable.
- The ADR defines what invalidates snapshots, replays, and rollback history.
- The ADR names the first three vertical-slice entity families.
- Mechanisms known to be provisional are marked as such.

### Phase 1 — normalize registration ownership and diagnostics — **COMPLETE 2026-07-18**

#### Objective

Create a consistent internal lifecycle before adding another major registry.

#### Tasks

1. Introduce shared provider/source ownership metadata.
2. Introduce structured `ContentDiagnostic` and content paths.
3. Define deterministic assembly and canonical hashing helpers.
4. Migrate a representative set:
   - one mature transactional registry such as audio;
   - one permissive registry such as parameter schemas;
   - one panic-based registry such as placement lowering.
5. Make duplicate behavior explicit:
   - identical registration may be idempotent where appropriate;
   - conflicting registration fails before canonical state mutates;
   - override is deliberately modeled, never accidental.
6. Remove last-registration-wins and ordinary-content panics from migrated
   registries unless the domain explicitly justifies those semantics.
7. Provide deterministic dumps of assembled registrations and ownership.

#### Exit

- Representative registries emit structured diagnostics.
- Conflict checks are transactional.
- Reordered provider input assembles identically.
- Registry fingerprints remain stable under equivalent insertion orders.
- No migrated registry uses panic for an expected authored-content error.

### Phase 2 — introduce `PreparedContent` — **COMPLETE 2026-07-18**

#### Objective

Pin one immutable, fingerprinted content definition to every active session.

#### Tasks

1. Add `ContentEpoch`, `ContentFingerprint`, and fingerprint-schema versioning.
2. Assemble selected registries into `PreparedContent`.
3. Associate the prepared object with provider/session activation.
4. Ensure active sessions cannot observe piecemeal canonical-registry mutation.
5. Make hot reload build a candidate prepared epoch off to the side.
6. Extend snapshot world identity with:
   - content fingerprint;
   - snapshot schema fingerprint.
7. Reject incompatible restore before world mutation.
8. Add developer output that identifies active content and explains
   incompatibilities where practical.

#### Exit

- Meaningfully different definitions produce different fingerprints even when
  provider names and room IDs are equal.
- Equivalent provider/fragment insertion orders produce the same fingerprint.
- The active session is pinned to an immutable prepared epoch.
- Incompatible snapshots are rejected before mutation.
- Hot-reload preparation leaves the active epoch untouched.

### Phase 3 — explicit provenance and construction-plan vertical slice — **LANDED 2026-07-22**

`ambition_platformer_primitives::construction` is the content-free planner:
`RecipeId`, `SpawnOrigin`, `ConstructionRequest`/`Plan`/`PlannedEntity`/
`PlannedRelation`, a registry on the Phase-1 lifecycle, and a canonical dump.
`ambition_actors::construction` is the domain that puts three real families
through it — an authored `GroundItemSpec`, a provider-staged `SpawnActorRequest`,
and a minion summoned by `Effect::Summon`.

**The load-bearing result is that provenance stopped being a spelling.**
`heal_projectile_owners` recovered a projectile's firer with
`id.as_str().rsplit_once('/')`; that was the only parse of a `SimId` in the
tree, and it is gone. `mint_spawned_sim_ids` now states the parent in
`SpawnOrigin::Dynamic` at the moment it already has it in hand, and the healer
reads it. Two consequences that were not obvious before doing it: the derived-
state justification registered for `ProjectileOwner` was factually wrong (it
named `ProjectileOwnerId`, which is EMPTY for every player projectile, so it
could not have carried the owner for the largest pool in the game), and
`SimId::as_str`'s "never parsed" doc comment was false while it was written.
Both are corrected.

**Three deviations from §8's sketch, each with a reason:**

1. **`SpawnOrigin` does not carry a `RecipeId`.** The sketch put one in two of
   the three variants, but the planned row already names the recipe. Storing it
   twice creates a state where they disagree and nothing says which wins.
2. **`ConstructionScope` carries no session scope.** Session ownership is a
   COMMIT-time fact — one prepared room plan is committed by whichever
   activation requested it — which is why `PlacementLoweringPlan` also takes its
   `SessionSpawnScope` at `lower_all` rather than at `plan_room`. It lives in the
   domain's `Services` instead. Putting it in the scope meant writing
   `UNSCOPED` into a field that was then ignored.
3. **`ContentEpoch` moved down to `ambition_engine_core`.** The plan states the
   generation it was prepared against, and construction planning sits far below
   `ambition_runtime`, which owns content identity. Allocation stayed put; only
   the stamp moved. Provider activation is the one site holding the exact
   prepared definition, so it is the one site that states a real epoch rather
   than defaulting.

**What the slice bought beyond the plumbing** — each family was losing something
real to the absence of a plan, and each is now a preflight failure:

- an authored ground item naming an unregistered held item used to `return`
  silently out of `spawn_ground_item`, producing no entity and no diagnostic;
- a staged duellist's `grudge_against` naming an actor outside its batch was
  dropped by `wire_staged_grudges`, leaving two fighters who ignored each other;
- a summoned minion carried a `FeatureId`, so `ensure_sim_id` filed it under the
  AUTHORED `placement:` namespace — the one namespace it categorically is not
  in — and two summons reusing an authored id collided outright. It now takes
  `SimId::spawned` under its summoner.

Provider-staged actors also stopped being deferred: they were written as
`SpawnActorRequest` MESSAGES and applied a system later, and are now plan rows
committed with the rest of the room.

**Not migrated — the exact remaining count, surveyed 2026-07-22.** Phase 4 is
**NOT started** beyond the `ContentBinding` type above. Nine authoritative
families and one parallel path remain outside the planner:

| # | family | site | state |
|---|---|---|---|
| 1 | authored placement → NPC | `spawn/mod.rs` `lower_all` | authoritative (`SimId` via `ensure_sim_id`) |
| 2 | enemy | `spawn/mod.rs` enemy loop | authoritative; **1 row → 3 entities** for `"giant"` class |
| 3 | boss | `spawn/mod.rs` boss loop | authoritative |
| 4 | hazard | placement lowering | has `FeatureId`, **no `SimId`** (no `BodyKinematics`) |
| 5 | pickup / chest / breakable / switch | placement lowering | same — identified but not in the sim roster |
| 6 | portal (`cfg(feature="portal")`) | placement lowering | no `FeatureId` at all |
| 7 | shrine | `spawn/mod.rs` | anonymous, not in `expected_authoritative_ids` |
| 8 | gravity zone | `spawn/mod.rs` | anonymous |
| 9 | portal gun pickup (`cfg`) | `spawn/mod.rs` | anonymous |
| — | `apply_spawn_actor_requests` | registered in `stage.rs` | **parallel unplanned path to `spawn_staged_actor`**, still carries the silent-drop `wire_staged_grudges` |

**Fourth review round (checkpoint 2).** Five substrate repairs, before any
Phase-4 family migration:

1. **The prepared plan was not actually frozen.** `PlannedEntity` stored only the
   `RecipeId`; commit called `dispatch` again. `dispatch` is expected to be pure
   but nothing makes it so, so a plan could validate/dump/fingerprint recipe A
   and execute constructor B. The resolved `ConstructFn` is now stored on the
   row and commit runs it. Proven by a toy domain whose `dispatch` flips on an
   atomic: it fails against the re-dispatch implementation.
2. **Summon reservation is now one authoritative boundary.** The counter check,
   the construction, and the advance happen inside a single exclusive-world
   command. The `max()` recovery path is deleted — there is no longer a window
   for the value to move. Proven with a real interleaving (auto sync points
   disabled so a direct-write system lands between queueing and applying); the
   old shape builds the minion, the new one refuses with nothing built.
3. **Relations carry canonical schema metadata** (kind, owner, source, schema
   id), in conflict validation, the dump, and the fingerprint. A relation whose
   WIRING changes while kind and owner stay put now requires a schema bump to be
   visible — stated in the ADR.
4. **`verify_committed_roster` exists**: counts identities rather than
   set-comparing them, checks root ownership and provenance, flags unplanned
   authoritative roots and dangling relations, and returns structured
   `RosterViolation`s. Six adversarial toy recipes prove each case, plus a
   positive test that presentation-only children are permitted.
5. Stale claims swept from code, ADR, and this document.

## Substrate checkpoint 3 — verification made real (2026-07-22)

Fourth external review round. The previous checkpoint built a verifier; this one
makes it something a transaction actually has to pass.

1. **`TransactionBaseline` preserves entity identity and multiplicity.** It was
   a `BTreeSet<SimId>`, which cannot distinguish an original from a replacement,
   a survivor from a look-alike, or a clean start from one that already held
   duplicates. It is now `BTreeMap<SimId, BaselineEntry>` — entity **and**
   provenance — captured by querying, with a duplicate identity refusing capture
   outright. Retirement and reconstruction are **declared** on the baseline, never
   inferred from the plan naming an identity: inferring would mean any plan
   mentioning an id thereby authorised destroying whatever held it.
2. **Authoritative scope is read from the world.** `verify_committed_roster` took
   a caller-supplied `&[(SimId, Entity)]`, which made it exactly as complete as
   the caller's memory. `AuthoritativeScope::gather` queries instead and
   classifies by component: this transaction's `TransactionId` stamp, another
   transaction's, an explicit `PresentationOnly` opt-out, or **no ownership at
   all**. The opt-out direction is deliberate — an identity-bearing entity is
   authoritative until something says otherwise, so forgetting to classify is a
   loud finding rather than a quiet exemption.
3. **Relations verify their postconditions.** A receipt records that a wiring
   function was CALLED; a no-op, a write to the wrong entity, a removal, and a
   later overwrite all produce an identical receipt. `RelationOps` now carries
   `wire` **and** `verify` together, frozen onto the planned row, and the
   verifier reads the committed components.
4. **Function-address equality is gone from registration semantics.**
   `std::ptr::fn_addr_eq` made a registry contract depend on codegen — the
   compiler may merge identical functions to one address and emit one function at
   several. Identity is metadata only; behaviour is governed by `schema_id`.
5. **A real production path invokes it.** `RoomFeatureConstructionPlan::spawn`
   queues a baseline capture BEFORE its construction and a verify-and-publish
   AFTER it. Command queues apply in insertion order, so the sequencing is the
   mechanism, not a scheduling hope — and the deferred path and the
   exclusive-world `apply_to_world` path share one publication route rather than
   growing a second architecture.

⚠ **`RoomLoaded` is no longer written by `spawn`.** It used to be that
function's last statement, which announced a room whose contents were still
sitting unapplied in the command queue. Verification writes it now, or nothing
does.

⚠ **Roster verification DETECTS; it does not PREVENT, and Bevy commands do not
roll back.** By the time the verifier can run, construction has applied. A
violation can stop a transaction being published as successful; it cannot undo
it. There is no staging world, so nothing here should be read as rollback
atomicity. The structural fix — every authoritative root an explicit plan row —
is Phase-4 work.

⚠ **`Severity::Unmigrated` is a deliberate, temporary hole.** Nine families
still build authoritative roots outside the planner, so an identity with no
ownership stamp is REPORTED rather than fatal — making it fatal today would
refuse rooms that work exactly as designed. The class empties as families
migrate; Phase 4's last step deletes the severity split.

⚠ **There is still NO enforced plan-to-world roster parity.** A recipe receives
raw `Commands` and the root `Entity`, so it can despawn the root, remove or
overwrite its `SimId`, mutate unrelated entities, or spawn additional entities
that acquire authoritative identities. `ConstructionRoot` stops a recipe
NOMINATING a pre-existing entity as a row's root — that and no more. What has
changed is that all of it is now *detected at a real boundary*; what has not is
that any of it is *prevented*.

### Phase-4 readiness: what the first relation migration will find

Surveyed before starting, so the migration uses the descriptor machinery above
rather than provoking another redesign.

**Both target pairs are bidirectional**, which means each needs a `verify` that
checks *both* sides. A forward-only check passes on a half-wired pair while one
side of the world lies — the exact class `RelationCheck::ReverseMismatch` was
added for.

| pair | forward | reverse | raw `Entity`? |
|---|---|---|---|
| limbs | `Limb { of }` on each limb | `LimbRig { limbs: Vec<Entity> }` on the host | both |
| mounts | `RidingOn { mount }` on the rider | `MountSlot { rider: Option<Entity> }` on the mount | both |

**The stable identities already exist for limbs.** `spawn_giant_hand_limbs`
mints `SimId::spawned(&giant_sim, ordinal)` per hand, with `ordinal` fixed by the
array literal (left = 0, right = 1), and `giant_hand_feature_id` is a pure
function of the authored giant id. Nothing needs inventing; the identities are
there and only the *relation* is unplanned. This is why limbs go first.

**Dependency closure.** A limb's identity is derived from its host's, so the two
are inseparable: any subset containing one must contain the other, which
`relation_closure` already produces once the relation is planned. Mounts are the
looser case — rider and mount are independently authored — so the closure there
is genuinely two roots joined only by the link.

**Two pre-existing defects the migration should absorb rather than preserve:**

- **`MountSlot` is never inserted at spawn.** `attach_mount_role` inserts
  `Mountable`/`CanPilot`/`Mass` and no slot; `resolve_pending_mount_links`
  inserts it only when a link resolves. The post-rollback path in
  `autonomous_reconcile` then does `world.get_mut::<MountSlot>(..)` — a mutation
  that silently does nothing when the component is absent — while inserting
  `RidingOn` unconditionally. A mount reconstructed without a surviving
  `MountSlot` therefore ends up **one-directionally linked**, and nothing today
  reports it. Planned-relation wiring writes both ends by construction.
- **Limbs have no `SimId`-keyed shadow.** Mounts have one —
  `TemporaryControl::Mounted { mount: SimId }`, with a real `SnapshotState`
  codec — so a restore can rebuild the mount link from stable ids. There is no
  `LimbRig` equivalent, no reconcile pass, and no id-keyed representation of the
  rig at all.

**The snapshot codecs preserve these handles in a way that only works inside a
GGRS rollback.** All four register `rollback_component_clone` +
`rollback_map_entities`, i.e. a byte clone of the raw `Entity` plus `LoadWorld`
remapping. That remap has an old→new table only because the same entities are
recreated within one `LoadWorld`. Outside that — a room rebuild, a partial
reconstruction, any path that mints genuinely new entities — a restored giant's
`LimbRig.limbs` and each hand's `Limb.of` are stale allocator slots. None of the
four contributes a checksum projection either, since they use plain
`rollback_component_clone` rather than the `_state` variant the registry
documents for `Entity`-carrying components.

**There are zero tests in `crates/ambition_runtime/src/rollback/` covering any of
the four.** Nothing currently pins that a reconstructed rig or mount link comes
back with correct handles, which is the gap the migration closes.

## Phase 4, step 1 — limbs and mounts are relation kinds (2026-07-22)

`ambition.limb` and `ambition.mount` are registered relation kinds with
bidirectional wiring and bidirectional postcondition checks.

**Relations gained a typed payload.** `ConstructionDomain::RelationPayload`, on
the request, frozen onto the planned row, passed to both `wire` and `verify`, and
rendered into the dump. The reason is specific rather than general: `Limb` holds
`slot` and `home_offset`, and **both are stated relative to the host** —
`HandLeft` is meaningless without saying left hand *of what*, and `home_offset`
is read as `host.pos + gravity_frame(offset)`. They are facts about the pairing.
Putting them in the limb's construction parameters would place host-relative data
on a body that does not learn its host until the relation is wired, which is the
same shape as the duplicated `parent` field this campaign already deleted. A
domain whose relations are pure adjacency uses `()`; the toy domain does.

The dump gained a payload column, so **`CONSTRUCTION_PLAN_SCHEMA_VERSION` is 3**.
It is content: two plans whose limbs fill different slots describe different
worlds, and a dump that rendered them identically would call them the same plan.

**One function writes both ends.** That is the whole point for these two pairs,
because the way they break is a half-write. The rig case accumulates — a host
with two hands is two relations, appending in the plan's canonical relation order
(which sorts by the limb's `SimId`, so the hands' `…/0` and `…/1` come out
left-then-right). `fan_out_limb_intents` reads the rig positionally, so that
order is content, not incident.

**Verification checks the reverse side, and that is not redundant:**

- A limb absent from its host's `LimbRig` is **inert** — `fan_out_limb_intents`
  iterates the rig — while `Limb.of` still names the right host. Every
  forward-only assertion passes.
- A mount whose `MountSlot` does not point back stops obeying, because
  `steer_mount_from_rider` queries `With<MountSlot>`, while every rider-side
  assertion passes. **This is a defect that exists in the tree today**:
  `attach_mount_role` never inserts `MountSlot`, and
  `reconcile_autonomous_actors` re-establishes the link with
  `world.get_mut::<MountSlot>(..)`, a mutation that silently does nothing when the
  component is absent, while inserting `RidingOn` unconditionally.

⚠ **No production caller declares either relation yet.** Limbs, riders, and
mounts are not plan rows — the giant's hands are minted inside
`spawn_giant_hand_limbs`, and mount links still resolve a frame later through
`PendingMountLinks` / `resolve_pending_mount_links`. The kinds are registered,
fingerprinted, and tested against real components, but the migration is not
complete until those endpoints become plan rows. That is the next commit, and
until it lands the old paths remain the ones that run.

⚠ **Two facts that make the roster-parity claim narrower than it reads.**

- `spawn_enemy_with_faction_into` spawns **two extra authoritative roots** (giant
  hand limbs) that mint their own `SimId::spawned`, and it is reachable from
  *inside* the already-planned staged-actor recipe. A room containing a
  `"giant"`-class archetype therefore has authoritative identities the plan does
  not name. The roster-parity tests use non-giant archetypes, so they are true
  but do not cover this. Making the hands plan rows is Phase 4 work.
- `Limb`/`LimbRig` and `RidingOn`/`MountSlot` are raw `Entity` relationships
  wired *inside* spawn helpers rather than declared as plan relations, so they
  are invisible to `relation_closure` and to the cut-detection above. They are
  the next relations to migrate, for exactly the stale-handle reason that made
  the incoming-relation rule wrong.

`apply_spawn_actor_requests` survives because programmatic scene setup (RL
episode reset, demo crony spawns) legitimately wants a message — but it is a
second live path to the same helper and should shrink to that use alone.

**Verification.** 20 domain tests (`ambition_actors::construction`), 25 planner
tests (`ambition_platformer_primitives::construction`), 6 provenance tests
(`ambition_runtime::rollback::provenance_tests`). The provenance file records
which of its tests actually DISCRIMINATE between the old and new mechanisms —
two do, four are behavioural regression protection that passes either way — and
that was established by running the file against the pre-change implementation
rather than asserted.

**Review round (same day): five transactional gaps, all closed.** An external
review of the landed slice found five places where the *claim* was stronger than
the *mechanism*. None were caught by the tests above, and the pattern is worth
carrying into Phase 4: every one was a boundary that had been described as
atomic without anything enforcing it.

1. **`apply_summon_effects` advanced `SimIdCounter` before `prepare` ran**, so a
   rejected batch permanently consumed dynamic identities no entity was built
   for — while its error branch said "Nothing has been mutated". Sequence
   numbers are now taken into a local map and written back only after commit.
   *"Preparation is pure" has to be true of the caller, not only of `prepare`.*
2. **Recipe and parameters were chosen independently**, so a valid public request
   could pair them wrongly and reach the recipe's `unreachable!` mid-commit.
   Every recipe now registers an `AcceptsFn`, checked during preparation.
3. **The executor trusted the `Entity` a recipe returned**, so plan-to-world
   parity was the executor's bookkeeping agreeing with itself. The identity
   stamp now goes through the world and panics if the entity already holds a
   `SimId`. The exit-criterion test was rewritten to query live identities.
4. **`parent` was stored twice** — on the request and in `SpawnOrigin::Dynamic` —
   validated on one and read on the other. The request field is deleted and
   `Dynamic::parent` is no longer optional. The dump lost its now-redundant
   parent column: **plan schema v2**.
5. **`construct_one` never wired relations**, so rebuilding a duellist alone
   silently dropped its grudge, and `respawn_authoritative_entity` swallowed the
   result with `.is_ok()`. There is now ONE executor, `commit_subset`, which
   refuses before mutating when a rebuilt row's relation leaves the subset.

Deviation 1 below generalises as a result: *no fact about a planned entity is
stored in two places*, whether that is the recipe or the parent.

**Third review round (2026-07-22, checkpoint 1).** Four narrow repairs, one of
them correcting a process failure of mine rather than a design flaw:

1. **Four relation tests were silently DELETED by my own previous commit** — an
   edit that replaced from a marker to end-of-file took the appended block with
   it, including the poison test that had been verified against `896bfb1`. The
   commit then reported "25 -> 23 (two deleted, three added)", arithmetic that
   does not work and that nobody re-derived. Restored and extended to six cases:
   source-only refusal, target-only refusal, closure in both directions, closure
   transitivity across `A -> B -> C`, and closure rebuild proving relations point
   at the NEW generations. The target-only test was **re-verified** against the
   asymmetric rule and fails there with `Grudge(1v0)` vs `Grudge(1v1)`.
2. **`recipe_of` and `construct` were two matches that could drift** — a variant
   could be labelled with one recipe's identity and built by another's code and
   still compile. Collapsed into one `ConstructionDomain::dispatch` returning a
   `RecipeDispatch { recipe, construct }`, so both are chosen in the same arm.
3. **The construction registry was documented as contributing to the
   prepared-content fingerprint and did not** — `prepare_platformer_content` did
   not take it at all. It now hashes the canonical dump as the
   `construction.recipes` section. Verified load-bearing: removing the section
   makes a recipe-schema change stop moving the fingerprint.
4. **Summon counter advancement was ordered but unguarded.** Reservations now
   carry the value planning read; a summoner whose counter is missing or has
   moved is refused BEFORE anything is built, and a violation discovered after
   construction is logged loudly and resolved by taking the furthest value
   rather than silently skipped. ⚠ Ordered commands are not rollback atomicity
   and the comments now say so.

**Second review round: four of the five repairs above were incomplete, and one
encoded a new wrong invariant.** Recorded because the pattern repeated — each
time, the *claim* outran the *mechanism*, and each time the tests could not see
it.

1. **The relation rule was wrong in the incoming direction.** The first repair
   refused a subset that cut a relation's SOURCE but explicitly permitted one
   that cut its TARGET, reasoning that the relation "belongs to" the untouched
   source. It does — but what the source holds is an `Entity` handle, so
   rebuilding the target alone left the source pointing at a corpse. Proven, not
   argued: committing `a --grudge--> b`, despawning `b`, rebuilding `b` alone
   left `a` on `Grudge(1v0)` while the new `b` was `1v1`. The rule is now
   symmetric — a relation must be wholly in or wholly out — and
   `ConstructionPlan::relation_closure` turns a seed set into one that cannot be
   cut, so the refusal is solvable rather than a dead end.
2. **The executor still did not own the root.** It ran the recipe and trusted
   the returned `Entity`, guarded only by a deferred check that the entity held
   no `SimId`. A pre-existing entity WITHOUT one was commandeered silently, and
   the guard was a panic at flush rather than a refusal. The executor now
   allocates the root with `spawn_empty` and hands the recipe a
   `ConstructionRoot` it cannot forge, so freshness is structural and the check
   is gone rather than strengthened.
3. **`AcceptsFn` stored the compatibility fact twice.** It was registered
   independently of the constructor, so the two could disagree and a wrongly
   permissive validator still reached the constructor's `unreachable!`
   mid-commit. Both are deleted: `ConstructionDomain::recipe_of` derives the
   recipe from the payload (so `ConstructionRequest` has no `recipe` field to
   mispair) and `ConstructionDomain::construct` is one exhaustive match (so a
   missing arm is a compile error). The registry keeps its ADR-0026 identity
   role and loses dispatch entirely.
4. **The counter advance was not part of the commit.** `plan.commit` only
   *queues* commands; the counters were written directly afterward, so they
   advanced ahead of the construction they paid for. They are now queued last,
   landing after every command the commit produced.
5. **Epoch zero meant three different things** — "a fixture stated nothing", "a
   reset states no new generation", and "a summon is not content at all" — so no
   commit boundary could distinguish a stale content-bound plan from a
   legitimately generation-free one. `ConstructionScope` now carries a
   `ContentBinding` that is either `Content(epoch)` or `RuntimeDynamic`.

⚠ **Scope note the review prompted, worth stating plainly:**
`respawn_authoritative_entity` — the single-entity reconstruction path — has **no
production callers today**. `RoomConstructionPlan::apply_to_world` rebuilds a
room by committing the whole plan, and the per-entity wrapper in `stage.rs` is
reached only from tests. So "ordinary construction and reconstruction share one
constructor" is proven by construction (there is literally one executor,
`commit_subset`) and by test, but it is not yet exercised by a shipping code
path. **Phase 4 is what makes it live.** Until then, treat the refusal semantics
above as a contract being established rather than one being relied on — and note
that a change here would ride a fully green suite, which is the same shape of
gap that let `heal_projectile_owners` sit untested.

#### Original card (retained for the record)

Milestone A landed through ADR 0026. `PreparedContent`, versioned BLAKE3
fingerprints, App-local epochs, canonical registry/schema dumps, exact snapshot
compatibility, and transactional LDtk content replacement are now runtime
authority. Do not reopen those as parallel abstractions; build provenance and
planning on them.

#### Objective

Prove the model on a narrow set that crosses all important origin categories.

#### Select one family of each kind

1. **Authored placement:** a simple environmental or interactive placement.
2. **Provider-staged actor:** an enemy or NPC emitted from room content staging.
3. **Runtime-dynamic family:** preferably a summoned actor/minion whose
   reconstruction needs authored recipe data; retain projectile reconstruction
   as a comparison case.

#### Tasks

1. Add explicit `SpawnOrigin` and an internal stable `RecipeId`.
2. Introduce `ConstructionRequest`, `ConstructionPlan`, and `PlannedEntity`.
3. Add a prepared construction registry following the Phase-1 lifecycle.
4. Convert the selected families to validate, plan, and execute.
5. Use the same recipe for normal spawning and reconstruction.
6. Remove `SimId` parsing from reconstruction for the selected dynamic family.
7. Detect identity collisions during planning.
8. Validate parent and relation references before execution.
9. Add a deterministic human/tool-readable plan dump.
10. Prove that execution creates exactly the authoritative roster and
    relationships declared by the plan.

#### Exit

- The slice has no separate normal-spawn and reconstruction constructor.
- Planned and committed `SimId` rosters match exactly.
- Reordered plan input does not change deterministic output.
- Duplicate identities and unresolved relations fail before mutation.
- The selected dynamic family does not infer family/provenance from `SimId`
  delimiters.
- A failed plan leaves the active world unchanged.

## Substrate checkpoint A — the relation and transaction substrate repaired (2026-07-23)

Fifth external review round. Nine correctness gaps between what the last
checkpoint *claimed* and what its mechanism *enforced*, all closed before any
further family migration. Grouped by what each one stops being possible.

**A relation kind can no longer disagree with what it carries.** The request
held a caller-supplied `RelationKind` beside a separate payload, and nothing
checked they matched: `kind: ambition.limb` next to a `Grudge` payload passed
preparation, passed the registry check, and reached `unreachable!` *inside the
wiring function during commit*, after the outgoing room was retired. The two are
now one value — `ConstructionDomain::Relation` — and the kind, the wiring, and
the verifier are all derived from it by one exhaustive `dispatch_relation` match.
The mismatch is unrepresentable, not checked. `RelationPayload` is gone.

**Relation wiring is engine-owned, so plugin insertion order cannot pick which
implementation runs.** The registry stored a `RelationOps` and decided
idempotence on metadata alone, so two registrations with identical
owner/source/schema and *different* wiring functions were "the same" and the
first won — under a dump and fingerprint that could not tell them apart. The
registry now holds no function pointers at all; ops come from `dispatch_relation`.
There is no table to race for. (An earlier attempt compared `fn_addr_eq`, which a
registry contract cannot rest on — the compiler may merge or split function
addresses.)

**Duplicate and contradictory relations are refused before any spawn.** A
duplicate `(from, kind, to)` would execute twice and receipt once — corruption
for an accumulating relation. The generic planner refuses it, which also makes
`(from, kind, to)` a total order so arrival order reaches neither the dump nor
the execution sequence. The actor domain adds preflight cardinality and
compatibility rules — one host per limb, one limb per slot, one rider per mount,
one rider per mount, no self-mount, family legality, and pilot/mount class
compatibility — each rejecting the room while it is still whole. The last one
matters most: `resolve_pending_mount_links` *drops* an incompatible link with no
diagnostic, so an authored typo produced a rider standing next to its mount and
no explanation.

**An unowned identity is fatal; a legacy family must name itself.** `Unowned`
used to be `Severity::Unmigrated` — reported, published anyway — which made a
recipe inventing an authoritative root indistinguishable from a known un-migrated
family, so the one failure the verifier exists for was the one it tolerated. A
genuine legacy family now carries `LegacyConstructionRoot { family }`, and only a
name in the enumerated `KNOWN_LEGACY_FAMILIES` list is warned-and-published;
anything else — an arbitrary unowned identity, or an unrecognised legacy claim —
is fatal. The list is the migration ledger as code: it only shrinks, and Phase
4's last step deletes it with `Severity::Unmigrated`. It currently holds exactly
one entry, `giant-hand-limb`, which Checkpoint B removes.

**Ownership is verified per planned root, and the ownership key distinguishes
live sessions.** The executor stamps identity, provenance, AND transaction
ownership in one insert; verification checked the first two and not the third —
the one that *drives* scope classification, so an unstamped planned root was
invisible to the next transaction's gathering forever. It is checked now. And the
token was `binding + room`, a construction-scope identity, not a transaction
identity: the shell host runs two sessions in one process, so two sessions
committing the same room at the same content epoch minted the *same* token and
each classified the other's roots as its own. The committing `SessionSpawnScope`
is now part of the key.

**Relation postconditions verify everything the relation installs, and a
relation the executor never wired is a failure.** The limb check now verifies
`Limb.of`, the slot on both sides, `home_offset`, rig membership, and no
duplicate membership; the mount check verifies `RidingOn`, `Mounted`, `MountSlot`
back-reference, and that both ends still carry the mount capabilities the
preflight approved. `RelationCheck` gained `PayloadMismatch`, `MissingCapability`,
and `DuplicateMembership`. And the verifier no longer `continue`s past a planned
relation absent from the receipt — which had made the postcondition pass vacuous
for exactly the relations that failed hardest. The owed set is derived from the
identities actually committed.

**`LimbRig` is keyed by slot.** It was a `Vec<Entity>` "in spawn order", but
nothing read it positionally — `fan_out_limb_intents` looked up each limb's own
`Limb::slot`, so the vector supplied membership and the limb supplied meaning,
two places holding one fact, with a vector able to hold one limb twice or two
limbs in one slot. A `BTreeMap<LimbSlot, Entity>` makes both unrepresentable and
makes "the host's rig composition" an exactly checkable value. This deletes the
"reads the rig positionally, so that order is content" claim from the step-1
section above — that claim was the misconception.

**The mount-reconciliation half-write is repaired.** `reconcile_autonomous_actors`
re-established the rider→mount link with `world.get_mut::<MountSlot>(mount)`,
which silently does nothing when the mount lacks the component — and it easily
does, because `MountSlot` is installed by the pair wiring, not the mount's
construction. `RidingOn` was inserted unconditionally, so the rider pointed at a
mount that did not point back and `steer_mount_from_rider` (which queries
`With<MountSlot>`) quietly stopped obeying. It now inserts the whole `MountSlot`
side.

**Publication moved to the outer room transaction boundary.** `RoomFeatureConstructionPlan::spawn`
bracketed its own work with capture and verify-and-publish, but its caller
`spawn_contents` queued the moving-platform bodies and the last-commit receipt
*after* it returned — and command queues apply in insertion order, so `RoomLoaded`
announced a room with no platforms and no receipt, verified before the room
stopped being built. The bracket now lives in `spawn_contents` (a new
`world/rooms/transaction.rs`): `open` first, every participant's work between,
`close` last. A feature helper no longer publishes. An integration test observes
the world the instant `RoomLoaded` is delivered and proves the platforms, the
commit receipt, and the authoritative occupant are already present.

⚠ **Still not rollback.** Withholding publication after mutation is not
atomicity, and Bevy commands do not roll back. Every repair above is detection or
structural-impossibility, not undo. A staging world would be needed for real
rollback, and there isn't one. This is stated wherever the boundary is.

⚠ **`CONSTRUCTION_PLAN_SCHEMA_VERSION` stays 3** — the dump shape did not change
(the relation payload column was already there). The actor domain's relation
*behaviour* changed, so its `SCHEMA` id moved to `actor-construction-v2`, which
is what carries the change into the prepared-content fingerprint.

**Verification.** 48 domain tests (`ambition_actors::construction`), 68 planner
tests (`ambition_platformer_primitives::construction`), 7 reconcile tests, 6
provenance tests, 15 provider tests. Discriminating vs regression-only is noted
per test; the mismatch-is-unrepresentable and registration-order tests are
compile-shape and end-to-end proofs respectively, not poison tests, and say so.

## Checkpoint B, step 1 — giant hands are explicit plan rows (2026-07-23)

**The last `KNOWN_LEGACY_FAMILIES` entry migrated.** A `"giant"`-class enemy's
two hand limbs were minted inside `spawn_enemy_with_faction_into`, each with its
own `SimId::spawned`, as authoritative roots no plan named — the one family that
carried an owned-but-unstamped identity the boundary verifier could actually see.
They are construction rows now: `authored_giant_requests(room, roster)` emits one
`ambition.giant-host` row plus two `ambition.giant-hand` rows per authored giant,
joined by two `ambition.limb` relations, and the enemy loop skips the hosts it
plans (`planned_giant_host_ids`) so nothing is built twice.

- The hand identities are **unchanged** — `SimId::spawned(giant, ordinal)`, the
  feature id a pure function of the giant's authored id — so a snapshot taken
  before the migration still restores.
- The geometry that lived inside `spawn_giant_hand_limbs` (hand size, home
  offsets) is now a pure `giant_hand_plans` computed at plan time.
- `spawn_giant_hand_limbs` is **deleted**. `spawn_enemy_with_faction_into` no
  longer spawns hands for anyone; the giant host is `populate_giant_host_into`
  (the ordinary enemy body plus the host-side `LimbIntents`/`LimbRouteState`),
  each hand `populate_giant_hand_into`.
- **Reconstruction is by relation closure.** `respawn_authoritative_entity` now
  rebuilds `relation_closure({id})` rather than the bare row, so asking for the
  host or either hand rebuilds all three with fresh generations and a rewired
  rig. This also changed the duellist case: rebuilding one now rebuilds its
  grudge partner too, which is the only correct way to bring a related row back.
- `KNOWN_LEGACY_FAMILIES` is now **empty**. The type and `Severity::Unmigrated`
  survive because the remaining families will need them when they migrate to
  eager construction stamping — today they receive their `SimId` from
  `ensure_sim_id` *after* the verification pass, so they carry no identity at
  scope-gather time and are not classified at all.

**Not done: authored mount links (Checkpoint B, step 2).** `PendingMountLinks` /
`resolve_pending_mount_links` still resolve rider↔mount a frame later. Making
them planned `ambition.mount` relations needs BOTH ends to be plan rows, and the
rider is normally the `gnu_ton_rider` **boss** (a `boss_spawn`) or a rider enemy
(the `enemy` family loop) — neither migrated. So step 2 is gated on migrating the
boss and rider-enemy families into the planner, which is a further step, not a
contained follow-on. The giant mount itself is now a plan row, but its rider is
not, so the link stays on the frame-later path until those families move.

### Remaining families after the giant migration (surveyed 2026-07-23)

The count did **not** drop by a whole family — the enemy loop still builds every
non-giant enemy — but the giant's "1 row → 3 entities" hole is closed and the
legacy-family list emptied. Still outside the planner:

| # | family | site | state |
|---|---|---|---|
| 1 | authored placement → NPC | `spawn/mod.rs` `lower_all` | authoritative (`SimId` via `ensure_sim_id`, post-verify) |
| 2 | enemy (non-giant) | `spawn/mod.rs` enemy loop | authoritative; **giants now planned** |
| 3 | boss | `spawn/mod.rs` boss loop | authoritative; blocks planned mount links |
| 4 | hazard | placement lowering | `FeatureId`, no `SimId` |
| 5 | pickup / chest / breakable / switch | placement lowering | identified, not in sim roster |
| 6 | portal (`cfg`) | placement lowering | no `FeatureId` |
| 7 | shrine | `spawn/mod.rs` | anonymous |
| 8 | gravity zone | `spawn/mod.rs` | anonymous |
| 9 | portal gun pickup (`cfg`) | `spawn/mod.rs` | anonymous |
| — | `apply_spawn_actor_requests` | `stage.rs` | parallel unplanned path to `spawn_staged_actor` |

**Verification.** 52 domain tests (`ambition_actors::construction`, +4 for the
giant migration), 908 `ambition_actors` lib tests. The giant tests are
discriminating: the row-count/relation-shape test and the committed-rig test both
fail against the pre-migration helper (which built the hands invisibly).

## Checkpoint C — giant correctness across origins, honest outer artifacts (2026-07-23)

The step-1 migration was correct for authored rooms and incomplete everywhere
else; this checkpoint closes the review findings on it.

- **Every construction origin either builds a whole giant or refuses one.** The
  two plan-based origins (authored `enemy_spawns`, provider-staged
  `SpawnActorRequest`) lower a `"giant"`-class spec through ONE shared helper
  (`giant_cluster_rows`): host row + two hand rows + two limb relations,
  whichever door it entered by. The origins that do not pass through the planner
  (summon effect, runtime minion, encounter wave) **refuse** a giant spec during
  preparation (`reject_runtime_giant`) instead of silently spawning a handless
  host. Whether those runtime origins should later lower through the planner and
  gain hands is deliberately left open; bosses cannot be giants on any path.
- **The giant host keeps the room's frozen kinematic paths.** Step 1 planned the
  host with `paths: Vec::new()`; the authored request builder now threads the
  same frozen path set the ordinary enemy loop passes, resolved at planning.
- **The outer roster is derived from the plan, not re-enumerated.** The
  predicted roster is `planned_ids()` (hands included) ∪ an explicit
  `non_plan_authoritative_ids()` for the families still outside the planner, all
  in the one `SimId` spelling; the commit receipt unions `committed_ids()` the
  same way, so the outer predicted-vs-committed assert is a real cross-check
  instead of a clone compared with itself. The explicit non-plan union is the
  Phase-4 migration surface and shrinks as families move.
- **`RoomConstructionPlanId` is the identity of the complete frozen plan.** It
  now hashes the spec JSON **plus `deterministic_dump()`** — recipe ids, derived
  rows, relation payloads, content epoch — so a roster change that reshapes the
  prepared world (a hand's home offset, giant-vs-ordinary expansion, an epoch
  bump) moves the id even when the authored spec bytes are identical. Session
  and transaction values stay excluded: they are commit-time, not frozen-plan.
- **Exact rig composition is verified at the boundary.**
  `verify_rig_composition` compares each planned row's committed `LimbRig`
  slot-for-slot against `planned_rig_for_host` (which finally has a caller):
  every planned slot holds exactly the planned limb's committed entity
  (generation included), no extra slot, no duplicated limb body, and each
  occupant's forward `Limb` agrees on host and slot. Faults surface as the new
  fatal `RosterViolation::RigComposition` through the same
  `LastConstructionVerification` gate.
- **Reconstruction can start from a stable identity.**
  `respawn_authoritative_sim_id` accepts any planned `SimId` — including a
  hand's `SimId::spawned`, which no authored-id spelling reaches — and commits
  the relation closure; the authored-id entry point is now a wrapper over it.

### What verification honestly covers (Checkpoint C statement)

Three different claims, deliberately kept separate:

1. **Publication ordering is complete.** `RoomLoaded` is written by the outer
   transaction close, after features, planned rows, relations, moving platforms,
   and the commit receipt. No lifecycle path publishes earlier.
2. **Verification is complete for migrated plan rows.** Baseline preservation,
   roster occupancy, ownership, provenance, relation postconditions, and rig
   composition are all checked for every plan row, and any fatal finding
   withholds publication.
3. **Visibility is incomplete for the enumerated legacy families.** The
   families in the table above receive their `SimId` from `ensure_sim_id`
   *after* the boundary verifier runs, so they are invisible to
   `AuthoritativeScope::gather` at verification time — the verifier can neither
   bless nor indict them. They are enumerated and finite
   (`non_plan_authoritative_ids` names the authoritative ones), an unknown
   unowned root remains **fatal**, and no check was weakened to accommodate
   them. Do not read a published room as "fully verified" until the table
   empties.

### Phase 4 — migrate room lifecycle operations

#### Objective

Make room replacement operations variations of one construction transaction.

#### Migration order

1. Normal room activation.
2. Room reset.
3. Room transition.
4. Hot reload.
5. Snapshot-driven cross-room reconstruction.

#### Tasks

1. Produce a complete room construction plan.
2. Separate resource-reset/reseed policy from entity construction while
   representing transactionally relevant operations explicitly.
3. Execute destructive replacement in a staging/disposable world or equivalent
   commit boundary.
4. Publish `RoomLoaded` only after successful commit.
5. Resolve mount links and relationships from planned stable identities.
6. Remove family-specific direct loops once their families are represented in
   the plan.
7. Keep unmigrated paths behind explicit, enumerated legacy adapters during the
   transition; delete each adapter when its family migrates.
8. Make hot reload construct both candidate `PreparedContent` and candidate room
   state before activating either.

#### Exit

- Activation, reset, and transition share one planner and executor.
- Failed preparation cannot partially despawn or replace the active room.
- `RoomLoaded` cannot fire for a partial room.
- Hot reload activates a fully prepared epoch/world atomically from the app's
  perspective.
- The expected room roster is inspectable before commit.
- Legacy construction adapters are explicitly enumerated and shrinking.

### Phase 5 — snapshot and restore hardening

#### Objective

Close construction and transactionality gaps before promoting rollback as an
engine feature.

#### Tasks

1. Classify known component/resource debt into:
   - authoritative mutable simulation;
   - structurally derived state;
   - immutable session content;
   - presentation-only state;
   - ephemeral queues/caches;
   - unsupported rollback state.
2. Add explicit restore policy metadata where it improves enforcement or
   inspection.
3. Bind restore to content and snapshot-schema fingerprints.
4. Replace dynamic-family detection by string/component-row heuristics wherever
   construction provenance is available.
5. Ensure supported codecs can be completely preflighted before commit.
6. Where full preflight is impossible, restore into a disposable world and swap
   only after successful completion.
7. Add exact restore and replay-suffix tests using the migrated vertical slice.
8. Make unsupported state refuse rollback explicitly rather than silently
   omitting authority.

#### Exit

- The supported rollback profile is explicit and inspectable.
- Unsupported state fails at preflight.
- Restore failure cannot partially mutate the active world.
- Snapshot compatibility includes prepared-content and schema identity.
- Authored, staged, and dynamic examples restore through the common
  construction seam.
- Remaining real-room non-losslessness is classified rather than hidden in an
  undifferentiated debt count.

### Phase 6 — external architecture proof

#### Objective

Use the architecture from outside the monorepo's internal assumptions without
prematurely freezing a public prefab contract.

#### Tasks

1. Create an out-of-workspace consumer fixture.
2. Author one room, one character, one enemy, one construction recipe, and one
   transition through the umbrella engine surface where possible.
3. Record every internal API the fixture must import.
4. Expose only narrow facade additions justified by the fixture.
5. Keep construction APIs experimental until at least two real consumers prove
   them.
6. Ship developer-readable dumps for prepared content, ownership, construction
   plans, fingerprints, and validation failures.
7. Measure the workflow against the product objective:
   - engine-core edits required;
   - undocumented imports required;
   - time and commands to first playable room;
   - quality of deliberate-error diagnostics;
   - ability to run visibly and headlessly from the same content.

#### Exit

- The fixture runs without editing reusable engine crates.
- It does not reconstruct entities through a separate path.
- Internal API leaks are documented as evidence for SDK design.
- No final public prefab API has been guessed prematurely.
- The architecture is inspectable through useful headless tooling.

## 10. Testing strategy

### 10.1 Determinism

Equivalent inputs must yield identical:

- assembled registry order;
- content and schema fingerprints;
- construction plans;
- stable identities;
- relationship ordering;
- snapshot hashes within the supported same-build contract.

Tests should deliberately randomize insertion order.

### 10.2 Transactionality

Inject failures at:

- duplicate registration;
- invalid parameter hydration;
- missing recipe;
- identity collision;
- unresolved parent/relation;
- execution failure;
- snapshot codec failure.

The active prepared epoch and active world must remain unchanged.

### 10.3 Plan-to-world parity

After executing a plan, compare the committed roster against the plan's identity,
recipe, provenance, scope, and relationship declarations. Unexpected
authoritative entities are failures unless the plan explicitly declares a child
production rule.

### 10.4 Content compatibility

Verify that:

- identical provider IDs with changed room/content definitions are incompatible;
- changed recipe definitions are incompatible;
- insertion-order changes alone remain compatible;
- presentation-only changes may remain compatible only when deliberately
  excluded by policy;
- snapshot-schema changes are detected independently from content changes.

### 10.5 Lifecycle coverage

Exercise:

- initial activation;
- reset;
- transition;
- hot-reload success and failure;
- cross-room restore;
- dynamic reconstruction;
- session teardown and restart.

### 10.6 Headless-first acceptance

All architecture behavior must be verifiable without rendering. Visual tools may
later present the same prepared data and plans; they are not the source of truth.

## 11. Risks and countermeasures

### Risk: accidental universal abstraction

A plan can become an opaque “everything operation” containing arbitrary closures
and uninspectable mutation.

**Countermeasure:** keep the first plan narrow; require stable identity,
provenance, deterministic data, and plan-to-world parity. Add capabilities only
when a demonstrated family needs them.

### Risk: duplicating ECS state in plan structures

A construction plan can become a second serialized ECS.

**Countermeasure:** plans describe construction intent, identity, provenance,
relationships, and recipe parameters. Recipes produce runtime components.

### Risk: brittle or expensive fingerprints

Hashing runtime structures can depend on insertion order or implementation detail.

**Countermeasure:** fingerprint canonical prepared representations, use ordered
collections, version hash semantics, and test reordered inputs.

### Risk: hot-reload scope explosion

Content epochs may create pressure to migrate arbitrary live state between
revisions.

**Countermeasure:** the first valid policy may reconstruct the affected session and
invalidate incompatible snapshots/replays. Live migration is explicit future work.

### Risk: migrating every spawn path at once

Many specialized families make a big-bang conversion dangerous and hard to
review.

**Countermeasure:** prove one three-origin vertical slice, use explicit temporary
adapters, and migrate lifecycle paths incrementally while deleting each old path.

### Risk: freezing public APIs too early

Recipe/provenance types will evolve as activation, hot reload, and restore converge.

**Countermeasure:** keep APIs internal or experimental until multiple real
consumers use the same path.

### Risk: recreating editor-engine object graphs

Competitive comparison may tempt the project to copy Unity/Godot/Unreal concepts
rather than solve their user problems cleanly.

**Countermeasure:** compare workflow outcomes, not class hierarchies. Preserve
Bevy-native ECS ownership and the provider/content architecture.

### Risk: infrastructure without user-visible engine leverage

The campaign could produce technically elegant registries without improving the
experience of building a game.

**Countermeasure:** every milestone includes a lifecycle or external-authoring
customer, diagnostic output, and measurable competitive outcome.

## 12. Milestones

### Milestone A — deterministic content assembly — **COMPLETE 2026-07-18**

- The foundation ADR is accepted.
- Representative registries use provider/source ownership and structured
  diagnostics.
- `PreparedContent` exists.
- content fingerprints are deterministic.
- sessions pin one immutable epoch.

### Milestone B — planned construction vertical slice — **COMPLETE 2026-07-22**

- ✅ explicit provenance exists — `SpawnOrigin` is a snapshot-registered
  component, not a fact recovered from an id's spelling;
- ✅ authored, staged, and dynamic families produce plans;
- ✅ planned and committed rosters match;
- ✅ normal spawning and reconstruction use the same recipes;
- ✅ the selected dynamic family no longer depends on `SimId` parsing — the one
  parse site in the tree is deleted.

See Phase 3 below for the account, including the three deviations from this
document's sketch and what each one bought.

### Milestone C — transactional room lifecycle

- activation, reset, and transition consume construction plans;
- failed preparation leaves the active room intact;
- hot reload prepares a new content epoch and room off to the side;
- successful replacement is atomic from the application perspective.

### Milestone D — restore-ready construction substrate

- snapshot compatibility checks content and schema fingerprints;
- restore uses explicit provenance;
- the supported rollback state envelope is classified;
- supported restore is transactional;
- cross-room restore consumes the common construction seam.

### Milestone E — external engine-workflow proof

- an external fixture authors and runs content without engine-core edits;
- it uses prepared content and construction APIs;
- deliberate authoring failures produce actionable diagnostics;
- the same content runs visibly and headlessly;
- evidence exists to design a public recipe/prefab API.

## 13. Recommended execution sequence

```text
1. ~~Architecture inventory and foundation ADR~~ — ADR 0026 landed
2. ~~Shared source/ownership diagnostics~~ — landed for prepared content
3. ~~Registration ownership and deterministic assembly~~ — representative proof landed
4. ~~PreparedContent, ContentEpoch, and fingerprints~~ — landed
5. ~~Early snapshot content/schema compatibility checks~~ — landed
6. **SpawnOrigin and internal RecipeId**
7. **Pure ConstructionPlan**
8. **Three-origin vertical slice**
9. Normal room activation
10. Room reset and transition
11. Transactional hot reload
12. Cross-room snapshot reconstruction
13. Rollback-state classification and restore hardening
14. External consumer fixture
15. Public recipe/prefab design from evidence
```

The content/schema compatibility check should land soon after prepared epochs,
even before construction migration is complete, because it closes an existing
correctness hole independently.

## 14. Parallel and downstream work

### May proceed in parallel

- asset dependency graph/cooker work, sharing source provenance, diagnostics,
  hashes, and atomic publication where useful;
- an external SDK consumer **probe** that records internal leaks without
  promising stability;
- source-located schema vocabulary once registration ownership is established.

### Depends on this campaign

- generalized public entity recipes or prefab-like authoring;
- transactional content hot reload as a supported engine feature;
- save/load reconstruction across content families;
- full rollback driver promotion;
- stable public construction APIs;
- visual inspectors and editor integrations that need authoritative construction
  data.

### Depends primarily on action/control P3 instead

- local-N control and observer architecture;
- semantic animation/action presentation;
- remappable control authoring and replay input.

## 15. Final decision and review rule

Approve the next architecture push under the working title:

> **Immutable Content Assembly and Transactional Construction**

The push is successful not when a prefab type exists, but when:

- content is assembled and frozen deterministically;
- every active session has a meaningful exact content identity;
- entity origins and reconstruction requirements are explicit;
- construction can be inspected and validated before mutation;
- normal loading and reconstruction share construction semantics;
- destructive world replacement is transactional;
- another game can begin consuming the architecture without engine-core edits;
- the later public authoring API can be designed from evidence.

At every phase review, ask two questions:

1. **Does this make Ambition more capable and usable as an alternative to Unity,
   Unreal, and Godot for its target games?**
2. **Has the implementation revealed a cleaner path to that goal than this plan
   anticipated?**

A “no” to the first question means the campaign is drifting into infrastructure
for its own sake. A “yes” to the second means the plan should be revised rather
than defended.
