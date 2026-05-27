//! Yarn↔DialogState bridge.
//!
//! Owns the integration between `bevy_yarnspinner` and the sandbox's
//! poll-based [`DialogState`] view-model. Phase 5 of the migration:
//! the runner is now the authority. The custom UI
//! (`sync_dialog_ui`) keeps reading `DialogState` exactly as before
//! — this module just makes the runner the source of writes.
//!
//! ## Lifecycle
//!
//! - At startup, `bevy_yarnspinner::YarnSpinnerPlugin` compiles all
//!   `.yarn` files into a `YarnProject` resource.
//! - Once `YarnProject` lands, [`spawn_dialogue_runner`] spawns the
//!   singleton `DialogueRunner` entity, registers commands +
//!   functions (`super::yarn_bindings`), and stashes the entity id
//!   in [`DialogueRunnerEntity`]. Persistent runner so visited-node
//!   bookkeeping survives across NPC visits; the save-driven
//!   `visit_count(id)` function is the canonical "have I talked to
//!   X" probe.
//!
//! ## Two-way flow
//!
//! - **Caller → runner**: `DialogState::start/close/confirm_or_advance`
//!   write `pending_*` fields. [`dispatch_pending_dialog_requests`]
//!   drains them once per frame and calls
//!   `runner.start_node` / `stop` / `select_option` /
//!   `continue_in_next_update` against the live runner entity.
//!   Visit count increments here too (one per `start` call).
//! - **Runner → UI**: three observers translate the runner's
//!   lifecycle events into `DialogState` writes:
//!   - [`on_present_line`] — `current_speaker`, `current_line`, and
//!     the `[shout]/[whisper]` markup cue.
//!   - [`on_present_options`] — `current_options` + parallel
//!     `yarn_option_ids`.
//!   - [`on_dialogue_completed`] — clears `active` + flips
//!     `GameMode` back to `Playing`.

use bevy::prelude::*;
use bevy_yarnspinner::events::*;
use bevy_yarnspinner::prelude::*;

use super::content::DialogChoice;
use super::runtime::DialogState;
use super::yarn_bindings::{
    register_commands, register_functions, YarnPresentationCue, YarnStateMirror,
};
use crate::persistence::save::SandboxSave;

/// Bevy resource: entity id of the singleton `DialogueRunner`.
#[derive(Resource, Debug, Clone, Copy)]
pub struct DialogueRunnerEntity(pub Entity);

pub struct YarnBridgePlugin;

impl Plugin for YarnBridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            spawn_dialogue_runner.run_if(resource_added::<YarnProject>),
        );
        app.add_systems(Update, dispatch_pending_dialog_requests);
        app.add_observer(on_present_line);
        app.add_observer(on_present_options);
        app.add_observer(on_dialogue_completed);
    }
}

/// Spawn the singleton `DialogueRunner` once `YarnProject` is
/// available. Registers commands + functions before spawning so
/// authored content can use the full vocabulary on the first node
/// entered.
fn spawn_dialogue_runner(
    mut commands: Commands,
    project: Res<YarnProject>,
    mirror: Res<YarnStateMirror>,
) {
    let mut runner = project.create_dialogue_runner(&mut commands);
    register_commands(&mut commands, &mut runner);
    register_functions(&mut runner, &mirror);
    let entity = commands.spawn(runner).id();
    commands.insert_resource(DialogueRunnerEntity(entity));
    info!(
        target: "ambition_sandbox::dialog::yarn",
        "spawned DialogueRunner entity {entity:?}",
    );
}

/// Drain `DialogState.pending_*` fields each frame, translate them
/// into runner calls, and write visit-count side effects to save.
///
/// Order matters: `pending_start` is processed before
/// `pending_select` / `pending_advance` so a "start + immediate
/// advance" combo in the same frame works. `pending_close` always
/// runs last so the runner can stop mid-conversation.
fn dispatch_pending_dialog_requests(
    mut state: ResMut<DialogState>,
    runner_e: Option<Res<DialogueRunnerEntity>>,
    mut runner_q: Query<&mut DialogueRunner>,
    save: Option<ResMut<SandboxSave>>,
) {
    let Some(runner_e) = runner_e else {
        return;
    };
    let Ok(mut runner) = runner_q.get_mut(runner_e.0) else {
        return;
    };

    // start_node
    if let Some((dialogue_id, _npc_name)) = state.pending_start.take() {
        if let Some(mut save) = save {
            save.data_mut().increment_dialog_visit(&dialogue_id);
        }
        if let Err(e) = runner.try_start_node(&dialogue_id) {
            warn!(
                target: "ambition_sandbox::dialog::yarn",
                "try_start_node({dialogue_id}) failed: {e}",
            );
            // Bail out of the active state so the UI doesn't hang
            // on a node that the runner couldn't enter.
            state.active = false;
        }
    }

    // select_option (use snapshot of yarn_option_ids before taking)
    if let Some(idx) = state.pending_select.take() {
        let option_id = state.yarn_option_ids.get(idx).copied();
        if let Some(option_id) = option_id {
            if let Err(e) = runner.select_option(option_id) {
                warn!(
                    target: "ambition_sandbox::dialog::yarn",
                    "select_option({option_id:?}) failed: {e}",
                );
            }
            // Clear the option set — the next `PresentLine` /
            // `PresentOptions` repopulates it.
            state.current_options.clear();
            state.yarn_option_ids.clear();
            state.selected_option = 0;
        }
    }

    // continue (no-option line advance)
    if std::mem::take(&mut state.pending_advance) && runner.is_running() {
        runner.continue_in_next_update();
        // Don't clear current_line here — the runner emits the
        // next PresentLine which overwrites it.
    }

    // stop
    if std::mem::take(&mut state.pending_close) && runner.is_running() {
        runner.stop();
    }
}

fn on_present_line(
    event: On<PresentLine>,
    mut state: ResMut<DialogState>,
    mut cue: ResMut<YarnPresentationCue>,
) {
    state.current_speaker = event.line.character_name().unwrap_or("").to_string();
    state.current_line = event.line.text_without_character_name();
    // PresentLine that arrives without a following PresentOptions
    // means a non-branching line — clear stale options so the UI
    // doesn't show last-line's choices.
    state.current_options.clear();
    state.yarn_option_ids.clear();
    state.selected_option = 0;
    // Markup cue capture for [shout] / [whisper] hooks.
    for attr in &event.line.attributes {
        match attr.name.as_str() {
            "shout" => cue.shout = true,
            "whisper" => cue.whisper = true,
            _ => {}
        }
    }
}

fn on_present_options(event: On<PresentOptions>, mut state: ResMut<DialogState>) {
    state.current_options.clear();
    state.yarn_option_ids.clear();
    for option in &event.options {
        state.current_options.push(DialogChoice {
            label: option.line.text_without_character_name(),
            // RON-era `next_node` / `close_after` are no longer
            // consulted by the runtime — Yarn's `select_option(id)`
            // dispatches via the parallel `yarn_option_ids` vec.
            next_node: None,
            note: None,
            close_after: false,
        });
        state.yarn_option_ids.push(option.id);
    }
    state.selected_option = 0;
}

fn on_dialogue_completed(
    _event: On<DialogueCompleted>,
    mut state: ResMut<DialogState>,
    mut next_mode: Option<ResMut<NextState<crate::game_mode::GameMode>>>,
) {
    state.active = false;
    state.current_speaker.clear();
    state.current_line.clear();
    state.current_options.clear();
    state.yarn_option_ids.clear();
    state.selected_option = 0;
    if let Some(next_mode) = next_mode.as_deref_mut() {
        next_mode.set(crate::game_mode::GameMode::Playing);
    }
}
