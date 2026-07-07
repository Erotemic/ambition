//! Sim-side dialogue glue.
//!
//! The reusable dialogue runtime — the [`DialogState`] view model, the
//! typewriter reveal + input systems, and the `bevy_yarnspinner` bridge — was
//! carved into the `ambition_dialog` crate (E1c). This module keeps only what
//! is genuinely game-side:
//!
//! - [`yarn_bindings`] — Ambition's Yarn *commands* (`<<give_item>>`,
//!   `<<challenge>>`, shop verbs) and *functions* (`<<if boss_cleared("x")>>`),
//!   plus the per-frame [`yarn_bindings::refresh_yarn_state_mirror`] that fills
//!   the shared mirror from `SandboxSave`. These reference actor/save state, so
//!   they can't live in the reusable crate; they register onto the runtime
//!   through the [`ambition_dialog::YarnContentBindings`] installer seam.
//! - [`sync_dialogue_game_mode`] — the one host↔runtime coupling: the dialogue
//!   runtime owns no session `GameMode`, it just flips [`DialogState::active`].
//!   This system maps "dialogue ended" back onto `GameMode::Playing`.
//!
//! The runtime types are re-exported here so existing
//! `ambition_actors::dialog::*` paths (render, content, app, host) keep
//! resolving without a churn of import edits.

pub use ambition_dialog::{
    dialog_input, dialog_pointer_input, dialog_reveal_tick, DialogChoice, DialogChoiceSlot,
    DialogState,
};

/// Ambition's game-specific Yarn vocabulary + the mirror refresh. Also
/// re-exports the generic binding types (`YarnStateMirror`, `YarnContentBindings`,
/// …) from `ambition_dialog` so content plugins keep the same import path.
#[cfg(feature = "ui")]
pub mod yarn_bindings;

#[cfg(feature = "ui")]
pub use ambition_dialog::DialogueRunnerEntity;

#[cfg(feature = "ui")]
use bevy::prelude::*;

/// Host-side dialogue bindings plugin: brings up the reusable binding
/// resources ([`ambition_dialog::YarnBindingsPlugin`]), schedules Ambition's
/// per-frame state-mirror refresh (after the generic cue reset — content
/// extras still run `.after(refresh_yarn_state_mirror)`), and registers
/// Ambition's game vocabulary through the installer seam.
#[cfg(feature = "ui")]
pub struct YarnBindingsPlugin;

#[cfg(feature = "ui")]
impl Plugin for YarnBindingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ambition_dialog::YarnBindingsPlugin);
        app.add_systems(
            Update,
            yarn_bindings::refresh_yarn_state_mirror
                .after(ambition_dialog::clear_yarn_presentation_cue),
        );
        app.world_mut()
            .resource_mut::<ambition_dialog::YarnContentBindings>()
            .installers
            .push(yarn_bindings::install_game_bindings);
    }
}

/// Host-side dialogue bridge plugin: the reusable
/// [`ambition_dialog::YarnBridgePlugin`] plus the [`sync_dialogue_game_mode`]
/// coupling that maps the runtime's `active` flag onto Ambition's `GameMode`.
#[cfg(feature = "ui")]
pub struct YarnBridgePlugin;

#[cfg(feature = "ui")]
impl Plugin for YarnBridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ambition_dialog::YarnBridgePlugin);
        app.add_systems(Update, sync_dialogue_game_mode);
    }
}

/// Map the reusable runtime's `DialogState.active` onto Ambition's session
/// `GameMode`. Entering `Dialogue` stays the interaction system's job (it sets
/// the mode when it starts a conversation); this closes the loop by returning
/// to `Playing` the moment the conversation ends — reproducing the transitions
/// the old bridge made inline before the GameMode coupling was lifted out.
#[cfg(feature = "ui")]
fn sync_dialogue_game_mode(
    dialogue: Res<DialogState>,
    mode: Res<State<crate::game_mode::GameMode>>,
    mut next_mode: ResMut<NextState<crate::game_mode::GameMode>>,
) {
    if matches!(mode.get(), crate::game_mode::GameMode::Dialogue) && !dialogue.active() {
        next_mode.set(crate::game_mode::GameMode::Playing);
    }
}
