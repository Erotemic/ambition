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

/// ⛔⛔ NOTHING IN THIS TREE READS `LoadEvent`. Measured 2026-09-06: zero
/// `MessageReader<LoadEvent>` anywhere, and the only in-tree writers of a variant
/// are two `CommitAuthorized`/`CommitRejected` sites in the runtime. The channel
/// exists for the EXTERNAL consumers this crate is composed by — which is exactly
/// why its emissions need a test HERE. Deleting every `PlanChanged` push left all
/// 13 tests of this crate green.
///
/// So: a command that changes a plan announces it, and one that changes nothing
/// stays quiet. A listener that re-derives a snapshot per `PlanChanged` should
/// not be woken by a `RemoveWork` for an id that was never there.
#[test]
fn a_command_that_changes_a_plan_announces_it_and_one_that_does_not_stays_quiet() {
    let (mut coordinator, load, barrier) = setup();
    let work = LoadWorkId::new("shader");

    let changed = |events: &[LoadEvent]| {
        events
            .iter()
            .filter(|event| matches!(event, LoadEvent::PlanChanged { .. }))
            .count()
    };

    // Every mutating command on an active plan: each must announce exactly once.
    for (label, command) in [
        (
            "upsert",
            LoadCommand::UpsertWork {
                load_id: load.clone(),
                spec: LoadWorkSpec::required("shader", "Shader", barrier.clone()),
            },
        ),
        (
            "set state",
            LoadCommand::SetWorkState {
                load_id: load.clone(),
                work_id: work.clone(),
                state: LoadWorkState::Running { progress: None },
            },
        ),
        (
            "set priority",
            LoadCommand::SetWorkPriority {
                load_id: load.clone(),
                work_id: work.clone(),
                priority: LoadPriority::High,
            },
        ),
        (
            "promote",
            LoadCommand::PromoteWork {
                load_id: load.clone(),
                work_id: work.clone(),
                barrier_id: barrier.clone(),
            },
        ),
        (
            "discovery",
            LoadCommand::SetDiscovery {
                load_id: load.clone(),
                barrier_id: barrier.clone(),
                open: true,
                forecast: None,
            },
        ),
        (
            "remove",
            LoadCommand::RemoveWork {
                load_id: load.clone(),
                work_id: work.clone(),
            },
        ),
    ] {
        let events = coordinator.apply(command);
        assert_eq!(
            changed(&events),
            1,
            "`{label}` changed the plan and must announce it once, got {events:?}"
        );
    }

    // The same removal again finds the plan and changes nothing.
    let events = coordinator.apply(LoadCommand::RemoveWork {
        load_id: load.clone(),
        work_id: work,
    });
    assert_eq!(
        changed(&events),
        0,
        "removing work that is already gone changed nothing, got {events:?}"
    );

    // And a command naming a plan that does not exist reaches no plan at all.
    let events = coordinator.apply(LoadCommand::RemoveWork {
        load_id: LoadId::new("no-such-load"),
        work_id: LoadWorkId::new("shader"),
    });
    assert_eq!(
        changed(&events),
        0,
        "an unknown load has no plan to change, got {events:?}"
    );
}
