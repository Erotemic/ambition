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
//! The palette can only STAIRCASE a gentle slope — a flat, an 11° run, a flat
//! — so the traced outline is smoothed before it is ridden or drawn: every
//! stretch between two hard corners (sharper than 50°: a cliff top, a wall's
//! foot, a roof's edge) is resampled and blurred over a few cells, and the
//! hard corners stay exactly where they were painted. A flat stays flat; a
//! staircase becomes the slope it stands for. The painted cells stay the
//! editor's view, and the ridden surface never strays far from them.
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
    let grid = layer.grid_size as f32;
    let scale = grid / Q as f32;
    let to_world = |p: P| ae::Vec2::new(p.0 as f32 * scale, p.1 as f32 * scale) + offset;
    // Smooth each traced piece between its hard corners; the pieces of one
    // ring, joined, are that ring smoothed — the outline the earth is cut from.
    let mut pieces: Vec<(Vec<ae::Vec2>, bool)> = Vec::new();
    let mut rings: Vec<Vec<ae::Vec2>> = Vec::new();
    for ring in trace_rings(&cells) {
        let mut outline: Vec<ae::Vec2> = Vec::new();
        for piece in ring {
            let points: Vec<ae::Vec2> = piece.points.iter().map(|&p| to_world(p)).collect();
            let points = smooth(&points, piece.closed, grid);
            outline.extend(&points[..if piece.closed { points.len() } else { points.len() - 1 }]);
            pieces.push((points, piece.closed));
        }
        rings.push(outline);
    }
    if floors_only {
        pieces = pieces
            .into_iter()
            .flat_map(|(points, closed)| floor_runs(&points, closed))
            .map(|run| (run, false))
            .collect();
    }
    let mut chains: Vec<ae::SurfaceChain> = Vec::new();
    for (n, (points, closed)) in pieces.into_iter().enumerate() {
        let name = format!("{name_prefix}#{n}");
        let chain = if closed {
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
        first.earth = earth_strips(&rings);
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
#[cfg(test)]
fn trace(cells: &[(P, TerrainCell)]) -> Vec<Piece> {
    trace_rings(cells).into_iter().flatten().collect()
}

/// Union the cells and trace every region's outline: one list of pieces per
/// ring (outer boundary or hole), in ring order.
fn trace_rings(cells: &[(P, TerrainCell)]) -> Vec<Vec<Piece>> {
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
        pieces.push(split_at_wall_feet(merge_collinear(ring)));
    }
    pieces
}

/// The maximal runs of a piece that head left → right — the floors, whose
/// `(t.y, -t.x)` normal points up — each as its own open run.
fn floor_runs(points: &[ae::Vec2], closed: bool) -> Vec<Vec<ae::Vec2>> {
    let n = points.len();
    let segments = if closed { n } else { n - 1 };
    let mut runs = Vec::new();
    let mut current: Vec<ae::Vec2> = Vec::new();
    for i in 0..segments {
        let (a, b) = (points[i], points[(i + 1) % n]);
        if b.x > a.x {
            if current.is_empty() {
                current.push(a);
            }
            current.push(b);
        } else if !current.is_empty() {
            runs.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        runs.push(current);
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

/// A corner sharper than this is HARD: smoothing keeps it exactly where it
/// was painted. The palette's own joints (45° at most) are soft.
const HARD_TURN_DEG: f32 = 50.0;
/// Resample spacing, and the blur's reach, in cells. Three cells melts the
/// staircase an 11° run and a flat make of a gentle slope, and moves a crest
/// only a few pixels.
const SMOOTH_STEP_CELLS: f32 = 0.5;
const SMOOTH_REACH_CELLS: f32 = 3.0;
/// How far a simplified point may sit from the smoothed curve, in pixels.
const SIMPLIFY_TOLERANCE: f32 = 0.3;

/// Soften a traced piece (see the module doc). Hard corners and an open
/// piece's ends stay fixed; each stretch between them is resampled, blurred
/// and simplified.
fn smooth(points: &[ae::Vec2], closed: bool, grid: f32) -> Vec<ae::Vec2> {
    let n = points.len();
    let turn = |i: usize| {
        let (prev, at, next) = (points[(i + n - 1) % n], points[i], points[(i + 1) % n]);
        let (a, b) = ((at - prev).normalize_or_zero(), (next - at).normalize_or_zero());
        a.perp_dot(b).atan2(a.dot(b)).abs()
    };
    let hard_turn = HARD_TURN_DEG.to_radians();
    let hard: Vec<usize> = (0..n)
        .filter(|&i| (!closed && (i == 0 || i == n - 1)) || turn(i) > hard_turn)
        .collect();
    let step = grid * SMOOTH_STEP_CELLS;
    // Repeated [¼ ½ ¼] passes are a Gaussian of variance passes/2 samples².
    let passes = (2.0 * (SMOOTH_REACH_CELLS / SMOOTH_STEP_CELLS).powi(2)) as usize;
    if hard.is_empty() {
        // A closed outline with no hard corner at all: blur it all the way round.
        let mut ring = resample(&[points, &points[..1]].concat(), step);
        ring.pop();
        let m = ring.len();
        for _ in 0..passes {
            let prev = ring.clone();
            for i in 0..m {
                ring[i] = prev[(i + m - 1) % m] * 0.25 + prev[i] * 0.5 + prev[(i + 1) % m] * 0.25;
            }
        }
        return ring;
    }
    let runs = if closed { hard.len() } else { hard.len() - 1 };
    let mut out: Vec<ae::Vec2> = Vec::new();
    for k in 0..runs {
        let (from, to) = (hard[k], hard[(k + 1) % hard.len()]);
        let mut run = vec![points[from]];
        let mut i = from;
        while i != to {
            i = (i + 1) % n;
            run.push(points[i]);
        }
        let run = smooth_run(&run, step, passes);
        out.extend(if out.is_empty() { &run[..] } else { &run[1..] });
    }
    if closed {
        out.pop(); // back at the first hard corner
    }
    out
}

/// One stretch between two fixed ends: resample, blur, simplify.
fn smooth_run(run: &[ae::Vec2], step: f32, passes: usize) -> Vec<ae::Vec2> {
    let length: f32 = run.windows(2).map(|w| w[0].distance(w[1])).sum();
    if length < 2.0 * step {
        return run.to_vec();
    }
    let mut points = resample(run, step);
    let last = points.len() - 1;
    for _ in 0..passes {
        let prev = points.clone();
        for i in 1..last {
            points[i] = prev[i - 1] * 0.25 + prev[i] * 0.5 + prev[i + 1] * 0.25;
        }
    }
    simplify(&points, SIMPLIFY_TOLERANCE)
}

/// Evenly spaced points along a polyline, both ends kept exactly.
fn resample(run: &[ae::Vec2], step: f32) -> Vec<ae::Vec2> {
    let length: f32 = run.windows(2).map(|w| w[0].distance(w[1])).sum();
    let count = (length / step).round().max(1.0) as usize;
    let spacing = length / count as f32;
    let mut out = Vec::with_capacity(count + 1);
    out.push(run[0]);
    let (mut segment, mut walked) = (0usize, 0.0f32);
    for k in 1..count {
        let target = spacing * k as f32;
        loop {
            let len = run[segment].distance(run[segment + 1]);
            if walked + len >= target || segment + 2 == run.len() {
                let f = if len > 0.0 { ((target - walked) / len).clamp(0.0, 1.0) } else { 0.0 };
                out.push(run[segment].lerp(run[segment + 1], f));
                break;
            }
            walked += len;
            segment += 1;
        }
    }
    out.push(*run.last().expect("a run has points"));
    out
}

/// Douglas–Peucker: drop points within `tolerance` of the line their
/// neighbours keep, ends kept.
fn simplify(points: &[ae::Vec2], tolerance: f32) -> Vec<ae::Vec2> {
    fn keep(points: &[ae::Vec2], tolerance: f32, from: usize, to: usize, out: &mut Vec<bool>) {
        let (a, b) = (points[from], points[to]);
        let ab = b - a;
        let len = ab.length();
        let (mut worst, mut at) = (0.0f32, None);
        for i in from + 1..to {
            let d = if len > 0.0 { ab.perp_dot(points[i] - a).abs() / len } else { points[i].distance(a) };
            if d > worst {
                (worst, at) = (d, Some(i));
            }
        }
        if let Some(i) = at.filter(|_| worst > tolerance) {
            out[i] = true;
            keep(points, tolerance, from, i, out);
            keep(points, tolerance, i, to, out);
        }
    }
    let last = points.len() - 1;
    let mut kept = vec![false; points.len()];
    kept[0] = true;
    kept[last] = true;
    keep(points, tolerance, 0, last, &mut kept);
    points.iter().zip(kept).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

/// The earth inside `rings` (outer boundaries and holes alike, filled
/// even-odd) as vertical trapezoids: between each pair of consecutive vertex
/// x positions, the edges spanning that slab pair off top to bottom.
fn earth_strips(rings: &[Vec<ae::Vec2>]) -> Vec<Vec<ae::Vec2>> {
    let mut edges: Vec<(ae::Vec2, ae::Vec2)> = Vec::new();
    let mut xs: Vec<f32> = Vec::new();
    for ring in rings {
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            xs.push(a.x);
            if (b.x - a.x).abs() > 1.0e-4 {
                edges.push(if a.x < b.x { (a, b) } else { (b, a) });
            }
        }
    }
    xs.sort_by(f32::total_cmp);
    xs.dedup_by(|a, b| (*a - *b).abs() < 1.0e-3);
    edges.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
    let y_at = |(a, b): (ae::Vec2, ae::Vec2), x: f32| a.y + (b.y - a.y) * ((x - a.x) / (b.x - a.x));
    let mut strips = Vec::new();
    let (mut next, mut active): (usize, Vec<(ae::Vec2, ae::Vec2)>) = (0, Vec::new());
    for slab in xs.windows(2) {
        let (x0, x1) = (slab[0], slab[1]);
        while next < edges.len() && edges[next].0.x <= x0 + 1.0e-3 {
            active.push(edges[next]);
            next += 1;
        }
        active.retain(|edge| edge.1.x >= x1 - 1.0e-3);
        let mid = (x0 + x1) * 0.5;
        let mut crossing: Vec<(ae::Vec2, ae::Vec2)> =
            active.iter().copied().filter(|edge| edge.0.x <= x0 + 1.0e-3).collect();
        crossing.sort_by(|a, b| y_at(*a, mid).total_cmp(&y_at(*b, mid)));
        for pair in crossing.chunks_exact(2) {
            let (top, bottom) = (pair[0], pair[1]);
            strips.push(vec![
                ae::Vec2::new(x0, y_at(top, x0)),
                ae::Vec2::new(x1, y_at(top, x1)),
                ae::Vec2::new(x1, y_at(bottom, x1)),
                ae::Vec2::new(x0, y_at(bottom, x0)),
            ]);
        }
    }
    strips
}

#[cfg(test)]
#[path = "terrain_tests.rs"]
mod tests;
