//! Shell-to-gameplay-session lifecycle bridge.
//!
//! The shell owns route activation; platformer simulation owns session-scoped
//! entities. This module is the narrow adapter between those authorities. A
//! provider registers an experience as a gameplay session, then receives a
//! fresh engine-neutral [`SessionScopeId`] whenever that experience activates.
//! Route retirement emits the matching session-retirement signal exactly once.

use std::collections::BTreeMap;

use ambition_audio::catalog::AudioCatalogRegistry;
use ambition_audio::selection::ActiveAudioSelection;
use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionGatedSimulation, SessionScopeId, SessionScopePlugin,
    SessionScopeRetired, SessionScopeSet,
};
use bevy::prelude::*;

use crate::{
    ActiveShellExperience, AmbitionGameShellSet, ExperienceRegistration, LoadBarrierRef,
    ShellActivationId, ShellEvent, ShellExperienceAppExt, ShellExperienceId, ShellRouteCatalog,
    ShellRouteSpec,
};

/// Gameplay-session lifecycle facts delivered to provider systems.
#[derive(Message, Clone, Debug, Eq, PartialEq)]
pub enum GameplaySessionEvent {
    /// A registered gameplay experience activated with a fresh session scope.
    Activated {
        activation: ActiveShellExperience,
        scope: SessionScopeId,
    },
    /// The exact shell activation and session scope are retiring.
    Retiring {
        activation: ActiveShellExperience,
        scope: SessionScopeId,
    },
}

impl GameplaySessionEvent {
    pub fn activation(&self) -> &ActiveShellExperience {
        match self {
            Self::Activated { activation, .. } | Self::Retiring { activation, .. } => activation,
        }
    }

    pub fn scope(&self) -> SessionScopeId {
        match self {
            Self::Activated { scope, .. } | Self::Retiring { scope, .. } => *scope,
        }
    }
}

/// Per-experience session configuration a provider declares at registration.
/// Complete defaults: the common provider registers with `Default::default()`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GameplaySessionProfile {
    /// Audio-catalog provider id whose registered music/SFX become the active
    /// audio authority while this experience's session is live. `None` (the
    /// default) selects the experience id itself — providers conventionally
    /// register audio fragments under their own experience id.
    pub audio_provider: Option<String>,
}

/// Deterministic registry of experiences whose routes own gameplay sessions,
/// with each experience's session profile.
#[derive(Resource, Default)]
pub struct GameplaySessionRegistry {
    experiences: BTreeMap<ShellExperienceId, GameplaySessionProfile>,
}

impl GameplaySessionRegistry {
    pub fn register(&mut self, experience: ShellExperienceId) -> bool {
        self.register_with_profile(experience, GameplaySessionProfile::default())
    }

    pub fn register_with_profile(
        &mut self,
        experience: ShellExperienceId,
        profile: GameplaySessionProfile,
    ) -> bool {
        self.experiences.insert(experience, profile).is_none()
    }

    pub fn contains(&self, experience: &ShellExperienceId) -> bool {
        self.experiences.contains_key(experience)
    }

    pub fn profile(&self, experience: &ShellExperienceId) -> Option<&GameplaySessionProfile> {
        self.experiences.get(experience)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ShellExperienceId> {
        self.experiences.keys()
    }
}

/// Canonical identity of the one active top-level gameplay session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameplaySessionInstance {
    pub activation: ActiveShellExperience,
    pub scope: SessionScopeId,
    /// The load barrier that authorized this activation, when its route
    /// required one. Captured at activation so acceptance checks can prove no
    /// stale load transaction outlives the session it prepared.
    pub load: Option<LoadBarrierRef>,
}

/// App-local gameplay-session authority. It is `None` at launchers, credits,
/// startup sequences, and other non-gameplay shell experiences.
#[derive(Resource, Default, Debug)]
pub struct ActiveGameplaySession(pub Option<GameplaySessionInstance>);

/// Exact shell-activation to session-scope bindings.
#[derive(Resource, Default)]
pub struct GameplaySessionLinks {
    bindings: Vec<(ShellActivationId, SessionScopeId)>,
}

impl GameplaySessionLinks {
    pub fn scope_for(&self, activation: ShellActivationId) -> Option<SessionScopeId> {
        self.bindings
            .iter()
            .find_map(|(candidate, scope)| (*candidate == activation).then_some(*scope))
    }

    fn bind(&mut self, activation: ShellActivationId, scope: SessionScopeId) {
        assert!(
            self.scope_for(activation).is_none(),
            "shell activation {activation:?} already owns a gameplay session"
        );
        self.bindings.push((activation, scope));
    }

    fn unbind(&mut self, activation: ShellActivationId) -> Option<SessionScopeId> {
        let index = self
            .bindings
            .iter()
            .position(|(candidate, _)| *candidate == activation)?;
        Some(self.bindings.remove(index).1)
    }
}

/// Stable schedule seams for the bridge and game-specific session construction.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GameplaySessionSet {
    /// Translate shell route lifecycle into session lifecycle facts.
    Bridge,
    /// Provider systems construct or retire game-specific state here.
    Providers,
}

/// App-build extension for a provider whose route owns a gameplay session.
pub trait GameplaySessionAppExt {
    fn register_gameplay_experience(
        &mut self,
        registration: ExperienceRegistration,
        route: ShellRouteSpec,
    ) -> &mut Self;
}

impl GameplaySessionAppExt for App {
    fn register_gameplay_experience(
        &mut self,
        registration: ExperienceRegistration,
        route: ShellRouteSpec,
    ) -> &mut Self {
        let experience = registration.id.clone();
        self.register_experience(registration, route);
        self.world_mut()
            .get_resource_or_insert_with(GameplaySessionRegistry::default)
            .register(experience);
        self
    }
}

/// Installs the engine-neutral session scope and maps registered shell routes to
/// it. Add once per host; all providers share it.
pub struct GameplaySessionBridgePlugin;

impl Plugin for GameplaySessionBridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SessionScopePlugin)
            // Opt this App into session-gated simulation: composing the bridge
            // IS the declaration that gameplay belongs to shell-routed
            // sessions, so the gameplay-simulation root sleeps whenever no
            // session scope is live (launcher/title/loading frames).
            .init_resource::<SessionGatedSimulation>()
            .init_resource::<GameplaySessionRegistry>()
            .init_resource::<GameplaySessionLinks>()
            .init_resource::<ActiveGameplaySession>()
            .init_resource::<ActiveAudioSelection>()
            .add_message::<GameplaySessionEvent>()
            .configure_sets(
                Update,
                (GameplaySessionSet::Bridge, GameplaySessionSet::Providers)
                    .chain()
                    .after(AmbitionGameShellSet::Pending)
                    .before(SessionScopeSet::Presentation),
            )
            .add_systems(
                Update,
                (
                    translate_shell_session_lifecycle,
                    select_session_audio_authority,
                )
                    .chain()
                    .in_set(GameplaySessionSet::Bridge),
            );
    }
}

/// Derive the active audio authority from gameplay-session lifecycle.
///
/// Activation selects the session profile's audio provider (defaulting to the
/// experience id) out of the App-local [`AudioCatalogRegistry`]; a provider
/// that registered no fragments gets a DELIBERATE empty authority, never a
/// fallback to another provider's audio. Retirement clears playback authority
/// — cached assets may outlive the session, the selection does not.
///
/// Chained directly after [`translate_shell_session_lifecycle`], so providers
/// in [`GameplaySessionSet::Providers`] already observe the new selection on
/// the activation frame.
fn select_session_audio_authority(
    mut sessions: MessageReader<GameplaySessionEvent>,
    registry: Res<GameplaySessionRegistry>,
    catalogs: Option<Res<AudioCatalogRegistry>>,
    mut selection: ResMut<ActiveAudioSelection>,
) {
    for event in sessions.read() {
        match event {
            GameplaySessionEvent::Activated { activation, .. } => {
                let provider = registry
                    .profile(&activation.experience_id)
                    .and_then(|profile| profile.audio_provider.clone())
                    .unwrap_or_else(|| activation.experience_id.as_str().to_owned());
                let (music, sfx) = catalogs
                    .as_ref()
                    .map(|catalogs| {
                        (
                            catalogs.music_for(&provider).cloned(),
                            catalogs.sfx_for(&provider).cloned(),
                        )
                    })
                    .unwrap_or((None, None));
                selection.select(provider, music, sfx);
            }
            GameplaySessionEvent::Retiring { .. } => {
                selection.clear();
            }
        }
    }
}

fn translate_shell_session_lifecycle(
    mut shell_events: MessageReader<ShellEvent>,
    registry: Res<GameplaySessionRegistry>,
    routes: Res<ShellRouteCatalog>,
    mut active_scope: ResMut<ActiveSessionScope>,
    mut links: ResMut<GameplaySessionLinks>,
    mut active_session: ResMut<ActiveGameplaySession>,
    mut session_events: MessageWriter<GameplaySessionEvent>,
    mut retired: MessageWriter<SessionScopeRetired>,
) {
    for event in shell_events.read() {
        match event {
            ShellEvent::RouteDeactivated(activation) => {
                if let Some(scope) = links.unbind(activation.activation_id) {
                    if active_session.0.as_ref().is_some_and(|session| {
                        session.activation.activation_id == activation.activation_id
                    }) {
                        active_session.0 = None;
                    }
                    // Revoke spawn authority immediately. Exact entity cleanup
                    // remains deferred to `SessionScopeSet::Cleanup`, but systems
                    // later in this frame must not author new work into a retired
                    // activation. `clear_if_current` also preserves a newer scope
                    // when a gameplay-to-gameplay replacement occurs in one frame.
                    active_scope.clear_if_current(scope);
                    session_events.write(GameplaySessionEvent::Retiring {
                        activation: activation.clone(),
                        scope,
                    });
                    retired.write(SessionScopeRetired(scope));
                }
            }
            ShellEvent::RouteActivated(activation)
                if registry.contains(&activation.experience_id) =>
            {
                assert!(
                    active_session.0.is_none(),
                    "activating gameplay session {:?} while {:?} is still active",
                    activation.activation_id,
                    active_session
                        .0
                        .as_ref()
                        .map(|session| session.activation.activation_id),
                );
                let scope = active_scope.begin();
                links.bind(activation.activation_id, scope);
                active_session.0 = Some(GameplaySessionInstance {
                    activation: activation.clone(),
                    scope,
                    load: routes
                        .get(&activation.route_id)
                        .and_then(|route| route.required_barrier.clone()),
                });
                session_events.write(GameplaySessionEvent::Activated {
                    activation: activation.clone(),
                    scope,
                });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AmbitionGameShellPlugin, ShellCommand, ShellHostConfiguration, ShellHostSpec,
        ShellRouteCatalog, ShellRouter,
    };
    use ambition_platformer_primitives::lifecycle::{
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
                commands.spawn_session_scoped(
                    SessionSpawnScope::scoped(scope),
                    Name::new("session-owned"),
                );
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
        // Only `test_game` registers audio; `silent_game` deliberately none.
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
                        }],
                    }),
                    None,
                )
                .unwrap(),
            )
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
            .write_message(ShellCommand::ReplaceWith(SILENT_ROUTE.into()));
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

    /// W0: the canonical session instance captures the load barrier that
    /// authorized its activation.
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
}
