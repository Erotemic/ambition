use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::AudioSource as KiraAudioSource;

use ambition::actors::assets::game_assets as actor_game_assets;
use ambition::actors::assets::loading;
use ambition::actors::ldtk_world;
use ambition::actors::rooms;
use ambition::actors::session::{data, setup};
use ambition::actors::world::physics;
use ambition::dev_tools::dev_tools::{EditableAbilitySet, EditableMovementTuning};
use ambition::engine_core::RoomGeometry;
use ambition::persistence::settings::TextureResolutionScale;
use ambition::render::rendering::SceneEntities;
use ambition::render::ui_fonts;
use ambition::sprite_sheet::game_assets::{self, GameAssetConfig};

use super::scene_setup;

/// App-local authored catalogs consumed together by presentation asset loading.
/// Grouping them keeps Bevy system signatures below the function-parameter
/// implementation limit while preserving explicit authority.
#[derive(SystemParam)]
pub(crate) struct PresentationCatalogs<'w> {
    characters: Res<'w, ambition::characters::actor::character_catalog::CharacterCatalog>,
    bosses: Res<'w, ambition::actors::boss_encounter::BossCatalog>,
    assets: Res<'w, ambition::asset_manager::sandbox_assets::SandboxAssetCatalog>,
}

/// Sim-only startup. Calls `ambition::actors::session::setup::simulation_world` to spawn the
/// LdtkWorldBundle and the player entity (with gameplay-essential components
/// but no Sprite). Inserts SceneEntities with `hud: Entity::PLACEHOLDER`;
/// the presentation startup system later overwrites that with the real HUD entity.
pub(super) fn setup_simulation_system(
    mut commands: Commands,
    world: Res<RoomGeometry>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data_asset: Option<Res<data::SandboxDataAsset>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
    asset_server: Res<AssetServer>,
    ldtk_index: Res<ldtk_world::LdtkRuntimeIndex>,
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    starting_character: Res<ambition::actors::avatar::StartingCharacter>,
    character_catalog: Res<ambition::characters::actor::character_catalog::CharacterCatalog>,
    character_roster: Res<ambition::actors::features::CharacterRoster>,
    boss_catalog: Res<ambition::actors::boss_encounter::BossCatalog>,
    mut platform_set: ResMut<ambition::world::collision::MovingPlatformSet>,
) {
    let _player = setup::simulation_world(
        &mut commands,
        ambition::platformer::lifecycle::SessionSpawnScope::UNSCOPED,
        setup::SimulationSetup {
            world: &world,
            room_set: &room_set,
            ldtk_index: &ldtk_index,
            editable_abilities: &editable_abilities,
            editable_tuning: &editable_tuning,
            starting_character: &starting_character,
            character_catalog: &character_catalog,
            character_roster: &character_roster,
            boss_catalog: &boss_catalog,
            default_character_id: ambition_content::character_catalog::PLAYABLE_ROSTER[0],
            sandbox_data_asset: sandbox_data_asset.as_deref(),
            sandbox_asset_collection: sandbox_asset_collection.as_deref(),
            asset_server: &asset_server,
        },
    );
    platform_set.0 =
        ambition::actors::world::platforms::moving_platforms_for_room(room_set.active_spec());
    // `PlayerSafetyState::last_safe_pos` is initialized by the player
    // bundle to the player's spawn position (which is `world.0.spawn`),
    // so we don't need to overwrite it here. See
    // `ambition::actors::avatar::PlayerSimulationBundle::new`.
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
    catalogs: PresentationCatalogs,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    scene_entities: Res<SceneEntities>,
    ui_fonts: Option<Res<ui_fonts::UiFonts>>,
    quality: Option<Res<ambition::render::quality::ResolvedVisualQuality>>,
    mut profiler: ResMut<ambition::dev_tools::profiling::StartupProfiler>,
) {
    // `std::time::Instant::now()` panics on `wasm32-unknown-unknown`
    // with "time not implemented on this platform". Gate the per-step
    // wall-clock breakdown on non-wasm; the wasm build profiles via
    // browser devtools (see docs/recipes/web-build.md).
    #[cfg(not(target_arch = "wasm32"))]
    let t0 = std::time::Instant::now();
    let game_assets = actor_game_assets::load_game_assets(
        &asset_config,
        &catalogs.characters,
        &catalogs.bosses,
        &catalogs.assets,
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
            &catalogs.assets,
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
        // `ambition::actors::profiling`). The presentation world still spawns.
        let _ = &profiler; // silence unused-resource warning
        scene_setup::presentation_world(
            &mut commands,
            &mut audio_sources,
            &asset_server,
            &catalogs.assets,
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
    quality: Res<ambition::render::quality::ResolvedVisualQuality>,
    asset_config: Res<GameAssetConfig>,
    catalogs: PresentationCatalogs,
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
    *game_assets = actor_game_assets::load_game_assets(
        &asset_config,
        &catalogs.characters,
        &catalogs.bosses,
        &catalogs.assets,
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
    catalogs: PresentationCatalogs,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    scene_entities: Res<SceneEntities>,
    quality: Option<Res<ambition::render::quality::ResolvedVisualQuality>>,
) {
    let game_assets = actor_game_assets::load_game_assets(
        &asset_config,
        &catalogs.characters,
        &catalogs.bosses,
        &catalogs.assets,
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
