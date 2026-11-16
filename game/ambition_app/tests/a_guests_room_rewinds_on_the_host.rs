//! SIMULATION-ISLANDS, production: the hosted world's room rollback host
//! (`ambition_hosted_world::room_rollback`).
//!
//! Alice (the host's seat 0) leaves for the hub; Bob, a guest at seat 1,
//! stays in `switch_lab`. Bob's stamped inputs reach the host [`LATENCY`]
//! frames after the tick they name. With `RewindsGuestRooms` the host
//! predicts them and rewinds Bob's room when one arrives different; the run
//! must end where the run with his inputs on time ends. The control is the
//! same late run without it: each input drives his seat when it arrives.

use std::collections::{BTreeMap, VecDeque};

use ambition_app::Platformer2dSimHarness;
use ambition_hosted_world::bevy_replicon::prelude::{ClientId, FromClient, RepliconPlugins, ServerPlugin};
use ambition_hosted_world::{GuestInput, GuestSeat, HostedRole, HostedWorldPlugin, RewindsGuestRooms, RoomRollbackStats};
use ambition_platformer2d::characters::control::PlayerSlot;
use ambition_platformer2d::engine_core::ControlFrame;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::{Messages, PluginGroup as _, PostUpdate};

use crate::common::fixed_60hz_room_options;
use crate::hosted_world::{alice_fights, alice_leaves, alice_on_the_hub_door, bobs, bodies, LATENCY};
use crate::two_players_two_live_rooms::{bob_beside_alice, where_they_are, ROOM};

/// Frames after the rooms part.
const FRAMES: i32 = 150;

/// When Bob's input reaches the host.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Arrival {
    OnTime,
    Late,
}

struct Run {
    ending: BTreeMap<SimId, [f32; 5]>,
    stats: RoomRollbackStats,
    apart: bool,
}

/// Where Alice plays while Bob's inputs arrive.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Alice {
    /// In the hub, standing: Bob's room holds only guests.
    Apart,
    /// Beside Bob, fighting: his room holds a host seat.
    Beside,
}

fn boot_host(room: &str) -> Platformer2dSimHarness {
    Platformer2dSimHarness::build(fixed_60hz_room_options(room), |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        app.add_plugins((RepliconPlugins.set(ServerPlugin::new(PostUpdate)), HostedWorldPlugin { role: HostedRole::Host }));
        app.finish();
        app.cleanup();
        Ok(())
    })
    .expect("the room boots")
}

fn host_with_rooms(alice: Alice) -> (Platformer2dSimHarness, bool) {
    let mut host = boot_host(ROOM);
    bob_beside_alice(&mut host, ROOM, Some(PlayerSlot(1)));
    if alice == Alice::Apart {
        alice_on_the_hub_door(&mut host);
        for frame in 1..=60 {
            host.drive_seat(0, alice_leaves(frame));
            host.drive_seat(1, ControlFrame::default());
            host.app_mut().update();
        }
    }
    let (hub, lab) = where_they_are(&mut host);
    let apart = hub.is_some() && lab.flatten().is_some() && hub != lab.flatten();
    (host, apart)
}

fn run(alice: Alice, arrival: Arrival, rewinds: bool) -> Run {
    let (mut host, apart) = host_with_rooms(alice);
    let client = host.world_mut().spawn(GuestSeat(PlayerSlot(1))).id();
    if rewinds {
        host.world_mut().insert_resource(RewindsGuestRooms);
    }
    let mut in_flight: VecDeque<(i32, GuestInput)> = VecDeque::new();
    let deliver = |host: &mut Platformer2dSimHarness, input: GuestInput| {
        host.world_mut()
            .resource_mut::<Messages<FromClient<GuestInput>>>()
            .write(FromClient { client_id: ClientId::Client(client), message: input });
    };
    for frame in 1..=FRAMES {
        let tick = host.world().resource::<ambition_platformer2d::time::SimTick>().0 + 1;
        in_flight.push_back((frame, GuestInput { sequence: tick, tick, frame: bobs(frame) }));
        let due = match arrival {
            Arrival::OnTime => frame,
            Arrival::Late => frame - LATENCY,
        };
        while in_flight.front().is_some_and(|(sent, _)| *sent <= due) {
            let (_, input) = in_flight.pop_front().expect("checked");
            deliver(&mut host, input);
        }
        host.drive_seat(0, if alice == Alice::Beside { alice_fights(frame) } else { ControlFrame::default() });
        host.app_mut().update();
    }
    // The last inputs arrive after the last frame; one more frame takes them
    // in, on time and late alike.
    for (_, input) in in_flight.drain(..) {
        deliver(&mut host, input);
    }
    host.drive_seat(0, ControlFrame::default());
    host.app_mut().update();
    let stats = host.world().get_resource::<RoomRollbackStats>().copied().unwrap_or_default();
    Run { ending: bodies(&mut host, None), stats, apart }
}

fn differing(expected: &BTreeMap<SimId, [f32; 5]>, got: &BTreeMap<SimId, [f32; 5]>) -> Vec<String> {
    expected
        .iter()
        .filter(|(id, state)| got.get(*id) != Some(*state))
        .map(|(id, state)| format!("{id:?}: on time {state:?}, late {:?}", got.get(id)))
        .collect()
}

/// One scene's three runs: on time, late with rewinds, late applied on
/// arrival. Returns the late run's stats and the bodies each late run gets
/// wrong.
fn scene(alice: Alice) -> (Run, Run, Vec<String>, Vec<String>) {
    let on_time = run(alice, Arrival::OnTime, true);
    let late = run(alice, Arrival::Late, true);
    let control = run(alice, Arrival::Late, false);
    let wrong = differing(&on_time.ending, &late.ending);
    let control_wrong = differing(&on_time.ending, &control.ending);
    println!(
        "[room rollback host] {}: rewinds {}, ticks replayed {}, not rewound {}; {} of {} bodies differ from on time (control: {}); on time rewinds {}",
        if alice == Alice::Apart { "Alice in the hub" } else { "Alice beside Bob" },
        late.stats.rewinds,
        late.stats.replayed,
        late.stats.not_rewound,
        wrong.len(),
        on_time.ending.len(),
        control_wrong.len(),
        on_time.stats.rewinds
    );
    for line in wrong.iter().take(4) {
        println!("[room rollback host]   {line}");
    }
    assert_eq!(on_time.stats.rewinds, 0, "premise: inputs on time rewind nothing");
    assert!(late.stats.rewinds > 0, "premise: the late inputs were rewound for");
    (on_time, late, wrong, control_wrong)
}

/// A host that rewinds a guest's room ends where the host that had the
/// guest's inputs on time ends, while the host's player plays in another
/// room; without it the late inputs move Bob elsewhere.
#[test]
fn a_guests_late_input_rewinds_only_its_room_on_the_host() {
    let (on_time, late, wrong, control_wrong) = scene(Alice::Apart);
    assert!(on_time.apart && late.apart, "premise: Alice and Bob are in two rooms");
    assert!(wrong.is_empty(), "the rewinding host does not end where the on-time host ends: {wrong:?}");
    assert!(!control_wrong.is_empty(), "the control: late inputs applied on arrival ended where on-time ones do");
}

/// The same with the host's player in the guest's room, fighting: the replay
/// gives her seat what her devices gave each tick, and the room ends where
/// the on-time run ends.
#[test]
fn a_guests_late_input_rewinds_a_room_the_host_shares() {
    let (on_time, _, wrong, control_wrong) = scene(Alice::Beside);
    assert!(!on_time.apart, "premise: Alice and Bob share a room");
    assert!(wrong.is_empty(), "the rewinding host does not end where the on-time host ends: {wrong:?}");
    assert!(!control_wrong.is_empty(), "the control: late inputs applied on arrival ended where on-time ones do");
}


/// Two guest rooms receive their changed past input in the same host update.
#[test]
fn two_guest_rooms_keep_their_histories_when_both_rewind() {
    let run = |late: bool| {
        let (mut host, apart) = host_with_rooms(Alice::Apart);
        assert!(apart);
        host.world_mut().insert_resource(RewindsGuestRooms);
        let clients = [0, 1].map(|seat| host.world_mut().spawn(GuestSeat(PlayerSlot(seat))).id());
        let tick = host.world().resource::<ambition_platformer2d::time::SimTick>().0 + 2;
        for frame in 1..=12 {
            if frame == if late { 6 } else { 2 } {
                for (seat, client) in clients.iter().enumerate() {
                    host.world_mut().write_message(FromClient {
                        client_id: ClientId::Client(*client),
                        message: GuestInput { sequence: tick, tick, frame: ControlFrame {
                            axis_x: if seat == 0 { 1.0 } else { -1.0 },
                            ..Default::default()
                        } },
                    });
                }
            }
            host.app_mut().update();
        }
        (bodies(&mut host, None), *host.world().resource::<RoomRollbackStats>())
    };
    let (on_time, normal_stats) = run(false);
    let (late, stats) = run(true);
    assert_eq!(normal_stats.rewinds, 0);
    assert_eq!(stats.rewinds, 2, "each guest room has its own saved tick");
    assert_eq!(stats.not_rewound, 0);
    assert!(differing(&on_time, &late).is_empty(), "both rooms must reach the on-time result");
}

/// A pending heal in the other room must survive a guest room's replay.
#[test]
fn a_room_rewind_preserves_another_rooms_pending_heal() {
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::damage::healing::BodyHealRequested;
    use ambition_platformer2d::combat::components::FeatureId;
    use ambition_platformer2d::characters::actor::Health;
    let (mut host, apart) = host_with_rooms(Alice::Apart);
    assert!(apart);
    host.world_mut().insert_resource(RewindsGuestRooms);
    let client = host.world_mut().spawn(GuestSeat(PlayerSlot(0))).id();
    for _ in 0..8 {
        host.app_mut().update();
    }
    let bob = {
        let world = host.world_mut();
        world.query::<(bevy::prelude::Entity, &FeatureId)>().iter(world)
            .find(|(_, id)| id.0 == "ow1_bob").unwrap().0
    };
    host.world_mut().entity_mut(bob).insert(BodyHealth::new(Health { current: 1, max: 10, invulnerable: Default::default() }));
    host.world_mut().write_message(BodyHealRequested::new(3, bob));
    let tick = host.world().resource::<ambition_platformer2d::time::SimTick>().0 - 3;
    host.world_mut().write_message(FromClient {
        client_id: ClientId::Client(client),
        message: GuestInput { sequence: tick, tick, frame: ControlFrame { axis_x: 1.0, ..Default::default() } },
    });
    host.app_mut().update();
    assert_eq!(host.world().resource::<RoomRollbackStats>().rewinds, 1);
    assert_eq!(host.world().get::<BodyHealth>(bob).unwrap().current(), 4);
    host.world_mut().write_message(BodyHealRequested::new(3, bob));
    let tick = host.world().resource::<ambition_platformer2d::time::SimTick>().0 - 2;
    host.world_mut().write_message(FromClient {
        client_id: ClientId::Client(client),
        message: GuestInput { sequence: tick, tick, frame: ControlFrame { axis_x: -1.0, ..Default::default() } },
    });
    host.app_mut().update();
    assert_eq!(host.world().resource::<RoomRollbackStats>().rewinds, 2);
    assert_eq!(host.world().get::<BodyHealth>(bob).unwrap().current(), 7,
        "restoring a stream must also preserve its reader's cursor");
}

/// The other room's narrative takes one normal step, with no replay steps.
#[test]
fn a_guest_room_replay_does_not_advance_or_close_another_rooms_narrative() {
    use ambition_platformer2d::conversation::{ActiveConversation, LiveConversation};
    use ambition_platformer2d::cutscene::{ActiveCutscene, CutsceneBeat, CutsceneRuntime, CutsceneScript};
    use ambition_platformer2d::combat::components::FeatureId;
    use ambition_platformer2d::engine_core::{AabbExt as _, BodyKinematics};
    use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
    let run = |late: bool, queued: bool| {
        let (mut host, apart) = host_with_rooms(Alice::Beside);
        assert!(!apart);
        let bob = {
            let world = host.world_mut();
            world.query::<(bevy::prelude::Entity, &FeatureId)>().iter(world)
                .find(|(_, id)| id.0 == "ow1_bob").unwrap().0
        };
        let room = host.world().get::<InRoomInstance>(bob).unwrap().0;
        let pos = host.world().get::<BodyKinematics>(bob).unwrap().aabb().center();
        host.spawn_enemy_character_at("review_initiator", "Initiator", (pos.x, pos.y), (12.0, 16.0),
            ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive, "sandbag_infinite");
        host.spawn_enemy_character_at("review_talker", "Talker", (pos.x + 8.0, pos.y), (12.0, 16.0),
            ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive, "sandbag_infinite");
        for _ in 0..4 { host.app_mut().update(); }
        let bob = {
            let world = host.world_mut();
            world.query::<(bevy::prelude::Entity, &FeatureId)>().iter(world)
                .find(|(_, id)| id.0 == "review_initiator").unwrap().0
        };
        let talker = {
            let world = host.world_mut();
            world.query::<(bevy::prelude::Entity, &FeatureId)>().iter(world)
                .find(|(_, id)| id.0 == "review_talker").unwrap().0
        };
        alice_on_the_hub_door(&mut host);
        for frame in 1..=60 {
            host.drive_seat(0, alice_leaves(frame));
            host.drive_seat(1, ControlFrame::default());
            host.app_mut().update();
        }
        let (alice_room, bob_room) = where_they_are(&mut host);
        assert_ne!(alice_room, bob_room.flatten());
        host.world_mut().insert_resource(RewindsGuestRooms);
        let client = host.world_mut().spawn(GuestSeat(PlayerSlot(0))).id();
        for _ in 0..8 { host.app_mut().update(); }
        for body in [bob, talker] {
            assert!(host.world().get::<ambition_platformer2d::engine_core::CenteredAabb>(body).is_some(), "a conversation member needs its footprint");
        }
        host.world_mut().resource_mut::<ActiveConversation>().open(LiveConversation::for_test(
            Some(bob), Some(talker), "review_room_b", ambition_platformer2d::input::InputOwner::AllParticipants,
        ));
        let conversation = host.world().resource::<ActiveConversation>().clone();
        let script = CutsceneScript::new("review_room_b", vec![
            CutsceneBeat::Wait { seconds: 0.05 },
            CutsceneBeat::SetFlag { id: "review_room_b_advanced".into(), on: true },
            CutsceneBeat::Wait { seconds: 10.0 },
        ]);
        if queued {
            host.world_mut().resource_mut::<ambition_platformer2d::cutscene::CutsceneLibrary>().insert(script);
            host.world_mut().resource_mut::<ambition_platformer2d::cutscene::CutsceneTriggerQueue>()
                .request_in("review_room_b", room.ordinal());
        } else {
            let mut active = host.world_mut().resource_mut::<ActiveCutscene>();
            active.room = Some(room.ordinal());
            active.runtime = Some(CutsceneRuntime::new(script));
        }
        if late {
            let tick = host.world().resource::<ambition_platformer2d::time::SimTick>().0 - 3;
            host.world_mut().write_message(FromClient {
                client_id: ClientId::Client(client),
                message: GuestInput { sequence: tick, tick, frame: ControlFrame { axis_x: 1.0, ..Default::default() } },
            });
        }
        host.app_mut().update();
        assert_eq!(*host.world().resource::<ActiveConversation>(), conversation,
            "the other room's conversation must keep its members and authority (late={late})");
        let scene = host.world().resource::<ActiveCutscene>().runtime.as_ref().unwrap();
        let flag = host.world().resource::<ambition_platformer2d::session::AmbitionGameSave>().data().flag("review_room_b_advanced");
        ((scene.beat_index, scene.elapsed, flag), host.world().resource::<RoomRollbackStats>().rewinds)
    };
    for queued in [false, true] {
        let (normal, no_rewinds) = run(false, queued);
        let (replayed, rewinds) = run(true, queued);
        assert_eq!(no_rewinds, 0);
        assert_eq!(rewinds, 1);
        assert!(normal.1 > 0.0 && !normal.2, "the scene advances one normal tick");
        assert_eq!(replayed, normal, "a foreign room's replay must not advance dialogue or story effects (queued={queued})");
    }
}

/// A new connection starts its own input history on a reused seat.
#[test]
fn a_reconnected_guest_joins_without_the_previous_owners_held_input() {
    use ambition_platformer2d::characters::control::SlotControls;
    use ambition_platformer2d::session::participants::SessionParticipants;
    use ambition_platformer2d::platformer::lifecycle::session_world_component;
    let mut host = boot_host("central_hub_complex");
    crate::a_second_seat_joins_the_session::join(&mut host, 240).expect("seat 1 joins");
    host.drive_seat(1, ControlFrame::default());
    for _ in 0..3 { host.app_mut().update(); }
    host.world_mut().insert_resource(RewindsGuestRooms);
    let old = host.world_mut().spawn(GuestSeat(PlayerSlot(1))).id();
    host.app_mut().update();
    let send = |host: &mut Platformer2dSimHarness, client, sequence, frame| {
        let tick = host.world().resource::<ambition_platformer2d::time::SimTick>().0 + 1;
        host.world_mut().write_message(FromClient {
            client_id: ClientId::Client(client), message: GuestInput { sequence, tick, frame },
        });
    };
    send(&mut host, old, 200, ControlFrame { axis_x: 1.0, ..Default::default() });
    host.app_mut().update();
    assert_eq!(host.world().resource::<SlotControls>().get(PlayerSlot(1)).axis_x, 1.0);
    host.world_mut().despawn(old);
    for _ in 0..5 { host.app_mut().update(); }
    assert!(!session_world_component::<SessionParticipants>(host.world()).unwrap().contains(PlayerSlot(1)));
    assert_eq!(host.world().resource::<SlotControls>().get(PlayerSlot(1)).axis_x, 0.0);
    let new = host.world_mut().spawn(GuestSeat(PlayerSlot(1))).id();
    assert_ne!(old, new);
    host.app_mut().update();
    send(&mut host, new, 1, ControlFrame { start_pressed: true, ..Default::default() });
    for _ in 0..30 { host.app_mut().update(); }
    assert!(session_world_component::<SessionParticipants>(host.world()).unwrap().contains(PlayerSlot(1)),
        "the new owner's sequence and join press must be accepted");
    assert_eq!(host.world().resource::<SlotControls>().get(PlayerSlot(1)).axis_x, 0.0);
}
