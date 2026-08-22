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

/// The narrative finished, so the simulation's conversation is over. (sim)
///
/// it reads a released record, not a delivered event. The ledger hands
/// this over on the tick it was stamped for and on every replay of that tick, so
/// the conversation ends at the same `SimTick` in the replay as it did in the
/// original run — which is what "the hold, the scripted control and the input
/// capture rewind correctly" actually requires.
///
/// It runs at the HEAD of the continuity chain so a conversation that ended this
/// frame is not first judged for separation and barked about on its way out.
///
/// what this does NOT make deterministic: the Yarn runner itself. It is
/// content executing outside the simulation, so WHICH tick it finishes on is
/// still decided by presentation. What is now true is that whichever tick it
/// finished on, every replay of that tick agrees.
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

/// The text box shows whatever conversation the simulation is having.
/// (presentation)
///
/// The snapshot deliberately does not rewind the text box; replaying the side effect rewound it
/// anyway.
///
/// The simulation went on holding the talker and capturing a seat while the player looked at
/// nothing.
///
/// opening and closing are ONE system for that reason. They were two, and
/// the closing half wrote no bookkeeping at all, so presentation's record of what
/// it had done outlived the thing it was a record of. A projection is current
/// derived state or it is not a projection.
///
/// the three cases, and each one is a rule:
///
/// * no authority → close the box AND detach. Nothing is being said.
/// * a conversation this box is not attached to → start the runner. This
///   includes the same node entered again, which is a conversation the player has
///   not seen.
/// * the conversation this box is already attached to → nothing. A rewind
///   restores the authority with the same instance, and restarting the runner
///   under a player who is mid-sentence is the defect the memo exists for.
///
/// the memo is a `Local`, and it belongs there: it is presentation's record of
/// presentation's own state, on the side of the seam that is not rewound.
pub fn project_the_dialog_ui_from_the_conversation(
    conversation: Res<ActiveConversation>,
    mut dialog: ResMut<ambition_dialog::DialogState>,
    mut attached: Local<Option<ConversationInstanceId>>,
) {
    let Some(live) = conversation.live() else {
        // the close is UNCONDITIONAL on the box, not on the memo: a box opened
        // by something that is not a conversation (a cutscene, a scripted
        // request through the bridge) is closed here exactly as it was before
        // these two systems were one, and narrowing that to "only what I opened"
        // would strand it.
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
    // While the context was a sibling of the instance id, a correction that re-wore the
    // initiator produced an equal id, this returned early above, and Yarn kept running with the
    // abandoned branch's `$speaker_id` in its variable storage.
    dialog.start(live.dialogue_id(), &live.speaker_name, live.context());
}
