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
/// ⛔ **`OffscreenGpu` is NOT "headless"**: headless sets `backends: None`, so
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

#[cfg(feature = "visible")]
pub fn build_windowed_demo_app() -> App {
    build_windowed_demo_app_with(RenderMode::Windowed)
}

#[cfg(feature = "visible")]
pub fn build_windowed_demo_app_with(render: RenderMode) -> App {
    use bevy::window::{ExitCondition, WindowPlugin};

    let mut app = App::new();
    let plugins = DefaultPlugins
        .set(bevy::asset::AssetPlugin {
            file_path: ambition_platformer2d::asset_manager::actors_desktop_asset_root(),
            ..default()
        })
        .set(WindowPlugin {
            primary_window: match render {
                RenderMode::Windowed => Some(Window {
                    title: "TwinTrack — Special Relativity Laboratory".into(),
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                RenderMode::OffscreenGpu => None,
            },
            exit_condition: match render {
                RenderMode::Windowed => ExitCondition::OnAllClosed,
                RenderMode::OffscreenGpu => ExitCondition::DontExit,
            },
            close_when_requested: matches!(render, RenderMode::Windowed),
            ..default()
        });
    match render {
        RenderMode::Windowed => app.add_plugins(plugins),
        // ⛔ **`winit` is also the RUNNER.** Without it Bevy's default runner
        // performs ONE update and returns, so a capture exits 0 having rendered
        // nothing.
        RenderMode::OffscreenGpu => app
            .add_plugins(plugins.disable::<bevy::winit::WinitPlugin>())
            .add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(
                std::time::Duration::from_millis(0),
            )),
    };
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
