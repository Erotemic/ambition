use std::time::Duration;

use ambition_load::{
    BarrierReadiness, LoadBarrierId, LoadBarrierRef, LoadBarrierSnapshot, LoadFailure, LoadId,
};

use crate::*;

fn only_presentation(app: &bevy::prelude::App) -> &LoadPresentation {
    app.world().resource::<LoadPresentations>().iter().next().expect("one fixture load")
}

fn foreground(policy: ReadyTransitionPolicy) -> ActiveLoadForeground {
    ActiveLoadForeground {
        owner: LoadPresentationOwnerId::new("fixture-owner"),
        barrier: LoadBarrierRef::new(LoadId::new("load"), LoadBarrierId::new("ready")),
        spec: LoadExperienceSpec {
            id: LoadExperienceId::new("basic"),
            reveal_after: Duration::from_millis(250),
            ready_policy: policy,
            activity: None,
            show_estimated_percentage: true,
            input_owner: Some(ambition_input::InputOwner::AllParticipants),
        },
        phase: LoadForegroundPhase::Visible,
        elapsed: Duration::from_secs(1),
        activity_activation_id: None,
        engaged: false,
        ready_released: false,
    }
}

fn snapshot(active: &ActiveLoadForeground, readiness: BarrierReadiness) -> LoadBarrierSnapshot {
    LoadBarrierSnapshot {
        load_id: active.barrier.load_id.clone(),
        barrier_id: active.barrier.barrier_id.clone(),
        label: "Preparing".to_owned(),
        readiness,
        discovery_open: false,
        completed_steps: 1,
        active_steps: 0,
        known_remaining_steps: 0,
        failed_steps: 0,
        cancelled_steps: 0,
        estimated_additional_steps: None,
        estimated_total_remaining_steps: None,
        completed_labels: vec![],
        active_labels: Vec::new(),
        remaining_labels: Vec::new(),
        streamable_labels: Vec::new(),
        speculative_labels: Vec::new(),
        failures: Vec::new(),
        estimate: Some(ambition_load::ProgressEstimate {
            fraction: 1.0,
            confidence: ambition_load::EstimateConfidence::High,
            basis: ambition_load::EstimateBasis::EqualSteps,
            provenance: "fixture".to_owned(),
            may_decrease: false,
        }),
    }
}

#[test]
fn model_never_reports_pre_ready_one_hundred_percent() {
    let active = foreground(ReadyTransitionPolicy::AutoAdvance);
    let model = LoadPresentationModel::from_snapshot(
        snapshot(&active, BarrierReadiness::Preparing),
        &active,
    );
    assert!(model.estimate.unwrap().fraction < 1.0);
}

#[test]
fn failure_evidence_remains_visible() {
    let active = foreground(ReadyTransitionPolicy::AwaitConfirmation);
    let failure = LoadFailure::new("Could not load", "fixture").retryable(true);
    let mut failed = snapshot(&active, BarrierReadiness::Failed);
    failed.failed_steps = 1;
    failed.failures = vec![failure.clone()];
    failed.estimate = None;
    let model = LoadPresentationModel::from_snapshot(failed, &active);
    assert_eq!(model.failures, vec![failure]);
}

#[test]
fn arbitrary_activity_identity_is_data() {
    let mut first = LoadExperienceSpec::basic("first");
    first.activity = Some(LoadActivityId::new("platformer-practice"));
    let mut second = LoadExperienceSpec::basic("second");
    second.activity = Some(LoadActivityId::new("rhythm-toy"));
    assert_ne!(first.activity, second.activity);
    assert_eq!(first.ready_policy, second.ready_policy);
}

#[test]
fn ready_policies_distinguish_visibility_from_engagement() {
    assert!(!ReadyTransitionPolicy::AutoAdvance.holds_ready(true, true));
    assert!(!ReadyTransitionPolicy::AwaitConfirmation.holds_ready(false, false));
    assert!(ReadyTransitionPolicy::AwaitConfirmation.holds_ready(true, false));
    assert!(!ReadyTransitionPolicy::AutoUnlessEngaged.holds_ready(true, false));
    assert!(ReadyTransitionPolicy::AutoUnlessEngaged.holds_ready(true, true));
}

#[test]
fn presentation_can_hide_percentage_without_hiding_exact_facts() {
    let mut active = foreground(ReadyTransitionPolicy::AutoAdvance);
    active.spec.show_estimated_percentage = false;
    let mut pending = snapshot(&active, BarrierReadiness::Preparing);
    pending.completed_steps = 2;
    pending.active_steps = 1;
    pending.known_remaining_steps = 3;
    pending.active_labels = vec!["Decode".to_owned()];
    pending.remaining_labels = vec!["Build".to_owned()];
    pending.streamable_labels = vec!["Warm cache".to_owned()];
    pending.speculative_labels = vec!["Preload sequel".to_owned()];
    pending.estimate.as_mut().unwrap().fraction = 0.4;
    let model = LoadPresentationModel::from_snapshot(pending, &active);
    assert!(model.estimate.is_none());
    assert_eq!(
        (
            model.completed_steps,
            model.active_steps,
            model.known_remaining_steps
        ),
        (2, 1, 3)
    );
}

#[test]
fn generic_presentation_runs_without_shell_resources() {
    use ambition_load::{LoadBarrierSpec, LoadCommand, LoadPlanSpec, LoadWorkSpec, LoadWorkState};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins((
        ambition_load::AmbitionLoadPlugin,
        AmbitionLoadPresentationPlugin,
    ));
    app.insert_resource(Time::<()>::default());

    let load = LoadId::new("room-transition");
    let barrier = LoadBarrierId::new("target-ready");
    let owner = LoadPresentationOwnerId::new("room-transition:7");
    {
        let mut loads = app
            .world_mut()
            .resource_mut::<ambition_load::LoadCoordinator>();
        loads.apply(LoadCommand::Begin(LoadPlanSpec::new(
            load.clone(),
            "Prepare target room",
        )));
        loads.apply(LoadCommand::DeclareBarrier {
            load_id: load.clone(),
            spec: LoadBarrierSpec::new(barrier.clone(), "Preparing target room"),
        });
        loads.apply(LoadCommand::UpsertWork {
            load_id: load.clone(),
            spec: LoadWorkSpec::required("geometry", "Prepare geometry", barrier.clone()),
        });
        loads.apply(LoadCommand::SetWorkState {
            load_id: load.clone(),
            work_id: ambition_load::LoadWorkId::new("geometry"),
            state: LoadWorkState::Running { progress: None },
        });
    }
    app.world_mut()
        .write_message(LoadPresentationCommand::Begin {
            owner: owner.clone(),
            barrier: LoadBarrierRef::new(load.clone(), barrier.clone()),
            spec: LoadExperienceSpec {
                id: LoadExperienceId::new("room-transition"),
                reveal_after: Duration::from_millis(100),
                ready_policy: ReadyTransitionPolicy::AwaitConfirmation,
                activity: None,
                show_estimated_percentage: true,
            input_owner: Some(ambition_input::InputOwner::AllParticipants),
            },
        });
    app.update();
    assert!(!(&only_presentation(&app).model).visible);

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(125));
    app.update();
    assert!((&only_presentation(&app).model).visible);

    {
        let mut loads = app
            .world_mut()
            .resource_mut::<ambition_load::LoadCoordinator>();
        loads.apply(LoadCommand::SetWorkState {
            load_id: load.clone(),
            work_id: ambition_load::LoadWorkId::new("geometry"),
            state: LoadWorkState::Complete,
        });
        loads.apply(LoadCommand::SetDiscovery {
            load_id: load.clone(),
            barrier_id: barrier.clone(),
            open: false,
            forecast: None,
        });
    }
    app.update();
    assert!((&only_presentation(&app).model).ready_hold);

    app.world_mut()
        .write_message(LoadPresentationAction::Continue {
            owner: owner.clone(),
        });
    app.update();
    assert!(!(&only_presentation(&app).model).ready_hold);

    app.world_mut()
        .write_message(LoadPresentationCommand::Finish { owner });
    app.update();
    assert!(app.world().resource::<LoadPresentations>().is_empty());
}

#[test]
fn generic_failure_routes_actions_to_the_exact_owner_and_cleans_up() {
    use ambition_load::{LoadBarrierSpec, LoadCommand, LoadPlanSpec, LoadWorkSpec, LoadWorkState};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins((
        ambition_load::AmbitionLoadPlugin,
        AmbitionLoadPresentationPlugin,
    ));
    app.insert_resource(Time::<()>::default());

    let load = LoadId::new("room-transition-failure");
    let barrier = LoadBarrierId::new("target-ready");
    let owner = LoadPresentationOwnerId::new("room-transition:failure");
    let failure = LoadFailure::new("Could not prepare room", "fixture failure").retryable(true);
    {
        let mut loads = app
            .world_mut()
            .resource_mut::<ambition_load::LoadCoordinator>();
        loads.apply(LoadCommand::Begin(LoadPlanSpec::new(
            load.clone(),
            "Prepare target room",
        )));
        loads.apply(LoadCommand::DeclareBarrier {
            load_id: load.clone(),
            spec: LoadBarrierSpec::new(barrier.clone(), "Preparing target room"),
        });
        loads.apply(LoadCommand::UpsertWork {
            load_id: load.clone(),
            spec: LoadWorkSpec::required("geometry", "Prepare geometry", barrier.clone()),
        });
        loads.apply(LoadCommand::SetWorkState {
            load_id: load.clone(),
            work_id: ambition_load::LoadWorkId::new("geometry"),
            state: LoadWorkState::Failed(failure.clone()),
        });
    }
    app.world_mut()
        .write_message(LoadPresentationCommand::Begin {
            owner: owner.clone(),
            barrier: LoadBarrierRef::new(load, barrier.clone()),
            spec: LoadExperienceSpec {
                id: LoadExperienceId::new("room-transition"),
                reveal_after: Duration::ZERO,
                ready_policy: ReadyTransitionPolicy::AutoAdvance,
                activity: None,
                show_estimated_percentage: true,
            input_owner: Some(ambition_input::InputOwner::AllParticipants),
            },
        });
    app.update();

    let model = &only_presentation(&app).model;
    assert!(model.visible);
    assert_eq!(model.readiness, Some(BarrierReadiness::Failed));
    assert_eq!(model.failures, vec![failure]);

    app.world_mut()
        .write_message(LoadPresentationAction::Retry {
            owner: owner.clone(),
        });
    app.update();
    let events = app
        .world_mut()
        .resource_mut::<Messages<LoadPresentationEvent>>()
        .drain()
        .collect::<Vec<_>>();
    assert!(events.iter().any(|event| matches!(
        event,
        LoadPresentationEvent::RetryRequested {
            owner: event_owner,
            barrier: event_barrier,
        } if event_owner == &owner && event_barrier.barrier_id == barrier
    )));

    app.world_mut()
        .write_message(LoadPresentationCommand::Cancel { owner });
    app.update();
    assert!(app.world().resource::<LoadPresentations>().is_empty());
}

fn composed_app(
    ready_policy: ReadyTransitionPolicy,
    activity: Option<LoadActivityId>,
    reveal_after: Duration,
) -> (
    bevy::prelude::App,
    ambition_load::LoadId,
    ambition_load::LoadBarrierId,
) {
    use ambition_game_shell::{
        ShellHostConfiguration, ShellHostSpec, ShellRouteCatalog, ShellRouteSpec,
    };
    use ambition_load::{LoadBarrierSpec, LoadCommand, LoadPlanSpec};
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalLoadShellPlugins);
    app.insert_resource(Time::<()>::default());

    let load = ambition_load::LoadId::new("fixture-load");
    let barrier = ambition_load::LoadBarrierId::new("fixture-ready");
    {
        let mut loads = app
            .world_mut()
            .resource_mut::<ambition_load::LoadCoordinator>();
        loads.apply(LoadCommand::Begin(LoadPlanSpec::new(
            load.clone(),
            "Fixture load",
        )));
        loads.apply(LoadCommand::DeclareBarrier {
            load_id: load.clone(),
            spec: LoadBarrierSpec::new(barrier.clone(), "Fixture ready"),
        });
    }
    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(
            ShellRouteSpec::new("game", "fixture-game").requiring(load.clone(), barrier.clone()),
        );
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new("game", "game"));
    app.world_mut()
        .resource_mut::<ShellLoadPresentationCatalog>()
        .by_route
        .insert(
            ambition_game_shell::ShellRouteId::new("game"),
            LoadExperienceSpec {
                id: LoadExperienceId::new("fixture-presentation"),
                reveal_after,
                ready_policy,
                activity,
                show_estimated_percentage: true,
            input_owner: Some(ambition_input::InputOwner::AllParticipants),
            },
        );
    (app, load, barrier)
}

#[test]
fn composed_fast_load_activates_without_visible_foreground() {
    use ambition_game_shell::ShellRouter;
    use ambition_load::LoadCommand;

    let (mut app, load, barrier) = composed_app(
        ReadyTransitionPolicy::AutoAdvance,
        None,
        Duration::from_millis(250),
    );
    app.update();
    assert!(app.world().resource::<ShellRouter>().pending.is_some());

    app.world_mut()
        .resource_mut::<ambition_load::LoadCoordinator>()
        .apply(LoadCommand::SetDiscovery {
            load_id: load,
            barrier_id: barrier,
            open: false,
            forecast: None,
        });
    app.update();

    let router = app.world().resource::<ShellRouter>();
    assert!(router.pending.is_none());
    assert_eq!(
        router
            .active
            .as_ref()
            .map(|active| active.route_id.as_str()),
        Some("game"),
    );
    assert!(app.world().resource::<LoadPresentations>().is_empty());

    // In play the pass sleeps: nothing rewrites the empty model each frame.
    let last_changed = |app: &bevy::prelude::App| {
        app.world()
            .get_resource_change_ticks::<LoadPresentations>()
            .expect("the presentations have one owner resource")
            .changed
    };
    // The last commands outlive their frame by one, and the pass runs one
    // frame after them.
    for _ in 0..3 {
        app.update();
    }
    let settled = last_changed(&app);
    app.update();
    app.update();
    assert_eq!(
        last_changed(&app),
        settled,
        "an idle load presentation writes nothing",
    );
}

#[test]
fn engaged_activity_holds_ready_until_continue_and_cleans_scope() {
    use ambition_game_shell::ShellRouter;
    use ambition_load::LoadCommand;

    let activity_id = LoadActivityId::new("fixture-activity");
    let (mut app, load, barrier) = composed_app(
        ReadyTransitionPolicy::AutoUnlessEngaged,
        Some(activity_id.clone()),
        Duration::ZERO,
    );
    app.update();

    let activation_id = app.world().resource::<LoadPresentations>().iter().find_map(|entry| entry.activity.as_ref())
        .expect("zero-grace activity starts while load is pending")
        .activation_id;
    let owner = app.world().resource::<LoadPresentations>().iter().next().map(|entry| &entry.foreground)
        .unwrap()
        .owner
        .clone();
    let scoped = app
        .world_mut()
        .spawn(LoadActivityScopedEntity { activation_id })
        .id();

    app.world_mut()
        .write_message(LoadActivitySignal::Engaged { activation_id });
    app.world_mut()
        .resource_mut::<ambition_load::LoadCoordinator>()
        .apply(LoadCommand::SetDiscovery {
            load_id: load,
            barrier_id: barrier,
            open: false,
            forecast: None,
        });
    app.update();

    assert!(app.world().resource::<ShellRouter>().active.is_none());
    assert_eq!(
        app.world().resource::<LoadPresentations>().iter().next().map(|entry| &entry.foreground)
            .map(|active| active.phase),
        Some(LoadForegroundPhase::ReadyHold),
    );
    assert!(app.world().get_entity(scoped).is_ok());

    app.world_mut()
        .write_message(LoadPresentationAction::Continue { owner });
    app.update();

    assert_eq!(
        app.world()
            .resource::<ShellRouter>()
            .active
            .as_ref()
            .map(|active| active.route_id.as_str()),
        Some("game"),
    );
    assert!(app.world().get_entity(scoped).is_err());
}

#[test]
fn slow_required_work_reveals_exact_facts() {
    use ambition_load::{LoadCommand, LoadWorkSpec, LoadWorkState};
    use bevy::prelude::Time;

    let (mut app, load, barrier) = composed_app(
        ReadyTransitionPolicy::AutoAdvance,
        None,
        Duration::from_millis(200),
    );
    {
        let mut loads = app
            .world_mut()
            .resource_mut::<ambition_load::LoadCoordinator>();
        for (id, label) in [
            ("geometry", "Decode geometry"),
            ("entities", "Build entities"),
        ] {
            loads.apply(LoadCommand::UpsertWork {
                load_id: load.clone(),
                spec: LoadWorkSpec::required(id, label, barrier.clone()),
            });
        }
        loads.apply(LoadCommand::SetWorkState {
            load_id: load.clone(),
            work_id: ambition_load::LoadWorkId::new("geometry"),
            state: LoadWorkState::Complete,
        });
        loads.apply(LoadCommand::SetWorkState {
            load_id: load.clone(),
            work_id: ambition_load::LoadWorkId::new("entities"),
            state: LoadWorkState::Running { progress: None },
        });
    }

    app.update();
    assert!(!(&only_presentation(&app).model).visible);

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(250));
    app.update();
    let model = &only_presentation(&app).model;
    assert!(model.visible);
    assert_eq!(
        (
            model.completed_steps,
            model.active_steps,
            model.known_remaining_steps
        ),
        (1, 1, 1),
    );
    assert_eq!(model.stage, "Fixture ready");
}

/// The loading screen shows it is alive: its spinner moves as real time
/// passes, so a slow load never reads as a frozen game.
#[cfg(feature = "basic_presentation")]
#[test]
fn the_loading_spinner_turns_with_time() {
    use crate::spinner_frame;
    let frames: std::collections::BTreeSet<&str> = (0..8).map(|i| spinner_frame(i as f32 / 8.0)).collect();
    assert_eq!(frames.len(), 4, "every frame of the spinner is reached within half a second: {frames:?}");
    assert_ne!(spinner_frame(0.0), spinner_frame(0.125), "it moves eight times a second");
}

#[cfg(feature = "basic_presentation")]
#[test]
fn two_loading_owners_keep_their_roots_and_release_only_their_own_input() {
    use ambition_load::{LoadBarrierSpec, LoadCommand, LoadPlanSpec};
    use ambition_input::participant::*;
    use bevy::prelude::*;
    let mut app = App::new();
    app.add_plugins((ambition_load::AmbitionLoadPlugin, AmbitionLoadPresentationPlugin,
        BasicLoadPresentationPlugin));
    for slot in [0, 1] {
        app.world_mut().spawn((InputParticipant::with_id(ParticipantId(slot)), ParticipantContexts::default()));
        let load = format!("room-{slot}");
        let mut loads = app.world_mut().resource_mut::<ambition_load::LoadCoordinator>();
        loads.apply(LoadCommand::Begin(LoadPlanSpec::new(load.clone(), load.clone())));
        loads.apply(LoadCommand::DeclareBarrier { load_id: load.clone().into(),
            spec: LoadBarrierSpec::new("ready", "Prepare room") });
        let mut spec = LoadExperienceSpec::basic("fixture");
        spec.reveal_after = Duration::ZERO;
        spec.activity = Some(LoadActivityId::new("fixture-activity"));
        spec.input_owner = Some(InputOwner::Participant(ParticipantId(slot)));
        app.world_mut().write_message(LoadPresentationCommand::Begin {
            owner: LoadPresentationOwnerId::new(load.clone()), barrier: LoadBarrierRef::new(load, "ready"), spec });
    }
    app.update();
    let world = app.world_mut();
    assert_eq!(world.query_filtered::<Entity, With<BasicLoadRoot>>().iter(world).count(), 2);
    for (_, contexts) in world.query::<(&InputParticipant, &ParticipantContexts)>().iter(world) {
        assert!(contexts.is_declared(LOADING_CONTEXT));
    }
    let activations = [0, 1].map(|slot| world.resource::<LoadPresentations>()
        .get(&LoadPresentationOwnerId::new(format!("room-{slot}"))).unwrap().activity.as_ref().unwrap().activation_id);
    let scoped = activations.map(|activation_id| world.spawn(LoadActivityScopedEntity { activation_id }).id());
    world.write_message(LoadActivitySignal::Engaged { activation_id: activations[1] });
    world.write_message(LoadActivitySignal::Finished { activation_id: activations[0], outcome: LoadActivityOutcome::default() });
    app.update();
    let world = app.world_mut();
    assert!(world.get_entity(scoped[0]).is_err());
    assert!(world.get_entity(scoped[1]).is_ok());
    let entries = world.resource::<LoadPresentations>();
    assert!(entries.get(&LoadPresentationOwnerId::new("room-0")).unwrap().last_outcome.is_some());
    assert!(entries.get(&LoadPresentationOwnerId::new("room-1")).unwrap().foreground.engaged);
    world.write_message(LoadPresentationCommand::Finish { owner: LoadPresentationOwnerId::new("room-0") });
    app.update();
    let world = app.world_mut();
    let roots: Vec<_> = world.query_filtered::<&ambition_game_shell::FrontendOwnedEntity, With<BasicLoadRoot>>()
        .iter(world).collect();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].owner, ambition_game_shell::FrontendEntityOwner::Load("room-1".into()));
    for (participant, contexts) in world.query::<(&InputParticipant, &ParticipantContexts)>().iter(world) {
        assert_eq!(contexts.is_declared(LOADING_CONTEXT), participant.id == ParticipantId(1));
    }    world.write_message(LoadPresentationCommand::Finish { owner: LoadPresentationOwnerId::new("room-1") });
    let mut spec = LoadExperienceSpec::basic("fixture");
    spec.reveal_after = Duration::ZERO;
    spec.activity = Some(LoadActivityId::new("fixture-activity"));
    spec.input_owner = Some(InputOwner::Participant(ParticipantId(1)));
    world.write_message(LoadPresentationCommand::Begin { owner: LoadPresentationOwnerId::new("room-1"),
        barrier: LoadBarrierRef::new("room-1", "ready"), spec });
    app.update();
    let world = app.world();
    assert!(world.get_entity(scoped[1]).is_err());
    let restarted = world.resource::<LoadPresentations>().get(&LoadPresentationOwnerId::new("room-1")).unwrap();
    assert!(restarted.activity.as_ref().unwrap().activation_id > activations[1]);
    assert!(!restarted.foreground.engaged);

}
