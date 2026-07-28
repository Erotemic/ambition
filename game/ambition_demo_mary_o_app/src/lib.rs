//! The Super Mary-O demo's shell, as a function — so the binary and the exit-3
//! regression test assemble the SAME app.
//!
//! See `main.rs` for the doctrine this file encodes.

use bevy::prelude::*;

/// Assemble the demo under a standalone shell host: foundation + engine + host +
/// the Mary-O experience. **Zero engine edits, zero `ambition_app`.**
///
/// The shell owns entry: `initial_route = mary_o_gameplay` (direct standalone
/// entry) and `home_route = mary_o_launcher`, so `QuitToHome` returns to a
/// Mary-O-only launcher and a relaunch rebuilds a fresh, scope-clean session. The
/// SAME [`MaryOExperiencePlugin`](ambition_demo_mary_o::MaryOExperiencePlugin) powers
/// direct entry and launcher relaunch.
pub fn build_demo_app() -> App {
    build_demo_app_with_home(ambition_demo_mary_o::MARY_O_LAUNCHER_ROUTE)
}

/// The same standalone host with an explicitly named home route — exposed so a
/// lifecycle test can build a SECOND host from the identical provider and prove
/// `QuitToHome` resolves relative to whichever home this host declared.
pub fn build_demo_app_with_home(home_route: &str) -> App {
    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    compose_mary_o_shell(&mut app, home_route);
    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

/// Compose the Mary-O experience under a thin standalone host: session-scope +
/// minimal shell + the reusable provider + a launcher home. The provider is
/// host-independent — only these host lines are host-specific.
fn compose_mary_o_shell(app: &mut App, home_route: &str) {
    use ambition_demo_mary_o::{MARY_O_GAMEPLAY_ROUTE, MaryOExperiencePlugin};

    // The shell, the load coordinator, the loading presentation, the frontend
    // audio context and the two route registrations — the seven steps every
    // host of a platformer provider performs in the same order, none of which
    // is a decision this demo makes. Mary-O authors no frontend sound, so the
    // default profile keeps the launcher deliberately silent rather than
    // inheriting another provider's cached audio.
    ambition::provider::ShellComposition::new(
        ambition_demo_mary_o::MARY_O_EXPERIENCE,
        home_route,
        MARY_O_GAMEPLAY_ROUTE,
    )
    .install(app, MaryOExperiencePlugin);

    // The shell-gated simulation stays dormant until the provider publishes
    // its exact SessionRoot during activation. No process-resident bootstrap
    // world is installed here; loading and launcher frames have zero world
    // authority by construction.
}

/// The same demo, DRAWN — foundation swapped for `DefaultPlugins`, plus the
/// engine's generic presentation face (oracle-violation OV1).
///
/// The only difference from [`build_demo_app`] is the first call and one added
/// plugin. That is the claim the demos doctrine makes about a `<name>_app` shell,
/// and it is now true rather than aspirational.
///
/// `render` decides whether a rasterizer is created. `RenderMode::Headless` builds
/// the full render graph against **no wgpu backend** and opens no window — the
/// standard Bevy recipe for exercising presentation in CI. The entities, the
/// camera, and the schedule are identical either way, which is what makes
/// `tests/ov1_draws_the_world.rs` meaningful without a GPU.
#[cfg(feature = "visible")]
pub fn build_windowed_demo_app(render: RenderMode) -> App {
    use bevy::render::RenderPlugin;
    use bevy::render::settings::{RenderCreation, WgpuSettings};
    use bevy::window::{ExitCondition, WindowPlugin};

    let mut app = App::new();
    let plugins = DefaultPlugins
        // Point the AssetServer file root at the engine's on-disk asset tree
        // (`crates/ambition_actors/assets`, where the generated sprite sheets
        // live), exactly as the hosted app does — via the SHARED umbrella helper,
        // so the two apps cannot diverge. Without this the default cwd-relative
        // `"assets"` root has no `sprites/` tree and every character renders as a
        // bare box. Set on the builder BEFORE `add_plugins`, since `AssetPlugin`
        // reads its `file_path` when it builds and a later host plugin is too
        // late to change it.
        .set(bevy::asset::AssetPlugin {
            file_path: ambition::asset_manager::actors_desktop_asset_root(),
            ..default()
        })
        .set(WindowPlugin {
            primary_window: match render {
                RenderMode::Windowed => Some(Window {
                    title: "Super Mary-O — 1-1".into(),
                    ..default()
                }),
                RenderMode::Headless => None,
            },
            exit_condition: match render {
                RenderMode::Windowed => ExitCondition::OnAllClosed,
                RenderMode::Headless => ExitCondition::DontExit,
            },
            close_when_requested: matches!(render, RenderMode::Windowed),
            ..default()
        });
    match render {
        RenderMode::Windowed => app.add_plugins(plugins),
        RenderMode::Headless => app.add_plugins(
            plugins
                // These tests construct several Apps in one process. Logging
                // and Ctrl+C handlers are process-global and belong to a real
                // executable host, not a manually stepped no-window fixture.
                .disable::<bevy::log::LogPlugin>()
                .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
                // A `backends: None` renderer has no RenderApp. Do not install
                // extract/render-only plugins that would report that expected
                // absence as an error or warning.
                .disable::<bevy::core_pipeline::CorePipelinePlugin>()
                .disable::<bevy::gizmos_render::GizmoRenderPlugin>()
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends: None,
                        ..default()
                    }),
                    ..default()
                })
                .disable::<bevy::winit::WinitPlugin>(),
        ),
    };
    ambition::engine::init_engine_states(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    // Visible and headless hosts share one provider/shell/session lifecycle.
    // The provider installs Mary-O's content definitions before the shared asset
    // catalog is assembled below.
    compose_mary_o_shell(&mut app, ambition_demo_mary_o::MARY_O_LAUNCHER_ROUTE);
    // The UMBRELLA install, not a copy of it (2026-07-27). This shell used to
    // hand-roll ~90 lines here, and so did Sanic's, and the external-consumer
    // fixture recorded that a third party got neither — so it drew the world as
    // coloured rectangles. A helper the in-repo demos do not use is a helper
    // nothing exercises; these two ARE the regression test for it.
    //
    // Level 1-1 is authored in code rather than LDtk, so no world manifest: a
    // world-less catalog contributes no world rows and every other entry lands.
    app.add_plugins(
        ambition::game_assets::PlatformerAssetsPlugin::for_experience(
            ambition_demo_mary_o::MARY_O_EXPERIENCE,
        )
        // Startup asset binding precedes gameplay activation, so the theme comes
        // from the authored room rather than a session root that does not exist
        // yet.
        .with_room(ambition_demo_mary_o::mary_o_session_world().metadata.0),
    );

    // OV1, closed: a camera, the room's static visuals, and the sprite/animation
    // chain. The minimal launcher/loading presentation is composed by the host.
    // Without the asset resources installed above this plugin has an empty
    // `GameAssets` to draw from and every actor and block renders as a colored
    // rectangle — the exact divergence that made this demo assetless standalone
    // while it rendered fine inside the hosted app.
    app.add_plugins(ambition::presentation::PlatformerPresentationPlugin);
    // The engine's opt-in F1 debug visualizations (collision blocks, surface
    // chains + normals, read-model body/feature boxes). Shapes only — no dev
    // HUD. Starts OFF; press F1 in-game.
    app.add_plugins(ambition::render::rendering::debug_viz::DebugVizPlugin::default());

    // The windowed host uses the physical Kira backend. Mary-O's provider authors
    // a run+jump SFX voice and the "Support Theme" music cue; this wires the same
    // shared audio face the hosted app uses so both are audible standalone.
    install_mary_o_audio(&mut app);
    app
}

/// Install the engine-owned audio runtime plus Mary-O's resident catalog cache.
/// The same selection, intent, director, SFX, and frontend-reset path is used by
/// the multi-game host; only provider-authored resources differ here.
#[cfg(feature = "visible")]
fn install_mary_o_audio(app: &mut App) {
    use bevy::prelude::IntoScheduleConfigs as _;

    // `SfxBankAssetPath` is published by `PlatformerAssetsPlugin`, which
    // resolves it from the same catalog it builds — one resolution rather than
    // an app-local repeat that could name a different path.

    // Use the same engine-owned audio runtime as the multi-game host. The
    // standalone app contributes only its provider catalogs and resident asset
    // library; selection, intent priority, playback state, channels, and the
    // director are installed once by `SandboxAudioPlugin`.
    app.add_plugins(ambition::actors::audio::SandboxAudioPlugin)
        .add_systems(
            Startup,
            setup_mary_o_audio_library.in_set(ambition::actors::schedule::PresentationSetupSet),
        );
}

#[cfg(feature = "visible")]
fn setup_mary_o_audio_library(
    mut commands: Commands,
    catalogs: Res<ambition::audio::catalog::AudioCatalogRegistry>,
    mut audio_sources: ResMut<Assets<bevy_kira_audio::prelude::AudioSource>>,
) {
    let music = catalogs
        .music_for(ambition_demo_mary_o::MARY_O_EXPERIENCE)
        .expect("Mary-O provider registered its App-local music catalog");
    let sfx = catalogs
        .sfx_for(ambition_demo_mary_o::MARY_O_EXPERIENCE)
        .expect("Mary-O provider registered its App-local SFX catalog");
    let (library, music_state) = ambition::audio::library::AudioLibrary::new_with_playback_state(
        &mut audio_sources,
        sfx,
        music,
        None,
        None,
        None,
    );
    commands.insert_resource(library);
    commands.insert_resource(music_state);
}

/// Whether [`build_windowed_demo_app`] opens a window and creates a GPU device.
#[cfg(feature = "visible")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    /// A real window and a real backend. What `cargo run --features visible` wants.
    Windowed,
    /// The render graph, no backend, no window. What CI wants.
    Headless,
}
