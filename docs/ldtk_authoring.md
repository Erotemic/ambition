# LDtk authoring workflow

Ambition now treats `crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk`
as the sandbox world source. LDtk owns authored spatial data; Ambition owns
runtime gameplay semantics, validation, persistence, and hot-reload policy.

All LDtk authoring goes through the `ambition_ldtk_tools` modal CLI.
Agents should not hand-edit `sandbox.ldtk` JSON; use the semantic edits
in this CLI so mutations are repaired and validated before write.

```bash
python -m ambition_ldtk_tools doctor crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
python -m ambition_ldtk_tools area create <spec.yaml> --apply
python -m ambition_ldtk_tools entity add <spec.yaml> --in-place
```

## Standard edit loop

Before opening a generated or agent-patched LDtk file in the editor, run:

```bash
python -m ambition_ldtk_tools repair crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk --in-place
python -m ambition_ldtk_tools roundtrip crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

Then open the file in LDtk, move or add supported entities, save, and run:

```bash
python -m ambition_ldtk_tools roundtrip crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
python -m ambition_ldtk_tools validate crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
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
python -m ambition_ldtk_tools schema fetch
uv pip install jsonschema
python -m ambition_ldtk_tools validate \
  --schema tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json \
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
WaterVolume
```

`CameraZone` and `StitchedBoundary` are currently accepted as editor-native
markers but are not yet active gameplay systems.

### Static-collision entities are lowered to IntGrid

`Solid`, `OneWayPlatform`, `BlinkWall`, and `HazardBlock` are still
listed above because the LDtk editor accepts them and existing
tooling consumes them, but the canonical project representation is
the **`Collision` IntGrid layer**. Every gameplay level in
`sandbox.ldtk` lives on IntGrid; `tools/ldtk_intgrid_migration.py` is
the one-shot script that lowered entity instances into IntGrid cells,
and `python -m ambition_ldtk_tools area create` *automatically lowers* `Solid` /
`OneWayPlatform` / `BlinkWall` / `HazardBlock` rectangles in any new
spec into the same IntGrid cells.

The runtime treats IntGrid-derived blocks and entity-derived
Solid/OneWay/Blink/Hazard blocks as collision-equivalent
(`int_grid_value_to_block` reconstructs the same merged rectangles),
so the lowering is transparent. The benefits:

- per-cell editing in the LDtk GUI;
- one canonical representation across the project;
- the runtime renders IntGrid blocks via `Sprite::image_mode = Tiled`
  with seamless 32×32 tile textures (`solid_tile`, `one_way_tile`,
  `hazard_tile`, `soft_blink_tile`, `hard_blink_tile`), so a long
  floor / tall wall / wide spike strip repeats cleanly instead of
  smearing one stretched image across hundreds of pixels.

The audit test `no_static_collision_entities_in_embedded_ldtk` in
`crates/ambition_sandbox/src/ldtk_world.rs` fails the build if any
of these entity types reappear, so a future authoring patch can't
silently regress.

**`DamageVolume` deliberately stays as an entity** because it can
carry motion paths (`path_points` / `path_speed` / `path_mode`) and
per-volume damage that IntGrid cells can't represent. Use
`HazardBlock` for static damage surfaces and `DamageVolume` only for
moving / variable-damage hazards.

If a future patch needs to add static collision to a level, do it on
the IntGrid layer (paint cells in LDtk, or rect in YAML), not by
adding `Solid` / `HazardBlock` entity instances. See
`tools/ambition_ldtk_tools/specs/mob_lab_area.yaml` for the rect-spec form and
`tools/ldtk_intgrid_migration.py` for the entity → IntGrid value
mapping (1=Solid, 2=OneWayUp, 3=BlinkSoft, 4=BlinkHard, 5=Hazard).

## Programmatic authoring with `author_ldtk_area.py`

Hand-editing LDtk JSON is fragile (`defUid`s, `realEditorValues`, IntGrid
sizing). For new levels, prefer:

```bash
python -m ambition_ldtk_tools area create tools/ambition_ldtk_tools/specs/examples/crawl_lab.yaml --dry-run
```

The `--dry-run` mode parses and builds the level entirely in memory and
prints a structured summary (entity counts per type, exit links, IntGrid
cell totals, biome metadata, reciprocal LoadingZones) without writing
the file or running repair/validate. Use it to verify the result matches
intent before committing to `sandbox.ldtk`.

Drop `--dry-run` to apply the spec for real; the tool runs the standard
repair + validate pipeline before exiting.

### Spec features

The spec format supports four high-level conveniences agents commonly need:

1. **Static-collision lowering.** `Solid` / `OneWayPlatform` /
   `BlinkWall` / `HazardBlock` rectangles in `entities:` are painted
   into the level's `Collision` IntGrid layer instead of being emitted
   as entity instances. The runtime treats them identically.
2. **Reciprocal LoadingZones.** A top-level `connect_to:` list inserts
   companion `LoadingZone` entities into existing target levels so an
   agent doesn't have to hand-edit two files. The helper rejects
   placements that overlap existing entities and reports the missing
   target_room with a list of known levels.
3. **Biome metadata seam.** Top-level `biome` / `music_track` /
   `ambient_profile` / `visual_theme` keys are written as level field
   instances. Run `python tools/add_biome_level_fields.py <ldtk>` once
   on a project to add the matching `defs.levelFields` entries; the
   migration is idempotent. See
   `tools/ambition_ldtk_tools/specs/examples/music_biome_lab.yaml` for the full set.
   The runtime reads these fields into `RoomSpec::metadata` per active
   area (first non-empty value wins when an area spans multiple
   levels) and mirrors the active room's metadata into the
   `ActiveRoomMetadata` Bevy resource via
   `crate::rooms::sync_active_room_metadata`. Consumers can read the
   resource with change detection without depending on `RoomSet`.
   `music_track` additionally flows through `RoomMusicRequest`, which
   `audio::apply_encounter_music` consults as the room-default track
   (encounter overrides still take priority); a typo or unknown track
   id is silently ignored so playback can't stall. The HUD shows the
   active room's metadata under `ROOM:`. To audit the current
   metadata across the project from the command line, run
   `python tools/list_ldtk_metadata.py`.
4. **Actionable error messages.** Unknown entity types ("PlayerStrt"
   → "Did you mean 'PlayerStart'?"), unknown field identifiers, and
   bad field-value coercions (a string where a Float is expected) all
   fail fast with suggestions instead of producing an LDtk file that
   silently drops the value.

### Example specs

`tools/ambition_ldtk_tools/specs/examples/` ships starter specs covering the common
sandbox-room shapes:

- `crawl_lab.yaml` — body-mode proof: low-ceiling corridor that
  requires `BodyMode::Crouching` to traverse.
- `water_lab.yaml` — buoyancy / swim mechanic proof with a
  `WaterVolume`.
- `mob_arena.yaml` — single-encounter wave demo with reciprocal hub
  link.
- `music_biome_lab.yaml` — exercises the biome metadata seam.

A round-trip smoketest (`tools/author_ldtk_area_smoketest.py`) copies
the live `sandbox.ldtk` to a tempdir, drops in a tiny test area, and
verifies the result passes both the repair pass and full validation.

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

Use `python -m ambition_ldtk_tools repair` after generated/agent patches. It can repair
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
