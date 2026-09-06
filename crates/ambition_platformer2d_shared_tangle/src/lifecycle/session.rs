//! Session scope — activation-owned entity lifetime.
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
use bevy::ecs::component::Mutable;
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

/// Optional process-start gate for visible direct-entry hosts.
///
/// The canonical session world may be constructed synchronously before its
/// presentation assets have settled. A visible host can insert this resource
/// in the closed state, present an opaque loading surface while it gathers
/// real readiness evidence, then open it once the first coherent gameplay
/// frame is safe to reveal. Apps that do not insert the resource retain the
/// existing behavior.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitialGameplayReadiness {
    ready: bool,
}

impl InitialGameplayReadiness {
    /// Construct a gate that keeps gameplay simulation dormant.
    pub const fn closed() -> Self {
        Self { ready: false }
    }

    /// Construct an already-open gate.
    pub const fn open() -> Self {
        Self { ready: true }
    }

    /// Allow gameplay simulation to begin.
    pub fn mark_ready(&mut self) {
        self.ready = true;
    }

    /// Whether gameplay may run.
    pub const fn is_ready(self) -> bool {
        self.ready
    }
}

/// Run condition for the gameplay-simulation root set.
///
/// Every app requires exactly one [`SessionRoot`] before gameplay systems may
/// run. Direct-entry and headless apps do not require shell scope identity, but
/// they still publish the same canonical root synchronously. A visible direct
/// host may also install [`InitialGameplayReadiness`] while its first coherent
/// presentation frame is being prepared. Shell-routed hosts additionally
/// require [`ActiveSessionScope`] to name that exact root. This keeps
/// empty/minimal apps, frontend routes, provider preparation, startup reveal,
/// and stale delayed roots structurally dormant instead of letting required
/// world parameters fail validation.
pub fn simulation_authorized(
    gate: Option<Res<SessionGatedSimulation>>,
    initial_readiness: Option<Res<InitialGameplayReadiness>>,
    scope: Option<Res<ActiveSessionScope>>,
    roots: Query<&SessionRoot>,
) -> bool {
    if initial_readiness
        .as_deref()
        .is_some_and(|readiness| !readiness.is_ready())
    {
        return false;
    }
    live_scope_of(gate.as_deref(), scope.as_deref(), &roots).is_some()
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
pub type SessionWorldRef<'w, 's, T> = Single<'w, 's, Ref<'static, T>, With<SessionRoot>>;

/// Mutate one component on the exact canonical live session-world root.
pub type SessionWorldMut<'w, 's, T> = Single<'w, 's, &'static mut T, With<SessionRoot>>;

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
    live_scope_of(gate.as_deref(), active.as_deref(), &roots).is_some()
}

/// The query-side [`live_session_world_root`], shared by every system-parameter
/// form so the World-level and query-level answers cannot drift apart.
fn live_scope_of(
    gate: Option<&SessionGatedSimulation>,
    active: Option<&ActiveSessionScope>,
    roots: &Query<&SessionRoot>,
) -> Option<SessionScopeId> {
    match gate {
        // Shell-routed: the activation names its root. Selecting by scope means
        // a lingering retired root is not a candidate rather than an ambiguity.
        Some(_) => {
            let active = active.and_then(ActiveSessionScope::current)?;
            roots.iter().map(|root| root.0).find(|owner| *owner == active)
        }
        // Direct-entry: exactly one root IS the authority.
        None => roots.single().ok().map(|root| root.0),
    }
}

fn unique_session_world_root(world: &World) -> Option<(Entity, SessionScopeId)> {
    // `try_query` builds read-only query state from `&World` and yields `None`
    // when `SessionRoot` was never registered — the correct "no root" answer.
    let mut query = world.try_query::<(Entity, &SessionRoot)>()?;
    let mut roots = query.iter(world).map(|(entity, root)| (entity, root.0));
    let root = roots.next()?;
    assert!(
        roots.next().is_none(),
        "more than one canonical SessionRoot exists"
    );
    Some(root)
}

/// The canonical root of the LIVE session, and the scope that owns it.
///
/// ⭐⭐ A SHELL-ROUTED HOST SELECTS BY SCOPE; IT DOES NOT ASSERT UNIQUENESS.
/// The activation names its root, so a root from a retired activation that has
/// not been despawned yet is simply NOT A CANDIDATE — it can neither be chosen
/// nor make the choice ambiguous.
///
/// ⛔⛔ THIS ASSERTED INSTEAD, AND THE ASSERT WAS THE HAZARD. A composition that
/// briefly held two roots PANICKED rather than resolving the live one, which is
/// the opposite of what ownership is for: a stale entity must be harmless, and a
/// process abort is not harmless. The same panic is already recorded once in
/// `ambition_app::app::resources` against a build-time root coexisting with an
/// activation's.
///
/// A direct-entry host has no activation to name a root, so uniqueness IS the
/// authority there and the assert stays.
fn live_session_world_root(world: &World) -> Option<(Entity, SessionScopeId)> {
    let mut query = world.try_query::<(Entity, &SessionRoot)>()?;
    if world.contains_resource::<SessionGatedSimulation>() {
        let active = world
            .get_resource::<ActiveSessionScope>()
            .and_then(ActiveSessionScope::current)?;
        return query
            .iter(world)
            .find(|(_, root)| root.0 == active)
            .map(|(entity, root)| (entity, root.0));
    }
    unique_session_world_root(world)
}

/// Locate the one exact live session-world root without constructing a
/// persistent query state. Useful at imperative App/World boundaries such as
/// snapshot codecs, CLI inspection, and focused tests.
///
/// Shell-routed worlds additionally require the root owner to equal the active
/// session scope. A delayed root from a retired activation therefore remains
/// structurally unreadable even at imperative boundaries.
pub fn session_world_entity(world: &World) -> Option<Entity> {
    live_session_world_root(world).map(|(entity, _)| entity)
}

/// Which gameplay session owns the one exact live session world, if any.
///
/// ⭐⭐ THE SCOPE ANY SESSION-OWNED AUTHORITY MUST NAME TO BE READ. At a
/// frontend route it is `None`; during gameplay it is the activation's; in a
/// direct-entry app or headless harness it is the root's own, which is what
/// lets a harness and the shell host share ONE ownership rule instead of two.
///
/// ⛔ NOT [`ActiveSessionScope::current`], which is `None` in every app that
/// never installed shell routing — the root is the authority, and this is the
/// same question [`session_world_entity`] answers, returning who instead of
/// which entity.
pub fn live_session_scope(world: &World) -> Option<SessionScopeId> {
    live_session_world_root(world).map(|(_, owner)| owner)
}

/// [`live_session_scope`] as a system parameter.
#[derive(SystemParam)]
pub struct LiveSessionScope<'w, 's> {
    gate: Option<Res<'w, SessionGatedSimulation>>,
    active: Option<Res<'w, ActiveSessionScope>>,
    roots: Query<'w, 's, &'static SessionRoot>,
}

impl LiveSessionScope<'_, '_> {
    /// The owning scope, or `None` when no session world is live.
    pub fn get(&self) -> Option<SessionScopeId> {
        live_scope_of(self.gate.as_deref(), self.active.as_deref(), &self.roots)
    }
}

/// Advance until [`session_world_entity`] resolves, returning the frame count.
///
/// Shell-routed hosts may need several updates before the prepared session root
/// exists. Returns `Err(max_frames)` instead of panicking when the budget expires.
pub fn settle_until_session_world(app: &mut App, max_frames: u32) -> Result<u32, u32> {
    for frame in 0..=max_frames {
        if session_world_entity(app.world()).is_some() {
            return Ok(frame);
        }
        app.update();
    }
    Err(max_frames)
}

/// Advance until the session has both a world and a controlled subject.
///
/// Use [`settle_until_session_world`] when the caller only needs room state;
/// controlled-body materialization may complete on a later frame.
pub fn settle_until_controlled_subject(app: &mut App, max_frames: u32) -> Result<u32, u32> {
    for frame in 0..=max_frames {
        let seated = session_world_entity(app.world()).is_some()
            && app
                .world()
                .get_resource::<crate::markers::ControlledSubject>()
                .is_some_and(|subject| subject.0.is_some());
        if seated {
            return Ok(frame);
        }
        app.update();
    }
    Err(max_frames)
}

/// Frames a shell activation is given to produce its world before a caller
/// gives up.
///
/// Preparation is eight work items behind a load barrier; a handful of frames
/// covers it with room to spare, and a bound that is far too generous would
/// turn a genuine hang into a slow test rather than a failure.
pub const SESSION_SETTLE_FRAMES: u32 = 240;

/// Read one canonical session-world component at an imperative World boundary.
pub fn session_world_component<T: Component>(world: &World) -> Option<&T> {
    world.get::<T>(session_world_entity(world)?)
}

/// Mutate one canonical session-world component at an imperative World boundary.
pub fn session_world_component_mut<T: Component<Mutability = Mutable>>(
    world: &mut World,
) -> Option<Mut<'_, T>> {
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

/// Signal that a session scope has become the live one, before anything has
/// been built for it.
///
/// ⭐⭐ THE EDGE THAT MAKES CLEANUP HYGIENE. A process-global resource that
/// mirrors one live session is dangerous only if the NEXT session can read the
/// previous one's value. Re-establishing it when a session BEGINS closes that
/// without giving every reader an ownership check, because the value a session
/// reads is one its own activation wrote.
///
/// ⛔ Retirement alone could not do this. It is a cleanup that must happen, and
/// "must happen" is exactly the property a scheduling change, an abnormal exit
/// or a delayed frame can take away — which it did: a retired Smash match left
/// state that Ambition then read as its own.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionScopeActivated(pub SessionScopeId);

/// Stable schedule seam for exact scope retirement.
///
/// ⭐⭐ THE ORDER IS AN OWNERSHIP RULE, not three systems that happen to be
/// chained: an authority governing a scope stands down BEFORE the world it
/// governs is removed. Read the other way round and the authority observes its
/// own world vanishing underneath it, which is indistinguishable from
/// corruption — and that is precisely how a retired Smash match came to
/// poison the rollback timeline the next game would inherit.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SessionScopeSet {
    /// A newly live scope re-establishes the process-global state that mirrors
    /// one session, BEFORE any provider builds that session's world.
    ///
    /// ⭐ This seam is why the retirement resets below are hygiene. Whatever a
    /// skipped, delayed or abnormal teardown left standing is overwritten here
    /// by the session about to read it.
    Activate,
    /// Presentation systems may materialize activation-owned visuals after the
    /// provider has published its session world.
    Presentation,
    /// Authorities that GOVERN the retiring scope stand down: the rollback
    /// timeline, and anything else holding a claim over that session's world.
    ///
    /// ⛔ Scheduling here is hygiene, not correctness. An authority that misses
    /// this seam must still be inert for the next scope, because it names its
    /// owner and the next scope is not it.
    RetireAuthority,
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

    /// Populate an entity someone else allocated, giving it the same session
    /// ownership [`Self::spawn_session_scoped`] would have.
    ///
    /// The construction executor allocates a planned entity's root itself so a
    /// recipe cannot choose or commandeer one, which means recipes insert onto
    /// an entity rather than spawning it. These are the insert-shaped siblings
    /// of the spawn helpers above.
    fn insert_session_scoped<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        entity: Entity,
        bundle: B,
    ) -> EntityCommands<'_>;

    /// Populate an allocated entity as owned by both the active authored room
    /// and the captured gameplay session.
    fn insert_room_in_session<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        entity: Entity,
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

    fn insert_session_scoped<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        entity: Entity,
        bundle: B,
    ) -> EntityCommands<'_> {
        let mut entity = self.entity(entity);
        entity.insert(bundle);
        scope.apply_to(&mut entity);
        entity
    }

    fn insert_room_in_session<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        entity: Entity,
        bundle: B,
    ) -> EntityCommands<'_> {
        let mut entity = self.entity(entity);
        entity.insert((RoomScopedEntity, bundle));
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
            .add_message::<SessionScopeActivated>()
            .configure_sets(
                Update,
                (
                    SessionScopeSet::Activate,
                    SessionScopeSet::Presentation,
                    SessionScopeSet::RetireAuthority,
                    SessionScopeSet::Cleanup,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                despawn_retired_session_entities.in_set(SessionScopeSet::Cleanup),
            );
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod settle_tests {
    use super::*;
    use crate::lifecycle::markers::RoomVisual;

    /// A world that is already there settles in ZERO frames.
    ///
    /// They disagree only about when.
    #[test]
    fn a_build_time_root_settles_immediately() {
        let mut app = App::new();
        let entity = insert_session_world_component(app.world_mut(), RoomVisual);
        assert_eq!(settle_until_session_world(&mut app, 8), Ok(0));
        assert_eq!(session_world_entity(app.world()), Some(entity));
    }

    /// A world that never arrives is an ERROR, not a hang and not a panic.
    ///
    /// the caller this replaces reads `.expect("active session RoomSet")`
    /// three lines after its update loop, so a session that never activated
    /// surfaced as a panic naming the component rather than the barrier. An
    /// exhausted budget is a fact the caller can report.
    #[test]
    fn a_world_that_never_arrives_reports_the_budget_it_spent() {
        let mut app = App::new();
        assert_eq!(settle_until_session_world(&mut app, 4), Err(4));
    }

    /// A LATE root is found, which is the whole point.
    #[test]
    fn a_root_that_appears_on_a_later_frame_is_waited_for() {
        #[derive(Resource, Default)]
        struct Frames(u32);

        let mut app = App::new();
        app.init_resource::<Frames>();
        app.add_systems(
            Update,
            |mut frames: ResMut<Frames>, mut commands: Commands| {
                frames.0 += 1;
                if frames.0 == 3 {
                    commands.spawn((SessionRoot(SessionScopeId(0)), RoomVisual));
                }
            },
        );
        assert_eq!(settle_until_session_world(&mut app, 16), Ok(3));
    }
}
