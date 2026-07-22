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
use ambition::dev_tools::dev_tools::EditableAbilitySet;
use ambition::engine_core::RoomGeometry;
use ambition::persistence::settings::TextureResolutionScale;
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

/// The three App-installed authorities room construction reads: how authored
/// placements lower, what content stages into a room, and which construction
/// recipes exist. Grouped for the same reason [`PresentationCatalogs`] is —
/// Bevy's system-parameter limit — and they belong together anyway.
#[derive(SystemParam)]
pub(crate) struct RoomConstructionAuthorities<'w> {
    placement_lowering: Res<'w, ambition::actors::world::placements::PlacementLoweringRegistry>,
    content_staging: Res<'w, ambition::actors::features::RoomContentStagingRegistry>,
    recipes: Res<'w, ambition::actors::construction::ActorConstructionRegistry>,
}

/// Sim-only startup. Calls `ambition::actors::session::setup::simulation_world` to spawn the
/// LdtkWorldBundle and the player entity (with gameplay-essential components
/// but no Sprite). The presentation startup system discovers the home avatar by
/// its `PrimaryPlayer` marker and spawns the HUD/quest text as session-scoped,
/// marker-tagged entities.
pub(super) fn setup_simulation_system(
    mut commands: Commands,
    world: ambition::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<rooms::RoomSet>,
    sandbox_data_asset: Option<Res<data::SandboxDataAsset>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
    asset_server: Res<AssetServer>,
    ldtk_index: ambition::platformer::lifecycle::SessionWorldRef<ldtk_world::LdtkRuntimeIndex>,
    active_tuning: Res<ambition::engine_core::ActiveMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    starting_character: ambition::platformer::lifecycle::SessionWorldRef<
        ambition::actors::avatar::StartingCharacter,
    >,
    character_catalog: Res<ambition::characters::actor::character_catalog::CharacterCatalog>,
    character_roster: Res<ambition::actors::features::CharacterRoster>,
    boss_catalog: Res<ambition::actors::boss_encounter::BossCatalog>,
    construction: RoomConstructionAuthorities,
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
            tuning: &active_tuning,
            starting_character: &starting_character,
            character_catalog: &character_catalog,
            character_roster: &character_roster,
            placement_lowering: &construction.placement_lowering,
            content_staging: &construction.content_staging,
            // Direct entry builds its session root at plugin-build time rather
            // than through provider activation, so no prepared-content
            // generation is available to state here.
            construction: ambition::actors::features::ActorConstructionContext::new(
                &construction.recipes,
                Default::default(),
            ),
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

/// Presentation startup. Runs after `setup_simulation_system` so the home
/// avatar (its `PrimaryPlayer` marker) is queryable. Adds the player's Sprite,
/// spawns Camera2d, room visuals, and the marker-tagged HUD/quest text, plus the
/// generated Kira audio library.
#[cfg(feature = "audio")]
pub(crate) fn setup_presentation_system(
    mut commands: Commands,
    world: ambition::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<rooms::RoomSet>,
    music_registry: Res<data::MusicRegistry>,
    sfx_registry: Res<data::SfxRegistry>,
    catalogs: PresentationCatalogs,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
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
        );
    }
    commands.insert_resource(game_assets);
}

/// HOST-mode presentation startup: cameras, `GameAssets`, and the audio library.
/// No world visuals, no HUD, no player — those are SESSION-owned and spawn per activation
/// (`shell_host::ambition_activate_session_visuals`). The launcher/title route
/// therefore renders over an empty stage with zero gameplay entities.
#[cfg(feature = "audio")]
pub(crate) fn setup_host_presentation_system(
    mut commands: Commands,
    prepared_world: Res<ambition_content::provider::AmbitionPreparedWorld>,
    sfx_registry: Res<data::SfxRegistry>,
    audio_catalog: Res<ambition::audio::catalog::AudioCatalogRegistry>,
    catalogs: PresentationCatalogs,
    hosted: Option<Res<super::shell_host::AmbitionShellHosted>>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    quality: Option<Res<ambition::render::quality::ResolvedVisualQuality>>,
    world_manifest: Res<ambition::actors::ldtk_world::WorldManifest>,
) {
    // The host-resident music library must resolve EVERY linked provider's
    // authored tracks — not just Ambition's — so a Sanic or Mary-O session's
    // music plays through the same director in this shared host. Each track
    // keeps its own `asset_path`, so the sandbox-catalog path resolver in
    // `install_audio_library` still blesses Ambition's paths and falls back to
    // the provider-authored path for the others. A duplicate track id across
    // providers is a deterministic composition failure here.
    let music_registry = audio_catalog
        .combined_music_registry(ambition_content::AMBITION_CONTENT_PROVIDER)
        .unwrap_or_else(|error| panic!("host audio composition failed: {error}"));

    // As the multi-game host, the sandbox asset catalog built at startup
    // (`init_sandbox_resources`) predates the Sanic/Mary-O provider
    // registrations, so it carries only Ambition's character-sprite rows and
    // their actors would fall back to the colored-rectangle placeholder. Rebuild
    // it from the now-fully-merged character catalog so EVERY provider's sprites
    // resolve through the one shared `GameAssets` path — with no per-provider
    // host code. Direct-entry apps register only Ambition, so their frozen
    // catalog is already complete and no rebuild happens.
    let rebuilt_catalog = hosted.is_some().then(|| {
        ambition::actors::assets::sandbox_assets::build_sandbox_catalog_with(
            &asset_config,
            &catalogs.characters,
            &catalogs.bosses,
            &music_registry,
            &world_manifest,
            |manifest| {
                ambition_content::intro::sprites::extend_with_intro_sprite_entries(
                    manifest,
                    &asset_config.sprite_folder,
                    &catalogs.characters,
                );
            },
        )
    });
    let frozen_catalog: &ambition::asset_manager::sandbox_assets::SandboxAssetCatalog =
        &catalogs.assets;
    let asset_catalog = rebuilt_catalog.as_ref().unwrap_or(frozen_catalog);

    let game_assets = actor_game_assets::load_game_assets(
        &asset_config,
        &catalogs.characters,
        &catalogs.bosses,
        asset_catalog,
        &asset_server,
        &mut atlas_layouts,
        &prepared_world.room_set.active_spec().metadata,
        quality.as_deref().map(|q| &q.budget),
    );
    scene_setup::host_presentation_scaffold(&mut commands);
    scene_setup::install_audio_library(
        &mut commands,
        &mut audio_sources,
        &asset_server,
        asset_catalog,
        &music_registry,
        &sfx_registry,
    );
    commands.insert_resource(game_assets);
    // Publish the merged superset catalog so gameplay-time sprite/asset lookups
    // (any provider's actors) resolve against provider rows too.
    if let Some(catalog) = rebuilt_catalog {
        commands.insert_resource(catalog);
    }
}

/// Once the resident SFX bank is loaded, publish its ids as Ambition's
/// provider-relative SFX authority.
///
/// The bank is process-wide *storage*; authority is provider-relative. This
/// registers the bank's ids in the App-local [`SfxBankRegistry`] under the
/// owning provider (Ambition — the superset that packs every shared asset), so
/// the session bridge authorizes an Ambition session over the whole bank while
/// other providers get none of it. For a direct-entry host that statically
/// selected Ambition *before* the bank finished loading, it also folds the ids
/// into the live selection so Ambition's open-ended `Play { id }` sounds
/// resolve. Retries until it succeeds once (the bank may land asynchronously).
#[cfg(feature = "audio")]
pub(crate) fn publish_resident_sfx_bank_authority(
    bank: Option<Res<ambition::audio::SfxBankResource>>,
    mut registry: ResMut<ambition::audio::catalog::SfxBankRegistry>,
    mut selection: ResMut<ambition::audio::selection::ActiveAudioSelection>,
    mut published: Local<bool>,
) {
    if *published {
        return;
    }
    let Some(bank) = bank else {
        return;
    };
    let fingerprints = bank.fingerprints_for(ambition_content::AMBITION_CONTENT_PROVIDER);
    if fingerprints.is_empty() {
        return;
    }
    let ids: std::collections::BTreeSet<_> = fingerprints.keys().copied().collect();
    if let Err(error) = registry.register(ambition_content::AMBITION_CONTENT_PROVIDER, fingerprints)
    {
        warn!("resident sfx bank registration failed: {error}");
    }
    // Refresh whichever live context actually belongs to Ambition. This is
    // identity-safe for gameplay, direct entry, and the Ambition frontend; a
    // bank arriving late for one provider cannot expand another provider's
    // authority.
    selection.refresh_provider_sfx_ids(ambition_content::AMBITION_CONTENT_PROVIDER, ids);
    *published = true;
}

#[cfg(not(feature = "audio"))]
pub(crate) fn setup_host_presentation_system(
    mut commands: Commands,
    prepared_world: Res<ambition_content::provider::AmbitionPreparedWorld>,
    catalogs: PresentationCatalogs,
    hosted: Option<Res<super::shell_host::AmbitionShellHosted>>,
    audio_catalog: Res<ambition::audio::catalog::AudioCatalogRegistry>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    quality: Option<Res<ambition::render::quality::ResolvedVisualQuality>>,
    world_manifest: Res<ambition::actors::ldtk_world::WorldManifest>,
) {
    // Same provider-sprite composition as the audio variant: rebuild the sandbox
    // asset catalog from the merged character catalog so host-launched Sanic and
    // Mary-O actors resolve their sheets. The music registry only supplies
    // catalog music-track rows here (no playback in a headless build).
    let music_registry = audio_catalog
        .combined_music_registry(ambition_content::AMBITION_CONTENT_PROVIDER)
        .unwrap_or_else(|error| panic!("host asset composition failed: {error}"));
    let rebuilt_catalog = hosted.is_some().then(|| {
        ambition::actors::assets::sandbox_assets::build_sandbox_catalog_with(
            &asset_config,
            &catalogs.characters,
            &catalogs.bosses,
            &music_registry,
            &world_manifest,
            |manifest| {
                ambition_content::intro::sprites::extend_with_intro_sprite_entries(
                    manifest,
                    &asset_config.sprite_folder,
                    &catalogs.characters,
                );
            },
        )
    });
    let frozen_catalog: &ambition::asset_manager::sandbox_assets::SandboxAssetCatalog =
        &catalogs.assets;
    let asset_catalog = rebuilt_catalog.as_ref().unwrap_or(frozen_catalog);
    let game_assets = actor_game_assets::load_game_assets(
        &asset_config,
        &catalogs.characters,
        &catalogs.bosses,
        asset_catalog,
        &asset_server,
        &mut atlas_layouts,
        &prepared_world.room_set.active_spec().metadata,
        quality.as_deref().map(|q| &q.budget),
    );
    scene_setup::host_presentation_scaffold(&mut commands);
    commands.insert_resource(game_assets);
    if let Some(catalog) = rebuilt_catalog {
        commands.insert_resource(catalog);
    }
}

pub(crate) fn reload_visual_quality_assets_on_scale_change(
    quality: Res<ambition::render::quality::ResolvedVisualQuality>,
    asset_config: Res<GameAssetConfig>,
    catalogs: PresentationCatalogs,
    asset_server: Res<AssetServer>,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<rooms::RoomSet>,
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
    world: ambition::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    room_set: ambition::platformer::lifecycle::SessionWorldRef<rooms::RoomSet>,
    catalogs: PresentationCatalogs,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
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
    );
    commands.insert_resource(game_assets);
}
