//! **The seam between the authority and the text box.**
//!
//! A conversation has TWO enders and they come from opposite directions: the
//! simulation can break it (somebody was knocked away, somebody walked off), and
//! the NARRATIVE can finish it (the Yarn runner ran out of lines). Neither can
//! be derived from the other, so the seam is explicit and each direction is one
//! system with one job.
//!
//! ⭐ **the narrative direction is an EVENT, not a poll**, and that is the whole
//! difference. The sim used to ask `DialogState::active()` every tick; that
//! resource is not rewound, so a resimulated tick read the LIVE runner instead of
//! the runner as it was, and a rewind to before a conversation ended would end it
//! again immediately. A tick that was told nothing now changes nothing, which is
//! what a resimulated tick has to do.

use bevy::prelude::*;

use super::authority::ActiveConversation;

/// **The narrative ran out of lines.** Written by presentation, consumed by the
/// simulation.
///
/// ⚠ **it NAMES the conversation it is ending.** A bare marker would close
/// whatever happened to be live when it was read, so finishing one conversation
/// and immediately starting another could have the second closed by the first
/// one's ending — `an_end_from_the_previous_conversation_does_not_close_the_next_one`
/// is that case.
///
/// ⛔ **cleared on rollback** (`clear_message_on_rollback`), for the reason every
/// other sim-facing message is: a rewound end that stayed in the queue would be
/// re-consumed on the way back through and close a conversation the replayed
/// timeline had not finished.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct ConversationEnded {
    pub dialogue_id: String,
}

/// **Tell the simulation the narrative finished.** (presentation)
///
/// Runs outside the simulation schedule, where reading the live runner is
/// exactly right: this IS the moment the runner finished, observed once, in real
/// time.
///
/// ⚠ **it may write more than once** if several presentation frames pass before
/// the sim consumes it — the condition stays true until the authority closes.
/// That is harmless because the consumer matches on `dialogue_id` and the
/// conversation is gone after the first one lands, and it is preferable to a
/// latch, which would be a second record of "has this ended yet" living outside
/// the authority that owns the answer.
pub fn publish_the_narrative_end(
    conversation: Res<ActiveConversation>,
    dialog: Res<ambition_dialog::DialogState>,
    mut ended: MessageWriter<ConversationEnded>,
) {
    if !conversation.is_live() || dialog.active() {
        return;
    }
    let Some(dialogue_id) = conversation.dialogue_id() else {
        return;
    };
    ended.write(ConversationEnded {
        dialogue_id: dialogue_id.to_string(),
    });
}

/// **The narrative finished, so the simulation's conversation is over.** (sim)
///
/// ⭐ **this reads no view state at all now.** It consumes a delivered event, so
/// a resimulated tick with an empty queue leaves the conversation exactly as the
/// replayed timeline had it.
///
/// It runs at the HEAD of the continuity chain so a conversation that ended this
/// frame is not first judged for separation and barked about on its way out.
///
/// ⚠ **what this does NOT make deterministic**: the Yarn runner itself. It is
/// content executing outside the simulation, and a conversation's LIFETIME being
/// a genuine simulation input — the advance presses replayed like device frames,
/// the runner ticking inside the sim — is a much larger piece of work. What is
/// fixed here is narrower and real: the simulation no longer asks the view a
/// question on every tick it replays.
pub fn close_conversation_on_narrative_end(
    mut conversation: ResMut<ActiveConversation>,
    mut ended: MessageReader<ConversationEnded>,
) {
    for end in ended.read() {
        if conversation.dialogue_id() == Some(end.dialogue_id.as_str()) {
            conversation.close();
        }
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
