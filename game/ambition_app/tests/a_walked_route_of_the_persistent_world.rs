#![cfg(feature = "rl_sim")]
//! WORLD-ACCEPTANCE, the walked route: the player goes from the hub to Alice
//! with its own inputs, room by room, and is never put anywhere.
//!
//! In each room the test builds the surface graph for the player's own body
//! (its `MotionModel`, abilities and box), aims at the place where the body
//! overlaps the exit to the next room, and follows the legs with the rule a
//! brain follows them by (`follow_leg`): each leg's input is the player's
//! stick and jump for that step. A door then takes Interact. The crossing
//! after that is the shipped room transition. The placed playthrough
//! (`a_playthrough_of_the_persistent_world`) asserts the facts of the route;
//! this arm asserts that a body gets there by moving.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt as _;
use ambition_platformer2d::engine_core::navigation::{follow_leg, LegFacts, LegPhase, LegProgress, NavLeg, NavNext};
use ambition_platformer2d::world::navigation::NavGraph;
use ambition_app::rl_sim::AmbitionSim as _;

use crate::common::{a_save_that_has_seen_the_hub_intro, base, fixed_60hz_room_options};

const HUB: &str = "central_hub_complex";

/// The rooms from the hub to Alice, each through an exit of the room before:
/// the placed playthrough's route.
const ROUTE_TO_ALICE: &[&str] = &[
    "intro_wake_room",
    "intro_raid_corridor",
    "intro_escape_shaft",
    "drain_alley",
    "under_town_pipes",
    "alice_relay",
];

/// The most steps one room may take to walk (60 s).
const STEPS_FOR_A_ROOM: usize = 3600;

fn the_player_entity(sim: &mut ambition_app::Platformer2dSimHarness) -> bevy::prelude::Entity {
    let world = sim.world_mut();
    world
        .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .expect("one player")
}

/// The player's body as a probe: its movement law, abilities and box.
fn the_players_body(sim: &mut ambition_app::Platformer2dSimHarness) -> ae::BodyClusterScratch {
    let player = the_player_entity(sim);
    let world = sim.world();
    let model = world.get::<ae::MotionModel>(player).expect("a movement law").clone();
    let abilities = world.get::<ae::BodyAbilities>(player).expect("abilities").abilities;
    let size = world.get::<ae::BodyKinematics>(player).expect("a body").size;
    let mut body = ae::BodyClusterScratch::new_with_abilities(ae::Vec2::ZERO, abilities);
    body.model = model;
    body.kinematics.size = size;
    body.base_size.base_size = size;
    body
}

/// The feet point on a surface where the body comes nearest to `zone`, an
/// overlap before a touch: a body that stands there is in an edge exit
/// beside the end of a floor.
fn nearest_the_zone(graph: &NavGraph, zone: ae::Aabb) -> Option<ae::Vec2> {
    let gap = |feet: ae::Vec2| {
        let body = ae::Aabb::new(feet - graph.frame.down * graph.half.y, graph.half);
        let dx = (zone.min.x - body.max.x).max(body.min.x - zone.max.x);
        let dy = (zone.min.y - body.max.y).max(body.min.y - zone.max.y);
        if dx > 0.0 || dy > 0.0 {
            dx.max(0.0).hypot(dy.max(0.0))
        } else {
            -(dx * dy)
        }
    };
    (0..graph.surfaces.len())
        .map(|index| graph.point_on(index, graph.frame.along(zone.center())))
        .min_by(|a, b| gap(*a).total_cmp(&gap(*b)))
}

/// Walk from the live room to `target` by the graph, with the player's own
/// inputs. Returns the steps it took.
fn walk_to(sim: &mut ambition_app::Platformer2dSimHarness, target: &str) -> Result<usize, String> {
    let from = sim.observation().active_room.clone();
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(sim.world())
    .expect("the session keeps its room set")
    .clone();
    let definition = rooms.definition_by_id(&from).expect("the live room is authored");
    let (zone, door) = rooms
        .spec(definition)
        .loading_zones
        .iter()
        .find(|zone| {
            rooms
                .transition_for_player(definition, zone.aabb, ae::Vec2::ZERO, true)
                .is_some_and(|transition| rooms.rooms[transition.target_room].id == target)
        })
        .map(|zone| (zone.aabb, zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door))
        .ok_or_else(|| format!("`{from}` has no exit to `{target}`"))?;
    let frame = ae::MotionFrame::from_acceleration(ae::Vec2::new(0.0, ae::movement::GRAVITY)).expect("gravity");
    let body = the_players_body(sim);
    // Every other exit that fires on overlap takes the body to another room:
    // a leg through one is no leg.
    let exits: Vec<ae::Aabb> = rooms
        .spec(definition)
        .loading_zones
        .iter()
        .filter(|other| other.is_ready(false) && other.aabb != zone)
        .map(|other| other.aabb)
        .collect();
    let graph = NavGraph::build_avoiding(&rooms.spec(definition).world, &exits, &body, frame)
        .ok_or("the room has no graph for the player")?;
    let goal = nearest_the_zone(&graph, zone).ok_or("no surface near the exit")?;
    let player = the_player_entity(sim);
    let mut leg: Option<(NavLeg, LegPhase)> = None;
    for step in 0..STEPS_FOR_A_ROOM {
        if sim.observation().active_room == target {
            return Ok(step);
        }
        let world = sim.world();
        let kinematics = *world.get::<ae::BodyKinematics>(player).expect("the player has a body");
        let on_ground = world.get::<ae::BodyGroundState>(player).is_some_and(|ground| ground.on_ground);
        let feet = kinematics.pos + graph.frame.down * graph.half.y;
        let mut action = base();
        if leg.is_none() && on_ground {
            match graph.next(feet, goal) {
                // At the exit. A door takes a fresh press of Interact. An edge
                // exit takes the body that is in it: the goal is the last
                // place to stand, so the body walks on into the zone, as a
                // player does (the zone can begin past the end of the floor).
                NavNext::Arrived => {
                    action.interact = door && step % 10 == 0;
                    action.interact_held = door && step % 10 < 2;
                    if !door {
                        action.move_x = (zone.center() - feet).dot(graph.frame.side).signum() * 0.45;
                    }
                }
                NavNext::Leg(next) => leg = Some((next, LegPhase::Approach)),
                NavNext::Unreachable => return Err(format!("no route from {feet:?} to {goal:?} in `{from}`")),
                NavNext::Unknown => {}
            }
        }
        if let Some((current, phase)) = leg {
            let facts = LegFacts {
                feet,
                vel: kinematics.vel,
                on_ground,
                side: graph.frame.side,
                down: graph.frame.down,
                dt: 1.0 / 60.0,
            };
            let (input, progress) = follow_leg(&current, phase, &facts);
            // A walk is at a fraction of the top speed, as a brain's pace is;
            // a run-up and a jump are at full speed.
            action.move_x = if input.full_speed { input.axis } else { input.axis * 0.45 };
            // Up is toward -y in the body's local axes.
            action.move_y = if input.up { -1.0 } else { 0.0 };
            action.jump = input.jump_pressed;
            action.jump_held = input.jump_held;
            leg = match progress {
                LegProgress::Going(next) => Some((current, next)),
                LegProgress::Arrived | LegProgress::Failed => None,
            };
        }
        sim.step(action);
    }
    let at = sim.world().get::<ae::BodyKinematics>(player).map(|kinematics| kinematics.pos);
    Err(format!("still in `{from}` after {STEPS_FOR_A_ROOM} steps, at {at:?}, aiming at {goal:?}"))
}

/// ⭐ THE PLAYER WALKS FROM THE HUB TO ALICE. Each crossing is the shipped
/// room transition, and the body got to each exit by its own movement: the
/// live room changes to each room of the route in turn. No step puts the body
/// anywhere.
#[test]
fn the_player_walks_from_the_hub_to_alice() {
    let mut sim = ambition_app::Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(HUB).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .expect("the hub boots");
    sim.step_n(base(), 30);
    for target in ROUTE_TO_ALICE {
        let from = sim.observation().active_room.clone();
        let steps = walk_to(&mut sim, target).unwrap_or_else(|why| panic!("`{from}` to `{target}`: {why}"));
        eprintln!("WALKED `{from}` to `{target}` in {steps} steps");
        sim.step_n(base(), 30);
    }
    assert_eq!(sim.observation().active_room, "alice_relay");
}
