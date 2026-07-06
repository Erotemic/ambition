//! Reusable dialogue runtime (E1c carve out of `ambition_gameplay_core`).
//!
//! Owns the engine-side dialogue machinery, content-free:
//!
//! - [`DialogState`] — the poll-based UI read model. Callers mutate it through
//!   pure methods (`start`, `close`, `confirm_or_advance`, `select_delta`); the
//!   bridge drains its `pending_*` requests into the live `DialogueRunner` and
//!   writes runner events back.
//! - [`dialog_reveal_tick`] / [`dialog_input`] / [`dialog_pointer_input`] — the
//!   typewriter reveal + input translators (the last two `input`-gated).
//! - [`YarnBridgePlugin`] — the `bevy_yarnspinner` ↔ `DialogState` bridge, and
//!   the [`YarnContentBindings`] installer seam a host registers its
//!   game-specific Yarn vocabulary through (`ui`-gated).
//!
//! ## What stays host-side
//!
//! The game's Yarn *bindings* (commands like `<<give_item>>`, functions like
//! `<<if boss_cleared("x")>>`) reference actor/save state, so they live in the
//! host and register through [`YarnContentBindings`]. The runtime has no notion
//! of a host "game mode": it flips [`DialogState::active`] and the host maps
//! that onto its own session state. This is the seam that lets the dialogue
//! runtime be reused by another game.

mod content;
mod runtime;
mod speech_sfx;
mod systems;

// The Yarn runner bridge + binding-installer machinery need `bevy_yarnspinner`.
// The dialog-box overlay UI itself lives in the `ambition_render` crate.
#[cfg(feature = "ui")]
mod bindings;
#[cfg(feature = "ui")]
mod bridge;

#[allow(
    unused_imports,
    reason = "DialogChoice surfaces in the UI's choice-row layout"
)]
pub use content::DialogChoice;
pub use runtime::{DialogChoiceSlot, DialogState};
pub use systems::{dialog_input, dialog_pointer_input, dialog_reveal_tick};

#[cfg(feature = "ui")]
pub use bindings::{
    clear_yarn_presentation_cue, YarnBindingInstaller, YarnBindingsPlugin, YarnContentBindings,
    YarnPresentationCue, YarnStateMirror, YarnStateMirrorData,
};
#[cfg(feature = "ui")]
#[allow(
    unused_imports,
    reason = "DialogueRunnerEntity is exported for ad-hoc tooling and future tests"
)]
pub use bridge::{DialogueRunnerEntity, YarnBridgePlugin};

#[cfg(test)]
mod tests;
