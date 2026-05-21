# LDtk tools

Location: `tools/ambition_ldtk_tools/`

Purpose: validate, repair, roundtrip, compact, inspect metadata, initialize worlds, and author areas/entities in Ambition LDtk files.

## Use this instead of hand-editing JSON

Run from the repo root:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools --help
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools repair \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk \
  --in-place
```

Area/entity specs live under `tools/ambition_ldtk_tools/specs/`.

## Common commands

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools validate \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools roundtrip \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk

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
