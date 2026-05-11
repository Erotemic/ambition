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
use super::setup_systems::*;
#[allow(unused_imports)]
use super::update::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;

/// Programmatic start-room override. SandboxSim and other library
/// callers insert this resource before `init_sandbox_resources` runs;
/// the function consumes it (taking precedence over the
/// `--start-room` CLI flag) so callers do not need to manipulate
/// `std::env::args` to pin a starting room.
#[derive(Resource, Clone, Debug)]
pub struct StartRoomOverride(pub String);

pub fn init_sandbox_resources(app: &mut App) {
    let sandbox_data = data::SandboxDataSpec::load_embedded();
    let ldtk_project = match ldtk_world::LdtkProject::load_default() {
        Ok(project) => project,
        Err(error) => {
            eprintln!("failed to load sandbox LDtk map: {error}");
            std::process::exit(2);
        }
    };
    let ldtk_report = ldtk_project.validate();
    ldtk_report.print_to_stderr();
    let valid_track_ids = sandbox_data
        .audio
        .music_tracks
        .iter()
        .map(|t| t.id.as_str());
    for warning in ldtk_project.music_track_warnings(valid_track_ids) {
        eprintln!("LDtk validation warning: {warning}");
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
            std::process::exit(2);
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
        .insert_resource(ldtk_world::LdtkHotReloadState::from_current_file())
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
        .insert_resource(DeveloperTools::default())
        .insert_resource(EditablePlayerStats::default())
        .insert_resource(SandboxFeelTuning::default())
        // PlayerInventory is simulation state, not only presentation UI state.
        // Headless SandboxSim runs quest reward systems without loading
        // `add_presentation_plugins`, so the resource must exist before the
        // first Update tick.
        .insert_resource(crate::inventory::PlayerInventory::starter())
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
        .insert_resource(crate::settings::UserSettings::default());
}
