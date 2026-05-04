# Lessons Learned

Debugging journals for surprises that took serious time to track down.
Ordered newest-first. Each lesson should make the next time you hit the
same class of bug 10× faster — symptom recognition, where to look, and
what the fix looks like in this codebase specifically.

## Wall-cling y-sweep teleports player to wall's far edge

**Date:** 2026-05-04. **Fixed in:** the next commit.

### Symptom

Player wall-clinging on a tall side-wall (e.g. `square_arena`'s left
wall, top at world `y=0`, bottom at the room floor) suddenly teleports
to `(prev_x, -23.0)` — exactly `0 − half_height` — and is reported as
`Grounded`. Subsequent leftward input then walks them off this invisible
ledge above the world, and the OOB detector fires a few frames later.

Two consecutive trace dumps captured the pattern. The post-fix-recorder
trace shows the smoking-gun event:

```
t= 1962  CollisionCorrection :: (62.0, 1678.7) → (62.0, -23.0)
                                [unexplained delta 1701.7px (vel-budget 17.2px)]
t= 1962  PlayerModeChanged    :: WallCling → Grounded
```

### Root cause

`movement::sweep_player_y` was returning a `time_of_impact = 0` swept
hit on the wall block the body was edge-touching / fractionally
penetrating on the X axis. The snap branch then unconditionally pushed
the body's bottom to the wall's TOP edge:

```rust
if delta.y > 0.0 || body.center().y < hit.block.aabb.center().y {
    player.pos.y += hit.block.aabb.top() - body.bottom();
    player.on_ground = true;
}
```

For a wall whose top is at world `y=0` and a body at `y≈1700`, this
push is `0 - 1700 = -1700` — a 1700-px upward teleport.

The symmetric guard already existed in `resolve_axis(Axis::X)` (with a
clear comment), but `sweep_player_y` and `resolve_vertical` were
missing it.

### Fix

Two-part:

1. New helper `dominantly_horizontal_overlap(body, block)` — true when
   the body's existing overlap with `block` is wider on the y axis than
   the x axis. Side-wall contacts have large y-overlap; floor/ceiling
   contacts have large x-overlap.
2. Both `sweep_player_y`'s `first_body_sweep` predicate and
   `resolve_vertical` skip blocks where this returns true. The X-axis
   sweep / resolve owns those.

Plus a regression test (`wall_cling_does_not_teleport_to_wall_top_on_y_sweep`)
that reproduces the exact pose: wall-cling on a tall left wall (top at
y=0) with `wall_slide_speed` downward, sub-pixel penetration into the
wall on x. Pre-fix: player teleports to y≈-23. Post-fix: |dy| < 50 px,
player stays in the world.

### Trace coverage that made the fix takeable

The ad-hoc trace recorder added shortly before this bug (see
`docs/gameplay_trace_recorder.md`) made the diagnosis 10× faster. Two
recorder upgrades from this fix's patch are worth keeping in mind:

- **`nearby_collision` now uses the feature-augmented collision world.**
  The wall the player was clinging to wasn't in `GameWorld.0.blocks`
  (it came from `runtime.features` via `world_with_sandbox_solids`), so
  the trace's nearby-collision view was empty and the wall was
  invisible. The recorder now calls `features::world_with_sandbox_solids`
  the same way `sandbox_update` does.
- **`last_safe_player_pos` is gated by `classify_player_safety`.** The
  pre-fix trace recorded `last_safe_player_pos = (62, -23)` because
  the player was technically `on_ground` after the teleport. The new
  gate refuses to remember any position that the OOB detector would
  reject, and also refuses while the player is taking damage / in
  hitstun / in blink-grace / mid-room-transition.

The shared classifier (`ambition_engine::classify_player_safety`) is
the single source of truth so the trace's OOB detector and the
sandbox's safe-pos gate cannot drift again.

### Takeaway

**A swept hit with `time_of_impact = 0` on an already-overlapping
block is not a landing — it's an existing contact, and the snap
direction has to come from the *shape* of the overlap, not the
direction of `delta`.** When you see an unconditional `pos += block_top
- body.bottom` in collision code, ask: what if the block's top is
hundreds of pixels away from the body? Add the symmetric overlap-shape
guard `resolve_axis(Axis::X)` already had.

---



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
