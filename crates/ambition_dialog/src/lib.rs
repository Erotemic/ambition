//! Reusable, content-free dialogue runtime.
//!
//! Owns dialog state, reveal/input translation, voice registration, and the Yarn
//! bridge. Game-specific Yarn commands/functions stay host-side and register
//! through `YarnContentBindings`; hosts map `DialogState::active` onto their own
//! session state.

mod content;
mod context;
mod continuity;
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
pub use continuity::DialogueBreak;
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
    YarnPresentationCue, YarnPresentationCueCleared, YarnStateMirror, YarnStateMirrorData,
    YarnStateMirrorRefreshed,
};
#[cfg(feature = "ui")]
#[allow(
    unused_imports,
    reason = "DialogueRunnerEntity is exported for ad-hoc tooling and future tests"
)]
pub use bridge::{DialogueRunnerEntity, YarnBridgePlugin};

#[cfg(test)]
mod tests;
