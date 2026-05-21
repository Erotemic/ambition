//! Sandbox dialogue runtime and UI.
//!
//! This module is intentionally a stable facade: public callers continue to use
//! `crate::dialog::*`, while the implementation lives in focused child modules.
//! Runtime state, authored content, Bevy input systems, and UI construction are
//! split so dialogue changes do not require loading one large mixed-concern file.
//!
//! Authored conversation data is still held in code-side tables for now. The app
//! also registers `bevy_yarnspinner` and includes Yarn source files so content
//! can migrate to real Yarn nodes without changing NPC/merchant-facing gameplay
//! semantics.

mod content;
mod runtime;
mod systems;
mod ui;

pub(crate) use content::known_dialogue_ids;
pub(crate) use content::DialogMode;
pub use content::{DialogChoice, DialogNode};
pub use runtime::DialogState;
pub use systems::{dialog_input, dialog_pointer_input, redirect_post_quest_dialog};
pub use ui::sync_dialog_ui;

#[cfg(feature = "ui")]
use bevy_yarnspinner::prelude::*;

/// Marker plugin: registers Yarn Spinner so dialogue assets and future Yarn
/// runners are available, while keeping this first sandbox dialogue view
/// intentionally custom and game-feel oriented. Gated behind the `ui`
/// feature; the rest of this module's dialogue runtime + custom Bevy UI
/// view does not depend on Yarn Spinner.
#[cfg(feature = "ui")]
pub fn yarn_spinner_plugin() -> YarnSpinnerPlugin {
    // Android cannot enumerate asset folders inside the APK, so use an
    // explicit Yarn source instead of YarnSpinnerPlugin::new() (which scans
    // the dialogue folder on desktop builds). Keep this path relative to
    // Bevy's asset root: crates/ambition_sandbox/assets/dialogue/...
    YarnSpinnerPlugin::with_yarn_source(YarnFileSource::file("dialogue/ambition_sandbox.yarn"))
}

#[cfg(test)]
mod tests;
