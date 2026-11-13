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

type Health = ambition_platformer2d::characters::actor::BodyHealth;
type Beat = ambition_platformer2d::combat::components::DeathBeat;

/// Bob's life on `sim`, as the life row carries it, and whether his beat
/// still draws him.
fn bobs_life(sim: &mut Platformer2dSimHarness) -> (ambition_hosted_world::LifeRow, bool) {
    let world = sim.world_mut();
    world
        .query::<(&ambition_platformer2d::characters::control::DrivingParticipant, &Health, Option<&Beat>)>()
        .iter(world)
        .find(|(driver, ..)| driver.0 .0 == 1)
        .map(|(_, health, beat)| {
            (ambition_hosted_world::LifeRow::of(health, beat), beat.is_none_or(|beat| beat.visible(health.alive())))
        })
        .expect("Bob has a body")
}

/// NA-2b through `bevy_replicon`: the host kills Bob, and the guest, which
/// simulates no death, holds his health and his death's beat (the seconds it
/// has run, what he leaves, whether he is still drawn) on every frame. The
/// beat runs only on the host, so a guest that agrees on it got it from the
/// row. The control is the unlinked guest, where Bob never falls.
#[test]
fn a_hosted_guest_shows_a_death_on_the_host() {
    const KILLED_AT: i32 = 20;
    let mut host = booted(HostedRole::Host);
    let mut guest = booted(HostedRole::Guest);
    let mut alone = booted(HostedRole::Guest);
    simulate_nothing(&mut guest);
    simulate_nothing(&mut alone);
    host.app_mut().connect_client(guest.app_mut());

    let (mut differs, mut host_dead, mut host_hidden, mut guest_hidden, mut alone_dead) = (0, None, None, None, false);
    let mut beat_ran = None;
    for frame in 1..=FRAMES {
        if frame == KILLED_AT {
            let world = host.world_mut();
            let mut bob = world
                .query::<(&ambition_platformer2d::characters::control::DrivingParticipant, &mut Health)>()
                .iter_mut(world)
                .find(|(driver, _)| driver.0 .0 == 1)
                .expect("Bob has a body");
            bob.1.health.current = 0;
        }
        host.app_mut().update();
        host.app_mut().exchange_with_client(guest.app_mut());
        guest.app_mut().update();
        alone.app_mut().update();
        let (on_host, drawn_on_host) = bobs_life(&mut host);
        let (on_guest, drawn_on_guest) = bobs_life(&mut guest);
        if on_host.current <= 0 {
            host_dead.get_or_insert(frame);
        }
        if on_host.since_s.is_some_and(|since| since > 0.0) {
            beat_ran = Some(on_host.since_s);
        }
        if !drawn_on_host {
            host_hidden.get_or_insert(frame);
        }
        if !drawn_on_guest {
            guest_hidden.get_or_insert(frame);
        }
        if (on_host, drawn_on_host) != (on_guest, drawn_on_guest) {
            differs += 1;
        }
        alone_dead |= bobs_life(&mut alone).0.current <= 0;
    }
    println!(
        "[hosted death] Bob dead on the host from frame {host_dead:?}, his beat at {beat_ran:?} s at the end; undrawn on the host from {host_hidden:?}, on the guest from {guest_hidden:?}; frames the guest's life differs: {differs}"
    );
    assert!(host_dead.is_some(), "premise: Bob died on the host");
    assert!(beat_ran.is_some(), "premise: Bob's death beat ran on the host");
    assert_eq!(differs, 0, "the guest's Bob did not hold the host's life on every frame");
    assert_eq!(guest_hidden, host_hidden, "the guest stopped drawing Bob on another frame than the host");
    assert!(!alone_dead, "the control: Bob fell on the unlinked guest too, so the run measures nothing about the link");
}
