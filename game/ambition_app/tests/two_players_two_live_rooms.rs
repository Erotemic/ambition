//! OW1 cut 6d: two players in one world, each in its own live room.
//!
//! Alice (the primary slot) goes through a door while Bob (slot 1) stays.
//! The room Bob is in stays live and whole, and the room Alice arrives in is
//! a second live room. This is the Alice/Bob customer driven end to end
//! through the shipped app, where cut 6c proved only the publication.

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};

use crate::common::{a_save_that_has_seen_the_hub_intro, base, fixed_60hz_room_options, walk_through_the_door_to};

const ROOM: &str = "switch_lab";
const HUB: &str = "central_hub_complex";
const BOB: &str = "ow1_bob";

/// The live rooms, by instance, and the id of the room each instantiates.
fn live_rooms(sim: &mut Platformer2dSimHarness) -> Vec<(LiveRoomInstance, String)> {
    let world = sim.world_mut();
    let definitions: Vec<_> = world
        .query_filtered::<
            (&LiveRoomInstance, &ambition_platformer2d::world::rooms::LiveRoomDefinition),
            bevy::prelude::With<RoomInstanceRoot>,
        >()
        .iter(world)
        .map(|(live, definition)| (*live, *definition))
        .collect();
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(world)
    .expect("the session keeps its room set");
    let mut named: Vec<_> = definitions
        .into_iter()
        .map(|(live, definition)| (live, rooms.spec(definition).id.clone()))
        .collect();
    named.sort();
    named
}

/// The live room each of Alice and Bob is in; `None` for Bob when his body
/// is gone.
fn where_they_are(
    sim: &mut Platformer2dSimHarness,
) -> (Option<LiveRoomInstance>, Option<Option<LiveRoomInstance>>) {
    let world = sim.world_mut();
    let alice = world
        .query_filtered::<&InRoomInstance, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .ok()
        .map(|room| room.0);
    let bob = world
        .query::<(&ambition_platformer2d::combat::components::FeatureId, Option<&InRoomInstance>)>()
        .iter(world)
        .find(|(feature, _)| feature.0 == BOB)
        .map(|(_, room)| room.map(|room| room.0));
    (alice, bob)
}

/// How far Bob's slot runs his body in 30 ticks, holding `direction`.
fn bob_runs(sim: &mut Platformer2dSimHarness, direction: f32) -> f32 {
    let bob_x = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world
            .query::<(&ambition_platformer2d::combat::components::FeatureId, &ambition_platformer2d::engine_core::BodyKinematics)>()
            .iter(world)
            .find(|(feature, _)| feature.0 == BOB)
            .map(|(_, kinematics)| kinematics.pos.x)
            .expect("Bob's body is in the world")
    };
    let start = bob_x(sim);
    for _ in 0..30 {
        sim.drive_seat(
            1,
            ambition_platformer2d::engine_core::ControlFrame {
                axis_x: direction,
                ..Default::default()
            },
        );
        sim.step(base());
    }
    bob_x(sim) - start
}

/// Build the world with Bob beside Alice in `switch_lab`, driven by `slot`
/// or by nobody, and send Alice through the door to the hub.
fn alice_leaves_bob(
    slot: Option<ambition_platformer2d::characters::control::PlayerSlot>,
) -> (Platformer2dSimHarness, LiveRoomInstance) {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .expect("switch_lab boots");
    for _ in 0..10 {
        sim.step(base());
    }
    let first = *ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<LiveRoomInstance>(
        sim.world_mut(),
    )
    .expect("the session has a live room");
    sim.spawn_enemy_character_at(
        BOB,
        "Bob",
        (620.0, 300.0),
        (12.0, 16.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
        "npc_puppy_slug",
    );
    for _ in 0..8 {
        sim.step(base());
    }
    {
        let world = sim.world_mut();
        let bob = world
            .query::<(bevy::prelude::Entity, &ambition_platformer2d::combat::components::FeatureId)>()
            .iter(world)
            .find(|(_, feature)| feature.0 == BOB)
            .map(|(entity, _)| entity)
            .expect("Bob's body reached the world");
        let mut bob = world.entity_mut(bob);
        bob.insert(InRoomInstance(first));
        if let Some(slot) = slot {
            bob.insert(ambition_platformer2d::characters::control::DrivingParticipant(slot));
        }
    }
    assert_eq!(
        where_they_are(&mut sim),
        (Some(first), Some(Some(first))),
        "precondition: Alice and Bob are not both in the first live room"
    );
    if slot.is_some() {
        let ahead = bob_runs(&mut sim, 1.0);
        assert!(ahead > 1.0, "control: slot 1 moved Bob's body {ahead} before anyone left");
    }
    assert_eq!(walk_through_the_door_to(&mut sim, HUB), HUB);
    for _ in 0..30 {
        sim.step(base());
    }
    (sim, first)
}

/// Alice goes through the door to the hub while Bob, driven by slot 1, stays
/// in `switch_lab`. The subject: two live rooms, `switch_lab` (#0) with Bob
/// in it and the hub (#1) with Alice in it, still so 30 ticks later, and
/// Bob's slot still runs his body in #0 (he runs back the way he came
/// before the crossing, so a wall is not why he moves or stands). The
/// control: the same body not driven by any slot. The crossing replaces the
/// room, so the hub is the one live room and the body left in `switch_lab`
/// is retired with it.
#[test]
fn a_door_crossed_by_one_player_leaves_the_other_players_room_live() {
    let (mut sim, first) = alice_leaves_bob(None);
    let second = first.next();
    assert_eq!(
        (live_rooms(&mut sim), where_they_are(&mut sim)),
        (vec![(second, HUB.to_string())], (Some(second), None)),
        "control: a crossing that left no other player behind did not replace the room"
    );

    let (mut sim, first) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    let second = first.next();
    let back = bob_runs(&mut sim, -1.0);
    assert!(back < -1.0, "Bob's slot moved his body {back} in the room Alice left: it is not simulated");
    assert_eq!(
        (live_rooms(&mut sim), where_they_are(&mut sim)),
        (
            vec![(first, ROOM.to_string()), (second, HUB.to_string())],
            (Some(second), Some(Some(first))),
        ),
        "Alice's crossing did not leave Bob's room live with Bob in it"
    );
    assert_eq!(sim.observation().active_room, HUB, "the observation is not Alice's own room");
}
