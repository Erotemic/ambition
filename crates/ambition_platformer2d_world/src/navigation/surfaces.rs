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
use ae::navigation::{mix, NavLeg, NavLegKind, NavNext, ARRIVE_TOLERANCE, LAND_TOLERANCE, NAV_WAYPOINTS};
use ae::{BlockKind, GeoId, SurfaceKind, Vec2, World};

/// A cardinal motion frame, as two axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavFrame {
    pub side: Vec2,
    pub down: Vec2,
}

impl NavFrame {
    pub(super) fn body_half(self, size: Vec2) -> Option<Vec2> {
        if !is_cardinal(self.side) || !is_cardinal(self.down) {
            return None;
        }
        Some(Vec2::new((size * self.side).length(), (size * self.down).length()) * 0.5)
    }

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

fn is_cardinal(axis: Vec2) -> bool {
    (axis.x.abs() < 1e-4 && (axis.y.abs() - 1.0).abs() < 1e-4)
        || (axis.y.abs() < 1e-4 && (axis.x.abs() - 1.0).abs() < 1e-4)
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

/// The room's standing geometry for one physical footprint and set of axes.
#[derive(Clone, Debug)]
pub struct GroundMap {
    pub frame: NavFrame,
    pub half: Vec2,
    pub surfaces: Vec<StandSurface>,
    pub(super) slope_pieces: Vec<(Vec2, Vec2)>,
}

impl GroundMap {
    pub fn build(world: &World, frame: NavFrame, physical_size: Vec2) -> Option<Self> {
        let half = frame.body_half(physical_size)?;
        let walks = chain_walks(world, frame);
        let surfaces = standing_with_flats(world, frame, half * 2.0, walks.flats);
        let surfaces = split_at_touchdowns(surfaces, &walks.pieces, frame, half.x);
        Some(Self { frame, half, surfaces, slope_pieces: walks.pieces })
    }

    /// A grounded start within the clearance span of one continuous support.
    pub fn walk(&self, feet: Vec2, goal: Vec2) -> Option<NavNext> {
        let from = self.surface_at(feet)?;
        let span = &self.surfaces[from];
        let along = self.frame.along(feet);
        if (self.frame.below(feet) - span.top).abs() > 1.0
            || along < span.left - 2.0 || along > span.right + 2.0
            || self.surface_at(goal)? != from
        {
            return None;
        }
        Some(self.walk_on(from, feet, goal))
    }

    pub(super) fn walk_on(&self, surface: usize, feet: Vec2, goal: Vec2) -> NavNext {
        let land = self.point_on(surface, self.frame.along(goal));
        if (self.frame.along(land) - self.frame.along(feet)).abs() <= ARRIVE_TOLERANCE {
            return NavNext::Arrived;
        }
        let span = &self.surfaces[surface];
        let land_span = [span.left - self.half.x, span.right + self.half.x];
        NavNext::Leg(NavLeg { kind: NavLegKind::Walk, start: land, takeoff: land, land, land_span })
    }

    /// Preserve the native destination choice when only one support exists.
    pub fn only_support_waypoints(&self, feet: Vec2, seed: u64) -> Option<([Vec2; NAV_WAYPOINTS], u8)> {
        if self.surfaces.len() != 1 {
            return None;
        }
        self.local_waypoints(feet, seed)
    }

    pub fn local_waypoints(&self, feet: Vec2, seed: u64) -> Option<([Vec2; NAV_WAYPOINTS], u8)> {
        self.local_trip(feet, seed, None)
    }

    pub fn local_trip(&self, feet: Vec2, seed: u64, minimum: Option<f32>) -> Option<([Vec2; NAV_WAYPOINTS], u8)> {
        let surface = self.surface_at(feet)?;
        self.walk(feet, feet)?;
        let mut destination = self.waypoint(surface, seed);
        if let Some(minimum) = minimum.filter(|minimum| destination.distance(feet) <= *minimum) {
            let span = &self.surfaces[surface];
            let along = self.frame.along(feet);
            let end = if along - span.left > span.right - along { span.left } else { span.right };
            let distance = end - along;
            if distance.abs() <= minimum {
                return None;
            }
            let offset = (minimum + distance.abs()) * 0.5 * distance.signum();
            destination = self.point_on(surface, along + offset);
        }
        let mut points = [Vec2::ZERO; NAV_WAYPOINTS];
        points[0] = destination;
        Some((points, 1))
    }

    pub(super) fn waypoint(&self, surface: usize, seed: u64) -> Vec2 {
        let stretch = &self.surfaces[surface];
        let fraction = (mix(seed.rotate_left(17) ^ surface as u64) % 1000) as f32 / 999.0;
        self.frame.point(stretch.left + stretch.width() * fraction, stretch.top)
    }

    /// The surface the feet point `feet` stands on.
    pub fn surface_at(&self, feet: Vec2) -> Option<usize> {
        let (along, below) = (self.frame.along(feet), self.frame.below(feet));
        self.surfaces
            .iter()
            .enumerate()
            .filter(|(_, surface)| {
                (surface.top - below).abs() <= LAND_TOLERANCE
                    && along >= surface.left - self.half.x
                    && along <= surface.right + self.half.x
            })
            .min_by(|(_, a), (_, b)| (a.clamp(along) - along).abs().total_cmp(&(b.clamp(along) - along).abs()))
            .map(|(index, _)| index)
    }

    /// The feet point at `along` on `surface`, kept on the surface.
    pub fn point_on(&self, surface: usize, along: f32) -> Vec2 {
        let surface = &self.surfaces[surface];
        self.frame.point(surface.clamp(along), surface.top)
    }

    /// The nearest standing point at or below the destination.
    pub fn standing_destination(&self, destination: Vec2) -> Option<Vec2> {
        (0..self.surfaces.len())
            .map(|index| self.point_on(index, self.frame.along(destination)))
            .filter(|feet| self.frame.below(*feet) >= self.frame.below(destination) - 1.0)
            .min_by(|a, b| (*a - destination).length_squared().total_cmp(&(*b - destination).length_squared()))
    }

    /// The nearest surface under `point` (a body's centre, or its feet),
    /// no more than `depth` below it.
    pub fn surface_under(&self, point: Vec2, depth: f32) -> Option<usize> {
        let (along, below) = (self.frame.along(point), self.frame.below(point));
        (0..self.surfaces.len())
            .filter(|index| {
                let surface = &self.surfaces[*index];
                along >= surface.left - self.half.x
                    && along <= surface.right + self.half.x
                    && surface.top >= below - LAND_TOLERANCE
                    && surface.top <= below + depth
            })
            .min_by(|a, b| self.surfaces[*a].top.total_cmp(&self.surfaces[*b].top).then(a.cmp(b)))
    }

    /// The start support, the target support, and the point beside the target.
    pub fn approach(&self, feet: Vec2, target: Vec2, beside: f32, depth: f32) -> Option<(usize, usize, Vec2)> {
        let from = self.surface_at(feet)?;
        let to = self.surface_under(target, depth)?;
        let along = self.frame.along(target);
        let side = if self.frame.along(feet) < along { -1.0 } else { 1.0 };
        Some((from, to, self.point_on(to, along + side * beside)))
    }

    pub(super) fn near_slope_end(&self, point: Vec2) -> impl Iterator<Item = usize> + '_ {
        let along = self.frame.along(point);
        let top = self.frame.below(point);
        let width = self.half.x + SLOPE_END_REACH;
        let height = SLOPE_END_REACH * 0.5;
        let first = self.surfaces.partition_point(|span| span.top < top - height);
        self.surfaces[first..].iter().enumerate()
            .take_while(move |(_, span)| span.top <= top + height)
            .filter(move |(_, span)| along >= span.left - width && along <= span.right + width)
            .map(move |(index, _)| index + first)
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
    standing_with_flats(world, frame, body_size, chain_walks(world, frame).flats)
}

fn standing_with_flats(
    world: &World,
    frame: NavFrame,
    body_size: Vec2,
    flats: Vec<(f32, f32, f32, bool, GeoId)>,
) -> Vec<StandSurface> {
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
    tops.extend(flats);
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
/// Maximum distance from a slope end to the support that joins it.
pub(super) const SLOPE_END_REACH: f32 = 12.0;

/// Each surface cut where a slope piece touches down inside it from above. A
/// walking body that reaches the touchdown leaves the floor up the slope (the
/// charging cart's ramp comes down onto the middle of the workshop's floor),
/// so the floor on the two sides of it is two surfaces: the body's feet stop
/// half a body short of the touchdown on each.
fn split_at_touchdowns(surfaces: Vec<StandSurface>, pieces: &[(Vec2, Vec2)], frame: NavFrame, half_width: f32) -> Vec<StandSurface> {
    let mut out = Vec::with_capacity(surfaces.len());
    for surface in surfaces {
        let mut cuts: Vec<f32> = pieces
            .iter()
            .flat_map(|&(p, q)| [(p, q), (q, p)])
            .filter(|(touch, other)| {
                let along = frame.along(*touch);
                (frame.below(*touch) - surface.top).abs() <= SLOPE_END_REACH / 2.0
                    && frame.below(*other) < surface.top - SLOPE_END_REACH
                    && along - half_width > surface.left
                    && along + half_width < surface.right
            })
            .map(|(touch, _)| frame.along(touch))
            .collect();
        cuts.sort_by(f32::total_cmp);
        let mut left = surface.left;
        for cut in cuts {
            if cut - half_width > left {
                out.push(StandSurface { left, right: cut - half_width, ..surface.clone() });
            }
            left = left.max(cut + half_width);
        }
        if left <= surface.right {
            out.push(StandSurface { left, ..surface });
        }
    }
    out
}
