# Ambition LDtk Tools

Modal CLI for editing, validating, and repairing the Ambition `sandbox.ldtk` world. Agents should not hand-edit LDtk JSON; use this package so mutations are repaired and validated before write.

Run commands from the repository root with the package directory on `PYTHONPATH`:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools <subcommand> ...
```

## Common commands

```bash
# Validate gameplay/editor contracts without mutating the file.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools validate \
  crates/ambition_content/assets/worlds/sandbox.ldtk

# Check whether the package repair pass would change the file.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools roundtrip \
  crates/ambition_content/assets/worlds/sandbox.ldtk

# Run roundtrip + validate.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  crates/ambition_content/assets/worlds/sandbox.ldtk

# Repair in place, then inspect the diff before committing.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools repair \
  crates/ambition_content/assets/worlds/sandbox.ldtk \
  --in-place
git diff -- crates/ambition_content/assets/worlds/sandbox.ldtk

# Schema helpers.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools schema fetch
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools schema validate \
  crates/ambition_content/assets/worlds/sandbox.ldtk \
  --schema tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json \
  --require-schema

# Authoring helpers.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/goblin_encounter_area.yaml \
  --dry-run
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/goblin_encounter_area.yaml
# Regenerate an existing spec-owned level in place.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/goblin_encounter_area.yaml \
  --replace-existing
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity add \
  tools/ambition_ldtk_tools/specs/hub_lab_door.yaml \
  --in-place
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools def register-entity \
  tools/ambition_ldtk_tools/specs/encounter_and_switch_entities.yaml \
  --in-place

# Author a LINKED portal pair in one command. Two ends share a `link` id (the
# explicit pairing model); a link that is not exactly two members is closed in
# game. NORMAL = the surface the portal sits ON: up=floor, down=ceiling,
# left=right-wall, right=left-wall (world y is down). The box SIZE sets the
# opening length; a mismatched pair opens the MINIMUM, centered (no scaling).
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools portal pair \
  --level portal_lab --link demo_door \
  --a 300 891 up --b 600 700 left \
  --id demo --name "demo gate" --size 120 18 --in-place


# Auto-format Free-layout worlds by LoadingZone graph. This preserves activeArea
# groups as rigid clusters, anchors central_hub_main at 0,0, and places linked
# rooms near the door/edge that reaches them. Compare strategies with dry-run
# SVG reports before writing.
for strategy in greedy layered clustered; do
  PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools world auto-layout \
    crates/ambition_content/assets/worlds/sandbox.ldtk \
    --start central_hub_main --origin 0,0 --dry-run \
    --strategy "$strategy" --svg-report "/tmp/sandbox-layout-$strategy.svg"
done
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools world auto-layout \
  crates/ambition_content/assets/worlds/sandbox.ldtk \
  --start central_hub_main --origin 0,0 --strategy layered --padding 128 \
  --report /tmp/sandbox-layout.txt --svg-report /tmp/sandbox-layout.svg --in-place
# Strategies: greedy = legacy door-near packing, layered = Sugiyama-style ranks,
# clustered = low-degree linkage merging, then packing the merged room islands.
# Use --lock LEVEL_OR_AREA for one-off pinned placements. Persistent locks are
# duck-typed from a truthy level field named layoutLocked if the project defines it.

# Room-level sandbox helpers: summarize, render, and bundle room context without LDtk/the game.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools room describe \
  --level symmetry_room
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools room render \
  --level symmetry_room --out /tmp/symmetry_room.svg
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools room bundle-debug \
  --level symmetry_room --out /tmp/symmetry_room_debug.tar.gz

# Read-only spatial queries (answer placement questions before editing;
# see docs/concepts/llm-spatial-authoring-discipline.md).
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools intgrid query \
  --level goblin_encounter --px 480,400 --size 224,208   # what collision is here?
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity measure \
  --level goblin_encounter --identifier Switch            # size + center + nearest solids
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools gates audit \
  --level goblin_encounter                                # switches / lock walls / triggers / breakables
```

## Specs

Current specs live directly under `tools/ambition_ldtk_tools/specs/`. Prefer copying an existing spec and changing IDs/coordinates instead of inventing a new schema shape.

## Retired entry points

Older docs may mention top-level scripts such as the retired validate_ambition_ldtk.py script, the retired repair_ambition_ldtk.py script, the retired check_ldtk_editor_roundtrip.py script, or the retired author_ldtk_area.py script. Those entry points are retired. Use `python -m ambition_ldtk_tools` instead.

## Agent rules

- Do not hand-edit `sandbox.ldtk` JSON.
- Run `doctor` before committing LDtk changes.
- Use `repair --in-place` and inspect the diff when the editor/tooling format changes.
- Keep `docs/recipes/ldtk-authoring.md` and `docs/tools/ldtk-tools.md` aligned with this README.

### Entity layer hygiene

Use dedicated Entities layers for large editor-only volumes such as camera
zones. The runtime loader reads every Entities layer, so moving `CameraZone`
instances out of `Ambition` and into `AmbitionCameras` keeps gameplay behavior
unchanged while making LDtk editing saner.

```bash
# Move CameraZones in one room.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity change-layer \
  crates/ambition_content/assets/worlds/sandbox.ldtk \
  --level symmetry_room --identifier CameraZone \
  --from-layer Ambition --to-layer AmbitionCameras --in-place

# Make LDtk enforce the convention via entity tags and layer filters.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools layer apply-entity-rules \
  crates/ambition_content/assets/worlds/sandbox.ldtk \
  --type CameraZone --to-layer AmbitionCameras --from-layer Ambition \
  --tag Camera --in-place

# Check placement convention in CI/agent preflight.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools layer check-entity-rules \
  crates/ambition_content/assets/worlds/sandbox.ldtk
```

Layer relocation writes editor-style JSON directly and does not run full
LoadingZone validation as a post-pass. This keeps the commands safe for sandbox
worlds that intentionally link to rooms in other LDtk files. Use
`repair --in-place` separately when you specifically want full validation.

## Agent toolbox additions

These commands are intended for chat-sandbox authoring and CI preflight. They are
mostly read-only, emit compact reports, and avoid noisy raw LDtk JSON diffs.

### Semantic diff

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools diff semantic \
  before.ldtk after.ldtk
```

Reports level moves/resizes, entity layer moves, entity field changes, IntGrid
value-count changes, entity/layer definition additions/removals, and tileset
changes.

### Policy checks and safe fixes

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools policy check \
  crates/ambition_content/assets/worlds/sandbox.ldtk
```

Default policy includes `CameraZone=AmbitionCameras`. Add project-specific rules
with repeated `--rule Entity=Layer`. Use `policy fix --in-place` for safe entity
layer moves.

### Camera helpers

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools camera audit \
  crates/ambition_content/assets/worlds/sandbox.ldtk --level symmetry_room

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools camera auto-cover \
  crates/ambition_content/assets/worlds/sandbox.ldtk \
  --level symmetry_room --margin 64 --create --in-place
```

`auto-cover` creates or updates a `CameraZone` on `AmbitionCameras` using the
collision play envelope, expanded by `--margin`.

### Asset and editor-sprite helpers

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset catalog \
  crates/ambition_content/assets/worlds/sandbox.ldtk

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset link-entity-tile \
  crates/ambition_content/assets/worlds/sandbox.ldtk \
  --entity PlayerStart --tileset my_tiles --tile 0,0,16,16 --in-place
```

`asset catalog` lists registered LDtk tilesets, entity definitions that already
have editor tile art, PNGs under the game asset tree, and PNGs that are not yet
registered as LDtk tilesets. `link-entity-tile` points an entity definition at a
registered tileset tile so humans see useful editor art instead of abstract
rectangles.

### Compact room specs

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools room compile-spec \
  specs/my_room_patch.json --ldtk crates/ambition_content/assets/worlds/sandbox.ldtk \
  --dry-run
```

JSON specs can paint IntGrid rectangles, add common entities, and request camera
auto-cover. RON specs are supported when `python-ron` is installed.

### Visual manifest scaffolding

The sprite/tileset refactor owns runtime art metadata. The LDtk tools only own
editor integration: registering LDtk tilesets, assigning entity editor icons,
validating those refs, and making visual changes reviewable. Use a small visual
manifest as an adapter boundary until the final generator metadata lands.

```bash
# Generate a placeholder editor icon sheet that is independent of production art.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset generate-editor-icons \
  --out crates/ambition_actors/assets/sprites/editor_icons.png \
  --tile-size 32

# Draft a manifest that maps common editor entities to that sheet.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset suggest-manifest \
  crates/ambition_content/assets/worlds/sandbox.ldtk \
  --icons crates/ambition_actors/assets/sprites/editor_icons.png \
  --out tools/ambition_ldtk_tools/manifests/sandbox_visuals.json

# Apply or validate LDtk-side visual refs. This mutates only LDtk defs: tilesets
# and entity editor tileRect/uiTileRect references.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset apply-manifest \
  crates/ambition_content/assets/worlds/sandbox.ldtk \
  tools/ambition_ldtk_tools/manifests/sandbox_visuals.json \
  --in-place
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset validate-manifest \
  crates/ambition_content/assets/worlds/sandbox.ldtk \
  tools/ambition_ldtk_tools/manifests/sandbox_visuals.json

# Generate a compact HTML preview for human review.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools asset preview-manifest \
  crates/ambition_content/assets/worlds/sandbox.ldtk \
  tools/ambition_ldtk_tools/manifests/sandbox_visuals.json \
  --out /tmp/sandbox_visuals.html
```

The manifest is intentionally simple and generator-agnostic:

```json
{
  "editor_icons": {
    "identifier": "EditorIcons",
    "path": "crates/ambition_actors/assets/sprites/editor_icons.png",
    "tile_width": 32,
    "tile_height": 32
  },
  "entity_icons": {
    "CameraZone": {"tileset": "EditorIcons", "index": 0},
    "LoadingZone": {"tileset": "EditorIcons", "index": 1}
  }
}
```

When the sprite refactor settles, add an adapter that generates this manifest
from the canonical RON/YAML sprite metadata. Do not hand-maintain LDtk JSON rects.

## Internal helper layout

Low-level LDtk JSON mechanics live under `ambition_ldtk_tools.ldtk`:

- `io.py`: load/write editor-style projects.
- `ids.py`: `nextUid` allocation.
- `paths.py`: repo-relative and LDtk-relative paths plus PNG dimensions.
- `query.py`: definitions, levels, layers, tilesets, and entity iteration.
- `fields.py`: entity/default field helpers.
- `layers.py`: shared Entities-layer definition/instance creation.

New commands should use those helpers instead of creating local copies of
`load_project`, `write_project`, `alloc_uid`, `find_*`, or `png_dimensions`.
The CLI should remain a thin parser/dispatcher; command modules should express
intent and delegate raw LDtk mechanics to the shared core.

### Transaction and patch helpers

Mutating commands should use `ambition_ldtk_tools.ldtk.LdtkTransaction` plus
small patch operations from `ambition_ldtk_tools.ldtk.patch` instead of open-coding
writeback.  This keeps dry-run, no-op, backup, output, and editor-style JSON
behavior consistent across commands.

Current shared patch operations cover entity layer moves and LDtk tag-based
entity/layer placement rules.  Future refactors should move area authoring,
visual refs, IntGrid painting, and world layout writeback behind the same patch
boundary so commands become:

```text
parse CLI intent -> build patch ops -> transaction applies/writes
```

### Structured issues

Diagnostics should use `ambition_ldtk_tools.ldtk.Issue` instead of command-local
issue dataclasses. `Issue.as_dict()` is the stable JSON shape for agents and CI;
text output should pass through `format_issue_lines(...)` so severity, code,
location, fixability, and fix hints stay consistent across commands.

Commands already using the shared issue model include policy checks, camera
audits, and visual-manifest validation. Future validation, room-inspection, and
asset commands should emit the same shape instead of prose-only messages.

### Refactor seams added in v12-v15

The historical public modules remain importable and keep the CLI stable, but new
code should prefer these package seams:

- `ambition_ldtk_tools.validate.validate_issues(...)`: compatibility adapter that
  returns shared `Issue` objects while the long validator is split into rule
  modules.
- `ambition_ldtk_tools.room_support.issues.room_issues(...)`: structured room
  review notes used by `room describe` summaries.
- `ambition_ldtk_tools.edit.layout.model`: layout dataclasses shared by future
  graph, strategy, SVG, and writeback modules.
- `ambition_ldtk_tools.area.spec.load_spec(...)`: area spec loading extracted
  from the monolithic authoring command.
- `ambition_ldtk_tools.area.plan.AreaPatchPlan`: placeholder seam for the future
  `AreaSpec -> patch ops -> LdtkTransaction` compiler.

Because overlay ZIPs do not delete stale files, avoid converting existing
`room.py`-style modules directly into same-named packages unless the final
instructions include an explicit cleanup command.

## Internal refactor seams

The tool package is intentionally moving toward thin CLIs over shared internals:

- `ldtk/` for IO, queries, fields, patch ops, transactions, and shared issues.
- `validate_rules/` for validation rule helpers and first-class issue codes.
- `edit/layout/` for graph construction, layout strategies, SVG previews, and writeback.
- `room_support/` for room inspect/render/bundle helpers.
- `area/` for area spec loading and patch-plan compilation.

Keep new commands on these seams instead of re-implementing raw LDtk JSON access,
writeback, no-op handling, or diagnostic formatting locally.

### Refactor note: transactions and first-class validation issues

Recent LDtk-tool refactors established shared seams for future commands:

- Mutating commands should prefer `ambition_ldtk_tools.ldtk.transaction.LdtkTransaction` for dry-run/no-op/backup/writeback behavior.
- Commands that run repair + validate after writing should use `ambition_ldtk_tools.edit.postprocess.run_repair_and_validate`.
- Validation rule modules should emit `ambition_ldtk_tools.ldtk.issues.Issue` directly when possible; legacy string diagnostics are compatibility-only.
- Area-authoring create now compiles to `AreaPatchPlan` operations before applying them, which is the migration path toward fully typed LDtk patch ops.
