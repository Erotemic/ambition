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
/// SAME [`Smb1ExperiencePlugin`](ambition_demo_smb1::Smb1ExperiencePlugin) powers
/// direct entry and launcher relaunch.
pub fn build_demo_app() -> App {
    build_demo_app_with_home(ambition_demo_smb1::MARY_O_LAUNCHER_ROUTE)
}

/// The same standalone host with an explicitly named home route — exposed so a
/// lifecycle test can build a SECOND host from the identical provider and prove
/// `QuitToHome` resolves relative to whichever home this host declared.
pub fn build_demo_app_with_home(home_route: &str) -> App {
    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    compose_smb1_shell(&mut app, home_route);
    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

/// Compose the Mary-O experience under a thin standalone host: session-scope +
/// minimal shell + the reusable provider + a launcher home. The provider is
/// host-independent — only these host lines are host-specific.
fn compose_smb1_shell(app: &mut App, home_route: &str) {
    use ambition::game_shell::{
        ShellHostConfiguration, ShellHostSpec, ShellLaunchCatalog, ShellRouteCatalog,
        ShellRouteSpec,
    };
    use ambition_demo_smb1::{Smb1ExperiencePlugin, MARY_O_GAMEPLAY_ROUTE};

    app.add_plugins(ambition::game_shell::MinimalShellPlugins);
    // The standalone launcher is an explicit frontend audio context. Mary-O
    // authors an empty fragment, so the launcher and gameplay are deliberately
    // silent rather than inheriting another provider's cached sounds.
    app.insert_resource(ambition::audio::selection::FrontendAudioProfile::new(
        ambition_demo_smb1::MARY_O_EXPERIENCE,
    ));
    app.add_plugins(ambition::load::AmbitionLoadPlugin);
    app.add_plugins(ambition::load_presentation::MinimalLoadPresentationPlugins);
    app.add_plugins(Smb1ExperiencePlugin);

    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(ShellRouteSpec::new(
            home_route,
            ShellLaunchCatalog::basic_experience_id(),
        ));
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(MARY_O_GAMEPLAY_ROUTE, home_route));

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
    use bevy::render::settings::{RenderCreation, WgpuSettings};
    use bevy::render::RenderPlugin;
    use bevy::window::{ExitCondition, WindowPlugin};

    let mut app = App::new();
    let plugins = DefaultPlugins.set(WindowPlugin {
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
    compose_smb1_shell(&mut app, ambition_demo_smb1::MARY_O_LAUNCHER_ROUTE);

    // OV1, closed: a camera, the room's static visuals, and the sprite/animation
    // chain. The minimal launcher/loading presentation is composed by the host.
    app.add_plugins(ambition::presentation::PlatformerPresentationPlugin);
    app
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
