//! Sandbox dialogue runtime and UI — module facade over `dialog/`.
//!
//! This root file is the module entry: it declares and re-exports the
//! `dialog/` submodules ([`runtime`], [`systems`], [`content`],
//! [`yarn_bridge`], [`yarn_bindings`]). No dialogue logic lives here
//! directly — it all sits in `dialog/`.
//!
//! Authored conversation content is CONTENT (R3.2): the game's `.yarn`
//! set, the Yarn Spinner plugin constructor, and the validator's
//! known-dialogue-id surface live in `ambition_content::dialogue::yarn`.
//! This module owns only the engine-side runtime:
//!
//! - [`runtime::DialogState`] — Bevy resource view-model written
//!   by the Yarn bridge, read by the existing custom UI.
//! - [`systems::dialog_input`] / [`systems::dialog_pointer_input`] —
//!   player-input translators that write `pending_*` request fields
//!   on `DialogState`.
//! - [`systems::dialog_reveal_tick`] — presentation timer that
//!   advances the visible substring of the current line.
//! - The dialog-box renderer (poll-based, reads `DialogState`) lives in
//!   the `ambition_render` crate (`ambition_render::dialog_ui`), not here.
//! - [`yarn_bridge`] — observers + dispatch that route runner
//!   events into `DialogState` writes and `DialogState` requests
//!   into runner calls.
//! - [`yarn_bindings`] — custom commands / functions / markup cues
//!   that authored `.yarn` content can invoke.

mod content;
mod runtime;
mod speech_sfx;
mod systems;
// The dialog-box overlay UI moved to the `ambition_render` crate
// (`ambition_render::dialog_ui`); the sim-side dialog state/logic stays here.
/// Public so content plugins can reach the [`yarn_bindings::YarnContentBindings`]
/// installer seam + the mirror types.
#[cfg(feature = "ui")]
pub mod yarn_bindings;
#[cfg(feature = "ui")]
mod yarn_bridge;

#[allow(
    unused_imports,
    reason = "DialogChoice surfaces in the UI's choice-row layout"
)]
pub use content::DialogChoice;
pub use runtime::{DialogChoiceSlot, DialogState};
pub use systems::{dialog_input, dialog_pointer_input, dialog_reveal_tick};
#[cfg(feature = "ui")]
pub use yarn_bindings::YarnBindingsPlugin;
#[cfg(feature = "ui")]
#[allow(
    unused_imports,
    reason = "DialogueRunnerEntity is exported for ad-hoc tooling and future tests"
)]
pub use yarn_bridge::{DialogueRunnerEntity, YarnBridgePlugin};

#[cfg(test)]
mod tests;
