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

## Agent rules

- Validate before and after semantic LDtk edits.
- Use repair/roundtrip tooling to preserve editor-compatible shape.
- Update `docs/recipes/ldtk-authoring.md` if the workflow changes.
- Treat loading zones, collision IntGrid values, active areas, and coordinate transforms as spatial review areas.
- Do not reintroduce retired top-level scripts such as the retired validate_ambition_ldtk.py script or the retired author_ldtk_area.py script.


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
  --out crates/ambition_actors/assets/sprites/editor_icons.png --tile-size 32

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset suggest-manifest \
  game/ambition_content/assets/worlds/sandbox.ldtk \
  --icons crates/ambition_actors/assets/sprites/editor_icons.png \
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
