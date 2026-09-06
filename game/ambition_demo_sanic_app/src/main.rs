//! Demo executable composed from the reusable platformer runtime, windowed host,
//! and this demo's content/rules plugins. It intentionally does not depend on
//! `ambition_app`. Headless mode steps the real simulation and reports state;
//! the `visible` feature adds generic platformer presentation without the main
//! game's HUD, menus, or dev overlays.

use bevy::prelude::*;

use ambition_demo_sanic::{SanicActState, SANIC_MODE};

/// How many sim ticks to run before reporting. One second = 60.
const DEFAULT_TICKS: u32 = 300;

fn main() {
    #[cfg(feature = "visible")]
    if ambition_platformer2d::demo_shell::wants_a_window() {
        // The drawn demo. One plugin more than the sim-only shell below.
        ambition_demo_sanic_app::build_windowed_demo_app(
            ambition_demo_sanic_app::RenderMode::Windowed,
        )
        .run();
        return;
    }

    let ticks = ambition_platformer2d::demo_shell::headless_ticks(DEFAULT_TICKS);

    let mut app = ambition_demo_sanic_app::build_demo_app();

    app.update(); // Startup: builds the world, spawns the body. Zero ticks (dt=0).
    for _ in 0..ticks {
        app.update();
    }

    report(&mut app, ticks);
}

/// Read the sim through the same seams any consumer uses — the canonical timeline,
/// the mode-scoped act state, and the body's kinematics.
fn report(app: &mut App, requested: u32) {
    let tick = app
        .world()
        .resource::<ambition_platformer2d::runtime::SimTick>()
        .get();

    let elapsed = {
        let mut q = app.world_mut().query::<&SanicActState>();
        q.iter(app.world()).next().map(|s| s.elapsed)
    };

    let body = {
        let mut q = app
            .world_mut()
            .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, With<
                ambition_platformer2d::platformer::markers::PrimaryPlayer,
            >>();
        q.iter(app.world()).next().copied()
    };

    println!("sanic_demo — the shell booted and stepped the real sim.");
    println!("  mode            : {SANIC_MODE}");
    println!("  ticks requested : {requested}");
    println!("  SimTick         : {tick}");
    match elapsed {
        Some(t) => println!("  act timer       : {t:.3}s  (mode-scoped; engine owns its teardown)"),
        None => println!("  act timer       : ABSENT — the mode never woke. That is a bug."),
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
    println!("  `--features visible` and pass `--window` to draw the speedway.");
}
