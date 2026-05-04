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
use crate::rendering::{spawn_room_visuals, HudText, PlayerVisual, SceneEntities};
use crate::rooms::RoomSet;
use crate::{GameWorld, SandboxRuntime};

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
/// * constructing `SandboxRuntime` and inserting it as a resource
/// * spawning the player entity with the gameplay-essential components
///   (`Transform`, `PlayerVisual`). Leafwing's `ActionState` and
///   `InputMap` get attached by the presentation-side
///   `attach_player_input_components` startup system; sim-only builds
///   stay leafwing-free per the ADR 0012 input seam.
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
    // The LdtkWorldBundle spawn lives in the Ldtk-runtime startup system
    // (`crate::app::add_ldtk_runtime_plugin`) because asset_server.load on a
    // typed `LdtkProject` handle requires `LdtkPlugin` to be registered.
    // Headless builds skip LdtkPlugin (its tile pipeline needs RenderApp),
    // so this function must not assume the LDtk asset type is available.
    // Suppress the unused-binding warnings until follow-up patches retire
    // the `ldtk_asset` / `ldtk_index` / `asset_server` params or move them.
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

    // HUD entity is presentation-side; placeholder until presentation_world
    // overwrites this resource.
    commands.insert_resource(SceneEntities {
        player,
        hud: Entity::PLACEHOLDER,
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
    params: PresentationSetup<'_>,
    player: Entity,
) {
    let sandbox_data = params.sandbox_data;
    presentation_world_inner(commands, params, player);
    let audio_library = AudioLibrary::new(audio_sources, &sandbox_data.audio);
    let music_state = MusicPlaybackState::from_audio_spec(&sandbox_data.audio, &audio_library);
    commands.insert_resource(audio_library);
    commands.insert_resource(music_state);
}

#[cfg(not(feature = "audio"))]
pub fn presentation_world(commands: &mut Commands, params: PresentationSetup<'_>, player: Entity) {
    presentation_world_inner(commands, params, player);
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
    } = params;
    let character_sprites = &game_assets.characters;

    commands.spawn((Camera2d, Name::new("Main Camera")));

    spawn_room_visuals(
        commands,
        &world.0,
        room_set.active_loading_zones(),
        physics_settings,
        Some(game_assets),
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
            TextFont {
                font_size: 14.0,
                ..default()
            },
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

    // Overwrite the placeholder SceneEntities from simulation_world now
    // that the HUD entity exists. `commands.insert_resource` replaces the
    // existing resource on apply_deferred.
    commands.insert_resource(SceneEntities { player, hud });

    // Reserve the physics_settings binding for future presentation systems
    // that might need it; suppress the unused-variable warning until then.
    let _ = physics_settings;
}
