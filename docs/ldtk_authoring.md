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

### Static-collision entities are lowered to IntGrid

`Solid`, `OneWayPlatform`, and `BlinkWall` are still listed above
because the LDtk editor accepts them and existing tooling consumes
them, but the canonical project representation is the **`Collision`
IntGrid layer**. Every gameplay level in `sandbox.ldtk` already lives
on IntGrid; `tools/ldtk_intgrid_migration.py` is the one-shot script
that lowered the entity instances into IntGrid cells, and
`tools/author_ldtk_area.py` *automatically lowers* `Solid` /
`OneWayPlatform` / `BlinkWall` rectangles in any new spec into the
same IntGrid cells (so authoring by rectangle stays ergonomic without
re-introducing entity-shaped collision in the editor).

The runtime treats IntGrid-derived blocks and entity-derived
Solid/OneWay/Blink blocks as collision-equivalent
(`int_grid_value_to_block` reconstructs the same merged rectangles),
so the lowering is transparent. The benefit is per-cell editing in
the LDtk GUI and exactly one canonical representation across the
project.

If a future patch needs to add static collision to a level, do it on
the IntGrid layer (paint cells in LDtk, or rect in YAML), not by
adding `Solid` entity instances. See
`tools/specs/mob_lab_area.yaml` for the rect-spec form and
`tools/ldtk_intgrid_migration.py` for the entity → IntGrid value
mapping (1=Solid, 2=OneWayUp, 3=BlinkSoft, 4=BlinkHard).

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

## Runtime-spine authoring and debug overlays

The first promoted `bevy_ecs_ldtk` runtime-spine categories are:

```text
PlayerStart
LoadingZone
DebugLabel
CameraZone
```

These entities are now consumed from the plugin-spawned LDtk hierarchy into an
Ambition runtime-spine index every frame. The current gameplay room still uses
Ambition's typed `RoomSet`/`World` projection, but the plugin-spawned entities
are no longer only loader-health markers: they are visible in the HUD and debug
gizmos as the first direct runtime-spine resource.

When debug gizmos and loading-zone overlays are enabled, the overlay draws both:

- Ambition runtime loading zones from the active `RoomSet`.
- Promoted plugin-spawned LDtk entities in active-area-local coordinates.

This lets authors compare raw LDtk placement against the current Ambition runtime
projection after editor edits or hot reloads. If the raw LDtk outlines and
runtime outlines drift apart, treat that as a migration bug before adding more
content.

## Hot reload transaction rules

Hot reload should feel safe while editing. A reload prepares the replacement
world completely before mutating the live world. The reload is rejected if:

- the LDtk file cannot be parsed;
- the Ambition validator reports errors;
- the current active area was deleted or renamed;
- room graph links reference missing source/target zones.

Only after the replacement `RoomSet`, active room, level-set index, and repaired
player position are ready does the sandbox despawn old room visuals/physics and
commit the new world. This policy is intentionally conservative: move the player
or change rooms before deleting the active area under them.
