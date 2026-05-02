# LDtk world-composition adapter

The sandbox now starts moving toward LDtk-authored world composition.

## Files

```text
crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
crates/ambition_sandbox/src/ldtk_world.rs
tools/validate_ambition_ldtk.py
```

`assets/ambition/sandbox.ron` still owns abilities, movement tuning, generated audio specs, and fallback/historical room data. At startup, the sandbox loads the RON data, then replaces `rooms` with the embedded LDtk-derived manifest.

## Current proof of concept

The LDtk file now ports the old sandbox map into LDtk active areas. The central hub remains special: it contains two authored chunks that are stitched into one runtime room:

```text
central_hub_main      at world position (0, 0)
central_hub_basement  at world position (0, 1024)
```

Both chunks declare the same `activeArea` level field:

```text
central_hub_complex
```

The adapter composes them into one Ambition runtime room. The player starts in the hub and can drop through the authored floor opening into the basement without a loading zone. The basement hub itself contains restored doors to separate LDtk-authored feature labs:

```text
basement_hazards
basement_enemies
basement_boss
basement_breakables
basement_treasure
basement_npcs
```

The old overworld doors are also represented in LDtk as loading-zone links from `central_hub_complex` to `scroll_lab`, `vertical_shaft`, `square_arena`, and `tiny_chamber`. The boss is intentionally outside the stitched hub/basement area and lives in `basement_boss`.

## Validator

Run the standalone validator with:

```bash
python tools/validate_ambition_ldtk.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

The validator checks Ambition-specific constraints:

- project has LDtk levels,
- levels have an `Ambition` entity layer,
- level origins are grid-aligned,
- entity identifiers are known,
- Ambition entities use top-left pivots,
- entities stay inside their LDtk level bounds,
- each active area has exactly one `PlayerStart`,
- loading zones target valid active areas and destination zones,
- `EdgeExit` loading zones touch a level edge and do not overlap solid wall collision,
- transition arrivals computed from destination zones remain inside the target active area and do not start inside authored solids,
- selected entity types have required custom fields,
- LDtk FieldDef records use editor-openable internal `type` constructors such as `F_String` while keeping human-readable `__type` values such as `String`,
- LDtk-authored moving damage volumes and `KinematicPath` entities have valid point/speed/mode fields.

This validator is not a substitute for LDtk's official JSON schema. It validates Ambition's gameplay-authoring contract.

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
```

`CameraZone` and `StitchedBoundary` are accepted but not yet converted into gameplay behavior.

## Debug overview

`F5` toggles the overview camera. The initial overview simply centers the composed active-area bounds and increases orthographic scale. This is a POC for inspecting large or stitched areas, not the final camera-authoring system.

## Edge exits and collision openings

`EdgeExit` zones are gameplay triggers, not collision cutters. If an edge-exit zone is authored inside a wall, the zone may render/debug-label correctly but remain unreachable to the player. Split the adjacent side wall around the exit opening, and keep the zone touching the level edge without strictly overlapping any `Solid` entity. The validator now treats overlap between an `EdgeExit` and a `Solid` as an error.

For stitched active areas, remember that LDtk level positions are flattened into active-area coordinates before runtime collision and loading-zone checks. Any spatial assumption that depends on seams, wall openings, or transition spawn repair should be marked in Rust with `AMBITION_REVIEW(spatial)`.

## First-class Bevy LDtk asset path

The sandbox now treats `assets/ambition/worlds/sandbox.ldtk` as both:

1. an Ambition-authored gameplay source that is synchronously validated and materialized into runtime `RoomSpec` / `ae::World` data, and
2. a first-class Bevy LDtk asset loaded through `bevy_ecs_ldtk` and spawned with `LdtkWorldBundle`.

At startup, `ldtk_world::load_ldtk_asset_handle` inserts a typed handle for the LDtk project, `SandboxAssetCollection` also includes the LDtk handle, and setup spawns an `LDtk Runtime Spine Root` entity tagged with `SandboxLdtkWorldRoot`. The root uses a `LevelSet` built from the LDtk level iids that belong to the active Ambition active area.

On room transitions, `ldtk_world::sync_ldtk_level_set` updates that `LevelSet` to match the active Ambition room. For stitched spaces such as `central_hub_complex`, this means multiple LDtk levels are selected at once (`central_hub_main` and `central_hub_basement`). For standalone rooms, exactly one LDtk level is selected.

The plugin-owned LDtk world root is no longer hidden. Instead, every Ambition-authored LDtk entity identifier is registered with `bevy_ecs_ldtk` as a lightweight `AmbitionLdtkMarkerBundle`. This prevents the plugin from spawning unregistered placeholder visuals while still making `bevy_ecs_ldtk` responsible for LDtk entity lifecycle, stable identity, transform hierarchy, level selection, and hot reload. A follow-up should promote those marker entities into direct Ambition gameplay components category by category.

The current gameplay collision, loading zones, features, and debug visuals still mostly use the Ambition typed runtime path. This is now transitional: LDtk is the first-class asset/spawn source, and gameplay meaning should move from the adapter into systems that consume plugin-spawned LDtk marker entities and attach typed Ambition components.

The LDtk file should remain editor-shaped, not just parser-shaped. It should include a root `iid`, `worldLayout`, an `Ambition` entity-layer definition in `defs.layers`, and entity definitions in `defs.entities` for every Ambition entity used by instances. The validator checks these first-class requirements in addition to gameplay constraints.

## Runtime spine migration rule

Do not reimplement LDtk parsing, level selection, or entity spawning when `bevy_ecs_ldtk` already provides those lifecycle hooks. Register Ambition entity identifiers with `bevy_ecs_ldtk`, consume plugin-spawned `EntityInstance` components, and attach Ambition semantics from systems. Keep Ambition-specific validation and gameplay rules in Ambition code.

## RON world definition retirement

The sandbox no longer stores the LDtk-derived room manifest inside `SandboxDataSpec.rooms`. RON remains useful for abilities, movement tuning, generated audio, input presets, fixtures, and non-spatial data, but LDtk is the only active source for the sandbox world definition.

The LDtk path now builds the existing `RoomSet`/`GameWorld` runtime directly with `RoomSet::from_parts`. Treat remaining custom JSON parsing as transitional. As `bevy_ecs_ldtk` marker entities are promoted into direct Ambition components, remove matching categories from the custom parser instead of duplicating behavior in both places.

As of the LDtk runtime-spine migration, the old `rooms` block has been removed from `assets/ambition/sandbox.ron`, and the Rust RON-world manifest structs/builders have been deleted. This makes accidental edits to the obsolete RON world definition impossible in the main sandbox manifest and keeps `SandboxDataSpec` focused on non-spatial tuning/audio data.

### Bridge removal status

The old RON-shaped room manifest is no longer part of the sandbox runtime path. Runtime startup and hot reload ask LDtk for a `RoomSet` directly, and `ldtk_world.rs` materializes runtime `RoomSpec`, `ae::World`, loading zones, room objects, and graph links without using RON fixture structs. Remaining migration work should shrink the custom LDtk parser category by category as plugin-spawned `bevy_ecs_ldtk` marker entities become the source for Ambition runtime components.

## LDtk editor round-trip values

Generated or agent-patched LDtk files must populate `fieldInstances[*].realEditorValues` alongside the parser-friendly `__value` field. LDtk game/runtime documentation says game code can ignore `realEditorValues`, but the LDtk editor uses it as its raw editable representation when a level is opened and saved. If non-null `__value` fields have empty `realEditorValues`, moving an entity in the editor can cause the edited level to be saved with null custom fields.

Ambition treats this as an authoring error. The validator rejects non-null field values with empty `realEditorValues`, and `tools/validate_ambition_ldtk.py --normalize-editor-values` can rewrite editor values from existing `__value` data before an editor round-trip. This cannot recover fields after the editor has already saved them as null; use the last known-good LDtk file or a patch that restores those values.
