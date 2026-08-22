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
    #[cfg(feature = "visible")]
    app.add_plugins(
        ambition_platformer2d::render::rendering::debug_viz::DebugVizPlugin::default(),
    );
    compose_smash_shell(&mut app);
    // This shell was the third and never joined.
    //
    // **AFTER `compose_smash_shell`, because the plugin READS the catalogs it
    // registers** — the plugin's own doc says so, and it panics naming the
    // composition-order mistake rather than booting art-less.
    //
    // **`visible` only, deliberately.** `build_demo_app` is also the harness
    // for this crate's regression tests, and they assert on a stepping
    // simulation rather than on pixels; sanic draws the same line by keeping its
    // asset install in `build_windowed_demo_app`.
    #[cfg(feature = "visible")]
    {
        // No world manifest: the stage is authored in Rust, so this demo ships
        // no `.ldtk` and a world-less catalog contributes no world rows while
        // every other entry still lands — the same shape as sanic.
        app.add_plugins(
            ambition_platformer2d::game_assets::PlatformerAssetsPlugin::for_experience(
                ambition_demo_smash::SMASH_EXPERIENCE,
            )
            // Startup binding precedes activation, so the theme comes from the
            // authored stage rather than a session root that does not exist yet.
            .with_room(ambition_demo_smash::smash_stage().metadata.clone()),
        );
        app.add_plugins(ambition_platformer2d::presentation::PlatformerPresentationPlugin);
    }
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
    // by keying frontend audio on the route instead of the process.
    //
    // What stays here is the DEFAULT for this app's other frontend routes —
    // loading, and anything a future screen adds — which is a claim about this
    // composition and belongs to the composition.
    .with_frontend_audio(
        ambition_platformer2d::audio::selection::FrontendAudioProfile::new(
            ambition_demo_smash::SMASH_EXPERIENCE,
        )
        .with_title_track(ambition_demo_smash::SMASH_SELECT_TRACK)
        .with_sfx([
            ambition_platformer2d::sfx::ids::UI_MENU_MOVE,
            ambition_platformer2d::sfx::ids::UI_MENU_ACCEPT,
            ambition_platformer2d::sfx::ids::UI_MENU_BACK,
        ]),
    )
    .install(app, ambition_demo_smash::SmashExperiencePlugin);
}
