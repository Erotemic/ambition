# Authoring, world composition, and deterministic preparation

**Checked 2026-08-07.** This page compares authored content as a runtime contract:
what gets instantiated directly, what is prepared first, how reuse works, and
where hot reload/validation belong.

## The Ambition question

Ambition's current construction architecture is intentionally not "build a prefab
system". The architecture in
[`immutable-content-and-transactional-construction.md`](../planning/engine/immutable-content-and-transactional-construction.md)
is:

```text
provider fragments + authored sources
    -> validation / conflict detection
    -> deterministic PreparedContent
    -> content fingerprint + immutable epoch
    -> pure ConstructionPlan
    -> transactional execution or discard
    -> active world
```

The canonical world IR remains authoring-backend neutral; LDtk is the active
spatial backend, not the runtime object model. Hot reload prepares a candidate
epoch off to the side before commit.

The source shows that this is no longer only a target architecture.
[`ambition_content_pack::compile`](../../crates/ambition_content_pack/src/lib.rs)
is the single compiler front door used by tests/CLI/runtime preparation. It
resolves schema handlers, validates capabilities and facets, aggregates
multi-source schema fragments, resolves content references/assets, detects
conflicts, canonicalizes ordering, and emits a
[`PreparedContentPack`](../../crates/ambition_content_pack/src/prepared.rs).
That prepared pack contains canonical content, resolved references, prepared
asset provenance, diagnostics, **already-lowered runtime artifacts**, and a
stable content fingerprint. Runtime consumers therefore do not need to reparse
the authored bytes with subtly different validation rules.

The fingerprint is intentionally computed from canonical semantic content, not
raw source bytes or machine-specific asset roots. Schema handlers are required
to define the canonical form of the values they lower, and multi-source runtime
schemas must explicitly define how their fragments aggregate rather than letting
the compiler guess union/override behavior. That is already compiler design, not
a loose collection of loader callbacks.

Construction is similarly implemented.
[`ambition_platformer2d_shared_tangle::construction`](../../crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs)
plans stable `SimId` roots, stamps `SpawnOrigin`, resolves recipe/relation
dispatch before commit, and tests reconstruction/retirement invariants. The
remaining work is deciding how much editor-like convenience can be added
**without letting authoring object graphs become the simulation authority**.

---

## The compiler boundary is itself related work

Unity/Godot/Unreal authoring systems are often discussed in terms of the object
or scene representation authors manipulate. Ambition now has a second axis that
deserves explicit comparison: **source language -> semantic compiler -> frozen
runtime artifact**.

The closest mature analogies are import/cook/build pipelines rather than prefab
instancing alone. The important property to retain is that authored data is
validated once, lowered once, and then consumed through the prepared artifact.
A runtime path that reparses the same source independently would recreate the
validator/runtime disagreement this compiler exists to eliminate.

This also changes the hot-reload problem. A file watcher is only the trigger;
the semantic operation is compile a candidate epoch, prove it, then authorize a
transactional construction/activation commit. The new
[loading-coordination comparison](loading-coordination-activation-barriers-and-supersession.md)
covers the activation half of that pipeline, while the
[world-IR comparison](world-ir-level-authoring-and-backend-adapters.md) covers
the authoring-backend half.

---

## Unity Prefabs + Addressables — excellent reusable authored objects and asset location

Unity Prefabs store a configured `GameObject` hierarchy with components and
property values as a reusable asset. Changes to a Prefab asset can propagate to
instances, and Prefabs may be nested.

Source: [Prefabs](https://docs.unity3d.com/2022.3/Documentation/Manual/Prefabs.html)
(Unity, official). A Prefab is a "reusable Asset" representing configured object
hierarchy.

Unity Addressables adds a separate addressing/loading layer: an asset can be
requested by address, and the system resolves its local/remote location and
dependencies asynchronously.

Source: [Addressable Asset System](https://docs.unity3d.com/6000.0/Documentation/Manual/com.unity.addressables.html)
(Unity, official).

### Lesson for Ambition

Unity is the usability bar for reusable authored composition and indirect asset
identity. But copying Prefab semantics directly would invert Ambition's intended
boundary: a configured runtime object hierarchy would become the reusable source
of truth.

Ambition's equivalent authoring convenience should lower into canonical prepared
records and construction plans. An author can have "prefab-like" reuse without
the runtime engine promising that the editor hierarchy itself is authoritative.

---

## Godot PackedScene — scene serialization and instancing are the core reuse primitive

Godot's `PackedScene` is an abstraction of a serialized scene. It can save a node
and owned descendants, then instantiate the saved scene into a running tree.

Source: [PackedScene](https://docs.godotengine.org/en/stable/classes/class_packedscene.html)
(Godot, official). The documentation calls it an "abstraction of a serialized
scene".

### Lesson for Ambition

Godot demonstrates the elegance of one compositional primitive serving both
scene authoring and reusable objects. Ambition should seek similar author
simplicity, but its deterministic/rollback goals make a direct serialized
runtime-tree contract unattractive. Preparation is the opportunity to accept
multiple friendly source formats while converging them before simulation.

---

## Unreal World Partition / Data Layers / Level Instances — sophisticated editor-world composition

Unreal Data Layers organize Actors in the editor and at runtime and can be
loaded/unloaded for world composition and gameplay. Level Instances package
selected Actors into reusable level content; changes propagate to copies.

Sources:

- [World Partition - Data Layers](https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition---data-layers-in-unreal-engine)
  (Epic, official).
- [Level Instancing](https://dev.epicgames.com/documentation/unreal-engine/level-instancing-in-unreal-engine?lang=en-US)
  (Epic, official).

Unreal is therefore a strong bar for collaborative/editor-scale world assembly,
streaming and reusable spatial groups. It is solving a much broader authoring
problem than Ambition currently needs.

### Lesson for Ambition

Do not compete by rebuilding World Partition. Do copy the separation between
**authoring organization** and **runtime state**, and the expectation that large
content structures can be validated, referenced and reused without hand-written
spawn loops.

If Ambition later needs streaming regions/layers, they should lower to explicit
prepared construction/lifecycle facts rather than bypass the canonical planner.

---

## Bevy Scenes and hot reload — useful substrate close to Ambition's implementation level

Bevy's Scene system can create, save, load and instance ECS worlds. Bevy also
supports hot reloading of scenes and assets.

Source: [Bevy](https://bevy.org/) (Bevy, official). The site describes scene
"Instancing" and automatic scene-file hot reload.

### Lesson for Ambition

Bevy scenes are a capability to compose with, not a reason to expose arbitrary
ECS snapshots as Ambition's public authoring contract. Ambition needs stronger
semantic validation, provider ownership, content identity and deterministic
construction than raw scene instancing provides by itself.

---

## Competitive design frontier

| Problem | Strong prior art | Ambition bar |
|---|---|---|
| Reusable authored groups | Unity Prefabs, Godot PackedScene, Unreal Level Instances | backend-friendly reusable definitions that lower into canonical prepared records |
| Asset addressing/dependencies | Unity Addressables | exact content identity/readiness/failure integrated with provider composition |
| World organization | Unreal Data Layers | only add layer/streaming semantics when games need them; lower through construction/lifecycle |
| Hot reload | Bevy scenes/assets, editor engines | prepare candidate content off-world; validate/fingerprint; atomic commit or discard |
| Runtime instantiation | all three editor engines | pure `ConstructionPlan` before ECS mutation |
| Deterministic compatibility | not the authoring center of editor-first systems | `PreparedContentIdentity` binds session/replay/rollback to exact prepared definitions |
| Multiple authoring backends | import pipelines in mature engines | authoring-neutral world IR; add backends through acceptance tests, not core branches |

## What still needs to be designed

### 1. The author-facing reusable-definition surface

The construction plan is an internal semantic success, but external authors need
a concise way to declare reusable entity/world patterns. The design should be
measured against three source shapes before becoming public:

- LDtk-authored spatial records;
- provider-defined character/object recipes;
- one non-LDtk external backend when a real customer requires it.

Only then is there enough evidence to name a stable prefab-like API. The public
surface should describe *what to construct*, not expose planner internals.

### 2. Hot-reload transaction semantics visible to authors

"Prepare off to the side" needs a complete product contract:

- what changes create a new content epoch;
- what validation errors prevent commit;
- how the active session reports that it remains on the old epoch;
- which changes require restart/reconstruction rather than in-place commit;
- how rollback/replay sessions are invalidated or restarted;
- how diagnostics point to the authored source/provenance that failed.

This is more deterministic than the common "file changed, mutate live objects"
mental model and could be a genuine differentiator if error reporting is good.

### 3. Schema evolution and authored-content migration

As provider/world schemas mature, old content needs explicit version/migration
rules. This is separate from save-game compatibility. Preparation is the natural
place to reject, migrate or normalize older authored forms before they enter the
immutable epoch.

### 4. Dependency/readiness graph tied to preparation

Addressables shows the usability value of resolving dependencies behind an
address. Ambition needs an equivalent engine-facing contract for exact content
identity, readiness and failure, but should connect it to `PreparedContent` so a
session cannot become active with a half-resolved dependency graph.

### 5. Inspection without an editor-first runtime

The construction plan should be printable/queryable before commit:

- which entities will exist;
- which provider/recipe/source record owns each row;
- which stable identities will be allocated;
- which dependencies are required;
- which rule rejected a candidate.

That gives an editor, CLI, test harness and CI the same introspection surface.
The standalone editor remains deferred until a real workflow justifies it.

## What this changed

This comparison argues **against** treating a Unity/Godot-style prefab or scene
object graph as the competitive goal. Those systems are strong authoring UX; the
architectural opportunity is to deliver comparable reuse and iteration while
preserving a deterministic preparation boundary.

A useful Ambition product sentence is:

> Author in whatever supported source fits the workflow; Ambition validates and
> freezes it into the same inspectable construction contract before simulation.

If that sentence becomes true for LDtk, provider-authored recipes, hot reload and
one additional backend, Ambition is distinguished rather than merely missing an
editor.
