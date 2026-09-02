use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::AudioSource as KiraAudioSource;

use ambition_platformer2d::actors::assets::game_assets as actor_game_assets;
use ambition_platformer2d::world::rooms as world_rooms;

use ambition_platformer2d::actors::session::setup;
use ambition_platformer2d::dev_tools::dev_tools::EditableAbilitySet;
use ambition_platformer2d::engine_core::RoomGeometry;
use ambition_platformer2d::persistence::settings::TextureResolutionScale;
use ambition_platformer2d::sprite_sheet::game_assets::{self, GameAssetConfig};

use super::scene_setup;

/// App-local authored catalogs consumed together by presentation asset loading.
/// Grouping them keeps Bevy system signatures below the function-parameter
/// implementation limit while preserving explicit authority.
#[derive(SystemParam)]
pub(crate) struct PresentationCatalogs<'w> {
    characters:
        Res<'w, ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>,
    /// Provider-authored sheets (U1). Grouped with the catalogs because it is
    /// the same question — what did this app's providers declare — asked about
    /// art instead of identity.
    sheets: Res<'w, ambition_platformer2d::character::AuthoredSheets>,
    bosses: Res<'w, ambition_platformer2d::boss_encounter::BossCatalog>,
    assets:
        Res<'w, ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog>,
}

/// The three App-installed authorities room construction reads: how authored
/// placements lower, what content stages into a room, and which construction
/// recipes exist. Grouped for the same reason [`PresentationCatalogs`] is —
/// Bevy's system-parameter limit — and they belong together anyway.
#[derive(SystemParam)]
pub(crate) struct RoomConstructionAuthorities<'w> {
    placement_lowering:
        Res<'w, ambition_platformer2d::actors::world::placements::PlacementLoweringRegistry>,
    content_staging: Res<'w, ambition_platformer2d::actors::features::RoomContentStagingRegistry>,
    recipes: Res<'w, ambition_platformer2d::actors::construction::ActorConstructionRegistry>,
    /// What a DEVELOPER has forced every authored actor's brain to.
    ///
    /// ⛔ `Option`, and its absence is the ordinary case: a composition with no
    /// developer tools installs no such resource, and "the author decides" is
    /// what an unset environment variable has always meant. It rides with the
    /// other construction authorities because it IS one — lowering consults it
    /// while building a brain, and it used to do that by calling into
    /// `ambition_dev_tools` from inside the actor kernel.
    forced_brains:
        Option<Res<'w, ambition_platformer2d::characters::brain::AuthoredBrainOverride>>,
}

/// Who this app's characters ARE, in one parameter.
///
/// The catalog is the legacy cast's authority, the prepared registry is the
/// registered cast's, the sheets say what art they may reach, and the roster
/// says what a hostile one is built from. Four questions about the same subject,
/// and grouping them is what keeps this system under Bevy's 16-parameter limit —
/// which it went over the moment the prepared registry joined.
#[derive(SystemParam)]
pub(crate) struct CharacterAuthorities<'w> {
    catalog: Res<'w, ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>,
    /// `None` for a composition that registers no characters — the ordinary
    /// case, not a degraded one.
    prepared:
        Option<Res<'w, ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>>,
    sheets: Res<'w, ambition_platformer2d::character::AuthoredSheets>,
    brain_profiles: Option<
        Res<'w, ambition_platformer2d::characters::actor::character_catalog::BrainProfileRegistry>,
    >,
}

/// Sim-only startup. Calls `ambition_platformer2d::actors::session::setup::simulation_world` to spawn the
/// LdtkWorldBundle and the player entity (with gameplay-essential components
/// but no Sprite). The presentation startup system discovers the home avatar by
/// its `PrimaryPlayer` marker and spawns the HUD/quest text as session-scoped,
/// marker-tagged entities.
pub(super) fn setup_simulation_system(
    mut commands: Commands,
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    room_set: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<world_rooms::RoomSet>,
    active_tuning: Res<ambition_platformer2d::engine_core::ActiveMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    initial_body: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
        ambition_platformer2d::actors::avatar::InitialBodyPolicy,
    >,
    characters: CharacterAuthorities,
    boss_catalog: Res<ambition_platformer2d::boss_encounter::BossCatalog>,
    construction: RoomConstructionAuthorities,
    mut platform_set: ResMut<ambition_platformer2d::world::collision::MovingPlatformSet>,
) {
    let _player = setup::simulation_world(
        &mut commands,
        ambition_platformer2d::platformer::lifecycle::SessionSpawnScope::UNSCOPED,
        setup::SimulationSetup {
            world: &world,
            room_set: &room_set,
            // The CALLER converts: who edits the set is a developer
            // facility, and construction needs only the set.
            fallback_abilities: editable_abilities.as_engine(),
            tuning: &active_tuning,
            initial_body: &initial_body,
            character_catalog: &characters.catalog,
            prepared_characters: characters.prepared.as_deref(),
            authored_sheets: &characters.sheets,
            placement_lowering: &construction.placement_lowering,
            content_staging: &construction.content_staging,
            // Direct entry builds its session root at plugin-build time rather
            // than through provider activation, so no prepared-content
            // generation is available to state here — but the CAST is, two
            // lines above, and this road was handing over neither it nor the
            // published policies.
            construction:
                ambition_platformer2d::actors::features::ActorConstructionContext::for_room_construction(
                    &construction.recipes,
                    Default::default(),
                    None,
                    characters.prepared.as_deref(),
                    characters.brain_profiles.as_deref(),
                    // Direct entry builds the world at plugin-build time; no
                    // occurrence of anything exists yet to have a disposition.
                    None,
                    construction.forced_brains.as_deref(),
                ),
            boss_catalog: &boss_catalog,
            default_character_id: ambition_content::character_catalog::PLAYABLE_ROSTER[0],
        },
    );
    platform_set.0 =
        ambition_platformer2d::world::platforms::moving_platforms_for_room(room_set.active_spec());
    // `PlayerSafetyState::last_safe_pos` is initialized by the player
    // bundle to the player's spawn position (which is `world.0.spawn`),
    // so we don't need to overwrite it here. See
    // `ambition_platformer2d::actors::avatar::PlayerSimulationBundle::new`.
}

/// HOST-mode presentation startup: cameras, `GameAssets`, and the audio library.
/// No world visuals, no HUD, no player — those are SESSION-owned and spawn per activation
/// (`shell_host::ambition_activate_session_visuals`). The launcher/title route
/// therefore renders over an empty stage with zero gameplay entities.
#[cfg(feature = "audio")]
pub(crate) fn setup_host_presentation_system(
    mut commands: Commands,
    prepared_world: Res<ambition_content::provider::AmbitionPreparedWorld>,
    sfx_registry: Res<ambition_platformer2d::content::SfxRegistry>,
    audio_catalog: Res<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>,
    catalogs: PresentationCatalogs,
    hosted: Option<Res<super::shell_host::AmbitionShellHosted>>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    quality: Option<Res<ambition_platformer2d::render::quality::ResolvedVisualQuality>>,
    world_manifest: Res<ambition_platformer2d::world::world_manifest::WorldManifest>,
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
        ambition_platformer2d::actors::assets::platformer_assets::build_sandbox_catalog_with(
            &asset_config,
            &catalogs.characters,
            &catalogs.bosses,
            &music_registry,
            &world_manifest,
            |manifest| {
                ambition_content::intro::sprites::extend_with_intro_sprite_entries(
                    manifest,
                    &asset_config.sprite_folder,
                );
            },
        )
    });
    let frozen_catalog: &ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog =
        &catalogs.assets;
    let asset_catalog = rebuilt_catalog.as_ref().unwrap_or(frozen_catalog);

    let game_assets = actor_game_assets::load_game_assets(
        &asset_config,
        &catalogs.characters,
        &catalogs.sheets,
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
/// The bank is process-wide *storage*; authority is provider-relative. This registers the
/// bank's ids in the App-local [`SfxBankRegistry`] under the owning provider (Ambition — the
/// superset that packs every shared asset), so the session bridge authorizes an Ambition
/// session over the whole bank while other providers get none of it. Retries until it succeeds
/// once (the bank may land asynchronously).
#[cfg(feature = "audio")]
pub(crate) fn publish_resident_sfx_bank_authority(
    bank: Option<Res<ambition_platformer2d::audio::SfxBankResource>>,
    mut registry: ResMut<ambition_platformer2d::audio::catalog::SfxBankRegistry>,
    mut selection: ResMut<ambition_platformer2d::audio::selection::ActiveAudioSelection>,
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
    audio_catalog: Res<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_config: Res<GameAssetConfig>,
    quality: Option<Res<ambition_platformer2d::render::quality::ResolvedVisualQuality>>,
    world_manifest: Res<ambition_platformer2d::world::world_manifest::WorldManifest>,
) {
    // Same provider-sprite composition as the audio variant: rebuild the sandbox
    // asset catalog from the merged character catalog so host-launched Sanic and
    // Mary-O actors resolve their sheets. The music registry only supplies
    // catalog music-track rows here (no playback in a headless build).
    let music_registry = audio_catalog
        .combined_music_registry(ambition_content::AMBITION_CONTENT_PROVIDER)
        .unwrap_or_else(|error| panic!("host asset composition failed: {error}"));
    let rebuilt_catalog = hosted.is_some().then(|| {
        ambition_platformer2d::actors::assets::platformer_assets::build_sandbox_catalog_with(
            &asset_config,
            &catalogs.characters,
            &catalogs.bosses,
            &music_registry,
            &world_manifest,
            |manifest| {
                ambition_content::intro::sprites::extend_with_intro_sprite_entries(
                    manifest,
                    &asset_config.sprite_folder,
                );
            },
        )
    });
    let frozen_catalog: &ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog =
        &catalogs.assets;
    let asset_catalog = rebuilt_catalog.as_ref().unwrap_or(frozen_catalog);
    let game_assets = actor_game_assets::load_game_assets(
        &asset_config,
        &catalogs.characters,
        &catalogs.sheets,
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

/// Rebuild the asset families that have no residency model of their own —
/// entity sprites, boss sheets, parallax layers — for a confirmed quality change.
///
/// the CHARACTER sheet table is deliberately NOT rebuilt here. It has an
/// owner: the engine's character runtime retires each stale realization back to
/// `Declared` and re-demands it
/// (`character_runtime::converge_character_residency_to_active_quality`), which
/// is what makes a body already on screen converge instead of waiting for a room
/// load. Replacing the table wholesale defeated exactly that — a fresh table has
/// 140 declarations and zero residents, so there is nothing left to *notice* is
/// stale and nothing re-demands anything.
///
/// and it took two other things with it that nobody ever put back: the per-`Prop.kind` sheets
/// and the realizations a host published itself (`publish_under`, the intro's NPCs).
pub(crate) fn reload_visual_quality_assets_on_scale_change(
    quality: Res<ambition_platformer2d::render::quality::ResolvedVisualQuality>,
    asset_config: Res<GameAssetConfig>,
    catalogs: PresentationCatalogs,
    asset_server: Res<AssetServer>,
    room_set: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<world_rooms::RoomSet>,
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
    let rebuilt = actor_game_assets::load_game_assets(
        &asset_config,
        &catalogs.characters,
        &catalogs.sheets,
        &catalogs.bosses,
        &catalogs.assets,
        &asset_server,
        &mut atlas_layouts,
        &room_set.active_spec().metadata,
        Some(&quality.budget),
    );
    *game_assets = game_assets::GameAssets {
        // Owned by the engine's quality transition; see above.
        characters: std::mem::take(&mut game_assets.characters),
        ..rebuilt
    };
}
