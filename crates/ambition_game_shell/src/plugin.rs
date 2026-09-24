//! Bevy plugins that drive shell routing, sequences, and launcher commands.

use bevy::prelude::{
    App, Commands, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, Query, Res, ResMut,
    Time, Update, With,
};

use ambition_input::participant::{context_priority, ContextClaim};
use ambition_input::{
    InputParticipant, InputSet, ParticipantContexts, LAUNCHER_CONTEXT, STARTUP_ACKNOWLEDGE_CONTEXT,
};
use ambition_load::{AmbitionLoadSet, LoadCoordinator};
use tracing::{debug, error, warn};

use crate::audio_controls::ShellAudioControl;

use crate::{
    ActiveGameplaySession, ActiveShellSequence, AmbitionGameShellSet, PreparedSessionRegistry,
    ShellCommand, ShellCommandRejection, ShellEvent, ShellExperienceRegistry,
    ShellHostConfiguration, ShellLaunchCatalog, ShellLauncherCommand, ShellLauncherPresentation,
    ShellActivationGates, ShellLauncherState, ShellRouteCatalog, ShellRouteHolds, ShellRouter,
    ShellScopedEntity,
    ShellSegmentScopedEntity, ShellSequenceCatalog, ShellSequenceCommand, ShellSequenceRuntime,
    ShellSequenceSet, BASIC_LAUNCHER_EXPERIENCE,
};

#[derive(Default)]
pub struct AmbitionGameShellPlugin;

#[derive(Default)]
pub struct ShellSequencePlugin;

#[derive(Default)]
pub struct ShellLauncherPlugin;

impl Plugin for AmbitionGameShellPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ambition_load::LoadCoordinator>()
            .init_resource::<ShellRouteCatalog>()
            .init_resource::<ShellHostConfiguration>()
            .init_resource::<ShellRouter>()
            .init_resource::<PreparedSessionRegistry>()
            .init_resource::<ShellRouteHolds>()
            .init_resource::<ShellActivationGates>()
            .init_resource::<ShellFailureLog>()
            .add_message::<ShellCommand>()
            .add_message::<ShellEvent>()
            // Registered here, not by the pause menu that writes it, so an
            // experience can install its reader in a composition with no menu.
            .add_message::<crate::abandon::ShellAbandonRequested>()
            .add_message::<ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction>(
            )
            .configure_sets(
                Update,
                (
                    AmbitionGameShellSet::Commands,
                    AmbitionGameShellSet::Pending,
                    AmbitionGameShellSet::Cleanup,
                )
                    .chain()
                    .after(AmbitionLoadSet::Commands),
            )
            .add_systems(
                Update,
                (
                    initialize_shell,
                    quit_active_session_from_developer_action,
                    process_shell_commands,
                )
                    .chain()
                    .in_set(AmbitionGameShellSet::Commands),
            )
            .add_systems(
                Update,
                advance_pending_route.in_set(AmbitionGameShellSet::Pending),
            )
            .add_systems(
                Update,
                (
                    cleanup_scoped_entities,
                    // The resource form of the rule above: provider state lives
                    // only while the provider's routes are active. In `Cleanup`
                    // because `active` and `pending` are settled by then.
                    crate::scope::release_departed_experience_state,
                    log_shell_routing_failures,
                )
                    .chain()
                    .in_set(AmbitionGameShellSet::Cleanup),
            );
    }
}

impl Plugin for ShellSequencePlugin {
    fn build(&self, app: &mut App) {
        // Idempotent: a headless host needs `Time` so `drive_sequence` ticks.
        app.init_resource::<bevy::prelude::Time>()
            .init_resource::<ShellSequenceCatalog>()
            .init_resource::<ActiveShellSequence>()
            .add_message::<ShellSequenceCommand>()
            .configure_sets(
                Update,
                (
                    ShellSequenceSet::Sync,
                    ShellSequenceSet::Tick,
                    ShellSequenceSet::Programmatic,
                    ShellSequenceSet::Commands,
                    ShellSequenceSet::Cleanup,
                )
                    .chain()
                    .after(AmbitionGameShellSet::Pending)
                    .before(AmbitionGameShellSet::Cleanup),
            )
            // Apply confirm and card taps in the same frame as the input.
            .configure_sets(Update, ShellSequenceSet::Commands.after(InputSet::Consume))
            .add_systems(
                Update,
                start_or_stop_sequence.in_set(ShellSequenceSet::Sync),
            )
            // The sequence surface claims the startup-acknowledge context
            // while a card sequence is active. The claim is declared, not
            // inferred from `GameMode` or actor presence.
            .add_systems(
                Update,
                declare_startup_acknowledge_context
                    .in_set(InputSet::ResolveContext)
                    .after(ShellSequenceSet::Sync),
            )
            .add_systems(Update, drive_sequence.in_set(ShellSequenceSet::Tick))
            .add_systems(
                Update,
                process_sequence_commands.in_set(ShellSequenceSet::Commands),
            )
            .add_systems(
                Update,
                cleanup_segment_scoped_entities.in_set(ShellSequenceSet::Cleanup),
            );
    }
}

impl Plugin for ShellLauncherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShellLaunchCatalog>()
            .init_resource::<ShellExperienceRegistry>()
            .init_resource::<ShellLauncherPresentation>()
            .init_resource::<ShellLauncherState>()
            .add_message::<ShellLauncherCommand>()
            .add_systems(
                Update,
                (
                    crate::experience::sync_registry_into_launch_catalog,
                    sync_launcher_activation,
                    // Apply navigation in the same frame as the input.
                    process_launcher_commands.after(InputSet::Consume),
                )
                    .chain()
                    .after(AmbitionGameShellSet::Pending),
            )
            // The launcher claims its input context while its route is active.
            .add_systems(
                Update,
                declare_launcher_context
                    .in_set(InputSet::ResolveContext)
                    .after(sync_launcher_activation),
            );
    }
}

/// While a shell card sequence is active, the startup-acknowledge context
/// owns the participant's actions (one semantic "continue"; tap-anywhere).
fn declare_startup_acknowledge_context(
    sequence: Res<ActiveShellSequence>,
    mut participants: Query<&mut ParticipantContexts, With<InputParticipant>>,
) {
    let active = sequence.activation_id.is_some() && sequence.runtime.is_some();
    for mut contexts in &mut participants {
        if contexts.is_declared(STARTUP_ACKNOWLEDGE_CONTEXT) != active {
            contexts.sync(
                ContextClaim::capturing(
                    STARTUP_ACKNOWLEDGE_CONTEXT,
                    context_priority::STARTUP_ACKNOWLEDGE,
                ),
                active,
            );
        }
    }
}

/// While the launcher route is active, the launcher context captures the
/// participant's actions, so gameplay does not act under the title menu.
fn declare_launcher_context(
    state: Res<ShellLauncherState>,
    mut participants: Query<&mut ParticipantContexts, With<InputParticipant>>,
) {
    for mut contexts in &mut participants {
        if contexts.is_declared(LAUNCHER_CONTEXT) != state.active {
            contexts.sync(
                ContextClaim::capturing(LAUNCHER_CONTEXT, context_priority::LAUNCHER),
                state.active,
            );
        }
    }
}

fn initialize_shell(
    host: Res<ShellHostConfiguration>,
    router: Res<ShellRouter>,
    mut commands: MessageWriter<ShellCommand>,
) {
    if !router.is_initialized() && host.spec.is_some() {
        commands.write(ShellCommand::Initialize);
    }
}

fn quit_active_session_from_developer_action(
    mut actions: MessageReader<
        ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction,
    >,
    active: Option<Res<ActiveGameplaySession>>,
    mut shell: MessageWriter<ShellCommand>,
) {
    let requested = actions.read().any(|action| {
        *action == ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction::QuitToHome
    });
    if requested
        && active
            .as_deref()
            .and_then(|active| active.0.as_ref())
            .is_some()
    {
        shell.write(ShellCommand::QuitToHome);
    }
}

fn process_shell_commands(
    mut commands: MessageReader<ShellCommand>,
    catalog: Res<ShellRouteCatalog>,
    host: Res<ShellHostConfiguration>,
    mut loads: ResMut<LoadCoordinator>,
    mut prepared: ResMut<PreparedSessionRegistry>,
    mut router: ResMut<ShellRouter>,
    mut events: MessageWriter<ShellEvent>,
) {
    for command in commands.read() {
        for event in router.apply(command.clone(), &catalog, &host, &mut loads, &mut prepared) {
            events.write(event);
        }
    }
}

/// Ask this route's activation gates, then activate, in one exclusive
/// operation.
///
/// Exclusive for atomicity (`Q118`): if a gate were checked in an earlier
/// system, a rollback ownership change could land between the check and
/// `RouteActivated`. Each registered evaluator runs here, in the same world
/// access that emits `RouteActivated`.
///
/// A hold with no registered gate is an ordinary hold and still blocks (e.g.
/// the loading-screen hold in `ambition_load_presentation`, which releases
/// itself).
fn advance_pending_route(world: &mut bevy::prelude::World) {
    let pending_route = world
        .resource::<ShellRouter>()
        .pending
        .as_ref()
        .map(|pending| pending.route_id.clone());
    if let Some(route_id) = pending_route {
        // Ask only when the route would otherwise activate: `Admit` consumes
        // a hold. See `ShellRouter::ready_but_for_holds`.
        let ready = {
            let router = world.resource::<ShellRouter>();
            let loads = world.resource::<LoadCoordinator>();
            let prepared = world.resource::<PreparedSessionRegistry>();
            router.ready_but_for_holds(loads, prepared)
        };
        let gated: Vec<_> = if !ready {
            Vec::new()
        } else {
            let holds = world.resource::<ShellRouteHolds>();
            let gates = world.resource::<ShellActivationGates>();
            holds
                .held(&route_id)
                .into_iter()
                .filter_map(|hold| gates.evaluator(&hold).map(|system| (hold, system)))
                .collect()
        };
        for (hold, evaluator) in gated {
            // `run_system` runs inside this exclusive access, so the gate sees
            // the same world the activation uses.
            let verdict = world
                .run_system(evaluator)
                .unwrap_or(crate::router::ShellGateVerdict::Hold);
            match verdict {
                crate::router::ShellGateVerdict::Hold => return,
                crate::router::ShellGateVerdict::Admit => {
                    world
                        .resource_mut::<ShellRouteHolds>()
                        .release(&route_id, &hold);
                }
                crate::router::ShellGateVerdict::Refuse => {
                    // Cancel first, then release. Cancel directly on the
                    // router, not through `ShellCommand::CancelPending`: a
                    // `GoTo` transaction has no request id, so that command
                    // would not match it and the route would activate.
                    let cancelled = world.resource_scope(
                        |world, mut router: bevy::prelude::Mut<ShellRouter>| {
                            world.resource_scope(
                                |world, mut loads: bevy::prelude::Mut<LoadCoordinator>| {
                                    let mut prepared =
                                        world.resource_mut::<PreparedSessionRegistry>();
                                    router.cancel_pending(&mut loads, &mut prepared)
                                },
                            )
                        },
                    );
                    world
                        .resource_mut::<ShellRouteHolds>()
                        .release(&route_id, &hold);
                    for event in cancelled {
                        world.write_message(event);
                    }
                    return;
                }
            }
        }
    }

    let events = world.resource_scope(|world, mut router: bevy::prelude::Mut<ShellRouter>| {
        world.resource_scope(|world, mut loads: bevy::prelude::Mut<LoadCoordinator>| {
            world.resource_scope(
                |world, mut prepared: bevy::prelude::Mut<PreparedSessionRegistry>| {
                    let catalog = world.resource::<ShellRouteCatalog>();
                    let holds = world.resource::<ShellRouteHolds>();
                    router.advance_pending(catalog, &mut loads, &mut prepared, holds)
                },
            )
        })
    });
    for event in events {
        world.write_message(event);
    }
}

fn cleanup_scoped_entities(
    mut commands: Commands,
    mut events: MessageReader<ShellEvent>,
    entities: Query<(bevy::prelude::Entity, &ShellScopedEntity)>,
) {
    for event in events.read() {
        let ShellEvent::RouteDeactivated(active) = event else {
            continue;
        };
        for (entity, scope) in &entities {
            if scope.activation_id == active.activation_id {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// The reasons routing has refused this host, kept where a consumer can read
/// them.
///
/// A headless host (a scripted test, a CI gate, an external binary) has no load
/// presentation and may have no log subscriber. Without this record, a refused
/// route (e.g. a missing audio fragment in [`LoadFailure`] reasons from
/// [`ShellCommandRejection::LoadFailed`]) looks like a host that never starts.
#[derive(bevy::prelude::Resource, Default, Debug, Clone)]
pub struct ShellFailureLog {
    reasons: Vec<String>,
}

impl ShellFailureLog {
    /// Every refusal recorded, oldest first.
    pub fn reasons(&self) -> &[String] {
        &self.reasons
    }

    /// The most recent refusal, which is what a poll loop wants to print.
    pub fn latest(&self) -> Option<&str> {
        self.reasons.last().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.reasons.is_empty()
    }

    fn record(&mut self, reason: String) {
        self.reasons.push(reason);
    }
}

/// Record and log routing failures from `ShellEvent`, in one reader.
fn log_shell_routing_failures(
    mut events: MessageReader<ShellEvent>,
    mut failures: ResMut<ShellFailureLog>,
) {
    for event in events.read() {
        match event {
            ShellEvent::CommandRejected(rejection) => {
                log_shell_rejection(rejection);
                if let Some(reason) = describe_rejection(rejection) {
                    failures.record(reason);
                }
            }
            ShellEvent::ExperienceFailed {
                activation_id,
                message,
            } => {
                error!("shell experience {activation_id:?} failed: {message}");
                failures.record(format!("experience failed: {message}"));
            }
            // `TransactionEnded` is not recorded. Cancellation and supersession
            // are not faults, and a `Failed` end always comes with
            // `CommandRejected(LoadFailed { .. })`, which the arm above logs.
            _ => {}
        }
    }
}

/// A refusal as one line a consumer can act on, or `None` for the routine
/// non-faults.
///
/// Cancellation and supersession are ordinary navigation, so they are not
/// recorded.
fn describe_rejection(rejection: &ShellCommandRejection) -> Option<String> {
    match rejection {
        ShellCommandRejection::LoadFailed {
            readiness,
            failures,
        } => {
            if failures.is_empty() {
                return None;
            }
            Some(format!(
                "route load {readiness:?}: {}",
                failures
                    .iter()
                    .map(|failure| format!(
                        "{} ({})",
                        failure.player_message, failure.developer_detail
                    ))
                    .collect::<Vec<_>>()
                    .join("; ")
            ))
        }
        ShellCommandRejection::HostNotConfigured => {
            Some("the host names no initial route, so nothing was prepared".to_string())
        }
        ShellCommandRejection::UnknownRoute(route) => {
            Some(format!("unknown route `{}`", route.as_str()))
        }
        ShellCommandRejection::PreparedSessionUnavailable(_) => {
            Some("a route activated with no prepared session behind it".to_string())
        }
        ShellCommandRejection::LoadCommitRejected(rejection) => {
            Some(format!("load commit rejected: {rejection:?}"))
        }
        ShellCommandRejection::StaleActivation(_) => None,
    }
}

fn log_shell_rejection(rejection: &ShellCommandRejection) {
    match rejection {
        ShellCommandRejection::LoadFailed {
            readiness,
            failures,
        } => {
            if failures.is_empty() {
                // Cancellation or supersession: routine navigation, not a fault.
                debug!("shell route load ended {readiness:?} with no per-work failure");
            } else {
                for failure in failures {
                    if failure.retryable {
                        warn!(
                            "shell route load failed (retryable): {} ({})",
                            failure.player_message, failure.developer_detail
                        );
                    } else {
                        error!(
                            "shell route load failed: {} ({})",
                            failure.player_message, failure.developer_detail
                        );
                    }
                }
            }
        }
        // A command for an already-superseded activation is a benign race.
        ShellCommandRejection::StaleActivation(activation_id) => {
            debug!("shell command rejected — stale activation {activation_id:?}");
        }
        // Host misconfiguration or a load-commit contract violation.
        other => error!("shell command rejected: {other:?}"),
    }
}

fn start_or_stop_sequence(
    mut events: MessageReader<ShellEvent>,
    catalog: Res<ShellSequenceCatalog>,
    mut active: ResMut<ActiveShellSequence>,
) {
    for event in events.read() {
        match event {
            ShellEvent::RouteActivated(route) => {
                if let Some(spec) = catalog.get(&route.experience_id) {
                    active.activation_id = Some(route.activation_id);
                    active.runtime = Some(ShellSequenceRuntime::new(spec.clone()));
                }
            }
            ShellEvent::RouteDeactivated(route)
                if active.activation_id == Some(route.activation_id) =>
            {
                active.activation_id = None;
                active.runtime = None;
            }
            _ => {}
        }
    }
}

fn drive_sequence(
    time: Res<Time>,
    mut active: ResMut<ActiveShellSequence>,
    mut shell: MessageWriter<ShellCommand>,
) {
    let Some(activation_id) = active.activation_id else {
        return;
    };
    let Some(runtime) = active.runtime.as_mut() else {
        return;
    };
    if runtime.tick(std::time::Duration::from_secs_f32(time.delta_secs())) || runtime.finished {
        shell.write(ShellCommand::ExperienceCompleted { activation_id });
        active.activation_id = None;
        active.runtime = None;
    }
}

fn process_sequence_commands(
    mut commands: MessageReader<ShellSequenceCommand>,
    mut active: ResMut<ActiveShellSequence>,
    mut shell: MessageWriter<ShellCommand>,
) {
    for command in commands.read() {
        let Some(active_id) = active.activation_id else {
            continue;
        };
        let target_id = match command {
            ShellSequenceCommand::Skip { activation_id }
            | ShellSequenceCommand::Acknowledge { activation_id }
            | ShellSequenceCommand::ProgrammaticSegmentCompleted { activation_id, .. }
            | ShellSequenceCommand::ProgrammaticSegmentFailed { activation_id, .. } => {
                *activation_id
            }
        };
        if target_id != active_id {
            continue;
        }

        let mut failure = None;
        let completed = {
            let Some(runtime) = active.runtime.as_mut() else {
                continue;
            };
            let current_segment = runtime.current().map(|segment| segment.id.clone());
            let advanced = match command {
                ShellSequenceCommand::Skip { .. } => runtime.skip(),
                ShellSequenceCommand::Acknowledge { .. } => runtime.acknowledge(),
                ShellSequenceCommand::ProgrammaticSegmentCompleted { segment_id, .. }
                    if current_segment.as_ref() == Some(segment_id) =>
                {
                    runtime.complete_programmatic_segment()
                }
                ShellSequenceCommand::ProgrammaticSegmentFailed {
                    segment_id,
                    message,
                    ..
                } if current_segment.as_ref() == Some(segment_id) => {
                    failure = Some(message.clone());
                    false
                }
                ShellSequenceCommand::ProgrammaticSegmentCompleted { .. }
                | ShellSequenceCommand::ProgrammaticSegmentFailed { .. } => false,
            };
            advanced || runtime.finished
        };

        if let Some(message) = failure {
            shell.write(ShellCommand::ExperienceFailed {
                activation_id: active_id,
                message,
            });
            active.activation_id = None;
            active.runtime = None;
            continue;
        }
        if completed {
            shell.write(ShellCommand::ExperienceCompleted {
                activation_id: active_id,
            });
            active.activation_id = None;
            active.runtime = None;
        }
    }
}

fn cleanup_segment_scoped_entities(
    mut commands: Commands,
    active: Res<ActiveShellSequence>,
    entities: Query<(bevy::prelude::Entity, &ShellSegmentScopedEntity)>,
) {
    let current = active
        .runtime
        .as_ref()
        .and_then(|runtime| runtime.current())
        .and_then(|segment| {
            active
                .activation_id
                .map(|activation_id| (activation_id, &segment.id))
        });
    for (entity, scope) in &entities {
        let owned_by_current = current.is_some_and(|(activation_id, segment_id)| {
            scope.activation_id == activation_id && &scope.segment_id == segment_id
        });
        if !owned_by_current {
            commands.entity(entity).despawn();
        }
    }
}

fn sync_launcher_activation(
    router: Res<ShellRouter>,
    catalog: Res<ShellLaunchCatalog>,
    presentation: Res<ShellLauncherPresentation>,
    mut state: ResMut<ShellLauncherState>,
) {
    let active = router
        .active
        .as_ref()
        .is_some_and(|active| active.experience_id.as_str() == BASIC_LAUNCHER_EXPERIENCE);
    if state.active != active {
        state.active = active;
        state.selected = 0;
    }
    if state.active {
        // Selection space: the available experiences plus the built-in Exit
        // row (when the presentation shows one).
        let available = catalog
            .entries
            .iter()
            .filter(|entry| entry.available)
            .count();
        let selectable = available + usize::from(presentation.exit_label.is_some());
        state.selected = state.selected.min(selectable.saturating_sub(1));
    }
}

/// Apply a settings-tab row activation: move the cursor there and step the
/// control in the positive direction, as the pause menu does. The adjust rule
/// lives in `ShellAudioControl`.
///
/// The index is clamped to the settings rows, not the game list.
fn adjust_settings_row(
    index: usize,
    state: &mut crate::launcher::ShellLauncherState,
    settings: Option<&mut ResMut<ambition_persistence::settings::UserSettings>>,
) {
    let row = index.min(ShellAudioControl::ALL.len().saturating_sub(1));
    state.selected = row;
    if let (Some(control), Some(settings)) = (ShellAudioControl::ALL.get(row), settings) {
        control.adjust(1, settings);
    }
}

fn process_launcher_commands(
    mut commands: MessageReader<ShellLauncherCommand>,
    catalog: Res<ShellLaunchCatalog>,
    presentation: Res<ShellLauncherPresentation>,
    mut state: ResMut<ShellLauncherState>,
    mut shell: MessageWriter<ShellCommand>,
    // `Option`: a thin composition may have no user settings. The title screen
    // must still open.
    mut settings: Option<ResMut<ambition_persistence::settings::UserSettings>>,
) {
    if !state.active {
        return;
    }
    let available: Vec<_> = catalog
        .entries
        .iter()
        .filter(|entry| entry.available)
        .collect();
    // The Exit row sits after the last available experience.
    let exit_index = presentation.exit_label.is_some().then_some(available.len());
    let selectable = available.len() + usize::from(exit_index.is_some());
    // Not an early return: the settings tab must work with an empty catalog.
    let has_launchable = selectable > 0;
    for command in commands.read() {
        match command {
            // Tab arithmetic (with wraparound) lives on `LauncherTab`.
            ShellLauncherCommand::CycleTab(bump) => {
                state.tab = state.tab.cycled(*bump);
                // Both tabs share one row cursor, so reset it on a tab change.
                state.selected = 0;
            }
            // The pointer form of `CycleTab`. It also resets the shared cursor.
            ShellLauncherCommand::SelectTab(index) => {
                state.tab = crate::launcher::LauncherTab::at_index(*index);
                state.selected = 0;
            }
            ShellLauncherCommand::SelectRow(row) => {
                state.selected = *row;
            }
            // The adjust rule lives in `ShellAudioControl`, shared with the
            // pause menu.
            ShellLauncherCommand::AdjustSetting(direction) => {
                if let Some(control) = ShellAudioControl::ALL.get(state.selected) {
                    if let Some(settings) = settings.as_mut() {
                        control.adjust(*direction, settings);
                    }
                }
            }
            // Game-list arms guard themselves, so tab and volume commands still
            // work with no games. Uses `ListCursor` (wraps), like the other
            // menus, and clamps a stale `selected` first.
            ShellLauncherCommand::Previous | ShellLauncherCommand::Next => {
                // Guard inside the arm, not as a match guard, so the match stays
                // exhaustive with no catch-all.
                if !has_launchable {
                    continue;
                }
                let mut cursor = ambition_ui_nav::ListCursor::new(state.selected, selectable);
                cursor.apply_directional(
                    matches!(command, ShellLauncherCommand::Previous),
                    matches!(command, ShellLauncherCommand::Next),
                );
                state.selected = cursor.selected();
            }
            ShellLauncherCommand::LaunchSelected => {
                // Same rule as `Activate`: confirm on the settings tab is not a
                // launch. This path is keyboard and controller confirm.
                if state.tab == crate::launcher::LauncherTab::Settings {
                    adjust_settings_row(state.selected, &mut state, settings.as_mut());
                    continue;
                }
                // Zero rows would underflow `selectable - 1`; an empty catalog
                // with no exit label is supported. After the Settings check, so
                // settings rows still work on an empty Home list.
                if !has_launchable {
                    continue;
                }
                let selected = state.selected.min(selectable - 1);
                if exit_index == Some(selected) {
                    shell.write(ShellCommand::ExitProcess);
                } else if let Some(entry) = available.get(selected) {
                    shell.write(ShellCommand::GoTo(entry.route_id.clone()));
                }
            }
            ShellLauncherCommand::Focus(index)
                if state.tab == crate::launcher::LauncherTab::Settings =>
            {
                // Clamp to the settings row count, not the game count.
                state.selected = (*index).min(ShellAudioControl::ALL.len() - 1);
            }
            ShellLauncherCommand::Focus(index) if !has_launchable => {
                // Same zero-row rule as the confirm arms. Settings `Focus` is
                // matched above.
                let _ = index;
            }
            ShellLauncherCommand::Focus(index) => {
                // Clamp: the row count can shrink between the hover and this
                // frame.
                state.selected = (*index).min(selectable - 1);
            }
            ShellLauncherCommand::Activate(index) => {
                // The index is a position in the list the current tab shows.
                // Settings rows also use `BasicLauncherAction(index)`, so on
                // the settings tab this must adjust, not launch a game.
                if state.tab == crate::launcher::LauncherTab::Settings {
                    adjust_settings_row(*index, &mut state, settings.as_mut());
                    continue;
                }
                // Zero rows would underflow `selectable - 1`; an empty catalog
                // with no exit label is supported. After the Settings check, so
                // settings rows still work on an empty Home list.
                if !has_launchable {
                    continue;
                }
                let selected = (*index).min(selectable - 1);
                state.selected = selected;
                if exit_index == Some(selected) {
                    shell.write(ShellCommand::ExitProcess);
                } else if let Some(entry) = available.get(selected) {
                    shell.write(ShellCommand::GoTo(entry.route_id.clone()));
                }
            }
        }
    }
}
