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
//!   The bridge no longer auto-continues after `PresentLine`; the
//!   player now advances once the reveal finishes.
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
        // `resource_added` only fires the single frame a resource
        // is inserted. If our spawn system runs BEFORE
        // `compile_loaded_yarn_files` (Bevy's per-frame system
        // ordering inside `Update` is non-deterministic without
        // explicit `.after(...)` constraints), we'd miss the
        // signal forever and the runner would never spawn. Use
        // `resource_exists` + a one-shot guard inside the system
        // so we spawn the first frame YarnProject is alive,
        // regardless of relative ordering.
        app.add_systems(
            Update,
            spawn_dialogue_runner.run_if(resource_exists::<YarnProject>),
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
///
/// One-shot guarded by `DialogueRunnerEntity` already existing —
/// the run condition (`resource_exists::<YarnProject>`) fires every
/// frame, but the guard ensures we only spawn once.
fn spawn_dialogue_runner(
    mut commands: Commands,
    project: Res<YarnProject>,
    mirror: Res<YarnStateMirror>,
    existing: Option<Res<DialogueRunnerEntity>>,
) {
    if existing.is_some() {
        return;
    }
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
    mut next_mode: Option<ResMut<NextState<crate::game_mode::GameMode>>>,
) {
    // Early-return + visible diagnostic if the runner hasn't
    // spawned yet. Without this, dialog.start() requests pile up
    // on `pending_start` forever and the UI shows the empty
    // "Continue" fallback because no PresentLine ever fires.
    let Some(runner_e) = runner_e else {
        if state.pending_start.is_some() || state.pending_close {
            warn!(
                target: "ambition_sandbox::dialog::yarn",
                "dispatch_pending_dialog_requests: DialogueRunner not spawned yet; \
                 pending request will be retried next frame",
            );
        }
        return;
    };
    let Ok(mut runner) = runner_q.get_mut(runner_e.0) else {
        warn!(
            target: "ambition_sandbox::dialog::yarn",
            "dispatch_pending_dialog_requests: DialogueRunnerEntity points at {:?} \
             but no DialogueRunner component there",
            runner_e.0,
        );
        return;
    };

    // start_node
    if let Some((dialogue_id, _npc_name)) = state.pending_start.take() {
        if let Some(mut save) = save {
            save.data_mut().increment_dialog_visit(&dialogue_id);
        }
        if !runner.node_exists(&dialogue_id) {
            warn!(
                target: "ambition_sandbox::dialog::yarn",
                "start({dialogue_id:?}): Yarn node not found. Add it to a \
                 file in assets/dialogue/sandbox/*.yarn (and to \
                 KNOWN_DIALOGUE_IDS in dialog/content.rs)",
            );
            // Flip everything back so the game doesn't freeze in
            // Dialogue mode. The caller (`interact_*`) set
            // GameMode=Dialogue synchronously; we have to unset it.
            state.active = false;
            if let Some(next_mode) = next_mode.as_deref_mut() {
                next_mode.set(crate::game_mode::GameMode::Playing);
            }
            return;
        }
        if let Err(e) = runner.try_start_node(&dialogue_id) {
            warn!(
                target: "ambition_sandbox::dialog::yarn",
                "try_start_node({dialogue_id:?}) failed: {e}",
            );
            state.active = false;
            if let Some(next_mode) = next_mode.as_deref_mut() {
                next_mode.set(crate::game_mode::GameMode::Playing);
            }
            return;
        }
        // Reset the accumulator for the new conversation.
        state.current_line.clear();
        state.current_speaker.clear();
        state.line_reveal = crate::dialog::runtime::LineRevealState::default();
        info!(
            target: "ambition_sandbox::dialog::yarn",
            "start_node({dialogue_id}) — runner advancing next tick",
        );
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
            // Reset the body + option accumulator for the next beat.
            state.current_line.clear();
            state.current_speaker.clear();
            state.line_reveal = crate::dialog::runtime::LineRevealState::default();
            state.current_options.clear();
            state.yarn_option_ids.clear();
            state.selected_option = 0;
        }
    }

    // continue (manual advance on a no-option line). Note:
    // `runner_done_pending_close` takes precedence — if the runner
    // already finished, don't try to continue it.
    if std::mem::take(&mut state.pending_advance) {
        if !state.runner_done_pending_close && runner.is_running() {
            runner.continue_in_next_update();
        }
    }

    // stop / close
    if std::mem::take(&mut state.pending_close) {
        if runner.is_running() {
            runner.stop();
        }
        state.active = false;
        state.current_line.clear();
        state.current_speaker.clear();
        state.current_options.clear();
        state.yarn_option_ids.clear();
        state.selected_option = 0;
        state.runner_done_pending_close = false;
        if let Some(next_mode) = next_mode.as_deref_mut() {
            next_mode.set(crate::game_mode::GameMode::Playing);
        }
    }
}

fn on_present_line(
    event: On<PresentLine>,
    mut state: ResMut<DialogState>,
    mut cue: ResMut<YarnPresentationCue>,
) {
    // PresentLine is now the hand-off point for the typewriter
    // reveal. Store the raw line text here; the UI reads the
    // visible substring from DialogState each frame.
    let new_speaker = event.line.character_name().unwrap_or("").to_string();
    let new_text = event.line.text_without_character_name();
    state.current_speaker = new_speaker;
    state.start_revealing_line(new_text);
    // Drop stale options from the previous beat. The new beat's
    // options arrive via `PresentOptions`.
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
    // Stop auto-advancing — the runner is waiting for the player
    // to pick an option.
    state.pending_advance = false;
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
    state.pending_advance = false;
    if !state.current_line.is_empty() {
        // The runner finished but there's still accumulated text
        // the player hasn't seen acknowledged yet (a final aside,
        // last beat of a branch). Hold the dialog open until the
        // player confirms.
        state.runner_done_pending_close = true;
        return;
    }
    // Empty body + runner done → close immediately.
    state.active = false;
    state.current_speaker.clear();
    state.current_options.clear();
    state.yarn_option_ids.clear();
    state.selected_option = 0;
    if let Some(next_mode) = next_mode.as_deref_mut() {
        next_mode.set(crate::game_mode::GameMode::Playing);
    }
}
