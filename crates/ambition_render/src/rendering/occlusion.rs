//! What stops a light in a dark room (DR3,
//! `docs/planning/engine/dark-rooms-and-tunnels.md`): the room's solid
//! geometry as a grid, and each light's reach along each direction from it.
//!
//! The grid is built once a room, from what is solid to a body and drawn as
//! solid: solid blocks, a solid outline, painted earth (`SurfaceChain::earth`)
//! and the earth under a filled ground line. It is presentation: what
//! collides is unchanged, and a line a body walks on but that is not earth
//! does not stop light.
//!
//! A light's shadow is the distance its light reaches along each of a
//! number of directions (about one a cell round its rim), marched over the
//! grid. A light inside solid ground (a lantern hung into the ceiling)
//! shines from where it leaves it.

use bevy::prelude::*;

use ambition_platformer2d_core::world::{BlockKind, SurfaceKind, World};

/// The grid's cell, in world units.
pub const CELL: f32 = 16.0;
/// The fewest and most directions a light's reach is measured along.
const RAYS: std::ops::RangeInclusive<usize> = 32..=256;
/// How far into the solid a light still shows: the face of a wall the light
/// falls on is lit, not the cell before it.
const FACE: f32 = CELL;
/// How soft a shadow's edge is, in world units.
const SOFT: f32 = CELL;

/// The room's solid cells, in bevy's centered y-up space.
#[derive(Clone, Debug, PartialEq)]
pub struct SolidGrid {
    /// The bevy point of the grid's bottom-left corner.
    origin: Vec2,
    columns: u32,
    rows: u32,
    /// Row 0 is the bottom row.
    solid: Vec<bool>,
}

impl SolidGrid {
    /// The solid cells of `world`.
    pub fn of_world(world: &World) -> Self {
        let size = world.size.max(Vec2::splat(CELL));
        let columns = (size.x / CELL).ceil() as u32;
        let rows = (size.y / CELL).ceil() as u32;
        let mut grid = Self { origin: -size / 2.0, columns, rows, solid: vec![false; (columns * rows) as usize] };
        // World space is y-down from the room's top-left; a cell's middle in it.
        let middle = |column: u32, row: u32| Vec2::new((column as f32 + 0.5) * CELL, size.y - (row as f32 + 0.5) * CELL);
        for block in world.blocks.iter() {
            if !matches!(block.kind, BlockKind::Solid) {
                continue;
            }
            grid.fill(block.aabb.min, block.aabb.max, |_| true, &middle);
        }
        for chain in world.chains.iter() {
            for polygon in chain.earth.iter().chain((chain.kind == SurfaceKind::Solid && chain.closed).then_some(&chain.points)) {
                let (min, max) = bounds(polygon);
                grid.fill(min, max, |p| inside(polygon, p), &middle);
            }
            if chain.filled && chain.points.len() >= 2 {
                let (min, max) = bounds(&chain.points);
                let under = |p: Vec2| ground_at(&chain.points, p.x).is_some_and(|top| p.y >= top);
                grid.fill(Vec2::new(min.x, min.y), Vec2::new(max.x, size.y), under, &middle);
            }
        }
        grid
    }

    /// Mark solid each cell whose middle lies in the world-space box and
    /// passes `inside`.
    fn fill(&mut self, min: Vec2, max: Vec2, inside: impl Fn(Vec2) -> bool, middle: &impl Fn(u32, u32) -> Vec2) {
        let size_y = self.rows as f32 * CELL;
        let column = |x: f32| (x / CELL).clamp(0.0, self.columns as f32);
        // World y is down; row 0 is at the bottom.
        let row = |y: f32| ((size_y - y) / CELL).clamp(0.0, self.rows as f32);
        let (c0, c1) = (column(min.x).floor() as u32, column(max.x).ceil() as u32);
        let (r0, r1) = (row(max.y).floor() as u32, row(min.y).ceil() as u32);
        for r in r0..r1 {
            for c in c0..c1 {
                let p = middle(c, r);
                if p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y && inside(p) {
                    self.solid[(r * self.columns + c) as usize] = true;
                }
            }
        }
    }

    /// Whether the bevy point `p` is in a solid cell. Outside the room nothing is.
    pub fn solid_at(&self, p: Vec2) -> bool {
        let local = (p - self.origin) / CELL;
        if local.x < 0.0 || local.y < 0.0 {
            return false;
        }
        let (c, r) = (local.x as u32, local.y as u32);
        c < self.columns && r < self.rows && self.solid[(r * self.columns + c) as usize]
    }

    /// How far a light at `at` reaches along each of its directions, evenly
    /// round from `+x` by [`diamond`] turn, at most `radius`.
    pub fn reach(&self, at: Vec2, radius: f32) -> Vec<f32> {
        let step = CELL / 2.0;
        let steps = (radius / step).ceil() as u32;
        let rays = ((std::f32::consts::TAU * radius / CELL).ceil() as usize).clamp(*RAYS.start(), *RAYS.end());
        (0..rays)
            .map(|ray| {
                let direction = from_diamond(ray as f32 / rays as f32 * 4.0);
                // Out of the ground it starts in first.
                let mut left_the_ground = !self.solid_at(at);
                for n in 1..=steps {
                    let d = n as f32 * step;
                    let solid = self.solid_at(at + direction * d);
                    if !left_the_ground {
                        left_the_ground = !solid;
                    } else if solid {
                        return d;
                    }
                }
                radius
            })
            .collect()
    }
}

/// How much of a light at `at` reaches `p`, given its `reach` along each
/// direction: all of it short of the reach, none a little past it.
pub fn lit_part(reach: &[f32], at: Vec2, p: Vec2) -> f32 {
    let offset = p - at;
    let distance = offset.length();
    if distance < 1e-3 {
        return 1.0;
    }
    let rays = reach.len();
    let turn = diamond(offset) / 4.0 * rays as f32;
    let (below, t) = (turn.floor() as usize % rays, turn.fract());
    let limit = reach[below] * (1.0 - t) + reach[(below + 1) % rays] * t;
    ((limit + FACE - distance) / SOFT).clamp(0.0, 1.0)
}

/// A direction's turn from `+x`, `0..4` a quarter at a time, as the angle
/// orders it but with no trigonometry (the per-texel cost of a shadow).
fn diamond(v: Vec2) -> f32 {
    match (v.x >= 0.0, v.y >= 0.0) {
        (true, true) => v.y / (v.x + v.y),
        (false, true) => 1.0 - v.x / (v.y - v.x),
        (false, false) => 2.0 - v.y / (-v.x - v.y),
        (true, false) => 3.0 + v.x / (v.x - v.y),
    }
}

/// The unit direction of a [`diamond`] turn.
fn from_diamond(d: f32) -> Vec2 {
    let v = if d < 1.0 {
        Vec2::new(1.0 - d, d)
    } else if d < 2.0 {
        Vec2::new(1.0 - d, 2.0 - d)
    } else if d < 3.0 {
        Vec2::new(d - 3.0, 2.0 - d)
    } else {
        Vec2::new(d - 3.0, d - 4.0)
    };
    v.normalize()
}

fn bounds(points: &[Vec2]) -> (Vec2, Vec2) {
    points.iter().fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(min, max), p| (min.min(*p), max.max(*p)))
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

/// The highest point (least world y) of the line over `x`, if it spans it.
fn ground_at(points: &[Vec2], x: f32) -> Option<f32> {
    points
        .windows(2)
        .filter(|pair| x >= pair[0].x.min(pair[1].x) && x <= pair[0].x.max(pair[1].x))
        .map(|pair| {
            let span = pair[1].x - pair[0].x;
            if span.abs() < f32::EPSILON {
                pair[0].y.min(pair[1].y)
            } else {
                pair[0].y + (x - pair[0].x) / span * (pair[1].y - pair[0].y)
            }
        })
        .reduce(f32::min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::world::Block;

    /// A room 640 by 320 with a pillar in its middle, floor to ceiling.
    fn room_with_a_pillar() -> World {
        World::new(
            "pillar",
            Vec2::new(640.0, 320.0),
            Vec2::ZERO,
            vec![Block::solid("pillar", Vec2::new(304.0, 0.0), Vec2::new(32.0, 320.0))],
        )
    }

    #[test]
    fn a_wall_leaves_its_far_side_dark() {
        let grid = SolidGrid::of_world(&room_with_a_pillar());
        // Bevy space: the room is centred; the pillar spans x -16..16.
        assert!(grid.solid_at(Vec2::new(0.0, 0.0)));
        assert!(!grid.solid_at(Vec2::new(-100.0, 0.0)));
        let lamp = Vec2::new(-120.0, 0.0);
        let reach = grid.reach(lamp, 260.0);
        assert_eq!(lit_part(&reach, lamp, Vec2::new(-60.0, 0.0)), 1.0, "the near side is lit");
        assert_eq!(lit_part(&reach, lamp, Vec2::new(-16.0, 0.0)), 1.0, "the wall's face is lit");
        assert_eq!(lit_part(&reach, lamp, Vec2::new(80.0, 0.0)), 0.0, "past the pillar it is dark");
        assert_eq!(lit_part(&reach, lamp, Vec2::new(-120.0, 120.0)), 1.0, "up the open side is lit");
    }

    #[test]
    fn a_diamond_turn_goes_there_and_back() {
        for k in 0..64 {
            let d = k as f32 / 16.0;
            assert!((diamond(from_diamond(d)) - d).abs() < 1e-4, "{d}");
        }
    }

    #[test]
    fn a_lamp_hung_into_the_ceiling_shines_from_below_it() {
        let world = World::new(
            "ceiling",
            Vec2::new(640.0, 320.0),
            Vec2::ZERO,
            vec![Block::solid("ceiling", Vec2::new(0.0, 0.0), Vec2::new(640.0, 64.0))],
        );
        let grid = SolidGrid::of_world(&world);
        // The ceiling's underside is at bevy y 96; the lamp is just inside it.
        let lamp = Vec2::new(0.0, 100.0);
        let reach = grid.reach(lamp, 200.0);
        assert_eq!(lit_part(&reach, lamp, Vec2::new(0.0, -50.0)), 1.0);
    }
}
