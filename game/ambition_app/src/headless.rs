//! Headless simulation entry point.
//!
//! `run_headless` drives the full gameplay loop by adding
//! `SandboxSimulationPlugin`. The visible binary uses `run_visible` which
//! additionally adds `SandboxLdtkPlugin` and `SandboxPresentationPlugin`.
//! Headless skips the presentation half so audio, VFX, debris, HUD, and
//! inspector plugins are absent — the sim emits messages into the queue
//! and the queue drains harmlessly.
//!
//! Phase 1 only ticked the LDtk runtime-spine systems; Phase 2 added the
//! full gameplay loop including movement, collision, and typed-event
//! channels.

use std::fmt;

use bevy::prelude::*;

use crate::app::SandboxSimulationPlugin;
use ambition::actors::ldtk_world;
use ambition::actors::rooms::RoomSet;

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
    /// One-line HUD summary per active or completed quest. Drawn from
    /// `QuestRegistry::quest_log_lines()` at the end of the run so the
    /// headless smoke can verify intro-v1 quest progression without
    /// spinning up a renderer.
    pub quest_log: Vec<String>,
    /// Room ids the player visited during the run, in stable order
    /// (`MapMenuState::visited`). Empty when the run starts and stays
    /// in the cold-launch room.
    pub visited_rooms: Vec<String>,
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
        writeln!(f, "  solid revision : {}", self.solid_index_revision)?;
        writeln!(
            f,
            "  rooms visited  : {}",
            if self.visited_rooms.is_empty() {
                "<none>".to_string()
            } else {
                self.visited_rooms.join(", ")
            }
        )?;
        if self.quest_log.is_empty() {
            write!(f, "  quest log      : <empty>")?;
        } else {
            writeln!(f, "  quest log      :")?;
            for (i, line) in self.quest_log.iter().enumerate() {
                if i + 1 == self.quest_log.len() {
                    write!(f, "    {line}")?;
                } else {
                    writeln!(f, "    {line}")?;
                }
            }
        }
        Ok(())
    }
}

/// Run the sandbox simulation headless for `max_ticks` Bevy `Update` cycles.
///
/// Builds an `App` from `MinimalPlugins` plus the small set of Bevy
/// foundation plugins the sim's resources / assets / states / transforms
/// need, then composes `init_sandbox_resources` and
/// `init_sandbox_resources` / `add_simulation_plugins` from `ambition_app::app`. Calls `app.update()`
/// `max_ticks` times and returns a `HeadlessReport`.
///
/// Validation failures from the embedded LDtk project propagate as `Err`,
/// matching the production policy that an invalid LDtk file is a hard
/// error rather than a `.expect()` panic.
pub fn run_headless(max_ticks: u32) -> Result<HeadlessReport, String> {
    // Validate the embedded LDtk file up front so we can return Err with a
    // useful diagnostic. `init_sandbox_resources` does this too but exits
    // the process on failure; tests want a structured error instead.
    // Validate the embedded world before constructing the App. Provider-owned
    // character, hostile-archetype, boss, and audio catalogs are composed as
    // App-local resources by `SandboxSimulationPlugin`.
    let world_manifest = ambition_content::worlds::world_manifest();
    let project = ldtk_world::LdtkProject::load_default_for_dev(&world_manifest)?;
    let report = project.validate();
    if !report.is_ok() {
        report.print_to_stderr();
        return Err(format!(
            "embedded LDtk validation failed: {} error(s)",
            report.errors.len()
        ));
    }
    if let Err(errors) = project.to_room_set(&world_manifest) {
        return Err(errors.join("; "));
    }
    let room_count = project
        .to_room_set(&world_manifest)
        .expect("just validated above")
        .rooms
        .len();

    let mut app = App::new();
    // The shared engine foundation (schedules/time, asset + image registries,
    // transforms, states) — ONE definition in ambition::runtime for every
    // headless entry point.
    ambition::runtime::add_headless_foundation(&mut app);

    app.add_plugins(SandboxSimulationPlugin);

    for _ in 0..max_ticks {
        app.update();
    }

    let world = app.world();
    let stats = world
        .resource::<ldtk_world::LdtkRuntimeSpineStats>()
        .clone();
    let spine_index = world.resource::<ldtk_world::LdtkRuntimeSpineIndex>();
    let solid_index = world.resource::<ldtk_world::LdtkRuntimeSolidIndex>();
    let active_room_after =
        ambition::platformer::lifecycle::session_world_component::<RoomSet>(world)
            .expect("active session RoomSet")
            .active_spec()
            .id
            .clone();
    // Quest log + visited rooms are optional — both are inserted by
    // sandbox startup, but a hypothetical caller that swaps the plugin
    // set might omit them. Use try_resource where it exists so the
    // headless report still produces a partial result rather than
    // panicking on a missing resource.
    let quest_log = world
        .get_resource::<ambition_content::quest::QuestRegistry>()
        .map(|r| r.quest_log_lines())
        .unwrap_or_default();
    let visited_rooms = world
        .get_resource::<ambition::menu::map::MapMenuState>()
        .map(|m| m.visited.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    Ok(HeadlessReport {
        ticks_run: max_ticks,
        active_room: active_room_after,
        room_count,
        spawned_entities: stats.spawned_entities,
        spine_revision: spine_index.revision,
        solid_index_revision: solid_index.revision,
        quest_log,
        visited_rooms,
    })
}

#[cfg(test)]
mod tests;
