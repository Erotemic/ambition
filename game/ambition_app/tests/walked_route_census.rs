#![cfg(feature = "rl_sim")]
//! A MEASUREMENT for WORLD-ACCEPTANCE's walked route: which of the
//! playthrough's crossings the player's own body can walk to by the in-room
//! surface graph.
//!
//! Run it with
//! `cargo test -p ambition_app --test app_it walked_route_census -- --ignored --nocapture`.
//! For each crossing it builds the graph of the room for the player's body
//! (its `MotionModel`, abilities and box, read off the live body), starts at
//! the arrival from the room before, and asks for a route to the place where
//! the body overlaps the exit zone. It prints one row per crossing, with the body's real
//! abilities, with its air jump taken away, and with its wall verbs taken
//! away, so the share of each leg is a measured delta. It asserts only that
//! each room builds.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt as _;
use ambition_platformer2d::world::navigation::NavGraph;

use crate::common::{base, fixed_60hz_room_sim};

/// The playthrough's crossings, in its order: (the room, the room its exit
/// leads to).
const CROSSINGS: &[(&str, &str)] = &[
    ("central_hub_complex", "intro_wake_room"),
    ("intro_wake_room", "intro_raid_corridor"),
    ("intro_raid_corridor", "intro_escape_shaft"),
    ("intro_escape_shaft", "drain_alley"),
    ("drain_alley", "under_town_pipes"),
    ("under_town_pipes", "alice_relay"),
    ("alice_relay", "bob_relay"),
    ("bob_relay", "alice_relay"),
    ("bob_relay", "drain_alley"),
];

/// The player's body as the hub spawns it.
fn the_player(sim: &mut ambition_app::Platformer2dSimHarness) -> (ae::BodyClusterScratch, ae::Vec2) {
    let world = sim.world_mut();
    let player = world
        .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .expect("one player");
    let model = world.get::<ae::MotionModel>(player).expect("a movement law").clone();
    let abilities = world.get::<ae::BodyAbilities>(player).expect("abilities").abilities;
    let kinematics = *world.get::<ae::BodyKinematics>(player).expect("a body");
    let mut body = ae::BodyClusterScratch::new_with_abilities(ae::Vec2::ZERO, abilities);
    body.model = model;
    body.kinematics.size = kinematics.size;
    body.base_size.base_size = kinematics.size;
    (body, kinematics.pos)
}

/// The feet point on the nearest surface under `point`, or the nearest
/// surface at all.
fn on_a_surface(graph: &NavGraph, point: ae::Vec2) -> Option<ae::Vec2> {
    let under = graph.surface_under(point, 2000.0).or_else(|| {
        (0..graph.surfaces.len()).min_by(|a, b| {
            let distance = |index: usize| graph.point_on(index, graph.frame.along(point)).distance(point);
            distance(*a).total_cmp(&distance(*b))
        })
    })?;
    Some(graph.point_on(under, graph.frame.along(point)))
}

/// The feet point on a surface where the body comes nearest to `zone`: a
/// body that stands there overlaps an edge exit beside the end of a floor.
fn nearest_the_zone(graph: &NavGraph, zone: ae::Aabb) -> Option<ae::Vec2> {
    let gap = |feet: ae::Vec2| {
        let body = ae::Aabb::new(feet - graph.frame.down * graph.half.y, graph.half);
        let dx = (zone.min.x - body.max.x).max(body.min.x - zone.max.x);
        let dy = (zone.min.y - body.max.y).max(body.min.y - zone.max.y);
        // Apart: how far. Overlapping: less than zero, by how much, so a
        // body in the zone beats one that only touches its edge.
        if dx > 0.0 || dy > 0.0 {
            dx.max(0.0).hypot(dy.max(0.0))
        } else {
            dx * dy * -1.0
        }
    };
    (0..graph.surfaces.len())
        .map(|index| graph.point_on(index, graph.frame.along(zone.center())))
        .min_by(|a, b| gap(*a).total_cmp(&gap(*b)))
}

#[test]
#[ignore = "a measurement: run with --ignored --nocapture"]
fn walked_route_census() {
    let mut sim = fixed_60hz_room_sim("central_hub_complex");
    sim.step_n(base(), 10);
    let (player, spawn) = the_player(&mut sim);
    let mut one_jump = player.clone();
    one_jump.abilities.abilities.double_jump = false;
    let mut no_wall = player.clone();
    no_wall.abilities.abilities.wall_cling = false;
    no_wall.abilities.abilities.wall_climb = false;
    eprintln!("WALKED abilities={:?}", player.abilities.abilities);
    let frame = ae::MotionFrame::from_acceleration(ae::Vec2::new(0.0, ae::movement::GRAVITY)).expect("gravity");
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(sim.world())
    .expect("the session keeps its room set")
    .clone();
    let exit = |from: &str, to: &str| {
        let definition = rooms.definition_by_id(from).expect("an authored room");
        rooms
            .spec(definition)
            .loading_zones
            .iter()
            .find_map(|zone| {
                rooms
                    .transition_for_player(definition, zone.aabb, ae::Vec2::ZERO, true)
                    .filter(|transition| rooms.rooms[transition.target_room].id == to)
                    .map(|transition| (zone.aabb, transition.arrival))
            })
            .unwrap_or_else(|| panic!("`{from}` has no exit to `{to}`"))
    };
    let mut start = spawn;
    let mut routed = [0; 3];
    for (from, to) in CROSSINGS {
        let world = &rooms.rooms[rooms.definition_by_id(from).expect("an authored room").index()].world;
        let (zone, arrival) = exit(from, to);
        // The other exits that fire on overlap take the body to another
        // room: a leg through one is no leg.
        let exits: Vec<ae::Aabb> = rooms
            .spec(rooms.definition_by_id(from).expect("an authored room"))
            .loading_zones
            .iter()
            .filter(|other| other.is_ready(false) && other.aabb != zone)
            .map(|other| other.aabb)
            .collect();
        let mut row = Vec::new();
        for (column, (name, body)) in [("player", &player), ("no air jump", &one_jump), ("no wall verb", &no_wall)].into_iter().enumerate() {
            let graph = NavGraph::build_avoiding(world, &exits, body, frame).expect("the room builds a graph");
            let feet = on_a_surface(&graph, start);
            let goal = nearest_the_zone(&graph, zone);
            let route = feet.zip(goal).and_then(|(feet, goal)| graph.route(feet, goal));
            let kinds: Vec<_> = route
                .iter()
                .flatten()
                .map(|link| format!("{:?}", graph.links[*link].leg.kind))
                .collect();
            if route.is_some() {
                routed[column] += 1;
            }
            // Why a crossing has no route: where the body starts, the goal,
            // and the highest surface (least `top`) the body gets to.
            if route.is_none() && name == "player" {
                let from = feet.and_then(|feet| graph.surface_at(feet));
                let reached = from.map(|from| graph.reachable_from(from)).unwrap_or_default();
                let highest = reached.iter().map(|index| graph.surfaces[*index].top).fold(f32::INFINITY, f32::min);
                eprintln!("  WHY start {feet:?} goal {goal:?} reaches {} surfaces, the highest top {highest:.0}", reached.len());
            }
            row.push(format!(
                "{name}: {} apex={:.0} legs={kinds:?}",
                if route.is_some() { "ROUTED" } else { "no route" },
                graph.apex_rise
            ));
        }
        eprintln!("WALKED {from} -> {to}: {}", row.join(" | "));
        start = arrival;
    }
    eprintln!(
        "WALKED routed {} of {} (no air jump: {}, no wall verb: {})",
        routed[0],
        CROSSINGS.len(),
        routed[1],
        routed[2]
    );
}
