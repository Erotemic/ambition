//! Conversation-domain plugin registration and schedule ownership.
//!
//! Conversation owns its state, messages, and systems. Cross-domain ordering uses shared schedule
//! vocabulary; narrative input payloads remain installed by the domain that owns each payload.

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update};

use ambition_platformer2d_shared_tangle::schedule::{FeatureInteractionSet, SimScheduleExt as _};

use super::authority::ActiveConversation;
use super::hold::project_conversation_hold;
use super::ledger::NarrativeInputPlugin;
use super::rules::{break_dialogue_on_hit_or_separation, ConversationCutBark};
use super::ui_bridge::{
    close_conversation_on_narrative_end, project_the_dialog_ui_from_the_conversation,
    publish_the_narrative_end, ConversationEnded,
};

/// Installs conversation-owned state, messages, and systems.
/// Cross-domain order is expressed through the shared feature-interaction schedule sets.
pub struct ConversationPlugin;

impl Plugin for ConversationPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();

        // Conversation authority is simulation state; its UI projection is presentation state.
        app.init_resource::<ActiveConversation>();
        app.init_resource::<crate::NarrativeMusicRequest>();
        // This domain owns the cut-bark channel even though another domain consumes it.
        app.add_message::<ConversationCutBark>();
        // Narrative input is external to rollback state; the ledger replays the same stamped input
        // on resimulation. Conversation installs only payloads it owns.
        app.add_plugins(NarrativeInputPlugin::<ConversationEnded>::default());

        // Conversation owns the authored-command narrative writer; the lower execution layer cannot
        // depend upward on this ledger.
        app.add_plugins(NarrativeInputPlugin::<
            ambition_platformer2d_shared_tangle::authored_logic::RunAuthoredCommand,
        >::default());

        // Projection must run before completion detection: a newly opened runner is inactive until
        // projection starts it. The chain is internal to conversation and stays in `Update`.
        app.add_systems(
            Update,
            (
                project_the_dialog_ui_from_the_conversation,
                publish_the_narrative_end,
            )
                .chain(),
        );

        // Simulation systems join the shared feature-interaction phases they depend on.
        app.add_systems(
            sim,
            close_conversation_on_narrative_end.in_set(FeatureInteractionSet::NarrativeIntake),
        );
        app.add_systems(
            sim,
            break_dialogue_on_hit_or_separation.in_set(FeatureInteractionSet::Continuity),
        );
        app.add_systems(
            sim,
            project_conversation_hold.in_set(FeatureInteractionSet::HoldProjection),
        );
    }
}
