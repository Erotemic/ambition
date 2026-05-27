//! Sandbox dialogue runtime and UI.
//!
//! Authored conversation content lives in
//! `assets/dialogue/sandbox/*.yarn` and is loaded by
//! `bevy_yarnspinner`. The runtime is split across:
//!
//! - [`runtime::DialogState`] — Bevy resource view-model written
//!   by the Yarn bridge, read by the existing custom UI.
//! - [`systems::dialog_input`] / [`systems::dialog_pointer_input`] —
//!   player-input translators that write `pending_*` request fields
//!   on `DialogState`.
//! - [`ui::sync_dialog_ui`] — renderer (poll-based, reads
//!   `DialogState`).
//! - [`yarn_bridge`] — observers + dispatch that route runner
//!   events into `DialogState` writes and `DialogState` requests
//!   into runner calls.
//! - [`yarn_bindings`] — custom commands / functions / markup cues
//!   that authored `.yarn` content can invoke.

mod content;
mod runtime;
mod systems;
mod ui;
#[cfg(feature = "ui")]
mod yarn_bindings;
#[cfg(feature = "ui")]
mod yarn_bridge;

pub(crate) use content::known_dialogue_ids;
#[allow(unused_imports, reason = "DialogChoice surfaces in the UI's choice-row layout")]
pub use content::DialogChoice;
pub use runtime::DialogState;
pub use systems::{dialog_input, dialog_pointer_input};
pub use ui::sync_dialog_ui;
#[cfg(feature = "ui")]
pub use yarn_bindings::YarnBindingsPlugin;
#[cfg(feature = "ui")]
#[allow(
    unused_imports,
    reason = "DialogueRunnerEntity is exported for ad-hoc tooling and future tests"
)]
pub use yarn_bridge::{DialogueRunnerEntity, YarnBridgePlugin};

#[cfg(feature = "ui")]
use bevy_yarnspinner::prelude::*;

/// Marker plugin: registers Yarn Spinner so dialogue assets and future Yarn
/// runners are available, while keeping this first sandbox dialogue view
/// intentionally custom and game-feel oriented. Gated behind the `ui`
/// feature; the rest of this module's dialogue runtime + custom Bevy UI
/// view does not depend on Yarn Spinner.
#[cfg(feature = "ui")]
pub fn yarn_spinner_plugin() -> YarnSpinnerPlugin {
    // Android cannot enumerate asset folders inside the APK, so use
    // explicit Yarn sources instead of `YarnSpinnerPlugin::new()`
    // (which scans the dialogue folder on desktop builds). Paths
    // relative to Bevy's asset root
    // (`crates/ambition_sandbox/assets/`). One file per zone — the
    // content-swap unit for a future fork is the whole
    // `dialogue/<game_id>/` directory.
    YarnSpinnerPlugin::with_yarn_sources([
        YarnFileSource::file("dialogue/sandbox/intro.yarn"),
        YarnFileSource::file("dialogue/sandbox/kernel.yarn"),
        YarnFileSource::file("dialogue/sandbox/factions.yarn"),
        YarnFileSource::file("dialogue/sandbox/cove.yarn"),
        YarnFileSource::file("dialogue/sandbox/dojo.yarn"),
    ])
}

#[cfg(test)]
mod tests;
