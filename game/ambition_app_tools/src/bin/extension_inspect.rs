//! Extension inspect: what the procedural extension host runs, and what its
//! modules remember, in the assembled game, as text (fast-iteration I7 item 5).
//!
//! It answers, without a debugger or World access: which ports are installed;
//! which entries run, in which order, with linked or loaded code; what each
//! reads, writes and requests; which modules a loaded file replaced; the
//! generation text the content identity holds; and, after N ticks, each
//! body's records by field name. With `--try-replace` it stages a rebuilt
//! module file the way a hot reload does and prints whether admission takes
//! it, and why not.
//!
//! ```text
//! cargo run -p ambition_app_tools --release --bin extension_inspect -- \
//!     [--modules target/extension-modules/wasm32-unknown-unknown/release] \
//!     [--boss mockingbird] [--ticks 600] [--try-replace path/to/modules.wasm] [--time]
//! ```
//!
//! `--time` reports the wall time of the ticks (build it `--release`): run it
//! with and without `--modules` to see what the loaded road costs.

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions,
    TimestepMode,
};
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::BossBrain;
use ambition_platformer2d::extension::inspect;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;

struct Args {
    modules: Vec<std::path::PathBuf>,
    boss: Option<String>,
    ticks: usize,
    try_replace: Option<std::path::PathBuf>,
    time: bool,
}

fn parse() -> Args {
    let mut args = Args {
        modules: Vec::new(),
        boss: None,
        ticks: 0,
        try_replace: None,
        time: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = || it.next().unwrap_or_else(|| panic!("{arg} needs a value"));
        match arg.as_str() {
            "--modules" => args.modules.push(value().into()),
            "--boss" => args.boss = Some(value()),
            "--ticks" => args.ticks = value().parse().expect("--ticks takes a number"),
            "--try-replace" => args.try_replace = Some(value().into()),
            "--time" => args.time = true,
            "-h" | "--help" => {
                println!(
                    "extension_inspect [--modules PATH]... [--boss PROFILE] [--ticks N] [--try-replace FILE]"
                );
                std::process::exit(0);
            }
            other => panic!("unknown argument {other}; see --help"),
        }
    }
    args
}

fn main() {
    let args = parse();
    let mut options =
        Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    if !args.modules.is_empty() {
        options = options.with_extension_module_files(args.modules.clone());
    }
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");

    if let Some(profile) = &args.boss {
        let (px, py) = {
            let world = sim.world_mut();
            let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
            let kin = q.single(world).expect("the sandbox has a primary player");
            (kin.pos.x, kin.pos.y)
        };
        sim.spawn_boss_at(
            &format!("inspect_{profile}"),
            profile,
            (px + 150.0, py - 40.0),
            (40.0, 40.0),
            BossBrain::PhaseScript {
                script_id: profile.clone(),
            },
        );
    }
    let start = std::time::Instant::now();
    for _ in 0..args.ticks {
        sim.step(AgentAction::default());
    }
    let elapsed = start.elapsed();
    if args.time && args.ticks > 0 {
        println!(
            "time: {} ticks in {:.3} s ({:.1} us per tick)",
            args.ticks,
            elapsed.as_secs_f64(),
            elapsed.as_secs_f64() * 1e6 / args.ticks as f64
        );
    }

    print!("{}", inspect::describe_composition(sim.world()));
    print!("{}", inspect::describe_records(sim.world_mut()));

    if let Some(file) = &args.try_replace {
        println!("try-replace {}:", file.display());
        let bytes = std::fs::read(file).unwrap_or_else(|e| panic!("cannot read {}: {e}", file.display()));
        match ambition_platformer2d::extension::WasmModules::load(&bytes) {
            Err(error) => println!("  REFUSED at load: {error:?}"),
            Ok((backend, modules)) => {
                let keys: Vec<String> = modules.iter().map(|m| m.key.to_string()).collect();
                println!("  the file provides {keys:?}");
                // The file replaces the modules loaded from the same path.
                match ambition_platformer2d::extension::reload::stage_loaded_replacement(
                    sim.world_mut(),
                    &file.display().to_string(),
                    backend,
                    modules,
                ) {
                    Ok(()) => println!("  ADMITTED: staged; a running game would publish it"),
                    Err(reason) => println!("  REFUSED at admission: {reason}"),
                }
            }
        }
    }
}
