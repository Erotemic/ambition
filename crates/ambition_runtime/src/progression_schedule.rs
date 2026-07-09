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

use ambition_platformer_primitives::schedule::SandboxSet;

/// Schedules the `SandboxSet::Progression` system chain plus the
/// registry-populate systems that share the same set.
pub struct ProgressionSchedulePlugin;

impl Plugin for ProgressionSchedulePlugin {
    fn build(&self, app: &mut App) {
        // R5 encounter-script messages: the named gate (rope cut / hazard impact
        // / cues) + the on-death payload-release signal.
        app.add_message::<ambition_actors::boss_encounter::EncounterGate>();
        app.add_message::<ambition_actors::boss_encounter::PayloadReleased>();
        // ADR 0020 / Q19: mount dissolution → the rider boss's `mount_died`
        // external phase trigger. Written in the `Combat` set (earlier this
        // frame) by `enforce_mount_rider_link`, consumed by
        // `notify_bosses_on_mount_death` at the head of the boss chain below.
        app.add_message::<ambition_actors::features::MountDied>();
        // The ENGINE-generic Progression chain. Every content system that used
        // to be wedged into this chain (cut-rope setup/victory, quest-completion
        // rewards, the gnu-ton gate, the quest-registry populate) now hangs on a
        // labeled slot anchored below, so this plugin names NO content (anti-god
        // rule 3) — the E-track de-weave that lets the engine progression group
        // move to the runtime face later.
        app.add_systems(
            Update,
            (
                // Boss-encounter chain (grouped into a nested `.chain()` to keep
                // the outer tuple under Bevy's 20-element limit; internal order
                // preserved). Drives phase + encounter + script + payload.
                (
                    // Mount-death → `mount_died` external phase trigger, ahead
                    // of the phase driver so the swap is same-frame (Q19).
                    ambition_actors::boss_encounter::notify_bosses_on_mount_death,
                    ambition_actors::boss_encounter::update_boss_encounters,
                    ambition_actors::boss_encounter::sync_boss_encounter_entities,
                    ambition_actors::boss_encounter::update_encounter_progress,
                    // ContentEncounterScriptSet slot (setup_cut_rope_encounter)
                    // anchors between here and tick_falling_hazards.
                    ambition_actors::boss_encounter::tick_falling_hazards,
                    ambition_actors::boss_encounter::tick_encounter_scripts,
                    ambition_actors::boss_encounter::release_payloads_on_death,
                    ambition_actors::boss_encounter::boss_phase_transition_feedback,
                )
                    .chain(),
                // ContentEncounterVictorySet slot (spawn_cut_rope_victory_npc)
                // anchors between the boss chain and the save mirrors below.
                // One save-sync over the unified actor cluster (enemies +
                // persisted-hostile NPCs flip in place).
                ambition_actors::features::sync_ecs_actors_with_save,
                ambition_actors::features::sync_ecs_bosses_with_save,
                ambition_actors::quest::push_room_entered_quest_events,
                ambition_persistence::quest::apply_quest_advance_events,
                // ContentQuestRewardSet slot (grant_quest_completion_rewards)
                // anchors between the quest pump and the metadata sync below.
                ambition_actors::rooms::sync_active_room_metadata,
                ambition_actors::rooms::sync_room_music_request,
                // Portal lifecycle: advance every registered portal's
                // phase from its switch state + per-phase timers.
                // Pure state update; the visibility + ring-spin
                // systems below consume the phase. Lives in the
                // Progression set so the portal state is current
                // before `detect_room_transition_system` runs (which
                // is in CoreSimulation, ordered after Progression).
                ambition_actors::rooms::tick_portal_phases_system,
                ambition_actors::menu::map::track_room_visits,
                ambition_actors::menu::map::sync_map_from_save,
                ambition_dev_tools::dev_tools::sync_player_stats_with_inspector,
            )
                .chain()
                .in_set(SandboxSet::Progression),
        );

        // Anchor the content slots into the engine chain at their exact former
        // positions. Content plugins register `.in_set(the slot)`; ordering is
        // preserved byte-for-byte because each slot pins the SAME `.after`/
        // `.before` engine neighbors the wedged system had.
        use ambition_actors::boss_encounter::{
            ContentEncounterScriptSet, ContentEncounterVictorySet, ContentQuestRewardSet,
        };
        app.configure_sets(
            Update,
            ContentEncounterScriptSet
                .in_set(SandboxSet::Progression)
                .after(ambition_actors::boss_encounter::update_encounter_progress)
                .before(ambition_actors::boss_encounter::tick_falling_hazards),
        );
        app.configure_sets(
            Update,
            ContentEncounterVictorySet
                .in_set(SandboxSet::Progression)
                .after(ambition_actors::boss_encounter::boss_phase_transition_feedback)
                .before(ambition_actors::features::sync_ecs_actors_with_save),
        );
        app.configure_sets(
            Update,
            ContentQuestRewardSet
                .in_set(SandboxSet::Progression)
                .after(ambition_persistence::quest::apply_quest_advance_events)
                .before(ambition_actors::rooms::sync_active_room_metadata),
        );

        // Populate the encounter / boss registries from the LDtk project + save.
        // These run on Update (not Startup) with their existing `specs_loaded` /
        // `initialized` short-circuits so the first tick populates them and the
        // reset flow can flip the flags back to repopulate from a freshly-cleared
        // save. (The content quest-registry populate moved to
        // `AmbitionQuestContentPlugin`.)
        app.add_systems(
            Update,
            (
                ambition_actors::boss_encounter::populate_boss_encounter_registry,
                ambition_actors::encounter::populate_encounter_registry,
            )
                .in_set(SandboxSet::Progression),
        );
    }
}
