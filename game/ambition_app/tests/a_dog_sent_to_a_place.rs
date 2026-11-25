#![cfg(feature = "rl_sim")]
//! NAVIGATION: a place given from outside (a `Visit`), the planner's "go to".
//!
//! The basement dog is sent to a place in its room. It goes there by the
//! room's surface graph with its own movement and the visit ends arrived with
//! the dog standing there, or ends refused when no route gets there. A visit
//! under a sync test is witnessed by the scientists' bench hand-over
//! (`two_scientists_share_one_bench`); the dog's walk under rollback by
//! `a_dog_sent_for_an_item` and `companion_dog`.

use crate::common::{base, fixed_60hz_room_sim, walk::actor_navigation_graph};
use ambition_platformer2d::actors::body::navigation::RoomNavigation;
use ambition_platformer2d::actors::body::visit::{Visit, VisitOutcome};
use ambition_platformer2d::characters::actor::CharacterForm;
use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
use ambition_platformer2d::engine_core as ae;
use bevy::prelude::Entity;

const HUB: &str = "central_hub_complex";
/// Long enough for the dog to cross the basement and climb (30 s).
const PATIENCE: u32 = 1800;

fn the_dog(sim: &mut ambition_app::Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    world
        .query::<(Entity, &CharacterForm)>()
        .iter(world)
        .find(|(_, worn)| worn.id() == "npc_companion_dog")
        .map(|(entity, _)| entity)
        .expect("the basement stages the authored dog")
}

fn give_the_dog_its_jump(sim: &mut ambition_app::Platformer2dSimHarness, dog: Entity, jump: bool) {
    const SOURCE: &str = "test: the dog without its jump";
    let mut contributions = sim
        .world_mut()
        .get_mut::<ae::AbilityContributions>(dog)
        .expect("a body with an authored kit has contributions");
    if jump {
        contributions.clear(SOURCE);
    } else {
        contributions.set(SOURCE, ae::AbilityContribution::Ceiling(ae::AbilitySet { jump: false, ..ae::AbilitySet::ALL }));
    }
}

/// A place a little above the middle of a surface the dog gets to with its
/// jump and not without it, and the graph's `down`.
fn a_place_that_needs_a_jump() -> (ae::Vec2, ae::Vec2) {
    let mut sim = fixed_60hz_room_sim(HUB);
    sim.step_n(base(), 70);
    let dog = the_dog(&mut sim);
    let (with, feet) = actor_navigation_graph(&mut sim, dog);
    give_the_dog_its_jump(&mut sim, dog, false);
    sim.step_n(base(), 2);
    let (without, _) = actor_navigation_graph(&mut sim, dog);
    let reached_with = with.reachable_from(with.surface_at(feet).expect("the dog stands"));
    let reached_without = without.reachable_from(without.surface_at(feet).expect("the dog stands"));
    let surface = reached_with
        .iter()
        .copied()
        .find(|surface| {
            with.ground.surfaces[*surface].width() > 48.0
                && !reached_without.iter().any(|other| without.ground.surfaces[*other].id == with.ground.surfaces[*surface].id)
        })
        .expect("a surface in the hub needs the dog's jump");
    let middle = (with.ground.surfaces[surface].left + with.ground.surfaces[surface].right) * 0.5;
    (with.point_on(surface, middle) - with.ground.frame.down * 16.0, with.ground.frame.down)
}

/// Send the dog to `place` and step until the visit ends, at most `PATIENCE`.
/// Returns the outcome and the dog's feet then.
fn visit(place: ae::Vec2, down: ae::Vec2, jump: bool) -> (VisitOutcome, ae::Vec2) {
    let mut sim = fixed_60hz_room_sim(HUB);
    sim.step_n(base(), 70);
    let dog = the_dog(&mut sim);
    give_the_dog_its_jump(&mut sim, dog, jump);
    sim.step_n(base(), 2);
    assert_eq!(
        sim.world().get::<ae::BodyAbilities>(dog).map(|a| a.abilities.jump),
        Some(jump),
        "premise: the dog's projected verbs follow the ceiling"
    );
    sim.world_mut().entity_mut(dog).insert(Visit::to(place));
    for _ in 0..PATIENCE {
        sim.step(base());
        let outcome = sim.world().get::<Visit>(dog).expect("the dog keeps its visit").outcome;
        if outcome != VisitOutcome::Pending {
            let kinematics = sim.world().get::<ae::BodyKinematics>(dog).expect("a body");
            return (outcome, kinematics.pos + down * (kinematics.size().y * 0.5));
        }
    }
    (VisitOutcome::Pending, ae::Vec2::NAN)
}

#[test]
fn a_passive_dog_asks_nothing_and_a_direct_visit_needs_no_traversal_build() {
    let mut sim = fixed_60hz_room_sim(HUB);
    sim.step_n(base(), 10);
    let dog = the_dog(&mut sim);
    let Brain::StateMachine(StateMachineCfg::Roam { mut cfg, mut state }) =
        sim.world().get::<Brain>(dog).expect("dog brain").clone()
    else {
        panic!("dog starts with a roaming brain");
    };
    sim.world_mut().entity_mut(dog).insert(Brain::StateMachine(StateMachineCfg::StandStill));
    *sim.world_mut().resource_mut::<RoomNavigation>() = RoomNavigation::default();
    sim.step_n(base(), 65);
    let work = sim.world().resource::<RoomNavigation>();
    assert_eq!((work.questions(), work.ground_preparations(), work.motion_measures(), work.len()), (0, 0, 0, 0));

    let (reference, feet) = actor_navigation_graph(&mut sim, dog);
    let map = &reference.ground;
    let from = map.surface_at(feet).expect("dog stands on a surface");
    let span = &map.surfaces[from];
    let along = map.frame.along(feet);
    let mid = (span.left + span.right) * 0.5;
    let destination = map.point_on(from, along + 96.0_f32.copysign(mid - along));
    assert!(destination.distance(feet) > 64.0);
    assert!(matches!(map.walk(feet, destination), Some(ae::navigation::NavNext::Leg(_))));
    cfg.notice_radius = 0.0;
    cfg.greet_within = 0.0;
    state.until = 60.0;
    state.nav.stop();
    sim.world_mut().entity_mut(dog).insert((
        Brain::StateMachine(StateMachineCfg::Roam { cfg, state }),
        Visit::to(destination - map.frame.down * 16.0),
    ));
    let mut retained_ticks = 0;
    for _ in 0..240 {
        let retained = matches!(sim.world().get::<Brain>(dog),
            Some(Brain::StateMachine(StateMachineCfg::Roam { state, .. })) if state.nav.leg.is_some());
        let before = sim.world().resource::<RoomNavigation>().questions();
        sim.step(base());
        if retained {
            retained_ticks += 1;
            assert_eq!(sim.world().resource::<RoomNavigation>().questions() - before, 1);
        }
        if sim.world().get::<Visit>(dog).unwrap().outcome == VisitOutcome::Arrived {
            break;
        }
    }
    let work = sim.world().resource::<RoomNavigation>();
    let outcome = sim.world().get::<Visit>(dog).unwrap().outcome;
    assert_eq!(outcome, VisitOutcome::Arrived);
    assert!(retained_ticks >= 10);
    assert_eq!((work.ground_preparations(), work.motion_measures(), work.len()), (1, 0, 0));
}

/// ⭐ THE SAME PLACE, AND THE BODY'S OWN MOVEMENT DECIDES. Sent where only a
/// jump gets to, the dog goes there and the visit ends arrived with its feet
/// at the place. The same dog without its jump ends the visit refused, where
/// it stood.
#[test]
fn a_dog_sent_to_a_place_goes_there_or_says_it_cannot() {
    let (place, down) = a_place_that_needs_a_jump();
    let (outcome, feet) = visit(place, down, true);
    assert_eq!(outcome, VisitOutcome::Arrived, "with its jump the dog did not arrive at {place:?}");
    assert!(
        feet.distance(place) < 64.0,
        "the visit says arrived and the dog's feet {feet:?} are far from the place {place:?}"
    );
    let (outcome, feet) = visit(place, down, false);
    assert_eq!(outcome, VisitOutcome::NoRoute, "without its jump the dog did not refuse the place");
    assert!(feet.distance(place) > 64.0, "control: the dog that refused stands at the place anyway ({feet:?})");
}
