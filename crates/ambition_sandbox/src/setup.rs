//! Sim/presentation split for the sandbox's startup setup.
//!
//! Slice 4 of ADR 0012's events refactor: the previous monolithic `setup`
//! system in `main.rs` mixed simulation-only world construction
//! (`SandboxRuntime`, `LdtkWorldBundle`, the player entity's gameplay
//! components) with presentation-only spawns (Camera2d, sprites, HUD text,
//! and generated audio library setup). This module factors that into two
//! reusable helpers so the visible binary can call both, the future Slice 5
//! `add_simulation_plugins` / `add_presentation_plugins` split has a clean
//! seam, and the headless binary can call `simulation_world` standalone
//! once the LdtkPlugin-headless question is resolved.
//!
//! Both helpers take `&mut Commands` plus borrowed resource handles so they
//! can be invoked from any Bevy startup system that has gathered the right
//! parameters. They are not Bevy systems themselves; the outer `setup`
//! system in `main.rs` does the param wiring and calls them in sequence.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::AudioSource as KiraAudioSource;

#[cfg(feature = "audio")]
use crate::audio::{AudioLibrary, MusicPlaybackState};
use crate::character_sprites::{build_character_sprite, feet_anchor_for, CharacterAnimator};
use crate::config::{world_to_bevy, WORLD_Z_PLAYER};
use crate::data::{SandboxDataAsset, SandboxDataSpec};
use crate::dev_tools::{EditableAbilitySet, EditableMovementTuning};
use crate::game_assets::GameAssets;
use crate::ldtk_world::{LdtkRuntimeIndex, SandboxLdtkAsset};
use crate::loading::SandboxAssetCollection;
use crate::physics::PhysicsSandboxSettings;
use crate::platforms;
use crate::rendering::{spawn_room_visuals, HudText, PlayerVisual, QuestPanelText, SceneEntities};
use crate::rooms::RoomSet;
use crate::ui_fonts::{UiFontWeight, UiFonts};
use crate::{GameWorld, SandboxRuntime};
#[cfg(feature = "audio")]
use ambition_sfx::BankProvider;

/// Borrowed inputs for `simulation_world`.
pub struct SimulationSetup<'a> {
    pub world: &'a GameWorld,
    pub room_set: &'a RoomSet,
    pub ldtk_index: &'a LdtkRuntimeIndex,
    pub sandbox_data: &'a SandboxDataSpec,
    pub editable_abilities: &'a EditableAbilitySet,
    pub editable_tuning: &'a EditableMovementTuning,
    pub physics_settings: PhysicsSandboxSettings,
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
    // `UiFonts` is only required for the visible/default audio build path.
    // No-audio/headless-ish builds still compile presentation code for
    // checking, but they do not need custom font handles in the setup struct.
    #[cfg(feature = "audio")]
    pub ui_fonts: Option<&'a UiFonts>,
}

/// Spawn simulation-only entities and resources.
pub fn simulation_world(commands: &mut Commands, params: SimulationSetup<'_>) -> Entity {
    let SimulationSetup {
        world,
        room_set,
        ldtk_index,
        sandbox_data: _,
        editable_abilities,
        editable_tuning,
        physics_settings,
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
        eprintln!("room layout warning: {warning}");
    }
    let _ = (ldtk_asset, asset_server);
    let _ = ldtk_index;

    let runtime = SandboxRuntime::new(
        &world.0,
        editable_abilities.as_engine(),
        editable_tuning.as_engine(),
        physics_settings,
    );
    commands.insert_resource(runtime);

    let player = commands
        .spawn((
            Transform::from_translation(world_to_bevy(&world.0, world.0.spawn, WORLD_Z_PLAYER)),
            PlayerVisual,
            Name::new("Player"),
        ))
        .id();

    commands.insert_resource(SceneEntities {
        player,
        hud: Entity::PLACEHOLDER,
        quest_panel: Entity::PLACEHOLDER,
    });

    player
}

#[cfg(feature = "audio")]
pub fn presentation_world(
    commands: &mut Commands,
    audio_sources: &mut Assets<KiraAudioSource>,
    asset_server: &AssetServer,
    params: PresentationSetup<'_>,
    player: Entity,
) {
    let sandbox_data = params.sandbox_data;
    presentation_world_inner(commands, params, player);
    let bank_provider = try_load_sfx_bank();
    let audio_library = AudioLibrary::new(
        audio_sources,
        &sandbox_data.audio,
        Some(asset_server),
        bank_provider
            .as_ref()
            .map(|provider| provider as &dyn ambition_sfx::SfxProvider),
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

#[cfg(feature = "audio")]
#[derive(Resource, Clone)]
pub struct SfxBankResource(pub std::sync::Arc<BankProvider>);

#[cfg(feature = "audio")]
fn try_load_sfx_bank() -> Option<BankProvider> {
    use std::path::PathBuf;
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(env_path) = std::env::var("AMBITION_SFX_BANK_PATH") {
        candidates.push(PathBuf::from(env_path));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("assets/audio/sfx.bank"));
        candidates.push(cwd.join("crates/ambition_sandbox/assets/audio/sfx.bank"));
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/audio/sfx.bank"));

    for path in &candidates {
        if path.is_file() {
            match BankProvider::from_path(path) {
                Ok(provider) => {
                    debug!("sfx bank loaded from {}", path.display());
                    return Some(provider);
                }
                Err(error) => {
                    warn!(
                        "found sfx bank at {} but failed to parse: {error}",
                        path.display()
                    );
                }
            }
        }
    }
    info!(
        "no sfx bank found (looked in: {}); falling back to fundsp synthesis for SFX",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    None
}

fn presentation_world_inner(
    commands: &mut Commands,
    params: PresentationSetup<'_>,
    player: Entity,
) {
    let PresentationSetup {
        world,
        room_set,
        sandbox_data: _,
        physics_settings,
        game_assets,
        #[cfg(feature = "audio")]
        ui_fonts,
    } = params;
    #[cfg(not(feature = "audio"))]
    let ui_fonts: Option<&UiFonts> = None;
    let character_sprites = &game_assets.characters;

    commands.spawn((Camera2d, Name::new("Main Camera")));

    let t_room = std::time::Instant::now();
    spawn_room_visuals(
        commands,
        &world.0,
        room_set.active_loading_zones(),
        physics_settings,
        Some(game_assets),
    );
    let t_room_ms = t_room.elapsed().as_secs_f32() * 1000.0;
    eprintln!(
        "[startup]   presentation_world breakdown: spawn_room_visuals={t_room_ms:.1}ms (active room only)"
    );
    platforms::spawn_moving_platform(
        commands,
        &world.0,
        platforms::MovingPlatformState::time_reference(&world.0),
    );

    let player_collision = BVec2::new(28.0, 46.0);
    if let Some(asset) = &character_sprites.robot {
        let sprite = build_character_sprite(asset, player_collision);
        commands.entity(player).insert((
            sprite,
            feet_anchor_for(asset.spec, player_collision),
            CharacterAnimator::new(asset.spec),
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

    commands.insert_resource(SceneEntities {
        player,
        hud,
        quest_panel,
    });

    let _ = physics_settings;
}
