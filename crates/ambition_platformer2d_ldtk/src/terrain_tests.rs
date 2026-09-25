use super::*;

const FULL: TerrainCell = TerrainCell::Full;

/// Paint rows of a small map: `#` full, `/` Up45, `\` Down45, `.` empty.
fn paint(rows: &[&str]) -> Vec<(P, TerrainCell)> {
    let mut cells = Vec::new();
    for (cy, row) in rows.iter().enumerate() {
        for (cx, ch) in row.chars().enumerate() {
            let cell = match ch {
                '#' => FULL,
                '/' => terrain_cell(2).unwrap(),
                '\\' => terrain_cell(3).unwrap(),
                'r' => terrain_cell(16).unwrap(), // CeilDown45
                _ => continue,
            };
            cells.push(((cx as i32, cy as i32), cell));
        }
    }
    cells
}

fn layer(rows: &[&str]) -> LdtkLayerInstance {
    let c_wid = rows[0].len() as i32;
    let mut csv = Vec::new();
    for row in rows {
        for ch in row.chars() {
            csv.push(match ch {
                '#' => 1,
                '/' => 2,
                '\\' => 3,
                _ => 0,
            });
        }
    }
    LdtkLayerInstance {
        identifier: TERRAIN_LAYER.to_string(),
        layer_type: "IntGrid".to_string(),
        c_wid,
        c_hei: rows.len() as i32,
        grid_size: 16,
        entity_instances: Vec::new(),
        int_grid_csv: csv,
        grid_tiles: Vec::new(),
    }
}

#[test]
fn the_palette_values_are_distinct_and_every_slope_spans_a_real_rise() {
    let values: BTreeSet<i32> = TERRAIN_PALETTE.iter().map(|(v, _, _)| *v).collect();
    assert_eq!(values.len(), TERRAIN_PALETTE.len());
    for (value, name, cell) in TERRAIN_PALETTE {
        if let TerrainCell::Floor { left, right } | TerrainCell::Ceiling { left, right } = cell {
            assert_ne!(left, right, "{name} ({value}) is a slope with no rise");
            assert!((0..=Q).contains(&left) && (0..=Q).contains(&right), "{name}");
        }
    }
}

#[test]
fn one_cell_is_one_closed_loop_wound_with_its_normal_outward() {
    let pieces = trace(&paint(&["#"]));
    assert_eq!(pieces, vec![Piece { points: vec![(0, 0), (4, 0), (4, 4), (0, 4)], closed: true }]);
    // The top runs left → right: `(t.y, -t.x)` = (0, -1), up and out of the earth.
    let chain = ae::SurfaceChain::closed_loop("c", vec![
        ae::Vec2::new(0.0, 0.0),
        ae::Vec2::new(16.0, 0.0),
        ae::Vec2::new(16.0, 16.0),
        ae::Vec2::new(0.0, 16.0),
    ]);
    assert!(chain.frame_at(8.0).normal.y < 0.0);
}

#[test]
fn a_painted_run_is_one_segment_per_face_not_one_per_cell() {
    let pieces = trace(&paint(&["##########", "##########"]));
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].points, vec![(0, 0), (40, 0), (40, 8), (0, 8)]);
}

#[test]
fn a_slope_joins_its_flats_in_one_chain() {
    // Flat, a two-cell 45° rise (one `/` per row), a higher flat. The dip at
    // the slope's foot is a 45° concave joint — ridden, not split — and the
    // two slope cells are one straight segment.
    let pieces = trace(&paint(&[".../##", "../###", "######"]));
    assert_eq!(
        pieces,
        vec![Piece {
            points: vec![(16, 0), (24, 0), (24, 12), (0, 12), (0, 8), (8, 8)],
            closed: true,
        }]
    );
}

#[test]
fn the_foot_of_a_wall_splits_the_outline_so_a_runner_hits_it() {
    // A one-cell step: the floor meets a vertical face at a 90° concave corner.
    let pieces = trace(&paint(&["..##", "####"]));
    assert_eq!(pieces.len(), 1, "one foot, one open chain: {pieces:?}");
    let piece = &pieces[0];
    assert!(!piece.closed);
    assert_eq!(piece.points.first(), Some(&(8, 4)), "it starts at the foot, up the wall");
    assert_eq!(piece.points.get(1), Some(&(8, 0)));
    assert_eq!(piece.points.last(), Some(&(8, 4)), "and ends at the foot, off the floor");
}

#[test]
fn a_crest_is_convex_and_stays_joined() {
    // A plateau with a cliff: the top corner is convex, so one closed chain
    // carries the runner over it (the engine's launch rule decides the rest).
    let pieces = trace(&paint(&["##..", "####"]));
    assert_eq!(pieces.len(), 1);
    let points = &pieces[0].points;
    assert!(!pieces[0].closed, "the cliff's own foot, below it, splits it");
    assert_eq!(points.first(), Some(&(8, 4)));
    let crest = points.iter().position(|&p| p == (8, 0)).expect("the crest");
    assert!(0 < crest && crest < points.len() - 1, "the crest is ridden over: {points:?}");
}

#[test]
fn a_cave_is_its_own_outline_with_a_floor_walls_and_a_ceiling() {
    let pieces = trace(&paint(&["#####", "#...#", "#####"]));
    // The outer outline is one closed loop; the hole's four concave corners
    // are all wall feet, so it is four open pieces.
    let closed: Vec<_> = pieces.iter().filter(|p| p.closed).collect();
    let open: Vec<_> = pieces.iter().filter(|p| !p.closed).collect();
    assert_eq!(closed.len(), 1, "{pieces:?}");
    assert_eq!(open.len(), 4, "{pieces:?}");
    // The cave floor runs left → right: a floor, rideable.
    assert!(
        open.iter().any(|p| p.points == vec![(4, 8), (16, 8)]),
        "the cave floor is a left→right segment: {open:?}"
    );
}

#[test]
fn cells_touching_only_at_a_corner_are_two_outlines() {
    let pieces = trace(&paint(&["#.", ".#"]));
    assert_eq!(pieces.len(), 2, "{pieces:?}");
    assert!(pieces.iter().all(|p| p.closed && p.points.len() == 4));
}

#[test]
fn a_ceiling_slope_traces_its_underside_right_to_left() {
    // A roof with a 45° slope hanging from it: the underside is a ceiling,
    // walked with the earth on the right, i.e. right → left.
    let pieces = trace(&paint(&["###", "r.."]));
    assert_eq!(pieces.len(), 1);
    let points = &pieces[0].points;
    let i = points.iter().position(|&p| p == (4, 8)).expect("the slope's low tip");
    let next = points[(i + 1) % points.len()];
    assert_eq!(next, (0, 4), "from the tip the underside climbs left: {points:?}");
}

#[test]
fn the_layer_lowers_to_valid_chains_in_world_space_with_its_earth() {
    let emission = emit_terrain_from_intgrid(
        &layer(&["......", ".../##", "../###", "######"]),
        ae::Vec2::new(100.0, 0.0),
        "test/Terrain",
    )
    .expect("a painted hill lowers");
    assert_eq!(emission.chains.len(), 1);
    let chain = &emission.chains[0];
    assert_eq!(chain.name, "terrain:test/Terrain#0");
    assert!(chain.validate().is_empty());
    // The flat before the slope, three cells down from the level's top.
    assert!(chain.points.contains(&ae::Vec2::new(100.0, 48.0)), "{:?}", chain.points);
    assert!(chain.points.contains(&ae::Vec2::new(100.0 + 32.0, 48.0)));
    // The slope reaches the top flat two cells up, two cells on.
    assert!(chain.points.contains(&ae::Vec2::new(100.0 + 64.0, 16.0)));
    // Earth: each row's full cells merge into one rectangle, plus the slopes.
    assert_eq!(chain.earth.len(), 3 + 2);
}

#[test]
fn an_unknown_value_is_refused_with_its_number() {
    let mut layer = layer(&["#"]);
    layer.int_grid_csv[0] = 99;
    let error = emit_terrain_from_intgrid(&layer, ae::Vec2::ZERO, "t").unwrap_err();
    assert!(error.contains("99"), "{error}");
}

#[test]
fn a_track_lowers_only_its_top_so_you_can_jump_up_through_it() {
    // A one-cell-thick road that climbs a 45° step: its outline is a closed
    // ring, and only the left → right runs survive as floors.
    let mut track = layer(&["..../#", "..../.", "####/."]);
    track.int_grid_csv = vec![
        0, 0, 0, 0, 2, 1, //
        0, 0, 0, 2, 0, 0, //
        1, 1, 1, 0, 0, 0,
    ];
    track.identifier = TRACK_LAYER.to_string();
    let emission = emit_track_from_intgrid(&track, ae::Vec2::ZERO, "t/Track").expect("lowers");
    assert!(!emission.chains.is_empty());
    for chain in &emission.chains {
        assert!(!chain.closed, "{}", chain.name);
        assert!(chain.name.starts_with("track:t/Track#"));
        for pair in chain.points.windows(2) {
            assert!(pair[1].x > pair[0].x, "only floors survive: {:?}", chain.points);
        }
    }
    // Nothing faces down: the flat road's underside at y = 48 is gone.
    assert!(emission
        .chains
        .iter()
        .flat_map(|c| c.points.windows(2))
        .all(|pair| !(pair[0].y == 48.0 && pair[1].y == 48.0)));
}
