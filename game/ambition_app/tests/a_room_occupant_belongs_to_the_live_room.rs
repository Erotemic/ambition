//! Every room occupant says which live room it belongs to (OW1 cut 1).
//!
//! `LiveRoomInstance` on the session root tells two visits to one room apart.
//! A second live instance of a room (the Alice/Bob world) needs each occupant
//! to carry that identity too, because a sweep, a collision or a contact that
//! must stay inside one instance can only ask the occupant. Before this, an
//! occupant said only "room-scoped", and which room was inferred from the
//! moment of the sweep.

use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::platformer::lifecycle::{
    session_world_component, InRoomInstance, LiveRoomInstance, RoomResident, RoomScopedEntity,
    RoomVisual,
};

use crate::common::{
    base, fixed_60hz_room_options, walk_through_the_door_to,
};

const ROOM: &str = "switch_lab";
const HUB: &str = "central_hub_complex";

/// The live room the session is in, and the live room of every constructed
/// resident of the room: the roots a room plan built, which carry a
/// construction transaction.
fn census(sim: &mut Platformer2dSimHarness) -> (LiveRoomInstance, Vec<Option<LiveRoomInstance>>) {
    let world = sim.world_mut();
    let live = *ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<LiveRoomInstance>(world)
        .expect("the session root carries its live room");
    let occupants = world
        .query_filtered::<
            Option<&InRoomInstance>,
            (
                RoomResident,
                bevy::prelude::With<ambition_platformer2d::platformer::construction::TransactionId>,
            ),
        >()
        .iter(world)
        .map(|room| room.map(|room| room.0))
        .collect();
    (live, occupants)
}

/// The live room the driven body stands in.
fn body_room(sim: &mut Platformer2dSimHarness) -> Option<LiveRoomInstance> {
    let world = sim.world_mut();
    world
        .query_filtered::<&InRoomInstance, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .ok()
        .map(|room| room.0)
}

fn settle(sim: &mut Platformer2dSimHarness) {
    for _ in 0..10 {
        sim.step(base());
    }
}

/// The activation room's occupants belong to live room #0, and after each
/// crossing every occupant belongs to the room just published: none is left
/// unstamped, and none still names the room it replaced. Coming back to
/// `switch_lab` is a third live room, not the first one again. The body that
/// crosses moves with each publication: it is never left in the room it left.
///
/// The same session also holds two keyed behaviours, each with its control:
///
/// - OW1 cut 3d: the body step moves each body against the live room it is in.
///   The driven body stamped with a live room this session has not published
///   (#7) has no collision world, so it is not integrated and a held run does
///   not move it. It runs back the way the first control came, so a wall cannot
///   be the reason it stands still. The controls are the same body in its own
///   live room, before and after, running each way.
/// - OW1 cut 2: a publication retires the residents of the live room it
///   replaces, and no other live room's. The control is stamped with the room
///   being left and is swept; the subject is stamped with #7, which is what a
///   second instance's resident looks like to this road, and it stays, still
///   in #7.
#[test]
fn every_room_occupant_belongs_to_the_live_room_it_was_built_for() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM),
    )
    .expect("switch_lab boots");
    settle(&mut sim);
    let (first, _) = census(&mut sim);
    let elsewhere = (0..7).fold(LiveRoomInstance::ACTIVATION, |room, _| room.next());

    let right = run(&mut sim, 1.0);
    assert!(right > 1.0, "control: a held run moved the body {right} in its own live room");
    stamp_the_body(&mut sim, elsewhere);
    let stranded = run(&mut sim, -1.0);
    assert_eq!(
        stranded, 0.0,
        "a body in a room that is not live moved {stranded}: it collided with another live room"
    );
    stamp_the_body(&mut sim, first);
    let left = run(&mut sim, -1.0);
    assert!(left < -1.0, "control: back in its own live room, a held run moved the body {left}");

    let world = sim.world_mut();
    let control = world.spawn((RoomScopedEntity, InRoomInstance(first))).id();
    let subject = world.spawn((RoomScopedEntity, InRoomInstance(elsewhere))).id();

    let mut seen = Vec::new();
    for target in [None, Some(HUB), Some(ROOM)] {
        if let Some(target) = target {
            assert_eq!(walk_through_the_door_to(&mut sim, target), target);
            settle(&mut sim);
        }
        if target == Some(HUB) {
            let world = sim.world_mut();
            assert!(
                world.get_entity(control).is_err(),
                "a resident of the live room that was left survived its replacement"
            );
            assert_eq!(
                world.get::<InRoomInstance>(subject).map(|room| room.0),
                Some(elsewhere),
                "a resident of another live room was retired, or moved, by this room's replacement"
            );
        }
        let (live, occupants) = census(&mut sim);
        assert!(
            !occupants.is_empty(),
            "live room {live} has no constructed occupant, so this proves nothing"
        );
        let strays: Vec<_> = occupants.iter().filter(|room| **room != Some(live)).collect();
        assert!(
            strays.is_empty(),
            "in live room {live}, {} of {} occupants belong to another room or to none: {strays:?}",
            strays.len(),
            occupants.len()
        );
        assert_eq!(
            body_room(&mut sim),
            Some(live),
            "the body that crossed is not in the live room it arrived in"
        );
        seen.push(live);
    }
    assert_eq!(
        seen,
        vec![
            LiveRoomInstance::ACTIVATION,
            LiveRoomInstance::ACTIVATION.next(),
            LiveRoomInstance::ACTIVATION.next().next(),
        ],
        "each publication did not mint the next live room"
    );
}

/// The unstamped-resident arm of `InRoomInstance::leaves_with` is exact only
/// while no simulated room resident is unstamped. This holds that population at
/// zero over a sample of the shipped rooms, after each room loads and after the
/// body has run and attacked through it (drops, projectiles, split offspring).
///
/// ⭐ A STAMP IS GIVEN BY THE ROAD THAT SPAWNS A RESIDENT, NOT BY THE ROOM, so
/// the sample covers spawn families rather than rooms: [`a_room_per_family`]
/// picks rooms until every authored enemy character, boss brain, held item,
/// placement kind and feature family of the shipped world is in one of them.
/// Touring every room cost a time that grew with each room authored.
///
/// One session tours the sample: a departure (`Destination::Room`, the road a
/// level's end takes) brings the body to each in turn, and each arrival is a
/// room loaded and published as at boot. A room the tour cannot reach fails
/// the census: it is not left out.
///
/// ⚠ The sample is what running right and attacking reaches in the headless
/// harness: no thrown item, portal shot or match item, and no presentation.
#[test]
fn every_room_resident_carries_its_live_room_after_combat() {
    use ambition_platformer2d::platformer::lifecycle::{Departure, Destination};
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(HUB),
    )
    .expect("the hub boots");
    let set = session_world_component::<ambition_platformer2d::world::rooms::RoomSet>(sim.world_mut())
        .expect("the session has a room set");
    assert!(set.rooms.len() > 50, "only {} rooms: not the shipped world", set.rooms.len());
    let rooms = a_room_per_family(&set.rooms);
    let leaver = sim.world_mut().spawn(Departure::default()).id();

    let mut stamped = 0;
    let mut unstamped = Vec::new();
    let mut unreached = Vec::new();
    for room in &rooms {
        if sim.observation().active_room != *room {
            sim.world_mut()
                .get_mut::<Departure>(leaver)
                .expect("the departure")
                .leave(Destination::Room(room.clone()));
            let reached = (0..600).any(|_| {
                sim.step(base());
                sim.observation().active_room == *room
            });
            if !reached {
                unreached.push(room.clone());
                continue;
            }
        }
        for tick in 0..150 {
            sim.step(AgentAction {
                attack: tick % 20 < 3,
                move_x: 1.0,
                ..base()
            });
        }
        let world = sim.world_mut();
        for (entity, stamp) in world
            .query_filtered::<(bevy::prelude::Entity, Option<&InRoomInstance>), (RoomResident, bevy::prelude::Without<RoomVisual>)>()
            .iter(world)
        {
            match stamp {
                Some(_) => stamped += 1,
                None => unstamped.push((room.clone(), entity)),
            }
        }
    }
    assert!(unreached.is_empty(), "the tour did not reach {unreached:?}");
    // The floor refuses a census that saw nothing, not a room that lost an
    // occupant.
    assert!(stamped >= 20, "only {stamped} stamped residents over {} rooms: the census saw nothing", rooms.len());
    assert!(
        unstamped.is_empty(),
        "{} room resident(s) carry no live room, so a keyed sweep takes them with \
         whichever room is left: {unstamped:?}",
        unstamped.len()
    );
}

/// The spawn families `room` authors: each enemy character, boss brain, held
/// item and placement kind by name, and each other feature family by kind.
fn families(room: &ambition_platformer2d::world::rooms::RoomSpec) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    out.extend(room.enemy_spawns.iter().map(|enemy| format!("enemy {}", enemy.payload.character_id.0)));
    out.extend(room.boss_spawns.iter().map(|boss| format!("boss {:?}", boss.payload)));
    out.extend(room.ground_items.iter().map(|item| format!("item {}", item.held_item)));
    out.extend(room.placements.iter().map(|placement| {
        let schema = format!("{:?}", placement.schema);
        let kind = schema.split(|c: char| !c.is_alphanumeric()).next().unwrap_or_default().to_string();
        format!("placement {kind}")
    }));
    for (family, present) in [
        ("portal gun", !room.portal_gun_spawns.is_empty()),
        ("shrine", !room.shrines.is_empty()),
        ("moving platform", !room.moving_platforms.is_empty()),
        ("mount", !room.mount_links.is_empty()),
        ("boss succession", !room.boss_successions.is_empty()),
        ("encounter", !room.encounter_triggers.is_empty()),
    ] {
        if present {
            out.insert(family.to_string());
        }
    }
    out
}

/// Rooms, in room-set order, that together author every spawn family of
/// `rooms`: a greedy cover, each pick the room that adds the most families not
/// yet covered.
fn a_room_per_family(rooms: &[ambition_platformer2d::world::rooms::RoomSpec]) -> Vec<String> {
    let each: Vec<std::collections::BTreeSet<String>> = rooms.iter().map(families).collect();
    let mut covered = std::collections::BTreeSet::new();
    let mut picked = std::collections::BTreeSet::new();
    loop {
        let best = (0..rooms.len())
            .filter(|index| !picked.contains(index))
            .map(|index| (each[index].difference(&covered).count(), index))
            .max_by_key(|(adds, index)| (*adds, std::cmp::Reverse(*index)));
        match best {
            Some((adds, index)) if adds > 0 => {
                covered.extend(each[index].iter().cloned());
                picked.insert(index);
            }
            _ => break,
        }
    }
    picked.into_iter().map(|index| rooms[index].id.clone()).collect()
}

/// Hold a run in `direction` for half a second, and return how far the driven
/// body moved along x.
fn run(sim: &mut Platformer2dSimHarness, direction: f32) -> f32 {
    let start = sim.observation().player_pos.0;
    for _ in 0..30 {
        sim.step(AgentAction {
            move_x: direction,
            ..base()
        });
    }
    sim.observation().player_pos.0 - start
}

fn stamp_the_body(sim: &mut Platformer2dSimHarness, room: LiveRoomInstance) {
    let world = sim.world_mut();
    let body = world
        .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .expect("one driven body");
    world.entity_mut(body).insert(InRoomInstance(room));
}
