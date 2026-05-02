//! Headless simulation entry point.
//!
//! Phase 1 of the headless/RL track. Runs the sandbox simulation systems on a
//! Bevy `App` built from `MinimalPlugins` plus the asset/state foundations that
//! the runtime-spine systems need, with no windowing, rendering, audio, or
//! input plugins. This validates that:
//!
//! * the embedded LDtk world parses and validates,
//! * the runtime `RoomSet` and `LdtkRuntimeIndex` construct from LDtk,
//! * the runtime-spine systems compile and tick on a no-display machine.
//!
//! Phase 1 deliberately does **not** call `sandbox_update`, which still wires
//! audio, particle, physics-debris, and HUD side effects directly. Once the
//! sim/presentation events refactor lands, the same sim systems become
//! callable headless and the gameplay loop can run here for RL training and
//! CI smoke tests.
//!
//! It also does **not** install `bevy_ecs_ldtk::LdtkPlugin`, because that
//! plugin's tile-rendering pipeline depends on Bevy's image/render plugins.
//! Without LDtk-spawned entities the runtime-spine systems run as no-ops and
//! the `HeadlessReport` reflects zero spawned entities — which is the correct
//! Phase 1 outcome (the goal here is "no panic," not "RL-ready").

use std::fmt;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

use crate::game_mode::GameMode;
use crate::ldtk_world;
use crate::rooms::RoomSet;

/// Summary of a `run_headless` call. Used by tests, the headless binary, and
/// future RL drivers to verify the simulation actually progressed instead of
/// silently no-op.
#[derive(Debug, Clone)]
pub struct HeadlessReport {
    pub ticks_run: u32,
    pub active_room: String,
    pub room_count: usize,
    pub spawned_entities: usize,
    pub spine_revision: u64,
    pub solid_index_revision: u64,
}

impl fmt::Display for HeadlessReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "headless run completed: {} ticks", self.ticks_run)?;
        writeln!(f, "  active room    : {}", self.active_room)?;
        writeln!(f, "  rooms loaded   : {}", self.room_count)?;
        writeln!(
            f,
            "  ldtk entities  : {} (spawned by bevy_ecs_ldtk)",
            self.spawned_entities
        )?;
        writeln!(f, "  spine revision : {}", self.spine_revision)?;
        write!(f, "  solid revision : {}", self.solid_index_revision)?;
        Ok(())
    }
}

/// Run the sandbox simulation headless for `max_ticks` Bevy `Update` cycles.
///
/// Returns a `HeadlessReport`. Validation failures from the embedded LDtk
/// project propagate as `Err`, matching the production policy that an invalid
/// LDtk file should be a hard error rather than a `.expect()` panic.
pub fn run_headless(max_ticks: u32) -> Result<HeadlessReport, String> {
    let project = ldtk_world::LdtkProject::load_embedded();
    let report = project.validate();
    if !report.is_ok() {
        report.print_to_stderr();
        return Err(format!(
            "embedded LDtk validation failed: {} error(s)",
            report.errors.len()
        ));
    }
    let room_set = project.to_room_set().map_err(|errors| errors.join("; "))?;
    let active_room = room_set.active_spec().id.clone();
    let ldtk_index = ldtk_world::LdtkRuntimeIndex::from_project(&project, active_room);
    let room_count = room_set.rooms.len();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(StatesPlugin);
    app.init_state::<GameMode>();

    app.insert_resource(room_set);
    app.insert_resource(ldtk_index);
    app.insert_resource(ldtk_world::LdtkHotReloadState::from_current_file());
    app.insert_resource(ldtk_world::LdtkRuntimeSpineStats::default());
    app.insert_resource(ldtk_world::LdtkRuntimeSpineIndex::default());
    app.insert_resource(ldtk_world::LdtkRuntimeSolidIndex::default());

    app.add_systems(
        Update,
        (
            ldtk_world::poll_ldtk_file_changes,
            ldtk_world::sync_plugin_spawned_ambition_entities,
            ldtk_world::rebuild_ldtk_runtime_spine_index,
            ldtk_world::rebuild_ldtk_runtime_solid_index,
        )
            .chain(),
    );

    for _ in 0..max_ticks {
        app.update();
    }

    let world = app.world();
    let stats = world
        .resource::<ldtk_world::LdtkRuntimeSpineStats>()
        .clone();
    let spine_index = world.resource::<ldtk_world::LdtkRuntimeSpineIndex>();
    let solid_index = world.resource::<ldtk_world::LdtkRuntimeSolidIndex>();
    let active_room_after = world.resource::<RoomSet>().active_spec().id.clone();

    Ok(HeadlessReport {
        ticks_run: max_ticks,
        active_room: active_room_after,
        room_count,
        spawned_entities: stats.spawned_entities,
        spine_revision: spine_index.revision,
        solid_index_revision: solid_index.revision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_headless_completes_one_tick_without_panicking() {
        let report = run_headless(1).expect("headless one-tick run succeeds");
        assert_eq!(report.ticks_run, 1);
        assert!(
            report.room_count > 0,
            "embedded LDtk should produce at least one room"
        );
        assert!(!report.active_room.is_empty());
    }

    #[test]
    fn run_headless_runs_multiple_ticks() {
        let report = run_headless(8).expect("headless eight-tick run succeeds");
        assert_eq!(report.ticks_run, 8);
    }
}
