//! OW1 cut 6d: two players in one world, each in its own live room.
//!
//! Alice (the primary slot) goes through a door while Bob (slot 1) stays.
//! The room Bob is in stays live and whole, and the room Alice arrives in is
//! a second live room. This is the Alice/Bob customer driven end to end
//! through the shipped app, where cut 6c proved only the publication. When
//! Alice comes back, she joins the live room Bob holds (cut 6e). Who
//! crossed is the participant the crossing recorded when it was accepted,
//! not the one who drives the body when the crossing commits.

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

/// Send Alice through the door to `target` as `walk_through_the_door_to`
/// does, but take her slot off her body once the crossing is accepted and
/// before it commits: the window in which possession can move control.
fn walk_through_the_door_losing_her_slot(sim: &mut Platformer2dSimHarness, target: &str) -> String {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let before = sim.observation().active_room.clone();
    let door = crate::common::door_to(sim, target);
    let center = door.aabb.center();
    sim.teleport_player((center.x, center.y));
    let pending = |sim: &Platformer2dSimHarness| {
        sim.world()
            .resource::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
            .pending
            .is_some()
    };
    for _ in 0..120 {
        sim.step(ambition_app::AgentAction {
            interact: true,
            interact_held: true,
            ..base()
        });
        if pending(sim) {
            break;
        }
    }
    assert!(pending(sim), "precondition: the crossing to '{target}' was not accepted");
    {
        let world = sim.world_mut();
        let alice = world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("Alice's body is in the world");
        let slot = world
            .entity_mut(alice)
            .take::<ambition_platformer2d::characters::control::DrivingParticipant>();
        assert_eq!(
            slot.map(|slot| slot.0),
            Some(ambition_platformer2d::characters::control::PlayerSlot(0)),
            "precondition: slot 0 did not drive Alice's body when her crossing was accepted"
        );
    }
    for _ in 0..120 {
        let room = sim.step(base()).active_room;
        if room != before {
            return room;
        }
    }
    panic!("Alice's crossing from '{before}' to '{target}' never committed");
}

/// Build the world with Bob beside Alice in `switch_lab`, driven by `slot`
/// or by nobody, and send Alice through the door to the hub.
fn alice_leaves_bob(
    slot: Option<ambition_platformer2d::characters::control::PlayerSlot>,
) -> (Platformer2dSimHarness, LiveRoomInstance) {
    alice_leaves_bob_by(slot, walk_through_the_door_to)
}

/// [`alice_leaves_bob`], with Alice crossing by `cross`.
fn alice_leaves_bob_by(
    slot: Option<ambition_platformer2d::characters::control::PlayerSlot>,
    cross: fn(&mut Platformer2dSimHarness, &str) -> String,
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
    assert_eq!(cross(&mut sim, HUB), HUB);
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

/// OW1 cut 6e: Alice comes back through the hub's door to `switch_lab`,
/// where Bob is. She joins his live room, #0, and the hub (#1), which
/// nobody is in now, is retired: one live room, with both of them in it.
/// Before this cut, her crossing built a second live room of `switch_lab`
/// beside Bob's. The publication-level control is
/// `a_crossing_into_a_room_another_player_holds_joins_it`.
#[test]
fn a_player_who_comes_back_joins_the_room_the_other_player_holds() {
    let (mut sim, first) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    assert_eq!(walk_through_the_door_to(&mut sim, ROOM), ROOM);
    for _ in 0..30 {
        sim.step(base());
    }
    assert_eq!(
        (live_rooms(&mut sim), where_they_are(&mut sim)),
        (vec![(first, ROOM.to_string())], (Some(first), Some(Some(first)))),
        "Alice did not join the live room Bob holds"
    );
    let ahead = bob_runs(&mut sim, 1.0);
    assert!(ahead > 1.0, "Bob's slot moved his body {ahead} after Alice joined his room");
}

/// GPT review of OW1 cut 6: slot 0 is taken off Alice's body after her
/// crossing to the hub is accepted and before it commits. The crossing was
/// still slot 0's, so it leaves Bob (slot 1) in `switch_lab`, and that room
/// stays live with him in it, as in
/// `a_door_crossed_by_one_player_leaves_the_other_players_room_live`.
/// Before the intent recorded its participant, the commit asked who drives
/// Alice now, found nobody, and replaced Bob's room. The control is the
/// crossing with her slot kept, in that test.
#[test]
fn a_crossing_is_the_participant_it_was_accepted_for_when_it_opens_a_room() {
    let (mut sim, first) = alice_leaves_bob_by(
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_losing_her_slot,
    );
    let second = first.next();
    assert_eq!(
        (live_rooms(&mut sim), where_they_are(&mut sim)),
        (
            vec![(first, ROOM.to_string()), (second, HUB.to_string())],
            (Some(second), Some(Some(first))),
        ),
        "a crossing that lost its slot before it committed did not leave Bob's room live"
    );
}

/// The Join half: Alice comes back to `switch_lab` and loses her slot
/// between acceptance and commit. Her crossing still joins Bob's live room
/// (#0) and retires the hub, as in
/// `a_player_who_comes_back_joins_the_room_the_other_player_holds`, the
/// control. Before, the commit found no driven subject, joined nothing, and
/// built a second live room of `switch_lab` in place of the hub.
#[test]
fn a_crossing_is_the_participant_it_was_accepted_for_when_it_joins_a_room() {
    let (mut sim, first) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    assert_eq!(walk_through_the_door_losing_her_slot(&mut sim, ROOM), ROOM);
    for _ in 0..30 {
        sim.step(base());
    }
    assert_eq!(
        (live_rooms(&mut sim), where_they_are(&mut sim)),
        (vec![(first, ROOM.to_string())], (Some(first), Some(Some(first)))),
        "a crossing that lost its slot before it committed did not join the live room Bob holds"
    );
}

/// OW1 cut 7e: a boss in one of two live rooms fights, dies and drops its
/// reward chest in its own room. Alice is in the hub (#1) and Bob in
/// `switch_lab` (#0); a mockingbird stands with Alice in #1. It wakes and
/// its fight music plays; killed, it is recorded cleared and its chest
/// stands in #1. The boss driver read the sole live room, so while two rooms
/// were live it did not run: no boss woke, died or dropped anything, in
/// either room. The control is the one-room fight,
/// `defeated_boss_is_recorded_cleared_drops_reward_and_clears_music`.
#[cfg(feature = "rl_sim")]
#[test]
fn a_boss_in_one_of_two_live_rooms_fights_and_drops_its_chest_in_its_own_room() {
    use crate::boss_lifecycle::{boss_cleared, force_kill_boss, music_track, spawn_mockingbird, MOCKINGBIRD_TRACK};
    const BOSS: &str = "ow1_boss";
    let (mut sim, first) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    let second = first.next();
    assert_eq!(live_rooms(&mut sim).len(), 2, "precondition: two rooms are not live");
    spawn_mockingbird(&mut sim, BOSS);
    sim.step(base());
    {
        let world = sim.world_mut();
        let boss = world
            .query::<(bevy::prelude::Entity, &ambition_platformer2d::boss_encounter::BossConfig)>()
            .iter(world)
            .find(|(_, config)| config.id == BOSS)
            .map(|(entity, _)| entity)
            .expect("the boss reached the world");
        world.entity_mut(boss).insert(InRoomInstance(second));
    }
    for _ in 0..15 {
        sim.step(base());
    }
    assert_eq!(
        music_track(&sim).as_deref(),
        Some(MOCKINGBIRD_TRACK),
        "the boss in #1 did not wake while two rooms are live"
    );
    force_kill_boss(&mut sim, BOSS);
    for _ in 0..200 {
        sim.step(base());
    }
    let chests: Vec<_> = {
        let world = sim.world_mut();
        world
            .query_filtered::<Option<&InRoomInstance>, bevy::prelude::With<ambition_platformer2d::combat::components::BossRewardChest>>()
            .iter(world)
            .map(|room| room.map(|room| room.0))
            .collect()
    };
    assert_eq!(
        (boss_cleared(&sim, BOSS), chests, music_track(&sim)),
        (true, vec![Some(second)], None),
        "the boss in #1 was not recorded cleared with one chest in its own room and its music released"
    );
}
