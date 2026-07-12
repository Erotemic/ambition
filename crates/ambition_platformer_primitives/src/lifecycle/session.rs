//! **Session scope** — activation-owned entity lifetime.
//!
//! A *session* is one activated gameplay experience: a launched Sanic run, a
//! launched Mary-O run, or the main game entered from a launcher. Every entity
//! created on behalf of that activation belongs to its [`SessionScopeId`] and is
//! retired with it.
//!
//! Session lifetime is distinct from [`super::ModeScopedEntity`]. Consecutive
//! runs may share a mode while requiring completely fresh runtime ownership.
//! Session identity is therefore minted once per activation and propagated
//! explicitly through [`SessionSpawnScope`] at the moment spawn work is
//! requested. A later route change cannot reassign a deferred spawn to another
//! activation.
//!
//! This abstraction sits below the game shell. Route providers map shell
//! activations to session scopes, while simulation and world-construction code
//! use the scope without importing shell vocabulary.

use std::ops::{Deref, DerefMut};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::markers::RoomScopedEntity;

/// Stable identity of one activated gameplay session.
///
/// Minted from a deterministic monotonic counter so the same activation order
/// produces the same identities in replay and tests.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionScopeId(pub u64);

/// The currently active gameplay-session scope and its deterministic allocator.
#[derive(Resource, Default, Debug)]
pub struct ActiveSessionScope {
    current: Option<SessionScopeId>,
    next_raw: u64,
}

impl ActiveSessionScope {
    /// Mint a fresh scope, make it current, and return it.
    pub fn begin(&mut self) -> SessionScopeId {
        let id = SessionScopeId(self.next_raw);
        self.next_raw += 1;
        self.current = Some(id);
        id
    }

    /// The active scope, when gameplay currently owns a session.
    pub fn current(&self) -> Option<SessionScopeId> {
        self.current
    }

    /// Capture the current scope for spawn work requested now.
    pub fn spawn_scope(&self) -> SessionSpawnScope {
        SessionSpawnScope::new(self.current)
    }

    /// Clear the active scope unconditionally.
    pub fn clear(&mut self) {
        self.current = None;
    }

    /// Clear only when `id` is still current. Retiring A after B activated must
    /// not clear B's spawn context.
    pub fn clear_if_current(&mut self, id: SessionScopeId) {
        if self.current == Some(id) {
            self.current = None;
        }
    }
}

/// A captured entity-ownership context.
///
/// The value is copied into spawn commands when work is requested. It never
/// consults [`ActiveSessionScope`] during command application, so deferred work
/// remains attached to the activation that authored it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionSpawnScope {
    id: Option<SessionScopeId>,
}

impl SessionSpawnScope {
    /// Process-/frontend-resident work with no gameplay-session owner.
    pub const UNSCOPED: Self = Self { id: None };

    /// Capture an explicit gameplay-session owner.
    pub const fn scoped(id: SessionScopeId) -> Self {
        Self { id: Some(id) }
    }

    /// Construct from an optional scope.
    pub const fn new(id: Option<SessionScopeId>) -> Self {
        Self { id }
    }

    /// The captured owner.
    pub const fn id(self) -> Option<SessionScopeId> {
        self.id
    }

    /// Resolve the spawn policy for a system that supports both legacy apps and
    /// session-aware shell hosts.
    ///
    /// An absent [`ActiveSessionScope`] resource means the app has not installed
    /// session lifecycle and therefore uses process-resident legacy spawning.
    /// A present resource with no current scope means the shell is at a
    /// non-gameplay experience, so gameplay-owned spawning sleeps.
    pub fn for_optional_active_session(active: Option<&ActiveSessionScope>) -> Option<Self> {
        match active {
            None => Some(Self::UNSCOPED),
            Some(active) => active.current().map(Self::scoped),
        }
    }

    /// Attach this ownership context to an already-created entity command.
    pub fn apply_to(self, entity: &mut EntityCommands<'_>) {
        if let Some(id) = self.id {
            entity.insert(SessionScopedEntity(id));
        }
    }
}

impl From<SessionScopeId> for SessionSpawnScope {
    fn from(id: SessionScopeId) -> Self {
        Self::scoped(id)
    }
}

/// A single Bevy system parameter carrying entity commands and the session
/// ownership captured for work requested by that system invocation.
///
/// Besides making the intended spawn context explicit, this keeps large
/// gameplay systems within Bevy's supported system-parameter arity: replacing
/// separate `Commands` and `Option<Res<ActiveSessionScope>>` parameters with
/// `SessionCommands` consumes one parameter slot.
#[derive(SystemParam)]
pub struct SessionCommands<'w, 's> {
    commands: Commands<'w, 's>,
    active: Option<Res<'w, ActiveSessionScope>>,
}

impl SessionCommands<'_, '_> {
    /// Resolve the captured spawn policy for this system invocation.
    ///
    /// Legacy apps without [`SessionScopePlugin`] receive an unscoped command
    /// context. Shell hosts at a non-gameplay route receive `None`, allowing
    /// gameplay-owned systems to sleep rather than author frontend entities.
    pub fn spawn_scope(&self) -> Option<SessionSpawnScope> {
        SessionSpawnScope::for_optional_active_session(self.active.as_deref())
    }
}

impl<'w, 's> Deref for SessionCommands<'w, 's> {
    type Target = Commands<'w, 's>;

    fn deref(&self) -> &Self::Target {
        &self.commands
    }
}

impl<'w, 's> DerefMut for SessionCommands<'w, 's> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.commands
    }
}

/// Tag carried by every entity owned by a gameplay-session activation.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionScopedEntity(pub SessionScopeId);

/// Marker on the canonical root entity for a gameplay session.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionRoot(pub SessionScopeId);

/// Signal that a session scope has retired.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionScopeRetired(pub SessionScopeId);

/// Stable schedule seam for exact scope retirement.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SessionScopeSet {
    /// Presentation systems may materialize activation-owned visuals after the
    /// provider has published its session world.
    Presentation,
    /// Exact retirement of entities owned by the retired session.
    Cleanup,
}

/// `Commands` extensions that make captured session ownership explicit at each
/// spawn site.
pub trait SpawnSessionScopedExt {
    /// Spawn with the captured session owner. [`SessionSpawnScope::UNSCOPED`]
    /// deliberately creates process-/frontend-resident state.
    fn spawn_session_scoped<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        bundle: B,
    ) -> EntityCommands<'_>;

    /// Spawn with one explicit session identity.
    fn spawn_in_session<B: Bundle>(
        &mut self,
        scope: SessionScopeId,
        bundle: B,
    ) -> EntityCommands<'_>;

    /// Spawn an entity owned by both the active authored room and the captured
    /// gameplay session.
    fn spawn_room_in_session<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        bundle: B,
    ) -> EntityCommands<'_>;
}

impl SpawnSessionScopedExt for Commands<'_, '_> {
    fn spawn_session_scoped<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        bundle: B,
    ) -> EntityCommands<'_> {
        let mut entity = self.spawn(bundle);
        scope.apply_to(&mut entity);
        entity
    }

    fn spawn_in_session<B: Bundle>(
        &mut self,
        scope: SessionScopeId,
        bundle: B,
    ) -> EntityCommands<'_> {
        self.spawn_session_scoped(SessionSpawnScope::scoped(scope), bundle)
    }

    fn spawn_room_in_session<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        bundle: B,
    ) -> EntityCommands<'_> {
        let mut entity = self.spawn((RoomScopedEntity, bundle));
        scope.apply_to(&mut entity);
        entity
    }
}

/// Despawn every entity owned by a retired scope and clear the current pointer
/// when it still names that scope.
pub fn despawn_retired_session_entities(
    mut commands: Commands,
    mut retired: MessageReader<SessionScopeRetired>,
    mut active: ResMut<ActiveSessionScope>,
    scoped: Query<(Entity, &SessionScopedEntity)>,
) {
    for SessionScopeRetired(scope) in retired.read().copied() {
        for (entity, owner) in &scoped {
            if owner.0 == scope {
                commands.entity(entity).despawn();
            }
        }
        active.clear_if_current(scope);
    }
}

/// Installs session identity, retirement messages, and exact cleanup.
pub struct SessionScopePlugin;

impl Plugin for SessionScopePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveSessionScope>()
            .add_message::<SessionScopeRetired>()
            .configure_sets(
                Update,
                (SessionScopeSet::Presentation, SessionScopeSet::Cleanup).chain(),
            )
            .add_systems(
                Update,
                despawn_retired_session_entities.in_set(SessionScopeSet::Cleanup),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn session_app() -> App {
        let mut app = App::new();
        app.add_plugins(SessionScopePlugin);
        app
    }

    fn count_scoped(app: &mut App, scope: SessionScopeId) -> usize {
        let mut query = app.world_mut().query::<&SessionScopedEntity>();
        query
            .iter(app.world())
            .filter(|owner| owner.0 == scope)
            .count()
    }

    fn nested_spawn_helper(commands: &mut Commands, scope: SessionSpawnScope) {
        commands.spawn_session_scoped(scope, Name::new("nested-child"));
    }

    #[test]
    fn direct_and_nested_spawns_share_the_captured_scope() {
        let mut app = session_app();
        let scope = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        let spawn_scope = app.world().resource::<ActiveSessionScope>().spawn_scope();

        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                commands.spawn_session_scoped(spawn_scope, Name::new("root"));
                commands.spawn_session_scoped(spawn_scope, Name::new("sibling"));
                nested_spawn_helper(&mut commands, spawn_scope);
            })
            .unwrap();

        assert_eq!(count_scoped(&mut app, scope), 3);
    }

    #[test]
    fn captured_scope_survives_a_later_ambient_change() {
        let mut app = session_app();
        let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        let captured_a = app.world().resource::<ActiveSessionScope>().spawn_scope();
        let b = app.world_mut().resource_mut::<ActiveSessionScope>().begin();

        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                commands.spawn_session_scoped(captured_a, Name::new("late-a"));
            })
            .unwrap();

        assert_eq!(count_scoped(&mut app, a), 1);
        assert_eq!(count_scoped(&mut app, b), 0);
        assert_eq!(
            app.world().resource::<ActiveSessionScope>().current(),
            Some(b)
        );
    }

    #[test]
    fn one_command_queue_preserves_multiple_captured_owners() {
        let mut app = session_app();
        let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        let captured_a = app.world().resource::<ActiveSessionScope>().spawn_scope();
        let b = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        let captured_b = app.world().resource::<ActiveSessionScope>().spawn_scope();

        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                commands.spawn_session_scoped(captured_a, Name::new("queued-a"));
                commands.spawn_session_scoped(captured_b, Name::new("queued-b"));
            })
            .unwrap();

        assert_eq!(count_scoped(&mut app, a), 1);
        assert_eq!(count_scoped(&mut app, b), 1);
    }

    #[test]
    fn room_session_spawn_carries_both_lifetimes() {
        let mut app = session_app();
        let scope = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                commands.spawn_room_in_session(scope.into(), Name::new("room-feature"));
            })
            .unwrap();

        let mut query = app
            .world_mut()
            .query::<(&RoomScopedEntity, &SessionScopedEntity)>();
        let owners: Vec<_> = query.iter(app.world()).collect();
        assert_eq!(owners.len(), 1);
        assert_eq!(owners[0].1 .0, scope);
    }

    #[test]
    fn begin_mints_distinct_ids() {
        let mut app = session_app();
        let mut active = app.world_mut().resource_mut::<ActiveSessionScope>();
        let a = active.begin();
        active.clear();
        let b = active.begin();
        let c = active.begin();
        assert_eq!((a.0, b.0, c.0), (0, 1, 2));
    }

    #[test]
    fn retiring_one_scope_is_exact() {
        let mut app = session_app();
        let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                commands.spawn_in_session(a, Name::new("a1"));
                commands.spawn_in_session(a, Name::new("a2"));
            })
            .unwrap();
        let b = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                commands.spawn_in_session(b, Name::new("b1"));
            })
            .unwrap();

        app.world_mut().write_message(SessionScopeRetired(a));
        app.update();

        assert_eq!(count_scoped(&mut app, a), 0);
        assert_eq!(count_scoped(&mut app, b), 1);
    }

    #[test]
    fn retiring_a_stale_scope_leaves_the_live_one_active() {
        let mut app = session_app();
        let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        let b = app.world_mut().resource_mut::<ActiveSessionScope>().begin();

        app.world_mut().write_message(SessionScopeRetired(a));
        app.update();

        assert_eq!(
            app.world().resource::<ActiveSessionScope>().current(),
            Some(b)
        );
    }

    #[test]
    fn retiring_the_live_scope_clears_it() {
        let mut app = session_app();
        let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        app.world_mut().write_message(SessionScopeRetired(a));
        app.update();
        assert_eq!(app.world().resource::<ActiveSessionScope>().current(), None);
    }

    #[test]
    fn optional_session_policy_distinguishes_legacy_from_shell_home() {
        assert_eq!(
            SessionSpawnScope::for_optional_active_session(None),
            Some(SessionSpawnScope::UNSCOPED),
        );

        let mut active = ActiveSessionScope::default();
        assert_eq!(
            SessionSpawnScope::for_optional_active_session(Some(&active)),
            None,
        );
        let scope = active.begin();
        assert_eq!(
            SessionSpawnScope::for_optional_active_session(Some(&active)),
            Some(SessionSpawnScope::scoped(scope)),
        );
    }

    #[test]
    fn cleanup_leaves_unscoped_frontend_entities_intact() {
        let mut app = session_app();
        let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        let frontend = app.world_mut().spawn(Name::new("launcher-root")).id();
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                commands.spawn_in_session(a, Name::new("session-entity"));
            })
            .unwrap();

        app.world_mut().write_message(SessionScopeRetired(a));
        app.update();

        assert!(app.world().get_entity(frontend).is_ok());
        assert_eq!(count_scoped(&mut app, a), 0);
    }
}
