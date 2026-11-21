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
use ambition_platformer2d::engine_core::{ConfirmedFrameBoundary, ControlFrame};
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
    /// The most ticks the published boundary trailed the live tick, and the
    /// rewinds that replayed from a tick it had already confirmed.
    widest_lag: i32,
    rewound_confirmed: u32,
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
    let rewinds_on = rewinds;
    let (mut confirmed, mut widest_lag, mut rewound_confirmed) = (i32::MIN, 0, 0);
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
        let rewinds = host.world().get_resource::<RoomRollbackStats>().map_or(0, |stats| stats.rewinds);
        host.app_mut().update();
        let stats = host.world().get_resource::<RoomRollbackStats>().copied().unwrap_or_default();
        if stats.rewinds > rewinds && stats.last_rewound_from.is_some_and(|from| from as i32 <= confirmed) {
            rewound_confirmed += 1;
        }
        if rewinds_on {
            let boundary = *host.world().get_resource::<ConfirmedFrameBoundary>().expect("a host that rewinds publishes a boundary");
            widest_lag = widest_lag.max(boundary.current - boundary.confirmed);
            confirmed = confirmed.max(boundary.confirmed);
        }
    }
    // The last inputs arrive after the last frame; one more frame takes them
    // in, on time and late alike.
    for (_, input) in in_flight.drain(..) {
        deliver(&mut host, input);
    }
    host.drive_seat(0, ControlFrame::default());
    host.app_mut().update();
    let stats = host.world().get_resource::<RoomRollbackStats>().copied().unwrap_or_default();
    Run { ending: bodies(&mut host, None), stats, apart, widest_lag, rewound_confirmed }
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
    println!(
        "[room rollback host]   the boundary trailed the live tick by at most {} on time and {} late; rewinds from a confirmed tick: {} and {}",
        on_time.widest_lag, late.widest_lag, on_time.rewound_confirmed, late.rewound_confirmed
    );
    for line in wrong.iter().take(4) {
        println!("[room rollback host]   {line}");
    }
    assert_eq!(on_time.stats.rewinds, 0, "premise: inputs on time rewind nothing");
    assert!(late.stats.rewinds > 0, "premise: the late inputs were rewound for");
    // The live tick waits for the input after it; late inputs hold the line
    // back by their lateness, and no rewind reopens a tick it confirmed.
    assert_eq!(on_time.widest_lag, 1, "inputs stamped for the next tick confirm every tick before it");
    assert!(late.widest_lag > 1, "premise: late inputs hold the confirmed line back");
    assert_eq!(late.rewound_confirmed, 0, "a rewind replayed a tick the host had published as confirmed");
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
            host.world_mut().insert_resource(ActiveCutscene::playing(CutsceneRuntime::new(script), Some(room.ordinal())));
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
        let scene = &host.world().resource::<ActiveCutscene>().iter().next().unwrap().runtime;
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

/// A UDP relay on loopback that holds each datagram `delay` before passing
/// it on, both ways: a guest joins the relay's address and the host answers
/// through it, so the link has a round trip of twice `delay`.
struct Relay {
    address: std::net::SocketAddr,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Relay {
    fn to(host: std::net::SocketAddr, delay: std::time::Duration) -> Relay {
        use std::net::UdpSocket;
        use std::sync::atomic::Ordering;
        use std::time::Instant;
        let front = UdpSocket::bind("127.0.0.1:0").expect("the relay's guest side binds");
        let back = UdpSocket::bind("127.0.0.1:0").expect("the relay's host side binds");
        front.set_nonblocking(true).expect("nonblocking");
        back.set_nonblocking(true).expect("nonblocking");
        let address = front.local_addr().expect("an address");
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stopping = stop.clone();
        let thread = std::thread::spawn(move || {
            let mut guest = None;
            let mut up: VecDeque<(Instant, Vec<u8>)> = VecDeque::new();
            let mut down: VecDeque<(Instant, Vec<u8>)> = VecDeque::new();
            let mut buffer = [0u8; 2048];
            while !stopping.load(Ordering::Relaxed) {
                while let Ok((size, from)) = front.recv_from(&mut buffer) {
                    guest = Some(from);
                    up.push_back((Instant::now() + delay, buffer[..size].to_vec()));
                }
                while let Ok((size, _)) = back.recv_from(&mut buffer) {
                    down.push_back((Instant::now() + delay, buffer[..size].to_vec()));
                }
                let now = Instant::now();
                while up.front().is_some_and(|(due, _)| *due <= now) {
                    let (_, packet) = up.pop_front().expect("checked");
                    let _ = back.send_to(&packet, host);
                }
                while let Some(guest) = guest.filter(|_| down.front().is_some_and(|(due, _)| *due <= now)) {
                    let (_, packet) = down.pop_front().expect("checked");
                    let _ = front.send_to(&packet, guest);
                }
                std::thread::sleep(std::time::Duration::from_micros(100));
            }
        });
        Relay { address, stop, thread: Some(thread) }
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The host and guest Apps of a run over a real link.
fn boot_over_udp(room: &str, role: HostedRole) -> Platformer2dSimHarness {
    Platformer2dSimHarness::build(fixed_60hz_room_options(room), |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        app.add_plugins((
            RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
            ambition_hosted_world::udp::RepliconRenetPlugins,
            HostedWorldPlugin { role },
        ));
        app.finish();
        app.cleanup();
        Ok(())
    })
    .expect("the room boots")
}

/// What one paced run over a real link measured.
struct LinkRun {
    stats: RoomRollbackStats,
    /// Host update times of frames with no rewind and with one, in ms.
    plain: Vec<f64>,
    rewound: Vec<f64>,
    bob_moved: bool,
    /// The guest's run-ahead at the end, in ticks.
    lead: Option<u64>,
    /// Frames after which the host's boundary was fully confirmed.
    confirmed_frames: u32,
}

/// The guest drives Bob over UDP through a relay of `delay` each way, both
/// Apps paced at 60 Hz of wall time; the host predicts Bob and rewinds his
/// room when his stamped input arrives and differs.
fn over_a_real_link(alice: Alice, delay: std::time::Duration) -> LinkRun {
    use ambition_hosted_world::bevy_replicon::prelude::{ClientState, ConnectedClient};
    use ambition_hosted_world::udp;
    use std::time::{Duration, Instant};
    let bob = PlayerSlot(1);
    let mut host = boot_over_udp(ROOM, HostedRole::Host);
    bob_beside_alice(&mut host, ROOM, Some(bob));
    if alice == Alice::Apart {
        alice_on_the_hub_door(&mut host);
        for frame in 1..=60 {
            host.drive_seat(0, alice_leaves(frame));
            host.drive_seat(1, ControlFrame::default());
            host.app_mut().update();
        }
    }
    host.world_mut().insert_resource(RewindsGuestRooms);
    let mut guest = boot_over_udp(ROOM, HostedRole::Guest);
    bob_beside_alice(&mut guest, ROOM, Some(bob));
    crate::a_guest_app_applies_the_host::simulate_nothing(&mut guest);
    guest.world_mut().insert_resource(ambition_hosted_world::LocalSeat(bob));

    let socket = std::net::UdpSocket::bind("127.0.0.1:0").expect("the host binds");
    let bound = socket.local_addr().expect("an address");
    let relay = Relay::to(bound, delay);
    udp::host_on(host.world_mut(), socket, vec![relay.address], 1).expect("the host opens its socket");
    udp::join(guest.world_mut(), relay.address, 1).expect("the guest opens its socket");
    let step = Duration::from_secs_f64(1.0 / 60.0);
    let mut connected = false;
    for _ in 0..600 {
        let started = Instant::now();
        host.drive_seat(0, ControlFrame::default());
        host.app_mut().update();
        guest.app_mut().update();
        if *guest.world().resource::<bevy::state::state::State<ClientState>>() == ClientState::Connected {
            connected = true;
            break;
        }
        std::thread::sleep(step.saturating_sub(started.elapsed()));
    }
    assert!(connected, "the guest joined the host through the relay within 600 frames");
    let client = {
        let world = host.world_mut();
        world.query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ConnectedClient>>().single(world).expect("one guest")
    };
    host.world_mut().entity_mut(client).insert(GuestSeat(bob));
    let bob_at = |host: &mut Platformer2dSimHarness| {
        let world = host.world_mut();
        world
            .query::<(&ambition_platformer2d::characters::control::DrivingParticipant, &ambition_platformer2d::engine_core::BodyKinematics)>()
            .iter(world)
            .find(|(driver, _)| driver.0 == bob)
            .map(|(_, body)| body.pos)
    };
    let start = bob_at(&mut host);
    let mut plain = Vec::new();
    let mut rewound = Vec::new();
    let mut confirmed_frames = 0;
    for frame in 1..=600 {
        let started = Instant::now();
        guest.drive_seat(1, bobs(frame));
        guest.app_mut().update();
        host.drive_seat(0, if alice == Alice::Beside { alice_fights(frame) } else { ControlFrame::default() });
        let before = host.world().get_resource::<RoomRollbackStats>().copied().unwrap_or_default();
        let update = Instant::now();
        host.app_mut().update();
        let ms = update.elapsed().as_secs_f64() * 1e3;
        let after = host.world().get_resource::<RoomRollbackStats>().copied().unwrap_or_default();
        if after.rewinds > before.rewinds { rewound.push(ms) } else { plain.push(ms) }
        if host.world().get_resource::<ConfirmedFrameBoundary>().is_some_and(ConfirmedFrameBoundary::fully_confirmed) {
            confirmed_frames += 1;
        }
        std::thread::sleep(step.saturating_sub(started.elapsed()));
    }
    let stats = host.world().get_resource::<RoomRollbackStats>().copied().unwrap_or_default();
    let bob_moved = bob_at(&mut host) != start;
    let lead = ambition_hosted_world::guest_run_ahead(guest.world());
    drop(relay);
    LinkRun { stats, plain, rewound, bob_moved, lead, confirmed_frames }
}

fn spread(times: &[f64]) -> String {
    if times.is_empty() {
        return "none".into();
    }
    let mut sorted = times.to_vec();
    sorted.sort_by(f64::total_cmp);
    let at = |q: f64| sorted[((sorted.len() - 1) as f64 * q).round() as usize];
    let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
    format!("{} frames, mean {mean:.2}, p50 {:.2}, p95 {:.2}, max {:.2} ms", sorted.len(), at(0.5), at(0.95), at(1.0))
}

/// SIMULATION-ISLANDS: what a room rewind costs over a real link. The guest
/// drives Bob over UDP through a relay that delays each datagram, both Apps
/// paced at 60 Hz of wall time, and the host's update is timed on the frames
/// that rewind and on the frames that do not. Run it in release for the
/// cost a player pays; a debug build measures the debug build. The guest
/// runs ahead (`input_clock`): once its lead settles its inputs arrive
/// before the ticks they name, so the rewinds left are the ones before it
/// settled and the link's jitter. The lateness is read from the ticks a
/// rewind replays: renet's own round trip is timed on the App's clock, which
/// the harness steps, not on the wall.
#[test]
#[ignore = "a measurement over wall-clock time: run with --ignored --nocapture, in release for the cost"]
fn a_guests_room_rewinds_over_a_real_link() {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    for alice in [Alice::Apart, Alice::Beside] {
        for one_way in [0, 25, 50] {
            let run = over_a_real_link(alice, std::time::Duration::from_millis(one_way));
            let per_rewind = f64::from(run.stats.replayed) / f64::from(run.stats.rewinds.max(1));
            println!(
                "[room rewind over udp] {profile}, {}, {one_way} ms each way: guest run-ahead {:?} ticks; boundary fully confirmed after {} of 600 frames; rewinds {}, ticks replayed {} ({per_rewind:.1} a rewind), not rewound {}; host update without a rewind: {}; with one: {}",
                if alice == Alice::Apart { "Alice in the hub" } else { "Alice beside Bob" },
                run.lead,
                run.confirmed_frames,
                run.stats.rewinds,
                run.stats.replayed,
                run.stats.not_rewound,
                spread(&run.plain),
                spread(&run.rewound),
            );
            assert!(run.bob_moved, "premise: the guest's input moved Bob on the host");
        }
    }
}
