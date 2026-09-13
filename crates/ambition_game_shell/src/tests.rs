use std::time::Duration;

use ambition_load::{LoadBarrierSpec, LoadCommand, LoadCoordinator, LoadPlanSpec};

use crate::*;

#[test]
fn initial_and_home_routes_are_independent() {
    let mut loads = LoadCoordinator::default();
    let mut prepared = PreparedSessionRegistry::default();
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("direct-game", "game"));
    catalog.register(ShellRouteSpec::new("demo-home", "launcher"));
    let host = ShellHostConfiguration {
        spec: Some(ShellHostSpec::new("direct-game", "demo-home")),
    };
    let mut router = ShellRouter::default();
    let events = router.apply(
        ShellCommand::Initialize,
        &catalog,
        &host,
        &mut loads,
        &mut prepared,
    );
    assert!(
        matches!(events.last(), Some(ShellEvent::RouteActivated(active)) if active.route_id.as_str() == "direct-game")
    );
    let events = router.apply(
        ShellCommand::QuitToHome,
        &catalog,
        &host,
        &mut loads,
        &mut prepared,
    );
    assert!(
        matches!(events.last(), Some(ShellEvent::RouteActivated(active)) if active.route_id.as_str() == "demo-home")
    );
}

#[test]
fn route_waits_for_its_load_barrier() {
    let mut loads = LoadCoordinator::default();
    let mut prepared = PreparedSessionRegistry::default();
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
        &mut prepared,
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
    let events = router.advance_pending(&catalog, &mut loads, &mut prepared, &holds);
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
    let mut prepared = PreparedSessionRegistry::default();
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(
        ShellRouteSpec::new("credits", "credits").on_complete(ShellCompletionPolicy::ReturnHome),
    );
    catalog.register(ShellRouteSpec::new("home", "launcher"));
    let host = ShellHostConfiguration {
        spec: Some(ShellHostSpec::new("credits", "home")),
    };
    let mut router = ShellRouter::default();
    router.apply(
        ShellCommand::Initialize,
        &catalog,
        &host,
        &mut loads,
        &mut prepared,
    );
    let activation_id = router.active.as_ref().unwrap().activation_id;
    let events = router.apply(
        ShellCommand::ExperienceCompleted { activation_id },
        &catalog,
        &host,
        &mut loads,
        &mut prepared,
    );
    assert!(
        matches!(events.last(), Some(ShellEvent::RouteActivated(active)) if active.route_id.as_str() == "home")
    );
}

#[test]
fn route_hold_delays_commit_and_activation() {
    let mut loads = LoadCoordinator::default();
    let mut prepared = PreparedSessionRegistry::default();
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
        &mut prepared,
    );
    loads.apply(LoadCommand::SetDiscovery {
        load_id: load.clone(),
        barrier_id: barrier.clone(),
        open: false,
        forecast: None,
    });
    assert!(router
        .advance_pending(&catalog, &mut loads, &mut prepared, &holds)
        .is_empty());
    assert!(router.active.is_none());

    holds.release(&ShellRouteId::new("held"), &ShellHoldId::new("test-hold"));
    assert!(matches!(
        router
            .advance_pending(&catalog, &mut loads, &mut prepared, &holds)
            .last(),
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
        ActiveShellSequence, ExperienceRegistration, MinimalShellPlugins, ShellCommand,
        ShellCompletionPolicy, ShellExperienceId, ShellExperienceRegistry, ShellHostConfiguration,
        ShellHostSpec, ShellLaunchCatalog, ShellLauncherCommand, ShellLauncherState,
        ShellRouteCatalog, ShellRouteId, ShellRouteSpec, ShellRouter, ShellScopedEntity,
        ShellSegmentId, ShellSegmentRole, ShellSegmentSpec, ShellSequenceCatalog,
        ShellSequenceCommand, ShellSequenceSpec,
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

    /// Moving the cursor must not rebuild the launcher.
    ///
    /// The cursor is RUNTIME STATE, not structure. `follow_the_launcher_cursor`
    /// moves the highlight in place through `MenuVisualState`, and the restyle
    /// system recolours what changed; the key names only what a rebuild is
    /// actually needed for — which rows exist and what they say.
    ///
    /// this asserts entity IDENTITY, not a node count. A rebuild that happened
    /// to spawn the same number of nodes would pass a count and still have
    /// thrown the tree away.
    ///
    /// gated, because the presentation is. `basic_presentation` is not a
    /// default feature, so a bare `cargo test -p ambition_game_shell` renders no
    /// launcher at all — the 38 tests it reports never touch this. The runner's
    /// per-crate feature job is what runs it (and
    /// `scripts/tests/test_no_test_module_is_dark.py` is what keeps that job
    /// from being exempted out from under it).
    #[cfg(feature = "basic_presentation")]
    #[test]
    fn moving_the_launcher_cursor_does_not_respawn_the_ui_tree() {
        use crate::BasicShellUiRoot;
        use crate::ShellExperienceAppExt;
        use bevy::prelude::{Entity, With};

        let mut app = shell_app();
        register_home(&mut app, "launcher");
        register_alpha_provider(&mut app);
        app.register_experience(
            ExperienceRegistration::new("game.beta", "Beta", "beta-route"),
            ShellRouteSpec::new("beta-route", "beta-exp"),
        );
        app.world_mut()
            .resource_mut::<ShellHostConfiguration>()
            .spec = Some(ShellHostSpec::new("launcher", "launcher"));
        app.update();
        app.update();

        let roots = |app: &mut App| -> Vec<Entity> {
            let world = app.world_mut();
            let mut query = world.query_filtered::<Entity, With<BasicShellUiRoot>>();
            let mut found: Vec<Entity> = query.iter(world).collect();
            found.sort();
            found
        };

        let before = roots(&mut app);
        assert_eq!(
            before.len(),
            1,
            "the launcher should have rendered exactly one root, got {before:?}"
        );
        let selected_before = app.world().resource::<ShellLauncherState>().selected;

        app.world_mut().write_message(ShellLauncherCommand::Next);
        // TWO updates.
        app.update();
        app.update();

        assert_ne!(
            app.world().resource::<ShellLauncherState>().selected,
            selected_before,
            "the cursor did not move, so this test would pass without proving \
             anything about rebuilds"
        );
        assert_eq!(
            roots(&mut app),
            before,
            "the launcher tree was despawned and respawned by a cursor move — \
             `launcher.selected` has leaked back into the rebuild key, and every \
             arrow press now throws away hover state and any in-flight animation"
        );
    }

    #[test]
    fn launcher_activate_targets_the_pressed_selectable_row() {
        use crate::ShellExperienceAppExt;
        let mut app = shell_app();
        register_home(&mut app, "launcher");
        register_alpha_provider(&mut app);
        app.register_experience(
            ExperienceRegistration::new("game.beta", "Beta", "beta-route"),
            ShellRouteSpec::new("beta-route", "beta-exp"),
        );
        app.world_mut()
            .resource_mut::<ShellHostConfiguration>()
            .spec = Some(ShellHostSpec::new("launcher", "launcher"));
        app.update();

        app.world_mut()
            .write_message(ShellLauncherCommand::Activate(1));
        app.update();
        app.update();
        assert_eq!(active_route(&app), Some("beta-route".to_owned()));
    }

    /// An empty Home list survives confirm / focus / activate.
    ///
    /// ⛔ `selectable` is `available.len() + usize::from(exit_index.is_some())`, and an
    /// empty catalog with NO exit label makes it zero. `basic_presentation` has an
    /// explicit empty-page branch, so that is a supported state rather than a
    /// theoretical one — and three command arms computed `selectable - 1` before
    /// checking it, which underflows. `Previous`/`Next` already returned early on
    /// `!has_launchable`; these three did the arithmetic first.
    #[test]
    fn an_empty_home_list_survives_confirm_focus_and_activate() {
        let mut app = shell_app();
        register_home(&mut app, "launcher");
        // No experiences registered: `available` is empty. No exit label either, so
        // `selectable == 0` — the underflow case.
        app.world_mut()
            .resource_mut::<ShellHostConfiguration>()
            .spec = Some(ShellHostSpec::new("launcher", "launcher"));
        app.update();

        // ⛔ AND CLEAR THE EXIT LABEL. It defaults to `Some("Exit")`, which makes
        // `selectable == 1` and `selectable - 1 == 0` — no underflow, and a poison run
        // proved the test was passing for exactly that reason. The review's state is
        // an empty catalog AND no exit row.
        app.world_mut()
            .resource_mut::<crate::ShellLauncherPresentation>()
            .exit_label = None;
        app.update();

        // ⚠ ANTI-VACUITY: prove the zero-row state actually exists before asserting
        // that it survives. A test that never reaches `selectable == 0` passes whether
        // or not the guards are there — which is exactly what a poison run revealed.
        {
            let catalog = app.world().resource::<crate::ShellLaunchCatalog>();
            assert!(
                catalog.entries.is_empty(),
                "this test needs an EMPTY catalog to reach `selectable == 0`; it has {:?}",
                catalog.entries
            );
        }
        assert!(
            app.world()
                .resource::<crate::ShellLauncherPresentation>()
                .exit_label
                .is_none(),
            "this test needs NO exit row; with one, `selectable == 1` and the \
             subtraction it is guarding cannot underflow"
        );
        for command in [
            ShellLauncherCommand::LaunchSelected,
            ShellLauncherCommand::Focus(0),
            ShellLauncherCommand::Activate(0),
            ShellLauncherCommand::Focus(7),
            ShellLauncherCommand::Activate(7),
        ] {
            app.world_mut().write_message(command);
            app.update();
        }
        // Reaching here at all is the assertion: without the zero-row guards each
        // confirm arm panics on `0 - 1` in debug. Nothing should have been launched.
        // ⚠ The launcher's OWN route is the expected value here, not `None`: the shell
        // is sitting on its home page. The assertion is that it did not NAVIGATE
        // anywhere, which on an empty list means staying exactly where it was.
        assert_eq!(
            active_route(&app),
            Some("launcher".to_owned()),
            "an empty Home list navigated away from the launcher — there is nothing \
             to launch, so every confirm/activate should be inert"
        );
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

    // ── Startup sequence integration (acceptance #37, #39, #40) ─────────────────

    /// Register a startup route that plays a programmatic sequence, then routes
    /// to the launcher when the sequence completes.
    fn register_startup_sequence(app: &mut App, segments: Vec<&str>) {
        let experience = ShellExperienceId::new("startup-seq");
        app.world_mut()
            .resource_mut::<ShellRouteCatalog>()
            .register(
                ShellRouteSpec::new("startup", experience.clone())
                    .on_complete(ShellCompletionPolicy::GoTo(ShellRouteId::new("launcher"))),
            );
        app.world_mut()
            .resource_mut::<ShellSequenceCatalog>()
            .register(
                experience,
                ShellSequenceSpec {
                    segments: segments
                        .into_iter()
                        .map(|id| {
                            ShellSegmentSpec::registered(
                                id,
                                ShellSegmentRole::Vanity,
                                format!("{id}-card"),
                            )
                        })
                        .collect(),
                },
            );
    }

    fn active_registered_segment(app: &App) -> Option<(crate::ShellActivationId, ShellSegmentId)> {
        app.world()
            .resource::<ActiveShellSequence>()
            .registered_segment()
            .map(|(activation, segment, _)| (activation, segment.clone()))
    }

    #[test]
    fn startup_sequence_hands_off_to_configured_route() {
        let mut app = shell_app();
        register_home(&mut app, "launcher");
        register_startup_sequence(&mut app, vec!["boot"]);
        app.world_mut()
            .resource_mut::<ShellHostConfiguration>()
            .spec = Some(ShellHostSpec::new("startup", "launcher"));
        app.update();
        assert_eq!(active_route(&app), Some("startup".to_owned()));

        // Complete the one programmatic segment; the sequence finishes and the
        // route's on_complete policy hands off to the launcher.
        let (activation_id, segment_id) =
            active_registered_segment(&app).expect("boot segment is active");
        app.world_mut()
            .write_message(ShellSequenceCommand::ProgrammaticSegmentCompleted {
                activation_id,
                segment_id,
            });
        app.update();
        app.update();
        assert_eq!(active_route(&app), Some("launcher".to_owned()));
    }

    #[test]
    fn stale_segment_completion_cannot_advance_a_later_segment() {
        let mut app = shell_app();
        register_home(&mut app, "launcher");
        register_startup_sequence(&mut app, vec!["first", "second"]);
        app.world_mut()
            .resource_mut::<ShellHostConfiguration>()
            .spec = Some(ShellHostSpec::new("startup", "launcher"));
        app.update();

        let (activation_id, first_id) =
            active_registered_segment(&app).expect("first segment active");
        app.world_mut()
            .write_message(ShellSequenceCommand::ProgrammaticSegmentCompleted {
                activation_id,
                segment_id: first_id.clone(),
            });
        app.update();
        let (_, second_id) = active_registered_segment(&app).expect("second segment active");
        assert_ne!(first_id, second_id);

        // A stale completion naming the retired first segment must not advance the
        // now-current second segment.
        app.world_mut()
            .write_message(ShellSequenceCommand::ProgrammaticSegmentCompleted {
                activation_id,
                segment_id: first_id,
            });
        app.update();
        assert_eq!(
            active_registered_segment(&app).map(|(_, id)| id),
            Some(second_id),
            "stale completion must not advance the sequence",
        );
        assert_eq!(active_route(&app), Some("startup".to_owned()));
    }

    #[test]
    fn dev_host_bypasses_startup_sequence_and_enters_route_directly() {
        let mut app = shell_app();
        register_home(&mut app, "launcher");
        // A plain gameplay route with no registered sequence.
        app.world_mut()
            .resource_mut::<ShellRouteCatalog>()
            .register(ShellRouteSpec::new("gameplay", "game-exp"));
        app.world_mut()
            .resource_mut::<ShellHostConfiguration>()
            .spec = Some(ShellHostSpec::new("gameplay", "launcher"));
        app.update();
        assert_eq!(active_route(&app), Some("gameplay".to_owned()));
        assert!(
            app.world()
                .resource::<ActiveShellSequence>()
                .runtime
                .is_none(),
            "no sequence runs when entering a plain route directly",
        );
    }
}

/// ⭐⭐ **RE-ENTERING THE ROUTE YOU ARE ALREADY *ACTIVE ON* REQUESTS A FRESH
/// PREPARATION, AND THAT IS THE ROAD A CONTENT RELOAD SHOULD TAKE** (fast-
/// iteration I3, step 4's *"file watching calls the same request path"*).
///
/// ⛔⛤ **TWO SIBLINGS LOOK LIKE THEY ALREADY PROVE THIS AND NEITHER DOES.**
/// `provider_retry_supersedes_the_failed_transaction…` re-requests after the
/// first transaction FAILED, so it pins retry-after-failure.
/// `same_provider_relaunch_mints_a_fresh_load_transaction` runs a full round
/// trip and then **`QuitToHome` before relaunching** — it pins relaunch-after-
/// leaving. A reload leaves nothing: the route is ACTIVE, the session is
/// published, and the request arrives anyway. That branch had no witness.
///
/// ⇒ **SO A RELOAD NEEDS NO NEW PUBLICATION ROAD AND NO NEW ROUTE KIND.** Select
/// the new pack, re-request the route already running, and the existing
/// preparation lifecycle allocates the epoch, fingerprints the content and
/// publishes at the activation boundary — with the old generation authoritative
/// until it does.
#[test]
fn re_requesting_the_route_already_active_starts_a_new_transaction() {
    let mut loads = LoadCoordinator::default();
    let mut prepared = PreparedSessionRegistry::default();
    let plan = ProviderPreparationPlan::new("Prepare fixture", "ready", "Ready")
        .required("publish", "Publish prepared session");
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("game", "fixture").preparing_with(plan));
    let host = ShellHostConfiguration::default();
    let mut router = ShellRouter::default();

    // The same round trip the relaunch sibling runs — request, publish, complete,
    // close discovery, advance to activation — so the ONLY difference between
    // that test and this one is the `QuitToHome` it does and this does not.
    let launch = |router: &mut ShellRouter,
                  loads: &mut LoadCoordinator,
                  prepared: &mut PreparedSessionRegistry| {
        let transaction = router
            .apply(
                ShellCommand::GoTo(ShellRouteId::new("game")),
                &catalog,
                &host,
                loads,
                prepared,
            )
            .iter()
            .find_map(|event| match event {
                ShellEvent::PreparationRequested(transaction) => Some(transaction.clone()),
                _ => None,
            })
            .expect(
                "a request for this route produced no preparation, so a content \
                 reload cannot reach the existing lifecycle this way",
            );
        assert!(prepared.publish(&transaction).is_some());
        loads.apply(LoadCommand::SetWorkState {
            load_id: transaction.barrier.load_id.clone(),
            work_id: ambition_load::LoadWorkId::new("publish"),
            state: ambition_load::LoadWorkState::Complete,
        });
        loads.apply(LoadCommand::SetDiscovery {
            load_id: transaction.barrier.load_id.clone(),
            barrier_id: transaction.barrier.barrier_id.clone(),
            open: false,
            forecast: None,
        });
        let holds = ShellRouteHolds::default();
        assert!(
            matches!(
                router
                    .advance_pending(&catalog, loads, prepared, &holds)
                    .last(),
                Some(ShellEvent::RouteActivated(_))
            ),
            "the route did not activate, so the arm below is not about a LIVE route"
        );
        transaction
    };

    let first = launch(&mut router, &mut loads, &mut prepared);
    // ⛔ NO `QuitToHome`. The route stays active across this line, which is the
    // entire point.
    let second = launch(&mut router, &mut loads, &mut prepared);
    assert_ne!(
        first.barrier.load_id, second.barrier.load_id,
        "re-requesting the ACTIVE route reused its transaction, so a reload \
         would be indistinguishable from the preparation already done"
    );
    assert_eq!(
        (first.route_id.clone(), first.experience_id.clone()),
        (second.route_id.clone(), second.experience_id.clone()),
        "the two requests are not for the same route, so this is navigation \
         rather than re-preparation"
    );
}

#[test]
fn provider_retry_supersedes_the_failed_transaction_and_rejects_stale_publication() {
    let mut loads = LoadCoordinator::default();
    let mut prepared = PreparedSessionRegistry::default();
    let plan = ProviderPreparationPlan::new("Prepare fixture", "ready", "Ready")
        .required("publish", "Publish prepared session");
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("game", "fixture").preparing_with(plan));
    let host = ShellHostConfiguration::default();
    let mut router = ShellRouter::default();

    let first_events = router.apply(
        ShellCommand::GoTo(ShellRouteId::new("game")),
        &catalog,
        &host,
        &mut loads,
        &mut prepared,
    );
    let first = first_events
        .iter()
        .find_map(|event| match event {
            ShellEvent::PreparationRequested(transaction) => Some(transaction.clone()),
            _ => None,
        })
        .expect("first request creates a transaction");
    loads.apply(LoadCommand::SetWorkState {
        load_id: first.barrier.load_id.clone(),
        work_id: ambition_load::LoadWorkId::new("publish"),
        state: ambition_load::LoadWorkState::Failed(
            ambition_load::LoadFailure::new("fixture failed", "fixture").retryable(true),
        ),
    });

    let retry_events = router.apply(
        ShellCommand::ReplaceWith {
            route: ShellRouteId::new("game"),
            request: None,
        },
        &catalog,
        &host,
        &mut loads,
        &mut prepared,
    );
    let second = retry_events
        .iter()
        .find_map(|event| match event {
            ShellEvent::PreparationRequested(transaction) => Some(transaction.clone()),
            _ => None,
        })
        .expect("retry creates a replacement transaction");

    assert_ne!(first.barrier.load_id, second.barrier.load_id);
    assert!(
        !loads.contains(&first.barrier.load_id),
        "superseded plans are retired after their cancellation semantics are recorded",
    );
    assert!(loads.contains(&second.barrier.load_id));
    assert!(
        prepared.publish(&first).is_none(),
        "a delayed publication for the failed transaction cannot become authoritative",
    );
    assert!(prepared.publish(&second).is_some());
}

#[test]
fn a_failed_route_preparation_surfaces_the_provider_reason_not_just_failed() {
    // The terminal event must now carry the provider's developer detail so
    // `log_shell_routing_failures` — and any headless consumer inspecting the event — can name the
    // cause instead of watching the route stall.
    let mut loads = LoadCoordinator::default();
    let mut prepared = PreparedSessionRegistry::default();
    let plan = ProviderPreparationPlan::new("Prepare fixture", "ready", "Ready")
        .required("publish", "Publish prepared session");
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("game", "fixture").preparing_with(plan));
    let host = ShellHostConfiguration::default();
    let mut router = ShellRouter::default();

    let events = router.apply(
        ShellCommand::GoTo(ShellRouteId::new("game")),
        &catalog,
        &host,
        &mut loads,
        &mut prepared,
    );
    let transaction = events
        .iter()
        .find_map(|event| match event {
            ShellEvent::PreparationRequested(transaction) => Some(transaction.clone()),
            _ => None,
        })
        .expect("GoTo on a preparing route requests a preparation");

    // The provider refuses preparation with a specific, well-worded reason —
    // exactly the audio-fragment refusal the Outlander fixture recorded.
    loads.apply(LoadCommand::SetWorkState {
        load_id: transaction.barrier.load_id.clone(),
        work_id: ambition_load::LoadWorkId::new("publish"),
        state: ambition_load::LoadWorkState::Failed(ambition_load::LoadFailure::new(
            "This world could not be prepared.",
            "provider registered no explicit audio fragment",
        )),
    });

    let holds = ShellRouteHolds::default();
    let events = router.advance_pending(&catalog, &mut loads, &mut prepared, &holds);
    let failures = events
        .iter()
        .find_map(|event| match event {
            ShellEvent::CommandRejected(ShellCommandRejection::LoadFailed {
                readiness: ambition_load::BarrierReadiness::Failed,
                failures,
            }) => Some(failures.clone()),
            _ => None,
        })
        .expect("a failed preparation is reported as LoadFailed(Failed)");
    assert_eq!(
        failures.len(),
        1,
        "the single failed work item's reason is carried through, not discarded",
    );
    assert_eq!(
        failures[0].developer_detail, "provider registered no explicit audio fragment",
        "the provider's developer detail reaches the terminal event — a headless \
         host can now name why the route failed instead of watching it stall",
    );

    // The terminal report fires once, not on every advance, or a headless log
    // would spam the same failure every frame the route stays pending.
    let repeat = router.advance_pending(&catalog, &mut loads, &mut prepared, &holds);
    assert!(
        !repeat
            .iter()
            .any(|event| matches!(event, ShellEvent::CommandRejected(_))),
        "the failure is reported once (terminal_reported latch), not re-emitted",
    );
}

/// ⭐⭐ **A LOAD THAT FAILS WHILE THE SHELL WAITS NAMES THE REQUEST THAT ASKED
/// FOR IT.** This is the road a production transaction actually dies on and it
/// named nothing: `start_route`'s terminal check only fires for a barrier that is
/// ALREADY terminal when the command arrives, which never happens for a load the
/// router just minted. Every real failure arrives HERE, one `advance_pending` at
/// a time, and `CommandRejected(LoadFailed { .. })` carries a readiness and a
/// failure list and no identity at all.
///
/// ⇒ So a caller correlating on its own request — the content reload, whose
/// staged generation is refused while one is in flight — could only infer
/// ownership from `ShellRouter.pending`, and an unrelated rejection while its own
/// load was pending discarded the edit.
/// ⭐⭐ **A SUPERSEDED TRANSACTION NAMES THE REQUEST IT CANCELLED**, and before
/// `TransactionEnded` it named nothing whatsoever. `start_route` took
/// `self.pending`, called `prepared.cancel(&previous.barrier)` — a `records`
/// removal returning `bool`, a state mutation and not an observable event — and
/// returned events describing the NEW route only. The caller waiting on the old
/// one was left waiting on a load that would never activate, with no signal it
/// could ever receive.
///
/// ⛔ NOT DERIVABLE FROM THE OTHER EVENTS. The two that do carry a barrier,
/// `PreparationRequested` and `WaitingForLoad`, both name the SUPERSEDING
/// transaction; inferring the cancelled one from `ShellRouter.pending` reads the
/// very field that was just overwritten.
/// ⭐⭐ **A CANCELLED TRANSACTION NAMES THE REQUEST IT ABANDONED, AND
/// `cancel_pending` HAD NO TEST AT ALL.**
///
/// MEASURED 2026-09-12: `git grep cancel_pending` returns three hits — the
/// definition and its two callers in
/// `ambition_load_presentation::shell_adapter` (the load screen's CANCEL and
/// QUIT). **Zero tests.** A public method that abandons a live transaction was
/// exercised by nothing, which is why it could be a bare `self.pending.take()`
/// for as long as it was.
///
/// ⛔ `Cancelled` IS NOT `Superseded` AND NOT `Failed`. Nobody asked for
/// something else and nothing went wrong — a person pressed cancel. A caller that
/// retries on supersession must not retry here, and one that reports a failure
/// must not report this.
#[test]
fn a_cancelled_transaction_names_the_request_it_abandoned() {
    let mut loads = LoadCoordinator::default();
    let mut prepared = PreparedSessionRegistry::default();
    let plan = ProviderPreparationPlan::new("Prepare fixture", "ready", "Ready")
        .required("publish", "Publish prepared session");
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("game", "fixture").preparing_with(plan));
    let host = ShellHostConfiguration::default();
    let mut router = ShellRouter::default();

    let mine = ShellRequestId::new("reload.game.1");
    let transaction = router
        .apply(
            ShellCommand::ReplaceWith {
                route: ShellRouteId::new("game"),
                request: Some(mine.clone()),
            },
            &catalog,
            &host,
            &mut loads,
            &mut prepared,
        )
        .iter()
        .find_map(|event| match event {
            ShellEvent::PreparationRequested(transaction) => Some(transaction.clone()),
            _ => None,
        })
        .expect("a ReplaceWith on a preparing route requests a preparation");
    // ⛔ THE PREMISE: there IS a pending transaction to cancel. Cancelling
    // nothing correctly emits nothing, and this arm must not pass that way.
    assert!(
        router.pending.is_some(),
        "the fixture has nothing pending, so an empty result would be correct",
    );

    // ⛔ THE OTHER PREMISE: the two authorities a cancel must retire are
    // actually resident. Without these the lifecycle assertions below pass
    // against a registry and a coordinator that were empty all along — the
    // classic vacuous-absence arm.
    assert!(
        prepared.contains_load(&transaction.barrier.load_id),
        "the fixture staged no preparation record, so 'no record after cancel' \
         would prove nothing",
    );
    assert!(
        loads.contains(&transaction.barrier.load_id),
        "the fixture staged no load plan, so 'no plan after cancel' would prove \
         nothing",
    );

    let events = router.cancel_pending(&mut loads, &mut prepared);
    let ended = events
        .iter()
        .find_map(|event| match event {
            ShellEvent::TransactionEnded {
                route_id,
                barrier,
                request,
                reason,
            } => Some((
                route_id.clone(),
                barrier.clone(),
                request.clone(),
                reason.clone(),
            )),
            _ => None,
        })
        .expect("cancelling a pending transaction emitted no event naming it");
    assert_eq!(ended.0.as_str(), "game");
    assert_eq!(ended.1, transaction.barrier, "it named a different barrier");
    assert_eq!(ended.2.as_ref(), Some(&mine), "it named a different request");
    assert_eq!(ended.3, TransactionEnd::Cancelled);
    assert!(
        router.pending.is_none(),
        "the transaction was announced as ended and is still pending",
    );

    // ⛔⛤ **AND THE TRANSACTION'S AUTHORITIES ARE GONE, WHICH IS THE HALF THAT
    // WAS MISSING.** `cancel_pending` announced the end and retired nothing, so
    // the abandoned provider could still finish preparing and PUBLISH a prepared
    // session for a transaction the shell had already declared over — into a
    // record that, with `pending` cleared, nothing could ever activate.
    assert!(
        !prepared.contains_load(&transaction.barrier.load_id),
        "the cancelled transaction's preparation record survived its own \
         cancellation, so a late provider can still publish into it",
    );
    assert!(
        !loads.contains(&transaction.barrier.load_id),
        "the cancelled transaction's load plan is still resident authority",
    );
    assert!(
        prepared.is_empty() && loads.is_empty(),
        "cancelling returned the shell to its pre-request baseline, so repeated \
         start/cancel cannot accumulate abandoned authority: prepared={}, loads={}",
        prepared.len(),
        loads.len(),
    );

    // ⛔⛤ **THE LATE PROVIDER CANNOT RESURRECT IT.** This is the consequence the
    // retirement exists to produce, asserted directly rather than inferred from
    // the registry being empty: the provider that was preparing this transaction
    // finishes AFTER the person cancelled, and its publication is refused
    // because the record it would publish into no longer exists.
    //
    // ⚠ It is refused by ABSENCE, and that is worth naming: `publish` returns
    // `None` for a load it does not hold, so the same call that succeeded a
    // moment ago now fails for the one reason that is honest here — there is no
    // transaction to publish into.
    assert!(
        prepared.publish(&transaction).is_none(),
        "a provider finishing after the cancel published a prepared session for \
         a transaction the shell had already declared ended",
    );

    // ⭐ AND CANCELLING NOTHING SAYS NOTHING — the control, without which the
    // arm above is satisfied by a method that emits an event unconditionally.
    assert!(
        router.cancel_pending(&mut loads, &mut prepared).is_empty(),
        "cancelling with nothing pending invented a terminal event",
    );
}

/// ⛔⛤ **REPEATED start → cancel USED TO ACCUMULATE ABANDONED AUTHORITY, ONE
/// RECORD AND ONE PLAN PER CYCLE.**
///
/// The single-cancel arm above proves the two authorities are retired once. This
/// one proves the loop is CLOSED: three full cycles end where they began. A
/// cancel that retired only the newest record would satisfy the arm above and
/// still leak here.
///
/// ⚠ It asserts a RETURN TO BASELINE rather than a bound, because "fewer than
/// three" is the shape of a leak that got slower.
#[test]
fn repeated_start_and_cancel_returns_the_shell_to_its_baseline() {
    let mut loads = LoadCoordinator::default();
    let mut prepared = PreparedSessionRegistry::default();
    let plan = ProviderPreparationPlan::new("Prepare fixture", "ready", "Ready")
        .required("publish", "Publish prepared session");
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("game", "fixture").preparing_with(plan));
    let host = ShellHostConfiguration::default();
    let mut router = ShellRouter::default();

    let baseline = (prepared.len(), loads.len());
    assert_eq!(baseline, (0, 0), "the fixture did not start empty");

    for cycle in 1..=3 {
        router.apply(
            ShellCommand::ReplaceWith {
                route: ShellRouteId::new("game"),
                request: Some(ShellRequestId::new(format!("reload.game.{cycle}"))),
            },
            &catalog,
            &host,
            &mut loads,
            &mut prepared,
        );
        assert_eq!(
            (prepared.len(), loads.len()),
            (1, 1),
            "cycle {cycle} did not stage exactly one preparation and one load, \
             so the cancel below would have nothing to retire",
        );
        router.cancel_pending(&mut loads, &mut prepared);
        assert_eq!(
            (prepared.len(), loads.len()),
            baseline,
            "cycle {cycle} left abandoned authority behind: the shell does not \
             return to its baseline across start/cancel",
        );
    }
}

#[test]
fn a_superseded_transaction_names_the_request_it_cancelled() {
    let mut loads = LoadCoordinator::default();
    let mut prepared = PreparedSessionRegistry::default();
    let plan = ProviderPreparationPlan::new("Prepare fixture", "ready", "Ready")
        .required("publish", "Publish prepared session");
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("game", "fixture").preparing_with(plan));
    let host = ShellHostConfiguration::default();
    let mut router = ShellRouter::default();

    let first_request = ShellRequestId::new("reload.game.1");
    let second_request = ShellRequestId::new("reload.game.2");
    let first = router
        .apply(
            ShellCommand::ReplaceWith {
                route: ShellRouteId::new("game"),
                request: Some(first_request.clone()),
            },
            &catalog,
            &host,
            &mut loads,
            &mut prepared,
        )
        .iter()
        .find_map(|event| match event {
            ShellEvent::PreparationRequested(transaction) => Some(transaction.clone()),
            _ => None,
        })
        .expect("the first request creates a transaction");

    // ⛔ THE PREMISE: the first transaction is STILL PENDING, neither ready nor
    // terminal. A supersession of nothing would emit nothing correctly.
    let holds = ShellRouteHolds::default();
    assert!(
        router
            .advance_pending(&catalog, &mut loads, &mut prepared, &holds)
            .is_empty(),
        "the fixture's first transaction already resolved, so nothing is being          superseded",
    );

    // ⭐ A SECOND SAVE ARRIVES BEFORE THE FIRST FINISHED — what a file watcher
    // makes ordinary.
    let events = router.apply(
        ShellCommand::ReplaceWith {
            route: ShellRouteId::new("game"),
            request: Some(second_request.clone()),
        },
        &catalog,
        &host,
        &mut loads,
        &mut prepared,
    );
    let ended = events
        .iter()
        .find_map(|event| match event {
            ShellEvent::TransactionEnded {
                barrier,
                request,
                reason,
                ..
            } => Some((barrier.clone(), request.clone(), reason.clone())),
            _ => None,
        })
        .expect("supersession emitted no event naming the transaction it cancelled");
    assert_eq!(
        ended.0, first.barrier,
        "it named the SUPERSEDING barrier, which the caller already knew about",
    );
    assert_eq!(
        ended.1.as_ref(),
        Some(&first_request),
        "it named the superseding request rather than the cancelled one",
    );
    assert_eq!(ended.2, TransactionEnd::Superseded);
    // ⚠ AND NOT AS A FAILURE. Nothing went wrong; the caller asked for something
    // else. A caller that retried on `Failed` would loop forever here.
    assert_ne!(ended.2, TransactionEnd::Failed);

    let second = events
        .iter()
        .find_map(|event| match event {
            ShellEvent::PreparationRequested(transaction) => Some(transaction.clone()),
            _ => None,
        })
        .expect("the superseding request creates its own transaction");
    assert_eq!(
        second.request.as_ref(),
        Some(&second_request),
        "the new transaction carries the new caller's identity",
    );
    assert_ne!(first.barrier.load_id, second.barrier.load_id);
}

#[test]
fn a_load_that_fails_while_the_shell_waits_names_the_request_that_asked_for_it() {
    let mut loads = LoadCoordinator::default();
    let mut prepared = PreparedSessionRegistry::default();
    let plan = ProviderPreparationPlan::new("Prepare fixture", "ready", "Ready")
        .required("publish", "Publish prepared session");
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("game", "fixture").preparing_with(plan));
    let host = ShellHostConfiguration::default();
    let mut router = ShellRouter::default();

    let mine = ShellRequestId::new("reload.game.1");
    let events = router.apply(
        ShellCommand::ReplaceWith {
            route: ShellRouteId::new("game"),
            request: Some(mine.clone()),
        },
        &catalog,
        &host,
        &mut loads,
        &mut prepared,
    );
    let transaction = events
        .iter()
        .find_map(|event| match event {
            ShellEvent::PreparationRequested(transaction) => Some(transaction.clone()),
            _ => None,
        })
        .expect("a ReplaceWith on a preparing route requests a preparation");
    // ⛔ THE PREMISE: the request reached the transaction. Without it the arm
    // below would be asserting that `None` equals `None`.
    assert_eq!(
        transaction.request.as_ref(),
        Some(&mine),
        "the caller's request id did not reach the preparation transaction",
    );

    loads.apply(LoadCommand::SetWorkState {
        load_id: transaction.barrier.load_id.clone(),
        work_id: ambition_load::LoadWorkId::new("publish"),
        state: ambition_load::LoadWorkState::Failed(ambition_load::LoadFailure::new(
            "This world could not be prepared.",
            "the fixture's provider refused",
        )),
    });

    let holds = ShellRouteHolds::default();
    let events = router.advance_pending(&catalog, &mut loads, &mut prepared, &holds);
    let ended = events
        .iter()
        .find_map(|event| match event {
            ShellEvent::TransactionEnded {
                barrier,
                request,
                reason,
                ..
            } => Some((barrier.clone(), request.clone(), reason.clone())),
            _ => None,
        })
        .expect(
            "a load that failed while the shell waited emitted no event naming              the transaction it ended",
        );
    assert_eq!(ended.0, transaction.barrier, "it named a different barrier");
    assert_eq!(ended.1.as_ref(), Some(&mine), "it named a different request");
    assert_eq!(ended.2, TransactionEnd::Failed);
    // ⚠ AND THE DETAILED REPORT IS STILL THERE. The identity event carries no
    // failure list, so collapsing the two would cost every existing reader the
    // provider's reason.
    assert!(
        events.iter().any(|event| matches!(
            event,
            ShellEvent::CommandRejected(ShellCommandRejection::LoadFailed { .. })
        )),
        "the identity event replaced the failure report instead of joining it",
    );

    // ⛔ ONCE, NOT EVERY FRAME — the `terminal_reported` latch covers both.
    let repeat = router.advance_pending(&catalog, &mut loads, &mut prepared, &holds);
    assert!(
        !repeat
            .iter()
            .any(|event| matches!(event, ShellEvent::TransactionEnded { .. })),
        "the transaction's end was re-emitted every frame the route stayed pending",
    );
}

#[test]
fn same_provider_relaunch_mints_a_fresh_load_transaction() {
    let mut loads = LoadCoordinator::default();
    let mut prepared = PreparedSessionRegistry::default();
    let plan = ProviderPreparationPlan::new("Prepare fixture", "ready", "Ready")
        .required("publish", "Publish prepared session");
    let mut catalog = ShellRouteCatalog::default();
    catalog.register(ShellRouteSpec::new("game", "fixture").preparing_with(plan));
    catalog.register(ShellRouteSpec::new("home", "launcher"));
    let host = ShellHostConfiguration {
        spec: Some(ShellHostSpec::new("home", "home")),
    };
    let mut router = ShellRouter::default();

    let launch = |router: &mut ShellRouter,
                  loads: &mut LoadCoordinator,
                  prepared: &mut PreparedSessionRegistry| {
        let events = router.apply(
            ShellCommand::GoTo(ShellRouteId::new("game")),
            &catalog,
            &host,
            loads,
            prepared,
        );
        let transaction = events
            .iter()
            .find_map(|event| match event {
                ShellEvent::PreparationRequested(transaction) => Some(transaction.clone()),
                _ => None,
            })
            .expect("launch requests preparation");
        assert!(prepared.publish(&transaction).is_some());
        loads.apply(LoadCommand::SetWorkState {
            load_id: transaction.barrier.load_id.clone(),
            work_id: ambition_load::LoadWorkId::new("publish"),
            state: ambition_load::LoadWorkState::Complete,
        });
        loads.apply(LoadCommand::SetDiscovery {
            load_id: transaction.barrier.load_id.clone(),
            barrier_id: transaction.barrier.barrier_id.clone(),
            open: false,
            forecast: None,
        });
        let holds = ShellRouteHolds::default();
        assert!(matches!(
            router
                .advance_pending(&catalog, loads, prepared, &holds)
                .last(),
            Some(ShellEvent::RouteActivated(_))
        ));
        transaction
    };

    let first = launch(&mut router, &mut loads, &mut prepared);
    router.apply(
        ShellCommand::QuitToHome,
        &catalog,
        &host,
        &mut loads,
        &mut prepared,
    );
    loads.retire(&first.barrier.load_id);
    let second = launch(&mut router, &mut loads, &mut prepared);
    assert_ne!(first.barrier.load_id, second.barrier.load_id);
}
