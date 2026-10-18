use super::*;
use crate::navigation::graph::step;
use crate::navigation::NavGraph;
use ae::movement::GRAVITY;
use ae::navigation::{follow_flight, FlightFacts, FlightProgress};
use ae::{AbilitySet, Block, BodyClusterScratch, MotionFrame};

const FLOOR: f32 = 1100.0;

fn normal_frame() -> MotionFrame {
    MotionFrame::from_acceleration(Vec2::new(0.0, GRAVITY)).expect("a non-zero gravity")
}

/// A body that walks, jumps and toggles flight.
fn flier() -> BodyClusterScratch {
    let abilities = AbilitySet {
        move_horizontal: true,
        jump: true,
        fly: true,
        fly_toggle: true,
        ..AbilitySet::NONE
    };
    BodyClusterScratch::new_with_abilities(Vec2::ZERO, abilities)
}

/// A walled room with a floor, and a wall from the floor up to `wall_top`
/// in the middle of it.
fn room(wall_top: f32) -> World {
    World::new(
        "a wall to fly over",
        Vec2::new(2400.0, 1200.0),
        Vec2::ZERO,
        vec![
            Block::solid("floor", Vec2::new(0.0, FLOOR), Vec2::new(2400.0, 100.0)),
            Block::solid("ceiling", Vec2::new(0.0, 0.0), Vec2::new(2400.0, 32.0)),
            Block::solid(
                "wall",
                Vec2::new(1150.0, wall_top),
                Vec2::new(100.0, FLOOR - wall_top),
            ),
        ],
    )
}

fn map_of(world: &World) -> FlightMap {
    NavGraph::build(world, &flier(), normal_frame())
        .expect("an upright flier has a graph")
        .flight
        .expect("a body that flies has the air of its room")
}

/// The centre of a body of `half` that stands on the floor at `x`.
fn standing(map: &FlightMap, x: f32) -> Vec2 {
    Vec2::new(x, FLOOR - map.half.y)
}

/// Fly `route` in the kernel from its first point, by `follow_flight`, and
/// return the steps it took to arrive.
fn fly(world: &World, map: &FlightMap, route: &[Vec2]) -> Result<usize, String> {
    let frame = normal_frame();
    let dt = 1.0 / 60.0;
    let mut body = flier();
    {
        let (model, mut clusters) = body.parts();
        ae::movement::transit_body(
            model,
            &mut clusters,
            route[0],
            ae::movement::TransitVelocity::Zero,
        );
    }
    step(
        &mut body,
        world,
        frame,
        dt,
        ae::navigation::LegInput {
            fly_toggle: true,
            ..Default::default()
        },
    );
    let mut next = 1;
    for index in 0..1800 {
        let facts = FlightFacts {
            center: body.kinematics.pos,
            vel: body.kinematics.vel,
            side: frame.side(),
            down: frame.down(),
            law: map.law,
        };
        match follow_flight(route, next, &facts) {
            (_, FlightProgress::Arrived) => return Ok(index),
            (input, FlightProgress::Going(at)) => {
                next = at;
                if step(&mut body, world, frame, dt, input) {
                    return Err(format!("the body was reset at {:?}", body.kinematics.pos));
                }
            }
        }
    }
    Err(format!(
        "no arrival: at {:?}, flying to point {next} of {route:?}",
        body.kinematics.pos
    ))
}

/// ⭐ A FLIGHT GOES OVER A WALL, AND A BODY THAT FOLLOWS IT ARRIVES. The route
/// from one side of the floor to the other rises over the wall, each of its
/// points is free air, and the body flies it in the kernel to the end.
#[test]
fn a_flight_goes_over_a_wall_and_the_body_that_flies_it_arrives() {
    let world = room(500.0);
    let map = map_of(&world);
    assert!(
        map.law.top_speed > 0.0 && map.law.braking > 0.0,
        "measured: {:?}",
        map.law
    );
    let (from, to) = (standing(&map, 300.0), standing(&map, 2100.0));
    let mut route = Vec::new();
    assert!(
        map.route(from, to, &mut FlightSearch::default(), &mut route),
        "a route over the wall"
    );
    assert_eq!(route[0], from, "from where the body is");
    assert!(
        (*route.last().unwrap() - to).length() <= FLIGHT_CELL * 2.0,
        "to the goal: {route:?}"
    );
    assert!(
        route.iter().any(|at| at.y + map.half.y < 500.0),
        "the route rises over the wall top: {route:?}"
    );
    assert!(
        route[1..route.len() - 1]
            .iter()
            .all(|at| map.clearance_at(*at) > 0.0),
        "each turn is in the air"
    );
    assert!(
        route.len() < 12,
        "a few straight parts, not a cell path: {} points",
        route.len()
    );
    fly(&world, &map, &route).expect("the body flies the route");
}

/// A FLIGHT KEEPS TO THE AIR. Across an open room from floor to floor, the
/// route climbs to where it is clear of the floor, not along it: so it goes
/// over what walks there.
#[test]
fn a_flight_across_an_open_room_keeps_clear_of_the_floor() {
    let world = room(FLOOR - 1.0);
    let map = map_of(&world);
    let mut route = Vec::new();
    assert!(map.route(
        standing(&map, 200.0),
        standing(&map, 2200.0),
        &mut FlightSearch::default(),
        &mut route
    ));
    let highest = route
        .iter()
        .map(|at| FLOOR - (at.y + map.half.y))
        .fold(0.0, f32::max);
    assert!(
        highest >= 80.0,
        "the route's highest feet are {highest} px over the floor: {route:?}"
    );
    fly(&world, &map, &route).expect("the body flies the route");
}

/// No route into a closed box: the search says so, and the route is empty.
#[test]
fn there_is_no_flight_into_a_closed_box() {
    let mut world = room(FLOOR - 1.0);
    for (name, at, size) in [
        ("box top", Vec2::new(1800.0, 700.0), Vec2::new(400.0, 32.0)),
        ("box left", Vec2::new(1800.0, 700.0), Vec2::new(32.0, 400.0)),
        (
            "box right",
            Vec2::new(2168.0, 700.0),
            Vec2::new(32.0, 400.0),
        ),
    ] {
        world.blocks.push(Block::solid(name, at, size));
    }
    let map = map_of(&world);
    let mut route = vec![Vec2::ZERO];
    assert!(!map.route(
        standing(&map, 300.0),
        standing(&map, 2000.0),
        &mut FlightSearch::default(),
        &mut route
    ));
    assert!(route.is_empty());
}
