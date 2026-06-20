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
use super::setup_systems::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use ambition_gameplay_core::schedule::*;

/// Programmatic start-room override. SandboxSim and other library
/// callers insert this resource before `init_sandbox_resources` runs;
/// the function consumes it (taking precedence over the
/// `--start-room` CLI flag) so callers do not need to manipulate
/// `std::env::args` to pin a starting room.
#[derive(Resource, Clone, Debug)]
pub struct StartRoomOverride(pub String);

/// Per-frame flag set by `player_control_system` when an engine-
/// driven reset fired (control or simulation phase asked for one).
/// `player_simulation_system` reads it; when set, the simulation
/// phase short-circuits this frame so the reset's state changes
/// don't get clobbered by a same-frame sim integration.
///
/// Cleared at the start of each frame by
/// `clear_sandbox_reset_this_frame` in the PlayerInput set.
///
/// Replaces the early-return short-circuit that the deleted
/// `sandbox_update` orchestrator used to express via control flow.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SandboxResetThisFrame(pub bool);

#[cfg(test)]
fn sandbox_init_failed() -> ! {
    panic!("sandbox resource initialization failed; see diagnostics above");
}

#[cfg(not(test))]
fn sandbox_init_failed() -> ! {
    std::process::exit(2);
}

pub fn init_sandbox_resources(app: &mut App) {
    // Install the named boss roster into the machinery lib's holder before
    // anything in this function (or any later system) resolves a boss profile
    // via `BossBehaviorProfile::from_data` (the boss encounter registry +
    // content validation read it). This is the single choke point every sim
    // entry path flows through (the plugin and the test harnesses), so the
    // content data is always present. First-install-wins.
    ambition_content::bosses::install_boss_roster();

    let sandbox_data = data::SandboxDataSpec::load_embedded();

    // Build the singleton SandboxAssetCatalog before anything else asks
    // it for a path. Every asset path/source policy in the visible
    // sandbox flows through this — LDtk, SFX bank, fonts, sprites,
    // music. Consumes the already-parsed sandbox_data so music-track
    // ids land in the catalog.
    let asset_config = app
        .world()
        .get_resource::<ambition_gameplay_core::assets::game_assets::GameAssetConfig>()
        .cloned()
        .unwrap_or_default();
    let sandbox_catalog =
        ambition_gameplay_core::assets::sandbox_assets::build_sandbox_catalog_with(
            &asset_config,
            &sandbox_data.audio,
            |manifest| {
                ambition_content::intro::sprites::extend_with_intro_sprite_entries(
                    manifest,
                    &asset_config.sprite_folder,
                );
            },
        );

    let ldtk_project = match ldtk_world::LdtkProject::load_default(&sandbox_catalog) {
        Ok(project) => project,
        Err(error) => {
            eprintln!("failed to load sandbox LDtk map: {error}");
            sandbox_init_failed();
        }
    };
    let content_report = content_validation::validate_content_graph(&sandbox_data, &ldtk_project);
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

    app.insert_resource(ldtk_world::SandboxLdtkProject(ldtk_project.clone()))
        .insert_resource(GameWorld(active_world))
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
        .insert_resource(sandbox_catalog)
        .insert_resource(DeveloperTools::default())
        .insert_resource(EditablePlayerStats::default())
        .insert_resource(SandboxFeelTuning::default())
        // The OwnedItems catalog is simulation state, not only presentation UI
        // state. Headless SandboxSim runs quest reward systems (which grant into
        // OwnedItems) without loading `add_presentation_plugins`, so the resource
        // must exist before the first Update tick.
        .insert_resource(ambition_gameplay_core::items::OwnedItems::starter())
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
        .insert_resource(ambition_gameplay_core::persistence::settings::UserSettings::default());
}
