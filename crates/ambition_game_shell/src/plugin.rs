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

use crate::{
    ActiveGameplaySession, ActiveShellSequence, AmbitionGameShellSet, PreparedSessionRegistry,
    ShellCommand, ShellEvent, ShellExperienceRegistry, ShellHostConfiguration, ShellInputFocus,
    ShellLaunchCatalog, ShellLauncherCommand, ShellLauncherPresentation, ShellLauncherState,
    ShellRouteCatalog, ShellRouteHolds, ShellRouter, ShellScopedEntity, ShellSegmentScopedEntity,
    ShellSequenceCatalog, ShellSequenceCommand, ShellSequenceRuntime, ShellSequenceSet,
    BASIC_LAUNCHER_EXPERIENCE,
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
            .init_resource::<ShellInputFocus>()
            .init_resource::<ShellRouteHolds>()
            .add_message::<ShellCommand>()
            .add_message::<ShellEvent>()
            .add_message::<ambition_platformer_primitives::developer_hotkeys::DeveloperAction>()
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
                cleanup_scoped_entities.in_set(AmbitionGameShellSet::Cleanup),
            );
    }
}

impl Plugin for ShellSequencePlugin {
    fn build(&self, app: &mut App) {
        // Idempotent: a windowed host's DefaultPlugins already own Time; a bare
        // headless host needs one so drive_sequence can tick.
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
            .add_systems(
                Update,
                start_or_stop_sequence.in_set(ShellSequenceSet::Sync),
            )
            // The sequence surface OWNS the startup-acknowledge input
            // context: it declares the claim while a card sequence is
            // active and retracts it when the sequence ends. Ownership is
            // declared, never inferred from GameMode or actor presence.
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
                    process_launcher_commands,
                )
                    .chain()
                    .after(AmbitionGameShellSet::Pending),
            )
            // The launcher surface OWNS the launcher input context: claimed
            // while the launcher route is active, retracted when it is not.
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

/// While the launcher route is active, the launcher context owns the
/// participant's actions — capturing, so gameplay actions cannot route
/// underneath the title menu.
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
    mut actions: MessageReader<ambition_platformer_primitives::developer_hotkeys::DeveloperAction>,
    active: Option<Res<ActiveGameplaySession>>,
    mut shell: MessageWriter<ShellCommand>,
) {
    let requested = actions.read().any(|action| {
        *action == ambition_platformer_primitives::developer_hotkeys::DeveloperAction::QuitToHome
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
    mut focus: ResMut<ShellInputFocus>,
    mut events: MessageWriter<ShellEvent>,
) {
    for command in commands.read() {
        for event in router.apply(command.clone(), &catalog, &host, &mut loads, &mut prepared) {
            if let ShellEvent::RouteActivated(active) = &event {
                focus.activation_id = Some(active.activation_id);
            }
            if matches!(event, ShellEvent::ExitRequested) {
                focus.activation_id = None;
            }
            events.write(event);
        }
    }
}

fn advance_pending_route(
    catalog: Res<ShellRouteCatalog>,
    mut loads: ResMut<LoadCoordinator>,
    mut prepared: ResMut<PreparedSessionRegistry>,
    mut router: ResMut<ShellRouter>,
    holds: Res<ShellRouteHolds>,
    mut focus: ResMut<ShellInputFocus>,
    mut events: MessageWriter<ShellEvent>,
) {
    for event in router.advance_pending(&catalog, &mut loads, &mut prepared, &holds) {
        if let ShellEvent::RouteActivated(active) = &event {
            focus.activation_id = Some(active.activation_id);
        }
        events.write(event);
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

fn process_launcher_commands(
    mut commands: MessageReader<ShellLauncherCommand>,
    catalog: Res<ShellLaunchCatalog>,
    presentation: Res<ShellLauncherPresentation>,
    mut state: ResMut<ShellLauncherState>,
    mut shell: MessageWriter<ShellCommand>,
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
    if selectable == 0 {
        return;
    }
    for command in commands.read() {
        match command {
            ShellLauncherCommand::Previous => {
                state.selected = state.selected.checked_sub(1).unwrap_or(selectable - 1);
            }
            ShellLauncherCommand::Next => {
                state.selected = (state.selected + 1) % selectable;
            }
            ShellLauncherCommand::LaunchSelected => {
                let selected = state.selected.min(selectable - 1);
                if exit_index == Some(selected) {
                    shell.write(ShellCommand::ExitProcess);
                } else if let Some(entry) = available.get(selected) {
                    shell.write(ShellCommand::GoTo(entry.route_id.clone()));
                }
            }
            ShellLauncherCommand::Activate(index) => {
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
