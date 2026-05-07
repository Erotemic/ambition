# `ambition_ldtk_tools area create`

Author a new Ambition LDtk active area / level from a small YAML or
JSON spec. Hides the LDtk JSON minutiae (`defUid` sync,
`realEditorValues`, `__smartColor`, `intGridCsv` sizing, `activeArea`
field shape, world-frame overlap checks) behind a high-level entity
list so an agent (or human) can ship a new level in seconds without
re-learning the editor-roundtrip discipline that
`docs/lessons_learned.md` enumerates.

This used to live at `tools/author_ldtk_area.py` (a deprecation shim
still works); it now ships as `python -m ambition_ldtk_tools area
create`.

## Usage

```bash
# Default target is sandbox.ldtk; --backup is recommended on first runs.
python -m ambition_ldtk_tools area create path/to/spec.yaml --backup

# Validate against the official LDtk JSON schema as well (default).
python -m ambition_ldtk_tools area create spec.yaml \
    --ldtk crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk \
    --schema tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json

# Write to a separate file (good for one-shot review before committing).
python -m ambition_ldtk_tools area create spec.yaml --output /tmp/sandbox.preview.ldtk
```

The tool runs the package's repair pass (`ambition_ldtk_tools repair`)
followed by validation (`ambition_ldtk_tools validate`) automatically;
it exits non-zero if either fails.

## Spec format

```yaml
id: mob_lab                    # required: activeArea string (groups levels)
level_id: mob_lab              # required: level identifier (must be unique)
world_x: 16000                 # required: world-frame placement (16px aligned)
world_y: 1024
px_wid: 1800                   # required: level pixel size (multiple of grid_size)
px_hei: 900
grid_size: 16                  # optional, defaults to project defaultGridSize
fill_collision: solid_border   # optional: empty | solid_border | solid_floor
bg_color: "#1a1a24"            # optional, defaults to project defaultLevelBgColor
entities:
  - type: PlayerStart
    px: [60, 60]               # level-local pixel position (top-left corner)
    size: [28, 46]             # optional, defaults to entity def's authored size
    fields:
      name: lab_start
  - type: Solid
    px: [0, 800]
    size: [1800, 100]
    fields: { name: floor }
  - type: LoadingZone
    px: [0, 600]
    size: [60, 100]
    fields:
      id: lab_exit
      name: lab_exit
      activation: walk
      target_room: central_hub_complex
      target_zone: east_exit   # must match an existing LoadingZone id
      bidirectional: false
```

### Top-level keys

| Key                | Required | Notes                                                        |
| ------------------ | -------- | ------------------------------------------------------------ |
| `id`               | yes      | Goes into the `activeArea` level field. Levels with the same `id` compose into one runtime active area. |
| `level_id`         | yes      | LDtk level identifier; must be unique across the project.    |
| `world_x` / `world_y` | yes   | Top-left of the level in the LDtk world frame. Should be a multiple of `grid_size` (the validator emits a warning otherwise). |
| `px_wid` / `px_hei`   | yes   | Level pixel size; must be a multiple of `grid_size`.         |
| `grid_size`        | no       | Defaults to the project's `defaultGridSize` (16).            |
| `bg_color`         | no       | Defaults to the project's `defaultLevelBgColor`.             |
| `fill_collision`   | no       | `empty` (no IntGrid solids; entity-only collision), `solid_border` (1-cell ring around the room), `solid_floor` (1-cell strip on the bottom row). The IntGrid value `1` is `Solid` per the project's layer def. Default `empty`. |
| `entities`         | no       | List of entity dicts. Empty is OK — useful when only the IntGrid floor is needed. |

### Entity entries

| Key      | Required | Notes                                                       |
| -------- | -------- | ----------------------------------------------------------- |
| `type`   | yes      | Must match an `identifier` from the project's `defs.entities`. |
| `px`     | yes      | `[x, y]` top-left pixel coordinate, level-local.            |
| `size`   | no       | `[width, height]` pixels. Defaults to the entity def's authored size. |
| `name`   | no       | Convenience: shorthand for `fields: { name: ... }`.         |
| `fields` | no       | Mapping from field identifier → value. Coerced to the type declared in the entity def's `fieldDefs` (`String` / `Bool` / `Int` / `Float`). Missing fields are simply omitted. |

## What the tool does NOT do

- **Author non-rectangular geometry.** The IntGrid layer ships with one
  of three preset fills; richer collision needs hand-painting in the
  LDtk editor or extending `make_intgrid_csv`.
- **Validate `LoadingZone` targets.** It emits the field instances as
  given; `validate_ambition_ldtk.py` (run automatically afterward)
  catches missing `target_zone` references and rejects the file.
- **Apply LDtk room-graph repair.** The runtime `RoomSet::from_parts`
  loader still does final spawn / arrival repair when the level is
  consumed. The tool only ensures the file is editor-roundtrip-clean.
- **Edit existing levels.** It only appends a new level. To extend an
  existing area with another level, run the tool with a new
  `level_id` and the same `id`.

## Entity / field identifier reference

The tool reads the project's `defs.entities` directly, so the
identifiers it accepts always match the live schema. As of this
patch the available entity identifiers are:

`PlayerStart`, `Solid`, `LoadingZone`, `OneWayPlatform`,
`BlinkWall`, `PogoOrb`, `NpcSpawn`, `DebugLabel`, `EnemySpawn`,
`HazardBlock`, `ReboundPad`, `DamageVolume`, `KinematicPath`,
`BossSpawn`, `BreakablePlatform`, `BreakablePogoOrb`, `PickupSpawn`,
`ChestSpawn`, `CameraZone`, `StitchedBoundary`.

To see each entity's field set, grep the project file:

```bash
python3 -c "
import json
p = json.load(open('crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk'))
for e in p['defs']['entities']:
    fields = [(f['identifier'], f.get('__type')) for f in e.get('fieldDefs', [])]
    print(e['identifier'], 'color=', e.get('color'), 'fields=', fields)
"
```

## Smoke test

`tools/author_ldtk_area_smoketest.py` copies the live sandbox file
into a temp dir, drops in a tiny test area with `PlayerStart` +
`Solid` floor + `LoadingZone`, runs the tool, then asserts:

- the new level appears,
- the `activeArea` field is set,
- every entity got `realEditorValues` filled in by the repair pass,
- field types coerce correctly (`Bool` `False` survives the round
  trip),
- the `intGridCsv` length matches `cWid * cHei` and the
  `solid_floor` fill produced 1s on the bottom row only.

Run it before committing changes to the tool itself:

```bash
python tools/author_ldtk_area_smoketest.py
```

## Why a tool instead of multiple LDtk files

LDtk groups levels in one file via the `activeArea` level field; the
sandbox runtime composes them in `RoomSet::from_parts`. Adding a new
area is fundamentally a new level in the existing file, and the only
real pain was the JSON minutiae — exactly what this tool removes.
Splitting the project across multiple `.ldtk` files would require
reworking `RoomSet::from_parts`, hot reload, and the asset
collection for marginal benefit; see `docs/AGENT_HANDOFF.md` for the
LDtk runtime spine roadmap.
