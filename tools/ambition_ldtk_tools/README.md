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
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk

# Check whether the package repair pass would change the file.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools roundtrip \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk

# Run roundtrip + validate.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk

# Repair in place, then inspect the diff before committing.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools repair \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  --in-place
git diff -- crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk

# Schema helpers.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools schema fetch
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools schema validate \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
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
