//! The Sanic demo's shell, as a function — so the binary and the exit-3
//! regression test assemble the SAME app.
//!
//! See `main.rs` for the doctrine this file encodes.

use bevy::prelude::*;

/// Assemble the demo: foundation + the engine group + the host group + the Sanic
/// experience under a standalone shell host. **Zero engine edits, zero
/// `ambition_app`.**
///
/// The shell owns entry: `initial_route = sanic_gameplay` (direct standalone
/// entry) and `home_route = sanic_launcher`, so a `QuitToHome` returns to a
/// Sanic-only launcher and a relaunch rebuilds a fresh, scope-clean session. The
/// SAME [`SanicExperiencePlugin`] powers direct entry and launcher relaunch.
///
/// Headless-foundation here; a windowed shell swaps that one call for
/// `DefaultPlugins` + `ambition::engine::init_engine_states`.
pub fn build_demo_app() -> App {
    build_demo_app_with_home(ambition_demo_sanic::SANIC_LAUNCHER_ROUTE)
}

/// The same standalone Sanic host, but with an explicitly named home route.
///
/// Exposed so a lifecycle test can build a SECOND host from the identical
/// provider and prove that `QuitToHome` resolves relative to whichever home this
/// host declared — the provider never names either launcher.
pub fn build_demo_app_with_home(home_route: &str) -> App {
    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    // TODO(sanic-demo-trail-toggle): `PlatformerHostPlugins` currently carries
    // the sandbox's B-key trail debug affordance. Move that behind an explicit
    // host/dev capability later; it is inherited here, not a Sanic ability.
    compose_sanic_shell(&mut app, home_route);
    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

/// Compose the Sanic experience under a thin standalone host: the session-scope
/// mechanism, the minimal shell, the reusable Sanic provider, and a launcher
/// home. The provider is host-independent — only these host lines (the two
/// routes, and the host spec) are host-specific.
fn compose_sanic_shell(app: &mut App, home_route: &str) {
    use ambition::game_shell::{
        ShellHostConfiguration, ShellHostSpec, ShellLaunchCatalog, ShellRouteCatalog,
        ShellRouteSpec,
    };
    use ambition_demo_sanic::{SanicExperiencePlugin, SANIC_GAMEPLAY_ROUTE};

    app.add_plugins(ambition::game_shell::MinimalShellPlugins);
    app.insert_resource(
        ambition::audio::selection::FrontendAudioProfile::new(
            ambition_demo_sanic::SANIC_EXPERIENCE,
        )
        .with_sfx([
            ambition::sfx::ids::UI_MENU_MOVE,
            ambition::sfx::ids::UI_MENU_ACCEPT,
            ambition::sfx::ids::UI_MENU_BACK,
        ]),
    );
    app.add_plugins(ambition::load::AmbitionLoadPlugin);
    app.add_plugins(ambition::load_presentation::MinimalLoadPresentationPlugins);
    app.add_plugins(SanicExperiencePlugin);

    // This host's home route: a launcher listing this host's registered
    // experiences (here, just Sanic + the built-in exit).
    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(ShellRouteSpec::new(
            home_route,
            ShellLaunchCatalog::basic_experience_id(),
        ));
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(SANIC_GAMEPLAY_ROUTE, home_route));

    // The shell-gated simulation stays dormant until the provider publishes
    // its exact SessionRoot during activation. No process-resident bootstrap
    // world is installed here; loading and launcher frames have zero world
    // authority by construction.
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
    if matches!(render, RenderMode::Headless) {
        app.insert_resource(ambition::audio::AudioOutputMode::Recording);
    }
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
                .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
                // `backends: None` deliberately omits the RenderApp. Disable
                // extract/render-only plugins so expected test topology stays
                // silent instead of resembling a runtime failure.
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
    // TODO(sanic-demo-trail-toggle): `PlatformerHostPlugins` currently carries
    // the sandbox's B-key trail debug affordance. Move that behind an explicit
    // host/dev capability later; it is inherited here, not a Sanic ability.

    // Visible and headless hosts share one provider/shell/session lifecycle.
    // The provider installs Sanic's content definitions before the shared asset
    // catalog is assembled below.
    compose_sanic_shell(&mut app, ambition_demo_sanic::SANIC_LAUNCHER_ROUTE);
    let sfx_bank_path = install_sanic_asset_resources(&mut app);

    // OV1, closed: a camera, the room's static visuals, and the sprite/animation
    // chain. No HUD, no menus, no dev stack — those are the GAME's.
    app.insert_resource(ClearColor(Color::srgb(0.025, 0.045, 0.09)));
    app.add_plugins(ambition::presentation::PlatformerPresentationPlugin);
    // The engine's opt-in F1 debug visualizations (collision blocks, surface
    // chains + normals, rebound vectors, read-model body/feature boxes).
    // Shapes only — no dev HUD. Starts OFF; press F1 in-game.
    app.add_plugins(ambition::render::rendering::debug_viz::DebugVizPlugin::default());

    // Windowed hosts use the physical Kira backend; headless presentation
    // hosts select the device-free recording backend before this shared audio
    // composition is installed. Both paths therefore exercise the same
    // provider resolver, ownership, bank, and playback-evidence systems.
    install_sanic_audio(&mut app, sfx_bank_path);
    // Session music is authority-driven: the shell bridge selects Sanic's
    // registered music registry on activation and clears it on Quit to Home;
    // this shared driver plays whatever is selected.
    app.add_systems(Update, ambition::audio::music::drive_selected_session_music);
    app
}

#[cfg(feature = "visible")]
fn install_sanic_asset_resources(app: &mut App) -> Option<String> {
    use bevy::prelude::IntoScheduleConfigs as _;

    let config = ambition::sprite_sheet::game_assets::GameAssetConfig::from_args();
    // The Sanic course is procedural, not LDtk-backed. Build the ordinary
    // shared asset catalog without world-file rows instead of installing a fake
    // process-global world manifest just to reach sprites and parallax art.
    let music = app
        .world()
        .resource::<ambition::audio::catalog::AudioCatalogRegistry>()
        .music_for(ambition_demo_sanic::SANIC_EXPERIENCE)
        .expect("Sanic provider registered its App-local music catalog")
        .clone();
    let character_catalog = app
        .world()
        .resource::<ambition::characters::actor::character_catalog::CharacterCatalog>()
        .clone();
    let boss_catalog = app
        .world()
        .resource::<ambition::actors::boss_encounter::BossCatalog>()
        .clone();
    let catalog = ambition::actors::assets::sandbox_assets::build_sandbox_catalog_without_worlds(
        &config,
        &character_catalog,
        &boss_catalog,
        &music,
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
    character_catalog: Res<ambition::characters::actor::character_catalog::CharacterCatalog>,
    boss_catalog: Res<ambition::actors::boss_encounter::BossCatalog>,
    catalog: Res<ambition::asset_manager::sandbox_assets::SandboxAssetCatalog>,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    quality: Option<Res<ambition::render::quality::ResolvedVisualQuality>>,
    mut game_assets: ResMut<ambition::sprite_sheet::game_assets::GameAssets>,
) {
    // Startup asset binding precedes gameplay activation in the shared host, so
    // derive the presentation theme from Sanic's immutable authored world rather
    // than reaching for a not-yet-published live session root. Runtime consumers
    // still read the exact `SessionRoot` components after activation.
    let authored_room = ambition_demo_sanic::sanic_session_world().metadata;
    *game_assets = ambition::actors::assets::game_assets::load_game_assets(
        &config,
        &character_catalog,
        &boss_catalog,
        &catalog,
        &asset_server,
        &mut layouts,
        &authored_room.0,
        quality.as_deref().map(|q| &q.budget),
    );

    // Register the animated ring sheet as a prop the pickup renderer can bind.
    // The sanic lean asset catalog doesn't carry props, so load it straight from
    // its build-embedded manifest target (base resolution — one small demo prop).
    if let Some(asset) = ambition::actors::character_sprites::load_prop_sheet_for_target(
        &asset_server,
        &mut layouts,
        &config.sprite_folder,
        ambition_demo_sanic::RING_SPRITE_KIND,
        &ambition::sprite_sheet::character::SheetTuning::new(1.0, 2),
    ) {
        game_assets
            .characters
            .props
            .insert(ambition_demo_sanic::RING_SPRITE_KIND.to_string(), asset);
        info!("sanic_demo: bound the animated ring prop sheet");
    } else {
        warn!(
            "sanic_demo: no {} manifest embedded; rings draw the static coin",
            ambition_demo_sanic::RING_SPRITE_KIND
        );
    }

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
    use ambition::audio::AmbitionAudioAppExt as _;
    use bevy::prelude::IntoScheduleConfigs as _;

    app.init_resource::<ambition::audio::AudioOutputMode>()
        .add_plugins(ambition::audio::AmbitionAudioBackendPlugin);
    if let Some(path) = sfx_bank_path {
        info!("sanic_demo: SFX bank path = {path}");
        app.insert_resource(ambition::audio::SfxBankAssetPath::new(
            ambition_demo_sanic::SANIC_EXPERIENCE,
            path,
        ));
    } else {
        warn!("sanic_demo: no SFX bank path resolved; milestone cues will be silent stubs");
    }
    app.add_plugins(ambition::audio::SfxBankAssetPlugin)
        .init_resource::<ambition::audio::render::ProviderSfxHandleCache>()
        .add_ambition_audio_channel::<ambition::audio::library::SfxChannel>()
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
    catalogs: Res<ambition::audio::catalog::AudioCatalogRegistry>,
    mut audio_sources: ResMut<Assets<bevy_kira_audio::prelude::AudioSource>>,
) {
    let music = catalogs
        .music_for(ambition_demo_sanic::SANIC_EXPERIENCE)
        .expect("Sanic provider registered its App-local music catalog");
    let sfx = catalogs
        .sfx_for(ambition_demo_sanic::SANIC_EXPERIENCE)
        .expect("Sanic provider registered its App-local SFX catalog");
    let library = ambition::audio::library::AudioLibrary::new(
        &mut audio_sources,
        sfx,
        music,
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
    fn headless_demo_uses_the_device_free_recording_audio_backend() {
        let app = super::build_windowed_demo_app(super::RenderMode::Headless);
        let backend = app.world().resource::<ambition::audio::AudioBackendState>();
        assert_eq!(backend.mode, ambition::audio::AudioOutputMode::Recording);
        assert!(!backend.device_backend_installed);
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
        // This is an asset-publication test, not a shell/load-lifecycle test.
        // Run the real Startup schedule that owns `load_sanic_game_assets`
        // without also advancing Update into provider preparation/activation.
        // Full standalone-host lifecycle coverage lives in the integration
        // tests, while this test stays focused on PNG+RON -> GameAssets binding.
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(bevy::app::Startup);
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
