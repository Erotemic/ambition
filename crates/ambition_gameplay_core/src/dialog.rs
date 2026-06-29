//! Sandbox dialogue runtime and UI — module facade over `dialog/`.
//!
//! This root file is the module entry: it declares and re-exports the
//! `dialog/` submodules ([`runtime`], [`systems`], [`content`],
//! [`yarn_bridge`], [`yarn_bindings`]) and owns the Yarn Spinner plugin
//! wiring. No dialogue logic lives here directly — it all sits in `dialog/`.
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

pub use content::known_dialogue_ids;
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

#[cfg(feature = "ui")]
use bevy_yarnspinner::prelude::*;

/// Marker plugin: registers Yarn Spinner so dialogue assets and future Yarn
/// runners are available, while keeping this first sandbox dialogue view
/// intentionally custom and game-feel oriented. Gated behind the `ui`
/// feature; the rest of this module's dialogue runtime + custom Bevy UI
/// view does not depend on Yarn Spinner.
/// The Yarn dialogue files the game loads, as paths relative to Bevy's asset
/// root (`crates/ambition_gameplay_core/assets/`). One file per zone — the
/// content-swap unit for a future fork is the whole `dialogue/<game_id>/`
/// directory.
///
/// Single source of truth: [`yarn_spinner_plugin`] registers exactly these, and
/// the `dialog_yarn_compile` test compiles exactly these (as one project, the
/// way startup does). A new `.yarn` added here is automatically covered; a file
/// dropped on disk but not listed is intentionally unloaded.
pub(crate) const YARN_SOURCES: &[&str] = &[
    "dialogue/sandbox/intro.yarn",
    "dialogue/sandbox/kernel.yarn",
    "dialogue/sandbox/factions.yarn",
    "dialogue/sandbox/cove.yarn",
    "dialogue/sandbox/dojo.yarn",
    "dialogue/sandbox/symmetry.yarn",
    "dialogue/sandbox/hall.yarn",
];

#[cfg(feature = "ui")]
pub fn yarn_spinner_plugin() -> YarnSpinnerPlugin {
    // Android cannot enumerate asset folders inside the APK, so use
    // explicit Yarn sources instead of `YarnSpinnerPlugin::new()`
    // (which scans the dialogue folder on desktop builds).
    YarnSpinnerPlugin::with_yarn_sources(
        YARN_SOURCES.iter().map(|p| YarnFileSource::file(*p)),
    )
}

#[cfg(test)]
mod tests;
