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
use crate::hosted_world::{alice_leaves, alice_on_the_hub_door, bobs, bodies, LATENCY};
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

fn run(arrival: Arrival, rewinds: bool) -> Run {
    let mut host = Platformer2dSimHarness::build(fixed_60hz_room_options(ROOM), |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        app.add_plugins((RepliconPlugins.set(ServerPlugin::new(PostUpdate)), HostedWorldPlugin { role: HostedRole::Host }));
        app.finish();
        app.cleanup();
        Ok(())
    })
    .expect("the room boots");
    bob_beside_alice(&mut host, ROOM, Some(PlayerSlot(1)));
    alice_on_the_hub_door(&mut host);
    for frame in 1..=60 {
        host.drive_seat(0, alice_leaves(frame));
        host.drive_seat(1, ControlFrame::default());
        host.app_mut().update();
    }
    let (hub, lab) = where_they_are(&mut host);
    let apart = hub.is_some() && lab.flatten().is_some() && hub != lab.flatten();
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
        in_flight.push_back((frame, GuestInput { tick, frame: bobs(frame) }));
        let due = match arrival {
            Arrival::OnTime => frame,
            Arrival::Late => frame - LATENCY,
        };
        while in_flight.front().is_some_and(|(sent, _)| *sent <= due) {
            let (_, input) = in_flight.pop_front().expect("checked");
            deliver(&mut host, input);
        }
        host.drive_seat(0, ControlFrame::default());
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

/// A host that rewinds a guest's room ends where the host that had the
/// guest's inputs on time ends; without it the late inputs move Bob
/// elsewhere.
#[test]
fn a_guests_late_input_rewinds_only_its_room_on_the_host() {
    let on_time = run(Arrival::OnTime, true);
    let late = run(Arrival::Late, true);
    let control = run(Arrival::Late, false);
    let wrong = differing(&on_time.ending, &late.ending);
    let control_wrong = differing(&on_time.ending, &control.ending);
    println!(
        "[room rollback host] rewinds {}, ticks replayed {}, not rewound {}; {} of {} bodies differ from on time (control: {}); on time rewinds {}",
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
    assert!(on_time.apart && late.apart, "premise: Alice and Bob are in two rooms");
    assert_eq!(on_time.stats.rewinds, 0, "premise: inputs on time rewind nothing");
    assert!(late.stats.rewinds > 0, "premise: the late inputs were rewound for");
    assert!(wrong.is_empty(), "the rewinding host does not end where the on-time host ends: {wrong:?}");
    assert!(!control_wrong.is_empty(), "the control: late inputs applied on arrival ended where on-time ones do");
}
