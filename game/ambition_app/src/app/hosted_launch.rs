//! The visible app as a hosted world's host or guest (NET-AUTHORITY-EXPERIMENT).
//!
//! `--host-world PORT` opens a UDP socket on loopback and seats each guest that
//! joins; `--join-world ADDRESS` joins one. Either composes the simulation on
//! the fixed 60 Hz host instead of GGRS: a hosted world has one authority, so
//! nothing rewinds the whole App. A guest simulates its own boot, then sleeps
//! once it is connected and holds a live room, and from then on shows the
//! host's bodies; its keyboard and pads (its seat 0) drive the seat the host
//! gave it.
//!
//! A guest may boot anywhere: until it has a body of its own it watches the
//! host's player, and its own room lifecycle builds the room that player is
//! in. A guest's bodies meet the host's by `SimId`.
//! The owner is `docs/planning/engine/hosted-world-networking-experiment.md`.

use std::net::SocketAddr;

use bevy::prelude::*;

use ambition_hosted_world::bevy_replicon::prelude::{RepliconPlugins, ServerPlugin};
use ambition_hosted_world::{udp, HostedRole, HostedWorldPlugin};

/// Which end of a hosted world this process is, from its command line.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostedLaunch {
    /// Open the world on loopback `port` (0: any free port).
    Host { port: u16 },
    /// Join the world at `server`.
    Guest { server: SocketAddr },
}

/// The most guests a host accepts.
const GUESTS: usize = 3;

/// `--host-world PORT` or `--join-world ADDRESS` (also `--flag=value`).
pub fn hosted_launch_from(args: &[String]) -> Result<Option<HostedLaunch>, String> {
    let value = |flag: &str| -> Option<String> {
        args.iter().enumerate().find_map(|(i, arg)| {
            if arg == flag {
                args.get(i + 1).cloned()
            } else {
                arg.strip_prefix(&format!("{flag}=")).map(str::to_owned)
            }
        })
    };
    match (value("--host-world"), value("--join-world")) {
        (Some(_), Some(_)) => Err("--host-world and --join-world name two ends of one process".into()),
        (Some(port), None) => port
            .parse()
            .map(|port| Some(HostedLaunch::Host { port }))
            .map_err(|_| format!("--host-world takes a port, not {port:?}")),
        (None, Some(server)) => server
            .parse()
            .map(|server| Some(HostedLaunch::Guest { server }))
            .map_err(|_| format!("--join-world takes an address like 127.0.0.1:47010, not {server:?}")),
        (None, None) => Ok(None),
    }
}

/// Compose this App as `launch` asks. Runs in the pre-simulation hook: the
/// simulation host must be chosen before the simulation plugins register.
pub fn compose_hosted_world(app: &mut App, launch: HostedLaunch) {
    use ambition_platformer2d::runtime::SimulationHostAppExt as _;
    app.set_simulation_host(ambition_platformer2d::runtime::SimulationHost::Fixed60Hz);
    let role = match launch {
        HostedLaunch::Host { .. } => HostedRole::Host,
        HostedLaunch::Guest { .. } => HostedRole::Guest,
    };
    app.add_plugins((
        RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
        udp::RepliconRenetPlugins,
        HostedWorldPlugin { role },
    ));
    app.insert_resource(launch);
    app.add_systems(Startup, open_the_link);
    match role {
        HostedRole::Host => {
            app.insert_resource(ambition_hosted_world::SeatsGuests);
        }
        HostedRole::Guest => {
            ambition_hosted_world::arm_the_guest_gate(app);
            app.insert_resource(ambition_hosted_world::InputFromSeat(
                ambition_platformer2d::characters::control::PlayerSlot(0),
            ));
            app.add_systems(Update, ambition_hosted_world::sleep_once_connected);
        }
    }
    app.add_systems(Update, report_the_link);
}

/// Open the host's socket or join it. A link that cannot open ends the process:
/// a host or guest with no link is not the game that was asked for.
fn open_the_link(world: &mut World) {
    let launch = *world.resource::<HostedLaunch>();
    let opened = match launch {
        HostedLaunch::Host { port } => udp::host(world, port, GUESTS).map(|address| format!("hosting on {address}")),
        HostedLaunch::Guest { server } => {
            // Netcode tells clients apart by id; the process id is one no other
            // guest on this machine holds.
            udp::join(world, server, u64::from(std::process::id())).map(|()| format!("joining {server}"))
        }
    };
    match opened {
        Ok(what) => eprintln!("ambition_app: hosted world: {what}"),
        Err(error) => {
            eprintln!("ambition_app: hosted world: the link did not open: {error}");
            std::process::exit(2);
        }
    }
}

/// Once a second: who this end is linked to, and what a guest holds.
fn report_the_link(
    time: Res<Time<Real>>,
    mut next: Local<f32>,
    launch: Res<HostedLaunch>,
    guests: Query<&ambition_hosted_world::GuestSeat>,
    seat: Option<Res<ambition_hosted_world::LocalSeat>>,
    asleep: Option<Res<ambition_hosted_world::SimulationAsleep>>,
    held: Query<(), With<ambition_hosted_world::BodyRow>>,
) {
    let now = time.elapsed_secs();
    if now < *next {
        return;
    }
    *next = now + 1.0;
    match *launch {
        HostedLaunch::Host { .. } => {
            let seats: Vec<u8> = guests.iter().map(|seat| seat.0 .0).collect();
            eprintln!("ambition_app: hosted world: host, guests at seats {seats:?}");
        }
        HostedLaunch::Guest { .. } => eprintln!(
            "ambition_app: hosted world: guest at seat {:?}, simulation {}, {} body row(s) held",
            seat.map(|seat| seat.0 .0),
            if asleep.is_some() { "asleep" } else { "awake" },
            held.iter().count()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn the_launch_reads_either_flag_and_refuses_both() {
        assert_eq!(hosted_launch_from(&args("game --host-world 47010")), Ok(Some(HostedLaunch::Host { port: 47010 })));
        assert_eq!(
            hosted_launch_from(&args("game --join-world=127.0.0.1:47010")),
            Ok(Some(HostedLaunch::Guest { server: "127.0.0.1:47010".parse().unwrap() }))
        );
        assert_eq!(hosted_launch_from(&args("game")), Ok(None));
        assert!(hosted_launch_from(&args("game --host-world 1 --join-world 127.0.0.1:1")).is_err());
        assert!(hosted_launch_from(&args("game --host-world here")).is_err());
    }
}
