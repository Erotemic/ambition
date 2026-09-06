# World IR, level authoring, and backend adapters

**Checked 2026-08-07.** Ambition already has a backend-neutral authored world IR
plus an LDtk adapter and a historical/auxiliary RON-room serialization of that
IR. LDtk remains the active world/level authoring backend. That is enough to
compare its level-authoring boundary to editor-native tilemaps and external map
formats now, rather than treating "support another level editor" as future
plumbing.

## The Ambition capability that already exists

[`ambition_platformer2d_world`](../../crates/ambition_platformer2d_world/src/lib.rs)
describes itself as **backend-agnostic authored world IR**. It owns room graphs,
placements, room metadata, moving-platform definitions and the composited
collision world used by movement queries. Backend adapters convert authored
formats into those types; simulation consumes the IR through explicit lowering
seams.

[`ambition_platformer2d_ldtk`](../../crates/ambition_platformer2d_ldtk/src/lib.rs)
is therefore an adapter, not the engine's world model. Its validation checks
project/level structure and Ambition-specific authoring conventions before
conversion. `WorldManifest` declares which worlds exist and the entry room, so
the reusable engine ships no hard-coded project/start room.

The same world crate also has the historical/auxiliary
[`ron_room`](../../crates/ambition_platformer2d_world/src/ron_room.rs): a
`RoomSpec` plus graph links serialized as RON. It is **not** the current
authoring direction; its value here is evidence that the IR can be serialized
without carrying LDtk's object model into simulation.

The architecture is already:

```text
LDtk JSON ─┐
           ├─> validated adapter ─> RoomSpec / placements / room graph ─> lowering
RON IR (historical/auxiliary) ──┘
future X ──┘
```

That boundary deserves explicit competitive pressure.

---

## LDtk — typed external level editor with JSON entities/fields

LDtk stores projects/levels as JSON, including levels, layer instances, entity
instances and custom fields. Entities are generic placeable records with
constraints and author-defined fields.

Sources:

- [JSON documentation](https://ldtk.io/json/)
  (LDtk, official).
- [Entities](https://ldtk.io/docs/general/editor-components/entities/)
  (LDtk, official).
- [Entity fields](https://ldtk.io/docs/game-dev/json-overview/entity-fields/)
  (LDtk, official).

### Comparison

LDtk is not merely a dependency to document; it defines one side of Ambition's
public author workflow. Ambition adds a **semantic compiler/adaptation layer**
that LDtk itself cannot know: which entity identifiers are valid for a provider,
how surfaces become collision/contact/respawn semantics, how rooms link, how
placements map to stable simulation identity, and which authored values are
legal for the deterministic engine.

Keep those semantics out of arbitrary JSON-field probing in simulation code.
The LDtk adapter should remain the one place where editor vocabulary becomes
canonical world records.

A practical bar follows: every Ambition-specific validation error should name
the LDtk level/entity/field precisely enough that an author can fix the source
without stepping through conversion code.

---

## Tiled — portable map format plus custom properties

Tiled's TMX and JSON formats describe tile maps with layers, tilesets, objects and
custom properties, and the editor exports those formats for many runtimes.

Sources:

- [TMX Map Format](https://doc.mapeditor.org/en/stable/reference/tmx-map-format/)
  (Tiled, official).
- [JSON Map Format](https://doc.mapeditor.org/en/stable/reference/json-map-format/)
  (Tiled, official).

### Comparison

Tiled is the best obvious **second external-backend forcing case** even if Ambition
never ships first-class Tiled support. Its vocabulary differs from LDtk enough
to test whether the world IR is genuinely authoring-neutral: object layers,
custom properties, templates and tileset collision do not line up one-for-one
with LDtk entities/IntGrid.

A spike importer should therefore be judged by how much engine code it *doesn't*
change. If adding Tiled requires new branches in movement, construction or room
transition code, the IR boundary is not yet as neutral as intended.

---

## Unity Tilemaps — editor-native world representation feeds runtime components

Unity's Tilemap workflow builds 2D worlds by painting tiles into a Scene. The
Tilemap system passes tile information to runtime components such as the Tilemap
Renderer and Tilemap Collider 2D.

Sources:

- [Tilemaps](https://docs.unity3d.com/6000.0/Documentation/Manual/tilemaps/tilemaps-landing.html)
  (Unity, official).
- [Tilemap Collider 2D](https://docs.unity3d.com/6000.2/Documentation/Manual/tilemaps/work-with-tilemaps/tilemap-collider-2d.html)
  (Unity, official).

### Comparison

Unity makes the editor/runtime continuity extremely convenient: the authored
Scene/Tilemap is directly part of the engine's object model. Ambition chooses a
more explicit seam: external author data is validated and lowered into a
backend-neutral runtime contract.

That costs adapter code but buys several properties Ambition actually uses:
headless conversion tests, deterministic canonicalization, stable provenance,
multiple possible authoring backends and simulation crates that never need to
know an LDtk/Unity-style editor node exists.

The competitive requirement is therefore **not** to imitate Unity's object
model. It is to make Ambition's extra compilation step feel equally immediate to
a level author through diagnostics, hot reload and source-to-runtime inspection.

---

## Godot `TileMapLayer` — tilemap is a first-class scene node

Godot's `TileMapLayer` is a `Node2D` that uses a `TileSet` to create a grid-based
map; multiple layers are represented by multiple nodes.

Sources:

- [`TileMapLayer`](https://docs.godotengine.org/en/stable/classes/class_tilemaplayer.html)
  (Godot, official).
- [Using TileMaps](https://docs.godotengine.org/en/stable/tutorials/2d/using_tilemaps.html)
  (Godot, official).

### Comparison

Like Unity, Godot couples authoring convenience closely to the runtime scene
model. That is a strong UX benchmark and a useful contrast to Ambition's IR
boundary.

Ambition should especially copy the **inspectability** of editor-native maps:
authors can see layers, collision and instantiated scene tiles directly. The IR
pipeline should offer equivalent debug views — authored record -> canonical
placement/surface -> construction row -> live `SimId` — without making the
editor representation authoritative.

---

## What Ambition already distinguishes

| Concern | Editor-native / file-format precedent | Ambition today |
|---|---|---|
| authoring tool | Unity/Godot built in; LDtk/Tiled external | external backend adapters; LDtk active |
| runtime world representation | often editor scene/tilemap objects | backend-neutral `RoomSpec`, placements, graph, geometry |
| validation | editor constraints + importer errors | explicit Ambition semantic validation before simulation |
| second representation | engine-specific import formats | historical RON-room serialization proves the IR is not identical to LDtk's object model |
| entry/world selection | scene/project settings | game-owned `WorldManifest` |
| collision semantics | tile collider/editor metadata | authored surfaces lower to engine collision/contact semantics |
| stable provenance | object/file IDs vary | adapter data participates in stable construction provenance |
| headless tooling | possible but editor-centric in many systems | converter/IR can be tested without rendering/editor shell |

## Design work the comparison exposes now

### 1. Make source provenance survive every lowering stage

An inspector/error should be able to walk from a live collision/contact/entity
back to the canonical world record and then to the LDtk level/entity/field (or
other backend equivalent) that authored it. `SpawnOrigin` provides part of this;
world geometry/surface provenance needs the same quality.

### 2. Prove authoring neutrality with one deliberately different backend

A Tiled spike or another real customer format should import a representative
room with solids, one-way surfaces, moving geometry, transitions, placements and
metadata. The acceptance criterion is minimal/no change below the adapter/world
IR boundary.

### 3. Define a versioned world-IR compatibility surface

If future tools, caches or generated room artifacts emit a serialized `RoomSpec`
form, the IR becomes a format boundary. Decide what is stable, how versions migrate, and
which structures remain internal Rust-only implementation detail.

### 4. Add round-trip/source-to-runtime inspection

For every backend, tools should display:

```text
authored source record
  -> validated canonical record
  -> lowered collision/construction/room-link facts
  -> live stable simulation identities
```

This is how Ambition can recover much of editor-native engines' immediacy while
keeping deterministic preparation.

### 5. Separate visual tiles from gameplay geometry explicitly

Unity/Godot tilemaps make it easy for one tile asset to carry rendering and
collision. Ambition should keep testing that visual authoring can change without
implicitly changing deterministic collision semantics unless the authored
surface definition says so.

### 6. Treat room graph/transition validation as compiler work

Loading-zone targets, landing pads, entry room and graph consistency should be
validated before an outgoing room retires. The adapter/content pipeline should
make "bad destination" an authoring diagnostic, not a mid-transition runtime
surprise.

## What this comparison changed

World authoring is already an engine architecture, not just an LDtk integration.
The meaningful comparison is:

- **match Unity/Godot on author feedback and iteration speed;**
- **use LDtk/Tiled-style external formats as interchangeable source languages;**
- **differentiate with one validated, backend-neutral world IR feeding the same
  deterministic construction/movement/room contracts.**

A second substantially different importer is the cleanest test of whether that
claim is real.
