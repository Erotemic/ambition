# ADR 0015: LDtk tileset rendering as a presentation-only layer

## Status

Proposed.

## Context

Ambition's LDtk authoring rule ([feedback_ldtk_authoring_philosophy]):

```
Tiles    → visuals only
IntGrid  → grid gameplay (collision/hazard/water/ladder)
Entities → distinct identities
```

Today only the IntGrid and Entity sides are wired:

- `intro_lab_tileset.png` and `town_tileset.png` exist on disk under
  `crates/ambition_sandbox/assets/sprites/` but `intro.ldtk` does NOT
  reference them in `defs.tilesets` and has no Tiles layer instance.
- `bevy_ecs_ldtk` is configured with `LevelBackground::Nonexistent`
  and `IntGridRendering::Colorful` overrides in
  `ldtk_world/bevy_runtime/asset.rs`, so even if a tileset were
  referenced, the runtime would skip rendering it.
- Ambition's own renderer (`rendering/world.rs::spawn_block`) draws
  every collision block as a solid colored rectangle. That fights
  with a tileset visual once both are on screen.

Result: the LDtk editor view is colorful debug rectangles instead of
the authored tileset, and the in-game view is colorful debug
rectangles instead of the authored tileset. The whole "tiles are
the visual" half of the authoring philosophy is unwired.

This ADR captures the design of fixing that gap. It is proposed
rather than accepted because the coordinate-frame reconciliation
(see Consequences) needs a runtime spike before committing.

## Decision

LDtk tilesets render as a **presentation-only background layer**
managed by `bevy_ecs_ldtk`. Ambition's own block renderer continues
to render collision blocks (now optionally toggleable for debug),
but the visual identity of each tile comes from the LDtk tileset.

### Authoring

A Tiles layer is added to every intro level. Authors paint tiles in
the LDtk editor; the cell-to-tile mapping can be:

- **Hand-authored** for hero rooms (the cart corner, the lab
  centerpiece, the gate stack).
- **Auto-tile** for repetitive geometry — LDtk auto-tile rules
  watch the Collision IntGrid layer and pick a tile based on the
  cell's IntGrid value (`Solid` → wall tile, `OneWayUp` → platform
  tile, etc.). This means a future room author paints IntGrid and
  the visual tile snaps automatically.

The Collision IntGrid layer remains the source of truth for
gameplay. The Tiles layer is purely visual — if the two disagree,
gameplay wins and authoring tooling flags the mismatch.

### Runtime

`bevy_ecs_ldtk`'s `LdtkWorldBundle` renders the Tiles layer for the
active area. Three settings change:

- `LevelBackground::Nonexistent` → `LevelBackground::Translucent` so
  the level background quad participates in the render order.
- `IntGridRendering::Colorful` → `IntGridRendering::Invisible` for
  layers whose visual is owned by the Tiles layer. (Keep `Colorful`
  for IntGrid layers that have no authored tile mapping yet, so
  the visual debug fallback remains.)
- A new sandbox setting `RenderDebugBlocks: bool` (default false in
  release, true behind `dev` feature) gates the
  `rendering::world::spawn_block` call so dev builds can still see
  the colored-rectangle collision overlay without leaking it into
  shipping builds.

### Coordinate-frame reconciliation

`bevy_ecs_ldtk` renders tiles in **raw LDtk world-pixel space**: the
spawned `LdtkWorldBundle` entity sits at world origin and each level
quad is positioned at `(level.worldX, level.worldY)`.

Ambition's own renderer centers each active area at the origin via
`world_to_bevy(world, pos, z)` — the camera + every Ambition-spawned
visual is in this active-area-local frame.

The two frames disagree by the active-area's `(min_x, min_y)`
offset. The fix is a per-room transform on the LdtkWorldBundle:

```rust
ldtk_world_transform.translation =
    world_to_bevy_origin(active_area_min, WORLD_Z_BLOCK - 1.0);
```

so the LdtkWorldBundle renders behind Ambition's blocks but shares
the active-area coordinate system. The seam point is documented in
the comment block at `ldtk_world/bevy_runtime/asset.rs` — this ADR
makes that comment concrete.

When the active area changes, the LdtkWorldBundle's `LevelSet`
already swaps to the new level. The transform update piggybacks on
the same room-transition event (`RoomTransitionRequested` →
`apply_room_transition_system`) so the visual swap and the
coordinate-frame swap happen on the same frame.

### Tooling

Adding a tileset by hand to an LDtk JSON file is the same class of
editor-roundtrip pain that `def register-entity` solved. Mirror that
pattern:

```bash
python -m ambition_ldtk_tools tileset add <ldtk> <png> <grid_size> --in-place
```

The tool:

1. Allocates a fresh `uid` for `defs.tilesets[]`.
2. Computes columns/rows from the PNG dimensions.
3. Registers `relPath`, `pxWid`, `pxHei`, `tileGridSize`, default
   tag list, embedded color metadata.
4. Reuses the same `repair --in-place` + schema-validate post-pass
   as the other authoring tools.

A follow-on subcommand `tileset add-layer` (or `area add-layer`)
attaches a Tiles layer instance to a specific level, referencing
the registered tileset by uid.

## Consequences

- The Tiles layer is the visual; the IntGrid Collision layer is the
  gameplay. Two layers per room instead of one. Authoring discipline
  shifts: paint IntGrid first, then either auto-tile rules or
  hand-painted tiles fill the visual.
- The colored-rectangle debug overlay survives behind a feature
  gate, so dev / spatial-review sessions still see collision shapes
  at a glance.
- The coordinate-frame reconciliation creates one new place where
  LDtk world coords and Ambition active-area coords interact. Treat
  that seam with `AMBITION_REVIEW(spatial)` markers per
  [feedback_spatial_review].
- The intro slice immediately benefits: the lab, the raid corridor,
  the drain alley, the gate stack all get a visual identity beyond
  colored blocks. The sandbox hub levels can opt-in incrementally
  as authoring time allows.
- This ADR does NOT couple to ADR 0016 (Actor unification, also
  proposed). Tileset rendering and Actor unification are
  independent; pick whichever the next agent has appetite for
  first.

## Initial implementation target

1. Write `tileset add` python subcommand (additive to
   `ambition_ldtk_tools`; ~150 LOC).
2. Register `intro_lab_tileset.png` against `intro.ldtk`'s
   `defs.tilesets[]`. No level changes yet.
3. Add a Tiles layer instance to `intro_wake_room` only (smallest
   level; easiest to roundtrip). Hand-paint 4-6 tiles to verify
   the editor view.
4. Add the `RenderDebugBlocks` sandbox setting + gate
   `spawn_block`. Default true in dev, false in release.
5. Implement the per-room LdtkWorldBundle transform; verify the
   tiles appear in-game at the expected screen positions.
6. Add auto-tile rules: `Solid` → wall tile, `OneWayUp` → platform
   tile. Verify drain alley + gate stack pick up tiles without
   per-cell hand-painting.
7. Extend tooling: `tileset add-layer` + add layers to
   `intro_raid_corridor`, `drain_alley`, `gate_stack_lower`.

Step 5 is the spike that decides whether this approach is sound;
the earlier steps are safe and can be reverted by deleting the
tileset registration.

## Alternatives considered

**Ambition draws the tiles itself.** Reading the LDtk tile map at
load time and emitting one Bevy quad per tile from
`spawn_room_visuals`. Rejected: re-implements `bevy_ecs_ldtk`'s
work and forfeits its auto-tile rule engine. The whole point of the
`bevy_ecs_ldtk` dependency (ADR 0009) is that it owns the LDtk
tile-rendering path.

**Keep colored rectangles.** They're functional and fast to author.
Rejected: the design memory's "Tiles=visuals, IntGrid=grid
gameplay" philosophy and the user's explicit ask both call for the
tilesets to render. The intro slice's visual identity is currently
"colorful debug rectangles," which is not the shipping target.

[feedback_ldtk_authoring_philosophy]: see auto-memory entry of the
  same name in `~/.claude/projects/-home-joncrall-code-ambition/memory/`.
[feedback_spatial_review]: same source.
