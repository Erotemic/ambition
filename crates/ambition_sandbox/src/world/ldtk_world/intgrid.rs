use crate::engine_core as ae;

use super::LdtkLayerInstance;

pub(super) const AMBITION_LAYER: &str = "Ambition";
pub(super) const COLLISION_LAYER: &str = "Collision";
pub(super) const WATER_LAYER: &str = "Water";
pub(super) const CLIMBABLE_LAYER: &str = "Climbable";
pub(super) const GRID: i32 = 16;

/// IntGrid Water layer values. Distinct from Collision values because
/// they live on a separate layer (see `WATER_LAYER`).
pub(super) const WATER_INT_GRID_CLEAR: i32 = 1;
pub(super) const WATER_INT_GRID_MURKY: i32 = 2;

/// IntGrid Climbable layer values. Same separation rationale as
/// Water: a dedicated layer keeps ladders / vines / climbable walls
/// from sharing IntGrid value space with collision-affecting cells.
/// Authors paint these on the `Climbable` layer; the runtime lowers
/// each cell run into a `ClimbableRegion` of the matching `kind`.
pub(super) const CLIMBABLE_INT_GRID_LADDER: i32 = 1;
pub(super) const CLIMBABLE_INT_GRID_VINE: i32 = 2;
pub(super) const CLIMBABLE_INT_GRID_WALL: i32 = 3;

// IntGrid value → engine block kind. Mirrors `tools/ldtk_intgrid_migration.py`;
// the migration script is the source of truth for which value means what, but
// any new value here that isn't covered there will fail validation at compose
// time so authors can't silently introduce mismatched mappings.
pub(super) const INT_GRID_SOLID: i32 = 1;
pub(super) const INT_GRID_ONE_WAY: i32 = 2;
pub(super) const INT_GRID_BLINK_SOFT: i32 = 3;
pub(super) const INT_GRID_BLINK_HARD: i32 = 4;
pub(super) const INT_GRID_HAZARD: i32 = 5;

pub(super) fn int_grid_value_to_block(
    value: i32,
    min: ae::Vec2,
    size: ae::Vec2,
) -> Result<ae::Block, String> {
    match value {
        INT_GRID_SOLID => Ok(ae::Block::solid("ldtk solid", min, size)),
        INT_GRID_ONE_WAY => Ok(ae::Block::one_way("ldtk one-way", min, size)),
        INT_GRID_BLINK_SOFT => Ok(ae::Block::blink_wall(
            "ldtk blink-soft",
            min,
            size,
            ae::BlinkWallTier::Soft,
        )),
        INT_GRID_BLINK_HARD => Ok(ae::Block::blink_wall(
            "ldtk blink-hard",
            min,
            size,
            ae::BlinkWallTier::Hard,
        )),
        // Hazard tile: damages the player on contact. Static-only —
        // moving / per-volume-tuned hazards stay on the
        // `RoomObjectKind::DamageVolume` entity path because IntGrid
        // can't carry per-cell motion paths or damage amounts.
        INT_GRID_HAZARD => Ok(ae::Block::hazard("ldtk hazard", min, size)),
        other => Err(format!("unknown IntGrid value {other}")),
    }
}

/// Two-pass rectangle merge over the IntGrid:
///   1. Per-row horizontal coalesce: each row collapses adjacent
///      same-value cells into a single run.
///   2. Per-column vertical merge: adjacent rows that produced the
///      *exact same span* (same x extent, same value) are stacked into
///      one taller block.
///
/// This correctly handles:
///   - Long horizontal floors (pass 1 merges them; pass 2 finds nothing
///     more to do) → one block. Floor-walk friction fix preserved.
///   - Vertical walls of N-cell-wide cells stacked vertically (pass 1
///     produces N identical 1-tall blocks; pass 2 stacks them into one
///     N×H block) → one block. Wall-slide grinding fix.
///   - Staircase / diagonal patterns: pass 1 produces blocks of varying
///     widths per row (1, 2, 3, …); pass 2 finds no two adjacent rows
///     with the same span so nothing merges. Staircases stay per-row
///     visually (matches the editor's rendering). Regression fix from
///     the earlier greedy-row-major bug.
///
/// Invariant: every cell ends up covered by exactly one rectangle.
pub(super) fn merge_intgrid_rects(
    layer: &LdtkLayerInstance,
    offset: ae::Vec2,
) -> Result<Vec<(i32, ae::Vec2, ae::Vec2)>, String> {
    let cw = layer.c_wid;
    let ch = layer.c_hei;
    let grid = layer.grid_size as f32;
    if cw <= 0 || ch <= 0 || layer.int_grid_csv.is_empty() {
        return Ok(Vec::new());
    }
    let expected = (cw as usize) * (ch as usize);
    if layer.int_grid_csv.len() != expected {
        return Err(format!(
            "intGridCsv length {} does not match cWid*cHei = {}*{} = {expected}",
            layer.int_grid_csv.len(),
            cw,
            ch
        ));
    }
    let cells = &layer.int_grid_csv;
    let cw_usize = cw as usize;
    let ch_usize = ch as usize;

    // Pass 1: produce per-row runs as (cx, x_end, cy, value).
    let mut runs: Vec<(usize, usize, usize, i32)> = Vec::new();
    for cy in 0..ch_usize {
        let mut cx = 0;
        while cx < cw_usize {
            let value = cells[cy * cw_usize + cx];
            if value == 0 {
                cx += 1;
                continue;
            }
            let mut x_end = cx + 1;
            while x_end < cw_usize && cells[cy * cw_usize + x_end] == value {
                x_end += 1;
            }
            runs.push((cx, x_end, cy, value));
            cx = x_end;
        }
    }

    // Pass 2: stack runs vertically when the next-row run has the same
    // [cx, x_end) span and value.
    let mut consumed = vec![false; runs.len()];
    let mut by_row_cx: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::with_capacity(runs.len());
    for (i, &(cx, _, cy, _)) in runs.iter().enumerate() {
        by_row_cx.insert((cy, cx), i);
    }

    let mut rects = Vec::new();
    for i in 0..runs.len() {
        if consumed[i] {
            continue;
        }
        let (cx, x_end, cy, value) = runs[i];
        let mut y_end = cy + 1;
        while y_end < ch_usize {
            let Some(&next_idx) = by_row_cx.get(&(y_end, cx)) else {
                break;
            };
            let (n_cx, n_x_end, _, n_value) = runs[next_idx];
            if consumed[next_idx] || n_cx != cx || n_x_end != x_end || n_value != value {
                break;
            }
            consumed[next_idx] = true;
            y_end += 1;
        }
        let min = ae::Vec2::new(cx as f32 * grid, cy as f32 * grid) + offset;
        let size = ae::Vec2::new((x_end - cx) as f32 * grid, (y_end - cy) as f32 * grid);
        rects.push((value, min, size));
    }
    Ok(rects)
}

pub(super) fn emit_collision_blocks_from_intgrid(
    layer: &LdtkLayerInstance,
    offset: ae::Vec2,
) -> Result<Vec<ae::Block>, String> {
    let rects = merge_intgrid_rects(layer, offset)?;
    let mut blocks = Vec::with_capacity(rects.len());
    for (value, min, size) in rects {
        let block = int_grid_value_to_block(value, min, size)
            .map_err(|message| format!("rect value={value} {size:?}: {message}"))?;
        blocks.push(block);
    }
    Ok(blocks)
}

/// Lower a Water IntGrid layer to source-agnostic `WaterRegion`
/// rectangles. Cells with value 1 emit `WaterKind::Clear`; value 2
/// emits `WaterKind::Murky`. Per-region tuning falls back to
/// `WaterVolumeSpec::default()`; per-volume tuning is the entity
/// path's job (rare, irregular pools).
pub(super) fn emit_water_regions_from_intgrid(
    layer: &LdtkLayerInstance,
    offset: ae::Vec2,
) -> Result<Vec<ae::WaterRegion>, String> {
    let rects = merge_intgrid_rects(layer, offset)?;
    let mut regions = Vec::with_capacity(rects.len());
    for (value, min, size) in rects {
        let kind = match value {
            WATER_INT_GRID_CLEAR => ae::WaterKind::Clear,
            WATER_INT_GRID_MURKY => ae::WaterKind::Murky,
            other => return Err(format!("unknown Water IntGrid value {other}")),
        };
        regions.push(ae::WaterRegion::new(
            ae::aabb_from_min_size(min, size),
            kind,
            ae::WaterVolumeSpec::default(),
        ));
    }
    Ok(regions)
}

/// Lower a Climbable IntGrid layer to source-agnostic
/// `ClimbableRegion` rectangles. Mirrors `emit_water_regions_from_intgrid`.
/// Cells with value 1 → Ladder, 2 → Vine, 3 → Wall. Per-region tuning
/// falls back to `ClimbableSpec::default()` (180 px/sec climb_speed,
/// 0.25 strafe_factor); future LDtk fields could surface per-region
/// overrides if a particular ladder needs to feel faster/slower.
pub(super) fn emit_climbable_regions_from_intgrid(
    layer: &LdtkLayerInstance,
    offset: ae::Vec2,
) -> Result<Vec<ae::ClimbableRegion>, String> {
    let rects = merge_intgrid_rects(layer, offset)?;
    let mut regions = Vec::with_capacity(rects.len());
    for (value, min, size) in rects {
        let kind = match value {
            CLIMBABLE_INT_GRID_LADDER => ae::ClimbableKind::Ladder,
            CLIMBABLE_INT_GRID_VINE => ae::ClimbableKind::Vine,
            CLIMBABLE_INT_GRID_WALL => ae::ClimbableKind::Wall,
            other => return Err(format!("unknown Climbable IntGrid value {other}")),
        };
        regions.push(ae::ClimbableRegion::new(
            ae::aabb_from_min_size(min, size),
            kind,
            ae::ClimbableSpec::default(),
        ));
    }
    Ok(regions)
}
