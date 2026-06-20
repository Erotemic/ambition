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

