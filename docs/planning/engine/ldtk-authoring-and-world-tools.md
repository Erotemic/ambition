# LDtk authoring and world tools

**State:** OPEN authoring program. Ambition is the primary customer.

> **Guard pointer, added 0ac499bb1 (2026-09-02).**
> `scripts/check_authored_levels_survive.py` baselines each world file's set of
> level identifiers: adding levels is fine, REMOVING a recorded one is an error
> unless the baseline is explicitly `--bless`ed. Green at `0ac499bb1`: **77 authored
> levels across 6 worlds, none lost.** ⛔ Deliberately narrow — it guards roster
> SURVIVAL only; other validators own level contents and entity correctness. It
> is the thing standing between an editor session and a silently deleted level.

## Goal

Make LDtk a first-class **agent-operable** spatial authoring backend rather than
knowledge of JSON field conventions plus converter internals.

LDtk remains one spatial backend over the backend-neutral world model. The goal
is not to couple engine semantics permanently to LDtk, nor to require an LLM to
operate the graphical editor. The adapter/tooling should let an agent inspect a
room semantically, infer spatial intent, author through supported operations,
validate the result, and produce a concise review render without reading Rust or
hand-editing LDtk JSON. Native LDtk field types/editor affordances are still
valuable because they make the authored data clearer and preserve a good manual
editing path when a human wants one.

## Current strengths

The repository already has substantial authoring infrastructure:

- `ambition_platformer2d_ldtk` lowers editor data into world IR;
- `ambition_ldtk_tools` can validate, repair, roundtrip, create areas, edit
  entities, resolve `EntityRef`s, render rooms and produce semantic diffs;
- provider-owned `LdtkVocabulary` extension exists;
- hot reload validates/prepares candidate worlds before commit;
- entity-layer rules and editor icons/visual manifests keep authored data legible for both agents and optional human editing;
- room tooling can describe/render moving platforms and other spatial features.

The next phase should consolidate these into a coherent authoring product.

## Current weaknesses

1. Some authored relationships remain strings even though the toolchain already
   supports native LDtk `EntityRef` values.
2. `KinematicPath` points are currently parsed from an opaque string such as
   `"10,20; 30,40"` rather than an editor-native point/path representation.
3. Runtime converters contain field defaults and precedence rules that are hard
   to discover from the LDtk editor alone.
4. Engine/provider vocabulary, editor entity definitions, validation and docs can
   drift because they are not generated/checked from one declarative schema.
5. Some useful errors arrive only at runtime conversion rather than as immediate
   authoring diagnostics.
6. The tools know many intent-level operations, but capability-specific recipes
   are still scattered.
7. ⛔⛔ **`tools/ambition_ldtk_tools/specs/*.ron` LAG THE `.ldtk`, AND THE `.ldtk`
   IS THE TRUTH.** A spec is the input that AUTHORED a level; later
   `entity set-field` edits (the `specs/*.yaml` form) change the world without
   changing it. Measured 2026-09-02: `intro_wake_room_area.ron` still shows an
   `NpcSpawn` with `name: "Creator"` and no `character_id`, while `intro.ldtk`
   has `character_id: "npc_creator"` and `name: None` — the opposite shape.
   Anyone scoping work from the specs gets a world that no longer exists; read
   the `.ldtk` (or `static_world_text!`'s embedded copy) when the question is
   "what does the game load".

## First vertical slice: moving platforms

Moving platforms are **already controllable from LDtk**. Current conversion
supports:

- entity position and bounds -> platform start/size;
- stable `id` (falling back to iid);
- `speed`;
- `sweep_dx` for the simple horizontal ping-pong form;
- `path_id` referencing a `KinematicPath`. ⛔ **the legacy `patrol_path_id`
  spelling is DELETED (2026-08-14)** — it was declared by no entity definition and
  authored on no instance. ⚠⚠ **and `path_id` itself is authored on ZERO instances
  across all six worlds**, so this bullet describes a supported capability with no
  content: the corpus holds two `KinematicPath` entities and both are reached
  through `EnemySpawn.brain = "Patrol:<id>"`, a relationship hidden inside an
  unrelated string field;
- `loop_dy` and `loop_min_y` for a wrapping vertical elevator/conveyor shaft.

`KinematicPath` currently authors a string `points`, `speed`, `mode`, and
`start_offset_seconds`.

This is enough to ship content, but it is not the final authoring experience.
The first phase should make the authored relationship semantically explicit to
both tools and the optional editor, and make invalid combinations impossible or
loudly invalid.

### Desired platform authoring

- a platform entity clearly exposes one motion mode;
- a path-following platform uses an LDtk `EntityRef` to a `KinematicPath`, not a
  free-form string lookup;
- path points use native LDtk point/array authoring if the editor schema supports
  it cleanly; otherwise the tools provide an intent-level path editor/visualizer
  rather than asking authors to type coordinate strings;
- inactive fields for another motion mode are hidden or diagnosed;
- speed/range/loop validation appears before runtime launch;
- room render/describe shows the path, direction, speed, wrap/ping-pong mode and
  referenced platform identity;
- game-owned vocabulary can expose the same authoring quality without modifying
  reusable engine code.

## Program phases

### L1 — declarative authoring schema

Define one source describing an engine/provider-authored entity's fields,
editor types, defaults, documentation, validation and lowering identity where
practical. Generate or reconcile LDtk entity definitions from that schema rather
than duplicating field knowledge across project JSON, converter comments and
recipes.

Do not make the common world IR depend on LDtk-specific field metadata.

### L2 — typed references and native editor value types

Migrate string relationships to `EntityRef`/typed resolved IDs where the
relationship is spatial and editor-local. Improve point/path fields to use native
LDtk constructs where practical.

MovingPlatform -> KinematicPath is the first proof.

### L3 — preparation-time cross-reference validation

A world candidate should report all unresolved/ambiguous references together:
paths, mounts, portals, loading targets, content IDs, capability-owned facets,
and game-owned vocabulary.

Diagnostics should name the LDtk level/entity/field and the expected target.

⛔ **and one thing the tools must NOT do: classify motion themselves.**
`AuthoredPlatformMotion::classify` is now the single place that turns authored
fields into a motion and refuses ambiguous combinations, naming the LDtk level in
the message. A Python re-implementation inside `room describe`/`render` — so the
inspector could print "downward wrapping loop, 300px shaft" — would be a second
authority for exactly the precedence rule that was just deleted, and it would
drift. The inspector should keep showing the authored FIELDS; the diagnosis
belongs to preparation, where a wrong combination already fails the load with
provenance. If a semantic view is wanted in the tools, it should ask the engine
rather than re-derive.

### L4 — intent-level tooling

For common operations, tools should express the author's intent:

- add/link a moving platform and path;
- add reciprocal/one-way loading-zone routes;
- place encounters/characters with stable IDs;
- move capability-specific entities to appropriate layers;
- inspect and render semantic world state.

The generated `.ldtk` remains ordinary editor-compatible data.

### L5 — hot reload and transaction quality

Hot reload should use the same compiler/preparation diagnostics as cold load,
commit only valid candidates, and preserve enough provenance to explain why a
candidate was rejected.

### L6 — provider extensibility

A provider should be able to add authored spatial nouns/facets with editor
schema, validation, lowering and diagnostics without editing a closed engine
switch in several places.

## Acceptance

Given a natural-language spatial task and the repository, an agent should be able
to:

1. inspect the room/area, collision, entities, gates and relevant references;
2. infer the intended traversal/world relationship instead of asking for raw
   coordinates when the map makes the intent clear;
3. discover the supported moving-platform/path vocabulary;
4. plan/apply the change through semantic tooling or stable authored fields;
5. validate all affected references before runtime commit;
6. produce a semantic summary/render showing the result;
7. explain what changed and why without hand-editing LDtk JSON or reading the
   converter implementation.

Opening LDtk manually should remain a good optional review/edit path, not a
prerequisite for supported authoring. That quality bar should become the standard
for every spatial capability we consider supported.
