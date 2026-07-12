//! Bevy plugins that drive shell routing, sequences, and launcher commands.

use bevy::prelude::{
    App, Commands, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, Query, Res, ResMut,
    Time, Update,
};

use ambition_load::{AmbitionLoadSet, LoadCoordinator};

use crate::{
    ActiveShellSequence, AmbitionGameShellSet, ShellCommand, ShellEvent, ShellHostConfiguration,
    ShellInputFocus, ShellLaunchCatalog, ShellLauncherCommand, ShellLauncherPresentation,
    ShellLauncherState, ShellRouteCatalog, ShellRouteHolds, ShellRouter, ShellScopedEntity,
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
            .init_resource::<ShellInputFocus>()
            .init_resource::<ShellRouteHolds>()
            .add_message::<ShellCommand>()
            .add_message::<ShellEvent>()
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
                (initialize_shell, process_shell_commands)
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
        app.init_resource::<ShellSequenceCatalog>()
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
            .init_resource::<ShellLauncherPresentation>()
            .init_resource::<ShellLauncherState>()
            .add_message::<ShellLauncherCommand>()
            .add_systems(
                Update,
                (sync_launcher_activation, process_launcher_commands).chain(),
            );
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

fn process_shell_commands(
    mut commands: MessageReader<ShellCommand>,
    catalog: Res<ShellRouteCatalog>,
    host: Res<ShellHostConfiguration>,
    mut loads: ResMut<LoadCoordinator>,
    mut router: ResMut<ShellRouter>,
    mut focus: ResMut<ShellInputFocus>,
    mut events: MessageWriter<ShellEvent>,
) {
    for command in commands.read() {
        for event in router.apply(command.clone(), &catalog, &host, &mut *loads) {
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
    mut router: ResMut<ShellRouter>,
    holds: Res<ShellRouteHolds>,
    mut focus: ResMut<ShellInputFocus>,
    mut events: MessageWriter<ShellEvent>,
) {
    for event in router.advance_pending(&catalog, &mut *loads, &holds) {
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
        let available = catalog
            .entries
            .iter()
            .filter(|entry| entry.available)
            .count();
        state.selected = state.selected.min(available.saturating_sub(1));
    }
}

fn process_launcher_commands(
    mut commands: MessageReader<ShellLauncherCommand>,
    catalog: Res<ShellLaunchCatalog>,
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
    if available.is_empty() {
        return;
    }
    for command in commands.read() {
        match command {
            ShellLauncherCommand::Previous => {
                state.selected = state.selected.checked_sub(1).unwrap_or(available.len() - 1);
            }
            ShellLauncherCommand::Next => {
                state.selected = (state.selected + 1) % available.len();
            }
            ShellLauncherCommand::LaunchSelected => {
                shell.write(ShellCommand::GoTo(
                    available[state.selected.min(available.len() - 1)]
                        .route_id
                        .clone(),
                ));
            }
        }
    }
}
