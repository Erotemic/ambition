//! A walking body on a slope: up, down and over a crest with no tick in the
//! air, landing on one from a fall, passing one from below, and dropping
//! through one (`crate::walkable_lines`).

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
use crate::AbilitySet;
use crate::world::SurfaceChain;
use crate::{AabbExt, LocalAxes, Vec2, World};

const DOWN: Vec2 = Vec2::new(0.0, 1.0);

/// The test world's floor (its top at 852), and on it a ramp at 27 degrees
/// from x 400 up 150 to a landing at x 700, and down again on the far side
/// at 45 degrees to the floor.
fn ramp_world() -> World {
    let mut world = test_world();
    world.chains.push(SurfaceChain::open(
        "ramp",
        vec![
            Vec2::new(300.0, 852.0),
            Vec2::new(400.0, 852.0),
            Vec2::new(700.0, 702.0),
            Vec2::new(900.0, 702.0),
            Vec2::new(1050.0, 852.0),
            Vec2::new(1200.0, 852.0),
        ],
    ));
    world
}

fn jump(axis_y: f32, pressed: bool) -> InputState {
    InputState {
        axes: LocalAxes::new(0.0, axis_y),
        movement: ActionEdges::EMPTY.with(MovementAction::Jump, Edge { pressed, held: true, released: false }),
        ..Default::default()
    }
}

fn walk(dir: f32) -> InputState {
    InputState {
        axes: LocalAxes::new(dir, 0.0),
        ..Default::default()
    }
}

fn feet(scratch: &BodyClusterScratch) -> f32 {
    scratch.kinematics.aabb_oriented(DOWN).feet_coord(DOWN)
}

fn standing_body(x: f32) -> BodyClusterScratch {
    let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::new(x, 852.0 - 30.0), AbilitySet::sandbox_all());
    let world = ramp_world();
    for _ in 0..30 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(scratch.ground.on_ground, "the body settles on the floor");
    scratch
}

#[test]
fn a_body_walks_over_the_ramp_and_back_with_no_tick_in_the_air() {
    let world = ramp_world();
    let mut scratch = standing_body(250.0);
    let mut highest = f32::MAX;
    for (dir, ticks) in [(1.0, 240), (-1.0, 240)] {
        for tick in 0..ticks {
            let events = step_scratch(&world, &mut scratch, walk(dir));
            assert!(
                scratch.ground.on_ground,
                "in the air on tick {tick} walking {dir} at {:?} (feet {})",
                scratch.kinematics.pos,
                feet(&scratch)
            );
            assert!(
                !matches!(events.ground_contact, GroundContactTransition::Landed { .. }),
                "a landing on tick {tick} walking {dir} at {:?}",
                scratch.kinematics.pos
            );
            highest = highest.min(feet(&scratch));
        }
    }
    assert!((highest - 702.0).abs() < 0.5, "the body reached the top of the ramp (feet {highest})");
}

#[test]
fn a_falling_body_lands_on_a_ramp_once_and_stays() {
    let world = ramp_world();
    let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::new(550.0, 400.0), AbilitySet::sandbox_all());
    let mut landings = 0;
    for _ in 0..120 {
        let events = step_scratch(&world, &mut scratch, InputState::default());
        if matches!(events.ground_contact, GroundContactTransition::Landed { .. }) {
            landings += 1;
        }
    }
    assert_eq!(landings, 1);
    // Centred at 550, the box's uphill edge is at 564: the line there is at
    // 852 - (564 - 400) / 2.
    let at_edge = 852.0 - (550.0 + scratch.kinematics.size.x * 0.5 - 400.0) / 2.0;
    assert!((feet(&scratch) - at_edge).abs() < 0.5, "feet {} on the line {at_edge}", feet(&scratch));
}

#[test]
fn a_jump_under_the_ramp_passes_up_through_it_and_lands_on_it() {
    let world = ramp_world();
    // Under the middle of the ramp, standing on the floor.
    let mut scratch = standing_body(560.0);
    step_scratch(&world, &mut scratch, jump(0.0, true));
    let held = jump(0.0, false);
    let mut on_ramp = false;
    for _ in 0..90 {
        step_scratch(&world, &mut scratch, held);
        if scratch.ground.on_ground && feet(&scratch) < 840.0 {
            on_ramp = true;
            break;
        }
    }
    assert!(on_ramp, "the body jumped up through the ramp and landed on it (feet {})", feet(&scratch));
}

#[test]
fn a_drop_through_leaves_the_ramp() {
    let world = ramp_world();
    let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::new(550.0, 600.0), AbilitySet::sandbox_all());
    for _ in 0..60 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    let on_ramp = feet(&scratch);
    assert!(scratch.ground.on_ground && on_ramp < 840.0);
    step_scratch(&world, &mut scratch, jump(1.0, true));
    for _ in 0..60 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!((feet(&scratch) - 852.0).abs() < 0.5, "the body dropped through to the floor (feet {})", feet(&scratch));
}

/// A stair beside a street: it rises to the LEFT from its foot at x 900, and
/// its boarding is `Up`.
fn street_with_stair() -> World {
    let mut world = test_world();
    let mut stair = SurfaceChain::open("stair", vec![Vec2::new(450.0, 702.0), Vec2::new(600.0, 702.0), Vec2::new(900.0, 852.0)]);
    stair.boarding = crate::world::Boarding::Up;
    world.chains.push(stair);
    world
}

fn walk_up(dir: f32) -> InputState {
    InputState {
        axes: LocalAxes::new(dir, -1.0),
        ..Default::default()
    }
}

#[test]
fn a_body_walks_the_street_past_the_foot_of_a_stair() {
    let world = street_with_stair();
    let mut scratch = standing_body(1100.0);
    for _ in 0..180 {
        step_scratch(&world, &mut scratch, walk(-1.0));
        assert!((feet(&scratch) - 852.0).abs() < 0.5, "the body left the street at {:?}", scratch.kinematics.pos);
    }
    assert!(scratch.kinematics.pos.x < 600.0, "it walked past the stair ({:?})", scratch.kinematics.pos);
}

#[test]
fn a_body_that_holds_up_climbs_the_stair_and_walks_down_it_again() {
    let world = street_with_stair();
    let mut scratch = standing_body(1000.0);
    while scratch.kinematics.pos.x > 520.0 {
        step_scratch(&world, &mut scratch, walk_up(-1.0));
        assert!(scratch.ground.on_ground, "in the air at {:?}", scratch.kinematics.pos);
    }
    assert!((feet(&scratch) - 702.0).abs() < 1.0, "it climbed to the top (feet {})", feet(&scratch));
    let mut landings = 0;
    for _ in 0..180 {
        let events = step_scratch(&world, &mut scratch, walk(1.0));
        assert!(scratch.ground.on_ground, "in the air at {:?}", scratch.kinematics.pos);
        if matches!(events.ground_contact, GroundContactTransition::Landed { .. }) {
            landings += 1;
        }
    }
    assert_eq!(landings, 0);
    assert!((feet(&scratch) - 852.0).abs() < 0.5 && scratch.kinematics.pos.x > 900.0);
}

/// A switchback: flight 1 rises to the left from the foot at x 900 to a
/// landing at 568..632 (one chain), and flight 2 rises to the right from the
/// landing to a top at 760..860 (boarding `Up`).
fn switchback() -> World {
    let mut world = test_world();
    world.chains.push(SurfaceChain::open(
        "landing and flight 1",
        vec![Vec2::new(568.0, 772.0), Vec2::new(632.0, 772.0), Vec2::new(792.0, 852.0)],
    ));
    let mut flight_2 = SurfaceChain::open(
        "flight 2 and the top",
        vec![Vec2::new(600.0, 772.0), Vec2::new(760.0, 692.0), Vec2::new(860.0, 692.0)],
    );
    flight_2.boarding = crate::world::Boarding::Up;
    world.chains.push(flight_2);
    world
}

#[test]
fn a_switchback_is_climbed_holding_up_and_walked_down_without() {
    let world = switchback();
    let mut scratch = standing_body(850.0);
    // Up flight 1 to the landing, then, turning there, up flight 2.
    let mut ticks = 0;
    while scratch.kinematics.pos.x > 590.0 {
        step_scratch(&world, &mut scratch, walk_up(-1.0));
        ticks += 1;
        assert!(ticks < 300 && scratch.ground.on_ground, "lost on flight 1 at {:?}", scratch.kinematics.pos);
    }
    assert!((feet(&scratch) - 772.0).abs() < 1.0, "on the landing (feet {})", feet(&scratch));
    while scratch.kinematics.pos.x < 800.0 {
        step_scratch(&world, &mut scratch, walk_up(1.0));
        assert!(scratch.ground.on_ground, "in the air on flight 2 at {:?}", scratch.kinematics.pos);
    }
    assert!((feet(&scratch) - 692.0).abs() < 1.0, "it climbed both flights (feet {})", feet(&scratch));
    // Down flight 2 to the landing, turning there, and on down flight 1,
    // with no landing.
    let mut landings = 0;
    while scratch.kinematics.pos.x > 590.0 {
        if matches!(step_scratch(&world, &mut scratch, walk(-1.0)).ground_contact, GroundContactTransition::Landed { .. }) {
            landings += 1;
        }
        assert!(scratch.ground.on_ground, "in the air on flight 2 at {:?}", scratch.kinematics.pos);
    }
    for _ in 0..150 {
        if matches!(step_scratch(&world, &mut scratch, walk(1.0)).ground_contact, GroundContactTransition::Landed { .. }) {
            landings += 1;
        }
    }
    assert_eq!(landings, 0);
    assert!((feet(&scratch) - 852.0).abs() < 0.5, "it walked down to the street (feet {})", feet(&scratch));
}




#[test]
fn a_body_walks_off_a_terrace_down_its_stair_with_no_landing() {
    let mut world = test_world();
    world.blocks.push(crate::world::Block::solid("terrace", Vec2::new(36.0, 702.0), Vec2::new(564.0, 150.0)));
    let mut stair = SurfaceChain::open("stair", vec![Vec2::new(600.0, 702.0), Vec2::new(900.0, 852.0)]);
    stair.boarding = crate::world::Boarding::Up;
    world.chains.push(stair);
    let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::new(400.0, 702.0 - 30.0), AbilitySet::sandbox_all());
    for _ in 0..30 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(scratch.ground.on_ground && (feet(&scratch) - 702.0).abs() < 0.5);
    let mut landings = 0;
    for _ in 0..180 {
        if matches!(step_scratch(&world, &mut scratch, walk(1.0)).ground_contact, GroundContactTransition::Landed { .. }) {
            landings += 1;
        }
        assert!(scratch.ground.on_ground, "in the air at {:?}", scratch.kinematics.pos);
    }
    assert_eq!(landings, 0);
    assert!(scratch.kinematics.pos.x > 950.0 && (feet(&scratch) - 852.0).abs() < 0.5);
}
