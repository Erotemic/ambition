//! Conversation continuity and rollback authority.
//!
//! [`ActiveConversation`] is the simulation authority for a live conversation.
//! Holds, UI state, and other presentation are projections rebuilt from it.
//! Gameplay-bearing narrative outputs cross back through the narrative-input
//! ledger with the conversation identity and simulation tick attached.
//!
//! This crate does not derive the `ParticipantId`/player-slot correspondence; the
//! caller supplies [`ConversationInputOwner`]. Payload vocabulary remains owned by
//! the domain that consumes it, while [`ConversationPlugin`] owns conversation
//! registration and ordering.
//!
//! ⛔⛔ WHAT THIS CRATE REFUSES.
//!
//! - **Presentation.** A live conversation's UI, reveal timing and overlay are
//!   projections rebuilt FROM [`ActiveConversation`], and they belong to
//!   `ambition_dialog` — which this crate depends on and which declares itself
//!   content-free for the same reason.
//! - **Named content.** No line of dialogue, no speaker, no barks table. Banter
//!   arrived here as the RULE for when characters may talk over each other; the
//!   words are a game content crate's.
//! - **Who is speaking to whom.** The caller supplies
//!   [`ConversationInputOwner`]; deriving it from a player slot here would make
//!   this crate reason about seats it cannot see.

mod authority;
// ⛔ NOT GATED, and it must not be: the COMBAT hit path reads this, and the hit
// path exists in a headless sim with no `ui`. It sits below the `dialog` line
// rather than above it because that line's `#[cfg]` belongs to `dialog`.
pub mod banter;
// Session/UI glue that projects the conversation authority into `ambition_dialog`.
#[cfg(feature = "ui")]
pub mod dialog;
mod hold;
mod instance;
mod ledger;
mod music;
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
pub use music::NarrativeMusicRequest;
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
