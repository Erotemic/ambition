use bevy::prelude::*;

use ambition_content::world_preparation::{StartRoomMustResolve, StartRoomOverride};
use ambition_platformer2d::combat::feel::Platformer2dFeelTuningMonolith;
use ambition_platformer2d::sim as physics;
use ambition_platformer2d::dev_tools::dev_tools::{
    DeveloperTools, EditableAbilitySet, EditableMovementTuning, EditablePlayerStats,
};
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::ldtk_map as ldtk_world;

use super::cli::cli_start_room_arg;

pub fn init_sandbox_resources(app: &mut App) {
    let sandbox_data = ambition_content::gameplay_defaults::load();
    if !super::hub_games::installs("ambition") {
        init_host_resources_without_ambition(app);
        insert_host_resources(app, sandbox_data);
        return;
    }
    // A flag a human typed is a request, not a preference: it must resolve.
    // A programmatic override, when there is one, names the room.
    if let Some(start_room) = cli_start_room_arg() {
        app.insert_resource(StartRoomMustResolve);
        if !app.world().contains_resource::<StartRoomOverride>() {
            app.insert_resource(StartRoomOverride(start_room));
        }
    }
    // Register this provider's authored audio and character fragments into the
    // App-local registries. Boss content is contributed by
    // `AmbitionBossContentPlugin` through the same App-local composition model.
    ambition_content::audio_registries::register(app);
    ambition_content::character_catalog::register(app);
    ambition_content::bosses::register(app);
    // This inserted the same value the content plugin does — idempotent, since
    // both call `ambition_content::worlds::world_manifest()` — but two writers
    // of one global is the shape that has cost this repo a roster, a rebuild, a
    // retirement and a countdown. The provider that OWNS the worlds publishes
    // the declaration; the host reads it.
    //
    // the local value stays: it is threaded BY REFERENCE into every
    // preparation-time reader below (catalog rows, the LDtk load, the room-set
    // conversion, the hot-reload watcher), which run before any schedule and so
    // cannot take a `Res`. That is the K2a shape — no process global — and it is
    // unaffected by who inserts the resource.
    let world_manifest = ambition_content::worlds::world_manifest();

    // Audio lives in its own registries, separate from sandbox tuning and
    // from each other (SFX synthesis vs. generated music pointers).
    let (music_registry, sfx_registry) = {
        let catalogs = app
            .world()
            .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>();
        (
            // No music fragment when the music is not generated yet.
            catalogs
                .music_for(ambition_content::AMBITION_CONTENT_PROVIDER)
                .cloned()
                .unwrap_or_else(super::setup_systems::no_music),
            catalogs
                .sfx_for(ambition_content::AMBITION_CONTENT_PROVIDER)
                .expect("Ambition SFX fragment registered")
                .clone(),
        )
    };
    // K2b edit 4: the direct-entry AUDIO branch is gone with its host.
    //
    // It selected the active audio authority statically at composition, on the
    // argument that a direct-entry process runs exactly one provider. There is
    // no direct-entry process any more: `select_shell_audio_context`
    // (`game_shell/src/session.rs`) owns selection AND `SfxEmissionContext` on
    // activation, and it did even while this branch existed — the branch simply
    // ran first in a composition the shell never touched.
    //
    // the registries above are still read here, and still needed: they are
    // what the provider's audio fragment is published FROM. Only the static
    // selection went.
    let character_catalog = app
        .world()
        .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>()
        .clone();
    let boss_catalog = app
        .world()
        .resource::<ambition_platformer2d::boss_encounter::BossCatalog>()
        .clone();
    // Build the singleton Platformer2dAssetCatalog before anything else asks
    // it for a path. Every asset path/source policy in the visible
    // sandbox flows through this — LDtk, SFX bank, fonts, sprites,
    // music. Consumes the music registry so music-track ids land in the
    // catalog.
    let asset_config = app
        .world()
        .get_resource::<ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig>()
        .cloned()
        .unwrap_or_default();
    let sandbox_catalog =
        ambition_platformer2d::assets::platformer_assets::build_platformer2d_asset_catalog(
            app.world().resource::<ambition_platformer2d::asset_manager::content_library::ContentLibraries>(),            &asset_config,
            &character_catalog,
            &boss_catalog,
            &music_registry,
            &world_manifest,
        );
    app.insert_resource(ldtk_world::LdtkRuntimeSpineStats::default())
        .insert_resource(ldtk_world::LdtkRuntimeSpineIndex::default())
        .insert_resource(music_registry)
        .insert_resource(sfx_registry)
        .insert_resource(sandbox_catalog);
    insert_host_resources(app, sandbox_data);
}

/// The host's own resources: tuning, settings, input and developer state. A
/// hub installs these whatever games it carries.
fn insert_host_resources(
    app: &mut App,
    sandbox_data: ambition_platformer2d::assets::gameplay_defaults::Platformer2dGameplayDefaults,
) {
    let feel: Platformer2dFeelTuningMonolith = sandbox_data.feel;
    let editable_abilities = EditableAbilitySet::from(sandbox_data.abilities);
    let editable_tuning = EditableMovementTuning::from(sandbox_data.tuning);
    // The simulation's authority, seeded from the same authored value.
    let active_tuning =
        ambition_platformer2d::engine_core::ActiveMovementTuning(sandbox_data.tuning);
    app
        // `PhysicsSandboxSettings` must exist before the plugins that would
        // otherwise provide it: where Ambition's provider installs
        // `AmbitionPhysicsPlugin` (the App presents) it re-inserts the default
        // (harmless — same default), but headless does not load that plugin.
        //
        // ⚠ THE READER NAMED HERE WAS `setup_simulation_system`, DELETED IN
        // `d3135def0`, AND THE COMMENT KEPT CITING IT UNTIL 2026-09-19. The
        // live readers are `dev_runtime`'s editable panel and
        // `ambition_render`'s platformer presentation. ⛔ Which means the
        // NECESSITY above is no longer derived from anything measured: both of
        // those also `init_resource` it, and so does the engine group
        // (`sim_core_resources.rs`), so whether this insert is still load-
        // bearing for a headless composition is an open question rather than
        // the settled fact the old sentence read as. Left in place — removing
        // it is a change that needs a headless run, not a prose repair.
        .insert_resource(physics::PhysicsSandboxSettings::default())
        .insert_resource(sandbox_data)
        .insert_resource(DeveloperTools::default())
        .insert_resource(EditablePlayerStats::default())
        // The feel values the defaults file states over the engine's.
        .insert_resource(feel)
        // ⚠ `OwnedItems` IS NOT INSERTED HERE ANY MORE. The reason this line
        // gave — *"headless `Platformer2dSimHarness` runs quest reward systems
        // without loading `add_presentation_plugins`, so the resource must exist
        // before the first Update tick"* — is still true, and it is now answered
        // by `AmbitionContentPlugin`, which Ambition's provider installs in
        // every composition that selects Ambition. Two build-time inserts of the same value is one
        // authority too many; see the note there.
        .insert_resource(editable_abilities)
        // The neutral authority the SIMULATION reads, seeded from authored
        // content. `editable_tuning` beside it is the inspector's reflected
        // mirror; in a developer build `publish_editable_movement_tuning` pushes
        // its edits into this one. Nothing in the sim reads the mirror.
        .insert_resource(active_tuning)
        .insert_resource(editable_tuning)
        // Sim/presentation seam for input (ADR 0012): the sim reads
        // `Res<ControlFrame>`. Visible builds populate it from leafwing in
        // `populate_control_frame_from_actions`; headless tests can write
        // directly. Default = no buttons pressed = idle player.
        .init_resource::<ControlFrame>()
        // Aggregate user settings (video/audio/controls/gameplay).
        // Mutated by the pause menu; read by audio/video/gameplay
        // systems and the input deadzone/hysteresis filter.
        .insert_resource(ambition_platformer2d::persistence::settings::UserSettings::default());
}

/// A hub without Ambition: no Ambition world, cast, bosses or validation. The
/// asset catalog is built from the catalogs present, with no LDtk world;
/// `setup_host_presentation_system` rebuilds it when the providers have
/// registered.
fn init_host_resources_without_ambition(app: &mut App) {
    let character_catalog = app
        .world()
        .get_resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>()
        .cloned()
        .unwrap_or_else(ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog::empty);
    let boss_catalog = app
        .world()
        .get_resource::<ambition_platformer2d::boss_encounter::BossCatalog>()
        .cloned()
        .unwrap_or_default();
    let asset_config = app
        .world()
        .get_resource::<ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig>()
        .cloned()
        .unwrap_or_default();
    // The frontend (title music, menu sounds) is authored in Ambition's audio
    // catalog, so a hub without Ambition still registers that catalog. It is
    // data: it adds no system.
    ambition_content::audio_registries::register(app);
    let (music_registry, sfx_registry) = {
        let catalogs = app
            .world()
            .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>();
        (
            // No music fragment when the music is not generated yet.
            catalogs
                .music_for(ambition_content::AMBITION_CONTENT_PROVIDER)
                .cloned()
                .unwrap_or_else(super::setup_systems::no_music),
            catalogs
                .sfx_for(ambition_content::AMBITION_CONTENT_PROVIDER)
                .expect("Ambition SFX fragment registered")
                .clone(),
        )
    };
    let sandbox_catalog =
        ambition_platformer2d::assets::platformer_assets::build_sandbox_catalog_with(
            app.world().resource::<ambition_platformer2d::asset_manager::content_library::ContentLibraries>(),            &asset_config,
            &character_catalog,
            &boss_catalog,
            &music_registry,
            &ambition_platformer2d::world::world_manifest::WorldManifest::default(),
            |_| {},
        );
    app.insert_resource(sandbox_catalog)
        .insert_resource(music_registry)
        .insert_resource(sfx_registry)
        .insert_resource(ldtk_world::LdtkRuntimeSpineStats::default())
        .insert_resource(ldtk_world::LdtkRuntimeSpineIndex::default());
}

// what replaced it is `shell_host::compose_ambition_gameplay_host`: the shell host booted straight
// to the gameplay route, which is what direct entry was always supposed to be.
//
// `SessionScopeId(0)` went with it. It was a placeholder minted because a
// build-time root has no activation to mint one from, and `session_world_entity`
// panicked with "more than one canonical SessionRoot exists" whenever a
// composition managed to have both.
