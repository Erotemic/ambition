//! Named Ambition quest content registration.
//!
//! Owns the install of the default [`QuestRegistry`] so the named quest
//! roster is constructed in one content-owned place instead of inline in
//! `app/sim_resources.rs`. The registry's *contents* (quest definitions)
//! still live in `crate::quest`; this module only owns the
//! decision to register the default roster as a sandbox resource.

use bevy::prelude::*;

/// Installs the default Ambition quest registry resource.
pub struct AmbitionQuestContentPlugin;

impl Plugin for AmbitionQuestContentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(crate::quest::QuestRegistry::default());

        // Content quest progression, de-woven from the app's Progression chain
        // onto engine slots (E-track): the completion-reward grant hangs on the
        // labeled `ContentQuestRewardSet` (host-anchored after the engine's quest
        // advance pump, before room metadata sync); the quest-registry populate
        // rides the Progression set with its own `initialized` short-circuit.
        // The engine quest EVENT PUMP (push/apply) stays engine-side.
        app.add_systems(
            Update,
            crate::quest::grant_quest_completion_rewards
                .in_set(ambition_gameplay_core::boss_encounter::ContentQuestRewardSet),
        );
        app.add_systems(
            Update,
            crate::quest::populate_quest_registry
                .in_set(ambition_gameplay_core::schedule::SandboxSet::Progression),
        );
    }
}
