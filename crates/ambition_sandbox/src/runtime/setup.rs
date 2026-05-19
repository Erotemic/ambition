//! Sim/presentation split for the sandbox's startup setup.
//!
//! Slice 4 of ADR 0012's events refactor: the previous monolithic `setup`
//! system in `main.rs` mixed simulation-only world construction
//! (`LdtkWorldBundle`, the player entity's gameplay components) with
//! presentation-only spawns (Camera2d, sprites, HUD text, and generated
//! audio library setup). This module factors that into two reusable helpers
//! so the visible binary can call both, the future Slice 5
//! `add_simulation_plugins` / `add_presentation_plugins` split has a clean
//! seam, and the headless binary can call `simulation_world` standalone
//! once the LdtkPlugin-headless question is resolved.
//!
//! Both helpers take `&mut Commands` plus borrowed resource handles so they
//! can be invoked from any Bevy startup system that has gathered the right
//! parameters. They are not Bevy systems themselves; the outer `setup`
//! system in `main.rs` does the param wiring and calls them in sequence.

use ambition_engine as ae;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::AudioSource as KiraAudioSource;

use crate::assets::game_assets::GameAssets;
use crate::assets::loading::SandboxAssetCollection;
#[cfg(feature = "audio")]
use crate::assets::sandbox_assets::{ids, SandboxAssetCatalog};
#[cfg(feature = "audio")]
use crate::audio::{AudioLibrary, MusicPlaybackState};
use crate::config::{world_to_bevy, WORLD_Z_PLAYER};
use crate::content::data::{SandboxDataAsset, SandboxDataSpec};
use crate::dev::dev_tools::{EditableAbilitySet, EditableMovementTuning};
use crate::ldtk_world::{LdtkRuntimeIndex, SandboxLdtkAsset};
use crate::presentation::character_sprites::{
    build_character_sprite_with_render_size, feet_anchor_for_render_size,
    player_placeholder_render_size, CharacterAnimator,
};
use crate::presentation::rendering::{
    spawn_parallax_layers, spawn_room_visuals, HudText, PlayerSpriteBaseline, PlayerVisual,
    QuestPanelText, SceneEntities,
};
use crate::presentation::ui_fonts::{UiFontWeight, UiFonts};
use crate::rooms::RoomSet;
use crate::world::physics::PhysicsSandboxSettings;
use crate::world::platforms;
use crate::GameWorld;
#[cfg(feature = "audio")]
use ambition_sfx::BankProvider;

/// Borrowed inputs for `simulation_world`.
///
/// Grouped as a struct because Bevy's max-system-param budget is tight and
/// keeping these as positional args would push the calling startup system
/// past 16 params again. The struct also documents what the simulation
/// half of setup actually needs.
pub struct SimulationSetup<'a> {
    pub world: &'a GameWorld,
    pub room_set: &'a RoomSet,
    pub ldtk_index: &'a LdtkRuntimeIndex,
    pub sandbox_data: &'a SandboxDataSpec,
    pub editable_abilities: &'a EditableAbilitySet,
    pub editable_tuning: &'a EditableMovementTuning,
    pub sandbox_data_asset: Option<&'a SandboxDataAsset>,
    pub ldtk_asset: Option<&'a SandboxLdtkAsset>,
    pub sandbox_asset_collection: Option<&'a SandboxAssetCollection>,
    pub asset_server: &'a AssetServer,
}

/// Borrowed inputs for `presentation_world`.
pub struct PresentationSetup<'a> {
    pub world: &'a GameWorld,
    pub room_set: &'a RoomSet,
    pub sandbox_data: &'a SandboxDataSpec,
    pub physics_settings: PhysicsSandboxSettings,
    pub game_assets: &'a GameAssets,
    #[cfg(feature = "audio")]
    pub ui_fonts: Option<&'a UiFonts>,
}

/// Spawn simulation-only entities and resources.
///
/// Returns the player entity so `presentation_world` (or any future RL
/// adapter) can attach presentation components without re-querying.
///
/// This includes:
/// * pre-fetching sandbox/LDtk asset handles to keep the asset server alive
/// * logging room layout warnings
/// * spawning the `LdtkWorldBundle` so `bevy_ecs_ldtk` can own LDtk entity
///   lifecycle and the runtime-spine systems have something to query
/// * spawning the player entity with gameplay-essential ECS components
///   (`Transform`, `PlayerVisual`, `PlayerMovementAuthority`, etc.).
///   Leafwing's `ActionState` and `InputMap` get attached by the
///   presentation-side `attach_player_input_components` startup system;
///   sim-only builds stay leafwing-free per the ADR 0012 input seam.
/// * inserting a `SceneEntities` resource with `hud: Entity::PLACEHOLDER`
///   that `presentation_world` overwrites once the HUD entity exists
pub fn simulation_world(commands: &mut Commands, params: SimulationSetup<'_>) -> Entity {
    // `sandbox_data` is reserved on `SimulationSetup` for symmetry with
    // `PresentationSetup` and to support future sim-side reads (e.g. movement
    // tuning resolved through SandboxDataSpec instead of the editable
    // resources). Suppress the unused-field warning until then.
    let SimulationSetup {
        world,
        room_set,
        ldtk_index,
        sandbox_data: _,
        editable_abilities,
        editable_tuning,
        sandbox_data_asset,
        ldtk_asset,
        sandbox_asset_collection,
        asset_server,
    } = params;

    if let Some(handle) = sandbox_data_asset {
        let _asset_handle_for_async_reload = handle.0.clone();
    }
    if let Some(collection) = sandbox_asset_collection {
        let _loaded_sandbox_data_handle = collection.sandbox_data.clone();
        let _loaded_ldtk_project_handle = collection.ldtk_project.clone();
    }
    for warning in room_set.layout_warnings() {
        bevy::log::debug!(target: "ambition::room_layout", "{warning}");
    }
    // The LdtkWorldBundle spawn lives in the Ldtk-runtime startup system
    // (`crate::app::add_ldtk_runtime_plugin`) because asset_server.load on a
    // typed `LdtkProject` handle requires `LdtkPlugin` to be registered.
    // Headless builds skip LdtkPlugin (its tile pipeline needs RenderApp),
    // so this function must not assume the LDtk asset type is available.
    // Suppress the unused-binding warnings until follow-up patches retire
    // the `ldtk_asset` / `ldtk_index` / `asset_server` params or move them.
    let _ = (ldtk_asset, asset_server);
    let _ = ldtk_index;

    crate::features::spawn_room_feature_entities(commands, room_set.active_spec());

    let mut initial_player =
        ae::Player::new_with_abilities(world.0.spawn, editable_abilities.as_engine());
    initial_player.refresh_movement_resources(editable_tuning.as_engine());

    let player = commands
        .spawn((
            Transform::from_translation(world_to_bevy(&world.0, world.0.spawn, WORLD_Z_PLAYER)),
            PlayerVisual,
            crate::player::PlayerSimulationBundle::new(initial_player, ae::Health::new(20)),
        ))
        .id();

    // HUD entity is presentation-side; placeholder until presentation_world
    // overwrites this resource.
    commands.insert_resource(SceneEntities {
        player,
        hud: Entity::PLACEHOLDER,
        quest_panel: Entity::PLACEHOLDER,
    });

    player
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
    player: Entity,
) {
    let sandbox_data = params.sandbox_data;
    presentation_world_inner(commands, params, player);
    let bank_provider = try_load_sfx_bank_via_catalog(catalog);
    let audio_library = AudioLibrary::new(
        audio_sources,
        &sandbox_data.audio,
        Some(asset_server),
        bank_provider
            .as_ref()
            .map(|provider| provider as &dyn ambition_sfx::SfxProvider),
        Some(catalog),
    );
    let music_state = MusicPlaybackState::from_audio_spec(&sandbox_data.audio, &audio_library);
    commands.insert_resource(audio_library);
    commands.insert_resource(music_state);
    if let Some(provider) = bank_provider {
        info!("loaded sfx bank: {} entries", provider.entry_count());
        commands.insert_resource(SfxBankResource(std::sync::Arc::new(provider)));
    }
}

#[cfg(not(feature = "audio"))]
pub fn presentation_world(commands: &mut Commands, params: PresentationSetup<'_>, player: Entity) {
    presentation_world_inner(commands, params, player);
}

/// Process-wide handle to the loaded SFX bank, when one was found at
/// startup. Wrapped in `Arc` so future systems that need to play
/// catalog SFX (beyond the typed `SoundCue` set the `AudioLibrary`
/// preloads) can clone cheaply and look up by id without re-reading
/// the file. Absent when the bank file isn't on disk; missing cues
/// then resolve to a short silent stub (see
/// `crate::audio::render::silent_audio_source`).
#[cfg(feature = "audio")]
#[derive(Resource, Clone)]
pub struct SfxBankResource(pub std::sync::Arc<BankProvider>);

/// Load a statically packed SFX bank.
///
/// Android APK assets are not normal host filesystem paths, while the current
/// SFX bank loader is synchronous and path/byte based. Until that loader grows
/// an APK-asset backend, `build_for_android.sh` can enable `static_sfx_bank`
/// and pass `AMBITION_STATIC_SFX_BANK_PATH` so the packed bank is available to
/// the same runtime bank provider used on desktop.
#[cfg(all(feature = "audio", feature = "static_sfx_bank"))]
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

/// Resolve the SFX bank through the
/// [`crate::assets::sandbox_assets::SandboxAssetCatalog`] and synchronously
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

fn presentation_world_inner(
    commands: &mut Commands,
    params: PresentationSetup<'_>,
    player: Entity,
) {
    let world = params.world;
    let room_set = params.room_set;
    let physics_settings = params.physics_settings;
    let game_assets = params.game_assets;
    #[cfg(feature = "audio")]
    let ui_fonts = params.ui_fonts;
    #[cfg(not(feature = "audio"))]
    let ui_fonts: Option<&UiFonts> = None;
    let character_sprites = &game_assets.characters;

    commands.spawn((
        Camera2d,
        crate::presentation::screen_effects::ScreenEffectSettings::default(),
        Name::new("Main Camera"),
    ));

    // `Instant::now()` is unsupported under `wasm32-unknown-unknown`
    // (panics with "time not implemented on this platform"). Gate the
    // per-step wall-clock breakdown on non-wasm; the wasm build
    // measures via browser devtools.
    #[cfg(not(target_arch = "wasm32"))]
    let t_room = std::time::Instant::now();
    spawn_parallax_layers(
        commands,
        &world.0,
        &room_set.active_spec().metadata,
        Some(game_assets),
    );
    spawn_room_visuals(
        commands,
        &world.0,
        room_set.active_loading_zones(),
        room_set.active_props(),
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
    platforms::spawn_moving_platforms(
        commands,
        &world.0,
        &platforms::moving_platforms_for_room(room_set.active_spec()),
    );

    let player_collision = BVec2::new(
        ae::DEFAULT_PLAYER_BODY_WIDTH,
        ae::DEFAULT_PLAYER_BODY_HEIGHT,
    );
    if let Some(asset) = character_sprites
        .player
        .as_ref()
        .or(character_sprites.robot.as_ref())
    {
        let player_render = player_placeholder_render_size(asset.spec, player_collision);
        let sprite = build_character_sprite_with_render_size(asset, player_render);
        commands.entity(player).insert((
            sprite,
            feet_anchor_for_render_size(asset.spec, player_collision, player_render),
            CharacterAnimator::new(asset.spec),
            PlayerSpriteBaseline {
                standing_render: player_render,
                standing_collision: player_collision,
            },
        ));
    } else {
        commands.entity(player).insert(Sprite::from_color(
            Color::srgba(0.80, 0.95, 1.0, 1.0),
            player_collision,
        ));
    }

    let hud = commands
        .spawn((
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
        ))
        .id();

    // Quest panel: top-right corner, dedicated text widget. Separated
    // from the debug HUD so the quest log doesn't trail the stats dump.
    let quest_panel = commands
        .spawn((
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
        ))
        .id();

    // Overwrite the placeholder SceneEntities from simulation_world now
    // that the HUD entity exists. `commands.insert_resource` replaces the
    // existing resource on apply_deferred.
    commands.insert_resource(SceneEntities {
        player,
        hud,
        quest_panel,
    });
}
