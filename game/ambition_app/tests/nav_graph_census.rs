#![cfg(feature = "rl_sim")]
//! A MEASUREMENT of the navigation graph on every shipped room, for two bodies:
//! a walker with the default tuning, and the hub's dog (its basement level) as it is
//! spawned (its own `MotionModel`, abilities and box, read off the live body).
//!
//! Run it with
//! `cargo test -p ambition_app --test app_it nav_graph_census -- --ignored --nocapture`.
//! It prints one row per room and body: blocks, surfaces, rollouts, links,
//! the surfaces in the largest group that can reach each other, and the build
//! time. It asserts only that each room builds, so it measures, not gates.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::world::navigation::NavGraph;
use bevy::prelude::Entity;

use crate::common::{base, fixed_60hz_room_sim};

/// The dog as the hub basement spawns it.
fn the_dog(sim: &mut ambition_app::Platformer2dSimHarness) -> ae::BodyClusterScratch {
    let world = sim.world_mut();
    let dog: Entity = world
        .query::<(Entity, &ambition_platformer2d::characters::actor::WornCharacter)>()
        .iter(world)
        .find(|(_, worn)| worn.id() == "npc_companion_dog")
        .map(|(entity, _)| entity)
        .expect("the basement stages the dog");
    let model = world.get::<ae::MotionModel>(dog).expect("the dog has a movement law").clone();
    let abilities = world.get::<ae::BodyAbilities>(dog).expect("the dog has abilities").abilities;
    let size = world.get::<ae::BodyKinematics>(dog).expect("the dog has a body").size;
    let mut body = ae::BodyClusterScratch::new_with_abilities(ae::Vec2::ZERO, abilities);
    body.model = model;
    body.kinematics.size = size;
    body.base_size.base_size = size;
    body
}

fn walker() -> ae::BodyClusterScratch {
    ae::BodyClusterScratch::new_with_abilities(
        ae::Vec2::ZERO,
        ae::AbilitySet { move_horizontal: true, jump: true, ..ae::AbilitySet::NONE },
    )
}

/// The size of the largest set of surfaces that can each reach every other.
fn largest_group(graph: &NavGraph) -> usize {
    let n = graph.surfaces.len();
    let reach: Vec<Vec<bool>> = (0..n)
        .map(|from| {
            let mut row = vec![false; n];
            for to in graph.reachable_from(from) {
                row[to] = true;
            }
            row
        })
        .collect();
    (0..n)
        .map(|a| (0..n).filter(|b| reach[a][*b] && reach[*b][a]).count())
        .max()
        .unwrap_or(0)
}

#[test]
#[ignore = "a measurement: run with --ignored --nocapture"]
fn nav_graph_census_of_the_shipped_rooms() {
    let mut sim = fixed_60hz_room_sim("central_hub_complex");
    sim.step_n(base(), 10);
    let bodies = [("walker", walker()), ("dog", the_dog(&mut sim))];
    let frame = ae::MotionFrame::from_acceleration(ae::Vec2::new(0.0, ae::movement::GRAVITY)).expect("gravity");
    let rooms: Vec<(String, ae::World)> = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(sim.world())
    .expect("the session keeps its room set")
    .rooms
    .iter()
    .map(|room| (room.id.clone(), room.world.clone()))
    .collect();
    for (name, body) in &bodies {
        eprintln!(
            "CENSUS body={name} size={:?} model={:?}",
            body.kinematics.size,
            body.model.spec()
        );
    }
    let mut slowest = (String::new(), 0.0_f64);
    for (room, world) in &rooms {
        for (name, body) in &bodies {
            let started = std::time::Instant::now();
            let graph = NavGraph::build(world, body, frame).expect("the room builds a graph");
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            if ms > slowest.1 {
                slowest = (format!("{room}/{name}"), ms);
            }
            eprintln!(
                "CENSUS {room:<28} {name:<6} blocks={:<4} surfaces={:<4} rollouts={:<6} steps={:<7} failed={:<7} links={:<5} cost={:<8.2} group={:<4} ms={ms:.1}",
                world.blocks.len(),
                graph.surfaces.len(),
                graph.cost.rollouts,
                graph.cost.steps,
                graph.cost.failed_steps,
                graph.links.len(),
                graph.links.iter().map(|link| link.cost).sum::<f32>(),
                largest_group(&graph),
            );
        }
    }
    eprintln!("CENSUS rooms={} slowest={} {:.1} ms", rooms.len(), slowest.0, slowest.1);
}
