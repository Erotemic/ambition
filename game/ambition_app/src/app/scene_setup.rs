//! Presentation-side scene construction (composition root).
//!
//! `presentation_world` spawns the cameras, the player sprite, the HUD/quest-panel text, the static
//! room visuals + parallax, and wires the audio library / SFX bank. It is the render+audio
//! composition that pairs with `ambition_platformer2d::session::setup::simulation_world`
//! (which stays sim-only).
#![allow(clippy::too_many_arguments)]

use bevy::prelude::*;
#[cfg(feature = "audio")]
use ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog;
#[cfg(feature = "audio")]
use ambition_platformer2d::audio::library::AudioLibrary;
use ambition_platformer2d::content::MusicRegistry;
use ambition_platformer2d::render::rendering::{HudText, QuestPanelText};
use ambition_platformer2d::render::ui_fonts::{UiFontWeight, UiFonts};

/// Insert the shared music cache and its playback state.
/// The active audio selection controls playback authority.
#[cfg(feature = "audio")]
pub fn install_audio_library(
    commands: &mut Commands,
    asset_server: &AssetServer,
    catalog: &Platformer2dAssetCatalog,
    music_registry: &MusicRegistry,
) {
    // Resolve music-track ids through the sandbox asset catalog so the
    // library stores catalog-blessed paths (the generic library takes a
    // resolver closure instead of naming the catalog type).
    let resolve_track_path = |id: &str| {
        catalog.path_for(
            &ambition_platformer2d::asset_manager::platformer_assets::ids::music_track(id),
        )
    };
    let (mut audio_library, music_state) = AudioLibrary::new_with_playback_state(
        music_registry,
        Some(&resolve_track_path),
    );
    // Direct startup and shell activation can now include the initial track in
    // their real readiness evidence. Only the selected first track is warmed;
    // the rest of the catalog remains lazy.
    audio_library.preload_track(music_state.active_track(), asset_server);
    commands.insert_resource(audio_library);
    commands.insert_resource(music_state);
}

/// HOST-resident presentation scaffolding: the primary + front-HUD cameras.
/// Spawned once at startup and never owned by a gameplay session — the
/// launcher/title route renders through the same cameras a session does.
/// Additional view cameras are built by the shared SplitViewRigsPlugin.
pub fn host_presentation_scaffold(commands: &mut Commands) {
    // The MAIN camera (order 0) renders the gameplay world (sprites on layer 0), portal-window
    // meshes, and the main-camera-only parallax layer. It NO LONGER carries `IsDefaultUiCamera`:
    // the default UI camera is now the dedicated FRONT camera below (order 9), so all bevy_ui draws
    // IN FRONT of the order-8 cube-menu `Camera3d`. The cube's dim-scrim is the one exception — it
    // must draw BEHIND the cube — and it owns a display-scoped UI camera of its own for that (see
    // `menu::kaleidoscope_app::scrim`).
    let main_camera_layers =
        ambition_platformer2d::render::platformer_presentation::gameplay_camera_layers();
    let main_camera = commands
        .spawn((
            Camera2d,
            ambition_platformer2d::platformer::camera_layers::MainCamera,
            ambition_platformer2d::game_shell::FrontendOwnedEntity::host(
                ambition_platformer2d::game_shell::FrontendPresentationKind::HostCamera,
            ),
            main_camera_layers,
            // ⭐ ELIGIBLE, NOT ENROLLED. This says the main camera is the one the
            // screen filter belongs on; `sync_screen_effect_settings_from_video_settings`
            // adds the `ScreenEffectSettings` that actually enrols it, and only
            // while some effect is turned up. Spawning the settings here instead
            // made every default frame pay for a fullscreen pass that returns
            // the pixel it read.
            ambition_platformer2d::render::screen_effects::ScreenEffectCamera,
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
    ambition_platformer2d::platformer::camera_layers::publish_main_camera(commands, main_camera);
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
                font_size: FontSize::Px(14.0),
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
                font_size: FontSize::Px(14.0),
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
