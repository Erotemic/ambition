//! Thin standalone host for TwinTrack.

use bevy::prelude::*;

pub fn build_demo_app() -> App {
    let mut app = App::new();
    ambition_platformer2d::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    compose_twintrack_shell(&mut app);
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

fn compose_twintrack_shell(app: &mut App) {
    ambition_platformer2d::provider::ShellComposition::new(
        ambition_demo_twintrack::TWINTRACK_EXPERIENCE,
        ambition_demo_twintrack::TWINTRACK_LAUNCHER_ROUTE,
        ambition_demo_twintrack::TWINTRACK_GAMEPLAY_ROUTE,
    )
    .install(app, ambition_demo_twintrack::TwinTrackExperiencePlugin);
}

/// Whether [`build_windowed_demo_app_with`] opens a window.
///
/// `OffscreenGpu` is NOT "headless": headless sets `backends: None`, so
/// there is no RenderApp and no texture to read a picture out of. Sanic's and
/// Mary-O's binaries each learned that separately; this is the same variant.
#[cfg(feature = "visible")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    /// A real window and a real backend. What `-- --window` wants.
    Windowed,
    /// No window, real backend — pixels without a display, for `capture_twintrack`.
    OffscreenGpu,
}
// Gated exactly as `RenderMode` is. TwinTrack has no no-GPU face — its two modes
// are the engine's `Window` and `Offscreen`.
#[cfg(feature = "visible")]
impl From<RenderMode> for ambition_platformer2d::app::Display {
    fn from(mode: RenderMode) -> Self {
        match mode {
            RenderMode::Windowed => Self::Window,
            RenderMode::OffscreenGpu => Self::Offscreen,
        }
    }
}

#[cfg(feature = "visible")]
pub fn build_windowed_demo_app() -> App {
    build_windowed_demo_app_with(RenderMode::Windowed)
}

#[cfg(feature = "visible")]
pub fn build_windowed_demo_app_with(render: RenderMode) -> App {
    let mut app = App::new();
    // ⭐ THE ENGINE'S FACE, NOT A FOURTH COPY OF IT — see D183. The asset root,
    // the window/exit/close matrix and the winit disable were a duplicate of
    // `install_windowed_foundation`. `init_engine_states` is called by the
    // foundation and is no longer called again below.
    //
    // ⚠ The hand-rolled window also set `resolution: (1280, 720)`, which is
    // exactly `WindowResolution::default()` — a restatement of the default, so
    // nothing is lost by letting the foundation build the window.
    //
    // ⛔ The offscreen arm also installed `ScheduleRunnerPlugin`, because
    // disabling `winit` removes the app RUNNER and `run()` would otherwise do ONE
    // update and return. The engine's `Display::Offscreen` is deliberately
    // CALLER-STEPPED, so that runner moved to `bin/capture_twintrack.rs`.
    ambition_platformer2d::app::install_windowed_foundation(
        &mut app,
        "TwinTrack — Special Relativity Festival",
        render.into(),
    );
    ambition_platformer2d::engine::init_engine_states(&mut app);
    app.add_plugins(ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    compose_twintrack_shell(&mut app);
    app.add_plugins(
        ambition_platformer2d::game_assets::PlatformerAssetsPlugin::for_experience(
            ambition_demo_twintrack::TWINTRACK_EXPERIENCE,
        )
        .with_room(ambition_demo_twintrack::twintrack_room().metadata),
    );
    app.insert_resource(ClearColor(Color::srgb(0.015, 0.025, 0.055)));
    app.add_plugins(ambition_platformer2d::presentation::PlatformerPresentationPlugin);
    app.add_plugins(ambition_platformer2d::render::rendering::debug_viz::DebugVizPlugin::default());
    app
}
