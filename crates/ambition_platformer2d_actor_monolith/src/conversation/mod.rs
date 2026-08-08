//! **Conversation continuity: the authority, the hold, and the break rule.**
//!
//! ⛔ **this was three places, and the split was the bug** (GPT 5.6 review
//! through `c32e690`, finding 1). The break rule and the hold lived inside
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
//! - **Outward edges are all crates BELOW the monolith already**:
//!   `ambition_dialog` (`DialogueBreak`, the pure rule),
//!   `ambition_platformer2d_core` (`CenteredAabb`, `Vec2`),
//!   `ambition_platformer2d_shared_tangle` (`BodyKinematics`),
//!   `ambition_combat` (`ActorInteraction`), `ambition_characters`
//!   (`ScriptedControl`, `BodyCombat`, the catalog), `ambition_vfx`, and
//!   `ambition_input` (`ParticipantId`). None of them depend on the monolith, so
//!   none would cycle.
//! - **⚠ TWO inward edges remain, and both are the BARK**:
//!   `crate::features::npcs::npc_ambient_bark_line` and
//!   `crate::character_runtime::PreparedCharacterRegistry`. Neither is about
//!   continuity — they answer "what line does this character say", which is a
//!   CAST question. A carve would put a small port here ("give me a bark for
//!   this character in this situation") and leave the cast lookup behind.
//!
//! That is the whole list. Nothing else in this module reaches into the
//! monolith, and the carve is a port plus a `Cargo.toml`.
//!
//! ## The three files
//!
//! - [`instance`] — WHICH conversation this is, in a form a corrected timeline
//!   agrees with. Content-derived, so a resimulation re-mints it.
//! - [`authority`] — what the simulation believes about the live conversation.
//!   Rollback-owned. Reads nothing.
//! - [`ledger`] — what the NARRATIVE told the simulation, stamped with the tick
//!   it applies from. The mirror image of the effect quarantine, and the one
//!   crossing every gameplay-bearing Yarn command goes through.
//! - [`hold`] — the projection: which body is standing still because it is being
//!   talked to, rebuilt from the authority every tick.
//! - [`rules`] — when a conversation ENDS, and the bark that says so.
//! - [`ui_bridge`] — the text box as a projection, and the narrative end as the
//!   ledger's first payload.

mod authority;
mod hold;
mod instance;
mod ledger;
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
pub use rules::break_dialogue_on_hit_or_separation;
pub use ui_bridge::{
    close_conversation_on_narrative_end, project_the_dialog_ui_from_the_conversation,
    publish_the_narrative_end, ConversationEnded,
};
