//! Progression-phase schedule plugin.
//!
//! Boss-encounter advance, save→ECS actor/boss mirrors, quest event
//! pumping, room-metadata/music/portal sync, map-menu visit tracking,
//! and the populate-from-LDtk-and-save registry refreshers all run in
//! `SandboxSet::Progression`.
//!
//! Extracted from `app/plugins.rs` (ecs-cleanup-plan #8) so the top-level
//! simulation orchestration reads as a list of named domain plugins.

use bevy::prelude::*;

use ambition_gameplay_core::app::SandboxSet;

/// Schedules the `SandboxSet::Progression` system chain plus the
/// registry-populate systems that share the same set.
pub struct ProgressionSchedulePlugin;

impl Plugin for ProgressionSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                ambition_gameplay_core::boss_encounter::update_boss_encounters,
                // Feel feedback (shake + cry SFX) on dramatic boss phase changes;
                // diffs the registry phase, so it just needs to run after the
                // boss update advances it.
                ambition_gameplay_core::boss_encounter::boss_phase_transition_feedback,
                ambition_content::bosses::spawn_cut_rope_victory_npc,
                // Hides the gnu_ton arena's retreat ladder while the boss
                // is alive, re-adds it the frame the boss dies. Runs after
                // `update_boss_encounters` so a defeat this tick is
                // observable as `boss.alive = false`, and before player
                // movement consumes `world.climbable_regions` in the next
                // visual sync set.
                ambition_content::bosses::gate_gnu_ton_arena_ladder,
                ambition_gameplay_core::features::sync_ecs_actors_with_save,
                ambition_gameplay_core::features::sync_ecs_npc_actors_with_save,
                ambition_gameplay_core::features::sync_ecs_bosses_with_save,
                ambition_content::quest::push_room_entered_quest_events,
                ambition_content::quest::apply_quest_advance_events,
                ambition_content::quest::grant_quest_completion_rewards,
                ambition_gameplay_core::rooms::sync_active_room_metadata,
                ambition_gameplay_core::rooms::sync_room_music_request,
                // Portal lifecycle: advance every registered portal's
                // phase from its switch state + per-phase timers.
                // Pure state update; the visibility + ring-spin
                // systems below consume the phase. Lives in the
                // Progression set so the portal state is current
                // before `detect_room_transition_system` runs (which
                // is in CoreSimulation, ordered after Progression).
                ambition_gameplay_core::rooms::tick_portal_phases_system,
                ambition_gameplay_core::menu::map::track_room_visits,
                ambition_gameplay_core::menu::map::sync_map_from_save,
                ambition_gameplay_core::dev::dev_tools::sync_player_stats_with_inspector,
            )
                .chain()
                .in_set(SandboxSet::Progression),
        );

        // Populate the encounter / quest / boss registries from the LDtk
        // project + save. These run on Update (not Startup) with their
        // existing `specs_loaded` / `initialized` short-circuits so the
        // first tick populates them and the reset flow can flip the flags
        // back to repopulate from a freshly-cleared save. Cost when already
        // loaded is one ResMut + one bool check per registry per frame.
        app.add_systems(
            Update,
            (
                ambition_content::quest::populate_quest_registry,
                ambition_gameplay_core::boss_encounter::populate_boss_encounter_registry,
                ambition_gameplay_core::encounter::populate_encounter_registry,
            )
                .in_set(SandboxSet::Progression),
        );
    }
}
