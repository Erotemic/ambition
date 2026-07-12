use std::time::Duration;

use ambition_load::{LoadBarrierSpec, LoadCommand, LoadCoordinator, LoadPlanSpec};

use crate::*;

#[test]
fn initial_and_home_routes_are_independent() {
    let mut loads = LoadCoordinator::default();
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("direct-game", "game"));
    catalog.register(ShellRouteSpec::new("demo-home", "launcher"));
    let host = ShellHostConfiguration {
        spec: Some(ShellHostSpec::new("direct-game", "demo-home")),
    };
    let mut router = ShellRouter::default();
    let events = router.apply(ShellCommand::Initialize, &catalog, &host, &mut loads);
    assert!(
        matches!(events.last(), Some(ShellEvent::RouteActivated(active)) if active.route_id.as_str() == "direct-game")
    );
    let events = router.apply(ShellCommand::QuitToHome, &catalog, &host, &mut loads);
    assert!(
        matches!(events.last(), Some(ShellEvent::RouteActivated(active)) if active.route_id.as_str() == "demo-home")
    );
}

#[test]
fn route_waits_for_its_load_barrier() {
    let mut loads = LoadCoordinator::default();
    let load = ambition_load::LoadId::new("game-load");
    let barrier = ambition_load::LoadBarrierId::new("ready");
    loads.apply(LoadCommand::Begin(LoadPlanSpec::new(load.clone(), "Game")));
    loads.apply(LoadCommand::DeclareBarrier {
        load_id: load.clone(),
        spec: LoadBarrierSpec::new(barrier.clone(), "Ready"),
    });
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("game", "game").requiring(load.clone(), barrier.clone()));
    let mut router = ShellRouter::default();
    let host = ShellHostConfiguration::default();
    let events = router.apply(
        ShellCommand::GoTo(ShellRouteId::new("game")),
        &catalog,
        &host,
        &mut loads,
    );
    assert!(matches!(
        events.as_slice(),
        [ShellEvent::WaitingForLoad { .. }]
    ));
    assert!(router.active.is_none());

    loads.apply(LoadCommand::SetDiscovery {
        load_id: load,
        barrier_id: barrier,
        open: false,
        forecast: None,
    });
    let holds = ShellRouteHolds::default();
    let events = router.advance_pending(&catalog, &mut loads, &holds);
    assert!(matches!(events.last(), Some(ShellEvent::RouteActivated(_))));
    assert_eq!(
        loads.request_commit(
            &ambition_load::LoadId::new("game-load"),
            &ambition_load::LoadBarrierId::new("ready"),
        ),
        Err(ambition_load::LoadCommitRejection::AlreadyAuthorized),
    );
}

#[test]
fn completion_policy_routes_without_experience_knowing_target() {
    let mut loads = LoadCoordinator::default();
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(
        ShellRouteSpec::new("credits", "credits").on_complete(ShellCompletionPolicy::ReturnHome),
    );
    catalog.register(ShellRouteSpec::new("home", "launcher"));
    let host = ShellHostConfiguration {
        spec: Some(ShellHostSpec::new("credits", "home")),
    };
    let mut router = ShellRouter::default();
    router.apply(ShellCommand::Initialize, &catalog, &host, &mut loads);
    let activation_id = router.active.as_ref().unwrap().activation_id;
    let events = router.apply(
        ShellCommand::ExperienceCompleted { activation_id },
        &catalog,
        &host,
        &mut loads,
    );
    assert!(
        matches!(events.last(), Some(ShellEvent::RouteActivated(active)) if active.route_id.as_str() == "home")
    );
}

#[test]
fn route_hold_delays_commit_and_activation() {
    let mut loads = LoadCoordinator::default();
    let load = ambition_load::LoadId::new("held-load");
    let barrier = ambition_load::LoadBarrierId::new("held-ready");
    loads.apply(LoadCommand::Begin(LoadPlanSpec::new(load.clone(), "Held")));
    loads.apply(LoadCommand::DeclareBarrier {
        load_id: load.clone(),
        spec: LoadBarrierSpec::new(barrier.clone(), "Held ready"),
    });
    loads.apply(LoadCommand::SetDiscovery {
        load_id: load.clone(),
        barrier_id: barrier.clone(),
        open: false,
        forecast: None,
    });

    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("held", "game").requiring(load.clone(), barrier.clone()));
    let mut router = ShellRouter::default();
    let host = ShellHostConfiguration::default();
    let mut holds = ShellRouteHolds::default();
    holds.hold(ShellRouteId::new("held"), ShellHoldId::new("test-hold"));

    // A ready route commits immediately when first requested, so install the
    // hold through a pending route before readiness in real composition. This
    // direct hold test exercises the pending advance seam explicitly.
    loads.apply(LoadCommand::SetDiscovery {
        load_id: load.clone(),
        barrier_id: barrier.clone(),
        open: true,
        forecast: None,
    });
    router.apply(
        ShellCommand::GoTo(ShellRouteId::new("held")),
        &catalog,
        &host,
        &mut loads,
    );
    loads.apply(LoadCommand::SetDiscovery {
        load_id: load.clone(),
        barrier_id: barrier.clone(),
        open: false,
        forecast: None,
    });
    assert!(router
        .advance_pending(&catalog, &mut loads, &holds)
        .is_empty());
    assert!(router.active.is_none());

    holds.release(&ShellRouteId::new("held"), &ShellHoldId::new("test-hold"));
    assert!(matches!(
        router.advance_pending(&catalog, &mut loads, &holds).last(),
        Some(ShellEvent::RouteActivated(_))
    ));
}

#[test]
fn sequence_handles_text_and_programmatic_segments() {
    let custom = ShellSegmentKindId::new("custom-bevy-card");
    let mut runtime = ShellSequenceRuntime::new(ShellSequenceSpec {
        segments: vec![
            ShellSegmentSpec::text("text", "Powered by Ambition"),
            ShellSegmentSpec {
                id: ShellSegmentId::new("program"),
                role: ShellSegmentRole::TitleReveal,
                presentation: ShellSegmentPresentation::Registered(custom.clone()),
                policy: ShellSegmentPolicy {
                    auto_advance_after: None,
                    skip_policy: ShellSkipPolicy::Never,
                    requires_acknowledgement: false,
                },
            },
        ],
    });
    assert!(!runtime.tick(Duration::from_secs(1)));
    assert!(!runtime.tick(Duration::from_secs(1)));
    assert!(
        matches!(runtime.current().map(|item| &item.presentation), Some(ShellSegmentPresentation::Registered(id)) if id == &custom)
    );
    assert!(runtime.complete_programmatic_segment());
    assert!(runtime.finished);
}
