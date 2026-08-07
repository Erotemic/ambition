//! **The seam between the authority and the text box.**
//!
//! A conversation has TWO enders and they come from opposite directions: the
//! simulation can break it (somebody was knocked away, somebody walked off), and
//! the NARRATIVE can finish it (the Yarn runner ran out of lines). Neither can
//! be derived from the other, so the seam is explicit and each direction is one
//! system with one job.

use bevy::prelude::*;

use super::authority::ActiveConversation;

/// **The narrative finished, so the simulation's conversation is over.** (sim)
///
/// ⚠ **this is the one place a simulation system reads `DialogState`, and the
/// justification is narrow: it is an EXTERNAL INPUT, not a rule.** The Yarn
/// runner is content running outside the simulation, in the same class as a
/// device frame — the sim does not get to decide when a script runs out of
/// lines, it can only be told. The read is bounded to exactly that: it never
/// opens a conversation, never chooses a participant, and only ever CLOSES.
///
/// ⛔ **and it is honestly a rollback seam, not a solved problem.** `DialogState`
/// is not rewound, so on a resimulated tick this reads the live runner rather
/// than the runner as it was. That was equally true before the authority existed
/// — every continuity rule read this resource — so it is not a regression, and
/// the fix is a `ConversationEnded` message with `clear_message_on_rollback`
/// rather than anything this system can do. Recorded as its own row rather than
/// smuggled in here, because it needs the runner's own lifecycle to have an
/// opinion.
///
/// It runs at the HEAD of the continuity chain so a conversation that ended this
/// frame is not first judged for separation and barked about.
pub fn close_conversation_when_the_narrative_ends(
    mut conversation: ResMut<ActiveConversation>,
    dialog: Res<ambition_dialog::DialogState>,
) {
    if conversation.is_live() && !dialog.active() {
        conversation.close();
    }
}

/// **The simulation ended it, so the text box goes away.** (presentation)
///
/// The projection direction, and the reason `DialogState` can stop being an
/// authority without the box outliving the conversation it was showing. Runs
/// outside the simulation schedule: it writes presentation state only, and a
/// rewind must not un-close a text box the player already saw close.
pub fn close_dialog_ui_when_the_conversation_ends(
    conversation: Res<ActiveConversation>,
    mut dialog: ResMut<ambition_dialog::DialogState>,
) {
    if !conversation.is_live() && dialog.active() {
        dialog.close();
    }
}
