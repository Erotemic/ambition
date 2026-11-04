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
    LoadPresentations, LoadForegroundPhase, LoadPresentationAction,
    LoadPresentationCommand, LoadPresentationEvent, LoadPresentationModel, LoadPresentationSet,
};

#[derive(Default)]
pub struct AmbitionLoadPresentationPlugin;

impl Plugin for AmbitionLoadPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Time>()
            .init_resource::<LoadPresentations>()
            .add_message::<LoadPresentationCommand>()
            .add_message::<LoadPresentationAction>()
            .add_message::<LoadPresentationEvent>()
            .add_message::<LoadActivitySignal>()
            .add_systems(Update, declare_loading_context
                .in_set(ambition_input::InputSet::ResolveContext)
                .after(LoadPresentationSet::Drive))
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
                apply_presentation_commands.in_set(LoadPresentationSet::Observe),
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
            // Retirement from an action must reach cleanup and rendering in this frame.
            .add_systems(
                Update,
                apply_presentation_commands.in_set(LoadPresentationSet::Finalize),
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
    presentations: Res<LoadPresentations>,
    mut participants: Query<(&ambition_input::InputParticipant, &mut ambition_input::ParticipantContexts)>,
) {
    use ambition_input::participant::{ContextClaim, LOADING_CONTEXT, context_priority};
    for (participant, mut contexts) in &mut participants {
        let capture = presentations.iter().any(|entry| {
            entry.foreground.phase != LoadForegroundPhase::HiddenGrace
                && entry.foreground.spec.input_owner.is_some_and(|owner| owner.accepts(participant.id))
        });
        if contexts.is_declared(LOADING_CONTEXT) != capture {
            contexts.sync(ContextClaim::capturing(LOADING_CONTEXT, context_priority::LOADING), capture);
        }
    }
}

/// Run while a load is present, and once after its retirement for cleanup.
fn a_load_is_presented_or_was() -> impl FnMut(
    Res<LoadPresentations>,
    Res<Messages<LoadPresentationCommand>>,
    Res<Messages<LoadPresentationAction>>,
    Res<Messages<LoadActivitySignal>>,
    Query<(), With<LoadActivityScopedEntity>>,
) -> bool + Clone {
    let mut was_presented = false;
    move |presentations, commands, actions, signals, scoped| {
        let presented = !presentations.is_empty() || !commands.is_empty()
            || !actions.is_empty() || !signals.is_empty() || !scoped.is_empty();
        let run = presented || was_presented;
        was_presented = presented;
        run
    }
}

fn apply_presentation_commands(
    mut commands: ResMut<Messages<LoadPresentationCommand>>,
    mut presentations: ResMut<LoadPresentations>,
) {
    for command in commands.drain() {
        match command {
            LoadPresentationCommand::Begin { owner, barrier, spec } => {
                presentations.begin(ActiveLoadForeground {
                    owner, barrier, spec,
                    phase: LoadForegroundPhase::HiddenGrace, elapsed: Duration::ZERO,
                    activity_activation_id: None, engaged: false, ready_released: false,
                });
            }
            LoadPresentationCommand::Finish { owner } | LoadPresentationCommand::Cancel { owner } => {
                presentations.entries.remove(&owner);
            }
        }
    }
}

fn drive_foreground(
    time: Res<Time>,
    loads: Res<LoadCoordinator>,
    mut presentations: ResMut<LoadPresentations>,
) {
    let LoadPresentations { entries, next_activity_activation } = &mut *presentations;
    for entry in entries.values_mut() {
        let active = &mut entry.foreground;
        active.elapsed = active.elapsed.saturating_add(Duration::from_secs_f32(time.delta_secs()));
        let Some(snapshot) = loads.snapshot(&active.barrier.load_id, &active.barrier.barrier_id) else { continue };
        if active.phase == LoadForegroundPhase::HiddenGrace && active.elapsed >= active.spec.reveal_after {
            active.phase = LoadForegroundPhase::Visible;
            if let Some(activity_id) = active.spec.activity.clone() {
                *next_activity_activation = next_activity_activation.saturating_add(1);
                let activation_id = *next_activity_activation;
                active.activity_activation_id = Some(activation_id);
                entry.activity = Some(ActiveLoadActivity {
                    input_owner: active.spec.input_owner, activation_id, activity_id,
                    owner: active.owner.clone(), barrier: active.barrier.clone(),
                });
            }
        }
        if matches!(snapshot.readiness, BarrierReadiness::Failed | BarrierReadiness::Cancelled | BarrierReadiness::Superseded) {
            active.phase = LoadForegroundPhase::Failed;
        } else if snapshot.ready() {
            let should_hold = !active.ready_released && active.spec.ready_policy.holds_ready(
                active.phase != LoadForegroundPhase::HiddenGrace, active.engaged);
            if should_hold { active.phase = LoadForegroundPhase::ReadyHold; }
            else if active.phase == LoadForegroundPhase::ReadyHold { active.phase = LoadForegroundPhase::Visible; }
        }
        entry.model = LoadPresentationModel::from_snapshot(snapshot, active);
    }
}

fn process_activity_signals(
    mut signals: MessageReader<LoadActivitySignal>,
    mut presentations: ResMut<LoadPresentations>,
) {
    for signal in signals.read() {
        let signal_activation = match signal {
            LoadActivitySignal::Engaged { activation_id }
            | LoadActivitySignal::Finished { activation_id, .. }
            | LoadActivitySignal::Failed { activation_id, .. } => *activation_id,
        };
        let Some(entry) = presentations.entries.values_mut().find(|entry|
            entry.activity.as_ref().is_some_and(|activity| activity.activation_id == signal_activation)) else { continue };
        match signal {
            LoadActivitySignal::Engaged { .. } => entry.foreground.engaged = true,
            LoadActivitySignal::Finished { outcome, .. } => {
                entry.last_outcome = Some(outcome.clone());
                entry.activity = None;
            }
            LoadActivitySignal::Failed { .. } => entry.activity = None,
        }
    }
}

fn process_presentation_actions(
    mut actions: MessageReader<LoadPresentationAction>,
    mut presentations: ResMut<LoadPresentations>,
    mut events: MessageWriter<LoadPresentationEvent>,
) {
    for action in actions.read() {
        let owner = match action {
            LoadPresentationAction::Continue { owner } | LoadPresentationAction::Retry { owner }
            | LoadPresentationAction::Cancel { owner } | LoadPresentationAction::Quit { owner } => owner,
        };
        let Some(entry) = presentations.entries.get_mut(owner) else { continue };
        let active = &mut entry.foreground;
        match action {
            LoadPresentationAction::Continue { owner } => {
                active.ready_released = true;
                if active.phase == LoadForegroundPhase::ReadyHold { active.phase = LoadForegroundPhase::Visible; }
                entry.activity = None;
                entry.model.ready_hold = false;
                events.write(LoadPresentationEvent::ContinueRequested { owner: owner.clone() });
            }
            LoadPresentationAction::Retry { owner } => {
                events.write(LoadPresentationEvent::RetryRequested { owner: owner.clone(), barrier: active.barrier.clone() });
            }
            LoadPresentationAction::Cancel { owner } => {
                events.write(LoadPresentationEvent::CancelRequested { owner: owner.clone() });
            }
            LoadPresentationAction::Quit { owner } => {
                events.write(LoadPresentationEvent::QuitRequested { owner: owner.clone() });
            }
        }
    }
}


fn cleanup_activity_entities(
    mut commands: Commands,
    presentations: Res<LoadPresentations>,
    entities: Query<(bevy::prelude::Entity, &LoadActivityScopedEntity)>,
) {
    for (entity, scope) in &entities {
        if !presentations.iter().any(|entry| entry.activity.as_ref()
            .is_some_and(|activity| activity.activation_id == scope.activation_id)) {
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
        app.init_resource::<LoadPresentations>();
        app.init_resource::<SeatInputContexts>();
        app.add_systems(Update, (declare_loading_context, resolve_active_input_context).chain());
        for id in [ParticipantId::PRIMARY, ParticipantId::SECONDARY] {
            let mut contexts = ParticipantContexts::default();
            contexts.declare(ContextClaim::capturing(GAMEPLAY_CONTEXT, context_priority::GAMEPLAY));
            app.world_mut().spawn((InputParticipant::with_id(id), contexts));
        }
        let mut spec = crate::LoadExperienceSpec::basic("fixture");
        spec.input_owner = Some(InputOwner::Participant(ParticipantId::SECONDARY));
        app.world_mut().resource_mut::<LoadPresentations>().begin(ActiveLoadForeground {
            owner: crate::LoadPresentationOwnerId::new("fixture"),
            barrier: ambition_load::LoadBarrierRef::new("fixture", "ready"), spec,
            phase: LoadForegroundPhase::HiddenGrace, elapsed: Duration::ZERO,
            activity_activation_id: None, engaged: false, ready_released: false,
        });
        app.update();
        assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(0));
        assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(1));
        app.world_mut().resource_mut::<LoadPresentations>().entries.values_mut().next().unwrap().foreground.phase = LoadForegroundPhase::Visible;
        app.update();
        assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(0));
        assert_eq!(app.world().resource::<SeatInputContexts>().for_seat(1).owner(), Some(LOADING_CONTEXT));
        app.world_mut().resource_mut::<LoadPresentations>().entries.clear();
        app.update();
        assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(0));
        assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(1));
    }
}
