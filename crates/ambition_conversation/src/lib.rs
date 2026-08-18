//! **Conversation continuity: the authority, the hold, and the break rule.**
//!
//! ⛔ **this was three places, and the split was the bug.** The break rule and the hold lived inside
//! `features/ecs/interact.rs` — a module whose subject is "player presses
//! Interact at a switch or an NPC" — while the state they reasoned about lived
//! in `ambition_dialog::DialogState`, a UI read-model that is not rewound. A
//! conversation was therefore decided by two authorities with different rollback
//! lifetimes, and a rewind could leave a body held by one and free by the other.
//!
//! So the simulation gets ONE authority, [`ActiveConversation`], and everything
//! else about a live conversation is a PROJECTION rebuilt from it each tick. A
//! projection cannot outlive a rewind its authority did not.
//!
//! ## How this left the actor monolith, and what the departure cost
//!
//! ✔ **CARVED 2026-08-17 (D33 step 2).** This was a module inside the actor
//! monolith for its whole life until then, written to be liftable on Jon's instruction (2026-08-07: *"if we add
//! things to the monolith, try to do it so it's obvious what the decomposition
//! should be"*). It came out as a `Cargo.toml`, a `mod.rs` → `lib.rs` rename and
//! a pile of `crate::conversation::` → `ambition_conversation::` path rewrites
//! at the CALLERS. Not one line inside these files changed shape. That was the
//! point, and it took three prior passes to earn:
//!
//! - **the BARK port (2026-08-08).** [`rules`] named
//!   `crate::features::npcs::npc_ambient_bark_line` and
//!   `crate::character_runtime::PreparedCharacterRegistry` — both answering
//!   *"what line does this character say"*, a CAST question, not a continuity
//!   one. [`rules::ConversationCutBark`] is the channel that replaced them:
//!   continuity says WHO should speak, the cast answers WHAT they say, on the
//!   same tick. ⭐ it took the `ambition_vfx` edge with it — the bubble is the
//!   responder's to write.
//! - **the third edge that ALMOST landed, and is the instructive one.**
//!   [`opening`] arrived deriving [`ConversationInputOwner`] itself, which meant
//!   calling `crate::participant_seat::participant_of` — making this a second
//!   owner of the `ParticipantId` ↔ `PlayerSlot` correspondence that module
//!   exists to keep in ONE place. It takes the owner as a parameter instead.
//!   ⛔⛔ **that is a STANDING constraint, not a historical note: this crate must
//!   never re-acquire that correspondence.** It exists because `ambition_input`
//!   and `ambition_characters` are siblings that cannot see each other — see the
//!   `SessionSeatId`/`ControlChannelId` row in `docs/planning/tracks.md` — and a
//!   crate that re-derives it becomes a second authority on which pad drives
//!   which body.
//! - **the SCHEDULE pin (2026-08-15/16).** ⛔⛔ *"the carve is a `Cargo.toml`"*
//!   is what this header said from 2026-08-08, and it was FALSE for a reason an
//!   import count cannot see. `features::FeatureInteractionSchedulePlugin` owned
//!   every registration this module had — [`ActiveConversation`],
//!   [`ConversationCutBark`], the `NarrativeInputPlugin` installs, the systems —
//!   and interleaved three of them into ONE anonymous `.chain()` with
//!   `interact_ecs_actors_and_switches`, `npcs::speak_conversation_cut_barks`
//!   and the chest/breakable systems, every interleave load-bearing and
//!   documented only in prose at the call site.
//!   ⭐ **the generalisable lesson: a module with zero inward imports can still
//!   be pinned by the SCHEDULE. Count the registrations, not only the paths.**
//!   [`ConversationPlugin`] owns them now, and the order is stated as
//!   [`FeatureInteractionSet`](ambition_platformer2d_shared_tangle::schedule::FeatureInteractionSet).
//!   ⭐⭐ that vocabulary lives in `ambition_platformer2d_shared_tangle`, BELOW
//!   the monolith, **on purpose**: a set enum in `features` would have re-pinned
//!   this module by the schedule the moment it stopped importing `features`, the
//!   same bug one level up. ⇒ **when you name an ordering so a module can leave,
//!   the NAME has to live somewhere the module can still reach after it has
//!   left.**
//!
//! ⚠ **the per-payload `NarrativeInputPlugin::<T>` installs did NOT all come.**
//! `T` is sometimes a `features` type this crate cannot name, and a payload
//! belongs to whoever CONSUMES it, so only [`ConversationEnded`] travelled. See
//! [`plugin`] for the reasoning. **Conversation provides the ledger MECHANISM;
//! it does not decide another domain's vocabulary.**
//!
//! ⛔ **a measurement trap this carve nearly died on.** `grep -r "crate::"` over
//! these files reported edges to `participant_seat`, `features`,
//! `character_runtime`, `items` and `dialog` — **every one a DOC COMMENT**. This
//! repository's `//!` blocks cite paths densely enough that a path-grep measures
//! PROSE. ⇒ **measure `use` statements, never `crate::` occurrences.**
//!
//! ⚠ **what the carve did NOT buy, measured rather than hoped.** It removes no
//! capability from a movement-only game's resolved graph: the monolith depends
//! on this crate unconditionally, so `ambition_dialog` and `ambition_ui_nav`
//! still arrive through it. The win is compile isolation for every edit that is
//! NOT a conversation edit — editing this crate still rebuilds everything above
//! it. See `docs/planning/engine/actor-monolith-decomposition.md` §C4e.
//!
//! ## The files
//!
//! - [`instance`] — WHICH conversation this is, in a form a corrected timeline
//!   agrees with. Content-derived, so a resimulation re-mints it.
//! - [`authority`] — what the simulation believes about the live conversation.
//!   Rollback-owned. Reads nothing.
//! - [`ledger`] — what the NARRATIVE told the simulation, stamped with the tick
//!   it applies from. The mirror image of the effect quarantine, and the one
//!   crossing every gameplay-bearing Yarn command goes through.
//! - [`opening`] — deciding a conversation happens and opening it. The half of
//!   "somebody pressed Interact" that is about DIALOGUE, moved out of
//!   `features/ecs/interact.rs` so `features` names no dialogue type.
//! - [`hold`] — the projection: which body is standing still because it is being
//!   talked to, rebuilt from the authority every tick.
//! - [`rules`] — when a conversation ENDS, and the bark that says so.
//! - [`ui_bridge`] — the text box as a projection, and the narrative end as the
//!   ledger's first payload.
//! - [`plugin`] — what this module registers, and the phases its systems name.

mod authority;
// ⭐ **the dialogue HOST GLUE joined its domain, 2026-08-17 (D33).** These
// plugins wire `ambition_dialog`'s runtime into a session: the binding
// installer seam, the input/reveal presentation pair, and the mapping from
// "the conversation ended" back onto `GameMode::Playing`. They lived in the
// actor monolith because that is where the session mode lived; they belong
// beside the conversation authority that decides when a dialogue is live.
#[cfg(feature = "ui")]
pub mod dialog;
mod hold;
mod instance;
mod ledger;
mod opening;
mod plugin;
mod rules;
mod ui_bridge;

#[cfg(test)]
mod tests;

pub use authority::{ActiveConversation, ConversationInputOwner, LiveConversation};
pub use hold::{project_conversation_hold, HeldByConversation};
pub use instance::ConversationInstanceId;
pub use ledger::{
    release_narrative_inputs, NarrativeInputLedger, NarrativeInputPlugin, NarrativeInputWriter,
};
pub use opening::{character_id_of, DialogueDispatch};
pub use plugin::ConversationPlugin;
pub use rules::{break_dialogue_on_hit_or_separation, ConversationCutBark};
pub use ui_bridge::{
    close_conversation_on_narrative_end, project_the_dialog_ui_from_the_conversation,
    publish_the_narrative_end, ConversationEnded,
};

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
