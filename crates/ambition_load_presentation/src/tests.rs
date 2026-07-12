use std::time::Duration;

use ambition_game_shell::{LoadBarrierRef, ShellRouteId};
use ambition_load::{BarrierReadiness, LoadBarrierId, LoadBarrierSnapshot, LoadFailure, LoadId};

use crate::*;

fn foreground(policy: ReadyTransitionPolicy) -> ActiveLoadForeground {
    ActiveLoadForeground {
        route_id: ShellRouteId::new("game"),
        barrier: LoadBarrierRef {
            load_id: LoadId::new("load"),
            barrier_id: LoadBarrierId::new("ready"),
        },
        spec: LoadExperienceSpec {
            id: LoadExperienceId::new("basic"),
            reveal_after: Duration::from_millis(250),
            ready_policy: policy,
            activity: None,
            show_estimated_percentage: true,
        },
        phase: LoadForegroundPhase::Visible,
        elapsed: Duration::from_secs(1),
        activity_activation_id: None,
        engaged: false,
    }
}

#[test]
fn model_never_reports_pre_ready_one_hundred_percent() {
    let active = foreground(ReadyTransitionPolicy::AutoAdvance);
    let snapshot = LoadBarrierSnapshot {
        load_id: active.barrier.load_id.clone(),
        barrier_id: active.barrier.barrier_id.clone(),
        label: "Preparing".to_owned(),
        readiness: BarrierReadiness::Preparing,
        discovery_open: false,
        completed_steps: 1,
        active_steps: 0,
        known_remaining_steps: 0,
        failed_steps: 0,
        cancelled_steps: 0,
        estimated_additional_steps: None,
        estimated_total_remaining_steps: None,
        active_labels: Vec::new(),
        remaining_labels: Vec::new(),
        failures: Vec::new(),
        estimate: Some(ambition_load::ProgressEstimate {
            fraction: 1.0,
            confidence: ambition_load::EstimateConfidence::High,
            basis: ambition_load::EstimateBasis::EqualSteps,
            provenance: "fixture".to_owned(),
            may_decrease: false,
        }),
    };
    let model = LoadPresentationModel::from_snapshot(active.route_id.clone(), snapshot, &active);
    assert!(model.estimate.unwrap().fraction < 1.0);
}

#[test]
fn failure_evidence_remains_visible() {
    let active = foreground(ReadyTransitionPolicy::AwaitConfirmation);
    let failure = LoadFailure::new("Could not load", "fixture").retryable(true);
    let snapshot = LoadBarrierSnapshot {
        load_id: active.barrier.load_id.clone(),
        barrier_id: active.barrier.barrier_id.clone(),
        label: "Preparing".to_owned(),
        readiness: BarrierReadiness::Failed,
        discovery_open: false,
        completed_steps: 0,
        active_steps: 0,
        known_remaining_steps: 0,
        failed_steps: 1,
        cancelled_steps: 0,
        estimated_additional_steps: None,
        estimated_total_remaining_steps: None,
        active_labels: Vec::new(),
        remaining_labels: Vec::new(),
        failures: vec![failure.clone()],
        estimate: None,
    };
    let model = LoadPresentationModel::from_snapshot(active.route_id.clone(), snapshot, &active);
    assert_eq!(model.failures, vec![failure]);
}

#[test]
fn arbitrary_activity_identity_is_data() {
    let catalog = LoadPresentationCatalog::default();
    let mut first = catalog.default.clone();
    first.activity = Some(LoadActivityId::new("platformer-practice"));
    let mut second = catalog.default.clone();
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
    let snapshot = LoadBarrierSnapshot {
        load_id: active.barrier.load_id.clone(),
        barrier_id: active.barrier.barrier_id.clone(),
        label: "Preparing".to_owned(),
        readiness: BarrierReadiness::Preparing,
        discovery_open: false,
        completed_steps: 2,
        active_steps: 1,
        known_remaining_steps: 3,
        failed_steps: 0,
        cancelled_steps: 0,
        estimated_additional_steps: None,
        estimated_total_remaining_steps: None,
        active_labels: vec!["Decode".to_owned()],
        remaining_labels: vec!["Decode".to_owned()],
        failures: Vec::new(),
        estimate: Some(ambition_load::ProgressEstimate {
            fraction: 0.4,
            confidence: ambition_load::EstimateConfidence::Medium,
            basis: ambition_load::EstimateBasis::EqualSteps,
            provenance: "fixture".to_owned(),
            may_decrease: false,
        }),
    };
    let model = LoadPresentationModel::from_snapshot(active.route_id.clone(), snapshot, &active);
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

fn composed_app(
    ready_policy: ReadyTransitionPolicy,
    activity: Option<LoadActivityId>,
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
        .resource_mut::<LoadPresentationCatalog>()
        .by_route
        .insert(
            ShellRouteId::new("game"),
            LoadExperienceSpec {
                id: LoadExperienceId::new("fixture-presentation"),
                reveal_after: Duration::ZERO,
                ready_policy,
                activity,
                show_estimated_percentage: true,
            },
        );
    (app, load, barrier)
}

#[test]
fn composed_fast_load_activates_without_visible_foreground() {
    use ambition_game_shell::ShellRouter;
    use ambition_load::LoadCommand;

    let (mut app, load, barrier) = composed_app(ReadyTransitionPolicy::AutoAdvance, None);
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
    assert!(app
        .world()
        .resource::<LoadForegroundState>()
        .active
        .is_none());
    assert!(!app.world().resource::<LoadPresentationModel>().visible);
}

#[test]
fn engaged_activity_holds_ready_until_continue_and_cleans_scope() {
    use ambition_game_shell::ShellRouter;
    use ambition_load::LoadCommand;

    let activity_id = LoadActivityId::new("fixture-activity");
    let (mut app, load, barrier) = composed_app(
        ReadyTransitionPolicy::AutoUnlessEngaged,
        Some(activity_id.clone()),
    );
    app.update();

    let activation_id = app
        .world()
        .resource::<LoadActivityState>()
        .active
        .as_ref()
        .expect("zero-grace activity starts while load is pending")
        .activation_id;
    assert_eq!(
        app.world()
            .resource::<LoadActivityState>()
            .active
            .as_ref()
            .map(|active| &active.activity_id),
        Some(&activity_id),
    );
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
        app.world()
            .resource::<LoadForegroundState>()
            .active
            .as_ref()
            .map(|active| active.phase),
        Some(LoadForegroundPhase::ReadyHold),
    );
    assert!(app.world().get_entity(scoped).is_ok());

    app.world_mut()
        .write_message(LoadPresentationAction::Continue);
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
