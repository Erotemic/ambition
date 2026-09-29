//! Every room occupant says which live room it belongs to (OW1 cut 1).
//!
//! `LiveRoomInstance` on the session root tells two visits to one room apart.
//! A second live instance of a room (the Alice/Bob world) needs each occupant
//! to carry that identity too, because a sweep, a collision or a contact that
//! must stay inside one instance can only ask the occupant. Before this, an
//! occupant said only "room-scoped", and which room was inferred from the
//! moment of the sweep.

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::platformer::lifecycle::{
    session_world_component, InRoomInstance, LiveRoomInstance, RoomResident,
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
    let live = *session_world_component::<LiveRoomInstance>(world)
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

fn settle(sim: &mut Platformer2dSimHarness) {
    for _ in 0..10 {
        sim.step(base());
    }
}

/// The activation room's occupants belong to live room #0, and after each
/// crossing every occupant belongs to the room just published: none is left
/// unstamped, and none still names the room it replaced. Coming back to
/// `switch_lab` is a third live room, not the first one again.
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
