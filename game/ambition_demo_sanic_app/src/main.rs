//! **The Sanic demo's shell — playbook exit 3, executable.**
//!
//! > *"A demo app builds from runtime+host groups + its content crate with zero
//! > engine edits."* — `docs/planning/engine/decomposition.md`, exit criterion 3.
//!
//! This file is that sentence, compiled. It is the shape
//! `crates/ambition_host/tests/demo_shell_smoke.rs` prescribes and
//! `docs/planning/demos/README.md` mandates for every demo:
//!
//! ```text
//!   foundation
//!   + PlatformerEnginePlugins   (the engine, content-free)
//!   + PlatformerHostPlugins     (the windowed host's camera + input)
//!   + SanicDemoContentPlugin    (this demo's roster + world)
//!   + SanicRulesPlugin::global()(this demo's rules — it IS the game here)
//! ```
//!
//! It names `ambition` and `ambition_demo_sanic`. It does not name `ambition_app`,
//! and `git log --stat` for this crate touches zero engine crates. If a demo ever
//! needs an engine change to boot, that is an oracle violation and gets filed in
//! `docs/planning/tracks.md`, not patched here.
//!
//! ## What it does and does not show
//!
//! It runs the REAL simulation: the speedway room, its rideable Sonic loop, a
//! player body on the momentum kernel, and the mode-scoped act timer. It steps
//! that sim on the fixed 60 Hz timeline and reports what the sim knows.
//!
//! It draws nothing. **That is a known engine gap, not a demo shortcut:** room
//! and block VISUALS are spawned by `ambition_app`'s room-flow, which is app-local
//! residue the E5 carve left behind. A windowed demo needs that lifted into an
//! engine-side plugin. Filed as the first entry in tracks.md's oracle-violation
//! log. Until it lands, the shell proves the ASSEMBLY, which is what exit 3 asks.
//!
//! ```console
//! $ cargo run -p ambition_demo_sanic_app --bin sanic_demo -- --ticks 600
//! ```

use bevy::prelude::*;

use ambition_demo_sanic::{SanicActState, SANIC_MODE};

/// How many sim ticks to run before reporting. One second = 60.
const DEFAULT_TICKS: u32 = 300;

fn main() {
    let ticks = parse_ticks().unwrap_or(DEFAULT_TICKS);

    // The assembly lives in `lib.rs` so the exit-3 regression test builds the
    // SAME app this binary does.
    let mut app = ambition_demo_sanic_app::build_demo_app();

    app.update(); // Startup: builds the world, spawns the body. Zero ticks (dt=0).
    for _ in 0..ticks {
        app.update();
    }

    report(&mut app, ticks);
}

fn parse_ticks() -> Option<u32> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--ticks" {
            return args.next()?.parse().ok();
        }
    }
    None
}

/// Read the sim through the same seams any consumer uses — the canonical timeline,
/// the mode-scoped act state, and the body's kinematics.
fn report(app: &mut App, requested: u32) {
    let tick = app.world().resource::<ambition::runtime::SimTick>().get();

    let elapsed = {
        let mut q = app.world_mut().query::<&SanicActState>();
        q.iter(app.world()).next().map(|s| s.elapsed)
    };

    let body = {
        let mut q = app
            .world_mut()
            .query_filtered::<&ambition::actors::actor::BodyKinematics, With<
                ambition::actors::actor::PrimaryPlayer,
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
    println!("  Nothing was drawn: room/block visuals are still spawned app-side");
    println!("  (`ambition_app`'s room-flow). See the oracle-violation log in");
    println!("  docs/planning/tracks.md.");
}
