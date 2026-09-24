//! Unit tests for the session bridge. A child module keeps private access
//! through `use super::*;`.

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
    // Every gameplay provider must declare audio. This test game is silent,
    // declared as an explicit empty fragment.
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
    // `ActiveGameplaySession` is the only authority for the scope.
    let scope = live_scope_of(&app, activation).expect("activation owns session scope");

    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands
                .spawn_session_scoped(SessionSpawnScope::scoped(scope), Name::new("session-owned"));
        })
        .unwrap();

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);

    assert!(live_scope_of(&app, activation).is_none());
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

/// Activating a session selects its own provider's audio. A silent provider
/// gets an empty authority, never another provider's. Switching sessions
/// replaces the selection. Returning home retires playback authority.
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
    // `test_game` has music; `silent_game` registers an explicit empty
    // fragment.
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

/// A gameplay provider with no audio fragment is a composition error, not
/// silence. Activating it must fail loudly.
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
    let first_scope = live_scope_of(&app, first_activation).unwrap();

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
    let second_scope = live_scope_of(&app, second_activation).unwrap();
    assert_ne!(first_activation, second_activation);
    assert_ne!(first_scope, second_scope);
}

/// A retirement that arrives after its session already ended changes nothing.
///
/// The retirement block resets `GameMode` without a scope guard. A delayed
/// retirement for an old session must not reach it while a newer session is
/// live.
///
/// The stale activation is real: the fixture activates the route, retires it,
/// and activates it again. The test re-sends the first retirement.
#[test]
fn a_retirement_that_arrives_after_its_session_ended_changes_nothing() {
    let mut app = app();
    // The bridge takes `GameMode` as an `Option`, so install the state machine
    // or the reset could never fire.
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameMode>();
    settle(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo(GAME_ROUTE.into()));
    settle(&mut app);

    let live = app
        .world()
        .resource::<ShellRouter>()
        .active
        .as_ref()
        .expect("the game route is active")
        .activation_id;
    let scope = live_scope_of(&app, live).expect("the live activation owns a scope");
    // Premise: an activation that ran before the live one.
    let stale = ShellActivationId(live.0 - 1);
    assert_ne!(stale, live);
    assert_eq!(
        live_scope_of(&app, stale),
        None,
        "setup: the earlier activation must already be retired"
    );

    // A mode the session is legitimately in, so a reset to `Playing` is visible.
    app.world_mut()
        .resource_mut::<NextState<GameMode>>()
        .set(GameMode::Paused);
    app.update();
    assert_eq!(
        *app.world().resource::<State<GameMode>>().get(),
        GameMode::Paused,
        "setup: the arm cannot see a reset unless the mode starts away from its \
         default"
    );

    app.world_mut()
        .write_message(ShellEvent::RouteDeactivated(activation(stale.0, GAME)));
    app.update();

    assert_eq!(
        live_scope_of(&app, live),
        Some(scope),
        "a retirement for a session that already ended retired the live one"
    );
    assert_eq!(
        app.world().resource::<ActiveSessionScope>().current(),
        Some(scope),
        "the live session's scope was cleared by an earlier activation's \
         retirement"
    );
    assert_eq!(
        *app.world().resource::<State<GameMode>>().get(),
        GameMode::Paused,
        "the delayed retirement reached the unconditional `GameMode` reset and \
         handed a session that is still being played back to `Playing`"
    );
}

/// The scope `activation` owns, or `None` when it is not the live session.
///
/// The live session is the only authority for this.
fn live_scope_of(app: &App, activation: ShellActivationId) -> Option<SessionScopeId> {
    app.world()
        .resource::<ActiveGameplaySession>()
        .0
        .as_ref()
        .filter(|session| session.activation.activation_id == activation)
        .map(|session| session.scope)
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

/// A world prepared for activation A cannot be adopted into activation B.
///
/// Tests `adopt_world`, the only way a gameplay world reaches
/// [`ActiveGameplaySession`]. The positive half shows that the matching
/// activation does adopt the root, so an `adopt_world` that always returns
/// `None` fails.
#[test]
fn a_candidate_world_prepared_for_a_cannot_be_adopted_into_b() {
    let mut app = App::new();
    app.insert_resource(ActiveGameplaySession(Some(gameplay_instance(
        2,
        "provider-b",
        22,
    ))));
    // The candidate root A10 builds before anyone has agreed to adopt it.
    let candidate_root = app.world_mut().spawn_empty().id();

    let stale = activation(1, "provider-a");
    assert!(
        app.world_mut()
            .resource_mut::<ActiveGameplaySession>()
            .adopt_world(&stale, SessionScopeId(11), candidate_root)
            .is_none(),
        "a world prepared for a retired activation may not be adopted into the \
         live one",
    );
    assert!(
        app.world()
            .resource::<ActiveGameplaySession>()
            .active_world_entity()
            .is_none(),
        "the refused adoption still attached the candidate root",
    );

    let live = activation(2, "provider-b");
    let root = app
        .world_mut()
        .resource_mut::<ActiveGameplaySession>()
        .adopt_world(&live, SessionScopeId(22), candidate_root)
        .expect("the live activation adopts its own candidate");
    assert_eq!(root.activation_id, ShellActivationId(2));
    assert_eq!(
        app.world()
            .resource::<ActiveGameplaySession>()
            .active_world_entity(),
        Some(candidate_root),
    );
}

// The session root `SimId` must not carry `ShellActivationId`. That guard is
// `two_local_histories_name_every_simulated_entity_identically` in
// `game/ambition_app/tests/shell_host_lifecycle.rs`.
