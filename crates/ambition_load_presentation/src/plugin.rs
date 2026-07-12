//! Shell-integrated hidden-grace, ready-hold, activity, and cleanup lifecycle.

use std::time::Duration;

use ambition_game_shell::{
    AmbitionGameShellSet, ShellCommand, ShellEvent, ShellHoldId, ShellRouteHolds, ShellRouter,
};
use ambition_load::{BarrierReadiness, LoadCoordinator};
use bevy::prelude::{
    App, Commands, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, Query, Res, ResMut,
    Time, Update,
};

use crate::{
    ActiveLoadActivity, ActiveLoadForeground, LoadActivityScopedEntity, LoadActivitySignal,
    LoadActivityState, LoadForegroundPhase, LoadForegroundState, LoadPresentationAction,
    LoadPresentationCatalog, LoadPresentationEvent, LoadPresentationModel, LoadPresentationSet,
    ReadyTransitionPolicy,
};

const LOAD_PRESENTATION_HOLD: &str = "ambition.load-presentation.ready-hold";

#[derive(Default)]
pub struct AmbitionLoadPresentationPlugin;

impl Plugin for AmbitionLoadPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Time>()
            .init_resource::<LoadPresentationCatalog>()
            .init_resource::<LoadForegroundState>()
            .init_resource::<LoadPresentationModel>()
            .init_resource::<LoadActivityState>()
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
                )
                    .chain()
                    .after(AmbitionGameShellSet::Commands)
                    .before(AmbitionGameShellSet::Pending),
            )
            .configure_sets(
                Update,
                (
                    LoadPresentationSet::Finalize,
                    LoadPresentationSet::Cleanup,
                    LoadPresentationSet::Render,
                )
                    .chain()
                    .after(AmbitionGameShellSet::Pending),
            )
            .add_systems(
                Update,
                observe_shell_waits.in_set(LoadPresentationSet::Observe),
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
                finalize_activated_route.in_set(LoadPresentationSet::Finalize),
            )
            .add_systems(
                Update,
                cleanup_activity_entities.in_set(LoadPresentationSet::Cleanup),
            );
    }
}

fn observe_shell_waits(
    mut events: MessageReader<ShellEvent>,
    catalog: Res<LoadPresentationCatalog>,
    mut foreground: ResMut<LoadForegroundState>,
    mut model: ResMut<LoadPresentationModel>,
    mut activity: ResMut<LoadActivityState>,
    mut holds: ResMut<ShellRouteHolds>,
) {
    for event in events.read() {
        match event {
            ShellEvent::WaitingForLoad { route_id, barrier } => {
                let spec = catalog.for_route(route_id).clone();
                if let Some(previous) = foreground.active.take() {
                    holds.release(
                        &previous.route_id,
                        &ShellHoldId::new(LOAD_PRESENTATION_HOLD),
                    );
                }
                holds.release(route_id, &ShellHoldId::new(LOAD_PRESENTATION_HOLD));
                foreground.active = Some(ActiveLoadForeground {
                    route_id: route_id.clone(),
                    barrier: barrier.clone(),
                    spec,
                    phase: LoadForegroundPhase::HiddenGrace,
                    elapsed: Duration::ZERO,
                    activity_activation_id: None,
                    engaged: false,
                });
                activity.active = None;
                *model = LoadPresentationModel::default();
            }
            _ => {}
        }
    }
}

fn finalize_activated_route(
    mut events: MessageReader<ShellEvent>,
    mut foreground: ResMut<LoadForegroundState>,
    mut model: ResMut<LoadPresentationModel>,
    mut activity: ResMut<LoadActivityState>,
    mut holds: ResMut<ShellRouteHolds>,
) {
    for event in events.read() {
        let ShellEvent::RouteActivated(_) = event else {
            continue;
        };
        if let Some(previous) = foreground.active.take() {
            holds.release(
                &previous.route_id,
                &ShellHoldId::new(LOAD_PRESENTATION_HOLD),
            );
        }
        activity.active = None;
        *model = LoadPresentationModel::default();
    }
}

fn drive_foreground(
    time: Res<Time>,
    loads: Res<LoadCoordinator>,
    mut foreground: ResMut<LoadForegroundState>,
    mut model: ResMut<LoadPresentationModel>,
    mut activity: ResMut<LoadActivityState>,
    mut holds: ResMut<ShellRouteHolds>,
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
                route_id: active.route_id.clone(),
                barrier: active.barrier.clone(),
            });
        }
        let needs_provisional_hold = active.spec.ready_policy
            == ReadyTransitionPolicy::AwaitConfirmation
            || (active.spec.ready_policy == ReadyTransitionPolicy::AutoUnlessEngaged
                && active.spec.activity.is_some());
        if needs_provisional_hold {
            holds.hold(
                active.route_id.clone(),
                ShellHoldId::new(LOAD_PRESENTATION_HOLD),
            );
        }
    }

    if matches!(
        snapshot.readiness,
        BarrierReadiness::Failed | BarrierReadiness::Cancelled | BarrierReadiness::Superseded
    ) {
        active.phase = LoadForegroundPhase::Failed;
        holds.hold(
            active.route_id.clone(),
            ShellHoldId::new(LOAD_PRESENTATION_HOLD),
        );
    } else if snapshot.ready() {
        let should_hold = active.spec.ready_policy.holds_ready(
            active.phase != LoadForegroundPhase::HiddenGrace,
            active.engaged,
        );
        if should_hold {
            active.phase = LoadForegroundPhase::ReadyHold;
            holds.hold(
                active.route_id.clone(),
                ShellHoldId::new(LOAD_PRESENTATION_HOLD),
            );
        } else {
            holds.release(&active.route_id, &ShellHoldId::new(LOAD_PRESENTATION_HOLD));
        }
    }

    *model = LoadPresentationModel::from_snapshot(active.route_id.clone(), snapshot, &active);
    foreground.active = Some(active);
}

fn process_activity_signals(
    mut signals: MessageReader<LoadActivitySignal>,
    mut foreground: ResMut<LoadForegroundState>,
    mut activity: ResMut<LoadActivityState>,
    mut holds: ResMut<ShellRouteHolds>,
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
                    if active.spec.ready_policy == ReadyTransitionPolicy::AutoUnlessEngaged {
                        holds.hold(
                            active.route_id.clone(),
                            ShellHoldId::new(LOAD_PRESENTATION_HOLD),
                        );
                    }
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
    mut router: ResMut<ShellRouter>,
    mut foreground: ResMut<LoadForegroundState>,
    mut activity: ResMut<LoadActivityState>,
    mut holds: ResMut<ShellRouteHolds>,
    mut shell: MessageWriter<ShellCommand>,
    mut events: MessageWriter<LoadPresentationEvent>,
) {
    for action in actions.read() {
        match action {
            LoadPresentationAction::Continue => {
                if let Some(active) = foreground.active.as_ref() {
                    holds.release(&active.route_id, &ShellHoldId::new(LOAD_PRESENTATION_HOLD));
                }
                activity.active = None;
            }
            LoadPresentationAction::Retry => {
                if let Some(active) = foreground.active.as_ref() {
                    events.write(LoadPresentationEvent::RetryRequested {
                        route_id: active.route_id.clone(),
                        barrier: active.barrier.clone(),
                    });
                }
            }
            LoadPresentationAction::CancelToPrevious => {
                let had_active_route = router.active.is_some();
                if let Some(pending) = router.cancel_pending() {
                    holds.release(&pending.route_id, &ShellHoldId::new(LOAD_PRESENTATION_HOLD));
                }
                foreground.active = None;
                activity.active = None;
                if !had_active_route {
                    shell.write(ShellCommand::QuitToHome);
                }
            }
            LoadPresentationAction::QuitToHome => {
                if let Some(pending) = router.cancel_pending() {
                    holds.release(&pending.route_id, &ShellHoldId::new(LOAD_PRESENTATION_HOLD));
                }
                foreground.active = None;
                activity.active = None;
                shell.write(ShellCommand::QuitToHome);
            }
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
