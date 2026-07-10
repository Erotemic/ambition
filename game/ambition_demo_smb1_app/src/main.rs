//! **The Super Mary-O demo's shell — playbook exit 3, executable.**
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
//!   + Smb1DemoContentPlugin    (this demo's roster + world)
//!   + Smb1RulesPlugin::global()(this demo's rules — it IS the game here)
//! ```
//!
//! It names `ambition` and `ambition_demo_smb1`. It does not name `ambition_app`,
//! and `git log --stat` for this crate touches zero engine crates. If a demo ever
//! needs an engine change to boot, that is an oracle violation and gets filed in
//! `docs/planning/tracks.md`, not patched here.
//!
//! ## What it does and does not show
//!
//! It runs the REAL simulation: level 1-1 room, its rideable Sonic loop, a
//! player body on the momentum kernel, and the mode-scoped act timer. It steps
//! that sim on the fixed 60 Hz timeline and reports what the sim knows.
//!
//! By default it draws nothing and prints what the sim knows — the sim-only shell,
//! which pays for no renderer at all. **Built with `--features visible` it opens a
//! window and draws level 1-1**, adding exactly one plugin:
//! `ambition_render`'s `PlatformerPresentationPlugin` (the engine's generic
//! presentation face, minted to close oracle-violation OV1). No HUD, no menus, no
//! dev overlays — those are the GAME's, and `ambition_app` still assembles them.
//!
//! ```console
//! $ cargo run -p ambition_demo_smb1_app --bin mary_o_demo -- --ticks 600
//! $ cargo run -p ambition_demo_smb1_app --features visible --bin mary_o_demo -- --window
//! ```

use bevy::prelude::*;

use ambition_demo_smb1::{Smb1LevelState, SMB1_MODE, STARTING_TIME};

/// How many sim ticks to run before reporting. One second = 60.
const DEFAULT_TICKS: u32 = 300;

fn main() {
    #[cfg(feature = "visible")]
    if std::env::args().any(|a| a == "--window") {
        // The drawn demo. One plugin more than the sim-only shell below.
        ambition_demo_smb1_app::build_windowed_demo_app(
            ambition_demo_smb1_app::RenderMode::Windowed,
        )
        .run();
        return;
    }

    let ticks = parse_ticks().unwrap_or(DEFAULT_TICKS);

    // The assembly lives in `lib.rs` so the exit-3 regression test builds the
    // SAME app this binary does.
    let mut app = ambition_demo_smb1_app::build_demo_app();

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

    let remaining = {
        let mut q = app.world_mut().query::<&Smb1LevelState>();
        q.iter(app.world()).next().map(|s| s.time_remaining)
    };

    let body = {
        let mut q = app
            .world_mut()
            .query_filtered::<&ambition::actors::actor::BodyKinematics, With<
                ambition::actors::actor::PrimaryPlayer,
            >>();
        q.iter(app.world()).next().copied()
    };

    println!("mary_o_demo — the shell booted and stepped the real sim.");
    println!("  mode            : {SMB1_MODE}");
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
