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

// ── Provider registration, host-relative return, and teardown (App-driven) ──────

mod composed {
    use bevy::prelude::{App, Component};

    use crate::{
        ExperienceRegistration, MinimalShellPlugins, ShellCommand, ShellExperienceRegistry,
        ShellHostConfiguration, ShellHostSpec, ShellLaunchCatalog, ShellLauncherCommand,
        ShellLauncherState, ShellRouteCatalog, ShellRouteId, ShellRouteSpec, ShellRouter,
        ShellScopedEntity,
    };

    /// A minimal headless shell host: router + sequence + launcher, no rendering.
    fn shell_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalShellPlugins);
        app
    }

    fn active_route(app: &App) -> Option<String> {
        app.world()
            .resource::<ShellRouter>()
            .active
            .as_ref()
            .map(|a| a.route_id.as_str().to_owned())
    }

    /// Register a launcher home route whose experience is the basic launcher.
    fn register_home(app: &mut App, route: &str) {
        app.world_mut()
            .resource_mut::<ShellRouteCatalog>()
            .register(ShellRouteSpec::new(
                route,
                ShellLaunchCatalog::basic_experience_id(),
            ));
    }

    /// The SAME provider, installed identically under any host. It names no home
    /// route — only its own gameplay route — which is the host-independence claim.
    fn register_alpha_provider(app: &mut App) {
        use crate::ShellExperienceAppExt;
        app.register_experience(
            ExperienceRegistration::new("game.alpha", "Alpha", "alpha-route")
                .with_description("the alpha experience"),
            ShellRouteSpec::new("alpha-route", "alpha-exp"),
        );
    }

    #[test]
    fn registration_derives_catalog_and_launches_without_host_match() {
        use crate::ShellExperienceAppExt;
        let mut app = shell_app();
        register_home(&mut app, "launcher");
        register_alpha_provider(&mut app);
        // A second provider, present but unavailable, still lists with its reason.
        app.register_experience(
            ExperienceRegistration::new("game.beta", "Beta", "beta-route")
                .unavailable("needs the beta feature"),
            ShellRouteSpec::new("beta-route", "beta-exp"),
        );
        app.world_mut()
            .resource_mut::<ShellHostConfiguration>()
            .spec = Some(ShellHostSpec::new("launcher", "launcher"));
        app.update();

        // The launcher catalog is a pure projection of the registry — no host match.
        assert_eq!(app.world().resource::<ShellExperienceRegistry>().len(), 2);
        let catalog = app.world().resource::<ShellLaunchCatalog>();
        assert_eq!(catalog.entries.len(), 2);
        assert_eq!(catalog.entries[0].label, "Alpha");
        assert!(catalog.entries[0].available);
        assert!(!catalog.entries[1].available);
        assert_eq!(
            catalog.entries[1].unavailable_reason.as_deref(),
            Some("needs the beta feature"),
        );
        assert!(app.world().resource::<ShellLauncherState>().active);

        // Launching the selected (first available) entry activates its route.
        // The launcher emits GoTo after the router's command phase, so the route
        // change lands on the following frame.
        app.world_mut()
            .write_message(ShellLauncherCommand::LaunchSelected);
        app.update();
        app.update();
        assert_eq!(active_route(&app), Some("alpha-route".to_owned()));
    }

    #[test]
    fn same_provider_returns_to_each_hosts_home() {
        // Host A enters gameplay directly and returns to home-a.
        let mut host_a = shell_app();
        register_home(&mut host_a, "home-a");
        register_alpha_provider(&mut host_a);
        host_a
            .world_mut()
            .resource_mut::<ShellHostConfiguration>()
            .spec = Some(ShellHostSpec::new("alpha-route", "home-a"));
        host_a.update();
        assert_eq!(active_route(&host_a), Some("alpha-route".to_owned()));
        host_a.world_mut().write_message(ShellCommand::QuitToHome);
        host_a.update();
        assert_eq!(active_route(&host_a), Some("home-a".to_owned()));

        // Host B: the SAME provider, a DIFFERENT home. QuitToHome is semantic;
        // the provider never named either launcher route.
        let mut host_b = shell_app();
        register_home(&mut host_b, "home-b");
        register_alpha_provider(&mut host_b);
        host_b
            .world_mut()
            .resource_mut::<ShellHostConfiguration>()
            .spec = Some(ShellHostSpec::new("alpha-route", "home-b"));
        host_b.update();
        assert_eq!(active_route(&host_b), Some("alpha-route".to_owned()));
        host_b.world_mut().write_message(ShellCommand::QuitToHome);
        host_b.update();
        assert_eq!(active_route(&host_b), Some("home-b".to_owned()));
    }

    #[derive(Component)]
    struct GameplayScoped;

    #[test]
    fn repeated_launch_quit_relaunch_leaks_no_scoped_state() {
        let mut app = shell_app();
        register_home(&mut app, "home");
        for (route, exp) in [("game-a", "exp-a"), ("game-b", "exp-b")] {
            app.world_mut()
                .resource_mut::<ShellRouteCatalog>()
                .register(ShellRouteSpec::new(route, exp));
        }
        app.world_mut()
            .resource_mut::<ShellHostConfiguration>()
            .spec = Some(ShellHostSpec::new("home", "home"));
        app.update();
        assert_eq!(active_route(&app), Some("home".to_owned()));

        // launch -> (provider spawns scoped state) -> quit to home. Repeat, then
        // launch a DIFFERENT experience. A leak or duplicate would accumulate.
        let run_cycle = |app: &mut App, route: &str| {
            app.world_mut()
                .write_message(ShellCommand::GoTo(ShellRouteId::new(route)));
            app.update();
            let activation_id = app
                .world()
                .resource::<ShellRouter>()
                .active
                .as_ref()
                .expect("route active after launch")
                .activation_id;
            app.world_mut()
                .spawn((ShellScopedEntity { activation_id }, GameplayScoped));
            app.update();
            // Exactly one scoped entity while the experience is active.
            let live = {
                let mut q = app.world_mut().query::<&GameplayScoped>();
                q.iter(app.world()).count()
            };
            assert_eq!(live, 1, "one scoped entity while {route} is active");
            app.world_mut().write_message(ShellCommand::QuitToHome);
            app.update();
            assert_eq!(active_route(app), Some("home".to_owned()));
        };
        run_cycle(&mut app, "game-a");
        run_cycle(&mut app, "game-a");
        run_cycle(&mut app, "game-b");

        // Nothing experience-owned survived any return.
        let leaked = {
            let mut q = app.world_mut().query::<&GameplayScoped>();
            q.iter(app.world()).count()
        };
        assert_eq!(leaked, 0, "no scoped gameplay entity may survive a return");
    }
}
