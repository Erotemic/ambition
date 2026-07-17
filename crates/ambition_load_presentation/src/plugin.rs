//! Contributor-neutral hidden-grace, ready-hold, activity, and cleanup lifecycle.

use std::time::Duration;

use ambition_load::{BarrierReadiness, LoadCoordinator};
use bevy::prelude::{
    App, Commands, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, Query, Res, ResMut,
    Time, Update,
};

use crate::{
    ActiveLoadActivity, ActiveLoadForeground, LoadActivityScopedEntity, LoadActivitySignal,
    LoadActivityState, LoadForegroundPhase, LoadForegroundState, LoadPresentationAction,
    LoadPresentationCommand, LoadPresentationEvent, LoadPresentationModel, LoadPresentationSet,
    ReadyTransitionPolicy,
};

#[derive(Default)]
pub struct AmbitionLoadPresentationPlugin;

impl Plugin for AmbitionLoadPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Time>()
            .init_resource::<LoadForegroundState>()
            .init_resource::<LoadPresentationModel>()
            .init_resource::<LoadActivityState>()
            .add_message::<LoadPresentationCommand>()
            .add_message::<LoadPresentationAction>()
            .add_message::<LoadPresentationEvent>()
            .add_message::<LoadActivitySignal>()
            .configure_sets(
                Update,
                (
                    LoadPresentationSet::Observe,
                    LoadPresentationSet::Activity,
                    LoadPresentationSet::ActivitySignals,
                    LoadPresentationSet::Drive,
                    LoadPresentationSet::Input,
                    LoadPresentationSet::Actions,
                    LoadPresentationSet::Finalize,
                    LoadPresentationSet::Cleanup,
                    LoadPresentationSet::Render,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                observe_presentation_commands.in_set(LoadPresentationSet::Observe),
            )
            .add_systems(
                Update,
                process_activity_signals.in_set(LoadPresentationSet::ActivitySignals),
            )
            .add_systems(Update, drive_foreground.in_set(LoadPresentationSet::Drive))
            .add_systems(
                Update,
                process_presentation_actions.in_set(LoadPresentationSet::Actions),
            )
            .add_systems(
                Update,
                finalize_presentation_commands.in_set(LoadPresentationSet::Finalize),
            )
            .add_systems(
                Update,
                cleanup_activity_entities.in_set(LoadPresentationSet::Cleanup),
            );
    }
}

fn observe_presentation_commands(
    mut commands: MessageReader<LoadPresentationCommand>,
    mut foreground: ResMut<LoadForegroundState>,
    mut model: ResMut<LoadPresentationModel>,
    mut activity: ResMut<LoadActivityState>,
) {
    for command in commands.read() {
        match command {
            LoadPresentationCommand::Begin {
                owner,
                barrier,
                spec,
            } => {
                foreground.active = Some(ActiveLoadForeground {
                    owner: owner.clone(),
                    barrier: barrier.clone(),
                    spec: spec.clone(),
                    phase: LoadForegroundPhase::HiddenGrace,
                    elapsed: Duration::ZERO,
                    activity_activation_id: None,
                    engaged: false,
                    ready_released: false,
                });
                activity.active = None;
                *model = LoadPresentationModel::default();
            }
            LoadPresentationCommand::Finish { owner }
            | LoadPresentationCommand::Cancel { owner } => {
                if foreground.clear_owner(owner) {
                    activity.active = None;
                    *model = LoadPresentationModel::default();
                }
            }
        }
    }
}

fn drive_foreground(
    time: Res<Time>,
    loads: Res<LoadCoordinator>,
    mut foreground: ResMut<LoadForegroundState>,
    mut model: ResMut<LoadPresentationModel>,
    mut activity: ResMut<LoadActivityState>,
) {
    let Some(mut active) = foreground.active.take() else {
        *model = LoadPresentationModel::default();
        return;
    };
    active.elapsed = active
        .elapsed
        .saturating_add(Duration::from_secs_f32(time.delta_secs()));
    let Some(snapshot) = loads.snapshot(&active.barrier.load_id, &active.barrier.barrier_id) else {
        foreground.active = Some(active);
        return;
    };

    if active.phase == LoadForegroundPhase::HiddenGrace
        && active.elapsed >= active.spec.reveal_after
    {
        active.phase = LoadForegroundPhase::Visible;
        if let Some(activity_id) = active.spec.activity.clone() {
            let activation_id = foreground.next_activity_activation();
            active.activity_activation_id = Some(activation_id);
            activity.active = Some(ActiveLoadActivity {
                activation_id,
                activity_id,
                owner: active.owner.clone(),
                barrier: active.barrier.clone(),
            });
        }
    }

    if matches!(
        snapshot.readiness,
        BarrierReadiness::Failed | BarrierReadiness::Cancelled | BarrierReadiness::Superseded
    ) {
        active.phase = LoadForegroundPhase::Failed;
    } else if snapshot.ready() {
        let should_hold = !active.ready_released
            && active.spec.ready_policy.holds_ready(
                active.phase != LoadForegroundPhase::HiddenGrace,
                active.engaged,
            );
        if should_hold {
            active.phase = LoadForegroundPhase::ReadyHold;
        } else if active.phase == LoadForegroundPhase::ReadyHold {
            active.phase = LoadForegroundPhase::Visible;
        }
    }

    *model = LoadPresentationModel::from_snapshot(snapshot, &active);
    foreground.active = Some(active);
}

fn process_activity_signals(
    mut signals: MessageReader<LoadActivitySignal>,
    mut foreground: ResMut<LoadForegroundState>,
    mut activity: ResMut<LoadActivityState>,
) {
    for signal in signals.read() {
        let Some(active_activity) = activity.active.as_ref() else {
            continue;
        };
        let signal_activation = match signal {
            LoadActivitySignal::Engaged { activation_id }
            | LoadActivitySignal::Finished { activation_id, .. }
            | LoadActivitySignal::Failed { activation_id, .. } => *activation_id,
        };
        if signal_activation != active_activity.activation_id {
            continue;
        }
        match signal {
            LoadActivitySignal::Engaged { .. } => {
                if let Some(active) = foreground.active.as_mut() {
                    active.engaged = true;
                }
            }
            LoadActivitySignal::Finished { outcome, .. } => {
                activity.last_outcome = Some(outcome.clone());
                activity.active = None;
            }
            LoadActivitySignal::Failed { .. } => {
                activity.active = None;
            }
        }
    }
}

fn process_presentation_actions(
    mut actions: MessageReader<LoadPresentationAction>,
    mut foreground: ResMut<LoadForegroundState>,
    mut model: ResMut<LoadPresentationModel>,
    mut activity: ResMut<LoadActivityState>,
    mut events: MessageWriter<LoadPresentationEvent>,
) {
    for action in actions.read() {
        let owner = match action {
            LoadPresentationAction::Continue { owner }
            | LoadPresentationAction::Retry { owner }
            | LoadPresentationAction::Cancel { owner }
            | LoadPresentationAction::Quit { owner } => owner,
        };
        let Some(active) = foreground.active.as_mut() else {
            continue;
        };
        if &active.owner != owner {
            continue;
        }
        match action {
            LoadPresentationAction::Continue { owner } => {
                active.ready_released = true;
                if active.phase == LoadForegroundPhase::ReadyHold {
                    active.phase = LoadForegroundPhase::Visible;
                }
                activity.active = None;
                model.ready_hold = false;
                events.write(LoadPresentationEvent::ContinueRequested {
                    owner: owner.clone(),
                });
            }
            LoadPresentationAction::Retry { owner } => {
                events.write(LoadPresentationEvent::RetryRequested {
                    owner: owner.clone(),
                    barrier: active.barrier.clone(),
                });
            }
            LoadPresentationAction::Cancel { owner } => {
                events.write(LoadPresentationEvent::CancelRequested {
                    owner: owner.clone(),
                });
            }
            LoadPresentationAction::Quit { owner } => {
                events.write(LoadPresentationEvent::QuitRequested {
                    owner: owner.clone(),
                });
            }
        }
    }
}

fn finalize_presentation_commands(
    mut commands: MessageReader<LoadPresentationCommand>,
    mut foreground: ResMut<LoadForegroundState>,
    mut model: ResMut<LoadPresentationModel>,
    mut activity: ResMut<LoadActivityState>,
) {
    for command in commands.read() {
        let owner = match command {
            LoadPresentationCommand::Finish { owner }
            | LoadPresentationCommand::Cancel { owner } => owner,
            LoadPresentationCommand::Begin { .. } => continue,
        };
        if foreground.clear_owner(owner) {
            activity.active = None;
            *model = LoadPresentationModel::default();
        }
    }
}

fn cleanup_activity_entities(
    mut commands: Commands,
    activity: Res<LoadActivityState>,
    entities: Query<(bevy::prelude::Entity, &LoadActivityScopedEntity)>,
) {
    let active_id = activity.active.as_ref().map(|item| item.activation_id);
    for (entity, scope) in &entities {
        if Some(scope.activation_id) != active_id {
            commands.entity(entity).despawn();
        }
    }
}
