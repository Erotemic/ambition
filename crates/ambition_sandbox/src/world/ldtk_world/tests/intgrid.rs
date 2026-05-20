//! IntGrid rect-merge, Climbable IntGrid parsing, and the
//! promoted-runtime-role indexes (`LdtkRuntimeSpineIndex`,
//! `LdtkRuntimeSolidIndex`).

use ambition_engine as ae;

use super::super::intgrid::*;
use super::super::project::*;
use super::super::*;

/// IntGrid value 5 (Hazard) must round-trip through the
/// `int_grid_value_to_block` mapping into a `BlockKind::Hazard`
/// block. Pinning the conversion so a future renumbering can't
/// silently drop hazard cells from the runtime collision world.
#[test]
fn int_grid_hazard_value_maps_to_hazard_block() {
    let block = int_grid_value_to_block(5, ae::Vec2::ZERO, ae::Vec2::new(16.0, 16.0))
        .expect("value 5 must map to a block");
    assert!(matches!(block.kind, ae::BlockKind::Hazard));
    assert_eq!(block.name, "ldtk hazard");
}

#[test]
fn intgrid_rect_merge_collapses_a_horizontal_run() {
    // 5x1 row of value=1 cells should produce a single 5*16-wide block.
    let layer = LdtkLayerInstance {
        identifier: "Collision".to_string(),
        layer_type: "IntGrid".to_string(),
        c_wid: 5,
        c_hei: 1,
        grid_size: 16,
        entity_instances: Vec::new(),
        int_grid_csv: vec![1; 5],
        grid_tiles: Vec::new(),
    };
    let blocks =
        emit_collision_blocks_from_intgrid(&layer, ae::Vec2::ZERO).expect("merge succeeds");
    assert_eq!(blocks.len(), 1, "horizontal run should merge to one block");
    let block = &blocks[0];
    assert!(matches!(block.kind, ae::BlockKind::Solid));
    let size = ae::AabbExt::half_size(block.aabb) * 2.0;
    assert!(
        (size.x - 80.0).abs() < 0.001,
        "merged width = 5 cells * 16px"
    );
    assert!((size.y - 16.0).abs() < 0.001, "merged height = 1 cell");
}

#[test]
fn intgrid_rect_merge_does_not_collapse_columns_into_vertical_bars() {
    // A staircase pattern is the regression case: greedy vertical
    // merge previously collapsed each diagonal step into a tall
    // 1-wide bar, which rendered as vertical walls instead of the
    // staircase the editor shows. Horizontal-only merge keeps each
    // cell's row the way the artist painted it — so a 3-step
    // staircase produces 6 blocks (1 + 2 + 3 cells across), one per
    // run, not three vertical bars.
    let layer = LdtkLayerInstance {
        identifier: "Collision".to_string(),
        layer_type: "IntGrid".to_string(),
        c_wid: 3,
        c_hei: 3,
        grid_size: 16,
        entity_instances: Vec::new(),
        int_grid_csv: vec![
            0, 0, 1, // row 0
            0, 1, 1, // row 1
            1, 1, 1, // row 2
        ],
        grid_tiles: Vec::new(),
    };
    let blocks =
        emit_collision_blocks_from_intgrid(&layer, ae::Vec2::ZERO).expect("merge succeeds");
    assert_eq!(
        blocks.len(),
        3,
        "staircase should produce one block per row, not collapsed verticals"
    );
    let widths: Vec<i32> = blocks
        .iter()
        .map(|b| (ae::AabbExt::half_size(b.aabb).x * 2.0 / 16.0).round() as i32)
        .collect();
    assert_eq!(widths, vec![1, 2, 3]);
}

#[test]
fn intgrid_rect_merge_separates_distinct_values() {
    // Row [Solid, Solid, OneWay, Solid] should produce 3 blocks: a
    // 2-cell solid, a 1-cell one-way, and a 1-cell solid.
    let layer = LdtkLayerInstance {
        identifier: "Collision".to_string(),
        layer_type: "IntGrid".to_string(),
        c_wid: 4,
        c_hei: 1,
        grid_size: 16,
        entity_instances: Vec::new(),
        int_grid_csv: vec![
            INT_GRID_SOLID,
            INT_GRID_SOLID,
            INT_GRID_ONE_WAY,
            INT_GRID_SOLID,
        ],
        grid_tiles: Vec::new(),
    };
    let blocks =
        emit_collision_blocks_from_intgrid(&layer, ae::Vec2::ZERO).expect("merge succeeds");
    assert_eq!(blocks.len(), 3);
    assert!(matches!(blocks[0].kind, ae::BlockKind::Solid));
    assert!(matches!(blocks[1].kind, ae::BlockKind::OneWay));
    assert!(matches!(blocks[2].kind, ae::BlockKind::Solid));
}

#[test]
fn solid_is_a_promoted_runtime_role() {
    let role = LdtkRuntimeRole::from_identifier("Solid");
    assert_eq!(role, LdtkRuntimeRole::Solid);
    assert!(role.promoted(), "Solid is a Step 1 promoted runtime role");
    let summary = LdtkRuntimeSpineIndex::default().promoted_summary();
    assert!(
        summary.contains("solids"),
        "promoted summary surfaces solid count: {summary}"
    );
}

#[test]
fn solid_index_replaces_only_when_changed() {
    let mut index = LdtkRuntimeSolidIndex::default();
    let solid_a = LdtkRuntimeSolid {
        iid: "solid-a".to_string(),
        min: ae::Vec2::ZERO,
        size: ae::Vec2::new(64.0, 16.0),
    };
    let solid_b = LdtkRuntimeSolid {
        iid: "solid-b".to_string(),
        min: ae::Vec2::new(64.0, 0.0),
        size: ae::Vec2::new(64.0, 16.0),
    };
    index.replace_if_changed(LdtkRuntimeSolidIndex {
        active_area: "central_hub_complex".to_string(),
        solids: vec![solid_b.clone(), solid_a.clone()],
        revision: 0,
    });
    assert_eq!(index.count(), 2);
    assert_eq!(
        index.solids[0].iid, "solid-a",
        "solids are sorted by iid for stable diffs"
    );
    assert_eq!(index.revision, 1);

    let before = index.revision;
    index.replace_if_changed(LdtkRuntimeSolidIndex {
        active_area: "central_hub_complex".to_string(),
        solids: vec![solid_a, solid_b],
        revision: index.revision,
    });
    assert_eq!(
        index.revision, before,
        "no-op replace must not bump revision"
    );
}

#[test]
fn climbable_intgrid_emits_ladder_region_for_value_one() {
    // 4x3 layer, single column of ladder cells in the middle.
    // CSV is row-major: row0 row1 row2.
    let csv = vec![
        0, 0, 1, 0, // row 0
        0, 0, 1, 0, // row 1
        0, 0, 1, 0, // row 2
    ];
    let layer = super::intgrid_layer(CLIMBABLE_LAYER, 4, 3, csv);
    let regions = emit_climbable_regions_from_intgrid(&layer, ae::Vec2::ZERO).unwrap();
    assert_eq!(regions.len(), 1, "ladder column should merge to one region");
    assert_eq!(regions[0].kind, ae::ClimbableKind::Ladder);
    // Cell (cx=2, cy=0..2). With GRID=16, x in [32, 48], y in [0, 48].
    assert_eq!(regions[0].aabb.min.x, 32.0);
    assert_eq!(regions[0].aabb.min.y, 0.0);
    assert_eq!(regions[0].aabb.max.x, 48.0);
    assert_eq!(regions[0].aabb.max.y, 48.0);
}

#[test]
fn climbable_intgrid_distinguishes_ladder_vine_wall() {
    let layer = super::intgrid_layer(
        CLIMBABLE_LAYER,
        3,
        1,
        vec![
            CLIMBABLE_INT_GRID_LADDER,
            CLIMBABLE_INT_GRID_VINE,
            CLIMBABLE_INT_GRID_WALL,
        ],
    );
    let regions = emit_climbable_regions_from_intgrid(&layer, ae::Vec2::ZERO).unwrap();
    assert_eq!(regions.len(), 3);
    // Sort by min.x for deterministic comparison; merge_intgrid_rects
    // emits in row-major order, so regions[0] is leftmost.
    assert_eq!(regions[0].kind, ae::ClimbableKind::Ladder);
    assert_eq!(regions[1].kind, ae::ClimbableKind::Vine);
    assert_eq!(regions[2].kind, ae::ClimbableKind::Wall);
}

#[test]
fn climbable_intgrid_rejects_unknown_value() {
    let layer = super::intgrid_layer(CLIMBABLE_LAYER, 1, 1, vec![99]);
    let err = emit_climbable_regions_from_intgrid(&layer, ae::Vec2::ZERO)
        .expect_err("unknown value should error");
    assert!(
        err.contains("unknown Climbable IntGrid value 99"),
        "expected error to mention the bad value, got: {err}"
    );
}

#[test]
fn climbable_intgrid_returns_empty_for_all_zero_layer() {
    let layer = super::intgrid_layer(CLIMBABLE_LAYER, 4, 4, vec![0; 16]);
    let regions = emit_climbable_regions_from_intgrid(&layer, ae::Vec2::ZERO).unwrap();
    assert!(regions.is_empty());
}
