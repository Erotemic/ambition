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

/// How far a body box at `at` of half size `half` can move `[left, right,
/// up, down]` before it meets a solid of `world`, each at most `cap`. A body
/// already in a solid reaches `0` every way.
pub fn measure_reach(world: &ae::World, at: Vec2, half: Vec2, cap: f32) -> [f32; 4] {
    let solid = |block: &ae::Block| matches!(block.kind, ae::BlockKind::Solid);
    let body = ae::Aabb::new(at, half);
    let inside = |b: ae::Aabb| b.min.x < body.max.x && body.min.x < b.max.x && b.min.y < body.max.y && body.min.y < b.max.y;
    if world.blocks.near(body).into_iter().any(|block| solid(block) && inside(block.aabb)) {
        return [0.0; 4];
    }
    [Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0), Vec2::new(0.0, 1.0)].map(|way| {
        world
            .first_body_sweep(body, way * cap, solid)
            .map_or(cap, |hit| hit.time_of_impact * cap)
    })
}

#[cfg(test)]
mod reach_tests {
    use super::*;

    /// The four ways out of a box between a floor, a ceiling and a wall: the
    /// gaps, the cap where nothing stands, and nothing from inside a solid.
    #[test]
    fn reach_is_the_gap_to_the_first_solid_each_way() {
        let world = ae::World::new(
            "reach",
            Vec2::new(2000.0, 2000.0),
            Vec2::splat(16.0),
            vec![
                // (corner, size)
                ae::Block::solid("floor", Vec2::new(0.0, 1050.0), Vec2::new(2000.0, 100.0)),
                ae::Block::solid("ceiling", Vec2::new(0.0, 800.0), Vec2::new(2000.0, 100.0)),
                ae::Block::solid("wall", Vec2::new(1150.0, 900.0), Vec2::new(100.0, 150.0)),
            ],
        );
        let half = Vec2::new(20.0, 20.0);
        let [left, right, up, down] = measure_reach(&world, Vec2::new(1000.0, 1000.0), half, 512.0);
        assert!((right - 130.0).abs() < 1.0, "the wall's face is 150 off, the box 20 wide: {right}");
        assert!((up - 80.0).abs() < 1.0, "the ceiling's underside is 100 over: {up}");
        assert!((down - 30.0).abs() < 1.0, "the floor is 50 under: {down}");
        assert_eq!(left, 512.0, "nothing west within the cap");
        assert_eq!(measure_reach(&world, Vec2::new(1200.0, 975.0), half, 512.0), [0.0; 4], "inside the wall");
    }
}
