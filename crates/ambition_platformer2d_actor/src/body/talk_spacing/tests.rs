use super::*;
use ambition_platformer2d_core::{Block, World};

/// A body 24 x 48, standing on a floor whose top is at y = 100.
const HALF: Vec2 = Vec2::new(12.0, 24.0);
const DOWN: Vec2 = Vec2::new(0.0, 1.0);
const LEVEL: f32 = 100.0 - 24.0;

fn room(blocks: Vec<Block>) -> World {
    World::new("talk", Vec2::new(800.0, 400.0), Vec2::ZERO, blocks)
}

fn floor(from: f32, to: f32) -> Block {
    Block::solid("floor", Vec2::new(from, 100.0), Vec2::new(to - from, 32.0))
}

fn step(world: &World, from: f32, to: f32) -> bool {
    step_is_safe(world, Vec2::new(from, LEVEL), Vec2::new(to, LEVEL), HALF, DOWN)
}

/// The control: an open floor is safe to step on, in both directions.
#[test]
fn a_step_along_an_open_floor_is_safe() {
    let world = room(vec![floor(0.0, 400.0)]);
    assert!(step(&world, 100.0, 160.0));
    assert!(step(&world, 160.0, 100.0));
}

/// A wall, a hazard or a rebound block on the way stops the step; so does a
/// gap in the floor at the mark or on the way.
#[test]
fn a_step_into_a_wall_a_hazard_or_a_gap_is_not_safe() {
    let wall = Block::solid("wall", Vec2::new(130.0, 0.0), Vec2::new(8.0, 100.0));
    assert!(!step(&room(vec![floor(0.0, 400.0), wall]), 100.0, 160.0), "a wall");
    let spikes = Block::hazard("spikes", Vec2::new(130.0, 90.0), Vec2::new(16.0, 10.0));
    assert!(!step(&room(vec![floor(0.0, 400.0), spikes]), 100.0, 160.0), "a hazard");
    // The floor ends at 120 and starts again at 300: the mark is over the gap.
    assert!(!step(&room(vec![floor(0.0, 120.0), floor(300.0, 400.0)]), 100.0, 160.0), "a gap at the mark");
    // The mark has a floor, but the way crosses a gap wider than the body.
    assert!(!step(&room(vec![floor(0.0, 110.0), floor(160.0, 400.0)]), 90.0, 200.0), "a gap on the way");
}
