#![cfg(feature = "rl_sim")]
//! NAVIGATION, slice 1: a goal given from outside, in one room.
//!
//! The basement dog is sent for an item by its stable identity (an `Errand`).
//! It goes there by the room's surface graph with its own movement and takes
//! the item by the one take of a ground item, or the errand ends refused with
//! its reason. The item is the hub's Blink; where a refusal needs another
//! place, the Blink is put down there.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::actors::features::ecs::errand::{Errand, ErrandOutcome, ErrandRefusal};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::combat::held_items::HeldItem;
use ambition_platformer2d::held_items::{GroundItem, ItemCustody};
use ambition_platformer2d::platformer::sim_id::SimId;
use ambition_platformer2d::world::navigation::NavGraph;
use bevy::prelude::Entity;

const BLINK: &str = "ground_blink";
/// Long enough for the dog to cross the basement and climb (30 s).
const PATIENCE: u32 = 1800;

fn the_dog_and_the_blink() -> (ambition_app::Platformer2dSimHarness, Entity, Entity) {
    let mut sim = fixed_60hz_room_sim("central_hub_complex");
    sim.step_n(base(), 10);
    let world = sim.world_mut();
    let dog = world
        .query::<(Entity, &WornCharacter)>()
        .iter(world)
        .find(|(_, worn)| worn.id() == "npc_companion_dog")
        .map(|(entity, _)| entity)
        .expect("the basement stages the authored dog");
    let blink = world
        .query::<(Entity, &SimId, &GroundItem)>()
        .iter(world)
        .find(|(_, id, _)| **id == SimId::placement(BLINK))
        .map(|(entity, _, _)| entity)
        .expect("the Blink lies in the hub");
    (sim, dog, blink)
}

/// Step until the dog's errand ends, at most `ticks`.
fn run_errand(sim: &mut ambition_app::Platformer2dSimHarness, dog: Entity, ticks: u32) -> ErrandOutcome {
    for _ in 0..ticks {
        sim.step(base());
        let outcome = sim.world().get::<Errand>(dog).expect("the dog keeps its errand").outcome;
        if outcome != ErrandOutcome::Pending {
            return outcome;
        }
    }
    ErrandOutcome::Pending
}

fn send_for_the_blink(sim: &mut ambition_app::Platformer2dSimHarness, dog: Entity) {
    sim.world_mut().entity_mut(dog).insert(Errand::fetch(SimId::placement(BLINK)));
}

/// The graph the navigation advisor gave the dog on the last tick, and the
/// dog's feet. The advisor's own graph, not a second build of it: the room it
/// reads and the frame it builds under are the advisor's facts.
fn the_dogs_graph(sim: &mut ambition_app::Platformer2dSimHarness, dog: Entity) -> (NavGraph, ae::Vec2) {
    sim.step(base());
    let world = sim.world();
    let in_use: Vec<&NavGraph> = world
        .resource::<ambition_platformer2d::actors::features::ecs::navigation::RoomNavigation>()
        .graphs_in_use()
        .collect();
    assert_eq!(in_use.len(), 1, "the dog is the one body that navigates in the hub");
    let graph = in_use[0].clone();
    let kinematics = world.get::<ae::BodyKinematics>(dog).expect("a body");
    let feet = kinematics.pos + graph.frame.down * graph.half.y;
    (graph, feet)
}

/// Take the dog's jump away, or give it back, the way any source changes a
/// body's verbs: a keyed ceiling in its `AbilityContributions`. The advisor's
/// graph is keyed by the body's abilities, so the next tick's graph is the one
/// for this body.
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

/// Put the Blink down at rest on `surface`, in the middle of it.
fn put_the_blink_on(sim: &mut ambition_app::Platformer2dSimHarness, blink: Entity, graph: &NavGraph, surface: usize) {
    let middle = (graph.surfaces[surface].left + graph.surfaces[surface].right) * 0.5;
    let feet = graph.point_on(surface, middle);
    let mut ground = sim.world_mut().get_mut::<GroundItem>(blink).expect("the Blink lies in the world");
    ground.pos = feet - graph.frame.down * (ground.half_extent.y + 1.0);
    ground.vel = ae::Vec2::ZERO;
}

/// ⭐ SENT FOR THE BLINK, THE DOG TAKES IT. The custody of the one Blink is the
/// authority: it is held by the dog, and the dog's hand holds it.
///
/// The control is the same start with no errand: over the same time the
/// roaming dog never comes near the Blink, so the errand is what took it
/// there (a take can only happen where the dog touches the item).
#[test]
fn a_dog_sent_for_the_blink_takes_it() {
    let (mut roaming, dog, blink) = the_dog_and_the_blink();
    let at = sim_blink_at(&roaming, blink);
    let mut nearest = f32::INFINITY;
    for _ in 0..PATIENCE {
        roaming.step(base());
        let dog_at = roaming.world().get::<ae::BodyKinematics>(dog).expect("the dog").pos;
        nearest = nearest.min(dog_at.distance(at));
    }
    assert!(nearest > 96.0, "control: the roaming dog came within {nearest:.0} px of the Blink with no errand");

    let (mut sent, dog, blink) = the_dog_and_the_blink();
    send_for_the_blink(&mut sent, dog);
    let outcome = run_errand(&mut sent, dog, PATIENCE);
    assert_eq!(outcome, ErrandOutcome::Done, "the dog did not fetch the Blink");
    assert_eq!(
        sent.world().get::<ItemCustody>(blink).copied(),
        Some(ItemCustody::Held { holder: dog }),
        "the errand says done and the Blink is not the dog's"
    );
    assert!(sent.world().get::<HeldItem>(dog).is_some(), "the dog's hand is empty");
}

fn sim_blink_at(sim: &ambition_app::Platformer2dSimHarness, blink: Entity) -> ae::Vec2 {
    sim.world().get::<GroundItem>(blink).expect("the Blink lies in the world").pos
}

/// Put on a surface the dog cannot get to, the Blink is refused with the
/// reason, and stays where it lies.
#[test]
fn a_dog_sent_for_an_item_out_of_its_reach_says_there_is_no_route() {
    let (mut sim, dog, blink) = the_dog_and_the_blink();
    sim.step_n(base(), 60);
    let (graph, feet) = the_dogs_graph(&mut sim, dog);
    let from = graph.surface_at(feet).expect("the dog stands on a surface");
    let reached = graph.reachable_from(from);
    let out_of_reach = (0..graph.surfaces.len())
        .find(|surface| !reached.contains(surface) && graph.surfaces[*surface].width() > 48.0)
        .expect("the hub has a surface the dog cannot reach");
    put_the_blink_on(&mut sim, blink, &graph, out_of_reach);
    sim.step_n(base(), 10);
    send_for_the_blink(&mut sim, dog);
    let outcome = run_errand(&mut sim, dog, PATIENCE);
    assert_eq!(outcome, ErrandOutcome::Refused(ErrandRefusal::NoRoute));
    assert_eq!(sim.world().get::<ItemCustody>(blink).copied(), Some(ItemCustody::InWorld));
}

/// ⭐ THE SAME ITEM, AND THE BODY'S OWN MOVEMENT DECIDES. Put where only a
/// jump gets to, the Blink is fetched by the dog; the same dog without its
/// jump refuses it.
#[test]
fn without_its_jump_the_dog_refuses_what_needs_one() {
    let needs_a_jump = |sim: &mut ambition_app::Platformer2dSimHarness, dog: Entity| {
        let (with, feet) = the_dogs_graph(sim, dog);
        give_the_dog_its_jump(sim, dog, false);
        sim.step_n(base(), 2);
        assert_eq!(
            sim.world().get::<ae::BodyAbilities>(dog).map(|a| a.abilities.jump),
            Some(false),
            "premise: the ceiling took the dog's jump"
        );
        let (without, _) = the_dogs_graph(sim, dog);
        give_the_dog_its_jump(sim, dog, true);
        let reached_with = with.reachable_from(with.surface_at(feet).expect("the dog stands"));
        let reached_without = without.reachable_from(without.surface_at(feet).expect("the dog stands"));
        let surface = reached_with
            .iter()
            .copied()
            .find(|surface| {
                with.surfaces[*surface].width() > 48.0
                    && !reached_without.iter().any(|other| without.surfaces[*other].id == with.surfaces[*surface].id)
            })
            .unwrap_or_else(|| {
                panic!(
                    "no surface in the hub needs the dog's jump: {} reached with it, {} without",
                    reached_with.len(),
                    reached_without.len(),
                )
            });
        (with, surface)
    };
    let fetch = |jump: bool| {
        let (mut sim, dog, blink) = the_dog_and_the_blink();
        sim.step_n(base(), 60);
        let (graph, surface) = needs_a_jump(&mut sim, dog);
        put_the_blink_on(&mut sim, blink, &graph, surface);
        give_the_dog_its_jump(&mut sim, dog, jump);
        let put = sim.world().get::<GroundItem>(blink).expect("the Blink").pos;
        sim.step_n(base(), 10);
        let lies = sim.world().get::<GroundItem>(blink).expect("the Blink").pos;
        assert_eq!(
            sim.world().get::<ae::BodyAbilities>(dog).map(|a| a.abilities.jump),
            Some(jump),
            "premise: the dog's projected verbs follow the ceiling"
        );
        assert!(
            put.distance(lies) < 4.0,
            "premise: the Blink rests where it was put on surface {surface} ({:?}): put {put:?}, lies {lies:?}",
            graph.surfaces[surface]
        );
        send_for_the_blink(&mut sim, dog);
        run_errand(&mut sim, dog, PATIENCE)
    };
    assert_eq!(fetch(true), ErrandOutcome::Done, "with its jump the dog did not fetch the Blink");
    assert_eq!(fetch(false), ErrandOutcome::Refused(ErrandRefusal::NoRoute));
}

/// Give the dog its errand from inside the timeline: a function of the world,
/// so each replay gives it on the same frame. An errand inserted from outside
/// the timeline is undone by the first rewind past it, as any state is.
fn send_the_dog_from_inside_the_timeline(
    mut commands: bevy::prelude::Commands,
    dogs: bevy::prelude::Query<(Entity, &WornCharacter), bevy::prelude::Without<Errand>>,
) {
    for (dog, worn) in &dogs {
        if worn.id() == "npc_companion_dog" {
            commands.entity(dog).insert(Errand::fetch(SimId::placement(BLINK)));
        }
    }
}

/// ⭐ AN ERRAND RESIMULATES TO THE SAME WORLD. A sync test rewinds and replays
/// each frame and compares checksums. The take lands inside those windows: if
/// the errand were not rewound with the item's custody, a replay past the
/// take would find the errand done and the Blink on the ground again, and the
/// replayed world would differ. The premise: the errand ended, with the Blink
/// in the dog's custody.
#[test]
fn a_dog_on_an_errand_resimulates_to_the_same_world() {
    use crate::common::fixed_60hz_room_options;
    use ambition_platformer2d::sim::SimScheduleExt;

    let options = fixed_60hz_room_options("central_hub_complex").with_sync_test_rollback_settings(4, 10);
    let mut sim = ambition_app::Platformer2dSimHarness::build(options, |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        let label = app.sim_schedule();
        app.add_systems(label, send_the_dog_from_inside_the_timeline);
        Ok(())
    })
    .expect("the hub boots under a sync test");
    let (dog, blink) = {
        let world = sim.world_mut();
        let dog = world
            .query::<(Entity, &WornCharacter)>()
            .iter(world)
            .find(|(_, worn)| worn.id() == "npc_companion_dog")
            .map(|(entity, _)| entity)
            .expect("the dog");
        let blink = world
            .query::<(Entity, &SimId)>()
            .iter(world)
            .find(|(_, id)| **id == SimId::placement(BLINK))
            .map(|(entity, _)| entity)
            .expect("the Blink");
        (dog, blink)
    };
    let mut outcome = ErrandOutcome::Pending;
    let mut after_the_take = 0;
    for _ in 0..PATIENCE + 120 {
        if sim.try_step(base()).is_err() {
            break;
        }
        outcome = sim.world().get::<Errand>(dog).map_or(outcome, |errand| errand.outcome);
        if outcome == ErrandOutcome::Done {
            // Replays of the frames around the take.
            after_the_take += 1;
            if after_the_take > 30 {
                break;
            }
        }
    }
    assert_eq!(
        (outcome, sim.world().get::<ItemCustody>(blink).copied()),
        (ErrandOutcome::Done, Some(ItemCustody::Held { holder: dog })),
        "premise: the errand ended with the Blink in the dog's custody"
    );
    assert_eq!(
        (
            ambition_platformer2d::rollback::session_is_active(sim.world()),
            ambition_platformer2d::rollback::session_health(sim.world())
        ),
        (true, Ok(()))
    );
}

/// The basement dog and the Blink in a harness the caller built.
fn the_dog_and_the_blink_in(sim: &mut ambition_app::Platformer2dSimHarness) -> (Entity, Entity) {
    let world = sim.world_mut();
    let dog = world
        .query::<(Entity, &WornCharacter)>()
        .iter(world)
        .find(|(_, worn)| worn.id() == "npc_companion_dog")
        .map(|(entity, _)| entity)
        .expect("the basement stages the authored dog");
    let blink = world
        .query::<(Entity, &SimId, &GroundItem)>()
        .iter(world)
        .find(|(_, id, _)| **id == SimId::placement(BLINK))
        .map(|(entity, _, _)| entity)
        .expect("the Blink lies in the hub");
    (dog, blink)
}

/// The live room an entity is stamped with.
fn room_of(sim: &ambition_app::Platformer2dSimHarness, entity: Entity) -> Option<ambition_platformer2d::platformer::lifecycle::LiveRoomInstance> {
    sim.world().get::<ambition_platformer2d::platformer::lifecycle::InRoomInstance>(entity).map(|room| room.0)
}

const HUB: &str = "central_hub_complex";
const NEXT_DOOR: &str = "basement_npcs";

/// The hub and `basement_npcs` both live: Bob (slot 1) holds the hub beside
/// the dog, and Alice carries the Blink through the basement's door to
/// `basement_npcs` and puts it down there. Returns the harness, the dog, the
/// Blink, the hub and Alice's room.
fn the_blink_next_door() -> (
    ambition_app::Platformer2dSimHarness,
    Entity,
    Entity,
    ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
    ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
) {
    use crate::common::{a_save_that_has_seen_the_hub_intro, fixed_60hz_room_options};
    use ambition_app::rl_sim::AmbitionSim as _;
    let sim = ambition_app::Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(HUB).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .expect("the hub boots");
    put_the_blink_next_door(sim)
}

/// [`the_blink_next_door`], in a harness the caller built.
fn put_the_blink_next_door(
    mut sim: ambition_app::Platformer2dSimHarness,
) -> (
    ambition_app::Platformer2dSimHarness,
    Entity,
    Entity,
    ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
    ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
) {
    use crate::common::walk_through_the_door_to;
    let hub = crate::two_players_two_live_rooms::bob_beside_alice(
        &mut sim,
        HUB,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
    );
    let (dog, blink) = the_dog_and_the_blink_in(&mut sim);
    // Alice takes the Blink: a press of Attack where it lies.
    let at = sim_blink_at(&sim, blink);
    sim.teleport_player((at.x, at.y - 8.0));
    for _ in 0..20 {
        sim.step(ambition_app::AgentAction { attack: true, attack_held: true, ..base() });
        sim.step(base());
        if sim.world().get::<ItemCustody>(blink).is_some_and(|custody| !custody.in_world()) {
            break;
        }
    }
    assert!(
        sim.world().get::<ItemCustody>(blink).is_some_and(|custody| !custody.in_world()),
        "premise: Alice took the Blink"
    );
    assert_eq!(walk_through_the_door_to(&mut sim, NEXT_DOOR), NEXT_DOOR);
    for _ in 0..40 {
        sim.step(ambition_app::AgentAction { move_x: 1.0, ..base() });
    }
    sim.step_frame(ae::ControlFrame { grab_pressed: true, ..Default::default() });
    sim.step_n(base(), 30);
    let alices_room = crate::two_players_two_live_rooms::where_they_are(&mut sim).0.expect("Alice's room");
    let live: Vec<String> =
        crate::two_players_two_live_rooms::live_rooms(&mut sim).into_iter().map(|(_, id)| id).collect();
    assert_eq!(live, vec![HUB.to_string(), NEXT_DOOR.to_string()], "premise: the hub and Alice's room are live");
    assert_eq!(
        (sim.world().get::<ItemCustody>(blink).copied(), room_of(&sim, blink)),
        (Some(ItemCustody::InWorld), Some(alices_room)),
        "premise: the Blink lies in Alice's room"
    );
    assert_eq!(room_of(&sim, dog), Some(hub), "premise: the dog is in the hub");
    (sim, dog, blink, hub, alices_room)
}

/// Move the dog to `at`, at rest.
fn put_the_dog_at(sim: &mut ambition_app::Platformer2dSimHarness, dog: Entity, at: ae::Vec2) {
    let world = sim.world_mut();
    let mut bodies = world.query::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel)>();
    let (mut clusters, mut model) = bodies.get_mut(world, dog).expect("the dog is a body");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

/// The west end of the basement floor: the door to `basement_npcs` is about
/// 1350 px east of it, on the same floor.
const FAR_FROM_THE_DOOR: ae::Vec2 = ae::Vec2::new(150.0, 1920.0);

/// ⭐ SENT FOR AN ITEM IN ANOTHER LIVE ROOM, THE DOG GOES THROUGH THE DOOR AND
/// TAKES IT. The route from the hub to `basement_npcs` is one door; the dog
/// walks to it with its own movement from the far end of the basement, comes
/// out in Alice's room and takes the Blink there. The authorities: the
/// Blink's custody and the dog's room stamp.
///
/// The control is the same start with no errand: over the same time the
/// roaming dog never comes near the door, so the errand is what took it
/// there.
#[test]
fn a_dog_sent_for_an_item_in_another_live_room_goes_through_the_door() {
    let (mut roaming, dog, _blink, hub, _alices_room) = the_blink_next_door();
    let door = crate::two_players_two_live_rooms::door_of(&mut roaming, HUB, NEXT_DOOR).aabb;
    let door_at = ae::Vec2::new((door.min.x + door.max.x) * 0.5, (door.min.y + door.max.y) * 0.5);
    put_the_dog_at(&mut roaming, dog, FAR_FROM_THE_DOOR);
    let mut nearest = f32::INFINITY;
    for _ in 0..PATIENCE {
        roaming.step(base());
        let dog_at = roaming.world().get::<ae::BodyKinematics>(dog).expect("the dog").pos;
        nearest = nearest.min(dog_at.distance(door_at));
    }
    assert!(
        nearest > 200.0 && room_of(&roaming, dog) == Some(hub),
        "control: the roaming dog came within {nearest:.0} px of the door with no errand"
    );

    let (mut sim, dog, blink, _hub, alices_room) = the_blink_next_door();
    put_the_dog_at(&mut sim, dog, FAR_FROM_THE_DOOR);
    send_for_the_blink(&mut sim, dog);
    let outcome = run_errand(&mut sim, dog, PATIENCE);
    assert_eq!(
        (outcome, room_of(&sim, dog)),
        (ErrandOutcome::Done, Some(alices_room)),
        "the dog did not fetch the Blink from the other room"
    );
    assert_eq!(sim.world().get::<ItemCustody>(blink).copied(), Some(ItemCustody::Held { holder: dog }));
}

/// ⭐ A DOG THAT WENT INTO ANOTHER ROOM IS NOT AUTHORED AGAIN AT HOME. After
/// the dog crosses, Bob leaves the hub too, so the hub retires; Alice walks
/// back and the hub is built again. The dog lives in `basement_npcs`, so the
/// hub must not build a second one. The authority: one body of the dog's
/// identity, in the room it went to. Its ledger row is what the hub's build
/// reads.
#[test]
fn a_dog_that_went_next_door_is_not_built_again_at_home() {
    use crate::common::walk_through_the_door_to;
    let (mut sim, dog, _blink, _hub, alices_room) = the_blink_next_door();
    let identity = sim.world().get::<SimId>(dog).cloned().expect("the dog has a stable identity");
    send_for_the_blink(&mut sim, dog);
    assert_eq!(run_errand(&mut sim, dog, PATIENCE), ErrandOutcome::Done, "premise: the dog fetched the Blink");
    // Bob goes through the hub's door to Alice's room: nobody holds the hub.
    let door = crate::two_players_two_live_rooms::door_of(&mut sim, HUB, NEXT_DOOR).aabb;
    crate::two_players_two_live_rooms::put_bob_at(&mut sim, ae::Vec2::new((door.min.x + door.max.x) * 0.5, (door.min.y + door.max.y) * 0.5));
    for _ in 0..120 {
        sim.drive_seat(1, ae::ControlFrame { interact_pressed: true, interact_held: true, ..Default::default() });
        sim.step(base());
        if crate::two_players_two_live_rooms::where_they_are(&mut sim).1 == Some(Some(alices_room)) {
            break;
        }
    }
    sim.drive_seat(1, ae::ControlFrame::default());
    sim.step_n(base(), 30);
    let live: Vec<String> =
        crate::two_players_two_live_rooms::live_rooms(&mut sim).into_iter().map(|(_, id)| id).collect();
    assert_eq!(live, vec![NEXT_DOOR.to_string()], "premise: with nobody in it, the hub retired");
    assert_eq!(walk_through_the_door_to(&mut sim, HUB), HUB, "premise: Alice walks back to the hub");
    sim.step_n(base(), 30);
    let live: std::collections::BTreeSet<String> =
        crate::two_players_two_live_rooms::live_rooms(&mut sim).into_iter().map(|(_, id)| id).collect();
    assert_eq!(
        live,
        [HUB.to_string(), NEXT_DOOR.to_string()].into_iter().collect(),
        "premise: the hub is built again"
    );
    let world = sim.world_mut();
    let dogs: Vec<_> = world
        .query::<(Entity, &SimId)>()
        .iter(world)
        .filter(|(_, id)| **id == identity)
        .map(|(entity, _)| entity)
        .collect();
    let rooms: Vec<_> = dogs.iter().map(|dog| room_of(&sim, *dog)).collect();
    assert_eq!(rooms, vec![Some(alices_room)], "the bodies of the dog's identity, by room");
}

/// Send the dog for the Blink from inside the timeline when the Blink lies in
/// another room than the dog's: a function of the world, so each replay gives
/// the errand on the same frame.
fn send_the_dog_next_door_from_inside_the_timeline(
    mut commands: bevy::prelude::Commands,
    dogs: bevy::prelude::Query<
        (Entity, &WornCharacter, Option<&ambition_platformer2d::platformer::lifecycle::InRoomInstance>),
        bevy::prelude::Without<Errand>,
    >,
    items: bevy::prelude::Query<(&SimId, &ItemCustody, Option<&ambition_platformer2d::platformer::lifecycle::InRoomInstance>)>,
) {
    let Some(blink_room) = items
        .iter()
        .find(|(id, custody, _)| **id == SimId::placement(BLINK) && custody.in_world())
        .and_then(|(_, _, room)| room.copied())
    else {
        return;
    };
    for (dog, worn, room) in &dogs {
        if worn.id() == "npc_companion_dog" && room.is_some_and(|room| *room != blink_room) {
            commands.entity(dog).insert(Errand::fetch(SimId::placement(BLINK)));
        }
    }
}

/// ⭐ AN ERRAND THROUGH A DOOR RESIMULATES TO THE SAME WORLD. Under a sync test
/// each frame is rewound and replayed and the checksums compared. The
/// crossing and the take land inside those windows: the dog's room stamp,
/// its pose and the ledger row that the crossing writes must come back with
/// a rewind, or the replayed world differs. The premise: the errand ended in
/// Alice's room, with the Blink in the dog's custody.
#[test]
fn a_dog_that_goes_through_a_door_on_an_errand_resimulates_to_the_same_world() {
    use crate::common::{a_save_that_has_seen_the_hub_intro, fixed_60hz_room_options};
    use ambition_platformer2d::sim::SimScheduleExt;

    let options = fixed_60hz_room_options(HUB)
        .with_save(a_save_that_has_seen_the_hub_intro())
        .with_sync_test_rollback_settings(4, 10)
        .with_rollback_players(2);
    let sim = ambition_app::Platformer2dSimHarness::build(options, |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        let label = app.sim_schedule();
        app.add_systems(label, send_the_dog_next_door_from_inside_the_timeline);
        Ok(())
    })
    .expect("the hub boots under a sync test");
    let (mut sim, dog, blink, _hub, alices_room) = put_the_blink_next_door(sim);
    let mut outcome = ErrandOutcome::Pending;
    let mut after_the_take = 0;
    for _ in 0..PATIENCE {
        if sim.try_step(base()).is_err() {
            break;
        }
        outcome = sim.world().get::<Errand>(dog).map_or(outcome, |errand| errand.outcome);
        if outcome == ErrandOutcome::Done {
            after_the_take += 1;
            if after_the_take > 30 {
                break;
            }
        }
    }
    // The subject first: a replay that differs from the first run is a
    // desync, and the session says so before the outcome is read.
    assert_eq!(
        (
            ambition_platformer2d::rollback::session_is_active(sim.world()),
            ambition_platformer2d::rollback::session_health(sim.world())
        ),
        (true, Ok(())),
        "the replayed world differs from the world that ran first"
    );
    assert_eq!(
        (outcome, room_of(&sim, dog), sim.world().get::<ItemCustody>(blink).copied()),
        (ErrandOutcome::Done, Some(alices_room), Some(ItemCustody::Held { holder: dog })),
        "premise: the errand ended in Alice's room with the Blink in the dog's custody"
    );
}
