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
    a_save_that_has_seen_the_hub_intro, base, fixed_60hz_room_options, walk_through_the_door_to,
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
#[test]
fn every_room_occupant_belongs_to_the_live_room_it_was_built_for() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .expect("switch_lab boots");
    settle(&mut sim);

    let mut seen = Vec::new();
    for target in [None, Some(HUB), Some(ROOM)] {
        if let Some(target) = target {
            assert_eq!(walk_through_the_door_to(&mut sim, target), target);
            settle(&mut sim);
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

/// OW1 cut 2: a publication retires the residents of the live room it
/// replaces, and no other live room's.
///
/// The control is stamped with the room being left and is swept. The subject
/// is stamped with a live room this session has not published (#7), which is
/// what a second instance's resident looks like to this road, and it stays,
/// still in #7. Before the sweep was keyed, both were swept.
#[test]
fn a_resident_of_another_live_room_stays_when_this_one_is_replaced() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .expect("switch_lab boots");
    settle(&mut sim);

    let (live, _) = census(&mut sim);
    let elsewhere = (0..7).fold(LiveRoomInstance::ACTIVATION, |room, _| room.next());
    let world = sim.world_mut();
    let control = world.spawn((RoomScopedEntity, InRoomInstance(live))).id();
    let subject = world.spawn((RoomScopedEntity, InRoomInstance(elsewhere))).id();

    assert_eq!(walk_through_the_door_to(&mut sim, HUB), HUB);
    settle(&mut sim);

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

/// The unstamped-resident arm of `InRoomInstance::leaves_with` is exact only
/// while no simulated room resident is unstamped. This holds that population at
/// zero over every room the shipped world authors, after the room loads and
/// after the body has run and attacked through it (drops, projectiles, split
/// offspring).
///
/// ⚠ The sample is what running right and attacking reaches in the headless
/// harness: no thrown item, portal shot or match item, and no presentation.
#[test]
fn every_room_resident_carries_its_live_room_after_combat() {
    let rooms: Vec<String> = {
        let mut sim = Platformer2dSimHarness::new_with_options(
            fixed_60hz_room_options(HUB).with_save(a_save_that_has_seen_the_hub_intro()),
        )
        .expect("the hub boots");
        session_world_component::<ambition_platformer2d::world::rooms::RoomSet>(sim.world_mut())
            .expect("the session has a room set")
            .rooms
            .iter()
            .map(|room| room.id.clone())
            .collect()
    };
    assert!(rooms.len() > 50, "only {} rooms: not the shipped world", rooms.len());

    let mut stamped = 0;
    let mut unstamped = Vec::new();
    for room in &rooms {
        let Ok(mut sim) = Platformer2dSimHarness::new_with_options(
            fixed_60hz_room_options(room).with_save(a_save_that_has_seen_the_hub_intro()),
        ) else {
            continue;
        };
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
    // 66 over 72 rooms when this was written (2026-09-29). The floor refuses a
    // census that saw nothing, not a room that lost an occupant.
    assert!(stamped >= 50, "only {stamped} stamped residents: the census saw nothing");
    assert!(
        unstamped.is_empty(),
        "{} room resident(s) carry no live room, so a keyed sweep takes them with \
         whichever room is left: {unstamped:?}",
        unstamped.len()
    );
}
