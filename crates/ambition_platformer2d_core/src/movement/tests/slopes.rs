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

/// A ramp that moves (a ridden mount's slope): each tick its points move by
/// `step` and its velocity says so, as the overlay states it.
fn moved_ramp(world: &mut World, step: Vec2) {
    let ramp = world.chains.iter_mut().find(|c| c.name == "ramp").expect("the ramp");
    for p in &mut ramp.points {
        *p += step;
    }
    ramp.velocity = step;
}

#[test]
fn a_body_rides_a_moving_ramp_and_walks_on_it_with_no_tick_in_the_air() {
    let mut world = test_world();
    world.chains.push(SurfaceChain::open(
        "ramp",
        vec![Vec2::new(400.0, 700.0), Vec2::new(700.0, 550.0), Vec2::new(800.0, 550.0)],
    ));
    let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::new(550.0, 560.0), AbilitySet::sandbox_all());
    for _ in 0..60 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(scratch.ground.on_ground, "it settles on the ramp (feet {})", feet(&scratch));
    let line_at = |world: &World, scratch: &BodyClusterScratch| {
        crate::walkable_lines::standing_on_line(world, scratch.kinematics.aabb_oriented(DOWN), DOWN)
    };
    let start_x = scratch.kinematics.pos.x;
    // The ramp goes up and to the right, then back down and left; the body
    // stands still on it, then walks up it.
    let mut landings = 0;
    let mut stood_at = None;
    for (tick, step, input) in (0..210).map(|t| {
        let step = if t < 60 { Vec2::new(2.0, -1.5) } else if t < 120 { Vec2::new(-1.0, 2.0) } else { Vec2::new(1.0, 0.5) };
        (t, step, if t < 180 { InputState::default() } else { walk(1.0) })
    }) {
        moved_ramp(&mut world, step);
        let events = step_scratch(&world, &mut scratch, input);
        if matches!(events.ground_contact, GroundContactTransition::Landed { .. }) {
            landings += 1;
        }
        assert!(scratch.ground.on_ground, "in the air on tick {tick} at {:?}", scratch.kinematics.pos);
        assert!(line_at(&world, &scratch).is_some(), "off the ramp on tick {tick} at {:?}", scratch.kinematics.pos);
        if tick == 179 {
            stood_at = Some(scratch.kinematics.pos.x);
        }
    }
    assert_eq!(landings, 0);
    // Standing still for 180 ticks it went where the ramp went: 120 to the
    // right, 60 to the left, 60 to the right.
    let stood_at = stood_at.expect("tick 179 ran");
    assert!((stood_at - (start_x + 120.0)).abs() < 0.5, "the body drifted on the ramp: {start_x} -> {stood_at}");
    assert!(scratch.kinematics.pos.x > stood_at + 20.0, "and then it walked up it");
}

/// A plank that swings 25 degrees each way about its middle: a body standing
/// off its middle is carried round with it, stays on it and never lands.
#[test]
fn a_body_rides_a_swinging_plank() {
    let mut world = test_world();
    let pivot = Vec2::new(600.0, 600.0);
    let plank = |angle: f32| {
        let (s, c) = angle.sin_cos();
        let half = Vec2::new(c, s) * 150.0;
        vec![pivot - half, pivot + half]
    };
    world.chains.push(SurfaceChain::open("plank", plank(0.0)));
    let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::new(660.0, 560.0), AbilitySet::sandbox_all());
    for _ in 0..30 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(scratch.ground.on_ground);
    let along = scratch.kinematics.pos.x - pivot.x;
    let mut angle = 0.0_f32;
    let mut landings = 0;
    for tick in 1..=240 {
        let next = 25.0_f32.to_radians() * (std::f32::consts::TAU * tick as f32 / 240.0).sin();
        let line = world.chains.iter_mut().find(|c| c.name == "plank").expect("the plank");
        line.points = plank(next);
        line.turn = next - angle;
        line.pivot = pivot;
        angle = next;
        if matches!(step_scratch(&world, &mut scratch, InputState::default()).ground_contact, GroundContactTransition::Landed { .. }) {
            landings += 1;
        }
        assert!(scratch.ground.on_ground, "off the plank on tick {tick} at {:?}", scratch.kinematics.pos);
    }
    assert_eq!(landings, 0);
    // Back level, the body is where it stood along the plank.
    assert!((scratch.kinematics.pos.x - pivot.x - along).abs() < 2.0, "it slid along the plank: {} -> {}", along, scratch.kinematics.pos.x - pivot.x);
}

/// A plank that lifts fast and tilts as it lifts (a giant's shoulder that
/// rises as it rears): its motion is a move of its middle and a turn about
/// it, and a body standing off its middle is carried with it, never landing.
#[test]
fn a_body_rides_a_plank_that_lifts_and_tilts() {
    let mut world = test_world();
    let plank = |mid: Vec2, angle: f32| {
        let half = Vec2::from_angle(angle) * 60.0;
        vec![mid - half, mid + half]
    };
    let mut mid = Vec2::new(600.0, 700.0);
    let mut angle = 0.0_f32;
    world.chains.push(SurfaceChain::open("plank", plank(mid, angle)));
    let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::new(630.0, 660.0), AbilitySet::sandbox_all());
    for _ in 0..30 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(scratch.ground.on_ground);
    let mut landings = 0;
    for tick in 1..=24 {
        let (next_mid, next_angle) = (mid + Vec2::new(0.0, -5.0), angle - 0.5_f32.to_radians());
        let line = world.chains.iter_mut().find(|c| c.name == "plank").expect("the plank");
        line.points = plank(next_mid, next_angle);
        line.velocity = next_mid - mid;
        line.turn = next_angle - angle;
        line.pivot = next_mid;
        (mid, angle) = (next_mid, next_angle);
        if matches!(step_scratch(&world, &mut scratch, InputState::default()).ground_contact, GroundContactTransition::Landed { .. }) {
            landings += 1;
        }
        assert!(scratch.ground.on_ground, "off the plank on tick {tick} at {:?}", scratch.kinematics.pos);
    }
    assert_eq!(landings, 0);
}

/// A solid outline (`SurfaceKind::Solid`, what a portal pair carries of a
/// block it turns): a body that walks into its side is stopped there, and one
/// that falls on its top stands on it — both where the turned shape is, not
/// where the box around it is.
#[test]
fn a_body_meets_a_turned_solid_as_its_shape() {
    // A square of 80 turned by 30 degrees on the floor: its lowest corner
    // touches nothing, its top is a 30 degree slope.
    let centre = Vec2::new(600.0, 780.0);
    let turn = Vec2::from_angle(30f32.to_radians());
    let corners: Vec<Vec2> =
        [Vec2::new(-40.0, -40.0), Vec2::new(40.0, -40.0), Vec2::new(40.0, 40.0), Vec2::new(-40.0, 40.0)].iter().map(|c| centre + turn.rotate(*c)).collect();
    let mut world = test_world();
    world.chains.push(SurfaceChain::solid("turned", corners.clone()));
    let inside = |p: Vec2| crate::solid_lines::push_out(crate::Aabb::new(p, Vec2::splat(0.25)), &corners).is_some();
    // Walk at it from the left: stopped at its face, never in it.
    let mut scratch = standing_body(400.0);
    for tick in 0..240 {
        step_scratch(&world, &mut scratch, walk(1.0));
        let body = scratch.kinematics.aabb_oriented(DOWN);
        assert!(
            crate::solid_lines::push_out(body, &corners).is_none(),
            "in the solid on tick {tick} at {:?}",
            scratch.kinematics.pos
        );
    }
    // The box around it begins at x = 545.4, and its face at the body's
    // height is further right: the body comes past where the box would have
    // stopped it, to the face.
    let front = scratch.kinematics.aabb_oriented(DOWN).max.x;
    assert!(front > 550.0 && front < 600.0, "not stopped at the turned face: {front}");
    let body = scratch.kinematics.aabb_oriented(DOWN);
    assert!(
        (body.min.y as i32..body.max.y as i32).any(|y| inside(Vec2::new(front + 3.0, y as f32))),
        "the face is not where it stopped: {body:?}"
    );
    // Dropped on its top, it stands there, above the box around it nowhere.
    let mut scratch = BodyClusterScratch::new_with_abilities(centre + Vec2::new(0.0, -120.0), AbilitySet::sandbox_all());
    for _ in 0..120 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(scratch.ground.on_ground, "it does not stand on the top: {:?}", scratch.kinematics.pos);
    assert!(crate::solid_lines::push_out(scratch.kinematics.aabb_oriented(DOWN), &corners).is_none());
    assert!(scratch.kinematics.pos.y < centre.y - 40.0, "{:?}", scratch.kinematics.pos);
}
