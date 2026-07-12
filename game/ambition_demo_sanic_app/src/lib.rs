//! The Sanic demo's shell, as a function — so the binary and the exit-3
//! regression test assemble the SAME app.
//!
//! See `main.rs` for the doctrine this file encodes.

use bevy::prelude::*;

#[cfg(feature = "visible")]
use ambition_demo_sanic::SANIC_MUSIC_ASSET_PATH;
use ambition_demo_sanic::{SanicDemoContentPlugin, SanicRulesPlugin};

/// Assemble the demo: foundation + the engine group + the host group + this
/// demo's content and rules. **Zero engine edits, zero `ambition_app`.**
///
/// Headless-foundation here; a windowed shell swaps that one call for
/// `DefaultPlugins` + `ambition::engine::init_engine_states`. Everything below it
/// is identical, which is the claim exit 3 makes.
pub fn build_demo_app() -> App {
    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    app.add_plugins((SanicDemoContentPlugin, SanicRulesPlugin::global()));
    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

/// The same demo, DRAWN — foundation swapped for `DefaultPlugins`, plus the
/// engine's generic presentation face (oracle-violation OV1).
///
/// The simulation/content composition is identical to [`build_demo_app`]. The
/// visible shell swaps the foundation, adds the generic presentation face, and
/// starts this demo's authored soundtrack when it owns a real window.
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
    let asset_root = desktop_asset_root();
    eprintln!("sanic_demo: asset root = {asset_root}");
    let plugins = DefaultPlugins
        .set(bevy::asset::AssetPlugin {
            file_path: asset_root,
            ..default()
        })
        .set(WindowPlugin {
            primary_window: match render {
                RenderMode::Windowed => Some(Window {
                    title: "Sanic — momentum demo".into(),
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
    if matches!(render, RenderMode::Windowed) {
        // Keep headless render tests independent of the host audio device.
        app.add_plugins(bevy_kira_audio::prelude::AudioPlugin);
        app.add_systems(Startup, start_sanic_music);
    }
    ambition::engine::init_engine_states(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    // OV1, closed: a camera, the room's static visuals, and the sprite/animation
    // chain. No HUD, no menus, no dev stack — those are the GAME's.
    app.add_plugins(ambition::presentation::PlatformerPresentationPlugin);
    app.add_plugins((SanicDemoContentPlugin, SanicRulesPlugin::global()));
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

#[cfg(all(feature = "visible", not(target_arch = "wasm32")))]
fn desktop_asset_root() -> String {
    if std::env::var_os("BEVY_ASSET_ROOT").is_some() {
        return "assets".to_string();
    }
    let shared_assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/ambition_actors/assets");
    match shared_assets.canonicalize() {
        Ok(path) if path.is_dir() => path.to_string_lossy().into_owned(),
        _ => "assets".to_string(),
    }
}

#[cfg(all(feature = "visible", target_arch = "wasm32"))]
fn desktop_asset_root() -> String {
    "assets".to_string()
}

#[cfg(feature = "visible")]
fn start_sanic_music(asset_server: Res<AssetServer>, audio: Res<bevy_kira_audio::prelude::Audio>) {
    use bevy_kira_audio::prelude::AudioControl;

    audio
        .play(asset_server.load(SANIC_MUSIC_ASSET_PATH))
        .looped();
}

#[cfg(all(test, feature = "visible", not(target_arch = "wasm32")))]
mod tests {
    #[test]
    fn development_asset_root_contains_the_shared_shader_tree() {
        if std::env::var_os("BEVY_ASSET_ROOT").is_some() {
            return;
        }
        let root = std::path::PathBuf::from(super::desktop_asset_root());
        assert!(
            root.join("shaders/hit_flash.wgsl").is_file(),
            "Sanic's visible shell must resolve the shared Ambition asset tree; got {}",
            root.display()
        );
    }
}
