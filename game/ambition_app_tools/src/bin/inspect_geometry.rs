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
//!
//! `--cost` (GOBS-5) reports instead what each selected provider costs: its
//! pieces, how many are inactive, the median time of 21 runs of it alone,
//! and the size of its JSON; then the whole observation's.

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
            "usage: inspect_geometry [--list] [--room ROOM] [--ticks N] [--provider A,B,prefix.] [--no-subject] [--out FILE] [--cost]"
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
    let query = ObservationQuery { room: root, subject };
    if args.iter().any(|arg| arg == "--cost") {
        report_cost(world, query, &catalog, &selection);
        return;
    }
    let layers = observe_all(world, query, |provider| selected(&selection, provider));
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

/// The median time of `runs` observations through `wanted`, and the last answer.
fn timed(
    world: &mut bevy::prelude::World,
    query: ObservationQuery,
    wanted: impl Fn(&str) -> bool + Copy,
    runs: usize,
) -> (std::time::Duration, Vec<ambition_platformer2d::geometry_observation::ObservedLayer>) {
    let mut times = Vec::with_capacity(runs);
    let mut layers = Vec::new();
    for _ in 0..runs {
        let start = std::time::Instant::now();
        layers = observe_all(world, query, wanted);
        times.push(start.elapsed());
    }
    times.sort();
    (times[runs / 2], layers)
}

/// Each selected provider's pieces, inactive pieces, median time and JSON
/// size, then the whole observation's (GOBS-5).
fn report_cost(
    world: &mut bevy::prelude::World,
    query: ObservationQuery,
    catalog: &[(String, String, bool)],
    selection: &[String],
) {
    use ambition_platformer2d::geometry_observation::{ObservedState, ObserverAnswer};
    const RUNS: usize = 21;
    println!("{:<24} {:>7} {:>9} {:>11} {:>10}", "provider", "pieces", "inactive", "median_us", "json_bytes");
    for (name, ..) in catalog.iter().filter(|(name, ..)| selected(selection, name)) {
        let (median, layers) = timed(world, query, |provider| provider == name, RUNS);
        let (pieces, inactive, status) = match layers.first().map(|layer| &layer.answer) {
            Some(ObserverAnswer::Observed { geometry }) => (
                geometry.len(),
                geometry.iter().filter(|piece| matches!(piece.state, ObservedState::Inactive { .. })).count(),
                "",
            ),
            Some(ObserverAnswer::NeedsSubject) => (0, 0, " (needs a subject)"),
            Some(ObserverAnswer::Unavailable { .. }) => (0, 0, " (unavailable)"),
            None => (0, 0, " (not run)"),
        };
        println!(
            "{name:<24} {pieces:>7} {inactive:>9} {:>11} {:>10}{status}",
            median.as_micros(),
            to_json(&layers).len()
        );
    }
    let (median, layers) = timed(world, query, |provider| selected(selection, provider), RUNS);
    let pieces: usize = layers
        .iter()
        .map(|layer| match &layer.answer {
            ObserverAnswer::Observed { geometry } => geometry.len(),
            _ => 0,
        })
        .sum();
    println!("{:<24} {pieces:>7} {:>9} {:>11} {:>10}", "(all selected)", "", median.as_micros(), to_json(&layers).len());
}
