# Headless room verification

How to check a room's spatial layout without launching the visible
game — useful for reviewing authoring/collision changes in CI or on a
headless box. All of this runs on a pure-Rust pixel buffer / data scan;
no GPU, wgpu, or window.

## Render a room to a PNG

```bash
cargo run -p ambition_actors --example render_room_geometry -- <ROOM_ID> [OUT.png]
```

Draws the lowered `RoomSpec`: filled collision blocks (gray Solid, blue
OneWay, red Hazard, gold PogoOrb, purple BlinkWall, teal Rebound, tan
moving-platform), outlined entity families (enemy/boss/NPC-switch/
pickup/chest/breakable/hazard-volume/door), kinematic-path polylines +
waypoints, camera-zone outlines, the spawn cross, and each boss's live
rest-pose `damageable_volumes` hurtbox. Read the PNG to verify room
boundaries, mid-air doors, encounter/boss placement, and
hurtbox-vs-spawn-box alignment.

- No `ROOM_ID` → lists every room id.
- `-- all [DIR]` → renders every room into `DIR/room_<id>.png`.

## Scan all rooms for spatial anomalies

```bash
cargo run -p ambition_actors --example render_room_geometry -- report
```

Text-only. Flags runtime-projection bugs the LDtk validator can't see
(it validates LDtk-level data, not the lowered `RoomSpec`): authored
entity centers outside the room bounds, and player spawns embedded in a
Solid block.

## CI guard

The same checks run as a build-failing test so authoring regressions
are caught automatically:

```bash
cargo test -p ambition_actors --test room_spatial_integrity
```

## When to reach for this vs the LDtk validator

- `python -m ambition_ldtk_tools ... validate` — LDtk-level data: entity
  names, IntGrid meaning, mid-air doors, cross-world transition targets.
- These tools — the **runtime projection** (`RoomSpec` after IntGrid
  lowering): where collision/entities/paths actually land, and the
  spatial integrity of the spawn + entities.

Use both; they cover different layers.
