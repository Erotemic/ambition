//! Sim-side dialogue glue.
//!
//! The reusable dialogue runtime — the [`ambition_dialog::DialogState`] view
//! model, typewriter reveal/input systems, and the `bevy_yarnspinner` bridge —
//! lives in the `ambition_dialog` crate. This module keeps only what is genuinely
//! Ambition-side:
//!
//! - [`sync_dialogue_game_mode`] — the one host↔runtime coupling: the dialogue
//!   runtime owns no session `GameMode`, it just flips
//!   [`ambition_dialog::DialogState::active`]. This system maps "dialogue ended"
//!   back onto `GameMode::Playing`.

use bevy::prelude::*;

/// Host-side dialogue bindings plugin: brings up the reusable binding
/// resources ([`ambition_dialog::YarnBindingsPlugin`]) and the dialogue
/// input/reveal presentation pair.
///
/// it registers no GAME vocabulary. A game pushes its own commands and its
/// own state-mirror refresh through `ambition_dialog::YarnContentBindings` and
/// `YarnStateMirrorRefreshed`; this plugin only guarantees both seams exist.
///
/// it does register exactly one ENGINE verb, [`authored_conditions`]'s
/// `condition(…)`, which names no game content and no particular question — it
/// forwards whatever authored dialogue asks to whichever domain published the
/// answer. See that module for why one generic verb is not the same kind of
/// thing as a vocabulary table.
pub struct YarnBindingsPlugin;

impl Plugin for YarnBindingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ambition_dialog::YarnBindingsPlugin);
        // Ensuring it is the established pattern in this repo — `basic_presentation`, `pause_menu`
        // and `deterministic_activity` each do the same — and it is order-independent, which a run
        // condition would not be: "no menu frame" would then silently mean "dialogue accepts no
        // input".
        app.init_resource::<ambition_input::MenuControlFrame>();
        // CHAINED, and that is a fix rather than a transcription: both take
        // `ResMut<DialogState>`, and they sat in two different `Update` blocks with nothing
        // ordering them.
        //
        // the `.after(CoreSimulation)` pin is kept because it is load-bearing
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
        // no GAME installer is pushed from here any more. A game's
        // vocabulary arrives through `YarnContentBindings` from the crate that
        // owns the content — see `ambition_content::yarn_vocabulary`.
        //
        // what IS pushed from here is the one engine verb that lets
        // authored dialogue ask the condition catalog anything any installed
        // domain published. It travels the same installer seam a game's
        // vocabulary does, because the seam is how anything reaches the runner —
        // but it names no question, so a domain publishing a new one never
        // touches this line. See [`authored_conditions`].
        //
        // and its COMMAND twin, pushed the same way. Between them,
        // authored dialogue can ask any published question and ask for any
        // published verb — two lines here, forever, however many either domain
        // grows.
        app.init_resource::<ambition_dialog::YarnContentBindings>();
        let mut bindings = app
            .world_mut()
            .resource_mut::<ambition_dialog::YarnContentBindings>();
        bindings
            .installers
            .push(authored_conditions::install_condition_binding);
        bindings
            .installers
            .push(authored_commands::install_command_binding);
    }
}

pub mod authored_commands;
pub mod authored_conditions;
#[cfg(test)]
mod yarn_harness;

/// Host-side dialogue bridge plugin: the reusable
/// [`ambition_dialog::YarnBridgePlugin`] plus the [`sync_dialogue_game_mode`]
/// coupling that maps the runtime's `active` flag onto Ambition's `GameMode`.
pub struct YarnBridgePlugin;

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
        ambition_platformer2d_shared_tangle::world_log::note_game_mode_request(
            ambition_platformer2d_shared_tangle::schedule::GameMode::Playing,
            "dialogue_closed",
        );
        next_mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::Playing);
    }
}
