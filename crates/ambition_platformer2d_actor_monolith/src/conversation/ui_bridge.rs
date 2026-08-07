//! **The seam between the authority and the text box.**
//!
//! A conversation has TWO enders and they come from opposite directions: the
//! simulation can break it (somebody was knocked away, somebody walked off), and
//! the NARRATIVE can finish it (the Yarn runner ran out of lines). Neither can
//! be derived from the other, so the seam is explicit and each direction is one
//! system with one job.
//!
//! ⭐ **and the seam is the WHOLE crossing now — both ways** (GPT 5.6,
//! 2026-08-07, finding 2). It used to be half a seam:
//!
//! ```text
//! before                              now
//! ──────────────────────────────      ──────────────────────────────
//! sim  → runner.start_node   (!)      sim  → ActiveConversation
//! runner-ended → message → sim (!)      ↳ presentation projects the box
//!                                     runner-ended → STAMPED record → sim
//! ```
//!
//! Both starred lines were simulation-significant crossings with no replay
//! story:
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
//! ⭐ **so opening is a PROJECTION and ending is a STAMPED EXTERNAL INPUT.** The
//! simulation decides that a conversation exists; presentation reads that and
//! opens the runner. The narrative decides that it is over; that observation is
//! recorded with the tick it applies from, and the record is not rewound —
//! because it is the record of what arrived from outside, and rewinding an input
//! erases it. A resimulated tick therefore reaches the same decision at the same
//! `SimTick`, which is the whole invariant.

use bevy::prelude::*;

use super::authority::ActiveConversation;

/// **The narrative ran out of lines, and the tick that fact applies from.**
///
/// ⛔ **this replaced a `Message`, and the difference is what makes it
/// replayable.** A message is delivered once and cleared on rollback, and the
/// system that would re-deliver it — a presentation system watching the live
/// runner — does not execute between resimulated ticks. So a rewind past the end
/// simply lost it: the conversation stayed live through every replayed tick it
/// had already finished in, holding a body and capturing a seat, and
/// presentation ended it again afterwards at a different simulation time.
///
/// ⚠ **NOT rollback state, deliberately, and the reason is the same one that
/// keeps device input out of the snapshot.** This is the record of what arrived
/// from OUTSIDE the simulation. A rewind restores what the simulation decided;
/// erasing what it was told is how the replay reaches a different decision.
///
/// ⚠ **keyed on the conversation INSTANCE** — the node AND the tick it opened
/// on. A bare node id would let a finished conversation's end close the next
/// conversation of the same node, which is the bug the message's `dialogue_id`
/// was already carrying a field to avoid.
///
/// ⚠ **depth ONE, and that is a judgement rather than an oversight.** A second
/// conversation's end overwrites the first's, so a rewind reaching back past two
/// completed conversations would replay only the later one. The prediction
/// window is eight frames; two conversations cannot open and finish inside it,
/// because a player has to read the first. A queue would be machinery for a
/// state that needs somebody to read two text boxes in an eighth of a second.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct ObservedNarrativeEnd {
    last: Option<NarrativeEnd>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NarrativeEnd {
    dialogue_id: String,
    /// Which conversation instance this ends — see [`super::LiveConversation::opened_at`].
    opened_at: u64,
    /// The first `SimTick` on which the simulation may act on it.
    ///
    /// ⭐ the NEXT tick, because presentation observes the runner in `Update`,
    /// after this frame's simulation has already run. Stamping the tick that has
    /// been simulated would make the original frame and its replay disagree
    /// about whether the conversation was live during it — the same off-by-one
    /// `PreparedMatch::effective_from` exists to close.
    from_tick: u64,
}

impl ObservedNarrativeEnd {
    /// Record that the narrative finished `live`, effective from `from_tick`.
    pub fn record(&mut self, live: &super::LiveConversation, from_tick: u64) {
        self.last = Some(NarrativeEnd {
            dialogue_id: live.dialogue_id.clone(),
            opened_at: live.opened_at,
            from_tick,
        });
    }

    /// Whether this record already names `live`. ⚠ the observing system runs
    /// every frame until the simulation acts, and re-stamping a later tick each
    /// time would push the end forward and make a rewind land on a different
    /// answer than the original run.
    pub fn already_names(&self, live: &super::LiveConversation) -> bool {
        self.last.as_ref().is_some_and(|end| {
            end.dialogue_id == live.dialogue_id && end.opened_at == live.opened_at
        })
    }

    /// Whether the simulation should close `live` as of `now`.
    pub fn ends(&self, live: &super::LiveConversation, now: u64) -> bool {
        self.already_names(live) && self.last.as_ref().is_some_and(|end| end.from_tick <= now)
    }

    /// Forget it.
    ///
    /// ⛔ **not callable from the simulation**, and that is not a style rule: a
    /// replayable system that erased an external input is the exact defect this
    /// type replaced, one level down. Nothing in the shipped schedule calls it —
    /// a record survives its conversation harmlessly, because it applies only to
    /// a live conversation matching BOTH its node and the tick that conversation
    /// opened on, and by the time a later conversation could reproduce that pair
    /// the record's own `from_tick` has long since passed. This exists for a
    /// caller outside the timeline (a test, a tool) that wants a clean slate.
    pub fn forget(&mut self) {
        self.last = None;
    }
}

/// **Tell the simulation the narrative finished.** (presentation)
///
/// Runs outside the simulation schedule, where reading the live runner is
/// exactly right: this IS the moment the runner finished, observed once, in real
/// time.
///
/// ⚠ **observed once per conversation instance**, and the idempotence is
/// load-bearing rather than an optimization: the condition stays true until the
/// authority closes, and re-stamping a later tick on each of those frames would
/// mean a rewind replays an end the original run applied earlier.
pub fn publish_the_narrative_end(
    conversation: Res<ActiveConversation>,
    dialog: Res<ambition_dialog::DialogState>,
    // `Option` for the reason `prepare_the_match`'s own tick is: a composition
    // with no timeline has no replay to disagree with, and a plain `Res` would
    // panic in every one of them.
    tick: Option<Res<ambition_time::SimTick>>,
    mut observed: ResMut<ObservedNarrativeEnd>,
) {
    let Some(live) = conversation.live() else {
        return;
    };
    if dialog.active() || observed.already_names(live) {
        return;
    }
    let from_tick = tick.map_or(0, |tick| tick.0.saturating_add(1));
    observed.record(live, from_tick);
}

/// **The narrative finished, so the simulation's conversation is over.** (sim)
///
/// ⭐ **it reads a stamped record, not a delivered event.** A resimulated tick
/// asks the same question of the same record and gets the same answer, so the
/// conversation ends at the same `SimTick` in the replay as it did in the
/// original run — which is what "the hold, the scripted control and the input
/// capture rewind correctly" actually requires.
///
/// It runs at the HEAD of the continuity chain so a conversation that ended this
/// frame is not first judged for separation and barked about on its way out.
///
/// ⚠ **what this does NOT make deterministic**: the Yarn runner itself. It is
/// content executing outside the simulation, so WHICH tick it finishes on is
/// still decided by presentation. What is now true is that whichever tick it
/// finished on, every replay of that tick agrees.
pub fn close_conversation_on_narrative_end(
    mut conversation: ResMut<ActiveConversation>,
    observed: Res<ObservedNarrativeEnd>,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    // No timeline is no replay: a composition without `SimTick` acts on the
    // record the moment it sees it, which is what the message-based seam did.
    let now = tick.map_or(u64::MAX, |tick| tick.0);
    let ends = conversation
        .live()
        .is_some_and(|live| observed.ends(live, now));
    if ends {
        conversation.close();
    }
}

/// **The simulation opened a conversation, so the text box shows it.**
/// (presentation)
///
/// ⛔ **this used to be a `DialogState::start` call inside the interaction
/// system**, which is a simulation system: it runs in the sim schedule, so a
/// rollback across the tick somebody pressed Interact REPLAYS it — resetting the
/// line, the options and the typewriter of a box the player is already reading,
/// and enqueueing a second `runner.start_node`. The snapshot deliberately does
/// not rewind the text box; replaying the side effect rewound it anyway.
///
/// ⭐ **the memo is what makes a rewind quiet.** A rollback restores the
/// authority with the SAME `opened_at`, so this recognises the conversation it
/// already opened and does nothing. A conversation opened on a different tick —
/// including the same node entered again in the replayed timeline — is a
/// different instance and does start the runner, which is correct: that is a
/// conversation the player has not seen.
///
/// ⚠ the memo is a `Local`, and it belongs there: it is presentation's record of
/// what presentation has done, on the side of the seam that is not rewound.
pub fn open_dialog_ui_when_the_conversation_starts(
    conversation: Res<ActiveConversation>,
    mut dialog: ResMut<ambition_dialog::DialogState>,
    mut opened: Local<Option<(u64, String)>>,
) {
    let Some(live) = conversation.live() else {
        return;
    };
    let instance = (live.opened_at, live.dialogue_id.clone());
    if opened.as_ref() == Some(&instance) {
        return;
    }
    *opened = Some(instance);
    dialog.start(&live.dialogue_id, &live.speaker_name, live.context.clone());
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
