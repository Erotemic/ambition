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
    // The hard corners stay exactly where they were painted: the level-left
    // edge's top, and the top flat's far end over the right edge.
    assert!(chain.points.contains(&ae::Vec2::new(100.0, 48.0)), "{:?}", chain.points);
    assert!(chain.points.contains(&ae::Vec2::new(100.0 + 96.0, 16.0)));
    // The slope's foot is rounded, not a kink, and stays near the paint.
    let foot = chain
        .points
        .iter()
        .filter(|p| (p.x - 132.0).abs() < 12.0)
        .map(|p| p.y)
        .fold(f32::NAN, f32::max);
    assert!(foot.is_nan() || (40.0..=48.0).contains(&foot), "foot {foot}: {:?}", chain.points);
    // The earth covers what was painted (2 + 3 + 6 full cells and two half-cell
    // slopes, 256 px² per cell), give or take the rounding.
    let area: f32 = chain.earth.iter().map(|q| polygon_area(q)).sum();
    assert!((area - 12.0 * 256.0).abs() < 128.0, "earth area {area}");
}

fn polygon_area(points: &[ae::Vec2]) -> f32 {
    let n = points.len();
    (0..n).map(|i| points[i].perp_dot(points[(i + 1) % n])).sum::<f32>().abs() * 0.5
}

/// A gentle slope the palette can only staircase — an 11° run, then a flat,
/// again and again — rides as the slope it stands for.
#[test]
fn a_painted_staircase_rides_as_an_even_slope() {
    // Each step: a four-cell 11° run (rises one cell) and a four-cell flat.
    let steps = 6;
    let (width, height) = (steps * 8 + 4, steps + 2);
    let mut rows = vec![vec!['.'; width]; height];
    let mut csv = vec![0; width * height];
    for step in 0..steps {
        let row = height - 2 - step;
        for (k, x) in (step * 8..step * 8 + 4).enumerate() {
            csv[row * width + x] = 8 + k as i32; // Up11a..d
            rows[row][x] = '/';
        }
        for x in step * 8 + 4..width {
            for r in row..height {
                csv[r * width + x] = 1;
            }
        }
        for x in step * 8..step * 8 + 4 {
            for r in row + 1..height {
                csv[r * width + x] = 1;
            }
        }
    }
    for x in 0..width {
        csv[(height - 1) * width + x] = 1;
    }
    let mut layer = layer(&["."]);
    layer.c_wid = width as i32;
    layer.c_hei = height as i32;
    layer.int_grid_csv = csv;
    let emission = emit_terrain_from_intgrid(&layer, ae::Vec2::ZERO, "stairs").expect("lowers");
    let floor: Vec<ae::Vec2> = emission
        .chains
        .iter()
        .flat_map(|c| c.points.windows(2).filter(|w| w[1].x > w[0].x).flat_map(|w| [w[0], w[1]]).collect::<Vec<_>>())
        .filter(|p| p.x > 64.0 && p.x < (steps * 8 * 16) as f32 - 64.0)
        .collect();
    assert!(!floor.is_empty());
    // The painted staircase turns 0° → 14° → 0° every 64 px; ridden, the
    // slope between neighbouring points stays close to the average ~7°.
    for w in floor.windows(2) {
        let d = w[1] - w[0];
        if d.x <= 0.0 {
            continue;
        }
        let deg = (-d.y).atan2(d.x).to_degrees();
        assert!((2.0..12.0).contains(&deg), "a {deg:.1}° segment at x={:.0}: {floor:?}", w[0].x);
    }
}

#[test]
fn a_cliff_top_stays_sharp() {
    // A plateau ending in a sheer drop: its top corner is hard.
    let emission = emit_terrain_from_intgrid(&layer(&["###.....", "########"]), ae::Vec2::ZERO, "c")
        .expect("lowers");
    assert!(
        emission.chains.iter().any(|c| c.points.contains(&ae::Vec2::new(48.0, 0.0))),
        "{:?}",
        emission.chains.iter().map(|c| &c.points).collect::<Vec<_>>()
    );
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
