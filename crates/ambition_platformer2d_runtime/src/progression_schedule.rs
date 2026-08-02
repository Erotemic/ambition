//! Progression-phase schedule plugin.
//!
//! Boss-encounter advance, save→ECS actor/boss mirrors, quest event
//! pumping, room-metadata/music/portal sync, map-menu visit tracking,
//! and the populate-from-LDtk-and-save registry refreshers all run in
//! `Platformer2dSimulationPhaseMonolith::Progression`.
//!
//! Extracted from `app/plugins.rs` (ecs-cleanup-plan #8) so the top-level
//! simulation orchestration reads as a list of named domain plugins.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith;
use ambition_platformer2d_shared_tangle::schedule::ProgressionSet;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// Schedules the `Platformer2dSimulationPhaseMonolith::Progression` system chain plus the
/// registry-populate systems that share the same set.
pub struct ProgressionSchedulePlugin;

impl Plugin for ProgressionSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // R5 encounter-script messages: the named gate (rope cut / hazard impact
        // / cues) + the on-death payload-release signal.
        app.add_message::<ambition_platformer2d_actor_monolith::boss_encounter::EncounterGate>();
        app.add_message::<ambition_platformer2d_actor_monolith::boss_encounter::PayloadReleased>();
        // ADR 0020 / Q19: mount dissolution → the rider boss's `mount_died`
        // external phase trigger. Written in the `Combat` set (earlier this
        // frame) by `enforce_mount_rider_link`, consumed by
        // `notify_bosses_on_mount_death` at the head of the boss chain below.
        app.add_message::<ambition_platformer2d_actor_monolith::features::MountDied>();
        // The ENGINE-generic Progression chain. Every content system that used
        // to be wedged into this chain (cut-rope setup/victory, quest-completion
        // rewards, the gnu-ton gate, the quest-registry populate) now hangs on a
        // labeled slot anchored below, so this plugin names NO content (anti-god
        // rule 3) — the E-track de-weave that lets the engine progression group
        // move to the runtime face later.
        // ⭐ **the engine Progression chain, now placed by PHASE.** The systems
        // and their order are unchanged; what changed is that each group states
        // which phase it belongs to, so a slot elsewhere can order against the
        // phase instead of against a leaf system's name. `ProgressionSet` carries
        // the argument — this file held EIGHT leaf orderings, the largest
        // concentration left in the runtime after `PlayerInputSet` did the same
        // job for the input phase.
        app.configure_sets(
            sim,
            (
                ProgressionSet::BossAdvance,
                ProgressionSet::BossHazards,
                ProgressionSet::SaveMirror,
                ProgressionSet::Quest,
                ProgressionSet::WorldSync,
                ProgressionSet::Map,
            )
                .chain()
                .in_set(Platformer2dSimulationPhaseMonolith::Progression),
        );
        app.add_systems(
            sim,
            (
                // Mount-death → `mount_died` external phase trigger, ahead of the
                // phase driver so the swap is same-frame (Q19).
                ambition_platformer2d_actor_monolith::boss_encounter::notify_bosses_on_mount_death,
                ambition_platformer2d_actor_monolith::boss_encounter::update_boss_encounters,
                ambition_platformer2d_actor_monolith::boss_encounter::sync_boss_encounter_entities,
                ambition_platformer2d_actor_monolith::boss_encounter::update_encounter_progress,
            )
                .chain()
                .in_set(ProgressionSet::BossAdvance),
        );
        app.add_systems(
            sim,
            (
                ambition_platformer2d_actor_monolith::boss_encounter::tick_falling_hazards,
                ambition_platformer2d_actor_monolith::boss_encounter::tick_encounter_scripts,
                ambition_platformer2d_actor_monolith::boss_encounter::release_payloads_on_death,
                ambition_platformer2d_actor_monolith::boss_encounter::boss_phase_transition_feedback,
            )
                .chain()
                .in_set(ProgressionSet::BossHazards),
        );
        app.add_systems(
            sim,
            (
                // One save-sync over the unified actor cluster (enemies +
                // persisted-hostile NPCs flip in place).
                ambition_platformer2d_actor_monolith::features::sync_ecs_actors_with_save,
                ambition_platformer2d_actor_monolith::features::sync_ecs_bosses_with_save,
            )
                .chain()
                .in_set(ProgressionSet::SaveMirror),
        );
        app.add_systems(
            sim,
            (
                ambition_platformer2d_actor_monolith::quest::push_room_entered_quest_events,
                ambition_persistence::quest::apply_quest_advance_events,
            )
                .chain()
                .in_set(ProgressionSet::Quest),
        );
        app.add_systems(
            sim,
            (
                ambition_platformer2d_actor_monolith::rooms::sync_active_room_metadata,
                ambition_platformer2d_actor_monolith::rooms::sync_room_music_request,
                // Portal lifecycle: advance every registered portal's phase from
                // its switch state + per-phase timers.
                ambition_platformer2d_actor_monolith::rooms::tick_portal_phases_system,
            )
                .chain()
                .in_set(ProgressionSet::WorldSync),
        );
        app.add_systems(
            sim,
            (
                ambition_platformer2d_actor_monolith::menu::map::track_room_visits,
                ambition_platformer2d_actor_monolith::menu::map::sync_map_from_save,
            )
                .chain()
                .in_set(ProgressionSet::Map),
        );

        // The dev-tools inspector mirror (a DOMAIN set — its system lives in
        // `DevToolsSimPlugin`) keeps its former chain-tail slot.
        app.configure_sets(
            sim,
            ambition_dev_tools::DevInspectorMirrorSet
                .after(ProgressionSet::Map)
                .in_set(Platformer2dSimulationPhaseMonolith::Progression),
        );
        // The generic encounter lifecycle reducer (E8 — a DOMAIN set, its
        // system lives in `EncounterRegistryPlugin`): runs after the boss wrap
        // + participant-liveness refresh (`update_encounter_progress`) so this
        // frame's commands and liveness reduce this frame; the wave/boss
        // EFFECT adapters order themselves `.after(EncounterLifecycleSet)`.
        app.configure_sets(
            sim,
            ambition_encounter::EncounterLifecycleSet
                .after(ProgressionSet::BossAdvance)
                .before(ProgressionSet::BossHazards),
        );

        // Anchor the content slots into the engine chain at their exact former
        // positions. Content plugins register `.in_set(the slot)`; ordering is
        // preserved byte-for-byte because each slot pins the SAME `.after`/
        // `.before` engine neighbors the wedged system had.
        use ambition_platformer2d_actor_monolith::boss_encounter::{
            ContentEncounterScriptSet, ContentEncounterVictorySet, ContentQuestRewardSet,
        };
        app.configure_sets(
            sim,
            ContentEncounterScriptSet
                .after(ProgressionSet::BossAdvance)
                .before(ProgressionSet::BossHazards),
        );
        app.configure_sets(
            sim,
            ContentEncounterVictorySet
                .after(ProgressionSet::BossHazards)
                .before(ProgressionSet::SaveMirror),
        );
        app.configure_sets(
            sim,
            ContentQuestRewardSet
                .after(ProgressionSet::Quest)
                .before(ProgressionSet::WorldSync),
        );

        // Populate the encounter / boss registries from the LDtk project + save.
        // These run on Update (not Startup) with their existing `specs_loaded` /
        // `initialized` short-circuits so the first tick populates them and the
        // reset flow can flip the flags back to repopulate from a freshly-cleared
        // save. (The content quest-registry populate moved to
        // `AmbitionQuestContentPlugin`.)
        app.add_systems(
            sim,
            (
                ambition_platformer2d_actor_monolith::boss_encounter::populate_boss_encounter_registry,
                ambition_platformer2d_actor_monolith::encounter::populate_encounter_registry,
            )
                .in_set(Platformer2dSimulationPhaseMonolith::Progression),
        );
    }
}
