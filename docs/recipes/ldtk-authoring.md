# LDtk authoring workflow

Ambition treats `crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk` as the sandbox world source. LDtk owns authored spatial data; Ambition owns runtime gameplay semantics, validation, persistence, and hot-reload policy.

Agents should not hand-edit LDtk JSON. Use the `ambition_ldtk_tools` package so edits are repaired, normalized, and validated before write.

## Standard commands

Run from the repo root:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools repair \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk \
  --in-place

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools roundtrip \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools validate \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

## Standard edit loop

1. Run `doctor` on the current LDtk file.
2. Open the file in LDtk.
3. Edit levels, IntGrid collision, entities, loading zones, and metadata.
4. Save in LDtk.
5. Run `repair --in-place`, `roundtrip`, and `validate`.
6. Inspect the diff before committing.

## Runtime testing

Test an alternate map without recompiling:

```bash
cargo run -p ambition_sandbox -- --ldtk mods/my_world.ldtk
AMBITION_LDTK=mods/my_world.ldtk cargo run -p ambition_sandbox
```

For a self-contained executable, build with `--features static_map`; the checked-in `sandbox.ldtk` is embedded and used as fallback if the external map is missing or invalid.

For dev hot reload:

```bash
cargo run -p ambition_sandbox --features dev_hot_reload --release
```

Hotkeys:

- `F11`: validate/apply the on-disk LDtk file.
- `F12`: toggle auto-apply after file changes.
- `F5`: overview camera.

## Programmatic authoring

Area and entity specs live under `tools/ambition_ldtk_tools/specs/`.

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/goblin_encounter_area.yaml \
  --dry-run

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools area create \
  tools/ambition_ldtk_tools/specs/goblin_encounter_area.yaml \
  --apply

PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools entity add \
  tools/ambition_ldtk_tools/specs/hub_lab_door.yaml \
  --in-place
```

Prefer existing specs as examples. If a spec path in an old doc mentions the retired tools/examples/ldtk_specs/ directory or a top-level the retired author_ldtk_area.py script, that doc is stale.

## Collision representation

The canonical static-collision representation is the `Collision` IntGrid layer. Entity names such as `Solid`, `OneWayPlatform`, `BlinkWall`, and `HazardBlock` remain part of the editor/tool vocabulary, but current tooling lowers static rectangles into IntGrid cells where appropriate.

Common meanings:

```text
1 = Solid
2 = OneWayUp
3 = BlinkSoft
4 = BlinkHard
5 = Hazard
```

`DamageVolume` remains an entity because it may carry motion paths and per-volume damage that IntGrid cells cannot represent.

## Supported entity identifiers

```text
PlayerStart
Solid
OneWayPlatform
BlinkWall
HazardBlock
PogoOrb
ReboundPad
LoadingZone
DamageVolume
KinematicPath
NpcSpawn
PickupSpawn
ChestSpawn
Breakable
EnemySpawn
BossSpawn
DebugLabel
CameraZone
StitchedBoundary
WaterVolume
```

## Optional official schema validation

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools schema fetch
uv pip install jsonschema
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools validate \
  --schema tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json \
  --require-schema \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

The official schema catches editor-format problems. Ambition validation catches game-specific problems such as invalid loading-zone graph links, unsafe transition arrivals, missing active areas, stale definition IDs, and unknown music tracks.

Related docs: `docs/tools/ldtk-tools.md`, `docs/systems/ldtk-world-composition.md`, `docs/systems/transition-spawn-validation.md`, `docs/systems/ldtk-hot-reload.md`.
