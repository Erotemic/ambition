use bevy::prelude::*;

#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::AudioSource as KiraAudioSource;

use ambition_gameplay_core::assets::game_assets::{self, GameAssetConfig};
use ambition_gameplay_core::assets::loading;
use ambition_gameplay_core::dev::dev_tools::{EditableAbilitySet, EditableMovementTuning};
use ambition_gameplay_core::ldtk_world;
use ambition_gameplay_core::persistence::settings::TextureResolutionScale;
use ambition_gameplay_core::rooms;
use ambition_gameplay_core::session::{data, setup};
use ambition_gameplay_core::world::physics;
use ambition_gameplay_core::RoomGeometry;
use ambition_render::rendering::SceneEntities;
use ambition_render::ui_fonts;

use super::scene_setup;

/// Sim-only startup. Calls `ambition_gameplay_core::session::setup::simulation_world` to spawn the
/// LdtkWorldBundle and the player entity (with gameplay-essential components
/// but no Sprite). Inserts SceneEntities with `hud: Entity::PLACEHOLDER`;
/// the presentation startup system later overwrites that with the real HUD entity.
pub(super) fn setup_simulation_system(
    mut commands: Commands,
    world: Res<RoomGeometry>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data_asset: Option<Res<data::SandboxDataAsset>>,
    ldtk_asset: Option<Res<ldtk_world::SandboxLdtkAsset>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
    asset_server: Res<AssetServer>,
    ldtk_index: Res<ldtk_world::LdtkRuntimeIndex>,
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    mut platform_set: ResMut<ambition_gameplay_core::MovingPlatformSet>,
) {
    let _player = setup::simulation_world(
        &mut commands,
        setup::SimulationSetup {
            world: &world,
            room_set: &room_set,
            ldtk_index: &ldtk_index,
            editable_abilities: &editable_abilities,
            editable_tuning: &editable_tuning,
            sandbox_data_asset: sandbox_data_asset.as_deref(),
            ldtk_asset: ldtk_asset.as_deref(),
            sandbox_asset_collection: sandbox_asset_collection.as_deref(),
            asset_server: &asset_server,
        },
    );
    platform_set.0 =
        ambition_gameplay_core::world::platforms::moving_platforms_for_room(room_set.active_spec());
    // `PlayerSafetyState::last_safe_pos` is initialized by the player
    // bundle to the player's spawn position (which is `world.0.spawn`),
    // so we don't need to overwrite it here. See
    // `ambition_gameplay_core::player::PlayerSimulationBundle::new`.
}

/// Presentation startup. Runs after `setup_simulation_system` so the
/// SceneEntities resource (with player Entity) is visible. Adds the
/// player's Sprite, spawns Camera2d, room visuals, HUD text, generated
/// Kira audio library, and overwrites SceneEntities to fill in the HUD
/// entity.
#[cfg(feature = "audio")]
pub(crate) fn setup_presentation_system(
    mut commands: Commands,
    world: Res<RoomGeometry>,
    room_set: Res<rooms::RoomSet>,
    music_registry: Res<data::MusicRegistry>,
    sfx_registry: Res<data::SfxRegistry>,
    sandbox_catalog: Res<ambition_gameplay_core::assets::sandbox_assets::SandboxAssetCatalog>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    scene_entities: Res<SceneEntities>,
    ui_fonts: Option<Res<ui_fonts::UiFonts>>,
    quality: Option<Res<ambition_render::quality::ResolvedVisualQuality>>,
    mut profiler: ResMut<ambition_gameplay_core::dev::profiling::StartupProfiler>,
) {
    // `std::time::Instant::now()` panics on `wasm32-unknown-unknown`
    // with "time not implemented on this platform". Gate the per-step
    // wall-clock breakdown on non-wasm; the wasm build profiles via
    // browser devtools (see docs/recipes/web-build.md).
    #[cfg(not(target_arch = "wasm32"))]
    let t0 = std::time::Instant::now();
    let game_assets = game_assets::load_game_assets(
        &asset_config,
        &sandbox_catalog,
        &asset_server,
        &mut atlas_layouts,
        &room_set.active_spec().metadata,
        quality.as_deref().map(|q| &q.budget),
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        let t_assets = t0.elapsed().as_secs_f32() * 1000.0;
        profiler.marks.push((
            "setup_presentation::load_game_assets",
            std::time::Instant::now(),
        ));
        let t1 = std::time::Instant::now();
        scene_setup::presentation_world(
            &mut commands,
            &mut audio_sources,
            &asset_server,
            &sandbox_catalog,
            scene_setup::PresentationSetup {
                world: &world,
                room_set: &room_set,
                physics_settings: *physics_settings,
                game_assets: &game_assets,
                quality: quality.as_deref(),
                music_registry: &music_registry,
                sfx_registry: &sfx_registry,
                ui_fonts: ui_fonts.as_deref(),
            },
            scene_entities.player,
        );
        let t_present = t1.elapsed().as_secs_f32() * 1000.0;
        eprintln!(
            "[startup]   setup_presentation breakdown: load_game_assets={t_assets:.1}ms presentation_world={t_present:.1}ms"
        );
        profiler.marks.push((
            "setup_presentation::presentation_world",
            std::time::Instant::now(),
        ));
    }
    #[cfg(target_arch = "wasm32")]
    {
        // Wasm path: no per-step timing, no profiler marks (the
        // wasm `StartupProfiler` doesn't take Instants — see
        // `ambition_gameplay_core::profiling`). The presentation world still spawns.
        let _ = &profiler; // silence unused-resource warning
        scene_setup::presentation_world(
            &mut commands,
            &mut audio_sources,
            &asset_server,
            &sandbox_catalog,
            scene_setup::PresentationSetup {
                world: &world,
                room_set: &room_set,
                physics_settings: *physics_settings,
                game_assets: &game_assets,
                quality: quality.as_deref(),
                music_registry: &music_registry,
                sfx_registry: &sfx_registry,
                ui_fonts: ui_fonts.as_deref(),
            },
            scene_entities.player,
        );
    }
    commands.insert_resource(game_assets);
}

pub(crate) fn reload_visual_quality_assets_on_scale_change(
    quality: Res<ambition_render::quality::ResolvedVisualQuality>,
    asset_config: Res<GameAssetConfig>,
    sandbox_catalog: Res<ambition_gameplay_core::assets::sandbox_assets::SandboxAssetCatalog>,
    asset_server: Res<AssetServer>,
    room_set: Res<rooms::RoomSet>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut game_assets: Option<ResMut<game_assets::GameAssets>>,
    mut last_scales: Local<Option<(TextureResolutionScale, TextureResolutionScale)>>,
) {
    let scales = (
        quality.budget.sprites.resolution_scale,
        quality.budget.backgrounds.resolution_scale,
    );
    if last_scales.is_none() {
        *last_scales = Some(scales);
        return;
    }
    if *last_scales == Some(scales) {
        return;
    }
    *last_scales = Some(scales);
    let Some(game_assets) = game_assets.as_deref_mut() else {
        return;
    };
    *game_assets = game_assets::load_game_assets(
        &asset_config,
        &sandbox_catalog,
        &asset_server,
        &mut atlas_layouts,
        &room_set.active_spec().metadata,
        Some(&quality.budget),
    );
}

#[cfg(not(feature = "audio"))]
pub(crate) fn setup_presentation_system(
    mut commands: Commands,
    world: Res<RoomGeometry>,
    room_set: Res<rooms::RoomSet>,
    sandbox_catalog: Res<ambition_gameplay_core::assets::sandbox_assets::SandboxAssetCatalog>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    scene_entities: Res<SceneEntities>,
    quality: Option<Res<ambition_render::quality::ResolvedVisualQuality>>,
) {
    let game_assets = game_assets::load_game_assets(
        &asset_config,
        &sandbox_catalog,
        &asset_server,
        &mut atlas_layouts,
        &room_set.active_spec().metadata,
        quality.as_deref().map(|q| &q.budget),
    );
    scene_setup::presentation_world(
        &mut commands,
        scene_setup::PresentationSetup {
            world: &world,
            room_set: &room_set,
            physics_settings: *physics_settings,
            game_assets: &game_assets,
            quality: quality.as_deref(),
        },
        scene_entities.player,
    );
    commands.insert_resource(game_assets);
}
