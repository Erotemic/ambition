//! **A hosted guest shows the host's bodies by `SimId`, through
//! `bevy_replicon`** (NET-AUTHORITY-EXPERIMENT's prototype,
//! `engine/hosted-world-networking-experiment.md`).
//!
//! Both Apps build `switch_lab` and Bob beside Alice themselves; the guest
//! simulates nothing. The host drives Alice, and after each of its updates the
//! link carries what replication sent. The guest's bodies must then hold the
//! host's rows exactly. The control is the same guest never linked: Alice
//! moves on the host and not there.

use ambition_app::Platformer2dSimHarness;
use ambition_hosted_world::bevy_replicon::prelude::*;
use ambition_hosted_world::bevy_replicon::test_app::ServerTestAppExt as _;
use ambition_hosted_world::{HostedRole, HostedWorldPlugin};
use bevy::prelude::{PluginGroup as _, PostUpdate};

use crate::a_guest_app_applies_the_host::simulate_nothing;
use crate::common::fixed_60hz_room_options;
use crate::hosted_world::{alice_fights, bodies};
use crate::two_players_two_live_rooms::{bob_beside_alice, ROOM};

const FRAMES: i32 = 90;

fn booted(role: HostedRole) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::build(fixed_60hz_room_options(ROOM), |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        app.add_plugins((RepliconPlugins.set(ServerPlugin::new(PostUpdate)), HostedWorldPlugin { role }));
        // Replicon makes its protocol and channels in `Plugin::finish`, which
        // the harness's `update` loop never calls.
        app.finish();
        app.cleanup();
        Ok(())
    })
    .expect("the room boots");
    bob_beside_alice(&mut sim, ROOM, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    sim
}

#[test]
fn a_hosted_guest_shows_the_hosts_bodies() {
    let mut host = booted(HostedRole::Host);
    let mut guest = booted(HostedRole::Guest);
    let mut alone = booted(HostedRole::Guest);
    simulate_nothing(&mut guest);
    simulate_nothing(&mut alone);
    host.app_mut().connect_client(guest.app_mut());

    let mut bytes = 0usize;
    for frame in 1..=FRAMES {
        host.drive_seat(0, alice_fights(frame));
        host.app_mut().update();
        bytes += host.world().resource::<ServerMessages>().iter_sent().map(|(_, _, sent)| sent.len()).sum::<usize>();
        host.app_mut().exchange_with_client(guest.app_mut());
        guest.app_mut().update();
        alone.app_mut().update();
    }

    let on_host = bodies(&mut host, None);
    let on_guest = bodies(&mut guest, None);
    let on_alone = bodies(&mut alone, None);
    let differ: Vec<_> = on_host.iter().filter(|(id, row)| on_guest.get(*id) != Some(*row)).collect();
    println!(
        "[hosted] {} bodies; {:.1} link bytes a frame; differing on the guest: {}; on the unlinked guest: {}",
        on_host.len(),
        bytes as f64 / f64::from(FRAMES),
        differ.len(),
        on_host.iter().filter(|(id, row)| on_alone.get(*id) != Some(*row)).count()
    );
    assert!(
        differ.is_empty(),
        "the guest does not hold the host's bodies after the last snapshot: {differ:?}"
    );
    assert!(
        on_host.iter().any(|(id, row)| on_alone.get(id) != Some(row)),
        "the control: Alice moved on the host and the unlinked guest followed, so the \
         run measures nothing about the link"
    );
    assert_eq!(
        on_guest.keys().collect::<Vec<_>>(),
        on_host.keys().collect::<Vec<_>>(),
        "replication made a body of its own instead of continuing into the guest's (Signature)"
    );
}
