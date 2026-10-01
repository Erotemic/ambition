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
    alice_leaves_bob_in(ROOM, HUB, slot, cross)
}

/// [`alice_leaves_bob_by`], with Alice and Bob in `start` and Alice crossing
/// to `target`.
fn alice_leaves_bob_in(
    start: &str,
    target: &str,
    slot: Option<ambition_platformer2d::characters::control::PlayerSlot>,
    cross: fn(&mut Platformer2dSimHarness, &str) -> String,
) -> (Platformer2dSimHarness, LiveRoomInstance) {
    alice_leaves_bob_with(
        Platformer2dSimHarness::new_with_options(
            fixed_60hz_room_options(start).with_save(a_save_that_has_seen_the_hub_intro()),
        )
        .unwrap_or_else(|error| panic!("{start} boots: {error:?}")),
        start,
        target,
        slot,
        cross,
    )
}

/// [`alice_leaves_bob_in`], in the harness `sim`, booted in `start`.
fn alice_leaves_bob_with(
    mut sim: Platformer2dSimHarness,
    start: &str,
    target: &str,
    slot: Option<ambition_platformer2d::characters::control::PlayerSlot>,
    cross: fn(&mut Platformer2dSimHarness, &str) -> String,
) -> (Platformer2dSimHarness, LiveRoomInstance) {
    assert_eq!(sim.observation().active_room, start, "precondition: the harness did not boot in {start}");
    for _ in 0..10 {
        sim.step(base());
    }
    let first = *ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<LiveRoomInstance>(
        sim.world_mut(),
    )
    .expect("the session has a live room");
    // Bob stands where this test has always put him in `switch_lab`, and
    // beside Alice elsewhere.
    let bob_at = if start == ROOM {
        (620.0, 300.0)
    } else {
        let world = sim.world_mut();
        let alice = world
            .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("Alice's body is in the world")
            .pos;
        (alice.x + 40.0, alice.y)
    };
    sim.spawn_enemy_character_at(
        BOB,
        "Bob",
        bob_at,
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
    // A direct world edit is not a frame a rollback can replay: the history
    // starts again from here. Without rollback, this does nothing.
    sim.rebase_rollback_history().expect("the rollback history rebases over Bob");
    assert_eq!(
        where_they_are(&mut sim),
        (Some(first), Some(Some(first))),
        "precondition: Alice and Bob are not both in the first live room"
    );
    if slot.is_some() {
        let ahead = bob_runs(&mut sim, 1.0);
        assert!(ahead > 1.0, "control: slot 1 moved Bob's body {ahead} before anyone left");
    }
    assert_eq!(cross(&mut sim, target), target);
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

/// The Door from authored room `room` to `target`.
fn door_of(sim: &mut Platformer2dSimHarness, room: &str, target: &str) -> ambition_platformer2d::world::rooms::LoadingZone {
    let world = sim.world_mut();
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(world)
    .expect("the session keeps its room set");
    let definition = rooms.definition_by_id(room).expect("the room is authored");
    rooms
        .spec(definition)
        .loading_zones
        .iter()
        .filter(|zone| zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door)
        .find(|zone| {
            rooms
                .transition_for_player(definition, zone.aabb, ambition_platformer2d::engine_core::Vec2::ZERO, true)
                .and_then(|transition| rooms.rooms.get(transition.target_room))
                .is_some_and(|destination| destination.id == target)
        })
        .cloned()
        .unwrap_or_else(|| panic!("'{room}' has no Door to '{target}'"))
}

/// OW1 cut 7s: Bob, driven by slot 1, goes through a door himself. Alice
/// holds the hub (#1) and Bob holds `switch_lab` (#0); Bob stands in the
/// `switch_lab` door to the hub and his seat presses interact. He joins
/// Alice's live hub, and `switch_lab`, which nobody holds now, retires. The
/// door detector read only the primary seat's body, so Bob's press was
/// buffered and never used: no player but the first could leave a room.
#[test]
fn the_second_player_goes_through_a_door_of_his_own_room() {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let (mut sim, first) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    let second = first.next();
    assert_eq!(
        where_they_are(&mut sim),
        (Some(second), Some(Some(first))),
        "precondition: Alice is not in the hub (#1) with Bob in `switch_lab` (#0)"
    );
    let door = door_of(&mut sim, ROOM, HUB).aabb.center();
    {
        let world = sim.world_mut();
        let mut bob = world.query::<(
            &ambition_platformer2d::combat::components::FeatureId,
            ambition_platformer2d::engine_core::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
        )>();
        let (_, mut clusters, mut model) = bob
            .iter_mut(world)
            .find(|(feature, _, _)| feature.0 == BOB)
            .expect("Bob's body is in the world");
        let mut clusters = clusters.as_clusters_mut();
        ambition_platformer2d::engine_core::movement::transit_body(
            &mut model,
            &mut clusters,
            door,
            ambition_platformer2d::engine_core::movement::TransitVelocity::Zero,
        );
    }
    for _ in 0..120 {
        sim.drive_seat(
            1,
            ambition_platformer2d::engine_core::ControlFrame {
                interact_pressed: true,
                interact_held: true,
                ..Default::default()
            },
        );
        sim.step(base());
        if where_they_are(&mut sim).1 == Some(Some(second)) {
            break;
        }
    }
    // A seat's frame stands until it is replaced: let go of the press, or
    // Bob goes back through the hub's door when the cooldown ends.
    sim.drive_seat(1, ambition_platformer2d::engine_core::ControlFrame::default());
    for _ in 0..30 {
        sim.step(base());
    }
    assert_eq!(
        (live_rooms(&mut sim), where_they_are(&mut sim)),
        (vec![(second, HUB.to_string())], (Some(second), Some(Some(second)))),
        "Bob did not go through the door to join Alice's hub"
    );
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

/// OW1 review of cut 7c: a real wave runs in the live room that triggered it,
/// with another live room beside it.
///
/// Bob, driven by slot 1, stays in the hub (#0) while Alice goes
/// through its basement door to `goblin_encounter` (#1) and stands in its
/// trigger. The subject: the wave spawns, every mob it spawns is stamped
/// #1, and a mob chooses Alice as its target and not Bob, though Bob, a
/// player's body, is put nearer to it. When the encounter runtime knew an
/// encounter only by its authored id, a spawn request named no room: the
/// mobs were stamped into no live room, and with two rooms live they chose
/// no foe at all.
#[test]
fn a_wave_spawns_its_mobs_in_the_live_room_that_started_it() {
    use ambition_platformer2d::combat::components::{ActorTarget, EncounterMob};
    const ARENA: &str = "goblin_encounter";
    let (mut sim, first) = alice_leaves_bob_in(
        HUB,
        ARENA,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    let second = first.next();
    assert_eq!(
        where_they_are(&mut sim),
        (Some(second), Some(Some(first))),
        "precondition: Alice is not in #1 with Bob in #0"
    );
    let trigger = {
        let world = sim.world_mut();
        world
            .query::<(&ambition_platformer2d::encounter::EncounterWaves, &InRoomInstance)>()
            .iter(world)
            .find(|(waves, room)| waves.spec.id == ARENA && room.0 == second)
            .map(|(waves, _)| waves.spec.trigger_aabb())
            .expect("precondition: #1 has no occurrence of the goblin encounter")
    };
    {
        use ambition_platformer2d::engine_core::AabbExt as _;
        let center = trigger.center();
        sim.teleport_player((center.x, center.y));
    }
    let mobs = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        let mut mobs: Vec<_> = world
            .query_filtered::<(bevy::prelude::Entity, Option<&InRoomInstance>), bevy::prelude::With<EncounterMob>>()
            .iter(world)
            .map(|(mob, room)| (mob, room.map(|room| room.0)))
            .collect();
        mobs.sort();
        mobs
    };
    let mut spawned = Vec::new();
    for _ in 0..900 {
        sim.step(base());
        spawned = mobs(&mut sim);
        if !spawned.is_empty() {
            break;
        }
    }
    assert!(!spawned.is_empty(), "the goblin wave in #1 spawned no mob in 900 ticks");
    // Put Bob beside the first mob, nearer to it than Alice is.
    let (mob, _) = spawned[0];
    let (alice, bob) = {
        let world = sim.world_mut();
        let alice = world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("Alice's body is in the world");
        let bob = world
            .query::<(bevy::prelude::Entity, &ambition_platformer2d::combat::components::FeatureId)>()
            .iter(world)
            .find(|(_, feature)| feature.0 == BOB)
            .map(|(entity, _)| entity)
            .expect("Bob's body is in the world");
        (alice, bob)
    };
    let mut targets = Vec::new();
    for _ in 0..30 {
        {
            let world = sim.world_mut();
            let at = world.get::<ambition_platformer2d::engine_core::BodyKinematics>(mob).expect("the mob has a body").pos;
            world
                .get_mut::<ambition_platformer2d::engine_core::BodyKinematics>(bob)
                .expect("Bob has a body")
                .pos = at + ambition_platformer2d::engine_core::Vec2::new(4.0, 0.0);
        }
        sim.step(base());
        let world = sim.world_mut();
        targets = world
            .query_filtered::<&ActorTarget, bevy::prelude::With<EncounterMob>>()
            .iter(world)
            .map(|target| target.entity)
            .collect();
        if targets.iter().any(Option::is_some) {
            break;
        }
    }
    let spawned = mobs(&mut sim);
    assert_eq!(
        (
            spawned.iter().map(|(_, room)| *room).collect::<Vec<_>>(),
            targets.iter().any(|target| *target == Some(alice)),
            targets.iter().any(|target| *target == Some(bob)),
        ),
        (vec![Some(second); spawned.len()], true, false),
        "the wave's mobs are not all in #1, or none targets Alice there, or one targets Bob in #0"
    );
}

/// OW1 review of cut 7e: the cut-rope fight runs whole in its own live room,
/// with another live room beside it.
///
/// Bob, driven by slot 1, stays in `hall_of_bosses` (#0) while Alice goes
/// through its door to `you_have_to_cut_the_rope` (#1). The subject, the
/// real scripted road: the behemoth's wrap is stamped #1 with its script
/// prepared from #1's props; a hit on #1's rope fires `rope_cut` in #1; the
/// behemoth is sent to #1's anvil and walks toward it; the anvil falls as a
/// hazard stamped #1; `cut_rope_impact` fires in #1; the behemoth dies and
/// is recorded cleared; its reward, the victory NPC it releases (it authors
/// no chest), is in #1. When the wrap read the sole
/// live room, it had no props while two rooms were live, its script could
/// not be prepared, and nothing past the wake happened.
#[test]
fn the_cut_rope_fight_runs_in_its_own_live_room() {
    use ambition_platformer2d::boss_encounter::{BossConfig, CommandedMove, EncounterGate, FallingHazard};
    use ambition_platformer2d::combat::components::FeatureId;
    const HALL: &str = "hall_of_bosses";
    const ARENA: &str = "you_have_to_cut_the_rope";
    const BEHEMOTH: &str = "smirking_behemoth_boss";
    let (mut sim, first) = alice_leaves_bob_in(
        HALL,
        ARENA,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    let second = first.next();
    assert_eq!(
        where_they_are(&mut sim),
        (Some(second), Some(Some(first))),
        "precondition: Alice is not in #1 with Bob in #0"
    );
    let prop = |sim: &mut Platformer2dSimHarness, kind: &str| {
        let world = sim.world_mut();
        let definition = ambition_platformer2d::world::rooms::live_room_definition_in(world, Some(second))
            .expect("#1 is live");
        let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::world::rooms::RoomSet,
        >(world)
        .expect("the session keeps its room set");
        let spec = rooms.spec(definition);
        assert_eq!(spec.id, ARENA, "precondition: #1 is not the cut-rope room");
        spec.props
            .iter()
            .find(|prop| prop.kind == kind)
            .map(|prop| prop.pos)
            .unwrap_or_else(|| panic!("#1 authors no `{kind}` prop"))
    };
    let rope = prop(&mut sim, "cut_rope_rope");
    let anvil = prop(&mut sim, "cut_rope_anvil");
    let (behemoth, placement) = {
        let world = sim.world_mut();
        world
            .query::<(bevy::prelude::Entity, &BossConfig)>()
            .iter(world)
            .find(|(_, config)| config.behavior.id == BEHEMOTH)
            .map(|(entity, config)| (entity, config.id.clone()))
            .expect("precondition: #1 has no behemoth")
    };
    // The wrap: the behemoth's id, stamped #1, with its script.
    let wrap = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world
            .query::<(
                &ambition_platformer2d::encounter::Encounter,
                Option<&InRoomInstance>,
                bevy::prelude::Has<ambition_platformer2d::encounter::EncounterScript>,
            )>()
            .iter(world)
            .find(|(encounter, _, _)| encounter.id == placement)
            .map(|(_, room, script)| (room.map(|room| room.0), script))
    };
    let mut wrapped = None;
    for _ in 0..300 {
        sim.step(base());
        wrapped = wrap(&mut sim);
        if wrapped.is_some() {
            break;
        }
    }
    assert_eq!(
        wrapped,
        Some((Some(second), true)),
        "the behemoth's fight is not wrapped in #1 with its script prepared from #1's props"
    );
    {
        use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
        sim.world_mut().write_message(HitEvent {
            volume: ambition_platformer2d::engine_core::Aabb::new(
                rope,
                ambition_platformer2d::engine_core::Vec2::splat(24.0),
            )
            .into(),
            damage: 10,
            source: HitSource::Melee,
            attacker: None,
            room: Some(second),
            target: HitTarget::UnresolvedFeatures,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
            strike_sfx: None,
            attacker_move_instance: None,
        });
    }
    let mut rope_cut = Vec::new();
    let mut impact = Vec::new();
    let mut lure = None;
    let mut start_gap = None;
    let mut closest = f32::MAX;
    let mut hazards = std::collections::BTreeSet::new();
    let mut dropped = false;
    let mut dead = false;
    for _ in 0..1800 {
        sim.step(base());
        let world = sim.world_mut();
        if let Some(gates) = world.get_resource::<bevy::ecs::message::Messages<EncounterGate>>() {
            for gate in gates.iter_current_update_messages() {
                match gate.gate.as_str() {
                    "rope_cut" => rope_cut.push(gate.room),
                    "cut_rope_impact" => impact.push(gate.room),
                    _ => {}
                }
            }
        }
        if let (Some(command), Some(kinematics)) = (
            world.get::<CommandedMove>(behemoth),
            world.get::<ambition_platformer2d::engine_core::BodyKinematics>(behemoth),
        ) {
            lure = Some(command.target);
            let gap = (command.target.x - kinematics.pos.x).abs();
            start_gap.get_or_insert(gap);
            closest = closest.min(gap);
        }
        for (room, hazard) in world
            .query::<(Option<&InRoomInstance>, &FallingHazard)>()
            .iter(world)
        {
            hazards.insert(room.map(|room| room.0));
            dropped |= hazard.dropping;
        }
        dead = world
            .get::<ambition_platformer2d::characters::actor::BodyHealth>(behemoth)
            .is_none_or(|health| !health.alive());
        let released = world
            .query::<&FeatureId>()
            .iter(world)
            .any(|feature| feature.0 == "smirking_behemoth_victory_npc");
        if dead && released {
            break;
        }
    }
    // The death beat runs out before the defeat is recorded.
    for _ in 0..300 {
        sim.step(base());
    }
    let world = sim.world_mut();
    let cleared = matches!(
        world
            .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
            .data()
            .boss(&placement),
        ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared
    );
    let victory_npcs: Vec<_> = world
        .query::<(&FeatureId, Option<&InRoomInstance>)>()
        .iter(world)
        .filter(|(feature, _)| feature.0 == "smirking_behemoth_victory_npc")
        .map(|(_, room)| room.map(|room| room.0))
        .collect();
    let walked = start_gap.is_some_and(|start| closest < start * 0.5 || closest < 24.0);
    assert_eq!(
        (
            rope_cut,
            lure.map(|lure| (lure - anvil).length() < 1.0),
            walked,
            hazards.into_iter().collect::<Vec<_>>(),
            dropped,
            impact,
            dead,
            cleared,
            victory_npcs,
        ),
        (
            vec![Some(second)],
            Some(true),
            true,
            vec![Some(second)],
            true,
            vec![Some(second)],
            true,
            true,
            vec![Some(second)],
        ),
        "the cut-rope road did not run whole in #1: (rope_cut rooms, lured to #1's anvil, \
         walked toward it, hazard rooms, it fell, impact rooms, the behemoth died, it is \
         recorded cleared, victory NPC rooms)"
    );
}

/// OW1 cut 7h, a measurement of the owed rebase question: Alice's crossing
/// and Bob's run, driven through a GGRS sync test with two seats, resimulate
/// to the same checksums. The sync test rolls back and resimulates every
/// frame, across the crossing's confirmed-frame commit and the rebase it
/// asks for, and records a mismatch as a rollback health fault. The result
/// is the same two live rooms, and Bob's slot still runs his body in #0
/// afterwards. The control that the rollback did work is the load count.
#[test]
fn two_players_in_two_live_rooms_resimulate_to_the_same_world() {
    let options = fixed_60hz_room_options(ROOM)
        .with_save(a_save_that_has_seen_the_hub_intro())
        .with_sync_test_rollback_settings(4, 10)
        .with_rollback_players(2);
    let (mut sim, first) = alice_leaves_bob_with(
        Platformer2dSimHarness::new_with_options(options).expect("switch_lab boots under a sync test"),
        ROOM,
        HUB,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    let second = first.next();
    let back = bob_runs(&mut sim, -1.0);
    let loads = sim.rollback_execution_stats().map(|stats| stats.lifetime_load_runs);
    assert!(
        loads.is_some_and(|loads| loads > 0),
        "control: the sync test never rolled back ({loads:?}), so it checked nothing"
    );
    // One assertion: a desync halts the session, so Bob stops too, and the
    // health says why.
    assert_eq!(
        (sim.rollback_health(), back < -1.0, live_rooms(&mut sim), where_they_are(&mut sim)),
        (
            Ok(()),
            true,
            vec![(first, ROOM.to_string()), (second, HUB.to_string())],
            (Some(second), Some(Some(first))),
        ),
        "under rollback, two players in two live rooms did not resimulate to one world: \
         (health, Bob's slot ran his body in #0, live rooms, where they are)"
    );
}

/// The ground item `blink_run` authors: the one lying in the world.
fn blink_run_pickup(sim: &mut Platformer2dSimHarness) -> (bevy::prelude::Entity, ambition_platformer2d::platformer::sim_id::SimId) {
    let world = sim.world_mut();
    let found: Vec<_> = world
        .query::<(bevy::prelude::Entity, &ambition_platformer2d::platformer::sim_id::SimId, &ambition_platformer2d::held_items::ItemCustody)>()
        .iter(world)
        .filter(|(_, _, custody)| custody.in_world())
        .map(|(entity, id, _)| (entity, id.clone()))
        .collect();
    assert_eq!(found.len(), 1, "precondition: `blink_run` does not author exactly one ground item");
    found[0].clone()
}

/// Pick up `blink_run`'s item with the pressed pickup, then walk through the
/// door to `target`.
fn pick_up_and_walk_through_the_door_to(sim: &mut Platformer2dSimHarness, target: &str) -> String {
    let (item, _) = blink_run_pickup(sim);
    let at = sim
        .world()
        .get::<ambition_platformer2d::held_items::GroundItem>(item)
        .expect("the item is a ground item")
        .pos;
    sim.teleport_player((at.x, at.y));
    sim.step(ambition_app::AgentAction { attack: true, ..base() });
    sim.step(base());
    assert!(
        matches!(
            sim.world().get::<ambition_platformer2d::held_items::ItemCustody>(item),
            Some(ambition_platformer2d::held_items::ItemCustody::Held { .. })
        ),
        "precondition: the pressed pickup did not take the item"
    );
    walk_through_the_door_to(sim, target)
}

/// OW2, custody transfer between live rooms: an item carried out of a room
/// another player holds crosses whole, and belongs to the room it is put
/// down in.
///
/// Bob, driven by slot 1, stays in `blink_run` (#0). Alice picks up its
/// authored item and walks to `portal_bridge`, which opens #1. The subject:
/// the item is the same entity and `SimId`, the only one with that id,
/// still in Alice's hands, and stamped #1; #0 stays live. Thrown down in
/// #1, it is in the world, in #1. When Alice walks back into #0 (a join,
/// which retires #1), it goes with #1, and #0 has no copy of it. When the
/// crossing moved only the body, the item in her hands stayed stamped #0.
/// The one-room control is
/// `an_item_carried_through_a_door_survives_and_belongs_to_the_room_it_is_dropped_in`.
#[test]
fn an_item_carried_out_of_a_room_another_player_holds_crosses_whole() {
    use ambition_platformer2d::held_items::ItemCustody;
    use ambition_platformer2d::platformer::sim_id::SimId;
    const SOURCE: &str = "blink_run";
    const TARGET: &str = "portal_bridge";
    let (item, authored) = {
        let mut sim = Platformer2dSimHarness::new_with_options(
            fixed_60hz_room_options(SOURCE).with_save(a_save_that_has_seen_the_hub_intro()),
        )
        .expect("blink_run boots");
        for _ in 0..10 {
            sim.step(base());
        }
        blink_run_pickup(&mut sim)
    };
    let _ = item;
    let (mut sim, first) = alice_leaves_bob_in(
        SOURCE,
        TARGET,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        pick_up_and_walk_through_the_door_to,
    );
    let second = first.next();
    let occurrences = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world
            .query::<(bevy::prelude::Entity, &SimId, Option<&InRoomInstance>, &ItemCustody)>()
            .iter(world)
            .filter(|(_, id, _, _)| **id == authored)
            .map(|(entity, _, room, custody)| (entity, room.map(|room| room.0), *custody))
            .collect::<Vec<_>>()
    };
    let alice = {
        let world = sim.world_mut();
        world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("Alice's body is in the world")
    };
    let carried = occurrences(&mut sim);
    let rooms_after_crossing = live_rooms(&mut sim);

    // Throw it down in #1 (Shield + Attack, the real input).
    sim.step_frame(ambition_platformer2d::engine_core::ControlFrame {
        attack_pressed: true,
        shield_held: true,
        ..Default::default()
    });
    for _ in 0..30 {
        sim.step(base());
    }
    let thrown: Vec<_> = occurrences(&mut sim)
        .into_iter()
        .map(|(_, room, custody)| (room, custody.in_world()))
        .collect();

    // Back into #0, which Bob holds: #1 retires with what lies in it.
    assert_eq!(walk_through_the_door_to(&mut sim, SOURCE), SOURCE);
    for _ in 0..30 {
        sim.step(base());
    }
    let after_return = occurrences(&mut sim);
    assert_eq!(
        (
            carried.iter().map(|(_, room, custody)| (*room, *custody)).collect::<Vec<_>>(),
            rooms_after_crossing,
            thrown,
            after_return,
        ),
        (
            vec![(Some(second), ItemCustody::Held { holder: alice })],
            vec![(first, SOURCE.to_string()), (second, TARGET.to_string())],
            vec![(Some(second), true)],
            Vec::new(),
        ),
        "the carried item did not cross whole into #1 (one occurrence, held, stamped #1, #0 live), \
         or was not put down in #1, or outlived #1 or was copied into #0"
    );
}

/// Alice goes from `hall_of_bosses` (#0), where Bob, driven by slot 1, stays,
/// to `arena` (#1).
/// The boss's live room, and whether its conductor has measured its hall
/// within 120 ticks.
fn a_conducted_boss_beside_bob<C: bevy::prelude::Component>(
    arena: &str,
    hall: fn(&C) -> Option<ambition_content::bosses::hall::Hall>,
) -> (Option<Option<LiveRoomInstance>>, bool, LiveRoomInstance) {
    a_conducted_boss_beside_bob_by(arena, |world| {
        world
            .query::<(&C, Option<&InRoomInstance>)>()
            .iter(world)
            .next()
            .map(|(conductor, room)| (room.map(|room| room.0), hall(conductor).is_some()))
    })
}

/// As [`a_conducted_boss_beside_bob`], with the conducted boss and its hall
/// found by `conducted`: a boss whose conductor is a module keeps its hall in
/// the module's record, not in a component of its own.
fn a_conducted_boss_beside_bob_by(
    arena: &str,
    conducted: impl Fn(&mut bevy::prelude::World) -> Option<(Option<LiveRoomInstance>, bool)>,
) -> (Option<Option<LiveRoomInstance>>, bool, LiveRoomInstance) {
    let (mut sim, first) = alice_leaves_bob_in(
        "hall_of_bosses",
        arena,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    let second = first.next();
    assert_eq!(
        where_they_are(&mut sim),
        (Some(second), Some(Some(first))),
        "precondition: Alice is not in #1 with Bob in #0"
    );
    let mut seen = (None, false);
    for _ in 0..120 {
        sim.step(base());
        seen = conducted(sim.world_mut())
            .map(|(room, measured)| (Some(room), measured))
            .unwrap_or((None, false));
        if seen.1 {
            break;
        }
    }
    (seen.0, seen.1, second)
}

/// OW1 cut 7j: GNU-ton's conductor measures the hall of the scholar's own live
/// room while another room is live. When it read the sole live room, it did
/// not run with two rooms live, and the pair stood still.
#[test]
fn gnu_ton_measures_its_hall_in_its_own_live_room() {
    let (room, measured, second) = a_conducted_boss_beside_bob(
        "gnu_ton_arena",
        ambition_content::bosses::gnu_ton::GnuTonConductor::hall,
    );
    assert_eq!(
        (room, measured),
        (Some(Some(second)), true),
        "(the scholar's live room, its hall measured): GNU-ton was not conducted in #1"
    );
}

/// OW1 cut 7j: the flying spaghetti monster's conductor, the same.
#[test]
fn the_fsm_measures_its_hall_in_its_own_live_room() {
    // The god's conductor is the `fsm` module: its hall is in the module's
    // record on the god.
    let (room, measured, second) = a_conducted_boss_beside_bob_by("flying_spaghetti_monster_arena", |world| {
        let god = world
            .query::<(bevy::prelude::Entity, &ambition_platformer2d::boss_encounter::BossConfig)>()
            .iter(world)
            .find(|(_, config)| config.behavior.id == ambition_content::bosses::fsm::FSM_ID)
            .map(|(god, _)| god)?;
        let room = world.get::<InRoomInstance>(god).map(|room| room.0);
        Some((room, ambition_content::bosses::fsm::conductor_of(world, god).is_some_and(|v| v.hall.is_some())))
    });
    assert_eq!(
        (room, measured),
        (Some(Some(second)), true),
        "(the god's live room, its hall measured): the god was not conducted in #1"
    );
}

/// Spawn a passive body named `id` beside Alice, in live room `room`, and
/// strike it there with the robot's 3-damage slash. The strike cues played.
fn robot_strike_cues(
    sim: &mut Platformer2dSimHarness,
    id: &str,
    room: LiveRoomInstance,
) -> Vec<ambition_platformer2d::sfx::SfxId> {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    use ambition_platformer2d::sfx::{ids, OwnedSfxMessage, SfxMessage};
    let alice = {
        let world = sim.world_mut();
        world
            .query_filtered::<(bevy::prelude::Entity, &ambition_platformer2d::engine_core::BodyKinematics), bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .map(|(entity, kinematics)| (entity, kinematics.pos))
            .expect("Alice's body is in the world")
    };
    sim.spawn_enemy_character_at(
        id,
        id,
        (alice.1.x + 30.0, alice.1.y),
        (12.0, 16.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
        "npc_puppy_slug",
    );
    for _ in 0..4 {
        sim.step(base());
    }
    let victim = {
        let world = sim.world_mut();
        let victim = world
            .query::<(bevy::prelude::Entity, &ambition_platformer2d::combat::components::FeatureId)>()
            .iter(world)
            .find(|(_, feature)| feature.0 == id)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("{id}'s body reached the world"));
        world.entity_mut(victim).insert(InRoomInstance(room));
        victim
    };
    sim.world_mut().write_message(HitEvent {
        volume: ambition_platformer2d::engine_core::Aabb::new(
            alice.1,
            ambition_platformer2d::engine_core::Vec2::splat(24.0),
        )
        .into(),
        damage: 3,
        source: HitSource::Melee,
        attacker: Some(alice.0),
        room: Some(room),
        target: HitTarget::Body(victim),
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
        strike_sfx: Some(ids::PLAYER_ROBOT_SLASH_IMPACT),
        attacker_move_instance: None,
    });
    let mut cues = Vec::new();
    for _ in 0..3 {
        sim.step(base());
        let messages = sim
            .world()
            .resource::<bevy::ecs::message::Messages<OwnedSfxMessage>>();
        cues.extend(messages.iter_current_update_messages().filter_map(|owned| match owned.request {
            SfxMessage::Play { id, .. }
                if [
                    ids::PLAYER_ROBOT_SLASH_IMPACT_FLESH_DEEP,
                    ids::PLAYER_ROBOT_SLASH_IMPACT_FLESH_LIGHT,
                    ids::PLAYER_ROBOT_SLASH_IMPACT_METAL_GONG,
                    ids::PLAYER_ROBOT_SLASH_IMPACT_ROBOT,
                ]
                .contains(&id)
                || id == ids::PLAYER_ROBOT_SLASH_IMPACT =>
            {
                Some(id)
            }
            _ => None,
        }));
    }
    cues
}

/// GPT review of the per-game heavy-hit rule: with Alice and Bob in two
/// Ambition rooms, the robot's 3-damage strike in Alice's room is still
/// heavy. Ambition draws its line at 3. When the rules were resolved from THE
/// live room, two live rooms resolved the rules of no room, Ambition's line
/// was gone, and every robot hit was light. Control: the same strike with one
/// live room. The strike is a written `HitEvent`; the resolution, the room's
/// rules and the cue are the shipped ones.
#[test]
fn a_heavy_robot_strike_stays_heavy_while_another_room_is_live() {
    use ambition_platformer2d::sfx::ids;
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .unwrap_or_else(|error| panic!("{ROOM} boots: {error:?}"));
    for _ in 0..10 {
        sim.step(base());
    }
    let first = *ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<LiveRoomInstance>(
        sim.world_mut(),
    )
    .expect("the session has a live room");
    let one_room = robot_strike_cues(&mut sim, "ow1_struck_alone", first);
    let (mut sim, first) = alice_leaves_bob_with(
        sim,
        ROOM,
        HUB,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    let second = first.next();
    assert_eq!(
        where_they_are(&mut sim),
        (Some(second), Some(Some(first))),
        "precondition: Alice is not in #1 with Bob in #0"
    );
    let two_rooms = robot_strike_cues(&mut sim, "ow1_struck_beside_bob", second);
    assert_eq!(
        (one_room, two_rooms),
        (
            vec![ids::PLAYER_ROBOT_SLASH_IMPACT_FLESH_DEEP],
            vec![ids::PLAYER_ROBOT_SLASH_IMPACT_FLESH_DEEP],
        ),
        "(one live room, Alice's room beside Bob's): the robot's 3-damage strike did not \
         play the heavy cue in both"
    );
}

/// OW1 cut 7l: Alice fires the portal gun in #1 while Bob holds #0. The shot
/// steps against #1's solids and opens a portal in #1, and the portal's host
/// depth is measured in #1. When the shot step read the sole live room, it
/// did not run with two rooms live, and the shot hung in the air.
#[test]
fn a_portal_shot_opens_its_portal_in_the_live_room_it_was_fired_in() {
    let (mut sim, first) = alice_leaves_bob_in(
        ROOM,
        HUB,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    let second = first.next();
    assert_eq!(
        where_they_are(&mut sim),
        (Some(second), Some(Some(first))),
        "precondition: Alice is not in #1 with Bob in #0"
    );
    let alice = {
        let world = sim.world_mut();
        let alice = world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("Alice's body is in the world");
        world.entity_mut(alice).insert(ambition_platformer2d::portal::PortalGun {
            active: true,
            ..ambition_platformer2d::portal::PortalGun::default()
        });
        alice
    };
    sim.world_mut().write_message(ambition_platformer2d::portal::FirePortalGun {
        aim: bevy::math::Vec2::new(0.0, -1.0),
        body: alice,
    });
    let mut opened = None;
    for _ in 0..60 {
        sim.step(base());
        let world = sim.world_mut();
        opened = world
            .query::<(&ambition_platformer2d::portal::PlacedPortal, Option<&InRoomInstance>)>()
            .iter(world)
            .next()
            .map(|(portal, room)| (portal.channel, room.map(|room| room.0)));
        if opened.is_some() {
            break;
        }
    }
    let (channel, room) = opened.expect("Alice's shot opened no portal in 60 ticks");
    // The entry itself: `PortalHostDepths::depth` answers infinity for a
    // portal that was never measured. It is filed under the portal's room.
    sim.step(base());
    let depth = sim
        .world_mut()
        .resource::<ambition_platformer2d::portal::PortalHostDepthsByRoom>()
        .in_room(room)
        .0
        .iter()
        .find(|(measured, _)| *measured == channel)
        .map(|(_, depth)| *depth);
    assert_eq!(
        (room, depth.is_some_and(|depth| depth.is_finite() && depth > 0.0)),
        (Some(second), true),
        "(the portal's live room, a finite host depth measured): the portal was not opened and measured in #1 (depth {depth:?})"
    );
}

/// The live rooms of the NPCs that barked during the next `ticks`, one entry
/// for each bark, in bark order.
fn rooms_that_barked(sim: &mut Platformer2dSimHarness, ticks: usize) -> Vec<Option<LiveRoomInstance>> {
    use ambition_platformer2d::vfx::vfx::VfxMessage;
    let mut rooms = Vec::new();
    for _ in 0..ticks {
        sim.step(base());
        let world = sim.world_mut();
        let barked: Vec<String> = world
            .get_resource_mut::<bevy::prelude::Messages<VfxMessage>>()
            .map(|mut messages| {
                messages
                    .drain()
                    .filter_map(|message| match message {
                        VfxMessage::BarkGesture { feature_id, .. } => Some(feature_id),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        for feature_id in barked {
            let room = world
                .query::<(&ambition_platformer2d::combat::components::FeatureId, Option<&InRoomInstance>)>()
                .iter(world)
                .find(|(feature, _)| feature.0 == feature_id)
                .and_then(|(_, room)| room.map(|room| room.0));
            rooms.push(room);
        }
    }
    rooms
}

/// OW1 cut 7m: an NPC barks at the cadence of its own live room. Bob holds
/// the hub (#0); Alice enters the Hall of Characters (#1), a gallery room,
/// where the pedestals bark from their `Hall` pool, first at 28 s or later.
/// The idle cadence barks first at 20 s or earlier. When the ticker read the
/// sole live room's spec, it had none with two rooms live, so every NPC
/// barked at the idle cadence and the hall spoke before 24 s.
/// The control: the hall does speak, within 60 s.
#[test]
fn a_gallery_pedestal_barks_at_its_own_rooms_cadence_beside_another_live_room() {
    let (mut sim, first) = alice_leaves_bob_in(
        HUB,
        "hall_of_characters",
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    let second = first.next();
    assert_eq!(
        where_they_are(&mut sim),
        (Some(second), Some(Some(first))),
        "precondition: Alice is not in the hall (#1) with Bob in the hub (#0)"
    );
    // 30 ticks have passed since the crossing, so this ends 24 s after it.
    let early = rooms_that_barked(&mut sim, 24 * 60 - 30);
    let early_in_the_hall = early.iter().filter(|room| **room == Some(second)).count();
    let later = rooms_that_barked(&mut sim, 36 * 60);
    let later_in_the_hall = later.iter().filter(|room| **room == Some(second)).count();
    assert_eq!(
        (early_in_the_hall, later_in_the_hall > 0),
        (0, true),
        "(hall barks in the first 24 s, any hall bark from 24 s to 60 s): the hall did not bark at the gallery cadence ({} early barks, rooms {early:?})",
        early.len()
    );
}

/// The settled-sand solids in live room `room`'s collision overlay, and
/// whether `body` has the sand room's swim loan.
#[cfg(feature = "falling_sand")]
fn sand_in(sim: &mut Platformer2dSimHarness, room: LiveRoomInstance, body: bevy::prelude::Entity) -> (usize, bool) {
    let world = sim.world_mut();
    let solids = world
        .query_filtered::<
            (&LiveRoomInstance, &ambition_platformer2d::world::FeatureEcsWorldOverlay),
            bevy::prelude::With<RoomInstanceRoot>,
        >()
        .iter(world)
        .find(|(live, _)| **live == room)
        .map_or(0, |(_, overlay)| overlay.gate_solids.len());
    let swims = world
        .get::<ambition_platformer2d::engine_core::AbilityContributions>(body)
        .is_some_and(|contributions| contributions.get(ambition_content::falling_sand_sim::ROOM_SWIM).is_some());
    (solids, swims)
}

/// OW1 cut 7m: the falling-sand room runs while another room is live. Bob
/// holds the sand room (#0); Alice goes to the hub (#1), and the sand spout
/// opens in #0. The sand settles into #0's collision overlay, and the room's
/// swim loan leaves Alice. When every falling-sand system asked whether the
/// sole live room was the sand room, none of them ran with two rooms live:
/// no settled sand reached any overlay, and Alice kept the loan in the hub.
/// Gated as the sand plugins are: only the `falling_sand` feature adds them.
#[test]
#[cfg(feature = "falling_sand")]
fn the_falling_sand_room_runs_beside_another_live_room() {
    use ambition_content::falling_sand_sim::{FallingSandWorld, ROOM_ID, SAND_SWITCH};
    let (mut sim, first) = alice_leaves_bob_in(
        ROOM_ID,
        HUB,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    let second = first.next();
    assert_eq!(
        where_they_are(&mut sim),
        (Some(second), Some(Some(first))),
        "precondition: Alice is not in the hub (#1) with Bob in the sand room (#0)"
    );
    let alice = {
        let world = sim.world_mut();
        world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("Alice's body is in the world")
    };
    {
        let world = sim.world_mut();
        let activation = world
            .query::<&ambition_platformer2d::encounter::switches::SwitchFeature>()
            .iter(world)
            .map(|feature| feature.activation.clone())
            .find(|activation| activation.id == SAND_SWITCH)
            .unwrap_or_else(|| panic!("authored switch `{SAND_SWITCH}` exists in {ROOM_ID}"));
        world.write_message(ambition_platformer2d::encounter::switches::SwitchActivated {
            activation,
            pos: ambition_platformer2d::engine_core::Vec2::ZERO,
            room: Some(first),
        });
    }
    let mut seen = (0, true);
    for _ in 0..1200 {
        sim.step(base());
        seen = sand_in(&mut sim, first, alice);
        if seen.0 > 0 {
            break;
        }
    }
    let emitted = sim
        .world_mut()
        .get_resource::<FallingSandWorld>()
        .and_then(|sand| sand.grid.as_ref().map(|grid| grid.emitted()))
        .unwrap_or(0);
    assert_eq!(
        (seen.0 > 0, seen.1),
        (true, false),
        "(settled sand in #0's overlay, Alice still has the swim loan in the hub): {emitted} grains emitted"
    );
}

/// Alice goes from `hall_of_bosses` (#0), where Bob, driven by slot 1, stays,
/// to `arena` (#1), untouchable so the fight cannot end her stay, and stands
/// at `place(boss position)`, where its fight suite puts its player, after
/// `prepare` has changed the boss. The live rooms of the first shots the boss
/// `behavior` fires within `ticks` ticks that `shot` knows by their visual
/// and body size, and #1.
fn a_boss_fires_beside_bob(
    arena: &str,
    behavior: &str,
    prepare: fn(&mut ambition_platformer2d::characters::actor::BodyHealth),
    shot: fn(&str, ambition_platformer2d::engine_core::Vec2) -> bool,
    place: fn(ambition_platformer2d::engine_core::Vec2) -> ambition_platformer2d::engine_core::Vec2,
    ticks: usize,
) -> (Vec<Option<LiveRoomInstance>>, LiveRoomInstance) {
    let (mut sim, first) = alice_leaves_bob_in(
        "hall_of_bosses",
        arena,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    let second = first.next();
    assert_eq!(
        where_they_are(&mut sim),
        (Some(second), Some(Some(first))),
        "precondition: Alice is not in #1 with Bob in #0"
    );
    let boss = {
        let world = sim.world_mut();
        let mut alice = world.query_filtered::<
            &mut ambition_platformer2d::characters::actor::BodyHealth,
            bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        for mut health in alice.iter_mut(world) {
            health.health.invulnerable.set(
                ambition_platformer2d::characters::actor::Invulnerability::SCRIPTED,
                true,
            );
        }
        world
            .query::<(bevy::prelude::Entity, &ambition_platformer2d::boss_encounter::BossConfig)>()
            .iter(world)
            .find(|(_, config)| config.behavior.id == behavior)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("precondition: #1 has no {behavior}"))
    };
    prepare(
        &mut sim
            .world_mut()
            .get_mut::<ambition_platformer2d::characters::actor::BodyHealth>(boss)
            .expect("the boss has health"),
    );
    {
        let world = sim.world_mut();
        let at = place(
            world
                .get::<ambition_platformer2d::engine_core::BodyKinematics>(boss)
                .expect("the boss has a body")
                .pos,
        );
        let mut alice = world.query_filtered::<
            (
                ambition_platformer2d::engine_core::BodyClusterQueryData,
                &mut ambition_platformer2d::actor::MotionModel,
            ),
            ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
        >();
        let (mut clusters, mut model) = alice.single_mut(world).expect("Alice's body is in the world");
        let mut clusters = clusters.as_clusters_mut();
        ambition_platformer2d::engine_core::movement::transit_body(
            &mut model,
            &mut clusters,
            at,
            ambition_platformer2d::engine_core::movement::TransitVelocity::Zero,
        );
    }
    let mut rooms = Vec::new();
    for _ in 0..ticks {
        sim.step(base());
        let world = sim.world_mut();
        rooms = world
            .query::<(
                &ambition_platformer2d::projectiles::ProjectileOwner,
                &ambition_platformer2d::projectiles::ProjectileVisualId,
                &ambition_platformer2d::engine_core::BodyKinematics,
                Option<&InRoomInstance>,
            )>()
            .iter(world)
            .filter(|(owner, id, kin, _)| owner.0 == boss && shot(id.as_str(), kin.size))
            .map(|(_, _, _, room)| room.map(|room| room.0))
            .collect();
        if !rooms.is_empty() {
            break;
        }
    }
    rooms.dedup();
    (rooms, second)
}

/// OW1 cut 7n: GNU-ton's apple rain falls in the scholar's own live room
/// while another room is live. When the spawner read the sole live room's
/// width, it did not run with two rooms live, and no apple fell.
#[test]
fn gnu_ton_s_apple_rain_falls_in_its_own_live_room() {
    let (rooms, second) = a_boss_fires_beside_bob(
        "gnu_ton_arena",
        "gnu_ton_rider",
        |_| {},
        |visual, _| visual == "apple",
        // Where `gnu_ton_fight` stands its player for the apple rain.
        |_| ambition_platformer2d::engine_core::Vec2::new(300.0, 1200.0),
        3000,
    );
    assert_eq!(rooms, vec![Some(second)], "the apples' live rooms: the apple rain did not fall in #1");
}

/// OW1 cut 7q: the gnu's back is ground in the giant's own live room. Bob
/// holds the hall of bosses (#0); Alice goes to the gnu_ton arena (#1). The
/// back platform wrote to the sole live room's overlay, so with two rooms
/// live no room had it, and nobody could stand on the giant.
#[test]
fn the_gnu_s_back_is_ground_in_its_own_live_room() {
    let (mut sim, first) = alice_leaves_bob_in(
        "hall_of_bosses",
        "gnu_ton_arena",
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    let second = first.next();
    assert_eq!(
        where_they_are(&mut sim),
        (Some(second), Some(Some(first))),
        "precondition: Alice is not in #1 with Bob in #0"
    );
    sim.step_n(base(), 10);
    let world = sim.world_mut();
    let mut backs: Vec<LiveRoomInstance> = world
        .query_filtered::<
            (&LiveRoomInstance, &ambition_platformer2d::world::FeatureEcsWorldOverlay),
            bevy::prelude::With<RoomInstanceRoot>,
        >()
        .iter(world)
        .flat_map(|(live, overlay)| {
            overlay.blocks.iter().filter(|block| block.name == "gnu_back").map(move |_| *live)
        })
        .collect();
    backs.dedup();
    assert_eq!(backs, vec![second], "the live rooms whose overlay holds the gnu's back");
}

/// OW1 cut 7n: the overflow boss floods its own live room, the same.
#[test]
fn the_overflow_flood_fills_its_own_live_room() {
    let (rooms, second) = a_boss_fires_beside_bob(
        "overflow_arena",
        "overflow_boss",
        // The flood is a phase-2 move, and phase 2 starts below two thirds
        // of the boss's health.
        |health| {
            health.damage(health.max() / 2);
        },
        // A flood column has no visual and is 24 by 28. The overfit volley
        // that comes before it in the pattern also has no visual, but is
        // 16 by 16.
        |visual, size| visual.is_empty() && size == ambition_platformer2d::engine_core::Vec2::new(24.0, 28.0),
        |boss| boss + ambition_platformer2d::engine_core::Vec2::new(-200.0, 0.0),
        3000,
    );
    assert_eq!(rooms, vec![Some(second)], "the flood's live rooms: the flood did not fill #1");
}

/// OW1 cut 7n: the overflow boss swoops through its own live room while
/// another room is live. When the boss's body read the sole live room's
/// walls, it did not move with two rooms live.
#[test]
fn the_overflow_boss_swoops_in_its_own_live_room() {
    let (mut sim, first) = alice_leaves_bob_in(
        "hall_of_bosses",
        "overflow_arena",
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    let second = first.next();
    assert_eq!(
        where_they_are(&mut sim),
        (Some(second), Some(Some(first))),
        "precondition: Alice is not in #1 with Bob in #0"
    );
    let boss_at = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world
            .query::<(
                &ambition_platformer2d::boss_encounter::BossConfig,
                &ambition_platformer2d::engine_core::BodyKinematics,
                Option<&InRoomInstance>,
            )>()
            .iter(world)
            .find(|(config, ..)| config.behavior.id == "overflow_boss")
            .map(|(_, kin, room)| (kin.pos, room.map(|room| room.0)))
            .expect("precondition: #1 has the overflow boss")
    };
    let (start, room) = boss_at(&mut sim);
    assert_eq!(room, Some(second), "precondition: the overflow boss is not in #1");
    for _ in 0..120 {
        sim.step(base());
    }
    let (end, _) = boss_at(&mut sim);
    assert!(
        start.distance(end) > 10.0,
        "the overflow boss stood at {start} and then at {end}: its body did not move in #1"
    );
}

/// OW1 cut 7o: Alice blinks against the walls of her own live room while Bob
/// holds another. The blink read the walls of the sole live room. With two
/// rooms live it had none, so it went the full distance through any wall.
///
/// Bob, driven by slot 1, stays in `blink_run` (#0). Alice carries its blink
/// to `portal_bridge` (#1), stands 40 px left of a solid block of #1 that is
/// taller than she is, and blinks right. Her right side must stop at the
/// block's left side.
#[test]
fn a_blink_stops_at_a_wall_of_its_own_live_room() {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let (mut sim, first) = alice_leaves_bob_in(
        "blink_run",
        "portal_bridge",
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        pick_up_and_walk_through_the_door_to,
    );
    let second = first.next();
    assert_eq!(
        where_they_are(&mut sim),
        (Some(second), Some(Some(first))),
        "precondition: Alice is not in #1 with Bob in #0"
    );
    let alice = {
        let world = sim.world_mut();
        world
            .query_filtered::<bevy::prelude::Entity, ambition_platformer2d::platformer::markers::PrimaryPlayerOnly>()
            .single(world)
            .expect("Alice's body")
    };
    let held = sim
        .world()
        .get::<ambition_platformer2d::combat::held_items::HeldItem>(alice)
        .map(|held| held.spec.id.clone());
    assert_eq!(
        held.as_deref(),
        Some(ambition_platformer2d::abilities::traversal::blink::BLINK_ID),
        "precondition: Alice does not hold the blink"
    );
    let half = sim
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(alice)
        .expect("Alice's body")
        .size
        * 0.5;
    // A block of #1 to stand beside: solid, taller than Alice, with 40 px of
    // clear room on its left for her to stand in.
    let (start, wall) = {
        let world = sim.world_mut();
        let geometry = world
            .query::<(
                &LiveRoomInstance,
                &ambition_platformer2d::engine_core::RoomGeometry,
            )>()
            .iter(world)
            .find(|(room, _)| **room == second)
            .map(|(_, geometry)| geometry.0.clone())
            .expect("#1 has its geometry");
        let solid = |block: &ambition_platformer2d::engine_core::Block| {
            matches!(block.kind, ambition_platformer2d::engine_core::BlockKind::Solid)
        };
        geometry
            .blocks
            .iter()
            .filter(|block| solid(block) && block.aabb.height() > 2.0 * half.y + 8.0)
            .find_map(|wall| {
                let start = ambition_platformer2d::engine_core::Vec2::new(
                    wall.aabb.left() - half.x - 40.0,
                    wall.aabb.center().y,
                );
                // The box from Alice's start to the wall must be clear.
                let lane = ambition_platformer2d::engine_core::Aabb::new(
                    ambition_platformer2d::engine_core::Vec2::new(
                        (start.x - half.x + wall.aabb.left()) * 0.5,
                        start.y,
                    ),
                    ambition_platformer2d::engine_core::Vec2::new(
                        (wall.aabb.left() - (start.x - half.x)) * 0.5 - 0.5,
                        half.y,
                    ),
                );
                let clear = !geometry
                    .blocks
                    .iter()
                    .any(|other| solid(other) && other.aabb.strict_intersects(lane));
                (clear && start.x - half.x > 0.0).then_some((start, wall.aabb))
            })
            .expect("precondition: #1 has no solid block to blink against")
    };
    sim.teleport_player((start.x, start.y));
    sim.step(ambition_app::AgentAction {
        move_x: 1.0,
        right_pressed: true,
        attack: true,
        ..base()
    });
    sim.step(base());
    let now = sim
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(alice)
        .expect("Alice's body")
        .pos;
    assert!(
        now.x - start.x > 10.0,
        "precondition: Alice did not blink (from {start} to {now})"
    );
    assert!(
        now.x + half.x <= wall.left() + 1.0,
        "Alice blinked from {start} to {now}, through the wall whose left side is at {}",
        wall.left()
    );
}

/// OW1 cut 7p: a player's safe point is remembered on the walls of their own
/// live room. Alice walks in the hub (#1) while Bob holds `switch_lab` (#0);
/// standing again, her last safe point is where she stands. Before, the
/// damage step read the sole live room, so while two rooms were live it did
/// not run: no player took a hit and no safe point moved.
#[test]
fn a_safe_point_is_remembered_in_the_players_own_live_room() {
    use ambition_platformer2d::platformer::safe_position::PlayerSafetyState;
    let (mut sim, _) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    let alice_now = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world
            .query_filtered::<(&ambition_platformer2d::engine_core::BodyKinematics, &PlayerSafetyState), bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .map(|(kinematics, safety)| (kinematics.pos, safety.last_safe_pos))
            .expect("Alice's body is in the world")
    };
    let (start, _) = alice_now(&mut sim);
    for _ in 0..30 {
        sim.step(ambition_app::AgentAction { move_x: 1.0, ..base() });
    }
    for _ in 0..30 {
        sim.step(base());
    }
    let (pos, safe) = alice_now(&mut sim);
    assert_eq!(live_rooms(&mut sim).len(), 2, "precondition: Bob's room did not stay live beside Alice's");
    assert!(pos.distance(start) > 30.0, "control: Alice did not walk in #1 ({start} -> {pos})");
    assert!(
        safe.distance(pos) < 2.0,
        "Alice's safe point did not follow her in #1: she stands at {pos}, the safe point is {safe}"
    );
}

/// OW1: the dev traces record each body against its own live room. With
/// Alice in the hub (#1) beside Bob's `switch_lab` (#0), the player trace
/// keeps recording, in the hub, and the actor trace's frame holds both rooms
/// and tags Alice's body with the hub. Before, both traces read the sole live
/// room, so while two rooms were live neither recorded a frame.
#[test]
fn the_traces_record_each_body_in_its_own_live_room() {
    use ambition_platformer2d::gameplay_trace::{ActorTraceBuffer, GameplayTraceBuffer};
    let (mut sim, _) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    let rooms = live_rooms(&mut sim);
    assert_eq!(rooms.len(), 2, "precondition: Bob's room did not stay live beside Alice's");
    let hub = rooms.iter().find(|(_, id)| id == HUB).expect("the hub is live").0;
    let last_tick = |sim: &mut Platformer2dSimHarness| {
        sim.world_mut().resource::<GameplayTraceBuffer>().frames().last().map(|frame| frame.tick)
    };
    let before = last_tick(&mut sim);
    for _ in 0..5 {
        sim.step(base());
    }
    let world = sim.world_mut();
    let player = world.resource::<GameplayTraceBuffer>().frames().last().cloned().expect("the player trace has a row");
    assert!(
        Some(player.tick) > before,
        "the player trace recorded no row while two rooms were live (last tick {before:?})"
    );
    assert_eq!(player.active_area, HUB, "the player trace's row is not Alice's room");
    let actor = world.resource::<ActorTraceBuffer>().frames().last().cloned().expect("the actor trace has a frame");
    let mut areas: Vec<&str> = actor.rooms.iter().map(|room| room.area.as_str()).collect();
    areas.sort();
    assert_eq!(areas, vec![HUB, ROOM], "the actor trace's frame does not hold both live rooms");
    let alice = actor.bodies.iter().find(|body| body.actor_id == "player").expect("Alice's body is traced");
    assert_eq!(alice.room, Some(hub.ordinal()), "Alice's body is not traced in the hub");
}

/// OW4: each live room names the owners that hold it live, by the rule the
/// crossing uses to retire a room. Bob (slot 1) holds `switch_lab` (#0) and
/// Alice (slot 0) holds the hub (#1). Alice walks back to #0: her claim on
/// #1 is released with her, #1 retires, and #0 is held by both. Bob's claim
/// on #0 stands at every sample: before the crossing, after it, and on each
/// of the 30 ticks after it (the door walk itself hides its ticks). The
/// census prints the same answer.
#[test]
fn a_departing_player_releases_only_their_own_claim() {
    use ambition_platformer2d::actors::rooms::{live_room_claims_in, RoomClaim};
    use ambition_platformer2d::characters::control::PlayerSlot;
    let (mut sim, first) = alice_leaves_bob(Some(PlayerSlot(1)));
    let (alice, bob) = {
        let world = sim.world_mut();
        let alice = world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("Alice's body is in the world");
        let bob = world
            .query::<(bevy::prelude::Entity, &ambition_platformer2d::combat::components::FeatureId)>()
            .iter(world)
            .find(|(_, feature)| feature.0 == BOB)
            .map(|(entity, _)| entity)
            .expect("Bob's body is in the world");
        (alice, bob)
    };
    let hub = live_rooms(&mut sim).iter().find(|(_, id)| id == HUB).expect("the hub is live").0;
    let bob_holds_first = |claims: &[(LiveRoomInstance, Vec<RoomClaim>)]| {
        claims.iter().any(|(room, claims)| {
            *room == first && claims.contains(&RoomClaim { slot: PlayerSlot(1), body: bob })
        })
    };
    assert_eq!(
        live_room_claims_in(sim.world_mut()),
        vec![
            (first, vec![RoomClaim { slot: PlayerSlot(1), body: bob }]),
            (hub, vec![RoomClaim { slot: PlayerSlot(0), body: alice }]),
        ],
        "before Alice comes back: Bob holds #0 and Alice holds the hub"
    );
    assert_eq!(walk_through_the_door_to(&mut sim, ROOM), ROOM);
    for tick in 0..30 {
        let claims = live_room_claims_in(sim.world_mut());
        assert!(bob_holds_first(&claims), "tick {tick} after the crossing: Bob's claim on #0 was released: {claims:?}");
        sim.step(base());
    }
    let claims = live_room_claims_in(sim.world_mut());
    assert_eq!(
        claims,
        vec![(
            first,
            vec![RoomClaim { slot: PlayerSlot(0), body: alice }, RoomClaim { slot: PlayerSlot(1), body: bob }],
        )],
        "after Alice comes back: the hub retired with her claim, and both hold #0"
    );
    assert_eq!(
        ambition_platformer2d::runtime::runtime_census::room_holders_segment(&claims),
        format!(" holders=[{first}:slot0,slot1]"),
        "the census does not print the claims"
    );
}

/// Alice's position and health, and the authored spawn of `room`.
fn alice_and_spawn_of(sim: &mut Platformer2dSimHarness, room: &str) -> (bevy::prelude::Vec2, i32, i32, bevy::prelude::Vec2) {
    let world = sim.world_mut();
    let (pos, health) = world
        .query_filtered::<(&ambition_platformer2d::engine_core::BodyKinematics, &ambition_platformer2d::characters::actor::BodyHealth), bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .map(|(kinematics, health)| (kinematics.pos, (health.health.current, health.health.max)))
        .expect("Alice's body is in the world");
    let spawn = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(world)
    .expect("the session keeps its room set")
    .rooms
    .iter()
    .find(|spec| spec.id == room)
    .map(|spec| spec.world.spawn)
    .unwrap_or_else(|| panic!("the room set holds {room}"));
    (pos, health.0, health.1, bevy::prelude::Vec2::new(spawn.x, spawn.y))
}

/// Alice walks away from the hub spawn and is hurt, beside Bob's live
/// `switch_lab`. Returns the hub's spawn.
fn alice_walks_off_hurt_in_the_hub(sim: &mut Platformer2dSimHarness) -> bevy::prelude::Vec2 {
    for _ in 0..40 {
        sim.step(ambition_app::AgentAction { move_x: 1.0, ..base() });
    }
    for _ in 0..20 {
        sim.step(base());
    }
    {
        let world = sim.world_mut();
        let mut health = world
            .query_filtered::<&mut ambition_platformer2d::characters::actor::BodyHealth, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single_mut(world)
            .expect("Alice's body is in the world");
        health.health.current = 1;
    }
    sim.rebase_rollback_history().expect("the rollback history rebases over Alice's wound");
    let (pos, health, max, spawn) = alice_and_spawn_of(sim, HUB);
    assert_eq!(live_rooms(sim).len(), 2, "precondition: Bob's room did not stay live beside Alice's");
    assert!(pos.distance(spawn) > 60.0, "precondition: Alice is still at the hub spawn ({pos} vs {spawn})");
    assert!(health < max, "precondition: Alice is not hurt ({health}/{max})");
    spawn
}

/// OW1 Cut A: a room replay asked while two rooms are live replays the
/// live room of the player who plays it. Alice, hurt and away from the hub
/// spawn beside Bob's `switch_lab`, asks for a replay: she is back at the
/// hub spawn with full health, and Bob's room is still live. Before, the
/// admission read the sole live room, so the request was drained and lost,
/// and the return to spawn read the sole room's geometry.
#[test]
fn a_replay_beside_another_live_room_replays_the_players_own_room() {
    let (mut sim, first) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    let spawn = alice_walks_off_hurt_in_the_hub(&mut sim);
    sim.world_mut().write_message(ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual());
    for _ in 0..30 {
        sim.step(base());
    }
    let (pos, health, max, _) = alice_and_spawn_of(&mut sim, HUB);
    let rooms = live_rooms(&mut sim);
    assert!(rooms.iter().any(|(room, id)| *room == first && id == ROOM), "Bob's room did not stay live: {rooms:?}");
    assert!(rooms.iter().any(|(_, id)| id == HUB), "Alice's room is not live after the replay: {rooms:?}");
    assert!(pos.distance(spawn) < 40.0, "Alice was not returned to the hub spawn: {pos} vs {spawn}");
    assert_eq!(health, max, "Alice did not come back with full health");
}

/// OW1 Cut A: a checkpoint reset while two rooms are live is served in the
/// live room of the player who died. With no checkpoint saved, Alice comes
/// back at her own room's spawn (the hub's), Bob's room stays live, and the
/// session is no longer owed the reset. Before, the resume read the sole
/// live room, so while two rooms were live the reset was owed forever.
#[test]
fn a_checkpoint_reset_beside_another_live_room_is_served_in_the_players_own_room() {
    let (mut sim, first) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    let spawn = alice_walks_off_hurt_in_the_hub(&mut sim);
    sim.world_mut().write_message(ambition_platformer2d::platformer::lifecycle::ResetToCheckpoint);
    for _ in 0..30 {
        sim.step(base());
    }
    let owed = sim
        .world_mut()
        .resource::<ambition_platformer2d::actors::session::checkpoint::OutstandingCheckpointRequest>()
        .0;
    assert_eq!(owed, None, "the session is still owed the checkpoint reset");
    let (pos, _, _, _) = alice_and_spawn_of(&mut sim, HUB);
    let rooms = live_rooms(&mut sim);
    assert!(rooms.iter().any(|(room, id)| *room == first && id == ROOM), "Bob's room did not stay live: {rooms:?}");
    assert!(pos.distance(spawn) < 40.0, "Alice did not come back at the hub spawn: {pos} vs {spawn}");
}

/// OW1 Cut A: a level that ends while two rooms are live sends its player on
/// from that player's own live room. Alice's level in the hub asks to leave
/// for `switch_lab`: she joins the live room Bob holds there, and the hub
/// retires. Before, the departure read the sole live room, so while two
/// rooms were live no level could end.
#[test]
fn a_level_that_ends_beside_another_live_room_sends_its_player_on() {
    use ambition_platformer2d::platformer::lifecycle::{Departure, DepartureState, Destination};
    let (mut sim, first) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    assert_eq!(live_rooms(&mut sim).len(), 2, "precondition: Bob's room did not stay live beside Alice's");
    sim.world_mut().spawn(Departure { state: DepartureState::Requested(Destination::Room(ROOM.to_string())) });
    sim.rebase_rollback_history().expect("the rollback history rebases over the departure");
    for _ in 0..60 {
        sim.step(base());
    }
    let (alice, bob) = where_they_are(&mut sim);
    assert_eq!(live_rooms(&mut sim), vec![(first, ROOM.to_string())], "the hub did not retire behind Alice");
    assert_eq!((alice, bob), (Some(first), Some(Some(first))), "Alice did not join Bob's room");
}

/// OW1 Cut A: a replay of the cut-rope arena beside another live room hangs
/// the next heavy object. Alice in the arena (#1) beside Bob's Hall of
/// Bosses (#0) asks for a replay: the heavy object cycle advances,
/// and Bob's room stays live. Before, the arena reset read the sole live
/// room and did nothing while two rooms were live.
#[test]
fn a_replay_of_the_cut_rope_arena_beside_another_live_room_hangs_the_next_heavy_object() {
    use ambition_content::bosses::cut_rope::CutRopeHeavyObjectCycle;
    let (mut sim, first) = alice_leaves_bob_in(
        "hall_of_bosses",
        "you_have_to_cut_the_rope",
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    assert_eq!(live_rooms(&mut sim).len(), 2, "precondition: Bob's room did not stay live beside Alice's");
    let before = *sim.world_mut().resource::<CutRopeHeavyObjectCycle>();
    sim.world_mut().write_message(ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual());
    for _ in 0..30 {
        sim.step(base());
    }
    let after = *sim.world_mut().resource::<CutRopeHeavyObjectCycle>();
    let rooms = live_rooms(&mut sim);
    assert!(rooms.iter().any(|(room, id)| *room == first && id == "hall_of_bosses"), "Bob's room did not stay live: {rooms:?}");
    // Two heavy objects: one advance is a change.
    assert_ne!(after, before, "the heavy object did not advance on the arena's replay");
}

/// [`alice_leaves_bob_with`] from `switch_lab` to the hub, with `prepare`
/// run on the booted harness before Bob is placed.
fn alice_leaves_bob_after(
    slot: Option<ambition_platformer2d::characters::control::PlayerSlot>,
    prepare: impl FnOnce(&mut Platformer2dSimHarness),
) -> (Platformer2dSimHarness, LiveRoomInstance) {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .unwrap_or_else(|error| panic!("{ROOM} boots: {error:?}"));
    prepare(&mut sim);
    alice_leaves_bob_with(sim, ROOM, HUB, slot, walk_through_the_door_to)
}

/// OW1 Cut C: a room-entry cutscene plays when its room becomes live beside
/// another. A test cutscene is bound to the hub. Alice crosses into the hub:
/// with Bob driven (two rooms live) and with Bob undriven (the control: one
/// room), the cutscene plays. Before, the trigger read the sole live room, so
/// in the two-room arm it queued nothing.
#[test]
fn a_room_cutscene_plays_when_its_room_becomes_live_beside_another() {
    use ambition_platformer2d::cutscene::{ActiveCutscene, CutsceneBeat, CutsceneLibrary, CutsceneScript, RoomCutsceneBindings};
    const CUTSCENE: &str = "ow1_hub_entry_probe";
    for (slot, rooms) in [(None, 1), (Some(ambition_platformer2d::characters::control::PlayerSlot(1)), 2)] {
        let (mut sim, _) = alice_leaves_bob_after(slot, |sim| {
            let world = sim.world_mut();
            world
                .resource_mut::<CutsceneLibrary>()
                .insert(CutsceneScript::new(CUTSCENE, vec![CutsceneBeat::Wait { seconds: 30.0 }]));
            world.resource_mut::<RoomCutsceneBindings>().bindings.push((HUB.to_string(), CUTSCENE.to_string()));
        });
        assert_eq!(live_rooms(&mut sim).len(), rooms, "precondition ({slot:?}): the live room count");
        let playing = sim
            .world_mut()
            .resource::<ActiveCutscene>()
            .runtime
            .as_ref()
            .map(|runtime| runtime.script.id.clone());
        assert_eq!(playing.as_deref(), Some(CUTSCENE), "the hub's cutscene is not playing with {rooms} live room(s)");
    }
}

/// OW1 Cut C: the map records a visit to a room that becomes live beside
/// another. The save does not have the hub's visit flag; Alice crosses into
/// the hub, and the flag is set, with Bob driven (two rooms live) and with
/// Bob undriven (the control: one room). Before, the visit tracker read the
/// sole live room, so in the two-room arm the hub was never visited.
#[test]
fn the_map_records_a_room_visited_beside_another_live_room() {
    use ambition_platformer2d::menu::map::room_visited_flag;
    let flag = room_visited_flag(HUB);
    for (slot, rooms) in [(None, 1), (Some(ambition_platformer2d::characters::control::PlayerSlot(1)), 2)] {
        let (mut sim, _) = alice_leaves_bob_after(slot, |sim| {
            let visited = sim
                .world_mut()
                .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
                .data()
                .flag(&room_visited_flag(HUB));
            assert!(!visited, "precondition: the save has the hub's visit before Alice went there");
        });
        assert_eq!(live_rooms(&mut sim).len(), rooms, "precondition ({slot:?}): the live room count");
        let visited = sim
            .world_mut()
            .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
            .data()
            .flag(&flag);
        assert!(visited, "the hub visit was not recorded with {rooms} live room(s)");
    }
}
