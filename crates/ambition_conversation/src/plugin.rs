//! **What `conversation` registers, owned by `conversation`.**
//!
//! It was not: `features::FeatureInteractionSchedulePlugin` held [`ActiveConversation`],
//! [`ConversationCutBark`], seven `NarrativeInputPlugin` installs, and three of this module's
//! systems wedged into ONE anonymous `.chain()` between `interact_ecs_actors_and_switches` and the
//! chest systems.
//!
//! **the generalisable lesson: a module with zero inward imports can still be
//! pinned by the SCHEDULE.** Count the registrations, not only the paths.
//!
//! [`ConversationPlugin`] is the answer to the first half — the state, the port channel, the
//! payload this module itself defines, and its own systems now belong to it. The total order is
//! declared once, by the owner of the phase.
//!
//! **the vocabulary deliberately lives in
//! `ambition_platformer2d_shared_tangle`, a crate BELOW the monolith.** A set
//! enum defined in `features` would have re-pinned this module to `features` by
//! the schedule the moment it stopped importing it — the exact bug, one level up.
//!
//! ## What this plugin deliberately does NOT own
//!
//! Six of the seven `NarrativeInputPlugin::<T>` installs stay with
//! `FeatureInteractionSchedulePlugin`, and that is the correct seam rather than a leftover.
//! Conversation provides the ledger MECHANISM; it does not decide another domain's vocabulary.
//!
//! [`ConversationEnded`] is the one payload this module both defines and
//! consumes, so its install comes here with it.

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

/// **Everything the conversation domain installs into an `App`.**
///
/// Placement only: the cross-domain order it participates in is declared by
/// whoever owns
/// [`Platformer2dSimulationPhaseMonolith::FeatureInteraction`](ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::FeatureInteraction),
/// and this plugin names no system outside `conversation`.
pub struct ConversationPlugin;

impl Plugin for ConversationPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();

        // The conversation authority is sim state and lives for the whole App;
        // the UI projection that follows it is presentation and runs outside the
        // simulation schedule, so a rewind cannot un-close a box the player
        // already watched close.
        app.init_resource::<ActiveConversation>();
        app.init_resource::<crate::NarrativeMusicRequest>();
        // **REGISTER THE CHANNEL THE PORT ASKS THROUGH.** The break rule
        // writes it and the cast answers it — the cast lives in `features::npcs`
        // and the channel is this module's, so this plugin owns the
        // registration. Leaving it to whoever else wanted the message is how the
        // effect quarantine once worked in a shipped app and nowhere else; here
        // it failed parameter validation on frame one of the sandbox harness.
        app.add_message::<ConversationCutBark>();
        // **the ledger is NOT rollback state, and that is the whole design.**
        // It is the record of what the narrative — which runs outside the
        // simulation — told the simulation, stamped with the tick it applies
        // from. A rewind restores what the simulation DECIDED; erasing what it
        // was TOLD is how the replay reaches a different answer. The install
        // brings the ledger, its release at the head of the sim frame, and the
        // prune that ages a record out once its tick can never be replayed.
        //
        // **only the payload this module DEFINES.** The other six live with
        // the domains that consume them — see this file's header.
        app.add_plugins(NarrativeInputPlugin::<ConversationEnded>::default());

        // **and the AUTHORED-COMMAND request, which is the one exception to
        // the "the consumer owns the install" rule above — because its consumer
        // structurally CANNOT own it.** `RunAuthoredCommand` is performed by
        // `shared_tangle::authored_logic`, a crate below this one that cannot
        // name a narrative ledger at all. What this module owns is the
        // `<<command …>>` verb that produces it — the only narrative writer this
        // channel has, or will have.
        app.add_plugins(NarrativeInputPlugin::<
            ambition_platformer2d_shared_tangle::authored_logic::RunAuthoredCommand,
        >::default());

        // The TWO presentation halves of the seam: one projects the box from the
        // authority (and detaches from it), one observes the runner finishing and
        // records it for the simulation. Neither runs in the sim schedule, which
        // is what keeps a rewind from replaying a side effect onto state it does
        // not rewind.
        //
        // **`.chain()`, and the order is load-bearing.** "The runner is not active" is how the
        // second one recognises a finished conversation — and on the frame a conversation OPENS
        // that is also true until the first one has run.
        //
        // this one stays an anonymous chain ON PURPOSE: both members are
        // `conversation`'s, so the ordering is INTERNAL and no other domain can
        // depend on a boundary inside it. A set is owed to a contract somebody
        // else reads. and it must never acquire a cross-schedule `.before`/
        // `.after` against a `FeatureInteractionSet` variant — this pair is in
        // `Update`, those are in the sim schedule, and an ordering edge between
        // schedules does nothing and reports nothing.
        app.add_systems(
            Update,
            (
                project_the_dialog_ui_from_the_conversation,
                publish_the_narrative_end,
            )
                .chain(),
        );

        // The three sim systems, each placed by PHASE. The reason each phase sits
        // where it does is on the `FeatureInteractionSet` variant it names —
        // which is the point: the rationale now travels with the relationship
        // instead of with a position in a tuple.
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
