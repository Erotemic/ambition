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
    // TODO(sanic-demo-trail-toggle): `PlatformerHostPlugins` currently carries
    // the sandbox's B-key trail debug affordance. Move that behind an explicit
    // host/dev capability later; it is inherited here, not a Sanic ability.
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
                // Presentation tests construct several Apps in one process. Bevy's
                // logger/tracing subscriber is process-global, and the no-backend
                // render recipe intentionally has no RenderApp to receive extract
                // diagnostics. Disable logging in this test-only shell instead of
                // reporting expected global-subscriber/extract errors as failures.
                .disable::<bevy::log::LogPlugin>()
                // `backends: None` deliberately omits the RenderApp. Disable the
                // core-pipeline extractor as well so it does not report the
                // expected missing sub-app once per headless test App.
                .disable::<bevy::core_pipeline::CorePipelinePlugin>()
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
    // TODO(sanic-demo-trail-toggle): `PlatformerHostPlugins` currently carries
    // the sandbox's B-key trail debug affordance. Move that behind an explicit
    // host/dev capability later; it is inherited here, not a Sanic ability.

    // Content must install its character catalog before the shared asset catalog
    // is built: that catalog discovers Sanic's sheet through the same public
    // character-loader path the full Ambition game uses.
    app.add_plugins((SanicDemoContentPlugin, SanicRulesPlugin::global()));
    let sfx_bank_path = install_sanic_asset_resources(&mut app);

    // OV1, closed: a camera, the room's static visuals, and the sprite/animation
    // chain. No HUD, no menus, no dev stack — those are the GAME's.
    app.insert_resource(ClearColor(Color::srgb(0.025, 0.045, 0.09)));
    app.add_plugins(ambition::presentation::PlatformerPresentationPlugin);

    if matches!(render, RenderMode::Windowed) {
        // Keep headless render tests independent of the host audio device. The
        // demo still uses Ambition's standard SfxMessage -> packed-bank bridge.
        install_sanic_audio(&mut app, sfx_bank_path);
        app.add_systems(Startup, start_sanic_music);
    }
    app
}

#[cfg(feature = "visible")]
fn install_sanic_asset_resources(app: &mut App) -> Option<String> {
    use bevy::prelude::IntoScheduleConfigs as _;

    let config = ambition::sprite_sheet::game_assets::GameAssetConfig::from_args();
    // The Sanic course is procedural, not LDtk-backed. Build the ordinary
    // shared asset catalog without world-file rows instead of installing a fake
    // process-global world manifest just to reach sprites and parallax art.
    let catalog = ambition::actors::assets::sandbox_assets::build_sandbox_catalog_without_worlds(
        &config,
        ambition::actors::session::data::authored_music_registry(),
    );
    let sfx_bank_path = catalog.path_for(&ambition::asset_manager::sandbox_assets::ids::sfx_bank());

    app.insert_resource(config);
    app.insert_resource(catalog);
    app.init_resource::<ambition::sprite_sheet::game_assets::GameAssets>();
    app.add_systems(
        Startup,
        load_sanic_game_assets.before(ambition::presentation::PlatformerPresentationSetupSet),
    );
    sfx_bank_path
}

/// Load the same shared `GameAssets` resource consumed by Ambition's generic
/// presentation plugin. This is the single path for Sanic art, block/entity art,
/// and the room's skybridge parallax stack.
#[cfg(feature = "visible")]
fn load_sanic_game_assets(
    config: Res<ambition::sprite_sheet::game_assets::GameAssetConfig>,
    catalog: Res<ambition::asset_manager::sandbox_assets::SandboxAssetCatalog>,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    active_room: Res<ambition::world::rooms::ActiveRoomMetadata>,
    quality: Option<Res<ambition::render::quality::ResolvedVisualQuality>>,
    mut game_assets: ResMut<ambition::sprite_sheet::game_assets::GameAssets>,
) {
    *game_assets = ambition::actors::assets::game_assets::load_game_assets(
        &config,
        &catalog,
        &asset_server,
        &mut layouts,
        &active_room.0,
        quality.as_deref().map(|q| &q.budget),
    );

    for (character_id, sheet_stem) in [
        (ambition_demo_sanic::SANIC_CHARACTER_ID, "sanic_spritesheet"),
        (
            ambition_demo_sanic::SUPER_SANIC_CHARACTER_ID,
            "super_sanic_spritesheet",
        ),
    ] {
        if game_assets
            .characters
            .asset_for_character_id(character_id)
            .is_some()
        {
            info!(
                "sanic_demo: bound sprites/{sheet_stem}.png through the shared character asset path"
            );
        } else {
            warn!(
                "sanic_demo: no {character_id} sheet was bound; expected assets/sprites/\
                 {sheet_stem}.png and {sheet_stem}.ron. The marked player fallback \
                 remains visible. Rebuild after publishing the generated manifest."
            );
        }
    }
    info!(
        "sanic_demo: loaded {} parallax layer handle(s) for the active room",
        game_assets.parallax_layers.len()
    );
}

/// Minimal audio face for the demo: the regular packed SFX bank, one SFX
/// channel, and the standard `SfxMessage` consumer. Music remains the demo's
/// direct authored loop; this avoids importing the full Ambition app director.
#[cfg(feature = "visible")]
fn install_sanic_audio(app: &mut App, sfx_bank_path: Option<String>) {
    use bevy::prelude::IntoScheduleConfigs as _;
    use bevy_kira_audio::prelude::AudioApp as _;

    app.add_plugins(bevy_kira_audio::prelude::AudioPlugin);
    if let Some(path) = sfx_bank_path {
        info!("sanic_demo: SFX bank path = {path}");
        app.insert_resource(ambition::audio::SfxBankAssetPath(path));
    } else {
        warn!("sanic_demo: no SFX bank path resolved; milestone cues will be silent stubs");
    }
    app.add_plugins(ambition::audio::SfxBankAssetPlugin)
        .init_resource::<ambition::audio::render::SfxBankHandleCache>()
        .add_audio_channel::<ambition::audio::library::SfxChannel>()
        .add_systems(Startup, setup_sanic_audio_library)
        .add_systems(
            Update,
            ambition::audio::audio_play_sfx_messages
                .after(ambition::platformer::schedule::SandboxSet::CoreSimulation),
        );
}

#[cfg(feature = "visible")]
fn setup_sanic_audio_library(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<bevy_kira_audio::prelude::AudioSource>>,
) {
    let library = ambition::audio::library::AudioLibrary::new(
        &mut audio_sources,
        ambition::actors::session::data::authored_sfx_registry(),
        ambition::actors::session::data::authored_music_registry(),
        None,
        None,
        None,
    );
    commands.insert_resource(library);
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

    #[test]
    fn published_local_sanic_forms_bind_through_game_assets() {
        let root = std::path::PathBuf::from(super::desktop_asset_root());
        let forms = [
            (ambition_demo_sanic::SANIC_CHARACTER_ID, "sanic_spritesheet"),
            (
                ambition_demo_sanic::SUPER_SANIC_CHARACTER_ID,
                "super_sanic_spritesheet",
            ),
        ];
        if !forms.iter().all(|(_, stem)| {
            root.join(format!("sprites/{stem}.png")).is_file()
                && root.join(format!("sprites/{stem}.ron")).is_file()
        }) {
            return;
        }

        let mut app = super::build_windowed_demo_app(super::RenderMode::Headless);
        app.update();
        let assets = app
            .world()
            .resource::<ambition::sprite_sheet::game_assets::GameAssets>();
        for (character_id, _) in forms {
            assert!(
                assets
                    .characters
                    .asset_for_character_id(character_id)
                    .is_some(),
                "published {character_id} PNG+RON must bind through the shared GameAssets path"
            );
        }
    }
}
