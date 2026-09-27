//! The hall a boss fight happens in: its floor and its walls, measured once
//! from the room. Every boss that stages its moves against the room's floor
//! reads it here rather than sweeping for it on its own.

use ambition_platformer2d_core as ae;
use ae::Vec2;

/// The hall a fight happens in, measured once from the room.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hall {
    /// The floor's top surface.
    pub floor: f32,
    /// Inner faces of the side walls.
    pub left: f32,
    pub right: f32,
}

impl Hall {
    pub fn center_x(&self) -> f32 {
        (self.left + self.right) * 0.5
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }
}

/// Read the hall off the room: the floor under `from` and the walls either side
/// of it.
pub fn measure_hall(world: &ae::World, from: Vec2) -> Option<Hall> {
    let solid = |block: &ae::Block| matches!(block.kind, ae::BlockKind::Solid);
    let probe = ae::Aabb::new(from, Vec2::splat(2.0));
    let floor = world.first_body_sweep(probe, Vec2::new(0.0, 4000.0), solid)?.block.aabb.min.y;
    let row = ae::Aabb::new(Vec2::new(from.x, floor - 24.0), Vec2::splat(2.0));
    let left = world
        .first_body_sweep(row, Vec2::new(-8000.0, 0.0), solid)
        .map_or(0.0, |hit| hit.block.aabb.max.x);
    let right = world
        .first_body_sweep(row, Vec2::new(8000.0, 0.0), solid)
        .map_or(world.size.x, |hit| hit.block.aabb.min.x);
    Some(Hall { floor, left, right })
}
