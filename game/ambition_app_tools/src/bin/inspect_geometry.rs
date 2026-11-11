//! Inspect geometry: what each installed geometry provider says a room holds,
//! headless, as JSON (`ambition.geometry_observation.v1`, room pixels, y down).
//!
//! The same answer F1's Observed Geometry layer draws (`observe_all`): one
//! layer per provider, each `observed` (its shapes, active or inactive with
//! the owner's reason), `needs_subject`, or `unavailable`. Owner:
//! `docs/planning/engine/geometry-observability-and-f1.md` (GOBS-1).
//!
//! ```text
//! cargo run -p ambition_app_tools --bin inspect_geometry -- --list
//! cargo run -p ambition_app_tools --bin inspect_geometry -- --room basement_breakables
//! cargo run -p ambition_app_tools --bin inspect_geometry -- --room basement_breakables \
//!     --provider collision.composed,combat.volumes --ticks 30 --no-subject --out geometry.json
//! ```
//!
//! `--room` boots that room (default: the game's start room) and steps
//! `--ticks` frames (default 5) with no input. The subject is Player 1 unless
//! `--no-subject` asks for the room's own view. `--provider` takes a comma
//! list of provider names, or of prefixes ending in `.` (`collision.`).
//! `--out FILE` writes the JSON there alone: stdout also carries the host's
//! `[live-reload]` acknowledgement lines.

use ambition_app::rl_sim::{AmbitionSim as _, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::geometry_observation::{GeometryObservers, ObservationQuery, observe_all, to_json};
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::{Entity, With};

fn value_of(args: &[String], name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    args.iter().enumerate().find_map(|(i, arg)| {
        if arg == name {
            args.get(i + 1).cloned()
        } else {
            arg.strip_prefix(&prefix).map(str::to_string)
        }
    })
}

/// Whether `provider` is asked for: every provider when `selection` is
/// empty; otherwise a listed name, or a listed prefix ending in `.`.
fn selected(selection: &[String], provider: &str) -> bool {
    selection.is_empty()
        || selection
            .iter()
            .any(|wanted| wanted == provider || (wanted.ends_with('.') && provider.starts_with(wanted.as_str())))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "usage: inspect_geometry [--list] [--room ROOM] [--ticks N] [--provider A,B,prefix.] [--no-subject] [--out FILE]"
        );
        return;
    }
    let mut options = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    if let Some(room) = value_of(&args, "--room") {
        options = options.with_required_start_room(&room);
    }
    let mut sim = match Platformer2dSimHarness::new_with_options(options) {
        Ok(sim) => sim,
        Err(error) => {
            eprintln!("inspect_geometry: the room did not boot: {error:?}");
            std::process::exit(2);
        }
    };
    let world = sim.world_mut();
    let catalog: Vec<(String, String, bool)> = world
        .get_resource::<GeometryObservers>()
        .map(|observers| {
            observers.catalog().map(|info| (info.name.to_string(), info.description.to_string(), info.needs_subject)).collect()
        })
        .unwrap_or_default();
    if args.iter().any(|arg| arg == "--list") {
        for (name, description, needs_subject) in &catalog {
            println!("{name}{}: {description}", if *needs_subject { " (needs a subject)" } else { "" });
        }
        return;
    }
    let selection: Vec<String> = value_of(&args, "--provider")
        .map(|list| list.split(',').map(str::trim).filter(|name| !name.is_empty()).map(str::to_string).collect())
        .unwrap_or_default();
    let unknown: Vec<&String> = selection
        .iter()
        .filter(|wanted| !catalog.iter().any(|(name, ..)| selected(std::slice::from_ref(*wanted), name)))
        .collect();
    if !unknown.is_empty() {
        let names: Vec<&str> = catalog.iter().map(|(name, ..)| name.as_str()).collect();
        eprintln!("inspect_geometry: no provider is named {unknown:?}; installed: {names:?}");
        std::process::exit(2);
    }
    let ticks: u32 = value_of(&args, "--ticks").and_then(|ticks| ticks.parse().ok()).unwrap_or(5);
    for _ in 0..ticks {
        sim.app_mut().update();
    }

    let world = sim.world_mut();
    let Ok((player, room)) =
        world.query_filtered::<(Entity, &InRoomInstance), With<PrimaryPlayer>>().single(world).map(|(player, room)| (player, room.0))
    else {
        eprintln!("inspect_geometry: no Player 1 in a room after {ticks} ticks");
        std::process::exit(2);
    };
    let root = world
        .query_filtered::<(Entity, &LiveRoomInstance), With<RoomInstanceRoot>>()
        .iter(world)
        .find(|(_, live)| **live == room)
        .map(|(root, _)| root);
    let subject = (!args.iter().any(|arg| arg == "--no-subject")).then_some(player);
    let layers = observe_all(world, ObservationQuery { room: root, subject }, |provider| selected(&selection, provider));
    let json = to_json(&layers);
    match value_of(&args, "--out") {
        Some(path) => {
            if let Err(error) = std::fs::write(&path, json) {
                eprintln!("inspect_geometry: could not write {path}: {error}");
                std::process::exit(2);
            }
        }
        None => println!("{json}"),
    }
}
