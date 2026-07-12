use std::collections::BTreeSet;

use crate::*;

fn setup() -> (LoadCoordinator, LoadId, LoadBarrierId) {
    let mut coordinator = LoadCoordinator::default();
    let load = LoadId::new("session");
    let barrier = LoadBarrierId::new("session-ready");
    coordinator.apply(LoadCommand::Begin(LoadPlanSpec::new(
        load.clone(),
        "Session",
    )));
    coordinator.apply(LoadCommand::DeclareBarrier {
        load_id: load.clone(),
        spec: LoadBarrierSpec::new(barrier.clone(), "Session ready"),
    });
    (coordinator, load, barrier)
}

#[test]
fn exact_counts_and_discovery_control_readiness() {
    let (mut coordinator, load, barrier) = setup();
    for (id, state) in [
        ("done", LoadWorkState::Complete),
        ("active", LoadWorkState::Running { progress: None }),
        ("planned", LoadWorkState::Planned),
    ] {
        coordinator.apply(LoadCommand::UpsertWork {
            load_id: load.clone(),
            spec: LoadWorkSpec::required(id, id, barrier.clone()),
        });
        coordinator.apply(LoadCommand::SetWorkState {
            load_id: load.clone(),
            work_id: LoadWorkId::new(id),
            state,
        });
    }
    let snapshot = coordinator.snapshot(&load, &barrier).unwrap();
    assert_eq!(
        (
            snapshot.completed_steps,
            snapshot.active_steps,
            snapshot.known_remaining_steps
        ),
        (1, 1, 2)
    );
    assert!(!snapshot.ready());

    for id in ["active", "planned"] {
        coordinator.apply(LoadCommand::SetWorkState {
            load_id: load.clone(),
            work_id: LoadWorkId::new(id),
            state: LoadWorkState::Complete,
        });
    }
    assert!(!coordinator.snapshot(&load, &barrier).unwrap().ready());
    coordinator.apply(LoadCommand::SetDiscovery {
        load_id: load.clone(),
        barrier_id: barrier.clone(),
        open: false,
        forecast: None,
    });
    assert!(coordinator.snapshot(&load, &barrier).unwrap().ready());
}

#[test]
fn streamable_work_does_not_block_until_promoted_and_keeps_progress() {
    let (mut coordinator, load, barrier) = setup();
    let work_id = LoadWorkId::new("distant-art");
    coordinator.apply(LoadCommand::UpsertWork {
        load_id: load.clone(),
        spec: LoadWorkSpec::streamable(work_id.clone(), "Distant art"),
    });
    coordinator.apply(LoadCommand::SetWorkState {
        load_id: load.clone(),
        work_id: work_id.clone(),
        state: LoadWorkState::Running {
            progress: Some(UnitProgress::new(3.0, 4.0)),
        },
    });
    coordinator.apply(LoadCommand::SetDiscovery {
        load_id: load.clone(),
        barrier_id: barrier.clone(),
        open: false,
        forecast: None,
    });
    assert!(coordinator.snapshot(&load, &barrier).unwrap().ready());

    coordinator.apply(LoadCommand::PromoteWork {
        load_id: load.clone(),
        work_id: work_id.clone(),
        barrier_id: barrier.clone(),
    });
    let snapshot = coordinator.snapshot(&load, &barrier).unwrap();
    assert!(!snapshot.ready());
    assert_eq!(snapshot.active_steps, 1);
    assert_eq!(snapshot.estimate.unwrap().fraction, 0.75);
}

#[test]
fn forecasts_keep_facts_and_estimates_separate() {
    let (mut coordinator, load, barrier) = setup();
    coordinator.apply(LoadCommand::UpsertWork {
        load_id: load.clone(),
        spec: LoadWorkSpec::required("known", "Known", barrier.clone()).with_weight(2.0),
    });
    coordinator.apply(LoadCommand::SetWorkState {
        load_id: load.clone(),
        work_id: LoadWorkId::new("known"),
        state: LoadWorkState::Complete,
    });
    let mut forecast = DiscoveryForecast::new("authored region fanout");
    forecast.additional_steps = Some(2..=6);
    forecast.additional_weight = Some(2.0);
    forecast.confidence = EstimateConfidence::Medium;
    coordinator.apply(LoadCommand::SetDiscovery {
        load_id: load.clone(),
        barrier_id: barrier.clone(),
        open: true,
        forecast: Some(forecast),
    });
    let snapshot = coordinator.snapshot(&load, &barrier).unwrap();
    assert_eq!(snapshot.known_remaining_steps, 0);
    assert_eq!(snapshot.estimated_total_remaining_steps, Some(2..=6));
    let estimate = snapshot.estimate.unwrap();
    assert_eq!(estimate.fraction, 0.5);
    assert!(estimate.may_decrease);
}

#[test]
fn superseded_load_cannot_authorize_commit() {
    let (mut coordinator, old, barrier) = setup();
    coordinator.apply(LoadCommand::SetDiscovery {
        load_id: old.clone(),
        barrier_id: barrier.clone(),
        open: false,
        forecast: None,
    });
    let mut replacement = LoadPlanSpec::new("replacement", "Replacement");
    replacement.supersedes = Some(old.clone());
    coordinator.apply(LoadCommand::Begin(replacement));
    let events = coordinator.apply(LoadCommand::RequestCommit {
        load_id: old,
        barrier_id: barrier,
    });
    assert!(matches!(
        events.as_slice(),
        [LoadEvent::CommitRejected {
            reason: LoadCommitRejection::BarrierNotReady(BarrierReadiness::Superseded),
            ..
        }]
    ));
}

#[test]
fn requirement_can_name_multiple_barriers() {
    let mut requirement =
        ActivationRequirement::RequiredFor(BTreeSet::from([LoadBarrierId::new("a")]));
    requirement.add_barrier(LoadBarrierId::new("b"));
    assert_eq!(requirement.barriers().count(), 2);
}

#[test]
fn removed_work_leaves_no_barrier_debt() {
    let (mut coordinator, load, barrier) = setup();
    coordinator.apply(LoadCommand::UpsertWork {
        load_id: load.clone(),
        spec: LoadWorkSpec::required("temporary", "Temporary", barrier.clone()),
    });
    coordinator.apply(LoadCommand::RemoveWork {
        load_id: load.clone(),
        work_id: LoadWorkId::new("temporary"),
    });
    coordinator.apply(LoadCommand::SetDiscovery {
        load_id: load.clone(),
        barrier_id: barrier.clone(),
        open: false,
        forecast: None,
    });
    let snapshot = coordinator.snapshot(&load, &barrier).unwrap();
    assert_eq!(snapshot.known_remaining_steps, 0);
    assert!(snapshot.ready());
}

#[test]
fn commit_authorization_is_one_shot() {
    let (mut coordinator, load, barrier) = setup();
    coordinator.apply(LoadCommand::SetDiscovery {
        load_id: load.clone(),
        barrier_id: barrier.clone(),
        open: false,
        forecast: None,
    });
    assert_eq!(coordinator.request_commit(&load, &barrier), Ok(()));
    assert_eq!(
        coordinator.request_commit(&load, &barrier),
        Err(LoadCommitRejection::AlreadyAuthorized),
    );
}

#[test]
fn cancelled_load_cannot_authorize_commit() {
    let (mut coordinator, load, barrier) = setup();
    coordinator.apply(LoadCommand::SetDiscovery {
        load_id: load.clone(),
        barrier_id: barrier.clone(),
        open: false,
        forecast: None,
    });
    coordinator.apply(LoadCommand::Cancel {
        load_id: load.clone(),
    });
    assert_eq!(
        coordinator.request_commit(&load, &barrier),
        Err(LoadCommitRejection::BarrierNotReady(
            BarrierReadiness::Cancelled,
        )),
    );
}

#[test]
fn late_completion_is_ignored_after_cancellation() {
    let (mut coordinator, load, barrier) = setup();
    coordinator.apply(LoadCommand::UpsertWork {
        load_id: load.clone(),
        spec: LoadWorkSpec::required("late", "Late result", barrier.clone()),
    });
    coordinator.apply(LoadCommand::Cancel {
        load_id: load.clone(),
    });
    coordinator.apply(LoadCommand::SetWorkState {
        load_id: load.clone(),
        work_id: LoadWorkId::new("late"),
        state: LoadWorkState::Complete,
    });

    let snapshot = coordinator.snapshot(&load, &barrier).unwrap();
    assert_eq!(snapshot.readiness, BarrierReadiness::Cancelled);
    assert_eq!(snapshot.completed_steps, 0);
    assert_eq!(snapshot.known_remaining_steps, 1);
}
