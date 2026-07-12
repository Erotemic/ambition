//! Shell-to-gameplay-session lifecycle bridge.
//!
//! The shell owns route activation; platformer simulation owns session-scoped
//! entities. This module is the narrow adapter between those authorities. A
//! provider registers an experience as a gameplay session, then receives a
//! fresh engine-neutral [`SessionScopeId`] whenever that experience activates.
//! Route retirement emits the matching session-retirement signal exactly once.

use std::collections::BTreeSet;

use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionScopeId, SessionScopePlugin, SessionScopeRetired, SessionScopeSet,
};
use bevy::prelude::*;

use crate::{
    ActiveShellExperience, AmbitionGameShellSet, ExperienceRegistration, ShellActivationId,
    ShellEvent, ShellExperienceAppExt, ShellExperienceId, ShellRouteSpec,
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

/// Deterministic registry of experiences whose routes own gameplay sessions.
#[derive(Resource, Default)]
pub struct GameplaySessionRegistry {
    experiences: BTreeSet<ShellExperienceId>,
}

impl GameplaySessionRegistry {
    pub fn register(&mut self, experience: ShellExperienceId) -> bool {
        self.experiences.insert(experience)
    }

    pub fn contains(&self, experience: &ShellExperienceId) -> bool {
        self.experiences.contains(experience)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ShellExperienceId> {
        self.experiences.iter()
    }
}

/// Canonical identity of the one active top-level gameplay session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameplaySessionInstance {
    pub activation: ActiveShellExperience,
    pub scope: SessionScopeId,
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
            .init_resource::<GameplaySessionRegistry>()
            .init_resource::<GameplaySessionLinks>()
            .init_resource::<ActiveGameplaySession>()
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
                translate_shell_session_lifecycle.in_set(GameplaySessionSet::Bridge),
            );
    }
}

fn translate_shell_session_lifecycle(
    mut shell_events: MessageReader<ShellEvent>,
    registry: Res<GameplaySessionRegistry>,
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
