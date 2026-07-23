//! Outlander running headlessly — the Phase-6 "runs visibly and headlessly
//! from the same content" proof, headless half. Mirrors the in-repo standalone
//! demo shells (`ambition_demo_mary_o_app`): engine foundation + host +
//! minimal shell + THIS crate's provider plugin, zero engine edits.

use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    app.add_plugins(ambition::game_shell::MinimalShellPlugins);
    app.add_plugins(ambition::load::AmbitionLoadPlugin);
    app.add_plugins(outlander::OutlanderExperiencePlugin);

    // Pin frame dt to the tick dt so one update is one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));

    for _ in 0..120 {
        app.update();
    }
    println!("outlander: 120 headless ticks complete");
}
