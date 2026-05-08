#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::input_systems::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::update::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;

// `GameWorld`, `SandboxRuntime`, and the time-scale ramp helper `move_toward`
// have moved to `crate::lib` (`ambition_sandbox`) so both binaries can share
// them. They are re-imported above through `use ambition_sandbox::*;`.

/// Sim-only startup. Calls `crate::setup::simulation_world` to spawn the
/// LdtkWorldBundle, build the SandboxRuntime resource, and spawn the player
/// entity (with gameplay-essential components but no Sprite). Inserts
/// SceneEntities with `hud: Entity::PLACEHOLDER`; the presentation startup
/// system later overwrites that with the real HUD entity.
pub(super) fn setup_simulation_system(
    mut commands: Commands,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data_asset: Option<Res<data::SandboxDataAsset>>,
    ldtk_asset: Option<Res<ldtk_world::SandboxLdtkAsset>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
    asset_server: Res<AssetServer>,
    ldtk_index: Res<ldtk_world::LdtkRuntimeIndex>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    sandbox_data: Res<data::SandboxDataSpec>,
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
) {
    let _player = setup::simulation_world(
        &mut commands,
        setup::SimulationSetup {
            world: &world,
            room_set: &room_set,
            ldtk_index: &ldtk_index,
            sandbox_data: &sandbox_data,
            editable_abilities: &editable_abilities,
            editable_tuning: &editable_tuning,
            physics_settings: *physics_settings,
            sandbox_data_asset: sandbox_data_asset.as_deref(),
            ldtk_asset: ldtk_asset.as_deref(),
            sandbox_asset_collection: sandbox_asset_collection.as_deref(),
            asset_server: &asset_server,
        },
    );
}

/// Presentation startup. Runs after `setup_simulation_system` so the
/// SceneEntities resource (with player Entity) is visible. Adds the
/// player's Sprite, spawns Camera2d, room visuals, HUD text, generated
/// Kira audio library, and overwrites SceneEntities to fill in the HUD
/// entity.
#[cfg(feature = "audio")]
pub(super) fn setup_presentation_system(
    mut commands: Commands,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data: Res<data::SandboxDataSpec>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    scene_entities: Res<SceneEntities>,
    ui_fonts: Option<Res<ui_fonts::UiFonts>>,
    mut profiler: ResMut<crate::profiling::StartupProfiler>,
) {
    let t0 = std::time::Instant::now();
    let game_assets =
        game_assets::load_game_assets(&asset_config, &asset_server, &mut atlas_layouts);
    let t_assets = t0.elapsed().as_secs_f32() * 1000.0;
    profiler.marks.push((
        "setup_presentation::load_game_assets",
        std::time::Instant::now(),
    ));
    let t1 = std::time::Instant::now();
    setup::presentation_world(
        &mut commands,
        &mut audio_sources,
        &asset_server,
        setup::PresentationSetup {
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
    commands.insert_resource(game_assets);
}

#[cfg(not(feature = "audio"))]
pub(super) fn setup_presentation_system(
    mut commands: Commands,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    sandbox_data: Res<data::SandboxDataSpec>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    scene_entities: Res<SceneEntities>,
) {
    let game_assets =
        game_assets::load_game_assets(&asset_config, &asset_server, &mut atlas_layouts);
    setup::presentation_world(
        &mut commands,
        setup::PresentationSetup {
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
