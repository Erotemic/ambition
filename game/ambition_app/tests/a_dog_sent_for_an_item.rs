#![cfg(feature = "rl_sim")]
//! NAVIGATION: a goal given from outside.
//!
//! The basement dog is sent for an item by its stable identity (an `Errand`).
//! It goes there by the room's surface graph with its own movement and takes
//! the item by the one take of a ground item, or the errand ends refused with
//! its reason. The item is the hub's Blink; where a refusal needs another
//! place, the Blink is put down there. Where the Blink lies in another room,
//! the dog goes through the door into it when it is live, or leaves the live
//! world through the ledger when nobody holds it, and both crossings resimulate
//! to the same world under a sync test.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::actors::body::errand::Errand;
use ambition_platformer2d::actors::body::errand::ErrandOutcome;
use ambition_platformer2d::actors::body::errand::ErrandRefusal;
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
        .resource::<ambition_platformer2d::actors::body::navigation::RoomNavigation>()
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
    let mut ground = sim.world_mut().get_mut::<ae::BodyKinematics>(blink).expect("the Blink lies in the world");
    ground.pos = feet - graph.frame.down * (ground.size().y * 0.5 + 1.0);
    ground.vel = ae::Vec2::ZERO;
}

fn sim_blink_at(sim: &ambition_app::Platformer2dSimHarness, blink: Entity) -> ae::Vec2 {
    sim.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(blink).expect("the Blink lies in the world").pos
}

/// ⭐ THE SAME ITEM, AND THE BODY'S OWN MOVEMENT DECIDES. Put where only a
/// jump gets to, the Blink is fetched by the dog: the errand ends done, the
/// Blink in the dog's custody and hand. The same dog without its jump refuses
/// it with the reason (no route), and the Blink stays where it lies.
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
        let put = sim.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(blink).expect("the Blink").pos;
        sim.step_n(base(), 10);
        let lies = sim.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(blink).expect("the Blink").pos;
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
        let outcome = run_errand(&mut sim, dog, PATIENCE);
        (
            outcome,
            sim.world().get::<ItemCustody>(blink).copied(),
            sim.world().get::<HeldItem>(dog).is_some(),
            dog,
        )
    };
    let (outcome, custody, in_hand, dog) = fetch(true);
    assert_eq!(outcome, ErrandOutcome::Done, "with its jump the dog did not fetch the Blink");
    assert_eq!(
        custody,
        Some(ItemCustody::Held { holder: dog }),
        "the errand says done and the Blink is not the dog's"
    );
    assert!(in_hand, "the errand says done and the dog's hand is empty");
    let (outcome, custody, in_hand, _) = fetch(false);
    assert_eq!(
        outcome,
        ErrandOutcome::Refused(ErrandRefusal::NoRoute),
        "without its jump the dog did not refuse the Blink for want of a route"
    );
    assert_eq!(custody, Some(ItemCustody::InWorld), "the refused Blink does not stay where it lies");
    assert!(!in_hand, "the dog that refused holds something");
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
    use crate::common::fixed_60hz_room_options;
    use ambition_app::rl_sim::AmbitionSim as _;
    let sim = ambition_app::Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(HUB),
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
    // The dog's errand is the subject; the doors cross at once so its
    // crossings and the two rooms do not wait on a passage.
    crate::common::doors_cross_at_once(&mut sim);
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
/// ⭐ AND THE HUB DOES NOT BUILD IT AGAIN. After the dog crosses, Bob leaves
/// the hub too, so the hub retires; Alice walks back and the hub is built
/// again. The dog lives in `basement_npcs`, so the hub must not build a second
/// one. The authority: one body of the dog's identity, in the room it went to.
/// Its ledger row is what the hub's build reads.
///
/// The control is the same start with no errand: over the time the errand
/// took, the roaming dog never goes into the door's zone and stays in the hub,
/// so the errand is what took it through.
#[test]
fn a_dog_sent_for_an_item_in_another_live_room_goes_through_the_door() {
    use crate::common::walk_through_the_door_to;
    let (mut sim, dog, blink, _hub, alices_room) = the_blink_next_door();
    let identity = sim.world().get::<SimId>(dog).cloned().expect("the dog has a stable identity");
    put_the_dog_at(&mut sim, dog, FAR_FROM_THE_DOOR);
    send_for_the_blink(&mut sim, dog);
    let mut outcome = ErrandOutcome::Pending;
    let mut took = 0;
    while took < PATIENCE && outcome == ErrandOutcome::Pending {
        sim.step(base());
        took += 1;
        outcome = sim.world().get::<Errand>(dog).expect("the dog keeps its errand").outcome;
    }
    assert_eq!(
        (outcome, room_of(&sim, dog)),
        (ErrandOutcome::Done, Some(alices_room)),
        "the dog did not fetch the Blink from the other room"
    );
    assert_eq!(
        sim.world().get::<ItemCustody>(blink).copied(),
        Some(ItemCustody::Held { holder: dog }),
        "the errand says done and the Blink is not the dog's"
    );

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
    assert_eq!(rooms, vec![Some(alices_room)], "the hub built the dog again: the bodies of the dog's identity, by room");

    // THE CONTROL, over the ticks the errand took.
    let (mut roaming, dog, _blink, hub, _alices_room) = the_blink_next_door();
    let door = crate::two_players_two_live_rooms::door_of(&mut roaming, HUB, NEXT_DOOR).aabb;
    put_the_dog_at(&mut roaming, dog, FAR_FROM_THE_DOOR);
    // The door acts on a body in its zone, so the control asks for that and
    // not for a distance: the roaming dog climbs the ledge over the door
    // (192 px from its middle), and it never goes in.
    let mut in_the_door = None;
    for tick in 0..took {
        roaming.step(base());
        let body = *roaming.world().get::<ae::BodyKinematics>(dog).expect("the dog");
        let (half, near) = (body.size() * 0.5, body.pos);
        let overlaps = near.x + half.x > door.min.x
            && near.x - half.x < door.max.x
            && near.y + half.y > door.min.y
            && near.y - half.y < door.max.y;
        if overlaps {
            in_the_door = Some((tick, near));
            break;
        }
    }
    assert!(
        in_the_door.is_none() && room_of(&roaming, dog) == Some(hub),
        "control: the roaming dog went into the door's zone with no errand: {in_the_door:?}"
    );
}

/// ⛔ A GATE PORTAL THAT IS SHUT FOR THE PLAYER IS SHUT FOR THE ERRAND. The
/// basement's door to `basement_npcs` is made a gate portal whose switch is
/// off: the dog on its errand goes into the door's zone and waits there, in
/// the hub. Then the switch is turned on: the portal boots, and the dog goes
/// through and takes the Blink. The authorities: the portal's phase (what
/// the player's press reads) and the dog's room stamp.
///
/// The control is the arm above: the same door with no portal on it lets
/// the dog through.
#[test]
fn a_dog_on_an_errand_waits_at_a_shut_gate_portal() {
    use ambition_platformer2d::world::rooms::{GatePortalPhases, GatePortalRegistry};
    const SWITCH: &str = "a_dog_sent_for_an_item.npc_door_switch";
    let (mut sim, dog, blink, hub, alices_room) = the_blink_next_door();
    let door = crate::two_players_two_live_rooms::door_of(&mut sim, HUB, NEXT_DOOR);
    sim.world_mut()
        .resource_mut::<GatePortalRegistry>()
        .try_register(door.id.clone(), SWITCH, "no_sprite", "no_ring")
        .expect("the basement's door is no portal yet");
    put_the_dog_at(&mut sim, dog, FAR_FROM_THE_DOOR);
    send_for_the_blink(&mut sim, dog);
    // Until the dog has waited in the door's zone for longer than a crossing
    // takes, so a shut portal that let it through would have done so.
    let mut ticks_in_the_door = 0;
    for _ in 0..PATIENCE {
        sim.step(base());
        let dog_box = {
            let world = sim.world();
            world.get::<ae::BodyKinematics>(dog).expect("the dog").aabb()
        };
        if ae::AabbExt::strict_intersects(dog_box, door.aabb) {
            ticks_in_the_door += 1;
            if ticks_in_the_door > 90 {
                break;
            }
        }
    }
    assert!(
        !sim.world().resource::<GatePortalPhases>().allows_traversal(&door.id),
        "premise: the portal is shut while its switch is off"
    );
    assert_eq!(
        (sim.world().get::<Errand>(dog).map(|errand| errand.outcome), room_of(&sim, dog)),
        (Some(ErrandOutcome::Pending), Some(hub)),
        "the dog went through a gate portal that is shut"
    );
    assert!(ticks_in_the_door > 60, "premise: the dog waited in the door's zone ({ticks_in_the_door} ticks)");

    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_switch(SWITCH, true);
    let outcome = run_errand(&mut sim, dog, PATIENCE);
    assert_eq!(
        (outcome, room_of(&sim, dog)),
        (ErrandOutcome::Done, Some(alices_room)),
        "the dog did not go through the portal once it was on"
    );
    assert_eq!(sim.world().get::<ItemCustody>(blink).copied(), Some(ItemCustody::Held { holder: dog }));
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
    use crate::common::fixed_60hz_room_options;
    use ambition_platformer2d::sim::SimScheduleExt;

    let options = fixed_60hz_room_options(HUB)
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

/// The ledger row of `id`, if any.
fn whereabouts_of(
    sim: &ambition_app::Platformer2dSimHarness,
    id: &SimId,
) -> Option<ambition_platformer2d::platformer::lifecycle::OccurrenceWhereabouts> {
    sim.world()
        .resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
        .whereabouts(id)
        .cloned()
}

/// The live rooms the bodies of `id` are stamped with.
fn bodies_of(sim: &mut ambition_app::Platformer2dSimHarness, id: &SimId) -> Vec<Option<String>> {
    let live = crate::two_players_two_live_rooms::live_rooms(sim);
    let world = sim.world_mut();
    let mut bodies: Vec<_> = world
        .query::<(&SimId, Option<&ambition_platformer2d::platformer::lifecycle::InRoomInstance>)>()
        .iter(world)
        .filter(|(sim_id, _)| *sim_id == id)
        .map(|(_, room)| {
            room.and_then(|room| live.iter().find(|(instance, _)| *instance == room.0).map(|(_, name)| name.clone()))
        })
        .collect();
    bodies.sort();
    bodies
}

/// Alice alone: she carries the Blink through the basement's door, puts it
/// down in `basement_npcs`, and walks back to the hub, so `basement_npcs`
/// retires and the Blink lies in a room that is not live, where its ledger
/// row puts it. Returns the harness and the dog.
fn the_blink_in_a_room_nobody_holds() -> (ambition_app::Platformer2dSimHarness, Entity) {
    use crate::common::fixed_60hz_room_options;
    use ambition_app::rl_sim::AmbitionSim as _;
    let sim = ambition_app::Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(HUB),
    )
    .expect("the hub boots");
    leave_the_blink_in_a_room_nobody_holds(sim)
}

/// [`the_blink_in_a_room_nobody_holds`], in a harness the caller built.
fn leave_the_blink_in_a_room_nobody_holds(
    mut sim: ambition_app::Platformer2dSimHarness,
) -> (ambition_app::Platformer2dSimHarness, Entity) {
    use crate::common::walk_through_the_door_to;
    // The dog's errand is the subject; the doors cross at once so its
    // crossings and the two rooms do not wait on a passage.
    crate::common::doors_cross_at_once(&mut sim);
    sim.step_n(base(), 10);
    let (dog, blink) = the_dog_and_the_blink_in(&mut sim);
    let at = sim_blink_at(&sim, blink);
    sim.teleport_player((at.x, at.y - 8.0));
    for _ in 0..20 {
        sim.step(ambition_app::AgentAction { attack: true, attack_held: true, ..base() });
        sim.step(base());
        if sim.world().get::<ItemCustody>(blink).is_some_and(|custody| !custody.in_world()) {
            break;
        }
    }
    assert_eq!(walk_through_the_door_to(&mut sim, NEXT_DOOR), NEXT_DOOR, "premise: Alice carries the Blink next door");
    for _ in 0..40 {
        sim.step(ambition_app::AgentAction { move_x: 1.0, ..base() });
    }
    sim.step_frame(ae::ControlFrame { grab_pressed: true, ..Default::default() });
    sim.step_n(base(), 30);
    assert_eq!(walk_through_the_door_to(&mut sim, HUB), HUB, "premise: Alice walks back to the hub");
    sim.step_n(base(), 30);
    let live: Vec<String> =
        crate::two_players_two_live_rooms::live_rooms(&mut sim).into_iter().map(|(_, id)| id).collect();
    assert_eq!(live, vec![HUB.to_string()], "premise: with nobody in it, `basement_npcs` retired");
    assert!(
        matches!(
            whereabouts_of(&sim, &SimId::placement(BLINK)),
            Some(ambition_platformer2d::platformer::lifecycle::OccurrenceWhereabouts::Placed { room, .. }) if room == NEXT_DOOR
        ),
        "premise: the ledger puts the Blink in `basement_npcs`: {:?}",
        whereabouts_of(&sim, &SimId::placement(BLINK))
    );
    let dog = sim
        .world()
        .get_entity(dog)
        .ok()
        .map(|_| dog)
        .unwrap_or_else(|| the_dog_and_the_blink_in_hub_only(&mut sim));
    (sim, dog)
}

/// The dog, found again after the hub was built again.
fn the_dog_and_the_blink_in_hub_only(sim: &mut ambition_app::Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    world
        .query::<(Entity, &WornCharacter)>()
        .iter(world)
        .find(|(_, worn)| worn.id() == "npc_companion_dog")
        .map(|(entity, _)| entity)
        .expect("the hub stages the authored dog")
}

/// ⭐ SENT FOR AN ITEM IN A ROOM NOBODY HOLDS, THE DOG LEAVES FOR IT. The dog
/// walks to the door and leaves the live world through the ledger: no body of
/// its identity is live, and its row puts it in `basement_npcs`. When Alice
/// goes there, the room builds the dog beside the Blink; when she comes back,
/// the hub does not build it again. The authorities: the ledger row, and the
/// bodies of the dog's identity in each live room.
///
/// The control that a dog with no errand stays clear of the door from the same
/// place is the one in
/// [`a_dog_sent_for_an_item_in_another_live_room_goes_through_the_door`].
#[test]
fn a_dog_sent_for_an_item_in_a_room_nobody_holds_leaves_for_it() {
    use crate::common::walk_through_the_door_to;
    use ambition_platformer2d::platformer::lifecycle::OccurrenceWhereabouts;

    let (mut sim, dog) = the_blink_in_a_room_nobody_holds();
    let identity = sim.world().get::<SimId>(dog).cloned().expect("the dog has a stable identity");
    put_the_dog_at(&mut sim, dog, FAR_FROM_THE_DOOR);
    send_for_the_blink(&mut sim, dog);
    for _ in 0..PATIENCE {
        sim.step(base());
        if sim.world().get_entity(dog).is_err() {
            break;
        }
    }
    assert_eq!(bodies_of(&mut sim, &identity), Vec::<Option<String>>::new(), "the dog did not leave the live world");
    assert!(
        matches!(whereabouts_of(&sim, &identity), Some(OccurrenceWhereabouts::Placed { ref room, .. }) if room == NEXT_DOOR),
        "the dog's row: {:?}",
        whereabouts_of(&sim, &identity)
    );
    assert_eq!(walk_through_the_door_to(&mut sim, NEXT_DOOR), NEXT_DOOR);
    sim.step_n(base(), 30);
    assert_eq!(
        bodies_of(&mut sim, &identity),
        vec![Some(NEXT_DOOR.to_string())],
        "`basement_npcs` did not build the dog that went there"
    );
    assert_eq!(walk_through_the_door_to(&mut sim, HUB), HUB);
    sim.step_n(base(), 30);
    assert_eq!(bodies_of(&mut sim, &identity), Vec::<Option<String>>::new(), "the hub built the dog again");
}

/// Send the dog for the Blink from inside the timeline when the Blink lies in
/// no live room.
fn send_the_dog_away_from_inside_the_timeline(
    mut commands: bevy::prelude::Commands,
    dogs: bevy::prelude::Query<(Entity, &WornCharacter), bevy::prelude::Without<Errand>>,
    items: bevy::prelude::Query<&SimId, bevy::prelude::With<GroundItem>>,
    ledger: bevy::prelude::Res<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>,
) {
    let blink = SimId::placement(BLINK);
    let away = !items.iter().any(|id| *id == blink)
        && matches!(
            ledger.whereabouts(&blink),
            Some(ambition_platformer2d::platformer::lifecycle::OccurrenceWhereabouts::Placed { .. })
        );
    if !away {
        return;
    }
    for (dog, worn) in &dogs {
        if worn.id() == "npc_companion_dog" {
            commands.entity(dog).insert(Errand::fetch(blink.clone()));
        }
    }
}

/// ⭐ A DOG THAT LEAVES THE LIVE WORLD RESIMULATES TO THE SAME WORLD. Under a
/// sync test the frames around the departure are rewound and replayed: the
/// despawned body comes back with a rewind, and the ledger row with it. The
/// premise: the dog left, and its row puts it in `basement_npcs`.
#[test]
fn a_dog_that_leaves_for_a_room_nobody_holds_resimulates_to_the_same_world() {
    use crate::common::fixed_60hz_room_options;
    use ambition_platformer2d::platformer::lifecycle::OccurrenceWhereabouts;
    use ambition_platformer2d::sim::SimScheduleExt;

    let options = fixed_60hz_room_options(HUB)
        .with_sync_test_rollback_settings(4, 10);
    let sim = ambition_app::Platformer2dSimHarness::build(options, |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        let label = app.sim_schedule();
        app.add_systems(label, send_the_dog_away_from_inside_the_timeline);
        Ok(())
    })
    .expect("the hub boots under a sync test");
    let (mut sim, dog) = leave_the_blink_in_a_room_nobody_holds(sim);
    let identity = sim.world().get::<SimId>(dog).cloned().expect("the dog has a stable identity");
    let mut after = 0;
    for _ in 0..PATIENCE {
        if sim.try_step(base()).is_err() {
            break;
        }
        if bodies_of(&mut sim, &identity).is_empty() {
            after += 1;
            if after > 30 {
                break;
            }
        }
    }
    assert_eq!(
        (
            ambition_platformer2d::rollback::session_is_active(sim.world()),
            ambition_platformer2d::rollback::session_health(sim.world())
        ),
        (true, Ok(())),
        "the replayed world differs from the world that ran first"
    );
    assert!(
        bodies_of(&mut sim, &identity).is_empty()
            && matches!(whereabouts_of(&sim, &identity), Some(OccurrenceWhereabouts::Placed { ref room, .. }) if room == NEXT_DOOR),
        "premise: the dog left for `basement_npcs`: bodies {:?}, row {:?}",
        bodies_of(&mut sim, &identity),
        whereabouts_of(&sim, &identity)
    );
}
