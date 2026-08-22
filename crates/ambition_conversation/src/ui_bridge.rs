//! **The seam between the authority and the text box.**
//!
//! A conversation has TWO enders and they come from opposite directions: the
//! simulation can break it (somebody was knocked away, somebody walked off), and
//! the NARRATIVE can finish it (the Yarn runner ran out of lines). Neither can
//! be derived from the other, so the seam is explicit and each direction is one
//! system with one job.
//!
//! ```text
//! sim  → ActiveConversation ──▶ presentation PROJECTS the box
//! runner-ended ──▶ a stamped record in the ledger ──▶ sim closes it
//! ```
//!
//! Both crossings were simulation-significant and had no replay story:
//!
//! * **opening** ran `DialogState::start` from inside the sim schedule, which
//!   resets the line, the options, the typewriter and enqueues a
//!   `runner.start_node`. `DialogState` is deliberately not rewound — *"rewinding
//!   the typewriter should not stutter the text box"* — but a rollback across
//!   the opening tick REPLAYS that system, so the snapshot did not stutter the
//!   box and the replay did.
//! * **ending** arrived as a message written by a presentation system. Messages
//!   are cleared on rollback, and presentation does not run BETWEEN resimulated
//!   ticks — so a rewind across the end tick dropped the end, resimulated every
//!   tick after it with the conversation still holding a body and capturing a
//!   seat, and re-observed the end afterwards at a different simulation time.
//!
//! **so opening is a PROJECTION and ending is a STAMPED NARRATIVE INPUT.** The
//! simulation decides that a conversation exists; presentation reads that and
//! opens the runner. The narrative decides that it is over; that observation goes
//! into [`super::NarrativeInputLedger`] with the tick it applies from, and the
//! ledger is not rewound — because it is the record of what arrived from
//! outside, and rewinding an input erases it.
//!
//! **the ending is not a special case any more.** It is one payload type in a
//! ledger every gameplay-bearing narrative fact crosses through; see
//! [`super::ledger`] for the four rules and
//! the `dialog` module for which commands are which.

use bevy::prelude::*;

use super::authority::ActiveConversation;
use super::instance::ConversationInstanceId;
use super::ledger::NarrativeInputWriter;

/// **The narrative ran out of lines.**
///
/// Carries the conversation it ends even though the ledger only ever releases it
/// while that conversation is live: a message that cannot say what it is about
/// is one refactor away from being a global "stop whatever is running", which is
/// the poison `an_end_from_the_previous_conversation_does_not_close_the_next_one`
/// exists to keep out.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct ConversationEnded {
    pub instance: ConversationInstanceId,
}

/// **Tell the simulation the narrative finished.** (presentation)
///
/// Runs outside the simulation schedule, where reading the live runner is
/// exactly right: this IS the moment the runner finished, observed once, in real
/// time.
///
/// **observed once per conversation**, and the idempotence is load-bearing
/// rather than an optimization: the condition stays true until the authority
/// closes, and re-stamping a later tick on each of those frames would mean a
/// rewind replays an end the original run applied earlier. That is what
/// [`NarrativeInputWriter::write_once`] is for, and it is the only writer that
/// should use it — a conversation may legitimately grant two items.
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

/// **The narrative finished, so the simulation's conversation is over.** (sim)
///
/// **it reads a released record, not a delivered event.** The ledger hands
/// this over on the tick it was stamped for and on every replay of that tick, so
/// the conversation ends at the same `SimTick` in the replay as it did in the
/// original run — which is what "the hold, the scripted control and the input
/// capture rewind correctly" actually requires.
///
/// It runs at the HEAD of the continuity chain so a conversation that ended this
/// frame is not first judged for separation and barked about on its way out.
///
/// **what this does NOT make deterministic**: the Yarn runner itself. It is
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

/// **The text box shows whatever conversation the simulation is having.**
/// (presentation)
///
/// The snapshot deliberately does not rewind the text box; replaying the side effect rewound it
/// anyway.
///
/// The simulation went on holding the talker and capturing a seat while the player looked at
/// nothing.
///
/// **opening and closing are ONE system for that reason.** They were two, and
/// the closing half wrote no bookkeeping at all, so presentation's record of what
/// it had done outlived the thing it was a record of. A projection is current
/// derived state or it is not a projection.
///
/// the three cases, and each one is a rule:
///
/// * **no authority** → close the box AND detach. Nothing is being said.
/// * **a conversation this box is not attached to** → start the runner. This
///   includes the same node entered again, which is a conversation the player has
///   not seen.
/// * **the conversation this box is already attached to** → nothing. A rewind
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
