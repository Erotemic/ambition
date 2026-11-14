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

/// What `sim` holds of the body with feature id `feature`: its `SimId`, the
/// character it wears, whether replication made it, and whether it is a
/// whole actor (health and a death beat, as the staged-actor road builds).
fn the_spawned(sim: &mut Platformer2dSimHarness, feature: &str) -> Option<(ambition_platformer2d::platformer::sim_id::SimId, String, bool, bool)> {
    let world = sim.world_mut();
    world
        .query::<(
            &ambition_platformer2d::combat::components::FeatureId,
            Option<&ambition_platformer2d::platformer::sim_id::SimId>,
            Option<&ambition_platformer2d::characters::actor::CharacterForm>,
            Option<&Remote>,
            Option<&Health>,
            Option<&Beat>,
        )>()
        .iter(world)
        .find(|(id, ..)| id.0 == feature)
        .and_then(|(_, sim_id, worn, remote, health, beat)| {
            Some((sim_id?.clone(), worn?.0.as_str().to_owned(), remote.is_some(), health.is_some() && beat.is_some()))
        })
}

/// NA-2b's spawn row through `bevy_replicon`: the host spawns an enemy beside
/// Alice on frame 5, which the guest never built. The guest builds it from its
/// spawn row through the staged-actor road, into the replicated entity, and
/// then holds every body the host holds with the host's rows, the enemy's
/// included. The control is the unlinked guest, which never has the enemy.
#[test]
fn a_hosted_guest_builds_what_the_host_spawned() {
    const ENEMY: &str = "na_spawn_row";
    const CHARACTER: &str = "npc_puppy_slug";
    let mut host = booted(HostedRole::Host);
    let mut guest = booted(HostedRole::Guest);
    let mut alone = booted(HostedRole::Guest);
    simulate_nothing(&mut guest);
    simulate_nothing(&mut alone);
    host.app_mut().connect_client(guest.app_mut());
    for frame in 1..=FRAMES {
        if frame == 5 {
            let at = {
                let world = host.world_mut();
                world
                    .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
                    .single(world)
                    .expect("Alice has a body")
                    .pos
            };
            host.spawn_enemy_character_at(
                ENEMY,
                "Target",
                (at.x + 96.0, at.y),
                (12.0, 16.0),
                ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
                CHARACTER,
            );
        }
        host.drive_seat(0, alice_fights(frame));
        host.app_mut().update();
        host.app_mut().exchange_with_client(guest.app_mut());
        guest.app_mut().update();
        alone.app_mut().update();
    }
    let on_host = bodies(&mut host, None);
    let on_guest = bodies(&mut guest, None);
    let spawned_on_host = the_spawned(&mut host, ENEMY);
    let spawned_on_guest = the_spawned(&mut guest, ENEMY);
    println!(
        "[hosted spawn] the host's {spawned_on_host:?}; the guest's {spawned_on_guest:?}; bodies host {} guest {}",
        on_host.len(),
        on_guest.len()
    );
    let (id, character, _, _) = spawned_on_host.expect("premise: the host spawned the enemy");
    assert_eq!(character, CHARACTER, "premise: the host's enemy wears its character");
    assert_eq!(
        spawned_on_guest,
        Some((id, CHARACTER.to_owned(), true, true)),
        "the guest did not build the host's enemy, with its SimId and character, as a whole actor in the replicated entity"
    );
    let differ: Vec<_> = on_host.iter().filter(|(id, row)| on_guest.get(*id) != Some(*row)).collect();
    assert!(differ.is_empty(), "the guest does not hold the host's bodies: {differ:?}");
    assert_eq!(on_guest.len(), on_host.len(), "the guest holds bodies the host does not");
    assert!(the_spawned(&mut alone, ENEMY).is_none(), "the control: the unlinked guest has the enemy, so its arrival is not the link's");
}

/// `booted`, linked by renet's netcode over UDP instead of the in-process
/// test link.
fn booted_over_udp(role: HostedRole) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::build(fixed_60hz_room_options(ROOM), |app, options| {
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
    .expect("the room boots");
    bob_beside_alice(&mut sim, ROOM, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    sim
}

/// The first slice's run over a real link: the host opens a UDP socket on
/// loopback, the guest joins it through netcode, and Alice moves and swings
/// for 90 frames. After each frame (host update, then guest update) the
/// guest must hold the host's bodies; the packets cross the kernel's
/// loopback between the two updates.
#[test]
fn a_hosted_guest_shows_the_hosts_bodies_over_udp() {
    use ambition_hosted_world::udp;
    let mut host = booted_over_udp(HostedRole::Host);
    let mut guest = booted_over_udp(HostedRole::Guest);
    simulate_nothing(&mut guest);
    let address = udp::host(host.world_mut(), 0, 1).expect("the host opens a UDP socket on loopback");
    udp::join(guest.world_mut(), address, 1).expect("the guest opens its socket");

    let mut connected_after = None;
    for update in 0..600 {
        host.app_mut().update();
        guest.app_mut().update();
        if *guest.world().resource::<bevy::state::state::State<ClientState>>() == ClientState::Connected {
            connected_after = Some(update);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let connected_after = connected_after.expect("the guest joined the host over UDP within 600 updates");

    let at_start = bodies(&mut host, None);
    let mut agreed = 0;
    for frame in 1..=FRAMES {
        host.drive_seat(0, alice_fights(frame));
        host.app_mut().update();
        guest.app_mut().update();
        if bodies(&mut host, None) == bodies(&mut guest, None) {
            agreed += 1;
        }
    }
    let on_host = bodies(&mut host, None);
    let on_guest = bodies(&mut guest, None);
    // Renet's own account of the link (its clock is the App's `Time`).
    let link = host
        .world()
        .resource::<ambition_hosted_world::udp::RenetServer>()
        .0
        .network_info(1)
        .expect("the guest is connected");
    println!(
        "[hosted udp] joined after {connected_after} updates; {} bodies; the guest agreed after {agreed} of {FRAMES} frames; renet: rtt {:.1} ms, {:.0} bytes/s sent, loss {:.3}",
        on_host.len(),
        link.rtt,
        link.bytes_sent_per_second,
        link.packet_loss
    );
    assert_ne!(on_host, at_start, "premise: the host's bodies moved, so agreeing measures the link");
    assert_eq!(on_guest, on_host, "after the last frame the guest does not hold the host's bodies");
    assert!(agreed * 10 >= FRAMES * 9, "the guest agreed with the host after only {agreed} of {FRAMES} frames");
}

/// How the guest of [`bob_from_the_guest`] comes by its seat.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Seating {
    /// The test names the guest's seat on both Apps.
    Named,
    /// The host seats the guest when it connects (`SeatsGuests`) and tells it.
    Assigned,
    /// Nobody names a seat on the host.
    None,
}

/// One run of Bob driven from the guest: the guest holds `bobs(frame)` on its
/// seat and sends it; the host drives nothing itself. Returns how many frames
/// the host's bodies differ from a reference App where Bob is driven locally
/// with the same frames, whether Bob moved, the seat each App holds for the
/// guest after the run, and the seat whose body the guest's camera follows.
fn bob_from_the_guest(seating: Seating) -> (i32, bool, Option<u8>, Option<u8>, Option<u8>) {
    use crate::hosted_world::bobs;
    let bob = ambition_platformer2d::characters::control::PlayerSlot(1);
    let mut host = booted(HostedRole::Host);
    let mut guest = booted(HostedRole::Guest);
    let mut local = booted(HostedRole::Host);
    simulate_nothing(&mut guest);
    if seating == Seating::Assigned {
        host.world_mut().insert_resource(ambition_hosted_world::SeatsGuests);
        // As a player's App: its own devices are its seat 0.
        guest.world_mut().insert_resource(ambition_hosted_world::InputFromSeat(
            ambition_platformer2d::characters::control::PlayerSlot(0),
        ));
    } else {
        guest.world_mut().insert_resource(ambition_hosted_world::LocalSeat(bob));
    }
    host.app_mut().connect_client(guest.app_mut());
    let client = {
        let world = host.world_mut();
        world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ConnectedClient>>()
            .single(world)
            .expect("one guest is connected")
    };
    if seating == Seating::Named {
        host.world_mut().entity_mut(client).insert(ambition_hosted_world::GuestSeat(bob));
    }
    // `connect_client` updated the host once; the reference takes the same
    // step, so both start the run on the same tick. An assigned seat reaches
    // the guest with one exchange more, which the reference does not match:
    // the loop below exchanges twice a frame beside one update of each.
    local.app_mut().update();
    if seating == Seating::Assigned {
        host.app_mut().exchange_with_client(guest.app_mut());
    }
    let at_start = bodies(&mut local, None);
    let mut differs = 0;
    for frame in 1..=FRAMES {
        guest.drive_seat(if seating == Seating::Assigned { 0 } else { 1 }, bobs(frame));
        guest.app_mut().update();
        // The guest's input reaches the host before its update.
        host.app_mut().exchange_with_client(guest.app_mut());
        host.app_mut().update();
        host.app_mut().exchange_with_client(guest.app_mut());
        local.drive_seat(1, bobs(frame));
        local.app_mut().update();
        if bodies(&mut host, None) != bodies(&mut local, None) {
            differs += 1;
        }
    }
    let moved = bodies(&mut local, None) != at_start;
    let on_host = host.world().get::<ambition_hosted_world::GuestSeat>(client).map(|seat| seat.0 .0);
    let on_guest = guest.world().get_resource::<ambition_hosted_world::LocalSeat>().map(|seat| seat.0 .0);
    let viewed = {
        let world = guest.world_mut();
        let subject = world.resource::<ambition_platformer2d::platformer::markers::ControlledSubject>().0;
        subject.and_then(|body| world.get::<ambition_platformer2d::characters::control::DrivingParticipant>(body)).map(|driver| driver.0 .0)
    };
    (differs, moved, on_host, on_guest, viewed)
}

/// The guest's input reaches the host: Bob, driven only on the guest App, moves
/// on the host on every frame exactly as he does on an App that drives him
/// locally with the same frames. The control is the same run with no seat
/// named for the guest: its input reaches nobody, and the host's Bob stands.
#[test]
fn a_hosted_guest_drives_its_seat_on_the_host() {
    let (differs, moved, ..) = bob_from_the_guest(Seating::Named);
    let (differs_unseated, ..) = bob_from_the_guest(Seating::None);
    println!("[hosted input] frames the host differs from the local reference: seated {differs}, unseated {differs_unseated} of {FRAMES}");
    assert!(moved, "premise: Bob moves when driven");
    assert_eq!(differs, 0, "the host's Bob did not move as the guest drove him");
    assert!(differs_unseated > 0, "the control: with no seat named the host still matched, so the seat is not what delivered the input");
}

/// A host that seats its guests (`SeatsGuests`) gives a connecting guest the
/// first seat after its own and tells it: both Apps hold seat 1 for the guest,
/// and Bob, driven only by the guest's own seat 0 (`InputFromSeat`, as a
/// player's keyboard is), moves on the host as on the local reference. The
/// control is the unseated run of the test above.
#[test]
fn a_connecting_guest_is_given_a_seat() {
    let (differs, moved, on_host, on_guest, viewed) = bob_from_the_guest(Seating::Assigned);
    println!("[hosted seat] host holds {on_host:?}, guest holds {on_guest:?}; frames differing {differs} of {FRAMES}; the guest's camera follows seat {viewed:?}");
    assert!(moved, "premise: Bob moves when driven");
    assert_eq!(on_host, Some(1), "the host did not seat the guest at seat 1");
    assert_eq!(on_guest, Some(1), "the guest was not told its seat");
    assert_eq!(viewed, Some(1), "the guest's camera does not follow its own seat's body");
    assert_eq!(differs, 0, "the host's Bob did not move as the guest drove him");
}

/// Where `sim`'s seat `slot` body stands: its live room and that room's
/// definition.
fn seat_room(sim: &mut Platformer2dSimHarness, slot: u8) -> Option<(ambition_platformer2d::platformer::lifecycle::LiveRoomInstance, String)> {
    use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
    let room = {
        let world = sim.world_mut();
        world
            .query::<(&ambition_platformer2d::characters::control::DrivingParticipant, &InRoomInstance)>()
            .iter(world)
            .find(|(driver, _)| driver.0 .0 == slot)
            .map(|(_, room)| room.0)?
    };
    crate::a_guest_app_applies_the_host::live_room_ids(sim).into_iter().find(|(live, _)| *live == room)
}

/// One run of Bob crossing to the hub, driven from the guest. With
/// `following`, the guest names its seat (`LocalSeat`), so it follows its
/// body's crossing on the host's word and reports the room it holds.
/// Returns (the guest's Bob's room definition, the host's bodies in Bob's
/// room that no seat drives, the guest's bodies, bodies the guest holds
/// twice, whether the guest holds Alice).
#[allow(clippy::type_complexity)]
fn bob_crosses_from_the_guest(
    following: bool,
) -> (
    Option<String>,
    std::collections::BTreeMap<ambition_platformer2d::platformer::sim_id::SimId, [f32; 5]>,
    std::collections::BTreeMap<ambition_platformer2d::platformer::sim_id::SimId, [f32; 5]>,
    usize,
    (Option<[f32; 5]>, Option<[f32; 5]>),
) {
    use ambition_platformer2d::engine_core::{AabbExt as _, ControlFrame};
    use ambition_platformer2d::platformer::sim_id::SimId;
    const RUN: i32 = 300;
    let presses = |at: i32| move |frame: i32| ControlFrame {
        interact_pressed: frame == at,
        interact_held: (at..at + 3).contains(&frame),
        ..Default::default()
    };
    let (alice, bob) = (presses(30), presses(90));
    let on_the_door = |sim: &mut Platformer2dSimHarness| {
        crate::common::doors_cross_at_once(sim);
        let door = crate::common::door_to(sim, "central_hub_complex").aabb.center();
        sim.teleport_player((door.x, door.y));
        crate::two_players_two_live_rooms::put_bob_at(sim, door);
    };
    let bob_seat = ambition_platformer2d::characters::control::PlayerSlot(1);
    let mut host = booted(HostedRole::Host);
    on_the_door(&mut host);
    let mut guest = booted(HostedRole::Guest);
    on_the_door(&mut guest);
    crate::a_guest_app_applies_the_host::simulate_nothing_but_room_changes(&mut guest);
    if following {
        guest.world_mut().insert_resource(ambition_hosted_world::LocalSeat(bob_seat));
    }
    host.app_mut().connect_client(guest.app_mut());
    {
        let world = host.world_mut();
        let client = world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ConnectedClient>>()
            .single(world)
            .expect("one guest is connected");
        world.entity_mut(client).insert(ambition_hosted_world::GuestSeat(bob_seat));
    }
    for frame in 1..=RUN {
        guest.drive_seat(1, bob(frame));
        guest.app_mut().update();
        host.app_mut().exchange_with_client(guest.app_mut());
        host.drive_seat(0, alice(frame));
        if !following {
            // The control has no input road from the guest: Bob crosses on
            // the host by the same press.
            host.drive_seat(1, bob(frame));
        }
        host.app_mut().update();
        host.app_mut().exchange_with_client(guest.app_mut());
    }
    // The guest applies the last snapshot.
    guest.app_mut().update();
    let (hosts_room, hub) = seat_room(&mut host, 1).expect("Bob stands in a room on the host");
    assert_eq!(hub, "central_hub_complex", "premise: Bob crossed to the hub on the host");
    let alice_on_host = bodies(&mut host, Some(hosts_room)).get(&SimId::player_slot(0)).copied();
    let in_hub: std::collections::BTreeMap<SimId, [f32; 5]> = {
        let drivers: std::collections::BTreeSet<SimId> = {
            let world = host.world_mut();
            world
                .query_filtered::<&SimId, bevy::prelude::With<ambition_platformer2d::characters::control::DrivingParticipant>>()
                .iter(world)
                .cloned()
                .collect()
        };
        bodies(&mut host, Some(hosts_room)).into_iter().filter(|(id, _)| !drivers.contains(id)).collect()
    };
    let guests_room = seat_room(&mut guest, 1).map(|(_, definition)| definition);
    let on_guest = bodies(&mut guest, None);
    let twice = {
        let world = guest.world_mut();
        let mut held: std::collections::BTreeMap<SimId, usize> = Default::default();
        for id in world
            .query_filtered::<&SimId, bevy::prelude::With<ambition_platformer2d::engine_core::BodyKinematics>>()
            .iter(world)
        {
            *held.entry(id.clone()).or_insert(0) += 1;
        }
        held.values().filter(|count| **count > 1).count()
    };
    let alice_on_guest = on_guest.get(&SimId::player_slot(0)).copied();
    (guests_room, in_hub, on_guest, twice, (alice_on_host, alice_on_guest))
}

/// The room row through `bevy_replicon`: Bob, driven from the guest, crosses
/// to the hub on the host. The guest follows its own body's crossing on the
/// host's word (its room lifecycle builds the hub), tells the host it holds
/// the hub, and is then sent the hub's bodies, which continue into the ones
/// it built: it ends in the hub holding every body of the hub that no seat
/// drives, with the host's rows, and none twice. Alice, who crossed first,
/// left the guest's view and comes back into it with Bob: the guest builds
/// her body again from her player spawn row and holds her where the host
/// has her. The control is a guest that names no seat: it never leaves
/// `switch_lab`.
#[test]
fn a_hosted_guest_follows_its_players_crossing() {
    let (room, in_hub, on_guest, twice, (alice_on_host, alice_on_guest)) = bob_crosses_from_the_guest(true);
    let (control_room, ..) = bob_crosses_from_the_guest(false);
    let missing: Vec<_> = in_hub.iter().filter(|(id, row)| on_guest.get(*id) != Some(*row)).map(|(id, _)| id.clone()).collect();
    println!(
        "[hosted follow] the guest's Bob in {room:?} (control {control_room:?}); the hub's {} bodies no seat drives, {} not held with the host's row {missing:?}; held twice {twice}; Alice on the host {alice_on_host:?}, on the guest {alice_on_guest:?}",
        in_hub.len(),
        missing.len()
    );
    assert_eq!(room.as_deref(), Some("central_hub_complex"), "the guest's Bob did not follow his crossing to the hub");
    assert!(!in_hub.is_empty(), "premise: the hub has bodies no seat drives");
    assert!(missing.is_empty(), "the guest does not hold the hub's bodies with the host's rows: {missing:?}");
    assert_eq!(twice, 0, "the guest holds a body twice: replication made its own beside the one the guest built");
    assert!(alice_on_host.is_some(), "premise: Alice stands in the hub on the host");
    assert_eq!(alice_on_guest, alice_on_host, "the guest does not hold Alice, back in its view, where the host has her");
    assert_eq!(control_room.as_deref(), Some(ROOM), "the control: a guest naming no seat followed anyway");
}

/// One run of a guest leaving: seat 1 joins the host's session in the hub; a
/// guest connects (as seat 1 with `seated`) and then disconnects. Returns
/// seat 1's membership and bodies on the host before and after.
fn a_guest_disconnects(seated: bool) -> ((bool, usize), (bool, usize)) {
    use ambition_platformer2d::session::participants::SessionParticipants;
    const HUB: &str = "central_hub_complex";
    let boot = |role: HostedRole| {
        Platformer2dSimHarness::build(fixed_60hz_room_options(HUB), |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            app.add_plugins((RepliconPlugins.set(ServerPlugin::new(PostUpdate)), HostedWorldPlugin { role }));
            app.finish();
            app.cleanup();
            Ok(())
        })
        .expect("the hub boots")
    };
    let seat_one = |sim: &mut Platformer2dSimHarness| {
        let member = {
            let world = sim.world_mut();
            world
                .query::<&SessionParticipants>()
                .iter(world)
                .any(|members| members.contains(ambition_platformer2d::characters::control::PlayerSlot(1)))
        };
        (member, crate::a_second_seat_joins_the_session::bodies_of_the_seat(sim).len())
    };
    let mut host = boot(HostedRole::Host);
    let mut guest = boot(HostedRole::Guest);
    simulate_nothing(&mut guest);
    crate::a_second_seat_joins_the_session::join(&mut host, 240).expect("premise: seat 1 joins the host's session");
    // `join` returns with seat 1 still holding Start, which would join it
    // again on the tick it leaves: release it first.
    host.drive_seat(1, Default::default());
    for _ in 0..15 {
        host.app_mut().update();
    }
    host.app_mut().connect_client(guest.app_mut());
    if seated {
        let world = host.world_mut();
        let client = world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ConnectedClient>>()
            .single(world)
            .expect("one guest is connected");
        world
            .entity_mut(client)
            .insert(ambition_hosted_world::GuestSeat(ambition_platformer2d::characters::control::PlayerSlot(1)));
    }
    for _ in 0..10 {
        host.app_mut().update();
        host.app_mut().exchange_with_client(guest.app_mut());
        guest.app_mut().update();
    }
    let before = seat_one(&mut host);
    host.app_mut().disconnect_client(guest.app_mut());
    for _ in 0..10 {
        host.app_mut().update();
    }
    (before, seat_one(&mut host))
}

/// LOCAL-PLAY-MODES' remote leave, on the hosted world: a guest seated as
/// seat 1 disconnects, and the host retires seat 1 through the stamped leave
/// a local seat's "Leave Game" writes (membership ends, its body retires).
/// The control is a guest that names no seat: its disconnect leaves seat 1
/// in the session.
#[test]
fn a_hosted_guest_that_disconnects_leaves_the_session() {
    let (before, after) = a_guest_disconnects(true);
    let (_, unseated_after) = a_guest_disconnects(false);
    println!("[hosted leave] seat 1 (member, bodies): before {before:?}, after {after:?}; with no seat named after {unseated_after:?}");
    assert_eq!(before, (true, 1), "premise: seat 1 is a member with one body before the guest leaves");
    assert_eq!(after, (false, 0), "the departed guest's seat is still in the session");
    assert_eq!(unseated_after, (true, 1), "the control: a guest naming no seat took seat 1 with it");
}

/// One App of the lab at five past ten, its scientists' lives about to plan.
fn booted_in_the_lab(role: HostedRole) -> Platformer2dSimHarness {
    use ambition_platformer2d::time::{DayClock, DayCycle};
    let mut sim = Platformer2dSimHarness::build(fixed_60hz_room_options("eureka_coil_lab"), |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        app.add_plugins((RepliconPlugins.set(ServerPlugin::new(PostUpdate)), HostedWorldPlugin { role }));
        app.finish();
        app.cleanup();
        Ok(())
    })
    .expect("the lab boots");
    sim.step(crate::common::base());
    let world = sim.world_mut();
    let cycle = *world.resource::<DayCycle>();
    *world.resource_mut::<DayClock>() = DayClock::at_hour(&cycle, 10.05);
    sim
}

fn lives(
    sim: &mut Platformer2dSimHarness,
) -> std::collections::BTreeMap<ambition_platformer2d::platformer::sim_id::SimId, ambition_platformer2d::actors::body::life::Life> {
    let world = sim.world_mut();
    world
        .query::<(&ambition_platformer2d::platformer::sim_id::SimId, &ambition_platformer2d::actors::body::life::Life)>()
        .iter(world)
        .map(|(id, life)| (id.clone(), life.clone()))
        .collect()
}

/// NA-3's NPC lives through `bevy_replicon`: the host plays the lab from five
/// past ten until a scientist holds the workbench, and 30 frames more. A
/// guest that simulates nothing holds each scientist's life (its plan, where
/// it is in it, its claim) as the host has it after every snapshot. The
/// control is the same gated guest with no link: its lives never leave their
/// authored start.
#[test]
fn a_hosted_guest_holds_the_hosts_lives() {
    let mut host = booted_in_the_lab(HostedRole::Host);
    let mut guest = booted_in_the_lab(HostedRole::Guest);
    let mut control = booted_in_the_lab(HostedRole::Guest);
    simulate_nothing(&mut guest);
    simulate_nothing(&mut control);
    host.app_mut().connect_client(guest.app_mut());
    let holds_the_bench = |lives: &std::collections::BTreeMap<_, ambition_platformer2d::actors::body::life::Life>| {
        lives.values().any(|life| life.claim.as_deref() == Some("workbench"))
    };
    let (mut frames, mut differ, mut bytes, mut after_the_bench) = (0usize, 0usize, 0usize, None);
    while after_the_bench.is_none_or(|left| left > 0) {
        assert!(frames < 20_000, "premise: a scientist takes the bench on the host");
        host.app_mut().update();
        bytes += host.world().resource::<ServerMessages>().iter_sent().map(|(_, _, sent)| sent.len()).sum::<usize>();
        host.app_mut().exchange_with_client(guest.app_mut());
        guest.app_mut().update();
        control.app_mut().update();
        frames += 1;
        let on_host = lives(&mut host);
        if lives(&mut guest) != on_host {
            differ += 1;
        }
        after_the_bench = match after_the_bench {
            None if holds_the_bench(&on_host) => Some(30),
            None => None,
            Some(left) => Some(left - 1),
        };
    }
    let on_host = lives(&mut host);
    let on_control = lives(&mut control);
    let summary = |lives: &std::collections::BTreeMap<_, ambition_platformer2d::actors::body::life::Life>| {
        lives
            .values()
            .map(|life| format!("{:?} claim {:?} plan {:?} #{}", life.doing, life.claim, life.planned_in, life.revision))
            .collect::<Vec<_>>()
    };
    println!(
        "[hosted lives] {frames} frames; {} lives on the host {:?}; frames the guest's differ {differ}; {:.1} link bytes a frame; the control's {:?}",
        on_host.len(),
        summary(&on_host),
        bytes as f64 / frames as f64,
        summary(&on_control),
    );
    assert!(on_host.len() >= 2, "premise: the lab has its two scientists' lives");
    assert_eq!(differ, 0, "the guest's lives differ from the host's after a snapshot");
    assert_ne!(on_control, on_host, "the control: a gated guest with no link knows the host's lives anyway");
}
