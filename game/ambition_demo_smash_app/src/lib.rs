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
pub mod stage_diagram;

pub fn build_demo_app() -> App {
    let mut app = App::new();
    ambition_platformer2d::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    compose_smash_shell(&mut app);
    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

fn compose_smash_shell(app: &mut App) {
    // HOME is the select screen, not the stage. A platform fighter that opens on
    // the stage has already decided who you are, and leaving a match should
    // return to the screen that chose it rather than to a launcher listing one
    // experience.
    ambition_platformer2d::provider::ShellComposition::new(
        ambition_demo_smash::SMASH_EXPERIENCE,
        ambition_demo_smash::SMASH_SELECT_ROUTE,
        ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
    )
    // Boot onto SELECT. Neither `PrimaryGameplay` nor `Launcher` says this, which
    // is why `starting_at` exists.
    .starting_at(ambition_demo_smash::SMASH_SELECT_ROUTE)
    .install(app, ambition_demo_smash::SmashExperiencePlugin);
}
