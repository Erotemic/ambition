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

pub(crate) const ROOM: &str = "switch_lab";
const HUB: &str = "central_hub_complex";
const BOB: &str = "ow1_bob";

/// The live rooms, by instance, and the id of the room each instantiates.
pub(crate) fn live_rooms(sim: &mut Platformer2dSimHarness) -> Vec<(LiveRoomInstance, String)> {
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
pub(crate) fn where_they_are(
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

/// Alice's live room: her stamp, or with one room live, that room.
fn alices_room(sim: &mut Platformer2dSimHarness) -> LiveRoomInstance {
    let (alice, _) = where_they_are(sim);
    let live = live_rooms(sim);
    alice
        .or_else(|| (live.len() == 1).then(|| live[0].0))
        .expect("Alice stands in a live room")
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

/// [`alice_leaves_bob`] with Bob driven by slot 1, for another module's arm.
pub(crate) fn alice_leaves_bob_for_a_replay() -> (Platformer2dSimHarness, LiveRoomInstance) {
    alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)))
}

/// Bob, driven by slot 1, beside Alice in `switch_lab`, and Alice stays.
/// Returns their shared live room.
pub(crate) fn alice_beside_bob() -> (Platformer2dSimHarness, LiveRoomInstance) {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .unwrap_or_else(|error| panic!("{ROOM} boots: {error:?}"));
    let first = bob_beside_alice(&mut sim, ROOM, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    (sim, first)
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
pub(crate) fn alice_leaves_bob_in(
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
    let first = bob_beside_alice(&mut sim, start, slot);
    assert_eq!(cross(&mut sim, target), target);
    for _ in 0..30 {
        sim.step(base());
    }
    (sim, first)
}

/// Put Bob, driven by `slot` or by nobody, beside Alice in `start`, the room
/// `sim` booted in. Returns their live room.
pub(crate) fn bob_beside_alice(
    sim: &mut Platformer2dSimHarness,
    start: &str,
    slot: Option<ambition_platformer2d::characters::control::PlayerSlot>,
) -> LiveRoomInstance {
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
        where_they_are(sim),
        (Some(first), Some(Some(first))),
        "precondition: Alice and Bob are not both in the first live room"
    );
    if slot.is_some() {
        let ahead = bob_runs(sim, 1.0);
        assert!(ahead > 1.0, "control: slot 1 moved Bob's body {ahead} before anyone left");
    }
    first
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

/// OW1 (customer 2): Alice holds the hub and Bob holds `switch_lab`. A
/// gravity well (up) opens at the place Bob stands, stamped into Alice's
/// room: Bob keeps falling down. Control: the same well stamped into Bob's
/// own room turns him up. Before, `GravityZones` had no room and a body felt
/// every live room's zones at its position.
#[test]
fn a_gravity_well_lifts_only_the_bodies_of_its_own_live_room() {
    fn bobs_down_under_a_well_in_his_room(his_room: bool) -> ambition_platformer2d::engine_core::Vec2 {
        let (mut sim, first) =
            alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
        let (alice, bob) = where_they_are(&mut sim);
        assert_eq!((alice, bob), (Some(first.next()), Some(Some(first))), "Alice in the hub, Bob in switch_lab");
        let world = sim.world_mut();
        let at = world
            .query::<(&ambition_platformer2d::combat::components::FeatureId, &ambition_platformer2d::engine_core::BodyKinematics)>()
            .iter(world)
            .find(|(feature, _)| feature.0 == BOB)
            .map(|(_, kinematics)| kinematics.pos)
            .expect("Bob's body is in the world");
        let room = if his_room { first } else { first.next() };
        world.spawn((
            ambition_platformer2d::world::GravityZone {
                aabb: ambition_platformer2d::engine_core::Aabb::new(
                    at,
                    ambition_platformer2d::engine_core::Vec2::new(80.0, 80.0),
                ),
                dir: ambition_platformer2d::engine_core::Vec2::new(0.0, -1.0),
            },
            ambition_platformer2d::world::TemporaryZone { remaining: 5.0 },
            InRoomInstance(room),
        ));
        for _ in 0..3 {
            sim.step(base());
        }
        let world = sim.world_mut();
        world
            .query::<(&ambition_platformer2d::combat::components::FeatureId, &ambition_platformer2d::world::ResolvedMotionFrame)>()
            .iter(world)
            .find(|(feature, _)| feature.0 == BOB)
            .map(|(_, frame)| frame.get().down())
            .expect("Bob has a resolved frame")
    }
    assert_eq!(
        bobs_down_under_a_well_in_his_room(true),
        ambition_platformer2d::engine_core::Vec2::new(0.0, -1.0),
        "control: a well in Bob's own room turns him"
    );
    assert_eq!(
        bobs_down_under_a_well_in_his_room(false),
        ambition_platformer2d::engine_core::Vec2::new(0.0, 1.0),
        "a well in Alice's room turned Bob in his"
    );
}

/// The resolved down of each player's body: (Alice's, Bob's).
fn downs(sim: &mut Platformer2dSimHarness) -> (ambition_platformer2d::engine_core::Vec2, ambition_platformer2d::engine_core::Vec2) {
    let world = sim.world_mut();
    let alice = world
        .query_filtered::<&ambition_platformer2d::world::ResolvedMotionFrame, bevy::prelude::With<ambition_platformer2d::platformer::body::PrimaryBody>>()
        .single(world)
        .expect("Alice has a resolved frame")
        .get()
        .down();
    let bob = world
        .query::<(&ambition_platformer2d::combat::components::FeatureId, &ambition_platformer2d::world::ResolvedMotionFrame)>()
        .iter(world)
        .find(|(feature, _)| feature.0 == BOB)
        .map(|(_, frame)| frame.get().down())
        .expect("Bob has a resolved frame");
    (alice, bob)
}

/// OW1 (customer 2): a wave zooms the views of its own live room. Bob holds
/// the hub (#0) and Alice starts the goblin wave in `goblin_encounter` (#1).
/// The zoom the encounter publishes is in #1 (the control: the wave's own
/// room zooms) and not in #0. Before, the zoom was one value for the session,
/// and Bob's view zoomed out for Alice's fight.
#[test]
fn a_wave_zooms_only_the_views_of_its_own_live_room() {
    const ARENA: &str = "goblin_encounter";
    let (mut sim, first) = alice_leaves_bob_in(
        HUB,
        ARENA,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    let second = first.next();
    assert_eq!(where_they_are(&mut sim), (Some(second), Some(Some(first))), "precondition: Alice in #1, Bob in #0");
    let trigger = {
        let world = sim.world_mut();
        world
            .query::<(&ambition_platformer2d::encounter::EncounterWaves, &InRoomInstance)>()
            .iter(world)
            .find(|(waves, room)| waves.spec.id == ARENA && room.0 == second)
            .map(|(waves, _)| waves.spec.trigger_aabb())
            .expect("precondition: #1 has an occurrence of the goblin encounter")
    };
    {
        use ambition_platformer2d::engine_core::AabbExt as _;
        let center = trigger.center();
        sim.teleport_player((center.x, center.y));
    }
    let zooms = |sim: &mut Platformer2dSimHarness| {
        let view = sim.world_mut().resource::<ambition_platformer2d::encounter::EncounterView>().clone();
        (view.camera_zoom_in(Some(first)), view.camera_zoom_in(Some(second)))
    };
    let mut seen = zooms(&mut sim);
    for _ in 0..900 {
        sim.step(base());
        seen = zooms(&mut sim);
        if seen.1 > 1.0 {
            break;
        }
    }
    assert!(seen.1 > 1.0, "precondition: the goblin wave in #1 never asked for a zoom: {seen:?}");
    assert_eq!(seen.0, 1.0, "the wave in #1 zoomed the views of #0: {seen:?}");
    // And each view's camera reads the zoom of the room it frames: the view
    // of #1 zooms out (the control), the view of #0 does not.
    for _ in 0..120 {
        sim.step(base());
    }
    let world = sim.world_mut();
    let mut frames: Vec<_> = world
        .query::<&ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot>()
        .iter(world)
        .filter_map(|resolved| resolved.0.as_ref().map(|frame| (frame.room, frame.snapshot.zoom_multiplier)))
        .collect();
    frames.sort_by_key(|(room, _)| *room);
    assert_eq!(frames.len(), 2, "precondition: two views: {frames:?}");
    assert!(
        frames[0].0 == first && frames[1].0 == second && frames[1].1 > frames[0].1,
        "(the view of #0, the view of #1) by (room, zoom): the view of #1 must zoom out past #0's: {frames:?}"
    );
}

/// OW1 (customer 2): a gravity switch turns the ambient of its own live room.
/// Alice holds the hub and Bob holds `switch_lab`; a `FlipGravity` switch is
/// pressed in Bob's room. Bob falls up (the control: the switch acted), and
/// Alice still falls down. Before, `BaseGravity` was one direction for the
/// whole world, and Bob's switch turned Alice upside down in her room.
#[test]
fn a_gravity_switch_turns_only_the_live_room_it_is_in() {
    use ambition_platformer2d::encounter::switches::{QueuedSwitchActivation, SwitchActivationQueue};
    let (mut sim, first) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    let (alice, bob) = where_they_are(&mut sim);
    assert_eq!((alice, bob), (Some(first.next()), Some(Some(first))), "Alice in the hub, Bob in switch_lab");
    let down = ambition_platformer2d::engine_core::Vec2::new(0.0, 1.0);
    assert_eq!(downs(&mut sim), (down, down), "precondition: both players fall down");
    sim.world_mut().resource_mut::<SwitchActivationQueue>().0.push(QueuedSwitchActivation {
        activation: ambition_platformer2d::encounter::registry::SwitchActivation {
            id: "ow1_bobs_flip".to_string(),
            action: "FlipGravity".to_string(),
            target_encounter: String::new(),
        },
        room: Some(first),
    });
    for _ in 0..3 {
        sim.step(base());
    }
    assert_eq!(
        downs(&mut sim),
        (down, -down),
        "(Alice's down, Bob's down) after a switch in Bob's room"
    );
}

/// The Door from authored room `room` to `target`.
pub(crate) fn door_of(sim: &mut Platformer2dSimHarness, room: &str, target: &str) -> ambition_platformer2d::world::rooms::LoadingZone {
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

/// Move Bob's body to `at`, at rest.
pub(crate) fn put_bob_at(sim: &mut Platformer2dSimHarness, at: ambition_platformer2d::engine_core::Vec2) {
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
        at,
        ambition_platformer2d::engine_core::movement::TransitVelocity::Zero,
    );
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
    put_bob_at(&mut sim, door);
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

/// Alice goes from the hub back through its door to `switch_lab` while
/// Bob's seat (slot 1) runs left. For each tick of her crossing (the load
/// open as the tick began or as it ended): whether Bob's body stood still,
/// and whether the session was in `GameMode::RoomTransition`. Returns
/// (ticks of the crossing, ticks Bob stood still, ticks paused).
fn bob_runs_while_alice_crosses(sim: &mut Platformer2dSimHarness) -> (usize, Vec<usize>, usize) {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let bob_x = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world
            .query::<(&ambition_platformer2d::combat::components::FeatureId, &ambition_platformer2d::engine_core::BodyKinematics)>()
            .iter(world)
            .find(|(feature, _)| feature.0 == BOB)
            .map(|(_, kinematics)| kinematics.pos.x)
    };
    let loading = |sim: &Platformer2dSimHarness| {
        sim.world()
            .resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>()
            .active
            .is_some()
    };
    let paused = |sim: &Platformer2dSimHarness| {
        sim.world()
            .get_resource::<bevy::state::state::State<ambition_platformer2d::platformer::schedule::GameMode>>()
            .is_some_and(|mode| *mode.get() == ambition_platformer2d::platformer::schedule::GameMode::RoomTransition)
    };
    let door = crate::common::door_to(sim, ROOM).aabb.center();
    sim.teleport_player((door.x, door.y));
    let (mut crossing, mut still, mut held) = (0, Vec::new(), 0);
    let mut was_open = false;
    for tick in 0..120 {
        let before = bob_x(sim);
        sim.drive_seat(
            1,
            ambition_platformer2d::engine_core::ControlFrame {
                axis_x: -1.0,
                ..Default::default()
            },
        );
        let room = sim
            .step(ambition_app::AgentAction {
                interact: true,
                interact_held: true,
                ..base()
            })
            .active_room;
        let open = loading(sim);
        if open || was_open {
            crossing += 1;
            held += usize::from(paused(sim));
            if let (Some(before), Some(after)) = (before, bob_x(sim)) {
                if (after - before).abs() < 0.01 {
                    still.push(tick);
                }
            }
        }
        if room == ROOM && !open {
            break;
        }
        was_open = open;
    }
    sim.drive_seat(1, ambition_platformer2d::engine_core::ControlFrame::default());
    (crossing, still, held)
}

/// OW1 cut 7t: Bob's room keeps its time while Alice goes through a door.
/// Bob, on slot 1, runs in `switch_lab` (#0) while Alice goes from the hub
/// back through its door to `switch_lab`; on each tick of her crossing,
/// Bob's body must move. The eager host set `GameMode::RoomTransition` for
/// every load, and that mode stops every gameplay system of every live
/// room, so Bob stood still while another player's room was prepared. The
/// rollback host, which the game ships, never paused for a load. The
/// control: Alice alone (Bob driven by no seat, so his room retired with
/// her first crossing) still pauses for her load.
#[test]
fn a_door_one_player_takes_does_not_stop_the_other_players_room() {
    let (mut sim, _) = alice_leaves_bob(None);
    let (crossing, _, held) = bob_runs_while_alice_crosses(&mut sim);
    assert!(crossing > 0 && held > 0, "control: Alice's crossing alone did not pause ({held} of {crossing} ticks)");

    let (mut sim, _) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    let (crossing, still, held) = bob_runs_while_alice_crosses(&mut sim);
    assert!(crossing > 0, "precondition: Alice's crossing opened no load");
    assert_eq!(
        (still, held),
        (Vec::<usize>::new(), 0),
        "(ticks Bob stood still, ticks paused) in Alice's crossing of {crossing} ticks"
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
    use crate::boss_lifecycle::{boss_cleared, force_kill_boss, music_track_in, spawn_mockingbird, MOCKINGBIRD_TRACK};
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
        (music_track_in(&sim, Some(first)).as_deref(), music_track_in(&sim, Some(second)).as_deref()),
        (None, Some(MOCKINGBIRD_TRACK)),
        "(the fight music of #0, of #1): the boss in #1 did not wake while two rooms are live, \
         or its music was claimed in the other room"
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
        (boss_cleared(&sim, BOSS), chests, music_track_in(&sim, Some(second))),
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
    // The rooms the arena's effect requests name: the rope sparks and the
    // blast (`FxRequest`), the death fireworks, and the debris of the death.
    let mut fx_rooms = std::collections::BTreeSet::new();
    let mut firework_rooms = Vec::new();
    let mut debris_rooms = std::collections::BTreeSet::new();
    for _ in 0..1800 {
        sim.step(base());
        let world = sim.world_mut();
        effect_request_rooms(world, &mut fx_rooms, &mut firework_rooms, &mut debris_rooms);
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
    // The death beat runs out before the defeat is recorded. The arena's
    // blast and fireworks answer the impact gate on the tick after it.
    for _ in 0..300 {
        sim.step(base());
        effect_request_rooms(sim.world_mut(), &mut fx_rooms, &mut firework_rooms, &mut debris_rooms);
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
            fx_rooms.into_iter().collect::<Vec<_>>(),
            firework_rooms,
            debris_rooms.into_iter().collect::<Vec<_>>(),
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
            vec![Some(second)],
            vec![Some(second)],
            vec![Some(second)],
        ),
        "the cut-rope road did not run whole in #1: (rope_cut rooms, lured to #1's anvil, \
         walked toward it, hazard rooms, it fell, impact rooms, the behemoth died, it is \
         recorded cleared, victory NPC rooms, the rooms its effect requests name, the rooms \
         its fireworks name, the rooms its debris names)"
    );
}

/// The cut-rope boss's music claim is released while no live room is its
/// room, and kept while one is. A claim under the boss's owner name is put on
/// the music of each live room, and the release system runs one time, alone.
/// With two live rooms that are not the arena (`switch_lab` and the hub), the
/// claim is released in both. With two live rooms of which one is the arena
/// (the hall and the arena), the claim is kept in the arena and released in
/// the hall. The first fixture then runs 5 ticks
/// with a new claim, which shows that the scheduled system does the same.
/// When the release read the sole live room, it did not run while two rooms
/// were live, so a claim left behind was kept for as long as two rooms were
/// live.
///
/// The arena arm does not run ticks. The generic boss owner
/// (`BOSS_MUSIC_OWNER`) takes the priority tier while the boss fights, so
/// after a tick the track in the tier is not this owner's claim.
#[test]
fn the_cut_rope_music_claim_is_released_when_no_live_room_is_its_room() {
    use ambition_content::bosses::cut_rope::{release_cut_rope_music_outside_its_room, CUT_ROPE_MUSIC_OWNER};
    use ambition_platformer2d::characters::control::PlayerSlot;
    use ambition_platformer2d::encounter::EncounterMusicRequest;
    use bevy::ecs::system::RunSystemOnce;
    const TRACK: &str = "ow_probe_track";
    fn claim(sim: &mut Platformer2dSimHarness) {
        let rooms = live_rooms(sim);
        let mut music = ambition_platformer2d::platformer::lifecycle::session_world_component_mut::<EncounterMusicRequest>(
            sim.world_mut(),
        )
        .expect("the session has a music request");
        for (room, _) in rooms {
            music.claim_priority(Some(room), CUT_ROPE_MUSIC_OWNER, TRACK);
        }
    }
    /// Each live room by its authored id, and whether it keeps the claim.
    fn claimed(sim: &mut Platformer2dSimHarness) -> Vec<(String, bool)> {
        let rooms = live_rooms(sim);
        let music = ambition_platformer2d::platformer::lifecycle::session_world_component::<EncounterMusicRequest>(sim.world())
            .expect("the session has a music request");
        let mut claimed: Vec<_> = rooms
            .into_iter()
            .map(|(room, id)| (id, music.priority_track(Some(room)) == Some(TRACK)))
            .collect();
        claimed.sort();
        claimed
    }
    let claim_after_one_release = |sim: &mut Platformer2dSimHarness| {
        assert_eq!(live_rooms(sim).len(), 2, "precondition: two rooms are live");
        claim(sim);
        sim.world_mut()
            .run_system_once(release_cut_rope_music_outside_its_room)
            .expect("the release system runs");
        claimed(sim)
    };
    let mut elsewhere = alice_leaves_bob(Some(PlayerSlot(1))).0;
    let mut beside_the_arena =
        alice_leaves_bob_in("hall_of_bosses", "you_have_to_cut_the_rope", Some(PlayerSlot(1)), walk_through_the_door_to).0;
    let released_elsewhere = claim_after_one_release(&mut elsewhere);
    let kept_beside_the_arena = claim_after_one_release(&mut beside_the_arena);
    claim(&mut elsewhere);
    for _ in 0..5 {
        elsewhere.step(base());
    }
    assert_eq!(
        (released_elsewhere, claimed(&mut elsewhere), kept_beside_the_arena),
        (
            vec![(HUB.to_string(), false), (ROOM.to_string(), false)],
            vec![(HUB.to_string(), false), (ROOM.to_string(), false)],
            vec![("hall_of_bosses".to_string(), false), ("you_have_to_cut_the_rope".to_string(), true)],
        ),
        "(the claim after one release with no live arena, the same after 5 ticks, the claim after one \
         release with the arena live beside the hall)"
    );
}

/// Add the rooms that this tick's `FxRequest` rows and `FireworksRequest` rows
/// name to `fx` and `fireworks`, and the rooms that its debris rows name to
/// `debris`.
fn effect_request_rooms(
    world: &bevy::prelude::World,
    fx: &mut std::collections::BTreeSet<Option<LiveRoomInstance>>,
    fireworks: &mut Vec<Option<LiveRoomInstance>>,
    debris: &mut std::collections::BTreeSet<Option<LiveRoomInstance>>,
) {
    debris.extend(
        world
            .resource::<bevy::ecs::message::Messages<ambition_platformer2d::vfx::vfx::DebrisBurstMessage>>()
            .iter_current_update_messages()
            .map(|row| row.room),
    );
    fx.extend(
        world
            .resource::<bevy::ecs::message::Messages<ambition_platformer2d::vfx::FxRequest>>()
            .iter_current_update_messages()
            .map(|request| request.room),
    );
    fireworks.extend(
        world
            .resource::<bevy::ecs::message::Messages<ambition_platformer2d::vfx::FireworksRequest>>()
            .iter_current_update_messages()
            .map(|request| request.room),
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
    use ambition_platformer2d::vfx::vfx::VfxInRoom;
    let mut rooms = Vec::new();
    for _ in 0..ticks {
        sim.step(base());
        let world = sim.world_mut();
        let barked: Vec<String> = world
            .get_resource_mut::<bevy::prelude::Messages<VfxInRoom>>()
            .map(|mut messages| {
                messages
                    .drain()
                    .map(|m| m.vfx)
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
/// another. A test cutscene is the hub's `entry_cutscene`. Alice crosses into the hub:
/// with Bob driven (two rooms live) and with Bob undriven (the control: one
/// room), the cutscene plays. Before, the trigger read the sole live room, so
/// in the two-room arm it queued nothing.
#[test]
fn a_room_cutscene_plays_when_its_room_becomes_live_beside_another() {
    use ambition_platformer2d::cutscene::{ActiveCutscene, CutsceneBeat, CutsceneLibrary, CutsceneScript};
    use ambition_platformer2d::world::rooms::RoomSet;
    const CUTSCENE: &str = "ow1_hub_entry_probe";
    for (slot, rooms) in [(None, 1), (Some(ambition_platformer2d::characters::control::PlayerSlot(1)), 2)] {
        let (mut sim, _) = alice_leaves_bob_after(slot, |sim| {
            let world = sim.world_mut();
            world
                .resource_mut::<CutsceneLibrary>()
                .insert(CutsceneScript::new(CUTSCENE, vec![CutsceneBeat::Wait { seconds: 30.0 }]));
            // The room names its entry cutscene, as a world file does.
            let mut sets = world.query::<&mut RoomSet>();
            let mut set = sets.iter_mut(world).next().expect("a room set");
            let hub = set.rooms.iter_mut().find(|spec| spec.id == HUB).expect("the hub is a room");
            hub.metadata.entry_cutscene = Some(CUTSCENE.to_string());
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

/// Whether the crossing asked for the shared sim clock to be reset.
#[derive(bevy::prelude::Resource, Default)]
struct CrossingResetTheClock(bool);

/// The live room Alice crossed out of.
#[derive(bevy::prelude::Resource)]
struct TheRoomLeft(LiveRoomInstance);

/// [`walk_through_the_door_to`], with two facts of the room being left
/// planted first, a projectile stamped into it and the ambient gravity
/// flipped, and with each tick of the walk read for the crossing's request to
/// reset the sim clock ([`CrossingResetTheClock`]).
fn walk_through_the_door_leaving_a_shot_and_flipped_gravity(
    sim: &mut Platformer2dSimHarness,
    target: &str,
) -> String {
    use ambition_platformer2d::engine_core::AabbExt as _;
    use ambition_platformer2d::time::time_control::ClockResetRequest;
    {
        let world = sim.world_mut();
        let room = *ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<LiveRoomInstance>(world)
            .expect("one room is live before Alice crosses");
        world.spawn((
            ambition_platformer2d::projectiles::LiveProjectile,
            InRoomInstance(room),
            ambition_platformer2d::combat::components::FeatureId("ow1_bob_shot".to_string()),
        ));
        let mut gravity = world.resource_mut::<ambition_platformer2d::world::BaseGravity>();
        let up = -gravity.dir_in(Some(room));
        gravity.turn(Some(room), up);
        world.insert_resource(CrossingResetTheClock(false));
        world.insert_resource(TheRoomLeft(room));
    }
    let before = sim.observation().active_room.clone();
    let door = crate::common::door_to(sim, target);
    let center = door.aabb.center();
    sim.teleport_player((center.x, center.y));
    for _ in 0..120 {
        let room = sim.step(ambition_app::AgentAction { interact: true, interact_held: true, ..base() }).active_room;
        let world = sim.world_mut();
        let reset = world
            .resource::<bevy::ecs::message::Messages<ClockResetRequest>>()
            .iter_current_update_messages()
            .any(|request| request.reason == "room_transition");
        world.resource_mut::<CrossingResetTheClock>().0 |= reset;
        if room != before {
            return room;
        }
    }
    panic!("the '{}' door of '{before}' never took Alice across", door.name);
}

/// OW1, customer 2: a crossing resets only what it leaves behind. Before
/// Alice leaves `switch_lab`, the room holds a shot and its gravity is
/// flipped. With Bob driven, the room stays live with him: the shot stays,
/// the room keeps its gravity, and the world's sim clock is not reset. The
/// control, Bob not driven: the crossing replaces the world, and the shot
/// goes, gravity is put back down and the clock reset is asked for, as
/// before. In both, Alice enters the hub under the default gravity. Before,
/// every crossing did all three, so Alice's door unflipped Bob's room and
/// cancelled his bullet time.
#[test]
fn a_crossing_resets_only_what_it_leaves_behind() {
    use ambition_platformer2d::characters::control::PlayerSlot;
    for (slot, rooms, expected) in [(None, 1, (false, true, false)), (Some(PlayerSlot(1)), 2, (true, false, true))] {
        let (mut sim, _) = alice_leaves_bob_by(slot, walk_through_the_door_leaving_a_shot_and_flipped_gravity);
        assert_eq!(live_rooms(&mut sim).len(), rooms, "precondition ({slot:?}): the live room count");
        let world = sim.world_mut();
        let shot = world
            .query::<&ambition_platformer2d::combat::components::FeatureId>()
            .iter(world)
            .any(|feature| feature.0 == "ow1_bob_shot");
        let clock_reset = world.resource::<CrossingResetTheClock>().0;
        let left = world.resource::<TheRoomLeft>().0;
        let gravity = world.resource::<ambition_platformer2d::world::BaseGravity>();
        let default = ambition_platformer2d::world::BaseGravity::default().dir_in(None);
        let flipped = gravity.dir_in(Some(left)) != default;
        let alice = alices_room(&mut sim);
        let gravity = sim.world_mut().resource::<ambition_platformer2d::world::BaseGravity>();
        assert_eq!(gravity.dir_in(Some(alice)), default, "Alice entered the hub under a turned gravity with {rooms} live room(s)");
        assert_eq!(
            (shot, clock_reset, flipped),
            expected,
            "with {rooms} live room(s) after the crossing: (the shot is there, the clock reset was asked for, gravity is flipped)"
        );
    }
}

/// OW1: a replay of one player's room keeps the world's sim clock while
/// another live room stays, and puts down only the gravity of the room it
/// replays. Every live room is flipped; Alice, in the hub, asks for a
/// replay. With Bob driven (two rooms): no reset of the sim clock is asked
/// for, the hub is put back down and Bob's room stays flipped. The control,
/// Bob not driven (one room): the clock reset is asked for and the hub is
/// put back down, as before. Before, every replay did both for the whole
/// world, so Alice's retry cancelled Bob's bullet time and unflipped his
/// room.
#[test]
fn a_replay_keeps_the_worlds_clock_and_gravity_while_another_room_is_live() {
    use ambition_platformer2d::characters::control::PlayerSlot;
    use ambition_platformer2d::time::time_control::ClockResetRequest;
    let up = ambition_platformer2d::engine_core::Vec2::new(0.0, -1.0);
    for (slot, rooms, expected) in [(None, 1, (true, false, false)), (Some(PlayerSlot(1)), 2, (false, false, true))] {
        let (mut sim, _) = alice_leaves_bob(slot);
        let live = live_rooms(&mut sim);
        assert_eq!(live.len(), rooms, "precondition ({slot:?}): the live room count");
        let alice = alices_room(&mut sim);
        {
            let world = sim.world_mut();
            let mut gravity = world.resource_mut::<ambition_platformer2d::world::BaseGravity>();
            for (room, _) in &live {
                gravity.turn(Some(*room), up);
            }
            world.write_message(ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual());
        }
        let mut clock_reset = false;
        let mut admitted = false;
        for _ in 0..30 {
            sim.step(base());
            let world = sim.world_mut();
            clock_reset |= world
                .resource::<bevy::ecs::message::Messages<ClockResetRequest>>()
                .iter_current_update_messages()
                .any(|request| request.reason == "sandbox_reset");
            admitted |= world
                .resource::<bevy::ecs::message::Messages<ambition_platformer2d::combat::events::RoomReplayAdmitted>>()
                .iter_current_update_messages()
                .next()
                .is_some();
        }
        assert!(admitted, "precondition ({slot:?}): the replay was not admitted");
        let gravity = sim.world_mut().resource::<ambition_platformer2d::world::BaseGravity>();
        let hub_flipped = gravity.dir_in(Some(alice)) == up;
        let other_flipped = live.iter().any(|(room, _)| *room != alice && gravity.dir_in(Some(*room)) == up);
        assert_eq!(
            (clock_reset, hub_flipped, other_flipped),
            expected,
            "with {rooms} live room(s): (the clock reset was asked for, the hub is flipped, Bob's room is flipped)"
        );
    }
}

/// OW1: a hazard respawn keeps the world's sim clock while another room is
/// live. Alice, in the hub, takes a hazard hit that sends her back to her safe
/// point. With Bob driven (two rooms), she respawns and no clock reset is
/// asked for. The control, Bob not driven (one room): she respawns and the
/// reset is asked for, as before.
#[test]
fn a_hazard_respawn_keeps_the_worlds_clock_while_another_room_is_live() {
    use ambition_platformer2d::characters::control::PlayerSlot;
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    use ambition_platformer2d::time::time_control::ClockResetRequest;
    for (slot, rooms, expected) in [(None, 1, (true, true)), (Some(PlayerSlot(1)), 2, (true, false))] {
        let (mut sim, _) = alice_leaves_bob(slot);
        assert_eq!(live_rooms(&mut sim).len(), rooms, "precondition ({slot:?}): the live room count");
        {
            let world = sim.world_mut();
            let (alice, pos, room) = world
                .query_filtered::<(bevy::prelude::Entity, &ambition_platformer2d::engine_core::BodyKinematics, Option<&InRoomInstance>), bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
                .single(world)
                .map(|(entity, kinematics, room)| (entity, kinematics.pos, room.map(|room| room.0)))
                .expect("Alice's body is in the world");
            world.write_message(HitEvent {
                volume: ambition_platformer2d::engine_core::Aabb::new(pos, ambition_platformer2d::engine_core::Vec2::splat(8.0)).into(),
                damage: 1,
                source: HitSource::Hazard,
                attacker: None,
                room,
                target: HitTarget::Body(alice),
                mode: HitMode::SafeRespawn,
                knockback: None,
                ignored_targets: Vec::new(),
                strike_sfx: None,
                attacker_move_instance: None,
            });
        }
        let (mut respawned, mut clock_reset) = (false, false);
        for _ in 0..10 {
            sim.step(base());
            let world = sim.world_mut();
            respawned |= world
                .resource::<bevy::ecs::message::Messages<ambition_platformer2d::vfx::vfx::VfxInRoom>>()
                .iter_current_update_messages().map(|m| &m.vfx)
                .any(|message| matches!(message, ambition_platformer2d::vfx::vfx::VfxMessage::ResetEffects { .. }));
            clock_reset |= world
                .resource::<bevy::ecs::message::Messages<ClockResetRequest>>()
                .iter_current_update_messages()
                .any(|request| request.reason == "safe_respawn");
        }
        assert_eq!(
            (respawned, clock_reset),
            expected,
            "with {rooms} live room(s): (Alice respawned, the clock reset was asked for)"
        );
    }
}

/// The crossing cooldown of seat `seat`: whether it must wait before it
/// crosses again.
fn seat_waits(sim: &Platformer2dSimHarness, seat: usize) -> bool {
    sim.world()
        .resource::<ambition_platformer2d::platformer::safe_position::RoomTransitionCooldown>()
        .holds(seat)
}

/// Whether a crossing is accepted and waits to commit.
fn a_crossing_is_pending(sim: &Platformer2dSimHarness) -> bool {
    sim.world()
        .resource::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
        .pending
        .is_some()
}

/// What the seats' cooldowns were as Alice and then Bob went through their
/// doors: written by [`alice_and_then_bob_go_through_their_doors`].
#[derive(bevy::prelude::Resource, Debug)]
struct SeatsAtTheDoors {
    /// Whether seats 0 and 1 wait, on the tick Alice's crossing commits.
    at_alices_commit: (bool, bool),
    /// The tick of Bob's press on which his crossing is accepted, and whether
    /// Alice's seat still waits on it.
    bob_accepted: Option<(usize, bool)>,
    /// Whether seats 0 and 1 wait, on the tick Bob arrives in the hub.
    at_bobs_arrival: Option<(bool, bool)>,
}

/// Alice goes through the door to `target`. On the tick that commits, Bob is
/// put in `switch_lab`'s door to the hub and his seat presses interact until
/// his crossing is accepted. He then goes through it.
fn alice_and_then_bob_go_through_their_doors(sim: &mut Platformer2dSimHarness, target: &str) -> String {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let (_, bob_was) = where_they_are(sim);
    let room = walk_through_the_door_to(sim, target);
    let at_alices_commit = (seat_waits(sim, 0), seat_waits(sim, 1));
    let door = door_of(sim, ROOM, HUB).aabb.center();
    put_bob_at(sim, door);
    let bob_moved = |sim: &mut Platformer2dSimHarness| where_they_are(sim).1 != bob_was;
    let mut bob_accepted = None;
    for tick in 1..=30 {
        sim.drive_seat(
            1,
            ambition_platformer2d::engine_core::ControlFrame {
                interact_pressed: true,
                interact_held: true,
                ..Default::default()
            },
        );
        sim.step(base());
        if a_crossing_is_pending(sim) || bob_moved(sim) {
            bob_accepted = Some((tick, seat_waits(sim, 0)));
            break;
        }
    }
    // A seat's frame stands until it is replaced: let go of the press.
    sim.drive_seat(1, ambition_platformer2d::engine_core::ControlFrame::default());
    let mut at_bobs_arrival = None;
    for _ in 0..120 {
        if bob_moved(sim) {
            at_bobs_arrival = Some((seat_waits(sim, 0), seat_waits(sim, 1)));
            break;
        }
        sim.step(base());
    }
    sim.world_mut().insert_resource(SeatsAtTheDoors {
        at_alices_commit,
        bob_accepted,
        at_bobs_arrival,
    });
    room
}

/// OW1, customer 2: a door holds only the seat that went through it. Alice
/// (seat 0) goes through the door to the hub and leaves Bob (seat 1) in
/// `switch_lab`. Her seat waits out the crossing cooldown and his does not.
/// Bob, put in his room's door to the hub, goes through it on his first
/// press, while Alice's seat still waits. Then his own seat waits. Before
/// this, the cooldown was one countdown for the whole world, and every seat
/// waited after any seat's door. The control is
/// `a_seat_cannot_cross_back_inside_its_own_cooldown`.
#[test]
fn a_crossing_holds_only_the_seat_that_crossed() {
    let (sim, _) = alice_leaves_bob_by(
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        alice_and_then_bob_go_through_their_doors,
    );
    let seats = sim.world().resource::<SeatsAtTheDoors>();
    assert_eq!(
        seats.at_alices_commit,
        (true, false),
        "Alice's crossing did not hold her seat, and only hers ({seats:?})"
    );
    let (tick, alice_waits) = seats
        .bob_accepted
        .unwrap_or_else(|| panic!("Bob's press in his door was never accepted ({seats:?})"));
    assert!(
        alice_waits,
        "Bob's crossing was accepted only on press {tick}, after Alice's seat stopped waiting: \
         her cooldown held his seat ({seats:?})"
    );
    assert_eq!(
        seats.at_bobs_arrival.map(|(_, bob)| bob),
        Some(true),
        "Bob's crossing did not hold his own seat ({seats:?})"
    );
}

/// The control for `a_crossing_holds_only_the_seat_that_crossed`: the seat
/// that crossed still waits. Alice, alone, goes through the door to the hub
/// and is put in the hub's door back to `switch_lab`, pressing interact on
/// every tick. No crossing is accepted while her seat waits, and one is
/// accepted after, so the door is one she can go through.
#[test]
fn a_seat_cannot_cross_back_inside_its_own_cooldown() {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .expect("switch_lab boots");
    for _ in 0..10 {
        sim.step(base());
    }
    assert_eq!(walk_through_the_door_to(&mut sim, HUB), HUB);
    assert!(seat_waits(&sim, 0), "precondition: Alice's crossing did not hold her seat");
    let back = door_of(&mut sim, HUB, ROOM).aabb.center();
    sim.teleport_player((back.x, back.y));
    // The seat counts down inside the step, before the door detector reads
    // it: the seat waits through a step when more than one step is left.
    let step = 1.0 / 60.0;
    let mut waited = 0;
    let mut accepted_while_waiting = None;
    let mut accepted = false;
    for tick in 1..=60 {
        let waits = sim
            .world()
            .resource::<ambition_platformer2d::platformer::safe_position::RoomTransitionCooldown>()
            .remaining(0)
            > step;
        let room = sim
            .step(ambition_app::AgentAction {
                interact: true,
                interact_held: true,
                ..base()
            })
            .active_room;
        if a_crossing_is_pending(&sim) || room != HUB {
            if waits {
                accepted_while_waiting = Some(tick);
            }
            accepted = true;
            break;
        }
        waited += usize::from(waits);
    }
    assert_eq!(
        accepted_while_waiting, None,
        "Alice went back through the door inside her own cooldown"
    );
    assert!(waited > 0, "precondition: Alice's seat never waited at the door back");
    assert!(accepted, "control: Alice never went back through the hub's door to switch_lab");
}

/// Alice talks to the hub's dog. Returns whether the conversation opened.
fn alice_talks_to_the_dog(sim: &mut Platformer2dSimHarness) -> bool {
    let dog = {
        let world = sim.world_mut();
        let mut query = world.query::<(bevy::prelude::Entity, &ambition_platformer2d::characters::actor::WornCharacter)>();
        query
            .iter(world)
            .find(|(_, worn)| worn.id() == "npc_companion_dog")
            .map(|(entity, _)| entity)
            .expect("the hub stages the dog")
    };
    let here = sim
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(dog)
        .expect("the dog has a body")
        .pos;
    sim.teleport_player((here.x, here.y));
    sim.step(ambition_app::AgentAction {
        interact: true,
        interact_held: true,
        ..base()
    });
    for _ in 0..10 {
        sim.step(base());
    }
    sim.world()
        .get_resource::<ambition_platformer2d::conversation::ActiveConversation>()
        .is_some_and(|conversation| conversation.talker() == Some(dog))
}

/// The session's game mode now.
fn game_mode(sim: &Platformer2dSimHarness) -> Option<ambition_platformer2d::platformer::schedule::GameMode> {
    sim.world()
        .get_resource::<bevy::state::state::State<ambition_platformer2d::platformer::schedule::GameMode>>()
        .map(|mode| *mode.get())
}

/// OW1 cut 7u: Alice's conversation does not stop Bob's room. Alice talks to
/// the hub's dog (#1) while Bob, on slot 1, stands in `switch_lab`'s door to
/// the hub (#0) and his seat presses: he goes through and joins the hub. A
/// conversation set `GameMode::Dialogue`, and every gameplay-gated system
/// of every live room (the door detector among them) stops in that mode, so
/// while another player talked in another room, Bob could not take a door.
/// The talker's own presses are withheld by the dialogue input context, not
/// by the mode. The control: Alice alone still enters the dialogue mode.
#[test]
fn a_conversation_in_one_room_does_not_stop_the_other_players_room() {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let (mut sim, _) = alice_leaves_bob(None);
    assert!(alice_talks_to_the_dog(&mut sim), "control: Alice alone did not talk to the dog");
    assert_eq!(
        game_mode(&sim),
        Some(ambition_platformer2d::platformer::schedule::GameMode::Dialogue),
        "control: a conversation with one live room is not modal"
    );

    let (mut sim, first) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    let second = first.next();
    assert!(alice_talks_to_the_dog(&mut sim), "precondition: Alice did not talk to the dog");
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
    let mut talking_while_he_pressed = true;
    for _ in 0..60 {
        talking_while_he_pressed &= sim
            .world()
            .get_resource::<ambition_platformer2d::conversation::ActiveConversation>()
            .is_some_and(|conversation| conversation.talker().is_some());
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
    sim.drive_seat(1, ambition_platformer2d::engine_core::ControlFrame::default());
    assert!(talking_while_he_pressed, "precondition: Alice's conversation ended before Bob crossed");
    assert_eq!(
        where_they_are(&mut sim).1,
        Some(Some(second)),
        "Bob did not go through his door while Alice talked in the other room (mode {:?})",
        game_mode(&sim)
    );
}

/// Plant a sentry with a 1000 s lifetime in the sole live room, by the road
/// production spawns a module entity by. Returns that room.
fn plant_a_long_lived_sentry(sim: &mut Platformer2dSimHarness) -> LiveRoomInstance {
    use ambition_platformer2d::abilities::module_entity::{spawn_module_entity, ModuleEntity, Spawner};
    sim.step_n(base(), 5);
    let world = sim.world_mut();
    let room = *ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<LiveRoomInstance>(world)
        .expect("the session has one live room");
    let scope = world
        .get_resource::<ambition_platformer2d::platformer::lifecycle::ActiveSessionScope>()
        .map_or(ambition_platformer2d::platformer::lifecycle::SessionSpawnScope::UNSCOPED, |scope| {
            scope.spawn_scope()
        })
        .in_room(Some(room));
    let mut commands = world.commands();
    spawn_module_entity(
        &mut commands,
        ModuleEntity {
            kind: "sentry".into(),
            pos: bevy::math::Vec2::new(300.0, 300.0),
            remaining_s: 1000.0,
        },
        Spawner {
            scope,
            side: ambition_platformer2d::actor::ActorFaction::Player,
            team: None,
            presentation: None,
            id: ambition_platformer2d::platformer::sim_id::SimId::placement("ow1_long_lived_sentry"),
        },
    );
    world.flush();
    sim.rebase_rollback_history().expect("the rollback history rebases over the sentry");
    room
}

/// Every module entity: its live room and the lifetime it has left.
fn module_entities(sim: &mut Platformer2dSimHarness) -> Vec<(Option<LiveRoomInstance>, f32)> {
    let world = sim.world_mut();
    world
        .query::<(&ambition_platformer2d::abilities::module_entity::ModuleEntity, Option<&InRoomInstance>)>()
        .iter(world)
        .map(|(entity, room)| (room.map(|room| room.0), entity.remaining_s))
        .collect()
}

/// Bob, on slot 1, goes through `switch_lab`'s door to the hub, as
/// `the_second_player_goes_through_a_door_of_his_own_room` sends him.
pub(crate) fn bob_goes_to_the_hub(sim: &mut Platformer2dSimHarness, hub: LiveRoomInstance) {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let door = door_of(sim, ROOM, HUB).aabb.center();
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
        if where_they_are(sim).1 == Some(Some(hub)) {
            break;
        }
    }
    sim.drive_seat(1, ambition_platformer2d::engine_core::ControlFrame::default());
    sim.step_n(base(), 30);
}

/// OW1 cut 7v: a module entity retires with its live room, not with a
/// player. A sentry with 1000 s left is planted in `switch_lab` (#0).
/// Alone, Alice goes to the hub: #0 retires, and the sentry with it. With
/// Bob holding #0 on slot 1, the sentry stays and ticks on; when Bob goes
/// to the hub too, #0 retires and the sentry with it. A module entity was
/// spawned session-scoped and stamped into its room, but not room-scoped, so
/// no retirement swept it: it ticked on in a room that was gone until its
/// timer ran out.
#[test]
fn a_module_entity_retires_with_its_room_and_not_with_a_player() {
    let mut planted = None;
    let (mut sim, first) = alice_leaves_bob_after(None, |sim| planted = Some(plant_a_long_lived_sentry(sim)));
    assert_eq!(planted, Some(first), "precondition: the sentry is not in switch_lab");
    assert_eq!(live_rooms(&mut sim).len(), 1, "precondition: switch_lab did not retire behind Alice");
    assert_eq!(module_entities(&mut sim), Vec::new(), "a sentry outlived the room it was in");

    let mut planted = None;
    let (mut sim, held) = alice_leaves_bob_after(
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        |sim| planted = Some(plant_a_long_lived_sentry(sim)),
    );
    assert_eq!(planted, Some(held), "precondition: the sentry is not in the room Bob holds");
    let before = module_entities(&mut sim);
    sim.step_n(base(), 30);
    let after = module_entities(&mut sim);
    assert!(
        matches!((before.as_slice(), after.as_slice()), ([(Some(a), r0)], [(Some(b), r1)]) if *a == held && *b == held && r1 < r0),
        "the sentry in the room Bob holds did not stay and tick: {before:?} then {after:?}"
    );
    bob_goes_to_the_hub(&mut sim, held.next());
    assert_eq!(
        live_rooms(&mut sim).iter().map(|(room, _)| *room).collect::<Vec<_>>(),
        vec![held.next()],
        "precondition: switch_lab did not retire behind Bob"
    );
    assert_eq!(module_entities(&mut sim), Vec::new(), "a sentry outlived the room it was in");
}

/// Where view `id` looks: its follow point, and the centre of its frame.
fn view_frame(
    sim: &mut Platformer2dSimHarness,
    id: ambition_platformer2d::sim_view::LocalViewId,
) -> Option<(ambition_platformer2d::engine_core::Vec2, ambition_platformer2d::engine_core::Vec2)> {
    let world = sim.world_mut();
    world
        .query::<(
            &ambition_platformer2d::sim_view::LocalViewId,
            &ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot,
        )>()
        .iter(world)
        .find(|(view, _)| **view == id)
        .and_then(|(_, resolved)| resolved.0.as_ref().map(|frame| (frame.follow_world, frame.snapshot.center_world)))
}

/// Each view: its id, the seat it follows, and whether the live-room split
/// opened it.
fn the_views(sim: &mut Platformer2dSimHarness) -> Vec<(u8, Option<u8>, bool)> {
    let world = sim.world_mut();
    let mut views: Vec<(u8, Option<u8>, bool)> = world
        .query_filtered::<(
            &ambition_platformer2d::sim_view::LocalViewId,
            Option<&ambition_platformer2d::sim_view::ViewParticipant>,
            bevy::prelude::Has<ambition_platformer2d::sim_view::SplitForLiveRoom>,
        ), bevy::prelude::With<ambition_platformer2d::sim_view::LocalView>>()
        .iter(world)
        .map(|(id, seat, opened)| (id.0, seat.map(|seat| seat.0 .0), opened))
        .collect();
    views.sort();
    views
}

/// View half, cut V5: when Alice crosses a door and Bob stays, the screen
/// splits: a second view opens and follows Bob's seat. When Bob comes to the
/// hub, the players are in one room again and it closes. The control: before
/// the crossing, and when Bob crosses with her, there is one view.
#[test]
fn a_second_view_opens_while_the_players_are_in_two_rooms_and_closes_when_they_meet() {
    let (mut sim, _) = alice_leaves_bob(None);
    sim.step_n(base(), 2);
    assert_eq!(the_views(&mut sim), vec![(0, None, false)], "control: one player, one room, one view");

    let (mut sim, held) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    sim.step_n(base(), 2);
    assert_eq!(live_rooms(&mut sim).len(), 2, "precondition: Bob's room did not stay live");
    assert_eq!(
        the_views(&mut sim),
        vec![(0, None, false), (1, Some(1), true)],
        "Alice and Bob are in two rooms, and Bob has no view of his own"
    );
    bob_goes_to_the_hub(&mut sim, held.next());
    assert_eq!(live_rooms(&mut sim).len(), 1, "precondition: Bob did not join Alice");
    assert_eq!(the_views(&mut sim), vec![(0, None, false)], "the players met, and the split did not close");
}

/// Q150: each view's HUD shows the purse of the participant it follows.
/// Alice crosses and Bob stays; the split opens Bob's view, and its HUD
/// facts show Bob's purse while view 0 shows Alice's. Before, one HUD showed
/// the controlled body (Alice) for every view.
#[test]
fn each_view_of_the_split_shows_its_own_participants_purse() {
    use ambition_platformer2d::characters::actor::BodyWallet;
    let (mut sim, _) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    {
        let world = sim.world_mut();
        let mut bodies = world.query::<(
            bevy::prelude::Entity,
            Option<&ambition_platformer2d::combat::components::FeatureId>,
            bevy::prelude::Has<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        )>();
        let (alice, bob) = {
            let rows: Vec<_> = bodies.iter(world).collect();
            let alice = rows.iter().find(|(_, _, primary)| *primary).map(|(body, ..)| *body);
            let bob = rows
                .iter()
                .find(|(_, feature, _)| feature.is_some_and(|feature| feature.0 == BOB))
                .map(|(body, ..)| *body);
            (alice.expect("Alice's body"), bob.expect("Bob's body"))
        };
        world.entity_mut(alice).insert(BodyWallet { balance: 3 });
        world.entity_mut(bob).insert(BodyWallet { balance: 11 });
    }
    sim.step_n(base(), 2);
    let world = sim.world_mut();
    let mut shown: Vec<(u8, bool, i32)> = world
        .query_filtered::<(
            &ambition_platformer2d::sim_view::LocalViewId,
            &ambition_platformer2d::sim_view::ViewHudFacts,
        ), bevy::prelude::With<ambition_platformer2d::sim_view::LocalView>>()
        .iter(world)
        .map(|(id, facts)| (id.0, facts.0.present, facts.0.balance))
        .collect();
    shown.sort();
    assert_eq!(
        shown,
        vec![(0, true, 3), (1, true, 11)],
        "(view, its HUD shows a body, the purse it shows): view 0 follows Alice, view 1 Bob's seat"
    );
}

/// Q150 on a merged screen: Alice and Bob in one live room share view 0,
/// and Bob's purse is on it beside Alice's (`SharedViewHudFacts`). The
/// control is the split: when Alice leaves, Bob's view shows his purse and
/// view 0 shows no other participant.
#[test]
fn bob_beside_alice_has_his_own_hud_on_the_shared_view() {
    use ambition_platformer2d::characters::actor::BodyWallet;
    fn give_purses(sim: &mut Platformer2dSimHarness) {
        let world = sim.world_mut();
        let mut bodies = world.query::<(
            bevy::prelude::Entity,
            Option<&ambition_platformer2d::combat::components::FeatureId>,
            bevy::prelude::Has<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        )>();
        let rows: Vec<_> = bodies.iter(world).collect();
        let alice = rows.iter().find(|(_, _, primary)| *primary).map(|(body, ..)| *body);
        let bob = rows
            .iter()
            .find(|(_, feature, _)| feature.is_some_and(|feature| feature.0 == BOB))
            .map(|(body, ..)| *body);
        world.entity_mut(alice.expect("Alice's body")).insert(BodyWallet { balance: 3 });
        world.entity_mut(bob.expect("Bob's body")).insert(BodyWallet { balance: 11 });
    }
    fn shown(sim: &mut Platformer2dSimHarness) -> Vec<(u8, i32, Vec<(u8, i32)>)> {
        let world = sim.world_mut();
        let mut shown: Vec<_> = world
            .query_filtered::<(
                &ambition_platformer2d::sim_view::LocalViewId,
                &ambition_platformer2d::sim_view::ViewHudFacts,
                &ambition_platformer2d::sim_view::SharedViewHudFacts,
            ), bevy::prelude::With<ambition_platformer2d::sim_view::LocalView>>()
            .iter(world)
            .map(|(id, own, shared)| {
                let others = shared.0.iter().map(|(slot, facts)| (slot.0, facts.balance)).collect();
                (id.0, own.0.balance, others)
            })
            .collect();
        shown.sort();
        shown
    }
    let (mut sim, _) = alice_beside_bob();
    give_purses(&mut sim);
    sim.step_n(base(), 2);
    assert_eq!(
        shown(&mut sim),
        vec![(0, 3, vec![(1, 11)])],
        "(view, its own purse, the other seats on it) with Alice and Bob in one room"
    );
    let (mut sim, _) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    give_purses(&mut sim);
    sim.step_n(base(), 2);
    assert_eq!(
        shown(&mut sim),
        vec![(0, 3, vec![]), (1, 11, vec![])],
        "control: in two rooms each has a view of his own"
    );
}

/// Where Alice's and Bob's bodies are. Bob's is `None` when his body is gone.
fn their_positions(
    sim: &mut Platformer2dSimHarness,
) -> (ambition_platformer2d::engine_core::Vec2, Option<ambition_platformer2d::engine_core::Vec2>) {
    let world = sim.world_mut();
    let alice = world
        .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .expect("Alice's body is in the world")
        .pos;
    let bob = world
        .query::<(&ambition_platformer2d::combat::components::FeatureId, &ambition_platformer2d::engine_core::BodyKinematics)>()
        .iter(world)
        .find(|(feature, _)| feature.0 == BOB)
        .map(|(_, kinematics)| kinematics.pos);
    (alice, bob)
}

/// View half, cut V1: each view frames its own player while two rooms are
/// live. Alice is in the hub (#1) and Bob holds `switch_lab` (#0). The first
/// view frames the session's player, Alice; a second view follows slot 1,
/// Bob. Both run for 30 ticks, and each view's follow point moves with its
/// own player, by the geometry of that player's own room. The control: the
/// first view follows Alice when she is alone. Before V1 the camera resolve
/// read the sole live room, so it did not run while two rooms were live and
/// both views stayed at the frame before the crossing.
#[test]
fn each_view_frames_its_own_player_while_two_rooms_are_live() {
    use ambition_platformer2d::sim_view::LocalViewId;
    let run = |sim: &mut Platformer2dSimHarness| {
        for _ in 0..30 {
            sim.drive_seat(
                1,
                ambition_platformer2d::engine_core::ControlFrame {
                    axis_x: -1.0,
                    ..Default::default()
                },
            );
            sim.step(ambition_app::AgentAction { move_x: 1.0, ..base() });
        }
        sim.drive_seat(1, ambition_platformer2d::engine_core::ControlFrame::default());
    };
    // How far a view's follow point moved beside how far its player moved.
    let followed = |before: Option<(ambition_platformer2d::engine_core::Vec2, _)>,
                    after: Option<(ambition_platformer2d::engine_core::Vec2, _)>,
                    moved: ambition_platformer2d::engine_core::Vec2| {
        let (Some((from, _)), Some((to, _))) = (before, after) else {
            return None;
        };
        Some(((to - from) - moved).length())
    };

    let (mut sim, _) = alice_leaves_bob(None);
    let (alice, _) = their_positions(&mut sim);
    let before = view_frame(&mut sim, LocalViewId::FIRST);
    for _ in 0..30 {
        sim.step(ambition_app::AgentAction { move_x: 1.0, ..base() });
    }
    let moved = their_positions(&mut sim).0 - alice;
    assert!(moved.x > 20.0, "precondition: Alice did not run alone ({moved:?})");
    let lag = followed(before, view_frame(&mut sim, LocalViewId::FIRST), moved);
    assert!(lag.is_some_and(|lag| lag < 4.0), "control: the view did not follow Alice alone: {lag:?}");

    let (mut sim, first) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    assert_eq!(live_rooms(&mut sim).len(), 2, "precondition: Bob's room did not stay live");
    // Bob's view is the one the live-room split opened (V5).
    sim.step(base());
    let (alice, bob) = their_positions(&mut sim);
    let bob = bob.expect("Bob's body is in the world");
    let (alice_view, bob_view) = (view_frame(&mut sim, LocalViewId::FIRST), view_frame(&mut sim, LocalViewId(1)));
    run(&mut sim);
    let (alice_now, bob_now) = their_positions(&mut sim);
    let bob_now = bob_now.expect("Bob's body is in the world");
    assert!(
        (alice_now - alice).x > 20.0 && (bob_now - bob).x < -20.0,
        "precondition: Alice and Bob did not both run ({:?}, {:?})",
        alice_now - alice,
        bob_now - bob
    );
    let lags = (
        followed(alice_view, view_frame(&mut sim, LocalViewId::FIRST), alice_now - alice),
        followed(bob_view, view_frame(&mut sim, LocalViewId(1)), bob_now - bob),
    );
    assert!(
        matches!(lags, (Some(a), Some(b)) if a < 4.0 && b < 4.0),
        "a view did not follow its own player while two rooms were live (Bob in {first:?}): {lags:?}"
    );

    // Bob's view is clamped by Bob's room. Bob stands at the far right of
    // `switch_lab`, and the right edge of his view is that room's right wall.
    let sizes: Vec<(LiveRoomInstance, ambition_platformer2d::engine_core::Vec2)> = {
        let world = sim.world_mut();
        world
            .query_filtered::<(&LiveRoomInstance, &ambition_platformer2d::engine_core::RoomGeometry), bevy::prelude::With<RoomInstanceRoot>>()
            .iter(world)
            .map(|(room, geometry)| (*room, geometry.0.size))
            .collect()
    };
    let lab = sizes.iter().find(|(room, _)| *room == first).expect("switch_lab is live").1;
    put_bob_at(&mut sim, ambition_platformer2d::engine_core::Vec2::new(lab.x - 40.0, bob_now.y));
    sim.step_n(base(), 10);
    let frame = {
        let world = sim.world_mut();
        world
            .query::<(&LocalViewId, &ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot)>()
            .iter(world)
            .find(|(view, _)| **view == LocalViewId(1))
            .and_then(|(_, resolved)| resolved.0.as_ref().map(|frame| frame.snapshot.clone()))
            .expect("Bob's view resolved a frame")
    };
    let right = frame.center_world.x + frame.visible_view.x * 0.5;
    assert!(
        (right - lab.x).abs() < 1.0,
        "Bob's view is not clamped by switch_lab: its right edge is {right}, the room is {lab:?} (live rooms {sizes:?})"
    );
}

/// Each body's movement effects are drawn in its own live room (view half,
/// cut V2f). Alice (the hub) jumps, and Bob (the first room, driven by slot 1)
/// lands from a height. Every dust row names the room of the body that raised it, so Alice's
/// view draws her dust and Bob's view draws his. An unroomed row is drawn in
/// no room while two are live.
#[test]
fn each_body_s_movement_dust_is_drawn_in_its_own_live_room() {
    use ambition_platformer2d::vfx::vfx::{VfxInRoom, VfxMessage};
    let (mut sim, _) = alice_leaves_bob_for_a_replay();
    let (alice, bob) = where_they_are(&mut sim);
    let bob = bob.flatten();
    assert!(
        alice.is_some() && bob.is_some() && alice != bob,
        "precondition: Alice and Bob are in two live rooms ({alice:?}, {bob:?})"
    );
    // Bob lands from a height: the same movement emitter raises his dust.
    let bob_spot = {
        let world = sim.world_mut();
        world
            .query::<(&ambition_platformer2d::combat::components::FeatureId, &ambition_platformer2d::engine_core::BodyKinematics)>()
            .iter(world)
            .find(|(feature, _)| feature.0 == BOB)
            .map(|(_, kinematics)| kinematics.pos)
            .expect("Bob's body is in the world")
    };
    put_bob_at(&mut sim, bob_spot - ambition_platformer2d::engine_core::Vec2::new(0.0, 120.0));
    let mut rooms = std::collections::BTreeSet::new();
    let mut unroomed = 0usize;
    for tick in 0..120 {
        let press = tick % 20 == 0;
        let mut action = base();
        action.jump = press;
        action.jump_held = tick % 20 < 8;
        sim.step(action);
        let world = sim.world_mut();
        for row in world
            .resource::<bevy::ecs::message::Messages<VfxInRoom>>()
            .iter_current_update_messages()
            .filter(|row| matches!(row.vfx, VfxMessage::Dust { .. }))
        {
            match row.room {
                Some(room) => {
                    rooms.insert(room);
                }
                None => unroomed += 1,
            }
        }
    }
    assert_eq!(
        (rooms, unroomed),
        ([alice.unwrap(), bob.unwrap()].into_iter().collect(), 0),
        "(the rooms dust was drawn in, unroomed dust rows): each body's dust must name its own live room"
    );
}

/// The rooms that the effect rows `keep` accepts were written for, in the
/// next `ticks` ticks. `None` is an unroomed row.
fn rooms_of_effect_rows(
    sim: &mut Platformer2dSimHarness,
    ticks: usize,
    keep: fn(&ambition_platformer2d::vfx::vfx::VfxMessage) -> bool,
) -> std::collections::BTreeSet<Option<LiveRoomInstance>> {
    let mut rooms = std::collections::BTreeSet::new();
    for _ in 0..ticks {
        sim.step(base());
        rooms.extend(
            sim.world()
                .resource::<bevy::ecs::message::Messages<ambition_platformer2d::vfx::vfx::VfxInRoom>>()
                .iter_current_update_messages()
                .filter(|row| keep(&row.vfx))
                .map(|row| row.room),
        );
    }
    rooms
}

/// The effects of a hit are drawn in the struck body's own live room (view
/// half, cut V2f, the producers that write through a helper). Alice is in
/// the hub and Bob, driven by slot 1, is in the first room. A hit on Bob
/// (the actor road) writes its impact for Bob's room. A hit on Alice (the
/// player road) writes its impact for Alice's room, and so does the reset
/// effect of her hazard respawn. No row is unroomed: an unroomed row is
/// drawn in no room while two are live.
#[test]
fn each_hit_s_effects_are_drawn_in_the_struck_body_s_own_live_room() {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    use ambition_platformer2d::vfx::vfx::VfxMessage;
    let (mut sim, _) = alice_leaves_bob_for_a_replay();
    let (alice_room, bob_room) = where_they_are(&mut sim);
    let (alice_room, bob_room) = (
        alice_room.expect("Alice is in a live room"),
        bob_room.flatten().expect("Bob is in a live room"),
    );
    assert!(
        alice_room != bob_room && live_rooms(&mut sim).len() == 2,
        "precondition: Alice and Bob are in two live rooms ({alice_room:?}, {bob_room:?})"
    );
    let strike = |sim: &mut Platformer2dSimHarness, alice: bool, mode: HitMode| {
        let world = sim.world_mut();
        let (victim, pos, room) = if alice {
            world
                .query_filtered::<(bevy::prelude::Entity, &ambition_platformer2d::engine_core::BodyKinematics), bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
                .single(world)
                .map(|(entity, kinematics)| (entity, kinematics.pos, alice_room))
                .expect("Alice's body is in the world")
        } else {
            world
                .query::<(bevy::prelude::Entity, &ambition_platformer2d::combat::components::FeatureId, &ambition_platformer2d::engine_core::BodyKinematics)>()
                .iter(world)
                .find(|(_, feature, _)| feature.0 == BOB)
                .map(|(entity, _, kinematics)| (entity, kinematics.pos, bob_room))
                .expect("Bob's body is in the world")
        };
        world.write_message(HitEvent {
            volume: ambition_platformer2d::engine_core::Aabb::new(pos, ambition_platformer2d::engine_core::Vec2::splat(8.0)).into(),
            damage: 1,
            source: HitSource::Hazard,
            attacker: None,
            room: Some(room),
            target: HitTarget::Body(victim),
            mode,
            knockback: None,
            ignored_targets: Vec::new(),
            strike_sfx: None,
            attacker_move_instance: None,
        });
    };
    let impact: fn(&VfxMessage) -> bool = |vfx| matches!(vfx, VfxMessage::Impact { .. });
    let reset: fn(&VfxMessage) -> bool = |vfx| matches!(vfx, VfxMessage::ResetEffects { .. });

    strike(&mut sim, false, HitMode::Knockback);
    assert_eq!(
        rooms_of_effect_rows(&mut sim, 4, impact),
        [Some(bob_room)].into_iter().collect(),
        "the rooms the impact of a hit on Bob was written for (Bob is in {bob_room:?}, Alice in {alice_room:?})"
    );
    // The impact of the hit on Bob is drawn and gone before Alice is struck.
    for _ in 0..90 {
        sim.step(base());
    }
    strike(&mut sim, true, HitMode::Knockback);
    assert_eq!(
        rooms_of_effect_rows(&mut sim, 4, impact),
        [Some(alice_room)].into_iter().collect(),
        "the rooms the impact of a hit on Alice was written for (Alice is in {alice_room:?}, Bob in {bob_room:?})"
    );
    // Her invulnerability after the hit ends before the hazard strikes.
    for _ in 0..180 {
        sim.step(base());
    }
    strike(&mut sim, true, HitMode::SafeRespawn);
    assert_eq!(
        rooms_of_effect_rows(&mut sim, 10, reset),
        [Some(alice_room)].into_iter().collect(),
        "the rooms the reset effect of Alice's hazard respawn was written for (Alice is in {alice_room:?})"
    );
}

thread_local! {
    /// The rooms the reset effects of the last recorded crossing were written
    /// for. See [`walk_through_the_door_recording_its_reset_effects`].
    static CROSSING_RESET_EFFECT_ROOMS: std::cell::RefCell<Vec<Option<LiveRoomInstance>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// `walk_through_the_door_to`, which also records the room of each
/// `ResetEffects` row of the crossing (the tick that changes the room and
/// the four ticks after it).
fn walk_through_the_door_recording_its_reset_effects(sim: &mut Platformer2dSimHarness, target: &str) -> String {
    use ambition_platformer2d::engine_core::AabbExt as _;
    use ambition_platformer2d::vfx::vfx::{VfxInRoom, VfxMessage};
    let record = |sim: &Platformer2dSimHarness| {
        let rooms: Vec<_> = sim
            .world()
            .resource::<bevy::ecs::message::Messages<VfxInRoom>>()
            .iter_current_update_messages()
            .filter(|row| matches!(row.vfx, VfxMessage::ResetEffects { .. }))
            .map(|row| row.room)
            .collect();
        CROSSING_RESET_EFFECT_ROOMS.with(|recorded| recorded.borrow_mut().extend(rooms));
    };
    CROSSING_RESET_EFFECT_ROOMS.with(|recorded| recorded.borrow_mut().clear());
    let before = sim.observation().active_room.clone();
    let center = crate::common::door_to(sim, target).aabb.center();
    sim.teleport_player((center.x, center.y));
    for _ in 0..120 {
        let room = sim.step(ambition_app::AgentAction { interact: true, interact_held: true, ..base() }).active_room;
        record(sim);
        if room != before {
            for _ in 0..4 {
                sim.step(base());
                record(sim);
            }
            return room;
        }
    }
    panic!("held interact inside the door of '{before}' to '{target}' for 120 frames and the room never changed");
}

/// The arrival effect of a crossing is drawn in the live room the body
/// arrives in (view half, cut V2f). Alice goes through the door to the hub
/// while Bob, driven by slot 1, stays: two rooms are live when she arrives,
/// and the reset effect of her arrival names the hub's live room. An
/// unroomed row is drawn in no room while two are live.
#[test]
fn the_arrival_effect_of_a_crossing_is_drawn_in_the_room_the_body_arrives_in() {
    let (mut sim, first) = alice_leaves_bob_by(
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_recording_its_reset_effects,
    );
    let (alice_room, bob_room) = where_they_are(&mut sim);
    let alice_room = alice_room.expect("Alice is in a live room");
    assert!(
        bob_room == Some(Some(first)) && alice_room != first && live_rooms(&mut sim).len() == 2,
        "precondition: Alice and Bob are in two live rooms ({alice_room:?}, {bob_room:?})"
    );
    let recorded = CROSSING_RESET_EFFECT_ROOMS.with(|recorded| recorded.borrow().clone());
    assert_eq!(
        recorded,
        vec![Some(alice_room)],
        "the rooms the reset effects of Alice's crossing were written for (she arrived in {alice_room:?}, Bob holds {first:?})"
    );
}

/// The reset effects of a room replay are drawn in the live room of the
/// player who replays (view half, cut V2f). Alice, hurt and away from the hub
/// spawn beside Bob's live room, asks for a replay. The replay writes two
/// reset effects: one where she is put back, in the room she replays, and
/// one where she arrives, in the room the replay builds. Each names the room
/// she is in on the tick it is written, and no row is unroomed.
#[test]
fn the_reset_effects_of_a_replay_are_drawn_in_the_players_own_live_room() {
    use ambition_platformer2d::vfx::vfx::{VfxInRoom, VfxMessage};
    let (mut sim, _) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    alice_walks_off_hurt_in_the_hub(&mut sim);
    let before = where_they_are(&mut sim).0.expect("Alice is in a live room");
    sim.world_mut().write_message(ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual());
    // (the room a reset effect was written for, Alice's room on that tick)
    let mut rows: Vec<(Option<LiveRoomInstance>, Option<LiveRoomInstance>)> = Vec::new();
    for _ in 0..30 {
        sim.step(base());
        let written: Vec<_> = sim
            .world()
            .resource::<bevy::ecs::message::Messages<VfxInRoom>>()
            .iter_current_update_messages()
            .filter(|row| matches!(row.vfx, VfxMessage::ResetEffects { .. }))
            .map(|row| row.room)
            .collect();
        let hers = where_they_are(&mut sim).0;
        rows.extend(written.into_iter().map(|room| (room, hers)));
    }
    let after = where_they_are(&mut sim).0.expect("Alice is in a live room");
    assert_eq!(live_rooms(&mut sim).len(), 2, "precondition: Bob's room did not stay live through the replay");
    assert_eq!(
        rows,
        vec![(Some(before), Some(before)), (Some(after), Some(after))],
        "(the room each reset effect of Alice's replay was written for, her room on that tick): she replayed {before:?} and the replay built {after:?}"
    );
}

/// Alice dies in the hub while Bob, driven by slot 1, plays on in
/// `switch_lab`. Bob's body is not a `PlayerEntity`, so the roster finds
/// nobody left in play and the death resets to the checkpoint: Alice starts
/// again in a NEW instance of the hub, and Bob's room is the same live room
/// with the same body in it. One player's death does not take the other
/// player's room. Whether Alice should instead wait for Bob is Q151.
#[test]
fn a_death_in_one_room_restarts_that_player_and_leaves_the_other_players_room() {
    let (mut sim, first) = alice_leaves_bob(Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    let rooms_before = live_rooms(&mut sim);
    let alice_room = where_they_are(&mut sim).0.expect("Alice is in a live room");
    assert_eq!(
        rooms_before,
        vec![(first, ROOM.to_string()), (alice_room, HUB.to_string())],
        "precondition: Bob holds {ROOM} and Alice the hub"
    );
    let bob_body = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world
            .query::<(bevy::prelude::Entity, &ambition_platformer2d::combat::components::FeatureId)>()
            .iter(world)
            .find(|(_, feature)| feature.0 == BOB)
            .map(|(entity, _)| entity)
    };
    let bob = bob_body(&mut sim).expect("Bob's body is in the world");
    let alice = {
        let world = sim.world_mut();
        world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("Alice's body is in the world")
    };
    sim.world_mut().write_message(ambition_platformer2d::combat::death_rules::ActorDiedMessage {
        victim: alice,
        pos: ambition_platformer2d::engine_core::Vec2::new(0.0, 0.0),
        cause: ambition_platformer2d::combat::death_rules::DeathCause {
            source: ambition_platformer2d::combat::HitSource::Hazard,
            attacker: None,
        },
    });
    sim.step(base());
    assert!(
        sim.world().get::<ambition_platformer2d::combat::death_rules::OutOfPlay>(alice).is_some(),
        "precondition: the death took Alice out of play"
    );
    // The interlude, then the reset and the rebuild it asks for.
    sim.step_n(base(), 240);
    let rooms_after = live_rooms(&mut sim);
    let (alice_now, bob_now) = where_they_are(&mut sim);
    assert_eq!(bob_body(&mut sim), Some(bob), "Bob's body was rebuilt or removed by Alice's death");
    assert_eq!(bob_now, Some(Some(first)), "Bob left his live room when Alice died");
    let alice_now = alice_now.expect("Alice is in a live room again");
    assert_ne!(alice_now, alice_room, "Alice did not start again: her hub is the instance she died in");
    assert_eq!(
        rooms_after,
        vec![(first, ROOM.to_string()), (alice_now, HUB.to_string())],
        "Bob's room must stay the same live room, and Alice's must be a new hub"
    );
    let alice_again = {
        let world = sim.world_mut();
        world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("Alice's body is in the world")
    };
    assert!(
        sim.world().get::<ambition_platformer2d::combat::death_rules::OutOfPlay>(alice_again).is_none(),
        "Alice is still out of play after the reset"
    );
}

/// Alice replays (or resets to a checkpoint in) the room Bob is in, and
/// asks again for whatever leaves it: one live room per room
/// (`DefinitionAlreadyLive`, OW3).
fn one_room_after(reset: fn(&mut Platformer2dSimHarness)) -> (Vec<(LiveRoomInstance, String)>, (Option<LiveRoomInstance>, Option<Option<LiveRoomInstance>>)) {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .expect("switch_lab boots");
    let first = bob_beside_alice(&mut sim, ROOM, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    reset(&mut sim);
    for _ in 0..30 {
        sim.step(base());
    }
    let rooms = live_rooms(&mut sim);
    assert!(rooms.iter().all(|(room, _)| *room != first), "precondition: the reset did not rebuild the room: {rooms:?}");
    (rooms, where_they_are(&mut sim))
}

/// OW3: the durable rows name a place by its room id, so a room has at most
/// one live room. Alice and Bob (slot 1) stand in `switch_lab` (#0). A
/// replay, and a reset to a checkpoint, each rebuild it as #1 with both of
/// them in it: neither opens a second live room of `switch_lab` beside Bob's.
/// Measured before the refusal was added; it pins the roads it relies on.
#[test]
fn a_reset_beside_a_player_in_the_same_room_rebuilds_the_one_live_room() {
    let second = LiveRoomInstance::ACTIVATION.next();
    let replay = one_room_after(|sim| {
        sim.world_mut().write_message(ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual());
    });
    let checkpoint = one_room_after(|sim| {
        sim.world_mut().write_message(ambition_platformer2d::platformer::lifecycle::ResetToCheckpoint);
    });
    let one = (vec![(second, ROOM.to_string())], (Some(second), Some(Some(second))));
    assert_eq!((replay, checkpoint), (one.clone(), one), "(replay, checkpoint reset): (live rooms, (Alice's room, Bob's room))");
}

/// Nudge every body in the activation room (Bob's, once Alice has left it)
/// by a count that does not rewind, so a resimulated frame differs from the
/// first simulation of it.
/// Armed once Alice has left Bob's room, so the nudge reaches only his.
#[derive(bevy::prelude::Resource, Default)]
struct NudgeBobsRoom(bool);

fn nudge_bobs_room_from_outside_the_timeline(
    armed: bevy::prelude::Res<NudgeBobsRoom>,
    mut count: bevy::prelude::Local<u32>,
    mut bodies: bevy::prelude::Query<(&mut ambition_platformer2d::engine_core::BodyKinematics, &InRoomInstance)>,
) {
    if !armed.0 {
        return;
    }
    *count += 1;
    for (mut kin, room) in &mut bodies {
        if room.0 == LiveRoomInstance::ACTIVATION {
            kin.pos.x += (*count % 7) as f32 * 0.01;
        }
    }
}

/// (the rollback session ran, its health) after 300 frames with Alice in the
/// hub and Bob in `switch_lab`, under a sync test that rewinds and replays
/// every frame. With `poison`, a system nudges Bob's room from outside the
/// timeline.
fn two_rooms_under_a_sync_test(poison: bool) -> (bool, Result<(), String>) {
    use ambition_platformer2d::sim::SimScheduleExt;
    let options = fixed_60hz_room_options(ROOM)
        .with_save(a_save_that_has_seen_the_hub_intro())
        .with_sync_test_rollback_settings(4, 10);
    let sim = Platformer2dSimHarness::build(options, |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        app.init_resource::<NudgeBobsRoom>();
        if poison {
            let label = app.sim_schedule();
            app.add_systems(label, nudge_bobs_room_from_outside_the_timeline);
        }
        Ok(())
    })
    .expect("switch_lab boots under a sync test");
    let (mut sim, first) = alice_leaves_bob_with(
        sim,
        ROOM,
        HUB,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        walk_through_the_door_to,
    );
    assert_eq!(first, LiveRoomInstance::ACTIVATION, "precondition: Bob's room is the activation room");
    assert_eq!(live_rooms(&mut sim).len(), 2, "precondition: both rooms are live");
    sim.world_mut().resource_mut::<NudgeBobsRoom>().0 = true;
    // `try_step`: an unhealthy session refuses the step, and that refusal is
    // the reading, so the loop ends at it.
    for _ in 0..300 {
        if sim.try_step(base()).is_err() {
            break;
        }
    }
    (
        ambition_platformer2d::rollback::session_is_active(sim.world()),
        ambition_platformer2d::rollback::session_health(sim.world()),
    )
}

/// Two live rooms hold under a sync test. Both live room roots carry
/// `session:room_instance`, which is right under Q109: the live occurrence is
/// (`LiveRoomInstance`, `SimId`), and the construction baseline is scoped to
/// a transaction's rooms. This shows that both rooms' rollback state
/// resimulates to the same checksums. The control is the poison: a nudge of
/// Bob's room from outside the timeline is a mismatch, so the checksum covers
/// the room that is not the primary's. A single peer cannot show the order
/// two peers fold the roots in; that is the remote-peer row.
#[test]
fn two_live_rooms_hold_under_a_sync_test() {
    let (active, health) = two_rooms_under_a_sync_test(true);
    assert!(
        active && health.is_err(),
        "control: a nudge of Bob's room from outside the timeline must be a mismatch, \
         or this sync test does not see his room (active {active}, health {health:?})"
    );
    assert_eq!(two_rooms_under_a_sync_test(false), (true, Ok(())));
}

