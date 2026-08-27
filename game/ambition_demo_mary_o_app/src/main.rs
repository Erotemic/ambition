//! Demo executable composed from the reusable platformer runtime, windowed host,
//! and this demo's content/rules plugins. It intentionally does not depend on
//! `ambition_app`. Headless mode steps the real simulation and reports state;
//! the `visible` feature adds generic platformer presentation without the main
//! game's HUD, menus, or dev overlays.

use bevy::prelude::*;

use ambition_demo_mary_o::{MaryOLevelState, MARY_O_MODE, STARTING_TIME};

/// How many sim ticks to run before reporting. One second = 60.
const DEFAULT_TICKS: u32 = 300;

fn main() {
    #[cfg(feature = "visible")]
    if ambition_platformer2d::demo_shell::wants_a_window() {
        // The drawn demo. One plugin more than the sim-only shell below.
        let mut app = ambition_demo_mary_o_app::build_windowed_demo_app(
            ambition_demo_mary_o_app::RenderMode::Windowed,
        );
        if let Some(room) = parse_room() {
            app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(room));
        }
        app.run();
        return;
    }

    let ticks = ambition_platformer2d::demo_shell::headless_ticks(DEFAULT_TICKS);

    let mut app = ambition_demo_mary_o_app::build_demo_app();
    if let Some(room) = parse_room() {
        app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(room));
    }

    app.update(); // Startup: builds the world, spawns the body. Zero ticks (dt=0).
    for _ in 0..ticks {
        app.update();
    }

    report(&mut app, ticks);
}

/// Which room to open in. Absent means 1-1, the shipped entry.
///
/// validated here, for the reason `capture_mary_o` already states at its own
/// `--room`: the seam it feeds does NOT refuse. `RoomSet::from_parts` activates
/// room 0 for an id it does not hold, so an unknown room would silently open 1-1
/// and look like success — and a reviewer who asked for 1-3, got 1-1, and saw
/// nothing new would conclude the authoring was broken.
fn parse_room() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--room" {
            let asked = args.next().unwrap_or_else(|| {
                eprintln!("--room needs a room id");
                std::process::exit(2);
            });
            let known = ambition_demo_mary_o::provider::mary_o_room_ids();
            if !known.iter().any(|id| id == &asked) {
                eprintln!("unknown room '{asked}'; Mary-O has {known:?}");
                std::process::exit(2);
            }
            return Some(asked);
        }
    }
    None
}

/// Read the sim through the same seams any consumer uses — the canonical timeline,
/// the mode-scoped act state, and the body's kinematics.
fn report(app: &mut App, requested: u32) {
    let tick = app
        .world()
        .resource::<ambition_platformer2d::runtime::SimTick>()
        .get();

    let remaining = {
        let mut q = app.world_mut().query::<&MaryOLevelState>();
        q.iter(app.world()).next().map(|s| s.time_remaining)
    };

    let body = {
        let mut q = app
            .world_mut()
            .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, With<
                ambition_platformer2d::platformer::markers::PrimaryPlayer,
            >>();
        q.iter(app.world()).next().copied()
    };

    println!("mary_o_demo — the shell booted and stepped the real sim.");
    println!("  mode            : {MARY_O_MODE}");
    println!("  ticks requested : {requested}");
    println!("  SimTick         : {tick}");
    match remaining {
        Some(t) => println!(
            "  level clock     : {t:.1} / {STARTING_TIME:.0}  (mode-scoped; engine owns its teardown)"
        ),
        None => println!("  level clock     : ABSENT — the mode never woke. That is a bug."),
    }
    match body {
        Some(k) => println!(
            "  player body     : pos ({:.1}, {:.1})  vel ({:.1}, {:.1})",
            k.pos.x, k.pos.y, k.vel.x, k.vel.y
        ),
        None => println!("  player body     : ABSENT — `simulation_world` did not spawn it."),
    }
    println!();
    println!("  Nothing was drawn — this is the sim-only shell. Build with");
    println!("  `--features visible` and pass `--window` to draw level 1-1.");
}
