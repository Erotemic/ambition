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

use super::schedule::SandboxSet;

/// Schedules the `SandboxSet::Progression` system chain plus the
/// registry-populate systems that share the same set.
pub struct ProgressionSchedulePlugin;

impl Plugin for ProgressionSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                crate::boss_encounter::update_boss_encounters,
                // Feel feedback (shake + cry SFX) on dramatic boss phase changes;
                // diffs the registry phase, so it just needs to run after the
                // boss update advances it.
                crate::boss_encounter::boss_phase_transition_feedback,
                crate::boss_encounter::spawn_cut_rope_victory_npc,
                // Hides the gnu_ton arena's retreat ladder while the boss
                // is alive, re-adds it the frame the boss dies. Runs after
                // `update_boss_encounters` so a defeat this tick is
                // observable as `boss.alive = false`, and before player
                // movement consumes `world.climbable_regions` in the next
                // visual sync set.
                crate::boss_encounter::gate_gnu_ton_arena_ladder,
                crate::features::sync_ecs_actors_with_save,
                crate::features::sync_ecs_npc_actors_with_save,
                crate::features::sync_ecs_bosses_with_save,
                crate::content::quest::push_room_entered_quest_events,
                crate::content::quest::apply_quest_advance_events,
                crate::content::quest::grant_quest_completion_rewards,
                crate::rooms::sync_active_room_metadata,
                crate::rooms::sync_room_music_request,
                // Portal lifecycle: advance every registered portal's
                // phase from its switch state + per-phase timers.
                // Pure state update; the visibility + ring-spin
                // systems below consume the phase. Lives in the
                // Progression set so the portal state is current
                // before `detect_room_transition_system` runs (which
                // is in CoreSimulation, ordered after Progression).
                crate::rooms::tick_portal_phases_system,
                crate::map_menu::track_room_visits,
                crate::map_menu::sync_map_from_save,
                crate::dev::dev_tools::sync_player_stats_with_inspector,
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
                crate::content::quest::populate_quest_registry,
                crate::boss_encounter::populate_boss_encounter_registry,
                crate::encounter::populate_encounter_registry,
            )
                .in_set(SandboxSet::Progression),
        );
    }
}
