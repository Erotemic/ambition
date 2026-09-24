//! The runnable smash demo. Unit tests cover each step of the stocks loop
//! (spend → respawn → eliminate → end). This app runs the stage, so tests can
//! check that a fighter knocked off this platform, with this blast margin,
//! reaches the world's edge.

use bevy::prelude::*;

/// Draw the stage and its blast margins as a PNG.
pub mod stage_diagram;

/// The `smash_tool` subcommands, one module each (see `tools/mod.rs`).
pub mod tools;

/// Assemble the headless demo: foundation, engine group, host group, and
/// the smash experience under a standalone shell host, with no engine edits
/// and no `ambition_app`. This is also the harness for this crate's tests.
pub fn build_demo_app() -> App {
    let mut app = App::new();
    ambition_platformer2d::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    #[cfg(feature = "visible")]
    app.add_plugins(ambition_platformer2d::render::rendering::debug_viz::DebugVizPlugin::default());
    compose_smash_shell(&mut app);
    // After `compose_smash_shell`: the assets plugin reads the catalogs the
    // shell registers, and panics on the wrong order.
    //
    // `visible` only: the regression tests use this builder and assert on the
    // stepping simulation, not on pixels.
    #[cfg(feature = "visible")]
    {
        // No world manifest: the stage is authored in Rust, so this demo ships
        // no `.ldtk`. A world-less catalog contributes no world rows; every
        // other entry still lands.
        app.add_plugins(
            ambition_platformer2d::game_assets::PlatformerAssetsPlugin::for_experience(
                ambition_demo_smash::SMASH_EXPERIENCE,
            )
            // Startup binding precedes activation, so the theme comes from the
            // authored stage.
            .with_room(ambition_demo_smash::smash_stage().metadata.clone()),
        );
        app.add_plugins(ambition_platformer2d::presentation::PlatformerPresentationPlugin);
    }
    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

/// The same demo, drawn.
///
/// [`build_demo_app`] starts from `add_headless_foundation`
/// (`MinimalPlugins` plus assets, images, transforms, and states), so it has
/// no renderer, window, or `winit` whatever features are on. This goes
/// through the engine's windowed foundation.
///
/// [`Display::Offscreen`] is a real backend with no window and no app
/// runner, so an offscreen caller steps the app itself. Unlike the old
/// hand-rolled builders, this foundation installs no `ScheduleRunnerPlugin`
/// on the offscreen arm; a `capture_*` binary that calls `run()` must add
/// it.
#[cfg(feature = "visible")]
pub fn build_windowed_demo_app(display: ambition_platformer2d::app::Display) -> App {
    let mut app = App::new();
    ambition_platformer2d::app::install_windowed_foundation(
        &mut app,
        "Super Smash Siblings",
        display,
    );
    app.add_plugins(ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    compose_smash_shell(&mut app);
    // After the shell: the assets plugin reads the catalogs it registers.
    app.add_plugins(
        ambition_platformer2d::game_assets::PlatformerAssetsPlugin::for_experience(
            ambition_demo_smash::SMASH_EXPERIENCE,
        )
        .with_room(ambition_demo_smash::smash_stage().metadata.clone()),
    );
    app.add_plugins(ambition_platformer2d::presentation::PlatformerPresentationPlugin);
    // One `update()` is exactly one sim tick, as in `build_demo_app`.
    let timestep = app
        .world()
        .resource::<bevy::time::Time<bevy::time::Fixed>>()
        .timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

fn compose_smash_shell(app: &mut App) {
    // Home is the select screen, not the stage: a platform fighter asks who
    // you are before the match, and leaving a match returns to that screen.
    ambition_platformer2d::provider::ShellComposition::new(
        ambition_demo_smash::SMASH_EXPERIENCE,
        ambition_demo_smash::SMASH_SELECT_ROUTE,
        ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
    )
    // Boot onto select. Neither `PrimaryGameplay` nor `Launcher` says this,
    // so use `starting_at`.
    .starting_at(ambition_demo_smash::SMASH_SELECT_ROUTE)
    // The default frontend audio for this app's other frontend routes
    // (loading, and any future screen). It belongs to this composition.
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

#[cfg(test)]
mod portal_presentation_tests {
    // No `use super::*`: this module builds no app. It checks the engine's
    // defaults; the behavioural half is
    // `the_ruleset_states_the_portal_presentation_not_the_binary`.

    /// The smash ruleset turns the seamless portal presentation off
    /// (`Pop`/`Static`). This checks the premise: the engine still defaults to
    /// `Continuous`/`Dynamic`, so the ruleset's choice is a real per-case
    /// override. If the engine default changes, this fails so that someone
    /// re-checks the override.
    #[test]
    fn the_engine_defaults_still_make_the_rulesets_choice_an_override() {
        use ambition_platformer2d::portal_presentation as portal_view;

        let engine_camera = portal_view::PortalCameraContinuitySelection::default().mode;
        let engine_cones = portal_view::PortalViewConeConfig::default().mode;
        assert_eq!(
            engine_camera,
            portal_view::PortalCameraTransitMode::Continuous,
            "the engine no longer defaults to the seamless camera, so this host's \
             `Pop` is a restatement rather than an override and the case-by-case \
             claim needs re-checking"
        );
        assert_eq!(
            engine_cones,
            portal_view::PortalViewConeMode::Dynamic,
            "the engine no longer defaults to viewer-dependent cones, so this \
             host's `Static` proves nothing about disabling it per case"
        );

        // `SmashExperiencePlugin` states the presentation, and every composition
        // installs it. `the_ruleset_states_the_portal_presentation_not_the_binary`
        // asks the composed app. This test only checks the engine defaults above.
    }
}
