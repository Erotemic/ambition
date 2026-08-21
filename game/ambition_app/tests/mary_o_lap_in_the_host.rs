#![cfg(all(feature = "input", feature = "visible"))]

//! **THE LAP, IN THE HOST JON ACTUALLY PLAYS — launcher, shell, rollback.**
//!
//! Jon, 2026-08-20: *"In maryo finishing 1-1 sends you back to 1-1."* The lap
//! test that answers this in `ambition_demo_mary_o_app` is GREEN: warped to each
//! authored pole it goes 1-1 → 1-2 → 1-3 → 1-1, right level every leg. So does
//! every other Mary-O test. And he sees 1-1 → 1-1.
//!
//! ⛔⛔ **because none of them build the composition that ships.** He reaches
//! Mary-O through `run_game.sh` → the launcher → the shell host, and
//! `build_visible_app` sets `SimulationHost::Rollback` for every route. The
//! standalone demo binary is `PlatformerEnginePlugins::fixed_tick()`. Those are
//! different simulation hosts running different schedules, and the difference
//! has already cost one live bug tonight: TwinTrack's second seat was inert in
//! the game and drove fine in its own binary, because a GGRS session publishes
//! what a fixed-tick host publishes for itself.
//!
//! ⚠ **a demo binary is not a coverage argument for the game.** That is the
//! lesson this file exists to hold, and it is worth more than the assertion
//! below: whenever a road is "tested" only by `build_demo_app()`, the shipped
//! path through the launcher is untested no matter how green the suite is.

use bevy::prelude::*;

use ambition_demo_mary_o::level_1_2::LEVEL_1_2_ROOM_ID;
use ambition_demo_mary_o::LEVEL_1_1_ROOM_ID;
use ambition_platformer2d::actors::actor::PrimaryPlayer;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::game_shell::{ShellLaunchCatalog, ShellLauncherCommand};
use ambition_platformer2d::world::rooms::RoomSet;
use ambition_app::app::shell_host;

/// Frames a leg may take before we call the transition wedged.
const COMMIT_CAP: usize = 900;

fn host_app() -> App {
    use bevy::asset::AssetPlugin;
    use bevy::image::ImagePlugin;
    use bevy::state::app::StatesPlugin;
    use bevy::transform::TransformPlugin;
    use bevy::MinimalPlugins;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::platformer::schedule::GameMode>();
    app.insert_resource(shell_host::AmbitionShellHosted);
    // ⛔ **the whole point of this file.** Without it the test falls to the
    // render-frame default and stops being the shipped composition.
    use ambition_platformer2d::runtime::SimulationHostAppExt as _;
    app.set_simulation_host(ambition_platformer2d::runtime::SimulationHost::Rollback);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    app.add_plugins(ambition_platformer2d::host::PlatformerHostPlugins);
    shell_host::compose_ambition_shell_host(&mut app);
    // ⚠ **and the clock has to move.** Under GGRS the simulation advances on the
    // fixed timestep; a test that calls `update()` inside a millisecond of real
    // time runs the GGRS schedule zero times and every reading is the boot state.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

fn room_id(app: &mut App) -> Option<String> {
    let mut q = app.world_mut().query::<&RoomSet>();
    q.iter(app.world())
        .next()
        .map(|set| set.rooms[set.active].id.clone())
}

/// Launch the row with this label, the way the launcher does.
fn launch(app: &mut App, label: &str) {
    let (index, total) = {
        let catalog = app.world().resource::<ShellLaunchCatalog>();
        let index = catalog
            .entries
            .iter()
            .position(|entry| entry.label == label)
            .unwrap_or_else(|| {
                let offered: Vec<&str> =
                    catalog.entries.iter().map(|e| e.label.as_str()).collect();
                panic!("the launcher offers no `{label}` row; it offers {offered:?}")
            });
        let exit = app
            .world()
            .resource::<ambition_platformer2d::game_shell::ShellLauncherPresentation>()
            .exit_label
            .is_some() as usize;
        (index, catalog.entries.len() + exit)
    };
    let current = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellLauncherState>()
        .selected;
    for _ in 0..((index + total - current % total) % total) {
        app.world_mut().write_message(ShellLauncherCommand::Next);
        app.update();
    }
    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    for _ in 0..240 {
        app.update();
        if room_id(app).is_some() {
            return;
        }
    }
    panic!("launching `{label}` from the host launcher never opened a room");
}

/// Finish `from` by touching its OWN authored pole; report where she landed.
fn finish_at_the_pole(app: &mut App, from: &str) -> String {
    let pole = ambition_demo_mary_o::pole_for_room(from);
    {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>();
        let world = app.world_mut();
        let mut body = q
            .iter_mut(world)
            .next()
            .unwrap_or_else(|| panic!("no controlled body in '{from}' to carry to its pole"));
        body.pos = ae::Vec2::new(pole.x, pole.base_y - 8.0);
        body.vel = ae::Vec2::ZERO;
    }
    for _ in 0..COMMIT_CAP {
        app.update();
        if let Some(id) = room_id(app) {
            if id != from {
                for _ in 0..30 {
                    app.update();
                }
                return id;
            }
        }
    }
    panic!(
        "in the HOST, touching the authored pole of '{from}' never changed the \
         room within {COMMIT_CAP} frames. ⚠ this is the composition Jon plays; \
         the same walk in `ambition_demo_mary_o_app` is green."
    );
}

/// **Finishing 1-1 in the host lands her in 1-2, not back in 1-1.**
#[test]
fn finishing_the_first_level_in_the_host_lands_in_the_second() {
    let mut app = host_app();
    for _ in 0..8 {
        app.update();
    }
    launch(&mut app, "Mary-O");
    let opened = room_id(&mut app).expect("Mary-O opened a room");
    assert_eq!(
        opened, LEVEL_1_1_ROOM_ID,
        "the host launched Mary-O into '{opened}' rather than 1-1",
    );

    let landed = finish_at_the_pole(&mut app, LEVEL_1_1_ROOM_ID);
    assert_eq!(
        landed, LEVEL_1_2_ROOM_ID,
        "finishing 1-1 in the HOST put her in '{landed}'. ⛔ if that is 1-1 \
         itself, `exit_for_room` answered `Replay` on the shipped path while \
         answering correctly in the demo binary — which is the shape of every \
         host-only bug in this tree.",
    );
    let mut q = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    let world = app.world_mut();
    assert!(
        q.iter(world).next().is_some(),
        "she arrived in 1-2 and there is no controlled body in it",
    );
}
