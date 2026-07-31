//! The smash demo's shell, as a function — so the binary and the regression
//! tests assemble the SAME app.
//!
//! This crate exists for one reason that the content crate cannot supply: until
//! something runs the stage, every claim about the stocks loop is a unit test.
//! The loop is spend → respawn → eliminate → end, and each step is covered
//! individually; what is NOT covered anywhere is that a fighter knocked off THIS
//! platform, with THIS blast margin, reaches the world's edge at all.
//!
//! That is the gap a demo app closes and a fixture cannot: the numbers stop
//! being arguments and start being a fight.

use bevy::prelude::*;

/// Assemble the demo: foundation + engine group + host group + the smash
/// experience under a standalone shell host. Zero engine edits, zero
/// `ambition_app`.
pub fn build_demo_app() -> App {
    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    compose_smash_shell(&mut app);
    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

fn compose_smash_shell(app: &mut App) {
    ambition::provider::ShellComposition::new(
        ambition_demo_smash::SMASH_EXPERIENCE,
        ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
    )
    .install(app, ambition_demo_smash::SmashExperiencePlugin);
}
