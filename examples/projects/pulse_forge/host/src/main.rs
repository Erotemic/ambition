//! The project host with Pulse Forge's Rust module linked.

use ambition_platformer2d::extension::ExtensionAppExt;

fn value(args: &[String], flag: &str) -> Option<String> {
    args.iter().enumerate().find_map(|(index, arg)| {
        if arg == flag { args.get(index + 1).cloned() }
        else { arg.strip_prefix(&format!("{flag}=")).map(str::to_owned) }
    })
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let project = value(&args, "--project").unwrap_or_else(|| {
        eprintln!("pulse_forge_host needs --project PATH; select project-rhai.ron, project-wasm.ron, or project-linked.ron");
        std::process::exit(2);
    });
    if let Some(ticks) = value(&args, "--headless-ticks") {
        let project = ambition_app::app::project::Project::load(std::path::Path::new(&project)).expect("the project loads");
        let options = ambition_app::Platformer2dSimHarnessOptions::default()
            .with_timestep(ambition_app::TimestepMode::fixed_60hz());
        let mut sim = pulse_forge_host::sim(&project, options).expect("the project starts");
        for _ in 0..ticks.parse::<u32>().expect("--headless-ticks needs a nonnegative integer") {
            sim.step(ambition_app::AgentAction::default());
        }
        if let Some(file) = value(&args, "--compare") {
            let linked = ambition_platformer2d::extension::linked_modules(sim.world());
            let comparison = ambition_platformer2d::extension::load_module_file(std::path::Path::new(&file), &linked)
                .and_then(|(_, modules)| ambition_platformer2d::extension::inspect::compare_contracts(sim.world(), &modules));
            match comparison {
                Ok(report) => print!("{report}"),
                Err(reason) => { eprintln!("{reason}"); std::process::exit(1); }
            }
        }
        println!("project: {}", project.name);
        print!("{}", ambition_platformer2d::extension::inspect::describe_composition(sim.world()));
        print!("{}", ambition_platformer2d::extension::inspect::describe_records(sim.world_mut()));
        return;
    }
    let frames = value(&args, "--presentation-frames");
    let mode = if frames.is_some() { ambition_app::app::VisibleRenderMode::NoWindow }
        else { ambition_app::app::VisibleRenderMode::Windowed };
    let mut app = ambition_app::app::build_visible_app_with(
        mode, false, |app| { app.add_extension_module(pulse_forge_modules::module()); },
    );
    if let Some(frames) = frames {
        app.finish();
        app.cleanup();
        for _ in 0..frames.parse::<u32>().expect("--presentation-frames needs a nonnegative integer") {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let world = app.world_mut();
        assert!(world.resource::<ambition_platformer2d::game_shell::ActiveGameplaySession>().0.is_some(),
            "the presentation probe did not activate the project");
        println!("project: {}", world.resource::<ambition_app::app::project::ProjectWatch>().project.name);
        print!("{}", ambition_platformer2d::extension::inspect::describe_composition(world));
    } else {
        app.run();
    }
}
