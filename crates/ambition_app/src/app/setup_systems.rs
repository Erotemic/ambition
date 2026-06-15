#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::player_tick::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::world_flow::*;
use super::scene_setup;
#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use ambition_sandbox::app::*;

// `GameWorld` and the time-scale ramp helper `move_toward` live in
// `ambition_sandbox::lib` (`ambition_sandbox`) and are re-imported above through
// `use ambition_sandbox::*;`.

/// Sim-only startup. Calls `ambition_sandbox::runtime::setup::simulation_world` to spawn the
/// LdtkWorldBundle and the player entity (with gameplay-essential components
/// but no Sprite). Inserts SceneEntities with `hud: Entity::PLACEHOLDER`;
/// the presentation startup system later overwrites that with the real HUD entity.
pub(super) fn setup_simulation_system(
    mut commands: Commands,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data_asset: Option<Res<data::SandboxDataAsset>>,
    ldtk_asset: Option<Res<ldtk_world::SandboxLdtkAsset>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
    asset_server: Res<AssetServer>,
    ldtk_index: Res<ldtk_world::LdtkRuntimeIndex>,
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    mut platform_set: ResMut<ambition_sandbox::MovingPlatformSet>,
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
        ambition_sandbox::world::platforms::moving_platforms_for_room(room_set.active_spec());
    // `PlayerSafetyState::last_safe_pos` is initialized by the player
    // bundle to the player's spawn position (which is `world.0.spawn`),
    // so we don't need to overwrite it here. See
    // `ambition_sandbox::player::PlayerSimulationBundle::new`.
}

/// Presentation startup. Runs after `setup_simulation_system` so the
/// SceneEntities resource (with player Entity) is visible. Adds the
/// player's Sprite, spawns Camera2d, room visuals, HUD text, generated
/// Kira audio library, and overwrites SceneEntities to fill in the HUD
/// entity.
#[cfg(feature = "audio")]
pub(crate) fn setup_presentation_system(
    mut commands: Commands,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data: Res<data::SandboxDataSpec>,
    sandbox_catalog: Res<ambition_sandbox::assets::sandbox_assets::SandboxAssetCatalog>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    scene_entities: Res<SceneEntities>,
    ui_fonts: Option<Res<ui_fonts::UiFonts>>,
    mut profiler: ResMut<ambition_sandbox::dev::profiling::StartupProfiler>,
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
                sandbox_data: &sandbox_data,
                physics_settings: *physics_settings,
                game_assets: &game_assets,
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
        // `ambition_sandbox::profiling`). The presentation world still spawns.
        let _ = &profiler; // silence unused-resource warning
        scene_setup::presentation_world(
            &mut commands,
            &mut audio_sources,
            &asset_server,
            &sandbox_catalog,
            scene_setup::PresentationSetup {
                world: &world,
                room_set: &room_set,
                sandbox_data: &sandbox_data,
                physics_settings: *physics_settings,
                game_assets: &game_assets,
                ui_fonts: ui_fonts.as_deref(),
            },
            scene_entities.player,
        );
    }
    commands.insert_resource(game_assets);
}

#[cfg(not(feature = "audio"))]
pub(crate) fn setup_presentation_system(
    mut commands: Commands,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data: Res<data::SandboxDataSpec>,
    sandbox_catalog: Res<ambition_sandbox::assets::sandbox_assets::SandboxAssetCatalog>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    scene_entities: Res<SceneEntities>,
) {
    let game_assets = game_assets::load_game_assets(
        &asset_config,
        &sandbox_catalog,
        &asset_server,
        &mut atlas_layouts,
        &room_set.active_spec().metadata,
    );
    scene_setup::presentation_world(
        &mut commands,
        scene_setup::PresentationSetup {
            world: &world,
            room_set: &room_set,
            sandbox_data: &sandbox_data,
            physics_settings: *physics_settings,
            game_assets: &game_assets,
        },
        scene_entities.player,
    );
    commands.insert_resource(game_assets);
}
