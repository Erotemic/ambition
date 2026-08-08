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
//! ## What this module would take to become `ambition_conversation`
//!
//! Recorded because the monolith's decomposition is coming (Jon, 2026-08-07:
//! *"if we add things to the monolith, try to do it so it's obvious what the
//! decomposition should be"*). This module was written to be liftable, and the
//! honest accounting is:
//!
//! ⚠ **RE-DERIVED 2026-08-08, from source rather than from this paragraph.**
//! Prose accounting goes stale silently: the ledger and `opening` both landed
//! after it was written, and `opening` DID add a third inward edge before it was
//! measured and removed. Re-derive with
//! `grep -oE "crate::[a-z_]+::[A-Za-z_]+" conversation/*.rs` before trusting it.
//!
//! - **Outward edges are all crates BELOW the monolith already**:
//!   `ambition_dialog` (`DialogueBreak`, `DialogueContext`, `DialogueNodeIndex`,
//!   the pure rule), `ambition_platformer2d_core` (`CenteredAabb`, `Vec2`,
//!   `ConfirmedFrameBoundary`), `ambition_platformer2d_shared_tangle`
//!   (`BodyKinematics`, `SimId`, the schedule sets), `ambition_time` (`SimTick`),
//!   `ambition_combat` (`ActorInteraction`, `ActorIdentity`),
//!   `ambition_characters` (`ScriptedControl`, `BodyCombat`, `Brain`, the
//!   catalog), `ambition_interaction`, `ambition_vfx`, and `ambition_input`
//!   (`ParticipantId`). None of them depend on the monolith, so none would cycle.
//! - **⭐ ZERO inward edges, as of 2026-08-08.** There were two, both in
//!   [`rules`], and both were the BARK:
//!   `crate::features::npcs::npc_ambient_bark_line` and
//!   `crate::character_runtime::PreparedCharacterRegistry`. Neither was about
//!   continuity — they answer "what line does this character say", which is a
//!   CAST question needing the catalog, the prepared registry and the
//!   `Interactable` → character-id resolution. [`rules::ConversationCutBark`] is
//!   the port that replaced them: continuity says WHO should speak, the cast
//!   answers WHAT they say, on the same tick.
//!   ⭐ it took `ambition_vfx` with it — the bubble is the responder's to write.
//!
//! ⛔ **and the third edge that ALMOST landed is the instructive one.**
//! [`opening`] arrived deriving `ConversationInputOwner` itself, which meant
//! calling `crate::participant_seat::participant_of` — making this module a
//! second owner of the `ParticipantId` ↔ `PlayerSlot` correspondence that
//! module exists to keep in ONE place. It hands the caller a `PlayerSlot` and
//! takes the owner as a parameter instead. ⚠ **that correspondence is a carve
//! hazard for anything leaving this crate**, because it exists precisely because
//! `ambition_input` and `ambition_characters` are siblings that cannot see each
//! other — see the `SessionSeatId`/`ControlChannelId` row in
//! `docs/planning/tracks.md`.
//!
//! That is the whole list, and it is now empty: **nothing in this module reaches
//! into the monolith at all.** The carve is a `Cargo.toml` — see the five
//! ordered steps in `docs/planning/engine/actor-monolith-decomposition.md`, of
//! which step 1 (this port) is done.
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

mod authority;
mod hold;
mod instance;
mod ledger;
mod opening;
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
pub use rules::{break_dialogue_on_hit_or_separation, ConversationCutBark};
pub use ui_bridge::{
    close_conversation_on_narrative_end, project_the_dialog_ui_from_the_conversation,
    publish_the_narrative_end, ConversationEnded,
};
