//! The standing surfaces of a room: where one body can stand.
//!
//! A standing surface is a stretch of the top of the room's support blocks
//! with room above it for the body. Blocks that touch at one height are one
//! surface. A wall, a low ceiling or a hazard takes its stretch out.
//!
//! Positions are in the body's motion frame: `along` is the side axis and
//! `below` is the direction the body falls ([`NavFrame`]). The top of a
//! surface is the face against the fall.

use ambition_platformer2d_core as ae;
use ae::collision_semantics::{is_full_collision_surface, is_support_surface};
use ae::{BlockKind, GeoId, Vec2, World};

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
    let obstructions: Vec<(f32, f32, f32, f32)> = world
        .blocks
        .iter()
        .filter(|block| is_full_collision_surface(block.kind) || block.kind == BlockKind::Hazard)
        .map(|block| frame.span(&block.aabb))
        .collect();
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
