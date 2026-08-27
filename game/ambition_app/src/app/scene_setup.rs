//! Presentation-side scene construction (composition root).
//!
//! `presentation_world` spawns the cameras, the player sprite, the HUD/quest-panel text, the static
//! room visuals + parallax, and wires the audio library / SFX bank. It is the render+audio
//! composition that pairs with `ambition_platformer2d::actors::session::setup::simulation_world`
//! (which stays sim-only).
#![allow(clippy::too_many_arguments)]

use bevy::prelude::*;
#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::AudioSource as KiraAudioSource;

#[cfg(feature = "audio")]
// The platform VISUAL spawn is presentation and lives in the actor monolith;
// the platform STATE it renders is the world crate's and is named there.
#[cfg(feature = "audio")]
use ambition_platformer2d::asset_manager::platformer_assets::{ids, Platformer2dAssetCatalog};
#[cfg(feature = "audio")]
use ambition_platformer2d::audio::library::AudioLibrary;
#[cfg(feature = "audio")]
use ambition_platformer2d::audio::SfxBankResource;
use ambition_platformer2d::render::rendering::{HudText, QuestPanelText};
use ambition_platformer2d::render::ui_fonts::{UiFontWeight, UiFonts};
#[cfg(feature = "audio")]
use ambition_platformer2d::sfx::BankProvider;
use ambition_platformer2d::content::MusicRegistry;
use ambition_platformer2d::content::SfxRegistry;

/// Build and insert the host-resident audio library (packed SFX bank +
/// catalog-resolved music assets) and its playback state. An asset CACHE —
/// host-owned, shared across sessions; the per-session audio AUTHORITY is
/// `ambition_platformer2d::audio::selection::ActiveAudioSelection`.
#[cfg(feature = "audio")]
pub fn install_audio_library(
    commands: &mut Commands,
    audio_sources: &mut Assets<KiraAudioSource>,
    asset_server: &AssetServer,
    catalog: &Platformer2dAssetCatalog,
    music_registry: &MusicRegistry,
    sfx_registry: &SfxRegistry,
) {
    let bank_provider = try_load_sfx_bank_via_catalog(catalog);
    // Resolve music-track ids through the sandbox asset catalog so the
    // library stores catalog-blessed paths (the generic library takes a
    // resolver closure instead of naming the catalog type).
    let resolve_track_path = |id: &str| {
        catalog.path_for(
            &ambition_platformer2d::asset_manager::platformer_assets::ids::music_track(id),
        )
    };
    let (mut audio_library, music_state) = AudioLibrary::new_with_playback_state(
        audio_sources,
        sfx_registry,
        music_registry,
        Some(asset_server),
        bank_provider
            .as_ref()
            .map(|provider| provider as &dyn ambition_platformer2d::sfx::SfxProvider),
        Some(&resolve_track_path),
    );
    // Direct startup and shell activation can now include the initial track in
    // their real readiness evidence. Only the selected first track is warmed;
    // the rest of the catalog remains lazy.
    audio_library.preload_track(music_state.active_track(), asset_server);
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
/// [`ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog`] and synchronously
/// load its bytes into a [`BankProvider`]. Fall-through order:
///
/// 1. the statically packed bank (`static_sfx_bank` feature),
/// 2. the catalog's resolved `LocalPath` candidate (preferred —
///    explicit `AMBITION_SFX_BANK_PATH` dev override or platform
///    bundle path),
/// 3. the catalog's `LooseFilesystem` synthesized default located via
///    [`Platformer2dAssetCatalog::resolve_local_file_path`],
/// 4. `None` + a single info log → the [`AudioLibrary`] uses a short
///    silent stub for any missing cue (procedural fallback retired).
///
/// All host-filesystem probing for the SFX bank happens through the
/// catalog. This function owns no candidate-roots walk.
#[cfg(feature = "audio")]
fn try_load_sfx_bank_via_catalog(catalog: &Platformer2dAssetCatalog) -> Option<BankProvider> {
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

/// HOST-resident presentation scaffolding: the main + front-HUD cameras. Spawned
/// once at startup and never owned by a gameplay session — the launcher/title
/// route renders through the same cameras a session does.
pub fn host_presentation_scaffold(commands: &mut Commands) {
    // The MAIN camera (order 0) renders the gameplay world (sprites on layer 0), portal-window
    // meshes, and the main-camera-only parallax layer. It NO LONGER carries `IsDefaultUiCamera`:
    // the default UI camera is now the dedicated FRONT camera below (order 9), so all bevy_ui draws
    // IN FRONT of the order-8 cube-menu `Camera3d`. The cube's dim-scrim is the one exception — it
    // must draw BEHIND the cube — and it owns a display-scoped UI camera of its own for that (see
    // `menu::kaleidoscope_app::scrim`).
    let mut main_camera_layers = bevy::camera::visibility::RenderLayers::layer(0)
        .with(ambition_platformer2d::platformer::camera_layers::PARALLAX_BACKGROUND_LAYER);
    #[cfg(feature = "portal_render")]
    {
        main_camera_layers = main_camera_layers
            .with(ambition_platformer2d::portal_presentation::PORTAL_WINDOW_RENDER_LAYER);
    }
    let main_camera = commands
        .spawn((
            Camera2d,
            ambition_platformer2d::platformer::camera_layers::MainCamera,
            ambition_platformer2d::game_shell::FrontendOwnedEntity::host(
                ambition_platformer2d::game_shell::FrontendPresentationKind::HostCamera,
            ),
            main_camera_layers,
            ambition_platformer2d::render::screen_effects::ScreenEffectSettings::default(),
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
        ambition_platformer2d::platformer::camera_layers::FrontHudCamera,
        ambition_platformer2d::game_shell::FrontendOwnedEntity::host(
            ambition_platformer2d::game_shell::FrontendPresentationKind::FrontendUiCamera,
        ),
        IsDefaultUiCamera,
        bevy::camera::visibility::RenderLayers::layer(
            ambition_platformer2d::platformer::camera_layers::FRONT_HUD_LAYER,
        ),
        Name::new("Front HUD Camera"),
    ));

    // The view is spawned at plugin BUILD time, so it is already here; binding the link at
    // SPAWN makes "which view does this camera show" a composition decision on the entity
    // rather than a uniqueness assumption re-derived every frame in `camera_follow`.
    //
    // deferred, because this helper takes only `Commands` — the view is spawned
    // at plugin BUILD time so it is already in the world when this runs.
    //
    // and it is resolved by `ViewsOnHand`, not by `iter().next()`. The
    // first cut took the first view the archetype yielded, which is right for one
    // view and a coin flip for two — this scaffold spawns exactly ONE main
    // camera, so with several views there is no view it can honestly claim to
    // present. Refusing (the rule's own answer, logged once) leaves the link off,
    // and every consumer then declines loudly instead of drawing an arbitrary
    // view through this rig. A composition that wants two rigs binds them itself
    // with `ambition_platformer2d::sim_view::compose_local_views`, which spawns N
    // views carrying exactly the facts the engine's single-view path spawns and
    // binds one camera to each.
    commands.queue(move |world: &mut bevy::prelude::World| {
        let mut views = world.query_filtered::<
            bevy::prelude::Entity,
            bevy::prelude::With<ambition_platformer2d::sim_view::LocalView>,
        >();
        let on_hand = ambition_platformer2d::sim_view::ViewsOnHand::survey(views.iter(world));
        let Some(view) = on_hand.presented_by(None) else {
            return;
        };
        if let Ok(mut camera) = world.get_entity_mut(main_camera) {
            camera.insert(ambition_platformer2d::sim_view::PresentsView(view));
        }
    });

    // a single-camera SPAWN RECORD, published through the shared writer that
    // complains about a second rig instead of letting the last one win. Nothing
    // in production reads it: `camera_follow` and the viewport applier each
    // resolve through the camera's own `PresentsView` link, and the cube's
    // full-screen dim-scrim now targets its own display-scoped UI camera rather
    // than borrowing this one (which carries a `Camera::viewport` under any
    // fixed-aspect profile, and is one pane of several under a split).
    ambition_platformer2d::platformer::camera_layers::publish_main_camera(
        commands,
        main_camera,
    );
}

/// Borrowed inputs for [`session_gameplay_dressing`].
///
/// The dressing is text widgets now, and its signature says so.
pub struct SessionDressingSetup<'a> {
    pub ui_fonts: Option<&'a UiFonts>,
}

/// The Ambition-specific SESSION dressing: the marker-tagged HUD/quest text
/// widgets. Split from the generic room visuals so the shell host can delegate
/// parallax/room visuals to the provider-agnostic `SessionRoomVisualsPlugin`
/// (one system serves every linked game) while Ambition keeps its own dressing.
pub fn session_gameplay_dressing(
    commands: &mut Commands,
    scope: ambition_platformer2d::platformer::lifecycle::SessionSpawnScope,
    params: SessionDressingSetup<'_>,
) {
    let ui_fonts = params.ui_fonts;
    // the moving platforms' visuals are no longer spawned here. They are
    // reconciled by a render family from the authoritative `MovingPlatformSet`,
    // like every other room feature — see
    // `ambition_render::rendering::moving_platforms`.

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
