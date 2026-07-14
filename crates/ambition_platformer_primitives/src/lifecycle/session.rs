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

use bevy::ecs::change_detection::Ref;
use bevy::ecs::system::{Single, SystemParam};
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

/// Marker resource: this App's gameplay simulation belongs to shell-routed
/// gameplay sessions. Inserted by the session bridge (the host composition that
/// routes gameplay through a launcher); never inserted by direct-entry apps or
/// headless harnesses, whose synchronously published root is sufficient authority.
///
/// [`simulation_authorized`] reads it: with the marker present, the gameplay
/// simulation root set runs only while a session scope is live, so launcher /
/// title / loading frames run zero simulation against zero session entities.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct SessionGatedSimulation;

/// Run condition for the gameplay-simulation root set.
///
/// Every app requires exactly one [`SessionRoot`] before gameplay systems may
/// run. Direct-entry and headless apps do not require shell scope identity, but
/// they still publish the same canonical root synchronously. Shell-routed hosts
/// additionally require [`ActiveSessionScope`] to name that exact root. This
/// keeps empty/minimal apps, frontend routes, provider preparation, and stale
/// delayed roots structurally dormant instead of letting required world
/// parameters fail validation.
pub fn simulation_authorized(
    gate: Option<Res<SessionGatedSimulation>>,
    scope: Option<Res<ActiveSessionScope>>,
    roots: Query<&SessionRoot>,
) -> bool {
    let Ok(root) = roots.single() else {
        return false;
    };
    gate.is_none()
        || scope.as_deref().and_then(ActiveSessionScope::current) == Some(root.0)
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

/// Read one component from the exact canonical live session-world root.
///
/// The root entity is the authority: at a frontend route no such entity exists,
/// while a gameplay activation owns exactly one. Systems using this parameter
/// therefore cannot accidentally fall back to process-resident world state.
pub type SessionWorldRef<'w, 's, T> =
    Single<'w, 's, Ref<'static, T>, With<SessionRoot>>;

/// Mutate one component on the exact canonical live session-world root.
pub type SessionWorldMut<'w, 's, T> =
    Single<'w, 's, &'static mut T, With<SessionRoot>>;

/// True only while the exact canonical live session-world root exists.
///
/// Direct-entry apps have no [`SessionGatedSimulation`] marker and therefore
/// require only one root. Shell-routed hosts additionally require that root's
/// scope to equal the active activation scope. A delayed root from A can never
/// wake gameplay or presentation while B is current or still preparing.
pub fn session_world_exists(
    gate: Option<Res<SessionGatedSimulation>>,
    active: Option<Res<ActiveSessionScope>>,
    roots: Query<&SessionRoot>,
) -> bool {
    let Ok(root) = roots.single() else {
        return false;
    };
    gate.is_none()
        || active.as_deref().and_then(ActiveSessionScope::current) == Some(root.0)
}

fn unique_session_world_root(world: &World) -> Option<(Entity, SessionScopeId)> {
    let mut roots = world.iter_entities().filter_map(|entity| {
        entity
            .get::<SessionRoot>()
            .map(|root| (entity.id(), root.0))
    });
    let root = roots.next()?;
    assert!(
        roots.next().is_none(),
        "more than one canonical SessionRoot exists"
    );
    Some(root)
}

/// Locate the one exact live session-world root without constructing a
/// persistent query state. Useful at imperative App/World boundaries such as
/// snapshot codecs, CLI inspection, and focused tests.
///
/// Shell-routed worlds additionally require the root owner to equal the active
/// session scope. A delayed root from a retired activation therefore remains
/// structurally unreadable even at imperative boundaries.
pub fn session_world_entity(world: &World) -> Option<Entity> {
    let (entity, owner) = unique_session_world_root(world)?;
    if world.contains_resource::<SessionGatedSimulation>()
        && world
            .get_resource::<ActiveSessionScope>()
            .and_then(ActiveSessionScope::current)
            != Some(owner)
    {
        return None;
    }
    Some(entity)
}

/// Read one canonical session-world component at an imperative World boundary.
pub fn session_world_component<T: Component>(world: &World) -> Option<&T> {
    world.get::<T>(session_world_entity(world)?)
}

/// Mutate one canonical session-world component at an imperative World boundary.
pub fn session_world_component_mut<T: Component>(world: &mut World) -> Option<Mut<'_, T>> {
    let entity = session_world_entity(world)?;
    world.get_mut::<T>(entity)
}

/// Insert one component into the canonical direct/test session-world root.
///
/// Provider activations should insert a complete prepared bundle through the
/// shell. This helper exists for small direct hosts and focused tests that
/// intentionally assemble the same root one component at a time.
pub fn insert_session_world_component<T: Component>(world: &mut World, component: T) -> Entity {
    let active_scope = world
        .get_resource::<ActiveSessionScope>()
        .and_then(ActiveSessionScope::current);
    let gated = world.contains_resource::<SessionGatedSimulation>();
    let entity = match unique_session_world_root(world) {
        Some((entity, owner)) => {
            assert!(
                !gated || active_scope == Some(owner),
                "cannot insert session-world state into stale root {owner:?} while {active_scope:?} is active"
            );
            entity
        }
        None => {
            let owner = active_scope.unwrap_or(SessionScopeId(0));
            world
                .spawn((Name::new("direct session world"), SessionRoot(owner)))
                .id()
        }
    };
    world.entity_mut(entity).insert(component);
    entity
}

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
mod tests;
