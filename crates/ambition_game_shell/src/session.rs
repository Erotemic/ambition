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
            // Required, never Option: session-audio composition must fail loudly
            // if the host was built without an audio system rather than treating
            // a missing registry as "everyone is silent."
            .init_resource::<AudioCatalogRegistry>()
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
    catalogs: Res<AudioCatalogRegistry>,
    mut selection: ResMut<ActiveAudioSelection>,
) {
    for event in sessions.read() {
        match event {
            GameplaySessionEvent::Activated { activation, scope } => {
                let provider = registry
                    .profile(&activation.experience_id)
                    .and_then(|profile| profile.audio_provider.clone())
                    .unwrap_or_else(|| activation.experience_id.as_str().to_owned());
                // The registry is REQUIRED (never Option) so a host composed
                // without an audio system cannot be silently mistaken for "every
                // provider is deliberately silent." A gameplay provider must
                // register a fragment — an explicitly-empty one for silence — so
                // absence is a real composition error, not inferred quiet.
                assert!(
                    catalogs.has_provider(&provider),
                    "gameplay provider '{provider}' activated a session but registered no \
                     audio catalog fragment; register one (empty music/SFX for deliberate \
                     silence) so composition is never mistaken for silence",
                );
                let music = catalogs.music_for(&provider).cloned();
                let sfx = catalogs.sfx_for(&provider).cloned();
                // Tag the selection with THIS session's scope token so a delayed
                // retirement for an older session cannot silence it.
                selection.select(Some(scope.0), provider, music, sfx);
            }
            GameplaySessionEvent::Retiring { scope, .. } => {
                // Clear ONLY if this exact session still owns the selection.
                selection.clear_if_owner(scope.0);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)] // Bevy system: each param is one authority
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
mod tests;
