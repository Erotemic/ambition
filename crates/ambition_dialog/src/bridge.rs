//! Yarn↔DialogState bridge.
//!
//! Integrates `bevy_yarnspinner` with the sandbox's poll-based [`DialogState`]
//! view model. The persistent `DialogueRunner` is the dialogue authority; the UI
//! and input systems keep reading/writing `DialogState`.
//!
//! Flow:
//! - startup compiles `.yarn` into `YarnProject`;
//! - [`spawn_dialogue_runner`] creates the singleton runner once the project exists
//!   and registers commands/functions;
//! - pending `DialogState` requests are drained into runner calls;
//! - runner lifecycle observers write speaker, line, options, markup cues, and
//!   completion state back into `DialogState`.
//!
//! The bridge auto-continues only on explicit `lastline` markers, so authored
//! options can appear immediately while normal lines still wait for confirmation.

use bevy::prelude::*;
use bevy_yarnspinner::events::*;
use bevy_yarnspinner::prelude::*;

use crate::bindings::{YarnContentBindings, YarnPresentationCue, YarnStateMirror};
use crate::content::DialogChoice;
use crate::context::{DialogueContext, DialogueNodeIndex};
use crate::runtime::{DialogSpeechStyle, DialogState};
use ambition_persistence::save::SandboxSave;
use ambition_sfx::{SfxMessage, SfxWriter};

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
        // The compiled project's node names, published for the SIMULATION to
        // ask "did content author a self branch?" without a Yarn dependency.
        app.init_resource::<DialogueNodeIndex>();
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
/// available. Runs every registered [`YarnContentBindings`] installer
/// before spawning so authored content can use the full vocabulary on
/// the first node entered.
///
/// One-shot guarded by `DialogueRunnerEntity` already existing —
/// the run condition (`resource_exists::<YarnProject>`) fires every
/// frame, but the guard ensures we only spawn once.
fn spawn_dialogue_runner(
    mut commands: Commands,
    project: Res<YarnProject>,
    mirror: Res<YarnStateMirror>,
    content_bindings: Res<YarnContentBindings>,
    mut node_index: ResMut<DialogueNodeIndex>,
    existing: Option<Res<DialogueRunnerEntity>>,
) {
    if existing.is_some() {
        return;
    }
    let mut runner = project.create_dialogue_runner(&mut commands);
    // Publish the compiled node set. The interact dispatcher reads it to decide
    // whether a self-conversation has a branch to enter — before it opens a
    // dialogue box, not after.
    node_index.populate(runner.inner().node_names().map(str::to_owned));
    // All Yarn vocabulary — the host's generic game commands/functions
    // AND content-side named vocabulary — is registered through the
    // installer seam. The bridge names no concrete command, so the
    // runtime stays reusable across games.
    for install in &content_bindings.installers {
        install(&mut commands, &mut runner, &mirror);
    }
    let entity = commands.spawn(runner).id();
    commands.insert_resource(DialogueRunnerEntity(entity));
    info!(
        target: "ambition_dialog::bridge",
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
    // Early-return + visible diagnostic if the runner hasn't
    // spawned yet. Without this, dialog.start() requests pile up
    // on `pending_start` forever and the UI shows the empty
    // "Continue" fallback because no PresentLine ever fires.
    let Some(runner_e) = runner_e else {
        if state.pending_start.is_some() || state.pending_close {
            warn!(
                target: "ambition_dialog::bridge",
                "dispatch_pending_dialog_requests: DialogueRunner not spawned yet; \
                 pending request will be retried next frame",
            );
        }
        return;
    };
    let Ok(mut runner) = runner_q.get_mut(runner_e.0) else {
        warn!(
            target: "ambition_dialog::bridge",
            "dispatch_pending_dialog_requests: DialogueRunnerEntity points at {:?} \
             but no DialogueRunner component there",
            runner_e.0,
        );
        return;
    };

    // start_node
    if let Some(pending) = state.pending_start.take() {
        let dialogue_id = pending.dialogue_id;
        // WHO is talking to WHOM, published before the node begins so content's
        // very first `<<if $speaker_is_self>>` reads a live value.
        publish_dialogue_context(&mut runner, &pending.context);
        if let Some(mut save) = save {
            save.data_mut().increment_dialog_visit(&dialogue_id);
        }
        if !runner.node_exists(&dialogue_id) {
            warn!(
                target: "ambition_dialog::bridge",
                "start({dialogue_id:?}): Yarn node not found. Add it to a \
                 file in assets/dialogue/sandbox/*.yarn with a matching title header",
            );
            // Flip everything back so the game doesn't freeze in
            // Dialogue mode. The caller (`interact_*`) set its session
            // mode synchronously; clearing `active` lets the host map
            // back out of it.
            state.active = false;
            return;
        }
        if let Err(e) = runner.try_start_node(&dialogue_id) {
            warn!(
                target: "ambition_dialog::bridge",
                "try_start_node({dialogue_id:?}) failed: {e}",
            );
            state.active = false;
            return;
        }
        // Reset the accumulator for the new conversation.
        state.current_line.clear();
        state.current_speaker.clear();
        state.line_reveal = crate::runtime::LineRevealState::default();
        state.line_last_before_options = false;
        state.options_reveal = crate::runtime::OptionsRevealState::default();
        info!(
            target: "ambition_dialog::bridge",
            "start_node({dialogue_id}) — runner advancing next tick",
        );
    }

    // select_option (use snapshot of yarn_option_ids before taking)
    if let Some(idx) = state.pending_select.take() {
        let option_id = state.yarn_option_ids.get(idx).copied();
        if let Some(option_id) = option_id {
            if let Err(e) = runner.select_option(option_id) {
                warn!(
                    target: "ambition_dialog::bridge",
                    "select_option({option_id:?}) failed: {e}",
                );
            }
            // Reset the body + option accumulator for the next beat.
            state.current_line.clear();
            state.current_speaker.clear();
            state.line_reveal = crate::runtime::LineRevealState::default();
            state.line_last_before_options = false;
            state.options_reveal = crate::runtime::OptionsRevealState::default();
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
        state.line_last_before_options = false;
        state.options_reveal = crate::runtime::OptionsRevealState::default();
        state.runner_done_pending_close = false;
    }
}

/// Write the conversation's identity context into the runner's variable storage.
///
/// This is the ONLY place the engine writes a Yarn `$variable`; everything else
/// content reads is a library FUNCTION over the state mirror. Identity is
/// different: it is fixed for the whole conversation and content branches on it
/// at line zero, so a variable — set once, before the node starts — is the right
/// shape, and it costs no per-line mirror read.
fn publish_dialogue_context(runner: &mut DialogueRunner, context: &DialogueContext) {
    let storage = runner.variable_storage_mut();
    let vars: [(&str, YarnValue); 3] = [
        ("$speaker_id", YarnValue::String(context.speaker_id.clone())),
        (
            "$listener_id",
            YarnValue::String(context.listener_id.clone()),
        ),
        (
            "$speaker_is_self",
            YarnValue::Boolean(context.speaker_is_self),
        ),
    ];
    for (name, value) in vars {
        if let Err(e) = storage.set(name.to_string(), value) {
            warn!(
                target: "ambition_dialog::bridge",
                "could not publish {name} into Yarn variable storage: {e}",
            );
        }
    }
}

fn on_present_line(
    event: On<PresentLine>,
    mut state: ResMut<DialogState>,
    mut cue: ResMut<YarnPresentationCue>,
    mut sfx: SfxWriter,
) {
    // PresentLine is now the hand-off point for the typewriter
    // reveal. Store the raw line text here; the UI reads the
    // visible substring from DialogState each frame.
    let new_speaker = event.line.character_name().unwrap_or("").to_string();
    let new_text = event.line.text_without_character_name();
    state.current_speaker = new_speaker;
    state.start_revealing_line(new_text);
    state.set_line_last_before_options(event.line.is_last_line_before_options());
    // Drop stale options from the previous beat. The new beat's
    // options arrive via `PresentOptions`.
    state.current_options.clear();
    state.options_reveal = crate::runtime::OptionsRevealState::default();
    state.yarn_option_ids.clear();
    state.selected_option = 0;
    // Markup cue capture for [shout] / [whisper] hooks. Shout wins for
    // typewriter tone if both attributes are present; the one-shot markup
    // accents still play for every authored attribute.
    let mut saw_shout = false;
    let mut saw_whisper = false;
    for attr in &event.line.attributes {
        match attr.name.as_str() {
            "shout" => {
                cue.shout = true;
                saw_shout = true;
                sfx.write(SfxMessage::Play {
                    id: ambition_sfx::ids::DIALOGUE_MARKUP_SHOUT,
                    pos: ambition_engine_core::Vec2::ZERO,
                });
            }
            "whisper" => {
                cue.whisper = true;
                saw_whisper = true;
                sfx.write(SfxMessage::Play {
                    id: ambition_sfx::ids::DIALOGUE_MARKUP_WHISPER,
                    pos: ambition_engine_core::Vec2::ZERO,
                });
            }
            _ => {}
        }
    }
    state.set_speech_style(DialogSpeechStyle::from_markup(saw_shout, saw_whisper));
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
    state.reveal_full_options();
    state.selected_option = 0;
}

fn on_dialogue_completed(_event: On<DialogueCompleted>, mut state: ResMut<DialogState>) {
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
    state.line_last_before_options = false;
    state.options_reveal = crate::runtime::OptionsRevealState::default();
}
