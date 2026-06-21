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


# Auto-format Free-layout worlds by LoadingZone graph. This preserves activeArea
# groups as rigid clusters, anchors central_hub_main at 0,0, and places linked
# rooms near the door/edge that reaches them. Compare strategies with dry-run
# SVG reports before writing.
for strategy in greedy layered clustered; do
  PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools world auto-layout \
    crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
    --start central_hub_main --origin 0,0 --dry-run \
    --strategy "$strategy" --svg-report "/tmp/sandbox-layout-$strategy.svg"
done
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools world auto-layout \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
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
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  --level symmetry_room --identifier CameraZone \
  --from-layer Ambition --to-layer AmbitionCameras --in-place

# Make LDtk enforce the convention via entity tags and layer filters.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools layer apply-entity-rules \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  --type CameraZone --to-layer AmbitionCameras --from-layer Ambition \
  --tag Camera --in-place

# Check placement convention in CI/agent preflight.
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools layer check-entity-rules \
  crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk
```
