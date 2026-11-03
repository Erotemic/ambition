//! Contributor-neutral hidden-grace, ready-hold, activity, and cleanup lifecycle.

use std::time::Duration;

use ambition_load::{BarrierReadiness, LoadCoordinator};
use bevy::ecs::schedule::SystemSet;
use bevy::prelude::{
    App, Commands, IntoScheduleConfigs, MessageReader, MessageWriter, Messages, Plugin, Query, Res,
    ResMut, Time, Update, With,
};

use crate::{
    ActiveLoadActivity, ActiveLoadForeground, LoadActivityScopedEntity, LoadActivitySignal,
    LoadActivityState, LoadForegroundPhase, LoadForegroundState, LoadPresentationAction,
    LoadPresentationCommand, LoadPresentationEvent, LoadPresentationModel, LoadPresentationSet,
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
            .add_systems(Update, declare_loading_context
                .in_set(ambition_input::InputSet::ResolveContext)
                .after(LoadPresentationSet::Drive)
                .before(ambition_input::resolve_active_input_context))
            .configure_sets(
                Update,
                (
                    LoadPresentationSet::Observe,
                    LoadPresentationSet::Activity,
                    LoadPresentationSet::ActivitySignals,
                    LoadPresentationSet::Drive,
                    LoadPresentationSet::Actions,
                    LoadPresentationSet::Finalize,
                    LoadPresentationSet::Cleanup,
                    LoadPresentationSet::Render,
                )
                    .chain()
                    .in_set(LoadPresentationPass),
            )
            .configure_sets(Update, LoadPresentationPass.run_if(a_load_is_presented_or_was()))
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

/// Every [`LoadPresentationSet`]: the pass that runs only around a load.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct LoadPresentationPass;

/// A visible foreground captures only the participants named by its owner.
fn declare_loading_context(
    foreground: Res<LoadForegroundState>,
    mut participants: Query<(&ambition_input::InputParticipant, &mut ambition_input::ParticipantContexts)>,
) {
    use ambition_input::participant::{ContextClaim, LOADING_CONTEXT, context_priority};
    let owner = foreground.active.as_ref()
        .filter(|active| active.phase != LoadForegroundPhase::HiddenGrace)
        .and_then(|active| active.spec.input_owner);
    for (participant, mut contexts) in &mut participants {
        let capture = owner.is_some_and(|owner| owner.accepts(participant.id));
        if contexts.is_declared(LOADING_CONTEXT) != capture {
            contexts.sync(ContextClaim::capturing(LOADING_CONTEXT, context_priority::LOADING), capture);
        }
    }
}

/// Run while a load is present, and once after its retirement for cleanup.
fn a_load_is_presented_or_was() -> impl FnMut(
    Res<LoadForegroundState>,
    Res<LoadActivityState>,
    Res<Messages<LoadPresentationCommand>>,
    Res<Messages<LoadPresentationAction>>,
    Res<Messages<LoadActivitySignal>>,
    Query<(), With<LoadActivityScopedEntity>>,
) -> bool
       + Clone {
    let mut was_presented = false;
    move |foreground, activity, commands, actions, signals, scoped| {
        let presented = foreground.active.is_some()
            || activity.active.is_some()
            || !commands.is_empty()
            || !actions.is_empty()
            || !signals.is_empty()
            || !scoped.is_empty();
        let run = presented || was_presented;
        was_presented = presented;
        run
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
                input_owner: active.spec.input_owner,
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

#[cfg(test)]
mod context_tests {
    use super::*;
    use ambition_input::participant::*;

    #[test]
    fn the_loading_foreground_claims_only_its_owner_and_releases_on_retirement() {
        let mut app = App::new();
        app.init_resource::<LoadForegroundState>();
        app.init_resource::<SeatInputContexts>();
        app.add_systems(Update, (declare_loading_context, resolve_active_input_context).chain());
        for id in [ParticipantId::PRIMARY, ParticipantId::SECONDARY] {
            let mut contexts = ParticipantContexts::default();
            contexts.declare(ContextClaim::capturing(GAMEPLAY_CONTEXT, context_priority::GAMEPLAY));
            app.world_mut().spawn((InputParticipant::with_id(id), contexts));
        }
        let mut spec = crate::LoadExperienceSpec::basic("fixture");
        spec.input_owner = Some(InputOwner::Participant(ParticipantId::SECONDARY));
        app.world_mut().resource_mut::<LoadForegroundState>().active = Some(ActiveLoadForeground {
            owner: crate::LoadPresentationOwnerId::new("fixture"),
            barrier: ambition_load::LoadBarrierRef::new("fixture", "ready"), spec,
            phase: LoadForegroundPhase::HiddenGrace, elapsed: Duration::ZERO,
            activity_activation_id: None, engaged: false, ready_released: false,
        });
        app.update();
        assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(0));
        assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(1));
        app.world_mut().resource_mut::<LoadForegroundState>().active.as_mut().unwrap().phase = LoadForegroundPhase::Visible;
        app.update();
        assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(0));
        assert_eq!(app.world().resource::<SeatInputContexts>().for_seat(1).owner(), Some(LOADING_CONTEXT));
        app.world_mut().resource_mut::<LoadForegroundState>().active = None;
        app.update();
        assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(0));
        assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(1));
    }
}
