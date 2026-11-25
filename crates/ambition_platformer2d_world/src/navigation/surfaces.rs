//! The standing surfaces of a room: where one body can stand.
//!
//! A standing surface is a stretch of the top of the room's support blocks,
//! or a flat of a still surface chain (painted terrain's floors, a flat on an
//! authored line), with room above it for the body. Pieces that touch at one
//! height are one surface. A wall, a low ceiling, painted earth or a hazard
//! takes its stretch out.
//!
//! A chain's flat is ground when it has earth under it (painted earth or a
//! solid outline just below its middle, or a filled ground line) and none
//! over it: a ceiling's underside is no floor, and a bare authored line (a
//! Hanoi disk's top, a hidden plank) stays a line, as it was before painted
//! ground was planned on (64 stacked disk tops as surfaces took the Hanoi
//! room's graph from 23 ms to 4.9 s). The parts of a chain between its flats
//! are its slope pieces ([`chain_walks`]), which a slope leg walks from one
//! surface to the next.
//!
//! Positions are in the body's motion frame: `along` is the side axis and
//! `below` is the direction the body falls ([`NavFrame`]). The top of a
//! surface is the face against the fall.

use ambition_platformer2d_core as ae;
use ae::collision_semantics::{is_full_collision_surface, is_support_surface};
use ae::{BlockKind, GeoId, SurfaceKind, Vec2, World};

/// A cardinal motion frame, as two axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavFrame {
    pub side: Vec2,
    pub down: Vec2,
}

impl NavFrame {
    pub fn along(&self, point: Vec2) -> f32 {
        point.dot(self.side)
    }

    pub fn below(&self, point: Vec2) -> f32 {
        point.dot(self.down)
    }

    /// The world point at `along` on the side axis and `below` on the fall axis.
    pub fn point(&self, along: f32, below: f32) -> Vec2 {
        self.side * along + self.down * below
    }

    /// A box as (along min, along max, below min, below max).
    fn span(&self, aabb: &ae::Aabb) -> (f32, f32, f32, f32) {
        let (a, b) = (self.along(aabb.min), self.along(aabb.max));
        let (c, d) = (self.below(aabb.min), self.below(aabb.max));
        (a.min(b), a.max(b), c.min(d), c.max(d))
    }
}

/// One stretch a body can stand on.
#[derive(Clone, Debug, PartialEq)]
pub struct StandSurface {
    /// The first block of the surface: its durable identity.
    pub id: GeoId,
    /// Where the body's feet centre can be, along the side axis.
    pub left: f32,
    pub right: f32,
    /// The top of the surface, on the fall axis.
    pub top: f32,
    /// Each block of the surface is one-way: a body can come up through it.
    pub one_way: bool,
}

impl StandSurface {
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn clamp(&self, along: f32) -> f32 {
        along.clamp(self.left, self.right)
    }
}

/// A still chain's flats and slope pieces, in the motion frame.
pub(crate) struct ChainWalks {
    /// (top, along min, along max, one way, id), as block tops are.
    pub flats: Vec<(f32, f32, f32, bool, GeoId)>,
    /// Each part of a chain between two flats, or between a flat and the
    /// chain's end, as its two end points (world).
    pub pieces: Vec<(Vec2, Vec2)>,
}

/// How level a chain segment is to count as flat, on the fall axis.
const FLAT_TOLERANCE: f32 = 0.5;
/// How far over and under a flat the probes for open air and ground look.
const OVER_PROBE: f32 = 2.0;

/// The flats and slope pieces of the room's still chains. A chain that
/// moves or turns is no ground to plan on, as a block that moves is not.
pub(crate) fn chain_walks(world: &World, frame: NavFrame) -> ChainWalks {
    let solids: Vec<&Vec<Vec2>> = solid_polygons(world).collect();
    let solid_at = |p: Vec2, q: Vec2, offset: f32| {
        let middle = (p + q) * 0.5;
        let probe = frame.point(frame.along(middle), frame.below(middle) + offset);
        solids.iter().any(|polygon| inside(polygon, probe))
    };
    let mut walks = ChainWalks { flats: Vec::new(), pieces: Vec::new() };
    for chain in world.chains.iter() {
        let points = &chain.points;
        if points.len() < 2 || chain.moves() || chain.carrier.is_some() {
            continue;
        }
        let segments = if chain.closed { points.len() } else { points.len() - 1 };
        let segment = |i: usize| (points[i % points.len()], points[(i + 1) % points.len()]);
        let flat = |i: usize| {
            let (p, q) = segment(i);
            (frame.below(q) - frame.below(p)).abs() < FLAT_TOLERANCE
                && (frame.along(q) - frame.along(p)).abs() >= FLAT_TOLERANCE
                && !solid_at(p, q, -OVER_PROBE)
                && (chain.filled || solid_at(p, q, OVER_PROBE))
        };
        // A closed chain is walked from a segment that is no flat, so no run
        // is cut in two by where the list starts.
        let first = if chain.closed {
            match (0..segments).find(|i| !flat(*i)) {
                Some(i) => i,
                None => continue,
            }
        } else {
            0
        };
        let mut piece_start = segment(first).0;
        let mut runs: Vec<(Vec2, Vec2)> = Vec::new();
        let mut opening_piece: Option<(Vec2, Vec2)> = None;
        let mut n = 0;
        while n < segments {
            let i = first + n;
            if !flat(i) {
                n += 1;
                continue;
            }
            let run_start = segment(i).0;
            let mut end = n;
            while end + 1 < segments && flat(first + end + 1) {
                end += 1;
            }
            let run_end = segment(first + end).1;
            if piece_start != run_start {
                let piece = (piece_start, run_start);
                if chain.closed && runs.is_empty() {
                    opening_piece = Some(piece);
                } else {
                    walks.pieces.push(piece);
                }
            }
            runs.push((run_start, run_end));
            piece_start = run_end;
            n = end + 1;
        }
        let end_point = if chain.closed { segment(first).0 } else { points[points.len() - 1] };
        match (chain.closed, opening_piece) {
            // Round the closing join, from the last flat to the first.
            (true, Some((_, to))) => walks.pieces.push((piece_start, to)),
            (true, None) if !runs.is_empty() && piece_start != runs[0].0 => walks.pieces.push((piece_start, runs[0].0)),
            (false, _) if piece_start != end_point => walks.pieces.push((piece_start, end_point)),
            _ => {}
        }
        for (k, (p, q)) in runs.into_iter().enumerate() {
            let (a, b) = (frame.along(p), frame.along(q));
            walks.flats.push((
                frame.below(p),
                a.min(b),
                a.max(b),
                // A walking body stands on a line as a one-way floor.
                chain.kind == SurfaceKind::Ground,
                GeoId::tile_layer(chain.name.clone(), u16::try_from(k).unwrap_or(u16::MAX)),
            ));
        }
    }
    walks
}

/// Painted earth and solid outlines: the room's solid that is no block.
fn solid_polygons(world: &World) -> impl Iterator<Item = &Vec<Vec2>> {
    world.chains.iter().flat_map(|chain| {
        chain.earth.iter().chain((chain.kind == SurfaceKind::Solid && chain.closed).then_some(&chain.points))
    })
}

/// Whether `p` is inside the polygon (even-odd).
fn inside(polygon: &[Vec2], p: Vec2) -> bool {
    let mut odd = false;
    for i in 0..polygon.len() {
        let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            odd = !odd;
        }
    }
    odd
}

/// How far from the end of a stretch the feet centre stays.
const EDGE_INSET: f32 = 2.0;
/// How far from the side of the room a surface stops. The side of a room is
/// an exit, and a route does not leave the room.
const ROOM_SIDE_INSET: f32 = 8.0;

/// The standing surfaces of `world` for a body `body_size` wide and tall (in
/// the frame: x along, y against the fall).
///
/// A block that moves is no surface: a route is planned one time for the room.
/// The order is by height, then along the side axis, so it is the same for the
/// same room.
pub fn standing_surfaces(world: &World, frame: NavFrame, body_size: Vec2) -> Vec<StandSurface> {
    let half_width = body_size.x * 0.5;
    let mut tops: Vec<(f32, f32, f32, bool, GeoId)> = world
        .blocks
        .iter()
        .filter(|block| is_support_surface(block.kind) && block.velocity == Vec2::ZERO)
        .map(|block| {
            let (a0, a1, top, _) = frame.span(&block.aabb);
            (top, a0, a1, block.kind == BlockKind::OneWay, block.id.clone())
        })
        .collect();
    tops.extend(chain_walks(world, frame).flats);
    tops.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)).then(a.2.total_cmp(&b.2)));

    // Blocks that touch at one height are one stretch.
    let mut stretches: Vec<(f32, f32, f32, bool, GeoId)> = Vec::new();
    for top in tops {
        match stretches.last_mut() {
            Some(last) if (last.0 - top.0).abs() < 0.5 && top.1 <= last.2 + 1.0 => {
                last.2 = last.2.max(top.2);
                last.3 &= top.3;
            }
            _ => stretches.push(top),
        }
    }

    // What takes room from a body that stands at a height: a solid or a hazard.
    let mut obstructions: Vec<(f32, f32, f32, f32)> = world
        .blocks
        .iter()
        .filter(|block| is_full_collision_surface(block.kind) || block.kind == BlockKind::Hazard)
        .map(|block| frame.span(&block.aabb))
        .collect();
    // Painted earth and solid outlines, by their boxes: a cave's roof.
    obstructions.extend(solid_polygons(world).map(|polygon| frame.span(&ae::layered::bounds_of(polygon))));
    let room = frame.span(&ae::aabb_from_min_size(Vec2::ZERO, world.size));
    let (room_left, room_right) = (room.0 + half_width + ROOM_SIDE_INSET, room.1 - half_width - ROOM_SIDE_INSET);

    let mut surfaces = Vec::new();
    for (top, a0, a1, one_way, id) in stretches {
        let mut free = vec![(a0.max(room_left), a1.min(room_right))];
        for (o0, o1, top_of, bottom_of) in &obstructions {
            // In the body's box: below the head and above the feet.
            if *top_of < top - 0.5 && *bottom_of > top - body_size.y + 0.5 {
                let (cut0, cut1) = (o0 - half_width, o1 + half_width);
                free = free
                    .into_iter()
                    .flat_map(|(f0, f1)| [(f0, f1.min(cut0)), (f0.max(cut1), f1)])
                    .filter(|(f0, f1)| f1 > f0)
                    .collect();
            }
        }
        for (f0, f1) in free {
            let inset = if f1 - f0 > 4.0 * EDGE_INSET { EDGE_INSET } else { 0.0 };
            if f1 - f0 >= 1.0 {
                surfaces.push(StandSurface { id: id.clone(), left: f0 + inset, right: f1 - inset, top, one_way });
            }
        }
    }
    surfaces
}
