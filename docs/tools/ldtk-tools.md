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
anchors a chosen start level or active area at an origin, and greedily places
connected groups near the door/edge that reaches them while avoiding overlapping
level rectangles.

```bash
# Report-only pass. Does not mutate the LDtk file. Add --svg-report to see
# the proposed editor layout visually before writing.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools world auto-layout \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  --start central_hub_main --origin 0,0 --dry-run \
  --svg-report /tmp/sandbox-layout.svg

# Write the layout after reviewing the dry-run report/SVG. Use --padding to
# control minimum clearance between packed groups, and --lock to keep a level
# or activeArea at its current editor coordinates while packing around it.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools world auto-layout \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  --start central_hub_main --origin 0,0 \
  --padding 128 --lock central_hub_complex \
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
