# LDtk authoring workflow

Ambition now treats `crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk`
as the sandbox world source. LDtk owns authored spatial data; Ambition owns
runtime gameplay semantics, validation, persistence, and hot-reload policy.

## Standard edit loop

Before opening a generated or agent-patched LDtk file in the editor, run:

```bash
python tools/repair_ambition_ldtk.py --in-place crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
python tools/check_ldtk_editor_roundtrip.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

Then open the file in LDtk, move or add supported entities, save, and run:

```bash
python tools/check_ldtk_editor_roundtrip.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
python tools/validate_ambition_ldtk.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

Run the sandbox with hot reload enabled:

```bash
cargo run -p ambition_sandbox --features dev_hot_reload --release
```

While the sandbox is running:

- `F11`: validate/apply the on-disk LDtk file.
- `F12`: toggle auto-apply after file changes.
- `F5`: overview camera for large/stitched spaces.

## Optional official LDtk schema validation

Ambition avoids npm for LDtk validation. Fetch LDtk's official JSON Schema with
Python and validate it through Python's `jsonschema` package:

```bash
python tools/fetch_ldtk_schema.py
uv pip install jsonschema
python tools/validate_ambition_ldtk.py \
  --schema tools/schemas/ldtk/JSON_SCHEMA.json \
  --require-schema \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

The official schema catches editor-format problems. Ambition's validator catches
game-specific problems such as invalid loading-zone graph links, edge exits
inside solids, unsafe transition arrivals, missing active areas, and stale
`defUid` values that can break direct `bevy_ecs_ldtk` spawning.

## Supported entity definitions

The LDtk project should define every supported Ambition entity, even if some are
not used in the current map yet. This lets designers add supported entities from
the LDtk UI without hand-editing JSON.

Currently supported identifiers:

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
```

`CameraZone` and `StitchedBoundary` are currently accepted as editor-native
markers but are not yet active gameplay systems.

## Important fields

`activeArea` is a level field. LDtk levels sharing the same `activeArea` are
stitched into one Ambition runtime room. The central hub uses this to stitch
`central_hub_main` and `central_hub_basement` into `central_hub_complex`.

`LoadingZone` fields:

- `id`: stable zone id, unique within the active area.
- `activation`: `Door` or `EdgeExit`.
- `target_room`: target active area or standalone room id.
- `target_zone`: target loading-zone id in the target room.
- `bidirectional`: whether to add the reverse graph edge.

`EdgeExit` loading zones must touch the level edge and must not overlap solid
collision. Loading zones do not cut collision. Split wall solids around exit
openings.

`DebugLabel` requires `text`. `BlinkWall` uses `tier` values `Soft` or `Hard`.
`KinematicPath` and moving `DamageVolume` path fields use semicolon-separated
points such as `0,0;64,0`.

## Do not hand-edit these unless updating tooling too

- Entity identifiers.
- Field identifiers.
- `defs.entities[*].uid`.
- Entity instance `defUid`.
- Field instance `defUid`.
- Level field `defUid`.
- `fieldInstances[*].realEditorValues`.

Use `tools/repair_ambition_ldtk.py` after generated/agent patches. It can repair
editor metadata and UID links derived from definitions, but it cannot infer
lost gameplay values after LDtk has already saved fields as `null`.
