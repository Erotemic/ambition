# Stable identity, provenance, and reconstruction

**Checked 2026-08-07.** Ambition already has several deliberately different
identity systems and construction provenance. This should be compared to asset
GUIDs, primary asset IDs, resource UIDs and ECS entity handles because stable
identity is doing real work in rollback, save/load, diagnostics and content
compilation today.

## The Ambition capability that already exists

The source has at least three identity layers that should stay distinct:

1. **authored/content identity** —
   [`ContentId`](../../crates/ambition_content_pack/src/identity.rs) is
   `(namespace, schema, name)` and is canonically ordered; `PackId`, schema IDs,
   versions and capability IDs describe the prepared content universe;
2. **simulation identity** —
   [`SimId`](../../crates/ambition_platformer2d_shared_tangle/src/sim_id.rs) is a
   stable, deterministic, human-readable identity derived from game facts rather
   than Bevy allocator order;
3. **runtime handle identity** — Bevy `Entity` remains the transient ECS handle
   used to access the current world.

Construction adds a fourth related concept that is not “another ID”:
[`SpawnOrigin`](../../crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs)
records **how an entity came to exist and what facts can reconstruct it**:
authored, provider-staged or dynamically spawned from a stable parent plus
sequence.

The implementation already uses these distinctions to canonicalize snapshots,
repair/rebuild references after rollback, report causal facts in meaningful
terms, reconstruct planned entities, and prevent authored content identity from
being confused with a machine-specific path.

---

## Bevy `Entity` — deliberately only an identity inside one `World`

Bevy's `Entity` is a generational identifier for an entity in a `World`. It is
not a project-level content identity or a portable semantic identifier; despawn
and later allocation change what handles are valid.

Source: [`Entity`](https://docs.rs/bevy/latest/bevy/ecs/entity/struct.Entity.html)
(Bevy API docs).

### Comparison

This is the substrate fact that justifies `SimId`. Ambition's source states the
problem precisely: allocator index/archetype visitation is not game state, so a
desync hash or snapshot should not become sensitive to it.

The public engine should teach this distinction explicitly. External consumers
must never infer that because a `bevy::Entity` appears in a low-level callback it
is safe to serialize, compare across sessions or expose as content identity.

---

## Unity GUIDs and `GlobalObjectId` — persistent editor/project object identity

Unity's Asset Database maps asset paths to GUIDs. Its `GlobalObjectId` is a
persistent unique identifier for a Unity object and includes the asset GUID plus
object-local information; moving an object to another scene can change the
GlobalObjectId because the scene's asset GUID participates in it.

Sources:

- [`AssetDatabase.TryGetGUIDAndLocalFileIdentifier`](https://docs.unity3d.com/current/Documentation/ScriptReference/AssetDatabase.TryGetGUIDAndLocalFileIdentifier.html)
  (Unity, official).
- [`GlobalObjectId`](https://docs.unity3d.com/2023.2/Documentation/ScriptReference/GlobalObjectId.html)
  (Unity, official).

### Comparison

Unity demonstrates the value of a stable identity layer above runtime object
references. It also demonstrates why the **scope of stability must be named**:
an identity can be stable within a project while still changing when authored
ownership/location changes.

Ambition should likewise state scopes rather than merely say “stable”:

- `ContentId`: stable within a provider/module namespace and schema contract;
- `SimId`: stable for the logical simulated entity under its construction facts;
- `Entity`: stable only as a live handle in the current world;
- save/replay references: stable only if their content/snapshot schema and
  migration policy remain compatible.

---

## Unreal `FPrimaryAssetId` — semantic asset identity independent of loading handle

Unreal's Asset Manager gives Primary Assets a `FPrimaryAssetId`; those IDs can
be used to find and explicitly load primary assets, while secondary assets are
managed through references/bundles.

Sources:

- [`FPrimaryAssetId`](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/CoreUObject/FPrimaryAssetId)
  (Epic, official).
- [Asset Management](https://dev.epicgames.com/documentation/en-us/unreal-engine/asset-management-in-unreal-engine)
  (Epic, official).

### Comparison

This is close prior art for Ambition's `ContentId` / pack identity layer: a
semantic identifier is different from the loaded runtime object. Ambition goes
further by making schema identity/version and capability ownership part of the
compiler boundary and by fingerprinting canonical prepared content for session
compatibility.

The lesson to borrow is discoverability: Unreal's primary asset model is a
named product concept. Ambition's content/sim/runtime identity taxonomy should
be equally visible in SDK docs and tools rather than being something users infer
from source types.

---

## Godot `ResourceUID` — path-independent resource identity

Godot's `ResourceUID` tracks registered resource UIDs and converts between UID
and path/string forms.

Source: [`ResourceUID`](https://docs.godotengine.org/en/latest/classes/class_resourceuid.html)
(Godot, official).

### Comparison

Again the important precedent is that paths and identities are different facts.
Ambition's `PreparedContentPack::canonical_bytes()` already encodes exactly this
principle for assets: the authored logical path participates in identity while
the machine-specific resolved asset root is provenance/diagnostics and is
excluded from the content fingerprint.

---

## Provenance is the part mature asset IDs do not cover

A stable ID answers **which thing?** `SpawnOrigin` answers **why does this thing
exist, and what would make it again?**

That distinction is particularly important for dynamic simulation entities. The
current `Dynamic { parent: SimId, sequence }` form prevents the engine from
creating an “unowned dynamic thing” that cannot be traced to a stable spawner.
Construction tests already verify that authored/provider/dynamic provenance
survives commit and reconstruction.

This is close in spirit to data provenance/event-sourcing ideas, but Ambition's
scope is deliberately narrower: it does not store every event needed to replay
the universe. It stores the minimal stable origin facts required to explain and
reconstruct world entities under the engine's construction recipes.

## What Ambition already distinguishes

| Layer | Example | Persist/serialize? | Primary use |
|---|---|---:|---|
| content identity | `ContentId`, pack/schema/capability IDs | yes, subject to migration/version contract | authored definitions, diagnostics, prepared content |
| simulation identity | `SimId` | yes | snapshots, rollback, save references, causal facts, reconstruction |
| construction provenance | `SpawnOrigin` | yes when reconstruction needs it | explain/rebuild origin and parent lineage |
| ECS handle | Bevy `Entity` | no as durable identity | fast access inside current world |
| content fingerprint | `ContentFingerprint` | yes | exact prepared-content compatibility/cache/session identity |

This table should become SDK doctrine. It prevents a large class of accidental
persistence and rollback bugs.

## Design work the comparison exposes now

### 1. Publish identity scopes and lifetime guarantees

For every public ID type, document:

- who mints it;
- its uniqueness domain;
- whether it survives reload/rollback/session restart/save migration;
- whether it is human-authored, generated or derived;
- whether it may be displayed/serialized as a compatibility contract.

### 2. Centralize migration/alias policy for renamed content

Unity/Godot asset IDs preserve references through many path changes. Ambition's
semantic content IDs are intentionally meaningful strings, which makes a rename
meaningful too. The content compiler needs an explicit alias/migration layer so
renaming `namespace:schema/name` does not force every save/replay/content
reference to invent its own migration table.

### 3. Make lineage queryable

`SpawnOrigin::Dynamic` already stores stable parent + sequence. Expose an
inspector query that walks parent lineage and joins it to causal facts and
construction-plan rows. “Where did this projectile/minion come from?” should be
one supported question.

### 4. Define identity across reconstruction epochs

If a room is torn down and reconstructed from the same plan, should the same
logical entity receive the same `SimId`? The current construction model strongly
suggests yes for authored/provider-staged rows, but the public rule should state
how session generations, round resets and dynamic spawn sequences affect
identity.

### 5. Put content and simulation identity into every external observation format

RL observations, trace artifacts, remote inspectors and save diagnostics should
quote stable IDs rather than reintroducing local array indices or entity handles.
That is how the source-level identity discipline becomes an engine property.

## What this comparison changed

Stable identity is not infrastructure trivia in Ambition; it is a cross-cutting
engine feature already supporting deterministic snapshots, content compilation,
construction and diagnostics. Mature engines strongly validate the separation of
asset/project identity from runtime handles.

Ambition's additional opportunity is to make **identity + reconstruction
provenance** one coherent public contract, so the engine can answer both “which
thing is this?” and “what facts made it exist?” across rollback, save/load and
debugging.
