use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::{IntGridRendering, LdtkSettings, LevelBackground};

use ambition::actors::ldtk_world;
use ambition::actors::rooms;
use ambition::actors::session::data;
use ambition::actors::time::feel::SandboxFeelTuning;
use ambition::actors::world::physics;
use ambition::dev_tools::dev_tools::{
    DeveloperTools, EditableAbilitySet, EditableMovementTuning, EditablePlayerStats,
};
use ambition::engine_core::RoomGeometry;
use ambition::input::ControlFrame;
use ambition_content::content_validation;

use super::cli::cli_start_room_arg;

/// Programmatic start-room override. SandboxSim and other library
/// callers insert this resource before `init_sandbox_resources` runs;
/// the function consumes it (taking precedence over the
/// `--start-room` CLI flag) so callers do not need to manipulate
/// `std::env::args` to pin a starting room.
#[derive(Resource, Clone, Debug)]
pub struct StartRoomOverride(pub String);

#[cfg(test)]
fn sandbox_init_failed() -> ! {
    panic!("sandbox resource initialization failed; see diagnostics above");
}

#[cfg(not(test))]
fn sandbox_init_failed() -> ! {
    std::process::exit(2);
}

pub fn init_sandbox_resources(app: &mut App) {
    // Register this provider's authored audio and character fragments into the
    // App-local registries. Boss content is contributed by
    // `AmbitionBossContentPlugin` through the same App-local composition model.
    ambition_content::audio_registries::register(app);
    ambition_content::character_catalog::register(app);
    ambition_content::enemy_roster::register(app);
    ambition_content::bosses::register(app);
    ambition_content::worlds::install();

    let sandbox_data = data::SandboxDataSpec::load_embedded();
    // Audio lives in its own registries, separate from sandbox tuning and
    // from each other (SFX synthesis vs. generated music pointers).
    let (music_registry, sfx_registry) = {
        let catalogs = app
            .world()
            .resource::<ambition::audio::catalog::AudioCatalogRegistry>();
        (
            catalogs
                .music_for(ambition_content::AMBITION_CONTENT_PROVIDER)
                .expect("Ambition music fragment registered")
                .clone(),
            catalogs
                .sfx_for(ambition_content::AMBITION_CONTENT_PROVIDER)
                .expect("Ambition SFX fragment registered")
                .clone(),
        )
    };
    // Direct-entry host: this process runs exactly one provider (Ambition), so
    // the active audio authority is selected statically at composition. The
    // shell-routed host instead selects one exact frontend/gameplay audio
    // context through the shell bridge. The title may own its theme and menu
    // cues; retired gameplay contexts may not leak work into it.
    if !app
        .world()
        .contains_resource::<super::shell_host::AmbitionShellHosted>()
    {
        // Bank ids are folded in by `publish_resident_sfx_bank_authority` once
        // the resident bank finishes loading; the cues are authorized here.
        app.insert_resource(ambition::audio::selection::ActiveAudioSelection::selected_direct(
            ambition_content::AMBITION_CONTENT_PROVIDER,
            Some(music_registry.clone()),
            Some(sfx_registry.clone()),
            std::collections::BTreeSet::new(),
        ));
        app.insert_resource(ambition::sfx::SfxEmissionContext::default());
        app.world_mut()
            .resource_mut::<ambition::sfx::SfxEmissionContext>()
            .set(ambition::sfx::AudioContextOwner::Direct);
    }
    let character_catalog = app
        .world()
        .resource::<ambition::characters::actor::character_catalog::CharacterCatalog>()
        .clone();
    let boss_catalog = app
        .world()
        .resource::<ambition::actors::boss_encounter::BossCatalog>()
        .clone();

    // Build the singleton SandboxAssetCatalog before anything else asks
    // it for a path. Every asset path/source policy in the visible
    // sandbox flows through this — LDtk, SFX bank, fonts, sprites,
    // music. Consumes the music registry so music-track ids land in the
    // catalog.
    let asset_config = app
        .world()
        .get_resource::<ambition::sprite_sheet::game_assets::GameAssetConfig>()
        .cloned()
        .unwrap_or_default();
    let sandbox_catalog = ambition::actors::assets::sandbox_assets::build_sandbox_catalog_with(
        &asset_config,
        &character_catalog,
        &boss_catalog,
        &music_registry,
        |manifest| {
            ambition_content::intro::sprites::extend_with_intro_sprite_entries(
                manifest,
                &asset_config.sprite_folder,
                &character_catalog,
            );
        },
    );
    #[cfg(feature = "audio")]
    let sfx_bank_asset_path = sandbox_catalog
        .path_for(&ambition::asset_manager::sandbox_assets::ids::sfx_bank())
        .map(|path| ambition::audio::SfxBankAssetPath::new(
            ambition_content::AMBITION_CONTENT_PROVIDER,
            path,
        ));

    let ldtk_project = match ldtk_world::LdtkProject::load_default(&sandbox_catalog) {
        Ok(project) => project,
        Err(error) => {
            eprintln!("failed to load sandbox LDtk map: {error}");
            sandbox_init_failed();
        }
    };
    let content_report = content_validation::validate_content_graph(
        &music_registry,
        &ldtk_project,
        &character_catalog,
    );
    for warning in &content_report.warnings {
        eprintln!("content validation warning: {warning}");
    }
    if !content_report.is_ok() {
        eprintln!("sandbox content graph failed validation; fix authored content before running:");
        for error in &content_report.errors {
            eprintln!("  - {error}");
        }
        sandbox_init_failed();
    }
    let editable_abilities = EditableAbilitySet::from(sandbox_data.abilities);
    let editable_tuning = EditableMovementTuning::from(sandbox_data.tuning);
    let mut room_set = match ldtk_project.to_room_set() {
        Ok(room_set) => room_set,
        Err(errors) => {
            eprintln!(
                "sandbox LDtk world failed validation; fix the configured map before running:"
            );
            for error in &errors {
                eprintln!("  - {error}");
            }
            sandbox_init_failed();
        }
    };
    // Programmatic override (SandboxSim / library callers) takes
    // precedence over the CLI flag. Either one resolving by id wins;
    // the other is silently ignored. If neither matches, the LDtk
    // project's authored start room stays active.
    let resource_override = app
        .world_mut()
        .remove_resource::<StartRoomOverride>()
        .map(|r| r.0);
    if let Some(start_room) = resource_override.or_else(cli_start_room_arg) {
        if room_set.set_start_by_id(&start_room) {
            eprintln!("[ambition] start room: {start_room}");
        } else {
            eprintln!(
                "[ambition] warning: start-room '{start_room}' did not match any room id/name"
            );
        }
    }
    let ldtk_index = ldtk_world::LdtkRuntimeIndex::from_project(
        &ldtk_project,
        room_set.active_spec().id.clone(),
    );
    let active_world = room_set.active_world().clone();

    // The immutable boot-prepared world data the shell host's Ambition
    // provider clones per activation. Captured here so activation republishes
    // FRESH room state (and the boot-resolved starting character) instead of
    // whatever a previous session left resident.
    app.insert_resource(super::shell_host::AmbitionPreparedWorld {
        room_set: room_set.clone(),
        ldtk_index: ldtk_index.clone(),
        starting_character: app
            .world()
            .get_resource::<ambition::actors::avatar::StartingCharacter>()
            .cloned()
            .unwrap_or_default(),
    });

    app.insert_resource(ldtk_world::SandboxLdtkProject(ldtk_project.clone()))
        .insert_resource(RoomGeometry(active_world))
        .insert_resource(rooms::ActiveRoomMetadata::default())
        .insert_resource(room_set)
        .insert_resource(ldtk_index)
        .insert_resource(ldtk_world::LdtkHotReloadState::from_catalog(
            &sandbox_catalog,
        ))
        .insert_resource(ldtk_world::LdtkRuntimeSpineStats::default())
        .insert_resource(ldtk_world::LdtkRuntimeSpineIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeSolidIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeOneWayIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeDamageIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeSpineParity::default())
        // PhysicsSandboxSettings is read by setup_simulation_system; on the
        // visible binary AmbitionPhysicsPlugin re-inserts the default value
        // (harmless — same default), but headless does not load that plugin
        // (it depends on ScenePlugin), so the resource must be available
        // up front.
        .insert_resource(physics::PhysicsSandboxSettings::default())
        .insert_resource(LdtkSettings {
            // Ambition still renders runtime rooms for now; let bevy_ecs_ldtk
            // own level/entity lifecycle without also drawing LDtk background
            // rectangles behind every level.
            level_background: LevelBackground::Nonexistent,
            // bevy_ecs_ldtk's default `IntGridRendering::Colorful` spawns a
            // colored tile sprite per non-zero IntGrid cell when no tileset
            // is configured (1004 sprites for central_hub_main alone). Those
            // tiles render in raw LDtk world-pixel coordinates from
            // `LdtkWorldBundle`'s default transform, while our compose path
            // (`int_grid_value_to_block` → `spawn_block`) renders in
            // Ambition's centered Bevy frame via `world_to_bevy`. The two
            // frames disagree by ~half-room-width on x, so the plugin's
            // IntGrid output appeared as a duplicated, horizontally-shifted
            // copy of our render. Force the plugin to emit no visual at all
            // for IntGrid cells; the runtime-spine `LdtkSolid` component
            // (our typed authority) is unaffected by this setting.
            int_grid_rendering: IntGridRendering::Invisible,
            ..default()
        })
        .insert_resource(sandbox_data)
        .insert_resource(music_registry)
        .insert_resource(sfx_registry)
        .insert_resource(sandbox_catalog)
        .insert_resource(DeveloperTools::default())
        .insert_resource(EditablePlayerStats::default())
        .insert_resource(SandboxFeelTuning::default())
        // The OwnedItems catalog is simulation state, not only presentation UI
        // state. Headless SandboxSim runs quest reward systems (which grant into
        // OwnedItems) without loading `add_presentation_plugins`, so the resource
        // must exist before the first Update tick.
        .insert_resource(ambition::items::OwnedItems::starter())
        .insert_resource(editable_abilities)
        .insert_resource(editable_tuning)
        // Sim/presentation seam for input (ADR 0012): the sim reads
        // `Res<ControlFrame>`. Visible builds populate it from leafwing in
        // `populate_control_frame_from_actions`; headless tests can write
        // directly. Default = no buttons pressed = idle player.
        .init_resource::<ControlFrame>()
        // Aggregate user settings (video/audio/controls/gameplay).
        // Mutated by the pause menu; read by audio/video/gameplay
        // systems and the input deadzone/hysteresis filter.
        .insert_resource(ambition::persistence::settings::UserSettings::default());
    #[cfg(feature = "audio")]
    if let Some(path) = sfx_bank_asset_path {
        app.insert_resource(path);
    }
}
