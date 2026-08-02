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
//!   the shared mirror from `AmbitionGameSave`. These reference actor/save state, so
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
                .in_set(ambition_dialog::YarnStateMirrorRefreshed)
                .after(ambition_dialog::YarnPresentationCueCleared),
        );
        // ⛔ a system that has only ever run in ONE composition has UNTESTED
        // requirements, and moving it turns them into panics rather than skips.
        // `dialog_input` takes a NON-optional `Res<MenuControlFrame>`; the app
        // always had one, the headless composition never did, and three app
        // tests died on "Resource does not exist" the moment these moved here.
        // Ensuring it is the established pattern in this repo — `basic_presentation`,
        // `pause_menu` and `deterministic_activity` each do the same — and it is
        // order-independent, which a run condition would not be: "no menu frame"
        // would then silently mean "dialogue accepts no input".
        app.init_resource::<ambition_input::MenuControlFrame>();
        // ⭐ the dialogue PRESENTATION pair, moved out of `ambition_app` on
        // 2026-08-02. The app hand-registered these two while every other part
        // of the feature lived in this plugin, so a composition could install
        // dialogue and get no input handling and no typewriter reveal.
        //
        // ⚠ CHAINED, and that is a fix rather than a transcription: both take
        // `ResMut<DialogState>`, and they sat in two different `Update` blocks
        // with nothing ordering them. Whether the reveal ticked before or after
        // the advance was arbitrary.
        //
        // ⚠ the `.after(CoreSimulation)` pin is kept because it is load-bearing
        // under the `RenderFrame` host, where `sim_schedule()` IS `Update`. It is
        // VACUOUS under `Fixed60Hz` and `Ggrs` — that set lives only in the sim
        // schedule — but the ordering still holds there, because Bevy runs
        // `PreUpdate` → `RunFixedMainLoop` → `Update` and both hosts put the sim
        // ahead of `Update`. Correct in all three, load-bearing in one.
        app.add_systems(
            Update,
            (ambition_dialog::dialog_input, ambition_dialog::dialog_reveal_tick)
                .chain()
                .after(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::CoreSimulation,
                )
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
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
    mode: Res<State<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
    mut next_mode: ResMut<NextState<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
) {
    if matches!(
        mode.get(),
        ambition_platformer2d_shared_tangle::schedule::GameMode::Dialogue
    ) && !dialogue.active()
    {
        next_mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::Playing);
    }
}
