//! Painted terrain: the `Terrain` IntGrid layer lowered to rideable surface chains.
//!
//! A level paints ground cell by cell from a small palette: full earth, and
//! floor and ceiling slopes at 45°, 22.5° (a two-cell run) and about 11° (a
//! four-cell run). The loader unions the painted cells, traces the outline of
//! every region, and emits it as ordinary [`ae::SurfaceChain`]s.
//!
//! ⭐ **THE PALETTE IS AN LDTK LIMITATION, NOT AN ENGINE ONE.** The engine's
//! ground is the arbitrary polyline it always was; this module is one way to
//! author it that a person can paint and read in the editor. `SurfaceChain`
//! entities still lower directly, so a level (or a future editor) that needs a
//! slope the palette lacks authors that polyline itself.
//!
//! What you paint is what you ride: the outline is not smoothed, so the map
//! is an exact reference for where the ground is. The engine's joint rules
//! handle the corners (a convex crest launches a fast rider; a concave dip is
//! followed).
//!
//! A second layer, `Track`, paints with the same palette but lowers only the
//! outline's upward-facing runs: a road you can jump up through from below
//! (a bridge, a high road), where painted `Terrain` is solid all round.
//!
//! One split: at a CONCAVE corner steeper than the palette's 45° (the foot of a
//! painted step or cliff) the outline is cut into separate chains. A single
//! chain follows every concave joint, so an unsplit foot would carry a fast
//! runner straight up the wall; split, the wall is a wall — the same answer a
//! block gives, whose faces meet only at convex corners.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use ambition_platformer2d_core as ae;

use super::LdtkLayerInstance;

pub(super) const TERRAIN_LAYER: &str = "Terrain";
pub(super) const TRACK_LAYER: &str = "Track";

/// The chain names painted layers produce start with one of these. A
/// `SurfaceLoop` whose `attach_to` is [`TERRAIN_FLOOR`] resolves to the painted
/// floor under it.
pub(crate) const TERRAIN_CHAIN_PREFIX: &str = "terrain:";
pub(crate) const TRACK_CHAIN_PREFIX: &str = "track:";

/// `SurfaceLoop.attach_to` value meaning "the painted ground under me".
pub(crate) const TERRAIN_FLOOR: &str = "terrain";

/// Quarter cells per cell edge: slope heights are multiples of a quarter cell.
const Q: i32 = 4;

/// One painted cell's solid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerrainCell {
    Full,
    /// Solid below a line. `left`/`right` are the solid's height at the cell's
    /// left and right edges, in quarter cells, measured up from its bottom.
    Floor { left: i32, right: i32 },
    /// Solid above a line. `left`/`right` are the solid's depth at the cell's
    /// left and right edges, in quarter cells, measured down from its top.
    Ceiling { left: i32, right: i32 },
}

/// The palette, value by value. Mirrored by the LDtk tools
/// (`ambition_ldtk_tools/terrain.py`), which also draw it as the layer's tiles.
///
/// Floor names say which way the SURFACE goes left to right; ceiling names
/// which way the UNDERSIDE goes.
pub(crate) const TERRAIN_PALETTE: [(i32, &str, TerrainCell); 29] = {
    use TerrainCell::{Ceiling as C, Floor as F, Full};
    [
        (1, "Ground", Full),
        (2, "Up45", F { left: 0, right: 4 }),
        (3, "Down45", F { left: 4, right: 0 }),
        (4, "Up22a", F { left: 0, right: 2 }),
        (5, "Up22b", F { left: 2, right: 4 }),
        (6, "Down22a", F { left: 4, right: 2 }),
        (7, "Down22b", F { left: 2, right: 0 }),
        (8, "Up11a", F { left: 0, right: 1 }),
        (9, "Up11b", F { left: 1, right: 2 }),
        (10, "Up11c", F { left: 2, right: 3 }),
        (11, "Up11d", F { left: 3, right: 4 }),
        (12, "Down11a", F { left: 4, right: 3 }),
        (13, "Down11b", F { left: 3, right: 2 }),
        (14, "Down11c", F { left: 2, right: 1 }),
        (15, "Down11d", F { left: 1, right: 0 }),
        (16, "CeilDown45", C { left: 0, right: 4 }),
        (17, "CeilUp45", C { left: 4, right: 0 }),
        (18, "CeilDown22a", C { left: 0, right: 2 }),
        (19, "CeilDown22b", C { left: 2, right: 4 }),
        (20, "CeilUp22a", C { left: 4, right: 2 }),
        (21, "CeilUp22b", C { left: 2, right: 0 }),
        (22, "CeilDown11a", C { left: 0, right: 1 }),
        (23, "CeilDown11b", C { left: 1, right: 2 }),
        (24, "CeilDown11c", C { left: 2, right: 3 }),
        (25, "CeilDown11d", C { left: 3, right: 4 }),
        (26, "CeilUp11a", C { left: 4, right: 3 }),
        (27, "CeilUp11b", C { left: 3, right: 2 }),
        (28, "CeilUp11c", C { left: 2, right: 1 }),
        (29, "CeilUp11d", C { left: 1, right: 0 }),
    ]
};

pub(crate) fn terrain_cell(value: i32) -> Option<TerrainCell> {
    TERRAIN_PALETTE
        .iter()
        .find(|(v, _, _)| *v == value)
        .map(|(_, _, cell)| *cell)
}

type P = (i32, i32);

impl TerrainCell {
    /// The cell's solid as a polygon in quarter cells (cell origin at 0,0, y
    /// down), wound so the solid is on the RIGHT of every edge — the winding
    /// under which `SurfaceChain`'s `(t.y, -t.x)` normal points out of it.
    fn outline(self) -> Vec<P> {
        let corners = match self {
            TerrainCell::Full => vec![(0, 0), (Q, 0), (Q, Q), (0, Q)],
            TerrainCell::Floor { left, right } => {
                vec![(0, Q - left), (Q, Q - right), (Q, Q), (0, Q)]
            }
            TerrainCell::Ceiling { left, right } => vec![(0, 0), (Q, 0), (Q, right), (0, left)],
        };
        let mut out: Vec<P> = Vec::with_capacity(corners.len());
        for point in corners {
            if out.last() != Some(&point) {
                out.push(point);
            }
        }
        if out.len() > 1 && out.first() == out.last() {
            out.pop();
        }
        out
    }
}

/// What a `Terrain` layer lowers to.
#[derive(Debug, Default)]
pub(super) struct TerrainEmission {
    pub chains: Vec<ae::SurfaceChain>,
}

/// Lower a `Terrain` IntGrid layer. `layer_key` names the chains
/// (`terrain:{layer_key}#{n}`), so it must be unique per level.
pub(super) fn emit_terrain_from_intgrid(
    layer: &LdtkLayerInstance,
    offset: ae::Vec2,
    layer_key: &str,
) -> Result<TerrainEmission, String> {
    emit_painted(layer, offset, &format!("{TERRAIN_CHAIN_PREFIX}{layer_key}"), false)
}

/// Lower a `Track` IntGrid layer: only the upward-facing runs of its outline.
pub(super) fn emit_track_from_intgrid(
    layer: &LdtkLayerInstance,
    offset: ae::Vec2,
    layer_key: &str,
) -> Result<TerrainEmission, String> {
    emit_painted(layer, offset, &format!("{TRACK_CHAIN_PREFIX}{layer_key}"), true)
}

fn emit_painted(
    layer: &LdtkLayerInstance,
    offset: ae::Vec2,
    name_prefix: &str,
    floors_only: bool,
) -> Result<TerrainEmission, String> {
    let (cw, ch) = (layer.c_wid, layer.c_hei);
    if cw <= 0 || ch <= 0 || layer.int_grid_csv.is_empty() {
        return Ok(TerrainEmission::default());
    }
    let expected = (cw as usize) * (ch as usize);
    if layer.int_grid_csv.len() != expected {
        return Err(format!(
            "intGridCsv length {} does not match cWid*cHei = {cw}*{ch} = {expected}",
            layer.int_grid_csv.len(),
        ));
    }
    let mut cells = Vec::new();
    for (index, &value) in layer.int_grid_csv.iter().enumerate() {
        if value == 0 {
            continue;
        }
        let cell = terrain_cell(value)
            .ok_or_else(|| format!("unknown {} IntGrid value {value}", layer.identifier))?;
        let (cx, cy) = ((index % cw as usize) as i32, (index / cw as usize) as i32);
        cells.push(((cx, cy), cell));
    }
    let scale = layer.grid_size as f32 / Q as f32;
    let to_world = |p: P| ae::Vec2::new(p.0 as f32 * scale, p.1 as f32 * scale) + offset;
    let mut pieces = trace(&cells);
    if floors_only {
        pieces = pieces.into_iter().flat_map(floor_runs).collect();
    }
    let mut chains: Vec<ae::SurfaceChain> = Vec::new();
    for (n, piece) in pieces.into_iter().enumerate() {
        let name = format!("{name_prefix}#{n}");
        let points = piece.points.into_iter().map(to_world).collect();
        let chain = if piece.closed {
            ae::SurfaceChain::closed_loop(name, points)
        } else {
            ae::SurfaceChain::open(name, points)
        };
        let problems = chain.validate();
        if !problems.is_empty() {
            return Err(problems.join("; "));
        }
        chains.push(chain);
    }
    if let Some(first) = chains.first_mut() {
        first.earth = earth(&cells)
            .into_iter()
            .map(|polygon| polygon.into_iter().map(to_world).collect())
            .collect();
    }
    Ok(TerrainEmission { chains })
}

/// One traced chain, in quarter cells.
#[derive(Clone, Debug, PartialEq)]
struct Piece {
    points: Vec<P>,
    closed: bool,
}

/// Union the cells and trace every region's outline into chain pieces.
fn trace(cells: &[(P, TerrainCell)]) -> Vec<Piece> {
    // Every cell contributes its outline as unit edges (axis edges split at
    // each quarter, so a half-height step cancels exactly against the part of
    // its neighbour's full edge it covers). An edge shared by two cells
    // appears once each way and cancels; what survives is the boundary.
    let mut edges: BTreeMap<(P, P), u32> = BTreeMap::new();
    for &((cx, cy), cell) in cells {
        let outline = cell.outline();
        for i in 0..outline.len() {
            let a = outline[i];
            let b = outline[(i + 1) % outline.len()];
            let a = (a.0 + cx * Q, a.1 + cy * Q);
            let b = (b.0 + cx * Q, b.1 + cy * Q);
            for (from, to) in unit_edges(a, b) {
                if let Some(count) = edges.get_mut(&(to, from)) {
                    *count -= 1;
                    if *count == 0 {
                        edges.remove(&(to, from));
                    }
                } else {
                    *edges.entry((from, to)).or_insert(0) += 1;
                }
            }
        }
    }
    let mut outgoing: HashMap<P, Vec<P>> = HashMap::new();
    for &(from, to) in edges.keys() {
        outgoing.entry(from).or_default().push(to);
    }
    let mut used: BTreeSet<(P, P)> = BTreeSet::new();
    let mut pieces = Vec::new();
    // Start each loop at its top-most, then left-most, vertex. The edge leaving
    // it heads along the top, so a closed loop's implicit closing segment is
    // never part of its floor (`attach_loop` only reads the listed segments).
    let mut starts: Vec<(P, P)> = edges.keys().copied().collect();
    starts.sort_by_key(|&(a, b)| (a.1, a.0, b.1, b.0));
    for start in starts {
        if used.contains(&start) {
            continue;
        }
        let mut ring: Vec<P> = vec![start.0];
        let (mut from, mut at) = start;
        used.insert(start);
        while at != start.0 {
            ring.push(at);
            let incoming = (at.0 - from.0, at.1 - from.1);
            let next = outgoing[&at]
                .iter()
                .copied()
                .filter(|&to| !used.contains(&(at, to)))
                .max_by(|&x, &y| {
                    let turn = |to: P| right_turn(incoming, (to.0 - at.0, to.1 - at.1));
                    turn(x).total_cmp(&turn(y))
                })
                .expect("a boundary is closed: every vertex it enters, it leaves");
            used.insert((at, next));
            from = at;
            at = next;
        }
        pieces.extend(split_at_wall_feet(merge_collinear(ring)));
    }
    pieces
}

/// The maximal runs of `piece` that head left → right — the floors, whose
/// `(t.y, -t.x)` normal points up — each as its own open piece.
fn floor_runs(piece: Piece) -> Vec<Piece> {
    let n = piece.points.len();
    let segments = if piece.closed { n } else { n - 1 };
    let is_floor = |i: usize| piece.points[(i + 1) % n].0 > piece.points[i].0;
    // A closed ring's first segment leaves its top-left vertex along the top:
    // a floor, so no run wraps past index 0 — but a run may END at the wrap.
    let mut runs = Vec::new();
    let mut current: Vec<P> = Vec::new();
    for i in 0..segments {
        if is_floor(i) {
            if current.is_empty() {
                current.push(piece.points[i]);
            }
            current.push(piece.points[(i + 1) % n]);
        } else if !current.is_empty() {
            runs.push(Piece { points: std::mem::take(&mut current), closed: false });
        }
    }
    if !current.is_empty() {
        runs.push(Piece { points: current, closed: false });
    }
    runs
}

/// The unit steps of edge `a → b`: an axis edge splits at every quarter; a
/// sloped edge stays whole (no two cells share one).
fn unit_edges(a: P, b: P) -> Vec<(P, P)> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    if dx != 0 && dy != 0 {
        return vec![(a, b)];
    }
    let steps = dx.abs().max(dy.abs());
    let (sx, sy) = (dx.signum(), dy.signum());
    (0..steps)
        .map(|i| ((a.0 + sx * i, a.1 + sy * i), (a.0 + sx * (i + 1), a.1 + sy * (i + 1))))
        .collect()
}

/// The signed turn from `a` to `b`, positive to the right (clockwise on a
/// y-down screen). At a pinch vertex the tightest right turn keeps the walk on
/// the region it is tracing.
fn right_turn(a: P, b: P) -> f32 {
    let cross = (a.0 * b.1 - a.1 * b.0) as f32;
    let dot = (a.0 * b.0 + a.1 * b.1) as f32;
    cross.atan2(dot)
}

fn direction(a: P, b: P) -> P {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let g = gcd(dx.abs(), dy.abs()).max(1);
    (dx / g, dy / g)
}

fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Drop every vertex that lies on a straight run, so a flat floor of a hundred
/// cells is one segment.
fn merge_collinear(ring: Vec<P>) -> Vec<P> {
    let n = ring.len();
    ring.iter()
        .enumerate()
        .filter(|&(i, &p)| {
            let prev = ring[(i + n - 1) % n];
            let next = ring[(i + 1) % n];
            direction(prev, p) != direction(p, next)
        })
        .map(|(_, &p)| p)
        .collect()
}

/// Cut a closed outline at the feet of its walls (see the module doc). With no
/// such corner it stays one closed chain.
fn split_at_wall_feet(ring: Vec<P>) -> Vec<Piece> {
    let n = ring.len();
    let is_foot = |i: usize| {
        let (prev, at, next) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        let (a, b) = (direction(prev, at), direction(at, next));
        let turn = right_turn(a, b);
        // Negative is a LEFT turn: with the solid on the right, a concave corner.
        let concave = turn < 0.0;
        let steeper_than_the_palette = -turn > 50f32.to_radians();
        concave && (steeper_than_the_palette || a.0 == 0 || b.0 == 0)
    };
    let feet: Vec<usize> = (0..n).filter(|&i| is_foot(i)).collect();
    if feet.is_empty() {
        return vec![Piece { points: ring, closed: true }];
    }
    let mut pieces = Vec::with_capacity(feet.len());
    for (k, &start) in feet.iter().enumerate() {
        let end = feet[(k + 1) % feet.len()];
        let mut points = vec![ring[start]];
        let mut i = start;
        loop {
            i = (i + 1) % n;
            points.push(ring[i]);
            if i == end {
                break;
            }
        }
        pieces.push(Piece { points, closed: false });
    }
    pieces
}

/// The painted cells as convex polygons for presentation: runs of full cells
/// merge into one rectangle per row; each slope cell is its own polygon.
fn earth(cells: &[(P, TerrainCell)]) -> Vec<Vec<P>> {
    let full: BTreeSet<P> = cells
        .iter()
        .filter(|(_, cell)| *cell == TerrainCell::Full)
        .map(|&(at, _)| at)
        .collect();
    let mut out = Vec::new();
    let mut rows: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    for &(cx, cy) in &full {
        rows.entry(cy).or_default().push(cx);
    }
    for (cy, mut xs) in rows {
        xs.sort_unstable();
        let mut i = 0;
        while i < xs.len() {
            let mut j = i;
            while j + 1 < xs.len() && xs[j + 1] == xs[j] + 1 {
                j += 1;
            }
            let (x0, x1) = (xs[i] * Q, (xs[j] + 1) * Q);
            let (y0, y1) = (cy * Q, (cy + 1) * Q);
            out.push(vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
            i = j + 1;
        }
    }
    for &((cx, cy), cell) in cells {
        if cell == TerrainCell::Full {
            continue;
        }
        out.push(
            cell.outline()
                .into_iter()
                .map(|(x, y)| (x + cx * Q, y + cy * Q))
                .collect(),
        );
    }
    out
}

#[cfg(test)]
#[path = "terrain_tests.rs"]
mod tests;
