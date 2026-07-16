//! Reusable dialogue runtime (E1c carve out of `ambition_actors`).
//!
//! Owns the engine-side dialogue machinery, content-free:
//!
//! - [`DialogState`] — the poll-based UI read model. Callers mutate it through
//!   pure methods (`start`, `close`, `confirm_or_advance`, `select_delta`); the
//!   bridge drains its `pending_*` requests into the live `DialogueRunner` and
//!   writes runner events back.
//! - [`dialog_reveal_tick`] / [`dialog_input`] / [`dialog_pointer_input`] — the
//!   typewriter reveal + input translators (the last two `input`-gated).
//! - [`DialogueVoiceCatalog`] — the App-local seam through which a provider
//!   contributes its cast voiceprints without naming characters here.
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
mod context;
mod runtime;
mod speech_sfx;
mod systems;

// The Yarn runner bridge + binding-installer machinery need `bevy_yarnspinner`.
// The dialog-box overlay UI itself lives in the `ambition_render` crate.
#[cfg(feature = "ui")]
mod bindings;
#[cfg(feature = "ui")]
mod bridge;

pub use ambition_ui_nav::DialogChoiceSlot;
#[allow(
    unused_imports,
    reason = "DialogChoice surfaces in the UI's choice-row layout"
)]
pub use content::DialogChoice;
pub use context::{DialogueContext, DialogueNodeIndex, SELF_NODE_SUFFIX};
pub use runtime::DialogState;
pub use speech_sfx::{DialogueVoiceCatalog, DialogueVoiceCatalogAppExt, DialogueVoiceCatalogError};
pub use systems::{dialog_input, dialog_pointer_input, dialog_reveal_tick};

/// The dialogue DOMAIN's sim-state plugin (track 6, decision #9): this crate
/// owns its local resources; the sim assembly only adds the plugin. `ui`-free
/// on purpose — the poll-based state model exists headless; the Yarn bridge
/// fills it in visible builds.
pub struct DialogSimStatePlugin;

impl bevy::prelude::Plugin for DialogSimStatePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<runtime::DialogState>();
        // Which Yarn nodes content compiled (empty + unpopulated headless).
        // The interact dispatcher reads it to resolve a self-conversation's
        // branch; the Yarn bridge fills it when the runner spawns.
        app.init_resource::<context::DialogueNodeIndex>();
    }
}

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
