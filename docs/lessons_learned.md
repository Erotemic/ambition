# Lessons Learned

Debugging journals for surprises that took serious time to track down.
Ordered newest-first. Each lesson should make the next time you hit the
same class of bug 10× faster — symptom recognition, where to look, and
what the fix looks like in this codebase specifically.

## bevy_ecs_ldtk renders IntGrid cells by default, even with no tileset

**Date:** 2026-05-04. **Fixed in:** `ded1dc2`.

### Symptom

Geometry painted in LDtk's IntGrid layer (the `Collision` layer for
`central_hub_main`) appeared **duplicated** in the running game: the
"real" merged block at the correct position, plus what looked like
copies of the leftmost cell pattern repeating horizontally across the
level. The duplicates rendered in the same colors as the IntGrid value
defs (gray for Solid, light blue for OneWay, light purple for
BlinkSoft). Entities (NPCs, doors, loading zones) were *not* duplicated.

### Root cause

`bevy_ecs_ldtk-0.14.0/src/level.rs:557-595`. When an IntGrid layer has
**no tileset configured** (which ours doesn't), the plugin's default
`IntGridRendering::Colorful` mode spawns **a colored tile sprite per
non-zero cell**, using the color from `intGridValues[i].color`. For
`central_hub_main` that was 762 + 152 + 90 = 1004 plugin-owned sprites.

`LdtkWorldBundle` is spawned at the default transform (origin (0,0)),
so the plugin's tilemap renders in **raw LDtk world-pixel space**
(top-left origin, +y down). Our `compose_runtime_area` →
`int_grid_value_to_block` → `spawn_block` path renders in Ambition's
**centered Bevy frame** (`world_to_bevy`: `x = p.x - world.size.x*0.5`).
The two frames disagree by ~half-room-width on x — exactly the visible
horizontal offset.

Entities were unaffected because our `AmbitionLdtkMarkerBundle`
intentionally doesn't include a `Sprite` component, so the
plugin-spawned entity instances are invisible markers only.

### Fix

```rust
// crates/ambition_sandbox/src/app.rs
.insert_resource(LdtkSettings {
    level_background: LevelBackground::Nonexistent,
    int_grid_rendering: IntGridRendering::Invisible,  // <-- this
    ..default()
})
```

`Invisible` mode still spawns the `IntGridCell` components the runtime-
spine indexer uses (so `LdtkSolid` still works); it just suppresses the
plugin's per-cell sprite. Our `spawn_block` is now the only thing
rendering IntGrid visuals.

### Takeaway

**Audit every Bevy plugin's defaults before assuming our compose path is
the only render path.** The plugin had a sensible default for *its*
intended use case (paint-and-go IntGrid editor preview), which silently
double-rendered when we used IntGrid as a data layer with our own
visualization. The diagnostic that finally cracked it was reading the
plugin's source directly — neither cargo logs nor the data dump showed
extra sprites in *our* world.blocks, because they weren't ours. Any
future "data is correct, but something is rendering it wrong" symptom
should immediately suspect a plugin's default render path.

GPT-review's coordinate-translation hypothesis (in `docs/gpt-review.md`)
was on the right scent — same coordinate-frame mismatch — but proposed
translating the plugin's root to overlap our render. That would have
worked visually but kept ~1000 redundant sprites under our blocks.
`Invisible` kills the redundant render entirely.

---

## LDtk computes cWid as `ceil(pxWid / gridSize)`, not floor

**Date:** 2026-05-04. **Fixed in:** `56acf3b`.

### Symptom

Migration script (`tools/ldtk_intgrid_migration.py`) painted clean
rectangular IntGrid cells (verified by Python dump). After the user
opened the file in LDtk, every column of cells was **smeared into a
1-cell-per-row staircase** going left-down. LDtk re-saved the smeared
state on Ctrl+S, locking it in.

### Root cause

The migration set `__cWid = pxWid // GRID` (floor division). LDtk
expects `cWid = ceil(pxWid / gridSize)`. For `central_hub_main`
(1900×1024, GRID 16): floor → 118, LDtk → 119. The migration wrote a
7552-element `intGridCsv`. LDtk loaded it and read with **stride 119**
(its expected cWid) instead of 118 (the script's), so column N at row
M moved by `M / 118 * (119 - 118) = M` cells per row — pure stride
slip, exactly diagonal.

### Fix

```python
def cells_for_size(px: int) -> int:
    return (px + GRID - 1) // GRID   # ceil
```

Plus rerun migration from the pre-IntGrid baseline (the smeared file
was already canonicalised by LDtk; you can't fix it by changing the
reader, you have to repaint).

### Takeaway

**When interoperating with an editor that owns the canonical file
format, cross-check at minimum *one* derived field (here:
`__cWid * __cHei == len(intGridCsv)`) against what the editor
actually emits**, not just what the JSON schema says is allowed. The
schema would have accepted either floor or ceil; the editor's behavior
distinguished them.

---

## Greedy row-major rect-merge produces vertical bars on diagonals

**Date:** 2026-05-04. **Fixed in:** `8332349` (replaced earlier
`1739312`).

### Symptom

After landing the IntGrid migration, painted staircase / diagonal cell
patterns in the editor rendered in-game as **stacks of tall thin
vertical bars** instead of stair-stepped tiles.

### Root cause

The first-pass `emit_collision_blocks_from_intgrid` was greedy
row-major with vertical extension: for each unconsumed non-zero cell,
extend right, then extend the resulting rectangle down as long as
every column matched. On a staircase pattern:

```
......#   row 0  (start: width-1 run at col 6)
.....##   row 1  (col 6 still matches → extend down)
....###
...####
..#####
```

The first iteration finds a 1-wide run on row 0 at the rightmost
column, then walks down — every row has that column filled, so the
merge produces a 1×N vertical bar. Each subsequent diagonal step
becomes another 1×(N-k) bar. The staircase visually inverts into a
column of vertical strips.

### Fix

Two-pass merge:
1. **Per-row horizontal coalesce** — collapse adjacent same-value cells
   in each row into runs.
2. **Per-column vertical span-stack** — adjacent rows that produced
   the *same* `[cx, x_end)` span and value get stacked.

Vertical walls of N-wide cells stack into one N×H block. Horizontal
floors are one row from pass 1. Staircases produce per-row runs that
*can't* stack (varying widths), so they stay as the cell mosaic the
editor shows.

### Takeaway

**Greedy rectangle merging biases toward whatever direction it extends
first.** The fix isn't a smarter greedy choice — it's two passes with
strict matching on the second one. Worst-case is still per-cell on
truly irregular shapes; that's the right outcome (faithful to author
intent).

---

## How to add to this file

When you fix a bug that took >1 hour to diagnose, ask yourself: **could
the lesson save the next person time, or is the fix obvious from the
diff?** If the *diagnosis* was the hard part, write it up here.

Template:

```
## One-line title of the lesson

**Date:** YYYY-MM-DD. **Fixed in:** `<commit hash>`.

### Symptom
What the bug looked like from outside.

### Root cause
What was actually wrong, including any non-obvious upstream code or
plugin-default that contributed.

### Fix
The minimum change. Code snippets if short.

### Takeaway
The general rule the next person should pattern-match against.
```

Skip the pretty narrative. The point is grep-ability — somebody
hunting for "duplicate sprite" or "staircase smear" should land on
the right entry.
