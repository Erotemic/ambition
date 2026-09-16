//! Unit tests for the session bridge, extracted to an adjacent child module
//! (test-placement: large private test modules live in `src/foo/tests.rs`,
//! keeping private access via `use super::*;` without widening any API).

#![allow(clippy::module_inception)]

use super::*;
use crate::{
    AmbitionGameShellPlugin, ShellCommand, ShellHostConfiguration, ShellHostSpec,
    ShellRouteCatalog, ShellRouter,
};
use ambition_audio::catalog::AudioCatalogAppExt;
use ambition_platformer2d_shared_tangle::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};
use bevy::ecs::system::RunSystemOnce;

const GAME: &str = "test_game";
const GAME_ROUTE: &str = "test_gameplay";
const HOME: &str = "test_home";

#[derive(Resource, Default)]
struct RetirementObservation {
    active_scope_seen: Option<Option<SessionScopeId>>,
}

fn observe_retirement_scope(
    mut events: MessageReader<GameplaySessionEvent>,
    active_scope: Res<ActiveSessionScope>,
    mut observation: ResMut<RetirementObservation>,
) {
    for event in events.read() {
        if matches!(event, GameplaySessionEvent::Retiring { .. }) {
            observation.active_scope_seen = Some(active_scope.current());
        }
    }
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((AmbitionGameShellPlugin, GameplaySessionBridgePlugin));
    app.register_gameplay_experience(
        ExperienceRegistration::new(GAME, "Test game", GAME_ROUTE),
        ShellRouteSpec::new(GAME_ROUTE, GAME),
    );
    // Every gameplay provider must declare audio intent; the plain test game is
    // deliberately silent, declared as an explicit empty fragment.
    app.register_audio_catalog_fragment(
        ambition_audio::catalog::AudioCatalogFragment::new(GAME, None, None).unwrap(),
    );
    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(ShellRouteSpec::new(HOME, "home"));
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(GAME_ROUTE, HOME));
    app.init_resource::<RetirementObservation>().add_systems(
        Update,
        observe_retirement_scope.in_set(GameplaySessionSet::Providers),
    );
    app
}

fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

#[test]
fn registered_gameplay_route_mints_and_retires_one_scope() {
    let mut app = app();
    settle(&mut app);
    let activation = app
        .world()
        .resource::<ShellRouter>()
        .active
        .as_ref()
        .expect("game active")
        .activation_id;
    let scope = app
        .world()
        .resource::<GameplaySessionLinks>()
        .scope_for(activation)
        .expect("activation owns session scope");
    assert_eq!(
        app.world()
            .resource::<ActiveGameplaySession>()
            .0
            .as_ref()
            .map(|session| session.scope),
        Some(scope),
    );

    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands
                .spawn_session_scoped(SessionSpawnScope::scoped(scope), Name::new("session-owned"));
        })
        .unwrap();

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);

    assert!(app
        .world()
        .resource::<GameplaySessionLinks>()
        .scope_for(activation)
        .is_none());
    assert!(app.world().resource::<ActiveGameplaySession>().0.is_none());
    let mut query = app.world_mut().query::<&SessionScopedEntity>();
    assert_eq!(query.iter(app.world()).count(), 0);
}

#[test]
fn retirement_revokes_spawn_authority_before_provider_teardown() {
    let mut app = app();
    settle(&mut app);
    assert!(app
        .world()
        .resource::<ActiveSessionScope>()
        .current()
        .is_some());

    app.world_mut().write_message(ShellCommand::QuitToHome);
    app.update();

    assert_eq!(
        app.world()
            .resource::<RetirementObservation>()
            .active_scope_seen,
        Some(None),
        "provider teardown must observe no ambient owner for the retired session",
    );
}

/// C1-audio-session: activating a session selects ITS provider's audio;
/// a provider with no registered fragments gets a deliberate empty
/// authority (never another provider's); switching sessions replaces the
/// selection; returning home retires playback authority entirely.
#[test]
fn session_activation_owns_audio_authority_and_home_retires_it() {
    use ambition_audio::catalog::{AudioCatalogFragment, AudioCatalogRegistry};
    use ambition_audio::selection::ActiveAudioSelection;
    use ambition_audio::spec::{MusicRegistry, MusicTrack};

    const SILENT_GAME: &str = "silent_game";
    const SILENT_ROUTE: &str = "silent_gameplay";

    let mut app = app();
    app.register_gameplay_experience(
        ExperienceRegistration::new(SILENT_GAME, "Silent game", SILENT_ROUTE),
        ShellRouteSpec::new(SILENT_ROUTE, SILENT_GAME),
    );
    // `test_game` authors music; `silent_game` registers an EXPLICIT empty
    // fragment (deliberate silence — never inherits another provider's music).
    let mut catalogs = AudioCatalogRegistry::default();
    catalogs
        .register(
            AudioCatalogFragment::new(
                GAME,
                Some(MusicRegistry {
                    default_track: "test_theme".into(),
                    tracks: vec![MusicTrack {
                        id: "test_theme".into(),
                        display_name: "Test theme".into(),
                        asset_path: None,
                        one_shot: false,
                    }],
                }),
                None,
            )
            .unwrap(),
        )
        .unwrap();
    catalogs
        .register(AudioCatalogFragment::new(SILENT_GAME, None, None).unwrap())
        .unwrap();
    app.insert_resource(catalogs);

    settle(&mut app);
    let selection = app.world().resource::<ActiveAudioSelection>();
    assert_eq!(selection.provider_id(), Some(GAME));
    assert_eq!(
        selection.music().map(|m| m.default_track.as_str()),
        Some("test_theme"),
        "activation selects the provider's registered music"
    );

    // Switch to the silent game: authority REPLACED, not inherited.
    app.world_mut()
        .write_message(ShellCommand::ReplaceWith {
            route: SILENT_ROUTE.into(),
            request: None,
        });
    settle(&mut app);
    let selection = app.world().resource::<ActiveAudioSelection>();
    assert_eq!(selection.provider_id(), Some(SILENT_GAME));
    assert!(
        selection.music().is_none(),
        "a provider that registered no audio gets a deliberate empty set, \
         never the previous provider's music"
    );

    // Home retires playback authority.
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    let selection = app.world().resource::<ActiveAudioSelection>();
    assert!(
        selection.current().is_none(),
        "no provider owns playback at the home route"
    );
}

/// Issue 5: a gameplay provider that registered NO audio fragment is a
/// composition error, not inferred silence. Activating it must fail loudly so a
/// host built without an audio system can never be silently mistaken for "every
/// provider is deliberately silent."
#[test]
#[should_panic(expected = "registered no audio catalog fragment")]
fn activating_a_provider_with_no_audio_fragment_panics() {
    const UNWIRED: &str = "unwired_game";
    const UNWIRED_ROUTE: &str = "unwired_gameplay";

    let mut app = App::new();
    app.add_plugins((AmbitionGameShellPlugin, GameplaySessionBridgePlugin));
    app.register_gameplay_experience(
        ExperienceRegistration::new(UNWIRED, "Unwired game", UNWIRED_ROUTE),
        ShellRouteSpec::new(UNWIRED_ROUTE, UNWIRED),
    );
    // Deliberately register NO audio fragment for `unwired_game`.
    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(ShellRouteSpec::new(HOME, "home"));
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(UNWIRED_ROUTE, HOME));
    settle(&mut app);
}

/// : the canonical session instance captures the load barrier that authorized its activation.
#[test]
fn session_instance_carries_its_load_barrier_identity() {
    use ambition_load::{
        AmbitionLoadPlugin, LoadBarrierSpec, LoadCommand, LoadCoordinator, LoadPlanSpec,
    };

    const LOADED_GAME: &str = "loaded_game";
    const LOADED_ROUTE: &str = "loaded_gameplay";

    let mut app = App::new();
    app.add_plugins((
        AmbitionLoadPlugin,
        AmbitionGameShellPlugin,
        GameplaySessionBridgePlugin,
    ));
    app.register_gameplay_experience(
        ExperienceRegistration::new(LOADED_GAME, "Loaded game", LOADED_ROUTE),
        ShellRouteSpec::new(LOADED_ROUTE, LOADED_GAME)
            .requiring("load-plan".into(), "ready-barrier".into()),
    );
    // Declare the loaded game's audio intent (silent) so activation composes.
    app.register_audio_catalog_fragment(
        ambition_audio::catalog::AudioCatalogFragment::new(LOADED_GAME, None, None).unwrap(),
    );
    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(ShellRouteSpec::new(HOME, "home"));
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(LOADED_ROUTE, HOME));
    // Declare and immediately satisfy the barrier (no work, discovery
    // closed) so the route can activate.
    {
        let mut coordinator = app.world_mut().resource_mut::<LoadCoordinator>();
        coordinator.apply(LoadCommand::Begin(LoadPlanSpec::new(
            "load-plan",
            "test plan",
        )));
        coordinator.apply(LoadCommand::DeclareBarrier {
            load_id: "load-plan".into(),
            spec: LoadBarrierSpec::new("ready-barrier", "ready"),
        });
        coordinator.apply(LoadCommand::SetDiscovery {
            load_id: "load-plan".into(),
            barrier_id: "ready-barrier".into(),
            open: false,
            forecast: None,
        });
    }
    settle(&mut app);
    let session = app.world().resource::<ActiveGameplaySession>();
    let instance = session.0.as_ref().expect("session active");
    let load = instance.load.as_ref().expect("load identity captured");
    assert_eq!(load.load_id.as_str(), "load-plan");
    assert_eq!(load.barrier_id.as_str(), "ready-barrier");
}

#[test]
fn relaunch_receives_a_fresh_scope() {
    let mut app = app();
    settle(&mut app);
    let first_activation = app
        .world()
        .resource::<ShellRouter>()
        .active
        .as_ref()
        .unwrap()
        .activation_id;
    let first_scope = app
        .world()
        .resource::<GameplaySessionLinks>()
        .scope_for(first_activation)
        .unwrap();

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(GAME_ROUTE.into()));
    settle(&mut app);

    let second_activation = app
        .world()
        .resource::<ShellRouter>()
        .active
        .as_ref()
        .unwrap()
        .activation_id;
    let second_scope = app
        .world()
        .resource::<GameplaySessionLinks>()
        .scope_for(second_activation)
        .unwrap();
    assert_ne!(first_activation, second_activation);
    assert_ne!(first_scope, second_scope);
}

fn activation(id: u64, experience: &str) -> ActiveShellExperience {
    ActiveShellExperience {
        activation_id: ShellActivationId(id),
        route_id: format!("{experience}-route").into(),
        experience_id: experience.into(),
        parameters: Default::default(),
        load_authorization: None,
        prepared_session: None,
    }
}

fn gameplay_instance(id: u64, experience: &str, scope: u64) -> GameplaySessionInstance {
    let activation = activation(id, experience);
    GameplaySessionInstance {
        activation,
        scope: SessionScopeId(scope),
        load: None,
        prepared: None,
        audio: GameplaySessionAudioContext {
            owner: AudioContextOwner::Gameplay(scope),
            provider_id: experience.to_owned(),
        },
        world: None,
    }
}

#[test]
fn delayed_retirement_for_a_cannot_retire_b() {
    let mut active = ActiveGameplaySession(Some(gameplay_instance(2, "provider-b", 22)));
    assert!(active.retire_if_activation(ShellActivationId(1)).is_none());
    assert_eq!(
        active
            .0
            .as_ref()
            .map(|instance| instance.activation.activation_id),
        Some(ShellActivationId(2)),
    );
}

#[derive(Component)]
struct DelayedWorldPublicationFixture;

#[test]
fn delayed_world_publication_for_a_cannot_attach_to_b() {
    let mut app = App::new();
    app.insert_resource(ActiveGameplaySession(Some(gameplay_instance(
        2,
        "provider-b",
        22,
    ))));
    let stale_activation = activation(1, "provider-a");
    let published = app
        .world_mut()
        .run_system_once(
            move |mut commands: Commands, mut active: ResMut<ActiveGameplaySession>| {
                active.spawn_world_for(
                    &mut commands,
                    &stale_activation,
                    SessionScopeId(11),
                    DelayedWorldPublicationFixture,
                )
            },
        )
        .expect("publication fixture runs");
    assert!(published.is_none());
    let mut worlds = app
        .world_mut()
        .query_filtered::<Entity, With<DelayedWorldPublicationFixture>>();
    assert_eq!(worlds.iter(app.world()).count(), 0);
    assert!(app
        .world()
        .resource::<ActiveGameplaySession>()
        .active_world_entity()
        .is_none());
}

/// ⛔⛤ **THE SESSION ROOT'S CANONICAL `SimId` IS A HOST-LOCAL ROUTE COUNTER, AND
/// THAT IDENTITY IS IN THE PEER CHECKSUM.**
///
/// Found by the GPT architecture review of 2026-09-15, and it is the campaign's
/// largest remaining ID-PEER hole. The chain, each link measured:
///
/// 1. production mints the root as `SimId::singleton("session", activation_id)`
///    — here at `spawn_world_for`, and again on the A10 candidate road in
///    `ambition_platformer2d_provider`'s `lifecycle.rs`;
/// 2. `ShellActivationId` is a per-App route-activation counter — it counts how
///    many shell routes this process has activated, menus included;
/// 3. the root carries `RoomSet` (it is a `PlatformerSessionWorld`), and
///    `require_rollback::<RoomSet>` anchors the entity for rollback;
/// 4. `entity.sim_id` is `component-canonical` in the rollback schema — the
///    WHOLE value is compared between peers.
///
/// ⇒ Two hosts that agreed completely about a gameplay session used to name its
/// root `session:4` and `session:11`, solely because one visited more shell
/// routes before joining. The key is a CONSTANT now and this arm holds that.
///
/// ⛔ THE STANDING `id_peer_audit` GUARD CANNOT SEE THIS CLASS. It censuses
/// registered TYPE NAMES, and `SimId` is a type that is supposed to be
/// canonical. The defect is its PROVENANCE, which no type census can read — the
/// same blind spot that hid `SimId::match_spawn` embedding the activation tick.
/// That is why this is a value-level arm on the production road rather than a
/// row in a list.
///
/// ⭐⭐ **THE REPLACEMENT IS A CONSTANT, AND THE REASON IS THAT THE COUNT WAS
/// DISAMBIGUATING NOTHING.** A canonical identity only has to be unique inside
/// the world a checksum compares. `shell_host_lifecycle`'s `assert_in_game` and
/// `assert_home` pin `session_roots == 1` and `== 0` at every point of a
/// four-session lifecycle, under rollback as well; an A10 candidate root
/// deliberately carries the SAME identity as the live root it will replace
/// (`a_hidden_candidate_may_share_the_live_worlds_identity_and_a_published_one_may_not`),
/// which a constant preserves exactly; and an UNHIDDEN duplicate is refused as
/// `BaselineCaptureError::DuplicateIdentity`.
///
/// ⚠ **THE INVARIANT IS "EXACTLY ONE SESSION ROOT IS VISIBLE", AND A FUTURE
/// RESIDENCY THAT BREAKS IT BREAKS MORE THAN THIS NAME.** Two published session
/// worlds alive at once would have defeated the activation count too: a peer
/// does not share your retirement schedule, so a root that lingers on one host
/// and not the other is already a divergence whatever it is called. The identity
/// is not what would need fixing there.
#[test]
fn two_hosts_with_different_route_histories_name_the_session_root_identically() {
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    // Both hosts enter the SAME agreed experience. Their local terms differ,
    // which is the whole ID-PEER premise: a route-activation count and a session
    // scope id are per-App, so two peers never share them.
    let root_sim_id = |activation_id: u64, scope: u64| -> String {
        let mut app = App::new();
        app.insert_resource(ActiveGameplaySession(Some(gameplay_instance(
            activation_id,
            "arena",
            scope,
        ))));
        let live = activation(activation_id, "arena");
        let spawned = app
            .world_mut()
            .run_system_once(
                move |mut commands: Commands, mut active: ResMut<ActiveGameplaySession>| {
                    active.spawn_world_for(
                        &mut commands,
                        &live,
                        SessionScopeId(scope),
                        DelayedWorldPublicationFixture,
                    )
                },
            )
            .expect("publication fixture runs")
            .expect("the live activation publishes its world");
        app.world()
            .get::<SimId>(spawned)
            .expect("production stamps the session root with a canonical SimId")
            .as_str()
            .to_string()
    };

    let veteran = root_sim_id(11, 63);
    let fresh = root_sim_id(4, 7);

    // ⚠ FIRST, THE PREMISE MUST HOLD, or the arm below is vacuous: the two hosts
    // really did activate different route counts.
    assert_ne!(11, 4, "the two hosts share an activation id");

    assert_eq!(
        veteran, fresh,
        "the session root's canonical SimId depends on the host's \
         route-activation count again, so two peers who agree completely about \
         a session disagree about the identity of its root"
    );
    // ⛔ AND THE COUNTER MUST NOT BE IN THE NAME AT ALL, which is what makes
    // this a provenance claim rather than a hash-collision one. Pinned so a
    // re-keying that merely makes two particular counts collide cannot satisfy
    // the arm above.
    assert_eq!(veteran, "session:root");
    assert_eq!(fresh, "session:root");
}
