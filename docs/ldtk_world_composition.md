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
- selected entity types have required custom fields,
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
