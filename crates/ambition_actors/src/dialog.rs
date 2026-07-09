//! Sim-side dialogue glue.
//!
//! The reusable dialogue runtime — the [`ambition_dialog::DialogState`] view
//! model, typewriter reveal/input systems, and the `bevy_yarnspinner` bridge —
//! lives in the `ambition_dialog` crate. This module keeps only what is genuinely
//! Ambition-side:
//!
//! - [`yarn_bindings`] — Ambition's Yarn *commands* (`<<give_item>>`,
//!   `<<challenge>>`, shop verbs) and *functions* (`<<if boss_cleared("x")>>`),
//!   plus the per-frame [`yarn_bindings::refresh_yarn_state_mirror`] that fills
//!   the shared mirror from `SandboxSave`. These reference actor/save state, so
//!   they register onto the reusable runtime through the
//!   [`ambition_dialog::YarnContentBindings`] installer seam.
//! - [`sync_dialogue_game_mode`] — the one host↔runtime coupling: the dialogue
//!   runtime owns no session `GameMode`, it just flips
//!   [`ambition_dialog::DialogState::active`]. This system maps "dialogue ended"
//!   back onto `GameMode::Playing`.

/// Ambition's game-specific Yarn vocabulary + the mirror refresh.
#[cfg(feature = "ui")]
pub mod yarn_bindings;

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
/// to `Playing` the moment the conversation ends.
#[cfg(feature = "ui")]
fn sync_dialogue_game_mode(
    dialogue: Res<ambition_dialog::DialogState>,
    mode: Res<State<ambition_platformer_primitives::schedule::GameMode>>,
    mut next_mode: ResMut<NextState<ambition_platformer_primitives::schedule::GameMode>>,
) {
    if matches!(
        mode.get(),
        ambition_platformer_primitives::schedule::GameMode::Dialogue
    ) && !dialogue.active()
    {
        next_mode.set(ambition_platformer_primitives::schedule::GameMode::Playing);
    }
}
