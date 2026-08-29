# LDtk tools

Location: `tools/ambition_ldtk_tools/`

Purpose: validate, repair, roundtrip, compact, inspect metadata, initialize worlds, and author areas/entities in Ambition LDtk files.

## Use this instead of hand-editing JSON

Run from the repo root:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools --help
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  game/ambition_content/assets/worlds/sandbox.ldtk
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools repair \
  game/ambition_content/assets/worlds/sandbox.ldtk \
  --in-place
```

Area/entity specs live under `tools/ambition_ldtk_tools/specs/`.

## Common commands

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools validate \
  game/ambition_content/assets/worlds/sandbox.ldtk

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools roundtrip \
  game/ambition_content/assets/worlds/sandbox.ldtk

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/goblin_encounter_area.yaml \
  --dry-run

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity add \
  tools/ambition_ldtk_tools/specs/hub_lab_door.yaml \
  --in-place
```

### Linking one entity to another (`EntityRef` fields)

`entity set-field` understands `EntityRef` fields (added 2026-08-09 restoring the
pirates' shark mounts). **The spec names the target's iid and nothing else:**

```yaml
mounted_on: EnemySpawn-6806
```

⭐ **the tool derives `layerIid` / `levelIid` / `worldIid` from where that iid
actually lives**, which is the whole point — an `EntityRef` value is a four-key
object and hand-writing the other three is how you get a ref that resolves to
nothing. It **refuses** an iid that resolves nowhere, **refuses** a prebuilt
four-key object (with the reason), and takes `null` to clear a link.

⛔ **the silent failure this exists to prevent: a dangling ref reads back as
UNSET.** The rider simply spawns alone, no error, no warning — which is exactly
how four shark mounts sat broken from 2026-07-06 until they were noticed in play.

⚠ **constraint**: it searches `project["levels"]` only, and takes the top-level
`iid` for the world — matching every other command in the package. A multi-world
`.ldtk` with levels nested under `worlds[]` is **not handled**.

⭐ **and guard a restored link by OVERLAP, never by equal position.** A rider is
not always centred on its mount: GNU-ton rides his mount's *back*
(`[869, 754]` on `[786, 832]`). An equal-`px` assertion looks obviously right,
passes on the pirates, and reddens the one boss mount that was already working.


## Moving-platform authoring direction

The runtime/backend already understands LDtk `MovingPlatform` and
`KinematicPath` content. Current authored platform fields include size/position,
speed, simple horizontal sweep, path linkage and vertical wrapping-elevator
parameters. Do not create a parallel hard-coded platform format because the
editor surface is imperfect.

The next tooling work is tracked in
[`../planning/engine/ldtk-authoring-and-world-tools.md`](../planning/engine/ldtk-authoring-and-world-tools.md): use the existing `EntityRef` support for typed path links, improve visible path/point authoring and provide semantic diagnostics/previews for motion mode and platform travel.

## Agent rules

- Validate before and after semantic LDtk edits.
- Use repair/roundtrip tooling to preserve editor-compatible shape.
- Update `docs/recipes/ldtk-authoring.md` if the workflow changes.
- Treat loading zones, collision IntGrid values, active areas, and coordinate transforms as spatial review areas.
- Do not reintroduce retired top-level scripts such as the retired validate_ambition_ldtk.py script or the retired author_ldtk_area.py script.


## A LoadingZone is an EXIT or a LANDING PAD

A zone names **both** `target_room` and `target_zone` (an exit), or **neither**
(the arrival end of a one-way trip). Half of a target is an error, and so is a
landing pad that nothing arrives through — that is the typo the old
"every zone needs a target" rule was really catching.

⚠ **do not give a landing pad a target "for symmetry".** The body arrives
standing inside the zone it arrived through (`door_arrival` puts it at the zone's
centre, 26px off its floor), so a pad with an outgoing edge fires the moment the
0.16s transition cooldown lapses and bounces the player straight back where they
came from. A two-way route is authored as two exits at *different* places — the
way Mary-O's vault does it with one shaft down and one alcove back up — not as
one pair of zones pointing at each other.

Both validators enforce this identically: `ambition_ldtk_tools validate` for
authoring, and `LdtkProject::validate` at load.


## ⭐ The authoring contract — what the converter will refuse

`crates/ambition_platformer2d_ldtk/ldtk_entity_contract.json` is the single table
of every rule the LDtk converters enforce: which fields are REQUIRED, which
closed sets a value must come from, which fields conflict, and what happens to a
value nobody recognises. **Both languages read that one file.**

- `contract::prover` (Rust, `cargo test -p ambition_platformer2d_ldtk contract`)
  builds each entity from the table, runs the real `entity_to_runtime`, and
  asserts every claim **in both directions**. A required field whose removal does
  not fail the converter is a test failure; so is an entity that fails to convert
  when only its declared-required fields are present.
- `validate_rules/entity_contract.py` enforces the same table with no cargo in
  sight, which is the point: **an agent without a build lease cannot otherwise
  discover any of it.** `mary_o_1_3` was authored through `area create` +
  `repair` + `validate` — three affirmative OKs — with six `EnemySpawn` entities
  carrying no `character_id`, which the converter refuses.

Adding a required field is one parser change plus one edit in that JSON. ⛔ **do
not add a second list in Python** — that is the same defect one layer up.

### The three dispositions, and why one makes authoring stricter than the runtime

Each field declares what the RUNTIME does with a value its grammar rejects:

| `on_invalid` | the converter | `validate` |
| --- | --- | --- |
| `refused` | returns `Err` | error |
| `open` | falls through to a real extension point with real consumers (`CharacterBrain::Custom("mary_o_snake")`, a `PropRegistry` id) | **silent** |
| `silent_default` | accepts it and quietly substitutes a fixed value | **error anyway** |

⛔ `silent_default` is deliberately stricter in authoring than at runtime.
Nothing consumes the misspelt string, so it can only ever be a typo:
`activation: "edgeexit"` is a Door, and `currancy:1` is a pickup that vanishes on
touch and grants nothing because `collect_pickups` has no `Custom` arm. Neither
is visible in play.

### Discovering the grammar

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools vocabulary \
  --ldtk game/ambition_content/assets/worlds/sandbox.ldtk list --identifier EnemySpawn
```

`vocabulary list` now prints the contract's grammar beside the census of what the
world already authors — `REQUIRED`, the legal set, and whether an unlisted value
is an extension or an invisible typo.


## World auto-layout

For non-GridVania sandbox worlds, use `world auto-layout` to reduce editor
sprawl. The command builds a graph from `LoadingZone.target_room` /
`target_zone`, preserves all levels sharing an `activeArea` as a rigid group,
anchors a chosen start level or active area at an origin, and places connected
groups while avoiding overlapping level rectangles.

Three layout strategies are available:

- `greedy`: deterministic door-near placement, good as a stable default.
- `layered`: Sugiyama-style rank placement inferred from LoadingZone directions,
  useful for hub/basement/layered sandbox organization.
- `clustered`: first merges low-degree, tightly linked room chains into islands,
  then packs those islands, useful for sequential local room runs.

```bash
# Compare strategies visually. These passes do not mutate the LDtk file.
for strategy in greedy layered clustered; do
  PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools world auto-layout \
    game/ambition_content/assets/worlds/sandbox.ldtk \
    --start central_hub_main --origin 0,0 --dry-run \
    --strategy "$strategy" --svg-report "/tmp/sandbox-layout-$strategy.svg"
done

# Write the layout after reviewing the dry-run report/SVG. Use --padding to
# control minimum clearance between packed groups, and --lock to keep a level
# or activeArea at its current editor coordinates while packing around it.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools world auto-layout \
  game/ambition_content/assets/worlds/sandbox.ldtk \
  --start central_hub_main --origin 0,0 \
  --strategy layered --padding 128 --lock central_hub_complex \
  --report /tmp/sandbox-layout.txt --svg-report /tmp/sandbox-layout.svg \
  --in-place
```

This is an editor-formatting pass only: it updates `level.worldX/worldY` and
cached entity `__worldX/__worldY`; it does not change room contents, LoadingZone
targets, collision, or authored gameplay data. Links to target rooms outside the
current LDtk file are reported as unresolved/partial links and are not used for
packing inside the current file.

Layout locks are optional. `--lock LEVEL_OR_AREA` pins a level/activeArea at its
current editor position for one command. For persistent locks, add a boolean or
truthy string level field named `layoutLocked` (or pass `--lock-field NAME`).
The field is duck-typed: if it is absent from the project nothing happens. Use
`--ignore-field-locks` for a one-off pass that ignores persistent locks.

## Room inspection/render/debug bundles

For chat-sandbox level design, prefer the room-level helpers before opening or
mutating LDtk JSON. They are read-only and pure Python, so they can run in agent
sandboxes without LDtk or the game runtime.

```bash
# Human-readable summary: size, IntGrid values, entities, gravity zones,
# loading zones, moving platforms, cameras, and static review notes.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools room describe \
  --level symmetry_room

# Visual room preview. SVG includes labels; PNG is dependency-free and useful
# when the chat UI previews raster images more reliably.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools room render \
  --level symmetry_room --out /tmp/symmetry_room.svg
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools room render \
  --level symmetry_room --out /tmp/symmetry_room.png

# Bundle the summary, JSON summary, render, matching specs, and relevant
# debug_traces JSON files into one uploadable artifact.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools room bundle-debug \
  --level symmetry_room --out /tmp/symmetry_room_debug.tar.gz
```

This is intended to make LLM-assisted room design less brittle: the assistant can
reason from a compact text summary, a single visual artifact, and relevant trace
failures instead of asking for the whole repo or guessing LDtk coordinates.

## Entity layer hygiene

Large editor-only volumes such as `CameraZone` should live on a dedicated
Entities layer instead of the catch-all `Ambition` layer. This makes the layer
lockable/hideable in LDtk and keeps future agent-authored content from placing
camera volumes on the gameplay interaction layer.

```bash
# Inspect the current camera zone placement in a room.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity query \
  --ldtk game/ambition_content/assets/worlds/sandbox.ldtk \
  --level symmetry_room --identifier CameraZone

# Move one room's CameraZone instances from Ambition to AmbitionCameras.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity change-layer \
  game/ambition_content/assets/worlds/sandbox.ldtk \
  --level symmetry_room --identifier CameraZone \
  --from-layer Ambition --to-layer AmbitionCameras \
  --in-place

# Or migrate all CameraZones currently on Ambition in the file.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools layer split-entities \
  game/ambition_content/assets/worlds/sandbox.ldtk \
  --type CameraZone --from-layer Ambition --to-layer AmbitionCameras \
  --in-place
```

If no entities match, the command is a no-op and leaves the file unchanged. The
command writes editor-style JSON directly and intentionally skips full
LoadingZone validation so cross-LDtk links do not break unrelated layer hygiene
changes.

LDtk supports entity tags plus layer `requiredTags` / `excludedTags`. The tool
can set those filters so the editor itself only offers camera zones on the
camera layer and hides them from the normal Ambition layer:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools layer apply-entity-rules \
  game/ambition_content/assets/worlds/sandbox.ldtk \
  --type CameraZone --to-layer AmbitionCameras --from-layer Ambition \
  --tag Camera --in-place
```

For CI or agent preflight, validate the convention without mutating the file:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools layer check-entity-rules \
  game/ambition_content/assets/worlds/sandbox.ldtk
```

Layer relocation writes editor-style JSON directly and does not run full
LoadingZone validation as a post-pass. This keeps the commands safe for sandbox
worlds that intentionally link to rooms in other LDtk files. Use
`repair --in-place` separately when you specifically want full validation.

The default rule is `CameraZone=AmbitionCameras`; add more with repeated
`--rule EntityIdentifier=LayerIdentifier` flags or pass `--no-defaults` to use
only explicit rules.

## Agent toolbox workflow

For reviewable generated LDtk edits, prefer this loop:

```bash
# 1. Inspect current room state.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools room describe --level symmetry_room
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools room render --level symmetry_room --out /tmp/symmetry_room.svg

# 2. Apply generated edits through intent-level tools, not raw JSON.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools room compile-spec specs/patch.json --ldtk sandbox.ldtk --dry-run

# 3. Check policy and camera coverage.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools policy check sandbox.ldtk
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools camera audit sandbox.ldtk --level symmetry_room

# 4. Review semantic changes instead of noisy JSON.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools diff semantic before.ldtk after.ldtk
```

Use `asset catalog` and `asset link-entity-tile` when generated sprites or
visual tiles are ready to be exposed to LDtk for nicer human editing.

## Visual manifests and editor icons

Runtime sprite metadata should remain owned by the sprite generator. LDtk should
consume concrete tileset/entity-icon refs compiled from a manifest. This keeps
LDtk useful for human editing without binding the tools to the transitional
sprite metadata schema.

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset generate-editor-icons \
  --out crates/ambition_platformer2d_actor_monolith/assets/sprites/editor_icons.png --tile-size 32

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset suggest-manifest \
  game/ambition_content/assets/worlds/sandbox.ldtk \
  --icons crates/ambition_platformer2d_actor_monolith/assets/sprites/editor_icons.png \
  --out tools/ambition_ldtk_tools/manifests/sandbox_visuals.json

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset apply-manifest \
  game/ambition_content/assets/worlds/sandbox.ldtk \
  tools/ambition_ldtk_tools/manifests/sandbox_visuals.json --in-place

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset validate-manifest \
  game/ambition_content/assets/worlds/sandbox.ldtk \
  tools/ambition_ldtk_tools/manifests/sandbox_visuals.json
```

`policy check` also validates stale or out-of-bounds entity editor tile refs, and
`diff semantic` reports `entity_def_visual` changes so generated visual updates
are reviewable without raw JSON diffs.

## Editor art: showing the level the engine will draw

`asset editor-art` is the one-command version of the above for a world that
wants to look like itself. It composes an atlas out of the ENGINE's own sprite
folder, registers it, gives every IntGrid layer a sibling AutoLayer that draws
its values and every 1:1 entity def a `tileRect` — so painting `Solid` draws
masonry in the editor, and a `ChestSpawn` looks like the chest.

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset editor-art \
  game/ambition_demo_mary_o/assets/worlds/mary_o.ldtk --in-place --preview /tmp/mary_o.png
```

- The art comes from `assets/sprites/entities/*.png` — the same textures the
  renderer binds — so an editor cell cannot drift from the block that spawns
  under it. Tile art is 32px against a 16px collision grid, so each texture is
  four cells and four `Single` rules phased by `xModulo`/`yModulo`.
- ⛔ **the art goes on a `<Layer>Art` AutoLayer, never on the IntGrid layer
  itself.** A matched rule replaces the cell's colour, so rules on the collision
  layer HIDE the collision — which values are where, and where a surface ends.
  The sibling layer reads it through `autoSourceLayerDefUid` instead, and the
  source layer's `inactiveOpacity` drops so its colours read as a tint over the
  art while you work on something else. Select it and it goes solid for
  painting.
- **A property can choose the picture.** `field_art` in the sidecar maps an
  ENUM field's values to art and sets the def's `editorDisplayMode` to
  `EntityTile`, so `MaryOBlock` draws a `?`-plate or masonry per placement
  according to its `kind`. The enum itself is declared in the world's entity
  manifest (`"type": "Enum"`, `"enum"`, `"values"`) — the vocabulary owns which
  words are sayable, this owns what they look like. A field on an ENGINE def
  that no per-world manifest declares can name its enum in the sidecar instead
  (`{"enum": …, "values": {…}}`), which is how Mary-O's `EnemySpawn.brain` shows
  a slop as a slop and a snake as a snake. ⚠ closing a field is refused when the
  level already authors a value the enum cannot spell, and it costs whatever the
  runtime allowed beyond the list — `parse_enemy_brain` also takes
  `Guard:<radius>` and any custom id, which a dropdown cannot say.
- **`field_display`** puts a field's value beside its entity
  (`"MaryOBlock.contents": "NameAndValue"`). The game never reads
  `editorDisplayMode`, so this is how the editor can show what a block HOLDS
  without the block announcing it in play.
- ⛔ **the tiles ARE baked into `autoLayerTiles`, because LDtk does not
  re-evaluate an auto-layer when it opens a file.** It renders its own cache, so
  a generated layer that leaves the cache empty draws nothing however right its
  rules are — which is what shipped first, and what a flat grey slab in the
  editor looks like. LDtk recomputes and overwrites on the first edit to the
  source, and the cache is a pure function of cells the file already carries.
- A world's own nouns go in `<world>.editor_art.json` beside the `.ldtk`
  (`{"entity_art": {"MaryOPipe": "props/mary_o_pipe_top"}}`); a character icon is
  `{"sheet": "ai_slop", "animation": "idle", "frame": 0}`, whose rect is read
  from the sheet's sidecar — ⛔ never computed from `frame_width`, which is the
  design size and not the packing pitch of a packed sheet.
- The atlas PNG is generated and gitignored with the sprites it is made of;
  `scripts/regen/sprites.sh` rebuilds it. The WIRING is committed in the `.ldtk`.
- `--preview` renders the level as the rules will draw it. It proves the art,
  the rects and the tile ids; it cannot prove LDtk agrees about the rule fields,
  which only opening the editor can.

## Internal architecture notes

The LDtk editor JSON stays as plain Python dictionaries, but low-level mechanics
should go through `ambition_ldtk_tools.ldtk` rather than being reimplemented in
feature modules. That package owns shared project load/write, UID allocation,
path normalization, PNG dimension probing, definition lookup, entity iteration,
field helpers, and Entities-layer creation.

Feature modules should follow this shape:

```text
CLI parser
  -> intent-specific service logic
  -> shared LDtk core helpers for lookup/writeback
```

Avoid adding new ad-hoc helpers named `load_project`, `write_project`,
`alloc_uid`, `find_layer_def`, `find_entity_def`, `find_layer_instance`, or
`png_dimensions` inside command modules. Add shared behavior to the LDtk core
package instead. This keeps correctness emergent from one implementation of the
LDtk file mechanics and makes no-op/dry-run/writeback behavior easier to audit.

### Transaction and patch boundary

The LDtk tool now has a small transaction/patch foundation under
`ambition_ldtk_tools.ldtk`:

- `patch.py`: composable dict-backed patch operations, currently including
  entity layer moves and tag-based layer rule metadata.
- `transaction.py`: one shared writeback path for dry-run/no-op/output/backup and
  editor-style LDtk JSON writes.

New mutating commands should not decide writeback semantics locally. Prefer:

```python
from ambition_ldtk_tools.ldtk import LdtkTransaction, MoveEntitiesToLayer

tx = LdtkTransaction(path, dry_run=args.dry_run, in_place=args.in_place, output=args.output)
tx.apply(MoveEntitiesToLayer(...))
tx.finish(noop_message="no matching entities; left file unchanged")
```

This is the migration seam for future cleanups: area specs, camera edits,
visual manifest writes, IntGrid paint commands, and layout writeback should all
compile down to shared patch/transaction operations over time.

### Structured issue model

LDtk diagnostics now have a shared `Issue` model under `ambition_ldtk_tools.ldtk`.
Use it for policy, validation, camera, visual-reference, and room-inspection
findings. JSON CLI output should use `Issue.as_dict()`; text output should use
`format_issue_lines(...)`. This gives agents stable fields such as `severity`,
`code`, `level`, `layer`, `entity`, `entity_iid`, `fixable`, and `fix_hint`
instead of forcing them to parse one-off prose.

### Current refactor roadmap snapshot

- Done: shared LDtk core helpers own JSON load/write, lookup, path, field, UID, and layer mechanics.
- Done: transaction/patch helpers own dry-run/no-op/writeback semantics for migrated mutating commands.
- Done: shared `Issue` diagnostics now cover policy, camera, visual refs, validation adapter, and room notes.
- Done: layout model, room issue checks, and area spec loading have package seams behind stable CLI entrypoints.
- Next: split `validate.py` internals into rule modules that emit first-class `Issue` codes directly.
- Next: move the remaining `world_layout.py` graph, strategy, SVG, and writeback functions into `edit/layout/*`.
- Next: move room inspection/render/bundle code into `room_support/*` and keep `room.py` as a CLI adapter.
- Next: compile area authoring specs to patch ops before mutating LDtk directly.
- Later: relocate game content specs out of the reusable Python package tree.

### Refactor architecture notes

The LDtk tools are being split so correctness comes from shared seams instead of
per-command JSON mutation logic:

- `ambition_ldtk_tools.ldtk.*` owns low-level LDtk IO, queries, fields, patch ops, transactions, and shared issue objects.
- `ambition_ldtk_tools.validate_rules.*` owns validation rule helpers and maps legacy messages to first-class issue codes.
- `ambition_ldtk_tools.edit.layout.*` owns world layout graph building, strategies, SVG previews, and writeback/reporting.
- `ambition_ldtk_tools.room_support.*` owns room inspection, rendering, and debug bundle construction.
- `ambition_ldtk_tools.area.*` owns area spec loading and the new patch-plan seam used before mutating LDtk projects.

The public CLI entrypoints remain stable while implementation files move behind
these packages. If a later overlay turns a legacy `.py` entrypoint into a package
or removes dead wrappers, include explicit `git rm` cleanup commands because ZIP
overlays cannot delete files.

### LDtk tool architecture notes

The LDtk tools are being migrated away from command-local JSON mutation.
Prefer these shared seams for new work:

- `ldtk.transaction.LdtkTransaction` for load/mutate/writeback behavior.
- `edit.postprocess.run_repair_and_validate` for standard post-write repair and validation.
- `ldtk.issues.Issue` for structured diagnostics and JSON output.
- `area.plan.AreaPatchPlan` for compiling authoring specs before mutating projects.

This keeps correctness in common helpers instead of duplicating dry-run, backup,
repair, and validation logic across every edit command.
