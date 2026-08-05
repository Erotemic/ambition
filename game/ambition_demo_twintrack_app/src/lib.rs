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

#[cfg(feature = "visible")]
pub fn build_windowed_demo_app() -> App {
    use bevy::window::{ExitCondition, WindowPlugin};

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(bevy::asset::AssetPlugin {
                file_path: ambition_platformer2d::asset_manager::actors_desktop_asset_root(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "TwinTrack — Special Relativity Laboratory".into(),
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                exit_condition: ExitCondition::OnAllClosed,
                ..default()
            }),
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
    app.add_plugins(
        ambition_platformer2d::render::rendering::debug_viz::DebugVizPlugin::default(),
    );
    app
}
