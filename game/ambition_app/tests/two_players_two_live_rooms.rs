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
        let world = sim.world_mut();
        seen = world
            .query::<(&C, Option<&InRoomInstance>)>()
            .iter(world)
            .next()
            .map(|(conductor, room)| (Some(room.map(|room| room.0)), hall(conductor).is_some()))
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
    let (room, measured, second) = a_conducted_boss_beside_bob(
        "flying_spaghetti_monster_arena",
        ambition_content::bosses::fsm::FsmConductor::hall,
    );
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
