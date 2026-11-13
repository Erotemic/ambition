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

/// One run of the crossing scene through the link: Alice leaves through the
/// hub door on frame 30 and Bob stays. With `scoped`, the host names the
/// guest's seat (Bob's), so replicon's room filter applies. Returns the host's
/// bodies in Bob's room, the guest's bodies, the entities replication made or
/// continued on the guest, and the link bytes a frame.
fn crossing_through_the_link(scoped: bool) -> (Vec<ambition_platformer2d::platformer::sim_id::SimId>, Vec<ambition_platformer2d::platformer::sim_id::SimId>, usize, f64, bool) {
    use crate::hosted_world::{alice_leaves, alice_on_the_hub_door, bobs};
    use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
    const RUN: i32 = 220;
    let mut host = booted(HostedRole::Host);
    alice_on_the_hub_door(&mut host);
    let mut guest = booted(HostedRole::Guest);
    simulate_nothing(&mut guest);
    host.app_mut().connect_client(guest.app_mut());
    if scoped {
        let world = host.world_mut();
        let client = world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ConnectedClient>>()
            .single(world)
            .expect("one guest is connected");
        world
            .entity_mut(client)
            .insert(ambition_hosted_world::GuestSeat(ambition_platformer2d::characters::control::PlayerSlot(1)));
    }
    let mut bytes = 0usize;
    for frame in 1..=RUN {
        host.drive_seat(0, alice_leaves(frame));
        host.drive_seat(1, bobs(frame));
        host.app_mut().update();
        bytes += host.world().resource::<ServerMessages>().iter_sent().map(|(_, _, sent)| sent.len()).sum::<usize>();
        host.app_mut().exchange_with_client(guest.app_mut());
        guest.app_mut().update();
    }
    let bobs_room = {
        let world = host.world_mut();
        world
            .query::<(&ambition_platformer2d::characters::control::DrivingParticipant, &InRoomInstance)>()
            .iter(world)
            .find(|(driver, _)| driver.0 .0 == 1)
            .map(|(_, room)| room.0)
            .expect("Bob is in a room")
    };
    let alice_left = {
        let world = host.world_mut();
        world
            .query::<(&ambition_platformer2d::characters::control::DrivingParticipant, &InRoomInstance)>()
            .iter(world)
            .any(|(driver, room)| driver.0 .0 == 0 && room.0 != bobs_room)
    };
    let in_bobs_room = bodies(&mut host, Some(bobs_room));
    let on_guest = bodies(&mut guest, None);
    // Agreement on the rows, not only the ids.
    let rows_agree = in_bobs_room.iter().all(|(id, row)| on_guest.get(id) == Some(row));
    let remote = {
        let world = guest.world_mut();
        world.query_filtered::<(), bevy::prelude::With<Remote>>().iter(world).count()
    };
    assert!(alice_left, "premise: Alice crossed into the hub and Bob stayed");
    (
        in_bobs_room.into_keys().collect(),
        on_guest.into_keys().collect(),
        remote,
        bytes as f64 / f64::from(RUN),
        rows_agree,
    )
}

/// NA-2b's interest through `bevy_replicon`: with the guest's seat named, the
/// host sends it the bodies of Bob's room only. After Alice leaves for the
/// hub, the guest holds exactly the host's bodies in Bob's room, with their
/// rows, and none of the hub's. The control is the same run with no seat
/// named: the guest is sent the hub too.
#[test]
fn a_hosted_guest_is_sent_its_own_room() {
    let (in_room, on_guest, remote, scoped_bytes, agree) = crossing_through_the_link(true);
    let (_, on_unscoped, remote_unscoped, all_bytes, _) = crossing_through_the_link(false);
    println!(
        "[hosted room] Bob's room on the host {} bodies; the scoped guest {} bodies, {remote} replicated, {scoped_bytes:.1} link bytes a frame; unscoped {} bodies, {remote_unscoped} replicated, {all_bytes:.1} bytes a frame",
        in_room.len(),
        on_guest.len(),
        on_unscoped.len()
    );
    assert_eq!(on_guest, in_room, "the scoped guest does not hold exactly the bodies of Bob's room");
    assert!(agree, "the scoped guest holds Bob's room's bodies with other rows than the host's");
    assert!(
        remote_unscoped > remote,
        "the control: with no seat named the guest was sent no more than the scoped one, so the filter is not what scoped it"
    );
}
