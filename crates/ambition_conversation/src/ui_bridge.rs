//! Bridge conversation simulation authority and the presentation text runner.
//!
//! Simulation-owned [`ActiveConversation`] projects openings to presentation.
//! Narrative completion is external input and is stamped into
//! [`super::NarrativeInputLedger`] at the tick where it applies so rollback can
//! replay it deterministically. Presentation state itself is not simulation
//! authority.

use bevy::prelude::*;

use super::authority::ActiveConversation;
use super::instance::ConversationInstanceId;
use super::ledger::NarrativeInputWriter;

/// Narrative completion for one conversation instance.
///
/// Carrying the instance prevents an end observed for one conversation from
/// closing a later conversation.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct ConversationEnded {
    pub instance: ConversationInstanceId,
}

/// Stamp narrative completion into simulation input once per conversation.
///
/// The runner remains finished until simulation closes the authority, so
/// idempotent stamping is required to preserve the original simulation tick
/// across rollback.
pub fn publish_the_narrative_end(
    conversation: Res<ActiveConversation>,
    dialog: Res<ambition_dialog::DialogState>,
    mut narrative: NarrativeInputWriter<ConversationEnded>,
) {
    let Some(instance) = conversation.instance().cloned() else {
        return;
    };
    if dialog.active() {
        return;
    }
    narrative.write_once(ConversationEnded { instance });
}

/// Close simulation authority from the stamped narrative-completion input.
///
/// The ledger replays completion on the same `SimTick`; Yarn execution itself remains presentation
/// work outside rollback. This runs before continuity checks so an ended conversation is not barked.
pub fn close_conversation_on_narrative_end(
    mut conversation: ResMut<ActiveConversation>,
    mut ended: MessageReader<ConversationEnded>,
) {
    let ends = ended
        .read()
        .any(|end| conversation.instance() == Some(&end.instance));
    if ends {
        conversation.close();
    }
}

/// Project the current simulation conversation into the presentation text runner.
///
/// No authority closes/detaches the box; a different instance starts the runner; the same instance
/// leaves it untouched. `attached` is presentation-local state and is intentionally not rewound.
pub fn project_the_dialog_ui_from_the_conversation(
    conversation: Res<ActiveConversation>,
    mut dialog: ResMut<ambition_dialog::DialogState>,
    mut attached: Local<Option<ConversationInstanceId>>,
) {
    let Some(live) = conversation.live() else {
        // No conversation authority means the shared dialog surface must be closed and detached.
        *attached = None;
        if dialog.active() {
            dialog.close();
        }
        return;
    };
    if attached.as_ref() == Some(&live.instance) {
        return;
    }
    *attached = Some(live.instance.clone());
    // Instance identity includes dialogue context, so corrected participants restart Yarn state.
    dialog.start(live.dialogue_id(), &live.speaker_name, live.context());
}
