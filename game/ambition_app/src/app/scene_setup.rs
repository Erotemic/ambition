//! Presentation-side scene construction (composition root).
//!
//! `presentation_world` spawns the cameras, the player sprite, the HUD/quest-panel
//! text, the static room visuals + parallax, and wires the audio library / SFX
//! bank. It is the render+audio composition that pairs with
//! `ambition::actors::session::setup::simulation_world` (which stays sim-only).
//! Moved out of the machinery crate so the sim never imports the render layer —
//! the app, as the composition root, owns the wiring that crosses the seam.
#![allow(clippy::too_many_arguments)]

use bevy::prelude::*;
#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::AudioSource as KiraAudioSource;

use ambition::actors::rooms::RoomSet;
#[cfg(feature = "audio")]
use ambition::actors::session::data::{MusicRegistry, SfxRegistry};
use ambition::actors::world::physics::PhysicsSandboxSettings;
use ambition::actors::world::platforms;
#[cfg(feature = "audio")]
use ambition::asset_manager::sandbox_assets::{ids, SandboxAssetCatalog};
#[cfg(feature = "audio")]
use ambition::audio::library::{AudioLibrary, MusicPlaybackState};
#[cfg(feature = "audio")]
use ambition::audio::SfxBankResource;
use ambition::engine_core::RoomGeometry;
use ambition::render::rendering::{
    spawn_parallax_layers, spawn_room_visuals, HudText, QuestPanelText,
};
use ambition::render::ui_fonts::{UiFontWeight, UiFonts};
#[cfg(feature = "audio")]
use ambition::sfx::BankProvider;
use ambition::sprite_sheet::game_assets::GameAssets;

/// Borrowed inputs for `presentation_world`.
pub struct PresentationSetup<'a> {
    pub world: &'a RoomGeometry,
    pub room_set: &'a RoomSet,
    pub physics_settings: PhysicsSandboxSettings,
    pub game_assets: &'a GameAssets,
    pub quality: Option<&'a ambition::render::quality::ResolvedVisualQuality>,
    #[cfg(feature = "audio")]
    pub music_registry: &'a MusicRegistry,
    #[cfg(feature = "audio")]
    pub sfx_registry: &'a SfxRegistry,
    #[cfg(feature = "audio")]
    pub ui_fonts: Option<&'a UiFonts>,
}

/// Spawn presentation-only entities (Camera2d, sprites, HUD text) and
/// presentation-only resources (`AudioLibrary`). Adds the player's `Sprite`
/// to the entity returned by `simulation_world`.
///
/// Skipped entirely in headless builds. With the `audio` feature off
/// the `KiraAudioSource` asset registry doesn't exist; the audio_sources
/// parameter is gated out and the audio library / music state inserts
/// are skipped.
#[cfg(feature = "audio")]
pub fn presentation_world(
    commands: &mut Commands,
    audio_sources: &mut Assets<KiraAudioSource>,
    asset_server: &AssetServer,
    catalog: &SandboxAssetCatalog,
    params: PresentationSetup<'_>,
) {
    let music_registry = params.music_registry;
    let sfx_registry = params.sfx_registry;
    presentation_world_inner(commands, params);
    install_audio_library(
        commands,
        audio_sources,
        asset_server,
        catalog,
        music_registry,
        sfx_registry,
    );
}

/// Build and insert the host-resident audio library (packed SFX bank +
/// catalog-resolved music assets) and its playback state. An asset CACHE —
/// host-owned, shared across sessions; the per-session audio AUTHORITY is
/// `ambition::audio::selection::ActiveAudioSelection`.
#[cfg(feature = "audio")]
pub fn install_audio_library(
    commands: &mut Commands,
    audio_sources: &mut Assets<KiraAudioSource>,
    asset_server: &AssetServer,
    catalog: &SandboxAssetCatalog,
    music_registry: &MusicRegistry,
    sfx_registry: &SfxRegistry,
) {
    let bank_provider = try_load_sfx_bank_via_catalog(catalog);
    // Resolve music-track ids through the sandbox asset catalog so the
    // library stores catalog-blessed paths (the generic library takes a
    // resolver closure instead of naming the catalog type).
    let resolve_track_path = |id: &str| {
        catalog.path_for(&ambition::asset_manager::sandbox_assets::ids::music_track(
            id,
        ))
    };
    let audio_library = AudioLibrary::new(
        audio_sources,
        sfx_registry,
        music_registry,
        Some(asset_server),
        bank_provider
            .as_ref()
            .map(|provider| provider as &dyn ambition::sfx::SfxProvider),
        Some(&resolve_track_path),
    );
    let music_state = MusicPlaybackState::from_music_registry(music_registry, &audio_library);
    commands.insert_resource(audio_library);
    commands.insert_resource(music_state);
    if let Some(provider) = bank_provider {
        info!("loaded sfx bank: {} entries", provider.entry_count());
        let mut banks = SfxBankResource::default();
        banks
            .register(
                ambition_content::AMBITION_CONTENT_PROVIDER,
                std::sync::Arc::new(provider),
            )
            .expect("initial Ambition SFX bank registration should be unique");
        commands.insert_resource(banks);
    }
}

#[cfg(not(feature = "audio"))]
pub fn presentation_world(commands: &mut Commands, params: PresentationSetup<'_>) {
    presentation_world_inner(commands, params);
}

/// Load a statically packed SFX bank.
///
/// Android APK assets are not normal host filesystem paths, while the current
/// SFX bank loader is synchronous and path/byte based. Until that loader grows
/// an APK-asset backend, `build_for_android.sh` can enable `static_sfx_bank`
/// and pass `AMBITION_STATIC_SFX_BANK_PATH` so the packed bank is available to
/// the same runtime bank provider used on desktop.
#[cfg(all(
    feature = "audio",
    feature = "static_sfx_bank",
    ambition_static_sfx_bank_path
))]
fn try_load_static_sfx_bank() -> Option<BankProvider> {
    let bytes = include_bytes!(env!("AMBITION_STATIC_SFX_BANK_PATH"));
    match BankProvider::from_bytes(bytes.to_vec()) {
        Ok(provider) => {
            info!(
                "loaded statically packed sfx bank: {} entries",
                provider.entry_count()
            );
            Some(provider)
        }
        Err(error) => {
            warn!("statically packed sfx bank failed to parse: {error}");
            None
        }
    }
}

#[cfg(all(
    feature = "audio",
    feature = "static_sfx_bank",
    not(ambition_static_sfx_bank_path)
))]
fn try_load_static_sfx_bank() -> Option<BankProvider> {
    warn!(
        "static_sfx_bank feature enabled without AMBITION_STATIC_SFX_BANK_PATH; \
         falling back to catalog-resolved SFX bank"
    );
    None
}

/// Resolve the SFX bank through the
/// [`ambition::asset_manager::sandbox_assets::SandboxAssetCatalog`] and synchronously
/// load its bytes into a [`BankProvider`]. Fall-through order:
///
/// 1. the statically packed bank (`static_sfx_bank` feature),
/// 2. the catalog's resolved `LocalPath` candidate (preferred —
///    explicit `AMBITION_SFX_BANK_PATH` dev override or platform
///    bundle path),
/// 3. the catalog's `LooseFilesystem` synthesized default located via
///    [`SandboxAssetCatalog::resolve_local_file_path`],
/// 4. `None` + a single info log → the [`AudioLibrary`] uses a short
///    silent stub for any missing cue (procedural fallback retired).
///
/// **All host-filesystem probing for the SFX bank happens through the
/// catalog.** This function owns no candidate-roots walk.
#[cfg(feature = "audio")]
fn try_load_sfx_bank_via_catalog(catalog: &SandboxAssetCatalog) -> Option<BankProvider> {
    #[cfg(feature = "static_sfx_bank")]
    if let Some(provider) = try_load_static_sfx_bank() {
        return Some(provider);
    }

    let id = ids::sfx_bank();
    let resolved = match catalog.resolve(&id) {
        Ok(r) => r,
        Err(error) => {
            warn!("sfx bank catalog resolve failed: {error}");
            return None;
        }
    };

    // 1. Explicit LocalPath candidate (the AMBITION_SFX_BANK_PATH env
    //    override, when set). Use directly without re-probing roots.
    if let Some(local) = resolved.location.as_local_path() {
        return load_bank_from_path(local);
    }

    // 2. Synthesized BevyPath (or any other Bevy-pathable location)
    //    located via the catalog's centralized desktop candidate-roots
    //    walker. `resolve_local_file_path` returns None for
    //    non-desktop profiles or when the file isn't present.
    if let Some(rel_path) = resolved.bevy_asset_path() {
        if let Some(local) = catalog.resolve_local_file_path(&rel_path) {
            return load_bank_from_path(&local);
        }
    }

    info!(
        "no sfx bank found for {} profile (resolved {:?}); SFX will play short silent stubs",
        catalog.profile().label(),
        resolved.location,
    );
    None
}

#[cfg(feature = "audio")]
fn load_bank_from_path(path: &std::path::Path) -> Option<BankProvider> {
    match BankProvider::from_path(path) {
        Ok(provider) => {
            debug!("sfx bank loaded from {}", path.display());
            Some(provider)
        }
        Err(error) => {
            warn!(
                "sfx bank at {} failed to parse: {error}; SFX will play short silent stubs",
                path.display()
            );
            None
        }
    }
}

fn presentation_world_inner(commands: &mut Commands, params: PresentationSetup<'_>) {
    #[cfg(feature = "audio")]
    let ui_fonts = params.ui_fonts;
    #[cfg(not(feature = "audio"))]
    let ui_fonts: Option<&UiFonts> = None;
    host_presentation_scaffold(commands);
    session_presentation(
        commands,
        ambition::platformer::lifecycle::SessionSpawnScope::UNSCOPED,
        SessionPresentationSetup {
            world: params.world,
            room_set: params.room_set,
            physics_settings: params.physics_settings,
            game_assets: params.game_assets,
            quality: params.quality,
            ui_fonts,
        },
    );
}

/// Borrowed inputs for the per-session half of the presentation scene.
pub struct SessionPresentationSetup<'a> {
    pub world: &'a RoomGeometry,
    pub room_set: &'a RoomSet,
    pub physics_settings: PhysicsSandboxSettings,
    pub game_assets: &'a GameAssets,
    pub quality: Option<&'a ambition::render::quality::ResolvedVisualQuality>,
    pub ui_fonts: Option<&'a UiFonts>,
}

/// HOST-resident presentation scaffolding: the main + front-HUD cameras. Spawned
/// once at startup and never owned by a gameplay session — the launcher/title
/// route renders through the same cameras a session does.
pub fn host_presentation_scaffold(commands: &mut Commands) {
    // The MAIN camera (order 0) renders the gameplay world (sprites on layer 0),
    // portal-window meshes, and the main-camera-only parallax layer. It NO LONGER
    // carries `IsDefaultUiCamera`: the default UI camera is now the dedicated
    // FRONT camera below (order 9), so all bevy_ui draws IN FRONT of the order-8
    // cube-menu `Camera3d`. The cube's dim-scrim is the one exception and is
    // explicitly retargeted back to this camera (see
    // `lunex_kaleidoscope_app::spawn_kaleidoscope_scrim`) so it stays BEHIND the cube.
    let mut main_camera_layers = bevy::camera::visibility::RenderLayers::layer(0)
        .with(ambition::platformer::camera_layers::PARALLAX_BACKGROUND_LAYER);
    #[cfg(feature = "portal_render")]
    {
        main_camera_layers =
            main_camera_layers.with(ambition::portal_presentation::PORTAL_WINDOW_RENDER_LAYER);
    }
    let main_camera = commands
        .spawn((
            Camera2d,
            ambition::platformer::camera_layers::MainCamera,
            ambition::game_shell::FrontendOwnedEntity::host(
                ambition::game_shell::FrontendPresentationKind::HostCamera,
            ),
            main_camera_layers,
            ambition::render::screen_effects::ScreenEffectSettings::default(),
            Name::new("Main Camera"),
        ))
        .id();

    // The FRONT HUD/UI camera (order 9): clears nothing, sits IN FRONT of the cube
    // (order 8), and is the default UI camera so the HUD / FPS / debug / control
    // overlays render on top of the cube during the pause menu (and normally
    // otherwise). It is pinned to a DEDICATED RenderLayers that the gameplay sprites
    // (layer 0) are NOT on, so it never re-draws the world over the cube — bevy_ui's
    // node→camera resolution is by `IsDefaultUiCamera`/`UiTargetCamera`, independent
    // of sprite RenderLayers, so UI still renders here.
    commands.spawn((
        Camera2d,
        Camera {
            order: 9,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        ambition::platformer::camera_layers::FrontHudCamera,
        ambition::game_shell::FrontendOwnedEntity::host(
            ambition::game_shell::FrontendPresentationKind::FrontendUiCamera,
        ),
        IsDefaultUiCamera,
        bevy::camera::visibility::RenderLayers::layer(
            ambition::platformer::camera_layers::FRONT_HUD_LAYER,
        ),
        Name::new("Front HUD Camera"),
    ));

    commands.insert_resource(ambition::platformer::camera_layers::MainCameraEntity(
        main_camera,
    ));
}

/// SESSION-owned presentation: parallax, static room visuals, moving
/// platforms, and the marker-tagged HUD/quest text widgets.
/// The direct-entry path calls it `UNSCOPED` at startup (process-resident,
/// the pre-shell behavior); the shell host calls it with the activation's
/// captured scope so the generic session sweep retires all of it.
pub fn session_presentation(
    commands: &mut Commands,
    scope: ambition::platformer::lifecycle::SessionSpawnScope,
    params: SessionPresentationSetup<'_>,
) {
    let world = params.world;
    let room_set = params.room_set;
    let physics_settings = params.physics_settings;
    let game_assets = params.game_assets;
    let quality = params.quality;

    // `Instant::now()` is unsupported under `wasm32-unknown-unknown`
    // (panics with "time not implemented on this platform"). Gate the
    // per-step wall-clock breakdown on non-wasm; the wasm build
    // measures via browser devtools.
    #[cfg(not(target_arch = "wasm32"))]
    let t_room = std::time::Instant::now();
    spawn_parallax_layers(
        commands,
        scope,
        &world.0,
        &room_set.active_spec().metadata,
        Some(game_assets),
        quality.map(|q| &q.budget.parallax),
    );
    spawn_room_visuals(
        commands,
        scope,
        room_set.active_spec(),
        physics_settings,
        Some(game_assets),
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        let t_room_ms = t_room.elapsed().as_secs_f32() * 1000.0;
        eprintln!(
            "[startup]   presentation_world breakdown: spawn_room_visuals={t_room_ms:.1}ms (active room only)"
        );
    }
    session_gameplay_dressing(
        commands,
        scope,
        SessionDressingSetup {
            world,
            room_set,
            ui_fonts: params.ui_fonts,
        },
    );
}

/// Borrowed inputs for [`session_gameplay_dressing`].
pub struct SessionDressingSetup<'a> {
    pub world: &'a RoomGeometry,
    pub room_set: &'a RoomSet,
    pub ui_fonts: Option<&'a UiFonts>,
}

/// The Ambition-specific SESSION dressing: moving platforms and the
/// marker-tagged HUD/quest text widgets. Split from the generic
/// room visuals so the shell host can delegate parallax/room visuals to the
/// provider-agnostic `SessionRoomVisualsPlugin` (one system serves every
/// linked game) while Ambition keeps its own dressing.
pub fn session_gameplay_dressing(
    commands: &mut Commands,
    scope: ambition::platformer::lifecycle::SessionSpawnScope,
    params: SessionDressingSetup<'_>,
) {
    let world = params.world;
    let room_set = params.room_set;
    let ui_fonts = params.ui_fonts;
    platforms::spawn_moving_platforms(
        commands,
        scope,
        &world.0,
        &platforms::moving_platforms_for_room(room_set.active_spec()),
    );

    // The player's character sprite is NO LONGER bound here. It is installed by
    // the reusable `bind_worn_character_presentation` system (in
    // `ambition_render::PresentationVisualAnimationPlugin`, which this app adds),
    // which reads the canonical sim-owned `WornCharacter` identity carried by the
    // player entity. The app owns only the scene composition below (cameras, HUD,
    // audio); character presentation is engine-generic so demos share it.

    let mut hud_entity = commands.spawn((
        Text::new("Ambition"),
        ui_fonts
            .map(|fonts| fonts.text_font(14.0, UiFontWeight::Monospace))
            .unwrap_or(TextFont {
                font_size: 14.0,
                ..default()
            }),
        TextColor(Color::srgba(0.82, 0.90, 1.0, 0.96)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(10.0),
            max_width: Val::Px(920.0),
            ..default()
        },
        Name::new("Debug HUD"),
        HudText,
    ));
    scope.apply_to(&mut hud_entity);

    // Quest panel: top-right corner, dedicated text widget. Separated
    // from the debug HUD so the quest log doesn't trail the stats dump.
    let mut quest_entity = commands.spawn((
        Text::new(""),
        ui_fonts
            .map(|fonts| fonts.text_font(14.0, UiFontWeight::Monospace))
            .unwrap_or(TextFont {
                font_size: 14.0,
                ..default()
            }),
        TextColor(Color::srgba(0.92, 0.86, 0.62, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(10.0),
            max_width: Val::Px(360.0),
            ..default()
        },
        Name::new("Quest Panel"),
        QuestPanelText,
    ));
    scope.apply_to(&mut quest_entity);

    // The HUD and quest-panel roots are session-scoped and marker-tagged
    // (`HudText` / `QuestPanelText`); their consumers discover them by marker, so
    // no process-global handle bag records them. They die with the session sweep.
}
