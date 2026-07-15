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
    use ambition::game_shell::{
        ShellHostConfiguration, ShellHostSpec, ShellLaunchCatalog, ShellRouteCatalog,
        ShellRouteSpec,
    };
    use ambition_demo_mary_o::{MaryOExperiencePlugin, MARY_O_GAMEPLAY_ROUTE};

    app.add_plugins(ambition::game_shell::MinimalShellPlugins);
    // The standalone launcher is an explicit frontend audio context. Mary-O
    // authors an empty fragment, so the launcher and gameplay are deliberately
    // silent rather than inheriting another provider's cached sounds.
    app.insert_resource(ambition::audio::selection::FrontendAudioProfile::new(
        ambition_demo_mary_o::MARY_O_EXPERIENCE,
    ));
    app.add_plugins(ambition::load::AmbitionLoadPlugin);
    app.add_plugins(ambition::load_presentation::MinimalLoadPresentationPlugins);
    app.add_plugins(MaryOExperiencePlugin);

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
    let sfx_bank_path = install_mary_o_asset_resources(&mut app);

    // OV1, closed: a camera, the room's static visuals, and the sprite/animation
    // chain. The minimal launcher/loading presentation is composed by the host.
    // Without the asset resources installed above this plugin has an empty
    // `GameAssets` to draw from and every actor and block renders as a colored
    // rectangle — the exact divergence that made this demo assetless standalone
    // while it rendered fine inside the hosted app.
    app.add_plugins(ambition::presentation::PlatformerPresentationPlugin);

    // The windowed host uses the physical Kira backend. Mary-O's provider authors
    // a run+jump SFX voice and the "Support Theme" music cue; this wires the same
    // shared audio face the hosted app uses so both are audible standalone.
    install_mary_o_audio(&mut app, sfx_bank_path);
    app.add_systems(Update, ambition::audio::music::drive_selected_session_music);
    app
}

/// Build and insert the shared asset resources the generic presentation plugin
/// reads — the single `SandboxAssetCatalog` and the `GameAssets` it fills. This
/// is the standalone equivalent of what `ambition_app` does for the hosted demo,
/// and the reason the two paths cannot silently diverge again: a demo that draws
/// nothing standalone was exactly this install being absent. Returns the resolved
/// SFX bank path so the audio face can bind it. Mirrors the Sanic demo shell.
#[cfg(feature = "visible")]
fn install_mary_o_asset_resources(app: &mut App) -> Option<String> {
    use bevy::prelude::IntoScheduleConfigs as _;

    let config = ambition::sprite_sheet::game_assets::GameAssetConfig::from_args();
    // Level 1-1 is authored in code, not LDtk-backed, so build the ordinary shared
    // catalog without world-file rows rather than installing a process-global world
    // manifest just to reach sprites and block art.
    let music = app
        .world()
        .resource::<ambition::audio::catalog::AudioCatalogRegistry>()
        .music_for(ambition_demo_mary_o::MARY_O_EXPERIENCE)
        .expect("Mary-O provider registered its App-local music catalog")
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
        load_mary_o_game_assets.before(ambition::presentation::PlatformerPresentationSetupSet),
    );
    sfx_bank_path
}

/// Fill the shared `GameAssets` the generic presentation plugin consumes — the one
/// path for Mary-O's character sheets (small and tall) and the level's block art.
#[cfg(feature = "visible")]
fn load_mary_o_game_assets(
    config: Res<ambition::sprite_sheet::game_assets::GameAssetConfig>,
    character_catalog: Res<ambition::characters::actor::character_catalog::CharacterCatalog>,
    boss_catalog: Res<ambition::actors::boss_encounter::BossCatalog>,
    catalog: Res<ambition::asset_manager::sandbox_assets::SandboxAssetCatalog>,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    quality: Option<Res<ambition::render::quality::ResolvedVisualQuality>>,
    mut game_assets: ResMut<ambition::sprite_sheet::game_assets::GameAssets>,
) {
    // Startup asset binding precedes gameplay activation, so derive the theme from
    // Mary-O's immutable authored room rather than a not-yet-published session root.
    let authored_room = ambition_demo_mary_o::mary_o_session_world().metadata;
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

    for (character_id, sheet_stem) in [
        (
            ambition_demo_mary_o::MARY_O_CHARACTER_ID,
            "super_mary_o_spritesheet",
        ),
        ("mary_o_tall", "super_mary_o_tall_spritesheet"),
    ] {
        if game_assets
            .characters
            .asset_for_character_id(character_id)
            .is_some()
        {
            info!("mary_o_demo: bound sprites/{sheet_stem}.png through the shared character asset path");
        } else {
            warn!(
                "mary_o_demo: no {character_id} sheet was bound; expected assets/sprites/\
                 {sheet_stem}.png and {sheet_stem}.ron. The marked player fallback remains \
                 visible. Rebuild after publishing the generated manifest (regen_sprites.sh)."
            );
        }
    }
}

/// Minimal audio face for the demo: the packed SFX bank, one SFX channel, and the
/// standard `SfxMessage` consumer, plus the library the music driver plays from.
/// Mirrors the Sanic demo shell so both standalone demos are audible.
#[cfg(feature = "visible")]
fn install_mary_o_audio(app: &mut App, sfx_bank_path: Option<String>) {
    use ambition::audio::AmbitionAudioAppExt as _;
    use bevy::prelude::IntoScheduleConfigs as _;

    app.init_resource::<ambition::audio::AudioOutputMode>()
        .add_plugins(ambition::audio::AmbitionAudioBackendPlugin);
    if let Some(path) = sfx_bank_path {
        info!("mary_o_demo: SFX bank path = {path}");
        app.insert_resource(ambition::audio::SfxBankAssetPath::new(
            ambition_demo_mary_o::MARY_O_EXPERIENCE,
            path,
        ));
    } else {
        warn!("mary_o_demo: no SFX bank path resolved; jump cues will be silent stubs");
    }
    app.add_plugins(ambition::audio::SfxBankAssetPlugin)
        .init_resource::<ambition::audio::render::ProviderSfxHandleCache>()
        .add_ambition_audio_channel::<ambition::audio::library::SfxChannel>()
        .add_systems(Startup, setup_mary_o_audio_library)
        .add_systems(
            Update,
            ambition::audio::audio_play_sfx_messages
                .after(ambition::platformer::schedule::SandboxSet::CoreSimulation),
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
