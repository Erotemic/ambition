# LDtk tools

Location: `tools/ambition_ldtk_tools/`

Purpose: validate, repair, roundtrip, compact, inspect metadata, initialize worlds, and author areas/entities in Ambition LDtk files.

## Use this instead of hand-editing JSON

Run from the repo root:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools --help
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools repair \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  --in-place
```

Area/entity specs live under `tools/ambition_ldtk_tools/specs/`.

## Common commands

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools validate \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools roundtrip \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk

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
    crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
    --start central_hub_main --origin 0,0 --dry-run \
    --strategy "$strategy" --svg-report "/tmp/sandbox-layout-$strategy.svg"
done

# Write the layout after reviewing the dry-run report/SVG. Use --padding to
# control minimum clearance between packed groups, and --lock to keep a level
# or activeArea at its current editor coordinates while packing around it.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools world auto-layout \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
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
  --ldtk crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  --level symmetry_room --identifier CameraZone

# Move one room's CameraZone instances from Ambition to AmbitionCameras.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity change-layer \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  --level symmetry_room --identifier CameraZone \
  --from-layer Ambition --to-layer AmbitionCameras \
  --in-place

# Or migrate all CameraZones currently on Ambition in the file.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools layer split-entities \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  --type CameraZone --from-layer Ambition --to-layer AmbitionCameras \
  --in-place
```

LDtk supports entity tags plus layer `requiredTags` / `excludedTags`. The tool
can set those filters so the editor itself only offers camera zones on the
camera layer and hides them from the normal Ambition layer:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools layer apply-entity-rules \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  --type CameraZone --to-layer AmbitionCameras --from-layer Ambition \
  --tag Camera --in-place
```

For CI or agent preflight, validate the convention without mutating the file:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools layer check-entity-rules \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk
```

The default rule is `CameraZone=AmbitionCameras`; add more with repeated
`--rule EntityIdentifier=LayerIdentifier` flags or pass `--no-defaults` to use
only explicit rules.
