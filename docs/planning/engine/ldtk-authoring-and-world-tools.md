# LDtk authoring and world tools

**State:** OPEN authoring program. Ambition is the primary customer.

`scripts/checks/check_authored_levels_survive.py` baselines each world file's level
identifiers. Adding levels is fine. Removing a recorded one is an error unless
the baseline is explicitly `--bless`ed. It guards roster survival only; other
validators own level contents and entity correctness.

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
- room tooling can describe/render moving platforms and other spatial features;
- level fields carry room behaviour in data: `mode`, `next_room`,
  `entry_cutscene` and `while_live` (an authored command line run while the room
  is live) lower to `RoomMetadata` (`crates/ambition_platformer2d_ldtk/src/project.rs`).

The next phase consolidates these into a coherent authoring product.

## Current weaknesses

1. One authored relationship is still a string: `MovingPlatform.path_id`. No
   shipped platform authors it. `EnemySpawn.path_ref` is a native `EntityRef`.
   See K5 in [`kinematic-world-objects.md`](kinematic-world-objects.md): author a
   path-following platform and migrate the field in the same change.
2. `KinematicPath` points are parsed from an opaque string such as
   `"10,20; 30,40"`, not an editor-native point/path value. Do this first: it has
   authored customers, and `SurfaceChain` authors its polyline the same way.
3. Runtime converters contain field defaults and precedence rules that are hard
   to discover from the LDtk editor alone.
4. Engine/provider vocabulary, editor entity definitions, validation and docs
   drift. `ldtk_entity_contract.json` is pinned against the Rust converters in
   both directions by `contract::prover`. `entity_contract_issues` checks
   authored placements against it. `contract_authorability_issues`
   (`validate_rules/entity_contract.py`, wired into `validate_issues`) warns per
   field when a project's `defs.entities` lacks a field the contract supports.
   These are warnings, not errors: a project that authors no vertical loops is
   not broken by lacking `loop_dy`. Missing entity definitions are warned once,
   by `validate.py`'s `missing_known_defs`.
   Count distinct projects by real path: the demo worlds under
   `game/ambition_demo_{mary_o,sanic}/assets/worlds/` are symlinks into
   `game/ambition_map_assets/`.
5. Some useful errors arrive only at runtime conversion rather than as immediate
   authoring diagnostics.
6. The tools know many intent-level operations, but capability-specific recipes
   are still scattered.
7. `tools/ambition_ldtk_tools/specs/*.ron` lag the `.ldtk`. The `.ldtk` is the
   truth. A spec is the input that authored a level; later `entity set-field`
   edits change the world without changing the spec. Read the `.ldtk` (or
   the embedded copy `build.rs` makes of each world `data/worlds.ron` lists) to
   know what the game loads.

## Tentative curve and local-gravity authoring

[Curved surfaces and local gravity](curved-surfaces-and-local-gravity.md)
is a decision aid, not a selected extension to this program. Existing
`SurfaceRamp`, `SurfaceLoop`, `AttachedLoop::route` and terrain profile
tools already prepare curved source into native geometry.

If CV2 or a planetary scene is selected, extend this semantic authoring
road with one source recipe, a declared solid side and typed references.
Prepare native bounds, adjacency, source parameters and mechanical error
limits. Objects placed on the boundary must use the prepared gameplay
answer, as the Sanic painted-height authoring already does. Render quality
and camera zoom must not resample collision geometry.

The author should be able to draw a hill, place an object on it, or name a
surface that attracts within a region. Inspection must show the prepared
boundary, field eligibility and ambiguous branches. A curve is not itself
a movement law; gravity attraction is not attachment. Keep those choices
explicit and validate them before publication. Do not expose segment count
as the ordinary control for movement smoothness or add a new runtime curve
interpreter/editor before the customer selects it.

## First vertical slice: moving platforms

Moving platforms are **already controllable from LDtk**. Current conversion
supports:

- entity position and bounds -> platform start/size;
- stable `id` (falling back to iid);
- `speed`;
- `sweep_dx` for the simple horizontal ping-pong form;
- `path_id` referencing a `KinematicPath` (supported, authored on no shipped
  platform);
- `loop_dy` and `loop_min_y` for a wrapping vertical elevator/conveyor shaft.

`KinematicPath` currently authors a string `points`, `speed`, `mode`, and
`start_offset_seconds`.

**Static review, 2026-11-22 (`00bf92c22142`):** a clear editor field is not
proof of a working native contract. The offset is stored but unused, the
platform and hazard evaluators duplicate the same nonterminating
coincident-point cycle, and authored actor `path_ref` does not currently
drive steering. [PATH1/PATH2](kinematic-world-objects.md#path1--prepare-one-spatial-path-law)
owns the bounded correction. Shipped inline hazard paths and the two actor
routes are its customers; no new platform placement is required to start it.

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

Existing hazard/actor paths are the first native point-value customers.
Keep a moving platform's string-to-`EntityRef` relation customer-gated as
K5 states above. A new editor representation does not replace PATH1's
preparation or PATH2's route consumer.

### L3 — preparation-time cross-reference validation

A world candidate should report all unresolved/ambiguous references together:
paths, mounts, portals, loading targets, content IDs, capability-owned facets,
and game-owned vocabulary.

Diagnostics should name the LDtk level/entity/field and the expected target.

For paths, use the shared native preparation from
[PATH1](kinematic-world-objects.md#path1--prepare-one-spatial-path-law) for
standalone paths and lifted inline hazard paths. Retain source waypoint
provenance after zero-length traversal legs are removed. A zero-total-length
moving path is a preparation error, not a loop that waits forever in play.
Inspection must distinguish initial phase, continuous closing travel,
discontinuous platform wrapping and a route that asks an actor to move.
PATH2's route target is not a prescribed physical trajectory. A successful
reference lookup or attached component is not acceptance for that authoring
surface. Do not duplicate traversal or phase law in Python tooling.

The tools must not classify motion themselves.
`AuthoredPlatformMotion::classify` (`ambition_platformer2d_world::platforms`) is the single place that turns authored
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

**Follow-up for district generators.** The Pirate Port generator applies
all ten rooms. To regenerate only the waterfront or the docks, the author
must call the room builder and repeat the ladder paint step. Consider a
room selector that runs the build and the selected room's later steps
together. Use the semantic diff to confirm the scope of the result.
The Frontier refinery spec puts its ladders in `intgrid.climbable`. Use
this field for the Pirate Port ladders when the generator can supply all
ladder positions in the room spec.
The Records & Processing room had a stale generator record after an edit
that put the same enemies in the map and the generator. A room preview
could show the generated changes before the author updates the record.
The Pirate Port generator's `build` function accepts `dangerous` and
`extra` through keyword arguments. It does not pass them to the room
builder. Check the intended values for the Reef and Lighthouse Point when
the district generators use the same room interface.

### L4.1 — prepare the complete edit before writing source

**State:** PLANNED after review of `3ed50e318392`, 2026-11-21. This is the
first concrete map customer for
[A3's staged edit contract](authoring-and-tools.md#a3---intent-level-mutation-and-reviewable-plans).
Start with entity move and level clone; do not rewrite all map tools.

**Evidence:**
[`LdtkTransaction.write_if_changed`](../../../tools/ambition_ldtk_tools/ambition_ldtk_tools/ldtk/transaction.py)
returns before finalization on dry-run.
[`write_project`](../../../tools/ambition_ldtk_tools/ambition_ldtk_tools/ldtk/io.py)
then normalizes the project only during the actual write. Both
[`edit/move.py`](../../../tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/move.py)
and
[`edit/level_clone.py`](../../../tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/level_clone.py)
write the source before `run_repair_and_validate` launches repair in place
and validation. A validation failure can therefore leave a modified source.
These commands do not yet implement the same final-candidate preview/apply
contract; level clone also needs that explicit preview surface.

#### Candidate preparation

Extend the existing `LdtkTransaction` and `PatchOp` seam. In order:

1. Load the selected source and record its bytes/revision and owning target.
2. Apply the ordered semantic operations in memory. Keep current move/clone
   identity and native `EntityRef` remapping rules.
3. Run the selected normalization/repair policy on that candidate.
4. Validate it with the relevant existing validators and original source
   context. Relative paths, vocabulary, and linked worlds resolve from that
   context, not an arbitrary temporary directory.
5. Serialize with `dump_editor_style`. Produce one final byte candidate,
   semantic diff, diagnostic result, and declared write set.

Preview displays this result. Apply installs this result. It does not rerun
a different repair pass after changing source. Reuse the existing semantic
diff and validator; do not add another identity matcher or JSON diff engine.
Keep source-relative diagnostics when validation accepts an in-memory project.

Validation policy remains explicit. The transaction currently avoids a full
world gate for local edits, and layer rules rely on that behavior. A local
operation can validate its affected structure/references and show existing
unrelated findings. A command that chooses full validation retains that
requirement. Do not turn a local move into authority to repair unrelated
rooms, drop authored fields, or change room ownership.

#### Commit and source mounts

The input source and declared destination are different roles. For in-place
editing they name the same file; `--output` names a separate destination.
Retain that explicit destination rather than redirecting it to the source.
Record whether the destination existed when preparation began.

Before writing, check the relevant source inputs and the resolved destination
against the prepared revisions or expected absence. If either changed, return a conflict
with the candidate available for review. Do not silently rebase the operation
or install a candidate prepared with different validation inputs. Specify
the participating-writer coordination used for the check/write interval; a
hash check followed by a rename is not atomic compare-and-swap against an
arbitrary external editor.

For the repository's mounted worlds, the map-submodule
[source mount rule](../../../game/ambition_map_assets/README.md)
is load-bearing: game paths are tracked symlinks. Renaming a temporary file
over the consuming link replaces the link, not the owning map source. Resolve
and verify the physical file in `game/ambition_map_assets`, stage any replacement
there, and retain the logical source path for diagnostics and references.
Refuse a dangling or changed mount. Successful commit preserves the link.
Ordinary `--ldtk` files, external projects, and explicit output paths remain
supported. They use the same verified-target rule without a requirement to
live in that submodule. Stage beside the actual target in either case.

Begin with one file. A batch of related operations shares one load and one
finalization, but retains operation order; identity allocation and edits are
not generally commutative. A later multi-file operation needs staged recovery
and a declared write set. Individual atomic replaces do not make the whole
operation atomic.

#### Migration and acceptance

Move `entity move` and `level clone` to this boundary and delete their
post-commit repair/validate subprocess sequence. Preserve clone's fresh
internal IDs and move's existing identities. Other commands can adopt the
boundary when they need it, with their own validation scope.

**Acceptance:** preview and commit have the same semantic result; refusal
writes nothing; a changed source or mount causes a conflict; a valid commit
preserves tracked symlinks; unchanged final bytes cause no rewrite. Internal
clone references and cross-level move references keep their existing meaning.
A local operation with an explicit local policy still works in a project with
unrelated validation findings. Small existing LDtk fixtures suffice; no game
build or render is needed.

**Follow-on customer:** a multi-room generator can prepare ordered room edits,
painting, links, and ownership records as one candidate set. Today
`generator_survey.py` runs a generator against live worlds and then restores
them from Git. Move that survey to candidate outputs only after the generator
accepts an explicit workspace/output context. Do not run the current survey
on a shared working tree as a proof for this planning packet.

Keep [one owner per room](../game/world-composition.md): adopted rooms remain
LDtk-owned; generated-room guards and placement ownership remain. Do not add
per-field mixed ownership or reset the generation record after a hand edit.
`area_authoring.carry_identities` already preserves and remaps identities on
stable regeneration. This review does not select a new fingerprint/identity
replacement or claim that native references alone cause false adoption.

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

## Import validation and placement lowering boundary

A3 in the [frontier](actor-monolith-work-frontier.md) moves actor/catalog-specific
placement lowering toward construction integration. LDtk and other providers
continue producing typed world input; generic world geometry must not depend on
character sheets and prepared actors merely because they share a placement file.

The carried-but-unconsumed fields (`requires_facing`, pickup `collected`, chest `persistent`, breakable
`debris_cue`) are deleted. The Q63 ruling (2026-10-03) wants the facing gate,
per-chest persistence and physical-pickup persistence, and defers the debris
cue. Each returns with its consumer in one change. An authored field that
reaches a runtime representation must have a consumer. Chest and pickup
persistence landed as occurrence-ledger rows, not as fields; `ChestSpec`
gained `opened` (Q105) and `InteractableSpec` gained `requires_facing`
(Q63), each with its consumer. The LDtk contract (`ldtk_entity_contract.json`)
declares both (`opened` on `ChestSpawn`, `requires_facing` on `NpcSpawn` and
`Switch`), the converters read them, and `sandbox.ldtk` defines them for the
editor.

Importer diagnostics should retain provider, source entity/field and normalized
semantic path so an authoring agent can fix the actual source. One shared
validation path must serve CLI preparation and game loading; a separate editor
validator is not authoritative.
