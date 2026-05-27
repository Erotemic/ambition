//! Yarn↔DialogState bridge.
//!
//! Owns the integration between `bevy_yarnspinner` and the sandbox's
//! poll-based [`DialogState`] view-model. The migration plan
//! introduces this in six phases (see TODO; this file lands phase 1
//! foundation — runner lifecycle + observer skeletons).
//!
//! ## Lifecycle
//!
//! - [`YarnBridgePlugin`] mounts as part of the sandbox app under
//!   the `ui` feature (same gate as `bevy_yarnspinner`).
//! - At startup, `bevy_yarnspinner::YarnSpinnerPlugin` compiles all
//!   `.yarn` files into a `YarnProject` resource.
//! - Once `YarnProject` lands, [`spawn_dialogue_runner`] spawns a
//!   single persistent `DialogueRunner` entity and stashes its id
//!   in [`DialogueRunnerEntity`]. Persistent so `$variables` and
//!   visited-node bookkeeping survive across NPC visits — we want
//!   first-vs-repeat checks to pull from save (`visit_count(npc_id)`
//!   function arrives in phase 3), not throwaway runner state.
//! - Three observers translate runner events into sandbox state:
//!   - [`on_present_line`]: a line is ready — translate
//!     `LocalizedLine.{character_name, text, attributes}` into the
//!     dialog UI's read-model.
//!   - [`on_present_options`]: the runner is awaiting a player
//!     choice — record the option labels + their `OptionId`s.
//!   - [`on_dialogue_completed`]: the runner finished a node chain
//!     with no `<<jump>>` follow-up — close the dialog.
//!
//! ## Phase 1 status (this commit)
//!
//! - Bridge module wired into the sandbox app.
//! - `DialogueRunner` spawn + entity-id resource wired.
//! - Observers exist but do not yet write into [`DialogState`].
//!   They emit `info!` traces so the in-engine plumbing can be
//!   confirmed when a `.yarn` node is started.
//!
//! Phase 5 (per the migration plan) flips the DialogState authority:
//! `DialogState::start(id)` will call `runner.start_node(id)` and
//! the observers below will populate the view-model the existing
//! `sync_dialog_ui` already reads.
//!
//! ## Why observers, not message readers
//!
//! `bevy_yarnspinner` 0.8 fires lifecycle events via
//! `commands.trigger(EntityEvent)` — Bevy 0.18 observer pattern.
//! Consumers register with `app.add_observer(...)` and receive a
//! `Trigger<E>` in the system signature. The rest of the sandbox
//! uses messages today; this is the first piece of code using
//! observers.

use bevy::prelude::*;
use bevy_yarnspinner::events::*;
use bevy_yarnspinner::prelude::*;

/// Bevy resource: entity id of the singleton `DialogueRunner`.
/// Inserted by [`spawn_dialogue_runner`] once `YarnProject` is
/// available. `DialogState` methods (added in phase 5) read this
/// to call `start_node` / `select_option` / `continue_in_next_update`.
#[derive(Resource, Debug, Clone, Copy)]
#[allow(
    dead_code,
    reason = "consumer arrives in phase 5 when DialogState routes through Yarn"
)]
pub struct DialogueRunnerEntity(pub Entity);

/// Plugin that wires the bridge:
/// 1. Spawns the persistent runner once `YarnProject` resolves.
/// 2. Registers the three observers translating runner events into
///    sandbox state.
pub struct YarnBridgePlugin;

impl Plugin for YarnBridgePlugin {
    fn build(&self, app: &mut App) {
        // Spawn the runner once `YarnProject` becomes available. The
        // `YarnSpinnerPlugin` in `crate::dialog::yarn_spinner_plugin`
        // inserts that resource asynchronously (after all `.yarn`
        // files finish loading + compiling), so we hang the spawn
        // on a `resource_added` run-condition.
        app.add_systems(
            Update,
            spawn_dialogue_runner.run_if(resource_added::<YarnProject>),
        );
        // Observers — translate Yarn events into DialogState writes.
        // Phase 1 leaves these as logging-only placeholders.
        app.add_observer(on_present_line);
        app.add_observer(on_present_options);
        app.add_observer(on_dialogue_completed);
    }
}

/// Spawn the singleton `DialogueRunner`. Runs once when
/// `YarnProject` becomes a resource.
fn spawn_dialogue_runner(mut commands: Commands, project: Res<YarnProject>) {
    let runner = project.create_dialogue_runner(&mut commands);
    let entity = commands.spawn(runner).id();
    commands.insert_resource(DialogueRunnerEntity(entity));
    info!(
        target: "ambition_sandbox::dialog::yarn",
        "spawned DialogueRunner entity {entity:?}",
    );
}

/// `PresentLine` observer — Yarn emits this every time the runner is
/// ready to show a line to the player.
///
/// Phase 1: log. Phase 5: extract `character_name + text + markup
/// attributes` and write into `DialogState`.
fn on_present_line(event: On<PresentLine>) {
    info!(
        target: "ambition_sandbox::dialog::yarn",
        "PresentLine: speaker={:?} text={:?}",
        event.line.character_name(),
        event.line.text_without_character_name(),
    );
}

/// `PresentOptions` observer — Yarn is waiting for the player to
/// pick one of N options.
///
/// Phase 1: log labels + ids. Phase 5: record into `DialogState` so
/// `confirm_or_advance` can call `runner.select_option(id)`.
fn on_present_options(event: On<PresentOptions>) {
    info!(
        target: "ambition_sandbox::dialog::yarn",
        "PresentOptions: {} options",
        event.options.len(),
    );
    for option in &event.options {
        info!(
            target: "ambition_sandbox::dialog::yarn",
            "  option id={:?} label={:?}",
            option.id,
            option.line.text_without_character_name(),
        );
    }
}

/// `DialogueCompleted` observer — the runner reached the end of a
/// node chain with no more `<<jump>>` follow-ups.
///
/// Phase 1: log. Phase 5: call `DialogState::close()`.
fn on_dialogue_completed(_event: On<DialogueCompleted>) {
    info!(target: "ambition_sandbox::dialog::yarn", "DialogueCompleted");
}
