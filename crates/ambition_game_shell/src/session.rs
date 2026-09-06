//! Shell-to-gameplay-session lifecycle bridge.
//!
//! The shell owns route activation; platformer simulation owns session-scoped
//! entities. This module is the narrow adapter between those authorities. A
//! provider registers an experience as a gameplay session, then receives a
//! fresh engine-neutral [`SessionScopeId`] whenever that experience activates.
//! Route retirement emits the matching session-retirement signal exactly once.

use std::collections::BTreeMap;

use ambition_audio::catalog::{AudioCatalogRegistry, SfxBankRegistry};
use ambition_audio::selection::{ActiveAudioSelection, AudioContextChanged, FrontendAudioRegistry};
use ambition_load::LoadBarrierRef;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionGatedSimulation, SessionRoot, SessionScopeId, SessionScopePlugin,
    SessionScopeActivated, SessionScopeRetired, SessionScopeSet, SpawnSessionScopedExt,
};
use ambition_platformer2d_shared_tangle::schedule::GameMode;
use ambition_sfx::{AudioContextOwner, SfxEmissionContext};
use bevy::prelude::*;

use crate::{
    ActiveFrontendAuthority, ActiveShellExperience, AmbitionGameShellSet, ExperienceRegistration,
    PreparedSessionIdentity, PresentationOwnershipPolicy, ShellActivationId, ShellEvent,
    ShellExperienceAppExt, ShellExperienceId, ShellRouteSpec,
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

/// Exact gameplay audio authority captured with one session activation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameplaySessionAudioContext {
    pub owner: AudioContextOwner,
    pub provider_id: String,
}

/// Marker and exact owner facts on the canonical live gameplay-world entity.
///
/// The shell deliberately does not name a provider's concrete world bundle.
/// Providers attach their typed components to this entity, while the shell
/// owns only the exact activation/scope identity and lifetime.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct GameplayInputOwner {
    pub activation_id: ShellActivationId,
    pub scope: SessionScopeId,
}

#[derive(Component, Clone, Debug, Eq, PartialEq)]
pub struct GameplaySessionWorldRoot {
    pub activation_id: ShellActivationId,
    pub experience_id: ShellExperienceId,
    pub scope: SessionScopeId,
    pub audio: GameplaySessionAudioContext,
    pub load: Option<LoadBarrierRef>,
    pub prepared: Option<PreparedSessionIdentity>,
}

/// Canonical identity of the one active top-level gameplay session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameplaySessionInstance {
    pub activation: ActiveShellExperience,
    pub scope: SessionScopeId,
    pub load: Option<LoadBarrierRef>,
    pub prepared: Option<PreparedSessionIdentity>,
    pub audio: GameplaySessionAudioContext,
    /// Canonical live gameplay-world entity. `None` only during the provider
    /// phase of a fresh activation.
    pub world: Option<Entity>,
}

#[cfg(test)]
impl GameplaySessionInstance {
    /// A minimal live-session instance for tests that only need
    /// [`ActiveGameplaySession`] to read as "a session is live" (its
    /// `.0.is_some()`); the field values are inert placeholders.
    pub(crate) fn stub_live() -> Self {
        Self {
            activation: ActiveShellExperience {
                activation_id: ShellActivationId(1),
                route_id: crate::ShellRouteId::new("test_route"),
                experience_id: ShellExperienceId::new("test_experience"),
                parameters: BTreeMap::new(),
                load_authorization: None,
                prepared_session: None,
            },
            scope: SessionScopeId(1),
            load: None,
            prepared: None,
            audio: GameplaySessionAudioContext {
                owner: AudioContextOwner::Gameplay(1),
                provider_id: "test".to_string(),
            },
            world: None,
        }
    }
}

/// App-local gameplay-session authority. It is `None` at launchers, credits,
/// startup sequences, and other non-gameplay shell experiences.
#[derive(Resource, Default, Debug)]
pub struct ActiveGameplaySession(pub Option<GameplaySessionInstance>);

impl ActiveGameplaySession {
    /// Spawn the canonical provider world only for the exact live activation.
    pub fn spawn_world_for<B: Bundle>(
        &mut self,
        commands: &mut Commands,
        activation: &ActiveShellExperience,
        scope: SessionScopeId,
        world: B,
    ) -> Option<Entity> {
        let instance = self.0.as_mut()?;
        if instance.activation.activation_id != activation.activation_id
            || instance.activation.experience_id != activation.experience_id
            || instance.scope != scope
            || instance.world.is_some()
        {
            return None;
        }

        let entity = commands
            .spawn_in_session(
                scope,
                (
                    Name::new(format!(
                        "{} gameplay session world",
                        activation.experience_id.as_str()
                    )),
                    // The root is rollback-anchored (it carries the room set)
                    // and there is one per activation: a derived identity, so
                    // the identity census admits no waiver (S4).
                    ambition_platformer2d_shared_tangle::sim_id::SimId::singleton(
                        "session",
                        &activation.activation_id.0.to_string(),
                    ),
                    SessionRoot(scope),
                    GameplaySessionWorldRoot {
                        activation_id: activation.activation_id,
                        experience_id: activation.experience_id.clone(),
                        scope,
                        audio: instance.audio.clone(),
                        load: instance.load.clone(),
                        prepared: instance.prepared.clone(),
                    },
                    world,
                ),
            )
            .id();
        instance.world = Some(entity);
        Some(entity)
    }

    /// Retire only the exact activation. Delayed retirement for A cannot
    /// disturb B, including a same-provider relaunch.
    pub fn retire_if_activation(
        &mut self,
        activation_id: ShellActivationId,
    ) -> Option<GameplaySessionInstance> {
        if self
            .0
            .as_ref()
            .is_none_or(|instance| instance.activation.activation_id != activation_id)
        {
            return None;
        }
        self.0.take()
    }

    pub fn active_world_entity(&self) -> Option<Entity> {
        self.0.as_ref().and_then(|instance| instance.world)
    }
}

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
            .init_resource::<ActiveFrontendAuthority>()
            .init_resource::<PresentationOwnershipPolicy>()
            .init_resource::<ActiveAudioSelection>()
            .init_resource::<SfxEmissionContext>()
            // Required, never Option: session-audio composition must fail loudly
            // if the host was built without an audio system rather than treating
            // a missing registry as "everyone is silent."
            .init_resource::<AudioCatalogRegistry>()
            // The bank-id index defaults empty (a host may ship no SFX bank);
            // providers that contribute a bank register their ids here so a
            // session's SFX authority spans its cues AND its bank content.
            .init_resource::<SfxBankRegistry>()
            .add_message::<GameplaySessionEvent>()
            .add_message::<AudioContextChanged>()
            .configure_sets(
                Update,
                // ⛔ `SessionScopeSet::Activate` sits BETWEEN them, not beside
                // them: the bridge announces the new scope, the scope's own
                // state is re-established, and only then does a provider build
                // the world that will read it.
                (
                    GameplaySessionSet::Bridge,
                    SessionScopeSet::Activate,
                    GameplaySessionSet::Providers,
                )
                    .chain()
                    .after(AmbitionGameShellSet::Pending)
                    .before(SessionScopeSet::Presentation),
            )
            .add_systems(
                Update,
                (
                    translate_shell_session_lifecycle,
                    select_shell_audio_context,
                    select_frontend_authority,
                )
                    .chain()
                    .in_set(GameplaySessionSet::Bridge),
            );
    }
}

fn select_frontend_authority(
    mut events: MessageReader<ShellEvent>,
    registry: Res<GameplaySessionRegistry>,
    mut authority: ResMut<ActiveFrontendAuthority>,
) {
    for event in events.read() {
        match event {
            ShellEvent::RouteActivated(active) => {
                authority.0 = (!registry.contains(&active.experience_id)).then_some(active.clone());
            }
            ShellEvent::RouteDeactivated(active)
                if authority
                    .0
                    .as_ref()
                    .is_some_and(|current| current.activation_id == active.activation_id) =>
            {
                authority.0 = None;
            }
            ShellEvent::ExitRequested => authority.0 = None,
            _ => {}
        }
    }
}

/// Derive the active audio authority from gameplay-session lifecycle.
///
/// Activation selects the session profile's audio provider (defaulting to the
/// experience id) out of the App-local [`AudioCatalogRegistry`]; a provider
/// that registered no fragment is a composition error; deliberate silence is
/// represented by an explicit empty fragment. Retirement clears playback authority
/// — cached assets may outlive the session, the selection does not.
///
/// Chained directly after [`translate_shell_session_lifecycle`], so providers
/// in [`GameplaySessionSet::Providers`] already observe the new selection on
/// the activation frame.
fn select_shell_audio_context(
    mut sessions: MessageReader<GameplaySessionEvent>,
    mut shell_events: MessageReader<ShellEvent>,
    registry: Res<GameplaySessionRegistry>,
    active_session: Res<ActiveGameplaySession>,
    catalogs: Res<AudioCatalogRegistry>,
    sfx_banks: Res<SfxBankRegistry>,
    mut frontend: Option<ResMut<FrontendAudioRegistry>>,
    mut selection: ResMut<ActiveAudioSelection>,
    mut emission: ResMut<SfxEmissionContext>,
    mut context_changes: MessageWriter<AudioContextChanged>,
) {
    // Gameplay lifecycle owns gameplay audio contexts.
    for event in sessions.read() {
        match event {
            GameplaySessionEvent::Activated { activation, scope } => {
                let instance = active_session
                    .0
                    .as_ref()
                    .filter(|instance| {
                        instance.activation.activation_id == activation.activation_id
                            && instance.scope == *scope
                    })
                    .expect("session activation publishes exact audio identity first");
                let provider = instance.audio.provider_id.as_str();
                assert!(
                    catalogs.has_provider(provider),
                    "gameplay provider '{provider}' activated a session but registered no audio catalog fragment; register an explicit empty fragment for silence",
                );
                let previous = selection.owner();
                selection.select_gameplay(
                    scope.0,
                    provider.to_owned(),
                    catalogs.music_for(provider).cloned(),
                    catalogs.sfx_for(provider).cloned(),
                    sfx_banks.ids_for(provider),
                );
                emission.set(instance.audio.owner, provider.to_owned());
                let current = selection.owner();
                if previous != current {
                    context_changes.write(AudioContextChanged { previous, current });
                }
            }
            GameplaySessionEvent::Retiring { scope, .. } => {
                let owner = AudioContextOwner::Gameplay(scope.0);
                let previous = selection.owner();
                selection.clear_if_owner(owner);
                emission.clear_if(owner);
                let current = selection.owner();
                if previous != current {
                    context_changes.write(AudioContextChanged { previous, current });
                }
            }
        }
    }

    // Plain shell routes (startup, launcher, loading, credits, a provider's own
    // character select) each get the frontend profile DECLARED FOR THAT ROUTE,
    // falling back to the host's default. They are not "ungoverned": menu SFX
    // and title music are authorized by the exact shell activation that emitted
    // them, while stale gameplay requests remain invalid.
    //
    // `activation.route_id` was in scope and unread.
    for event in shell_events.read() {
        match event {
            ShellEvent::RouteActivated(activation)
                if !registry.contains(&activation.experience_id) =>
            {
                let owner = AudioContextOwner::Frontend(activation.activation_id.0);
                let previous = selection.owner();
                if let Some(frontend) = frontend
                    .as_mut()
                    .and_then(|frontend| frontend.enter_route(activation.route_id.as_str()))
                {
                    let provider = frontend.provider_id();
                    emission.set(owner, provider.to_owned());
                    assert!(
                        catalogs.has_provider(provider),
                        "route '{}' declares frontend audio from provider '{provider}', which \
                         registered no audio fragment; register an explicit empty fragment for \
                         silence",
                        activation.route_id.as_str(),
                    );
                    selection.select_frontend(
                        activation.activation_id.0,
                        frontend,
                        catalogs.music_for(provider).cloned(),
                        catalogs.sfx_for(provider).cloned(),
                        sfx_banks.ids_for(provider),
                    );
                } else {
                    selection.clear();
                    emission.clear();
                }
                let current = selection.owner();
                if previous != current {
                    context_changes.write(AudioContextChanged { previous, current });
                }
            }
            ShellEvent::RouteDeactivated(activation)
                if !registry.contains(&activation.experience_id) =>
            {
                let owner = AudioContextOwner::Frontend(activation.activation_id.0);
                let previous = selection.owner();
                selection.clear_if_owner(owner);
                emission.clear_if(owner);
                let current = selection.owner();
                if previous != current {
                    context_changes.write(AudioContextChanged { previous, current });
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)] // Bevy system: each param is one authority
fn translate_shell_session_lifecycle(
    mut shell_events: MessageReader<ShellEvent>,
    registry: Res<GameplaySessionRegistry>,
    mut active_scope: ResMut<ActiveSessionScope>,
    mut links: ResMut<GameplaySessionLinks>,
    mut active_session: ResMut<ActiveGameplaySession>,
    mut loads: ResMut<ambition_load::LoadCoordinator>,
    mut session_events: MessageWriter<GameplaySessionEvent>,
    mut retired: MessageWriter<SessionScopeRetired>,
    mut activated: MessageWriter<SessionScopeActivated>,
    mut game_mode: Option<ResMut<NextState<GameMode>>>,
) {
    for event in shell_events.read() {
        match event {
            ShellEvent::RouteDeactivated(activation) => {
                if let Some(scope) = links.unbind(activation.activation_id) {
                    let retired_session =
                        active_session.retire_if_activation(activation.activation_id);
                    if let Some(load) = retired_session
                        .as_ref()
                        .and_then(|session| session.load.as_ref())
                    {
                        loads.retire(&load.load_id);
                    }
                    // `GameMode` is a Bevy `States` global, and pausing is the one thing that
                    // writes it from OUTSIDE the session's own systems. Quit to the title from a
                    // paused match and the mode stayed `Paused` with no session to explain it: the
                    // next match built its fighters, seated them, framed them and never advanced a
                    // tick — bodies hanging in the air with a menu that still answered.
                    //
                    // the pause menu already handed the sim back on its way
                    // out, and that is exactly the problem: `QuitToHome` has
                    // four writers (the pause menu, the F10 developer hotkey, the
                    // in-world system menu, the scripted route sweep) and only
                    // one of them remembered. A rule every caller must obey is a
                    // rule three callers will eventually break. The lifecycle
                    // that ended the session is the one place that cannot forget.
                    //
                    // `Dialogue`, `RoomTransition` and `Cutscene` reset too,
                    // for the same reason — every one of them describes a live
                    // world, and this one has just been retired.
                    if let Some(mode) = game_mode.as_mut() {
                        ambition_platformer2d_shared_tangle::world_log::note_game_mode_request(
                            GameMode::default(),
                            "session_retire",
                        );
                        mode.set(GameMode::default());
                    }
                    active_scope.clear_if_current(scope);
                    // Session lifetime was logged NOWHERE: the word "session"
                    // matched zero lines across a 258-second desktop capture.
                    // This system is the only translator from shell routing to
                    // gameplay-session lifetime, so both edges are marked here.
                    ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
                        "session-end experience={} activation={:?} scope={}",
                        activation.experience_id.as_str(),
                        activation.activation_id,
                        scope.0
                    ));
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
                // ⭐ BOTH EDGES ARE ANNOUNCED HERE. This is the one translator
                // from shell routing to gameplay-session lifetime, so it is the
                // one place that can say a session has begun before anything
                // has been built for it — which is what lets session-scoped
                // process globals be re-established by their owner rather than
                // by a teardown somebody has to remember.
                activated.write(SessionScopeActivated(scope));
                links.bind(activation.activation_id, scope);
                let audio_provider = registry
                    .profile(&activation.experience_id)
                    .and_then(|profile| profile.audio_provider.clone())
                    .unwrap_or_else(|| activation.experience_id.as_str().to_owned());
                active_session.0 = Some(GameplaySessionInstance {
                    activation: activation.clone(),
                    scope,
                    load: activation.load_authorization.clone(),
                    prepared: activation.prepared_session.clone(),
                    audio: GameplaySessionAudioContext {
                        owner: AudioContextOwner::Gameplay(scope.0),
                        provider_id: audio_provider,
                    },
                    world: None,
                });
                ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
                    "session-start experience={} activation={:?} scope={}",
                    activation.experience_id.as_str(),
                    activation.activation_id,
                    scope.0
                ));
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
