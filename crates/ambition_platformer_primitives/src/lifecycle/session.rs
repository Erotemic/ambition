//! **Session scope** — activation-owned entity lifetime.
//!
//! A *session* is one activated gameplay experience: a launched Sanic run, a
//! launched Mary-O run, the main game entered from a launcher. Every entity a
//! session spawns — the player body, room geometry, room-feature entities,
//! session cameras and UI, and dynamically-spawned gameplay entities — belongs to
//! that session's [`SessionScopeId`] and is torn down together when the session
//! retires.
//!
//! This is a DISTINCT lifetime from [`super::ModeScopedEntity`]. Two consecutive
//! Sanic runs share the mode `"sanic"`, so a mode sweep would carry the first
//! run's leftovers into the second; a session scope is minted fresh per
//! activation, so relaunch rebuilds from nothing. That freshness is exactly what
//! makes launch → quit → relaunch leak-free.
//!
//! The abstraction sits BELOW the game shell: a route provider maps its shell
//! activation to a session scope, but this module knows nothing about routing.
//! Simulation and world-construction code can therefore own session lifetime
//! without importing `ambition_game_shell`. The retire signal is a plain
//! [`SessionScopeRetired`] message so the sweep is testable in isolation.

use bevy::prelude::*;

/// Stable identity of one activated gameplay session.
///
/// Minted by [`ActiveSessionScope::begin`] from a deterministic monotonic
/// counter — no wall-clock, no `Entity` index — so a replay mints the same ids in
/// the same order (ADR 0023). Distinct per activation: re-entering an experience
/// yields a new id, which is what makes a relaunch leak-free rather than a reuse
/// of the previous session's entities.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionScopeId(pub u64);

/// The ambient active session scope plus its deterministic allocator.
///
/// While a session's world is being built AND for the session's whole live span,
/// `current` names that session. [`SpawnSessionScopedExt::spawn_session_scoped`]
/// reads it at command-flush time, so nested construction helpers and deferred
/// gameplay spawns inherit the scope without the caller threading an id through
/// every signature. [`begin`](Self::begin) mints the next id and makes it
/// current; a retire clears it.
#[derive(Resource, Default, Debug)]
pub struct ActiveSessionScope {
    current: Option<SessionScopeId>,
    next_raw: u64,
}

impl ActiveSessionScope {
    /// Mint a fresh scope id, make it the active scope, and return it.
    ///
    /// The counter only ever advances, so two `begin` calls never collide even if
    /// the first scope was already retired — a relaunch is a genuinely new
    /// identity, not a recycled one.
    pub fn begin(&mut self) -> SessionScopeId {
        let id = SessionScopeId(self.next_raw);
        self.next_raw += 1;
        self.current = Some(id);
        id
    }

    /// The scope entities spawned right now will inherit, if any.
    pub fn current(&self) -> Option<SessionScopeId> {
        self.current
    }

    /// Clear the active scope unconditionally.
    pub fn clear(&mut self) {
        self.current = None;
    }

    /// Clear the active scope only if it is `id`. Retiring session A while session
    /// B has since become the live one must NOT silently un-scope B's later
    /// spawns, so the clear is guarded by identity.
    pub fn clear_if_current(&mut self, id: SessionScopeId) {
        if self.current == Some(id) {
            self.current = None;
        }
    }
}

/// Tag: this entity is owned by session scope `.0`. Retiring the scope despawns
/// it. The one marker every session-owned entity carries, whether spawned during
/// construction or dynamically mid-session.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionScopedEntity(pub SessionScopeId);

/// Marker on a session's ROOT entity — the anchor a provider can track to know a
/// session exists and which scope it is. It is itself session-scoped (carry
/// [`SessionScopedEntity`] alongside it), so it dies with its scope.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionRoot(pub SessionScopeId);

/// Signal that a session scope has been retired.
///
/// A provider — a tier up, where the shell is visible — writes this when its route
/// deactivates; [`despawn_retired_session_entities`] consumes it. Living here,
/// shell-free, keeps the sweep testable without any routing machinery.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionScopeRetired(pub SessionScopeId);

/// `Commands` extension that attaches session ownership at the spawn site.
pub trait SpawnSessionScopedExt {
    /// Spawn tagged with the currently-active session scope.
    ///
    /// The tag is applied when the command buffer flushes and reads
    /// [`ActiveSessionScope`] at that moment, so even deferred spawns and spawns
    /// made by nested helpers inherit the ambient scope. If no session is active,
    /// the entity is left unscoped.
    fn spawn_session_scoped<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_>;

    /// Spawn tagged with an explicit scope, ignoring the ambient one. For a
    /// provider that already holds its activation's id, and for tests that drive
    /// two scopes at once.
    fn spawn_in_session<B: Bundle>(
        &mut self,
        scope: SessionScopeId,
        bundle: B,
    ) -> EntityCommands<'_>;
}

impl SpawnSessionScopedExt for Commands<'_, '_> {
    fn spawn_session_scoped<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_> {
        let mut entity = self.spawn(bundle);
        let id = entity.id();
        // Read the ambient scope at flush time, not now: `Commands` cannot see a
        // resource synchronously, and a deferred read is exactly what lets a
        // nested helper's spawn inherit whatever scope is live when it lands.
        entity.commands().queue(move |world: &mut World| {
            let scope = world
                .get_resource::<ActiveSessionScope>()
                .and_then(ActiveSessionScope::current);
            if let Some(scope) = scope {
                if let Ok(mut entity_mut) = world.get_entity_mut(id) {
                    entity_mut.insert(SessionScopedEntity(scope));
                }
            }
        });
        entity
    }

    fn spawn_in_session<B: Bundle>(
        &mut self,
        scope: SessionScopeId,
        bundle: B,
    ) -> EntityCommands<'_> {
        let mut entity = self.spawn(bundle);
        entity.insert(SessionScopedEntity(scope));
        entity
    }
}

/// Despawn every entity owned by a retired session scope, and clear the ambient
/// scope if it was the retired one.
///
/// Driven by [`SessionScopeRetired`]. Cleanup for scope A never touches scope B:
/// the filter is by exact id, and despawns are commutative so the `Query` walk
/// order does not matter (ADR 0023). Entities with no [`SessionScopedEntity`] —
/// process-resident frontend/launcher entities — are never candidates.
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

/// Installs the session-scope resource, the retire signal, and the sweep in
/// `Update`.
///
/// The sweep is also exposed as a free function ([`despawn_retired_session_entities`])
/// so a host that wants it in a sim schedule instead can place it itself; this
/// plugin is the batteries-included default for a headless composition.
pub struct SessionScopePlugin;

impl Plugin for SessionScopePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveSessionScope>()
            .add_message::<SessionScopeRetired>()
            .add_systems(Update, despawn_retired_session_entities);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// A tiny host with the session plugin and a manual message pump.
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

    /// A nested construction helper that itself spawns — proving inheritance
    /// survives a call boundary, not just a single spawn site.
    fn nested_spawn_helper(commands: &mut Commands) {
        commands.spawn_session_scoped(Name::new("nested-child"));
    }

    /// #1 + #2 + #3: everything a session constructs — direct, nested, deferred —
    /// carries the same scope, read from the ambient `ActiveSessionScope`.
    #[test]
    fn all_session_spawns_inherit_one_scope() {
        let mut app = session_app();
        let scope = app.world_mut().resource_mut::<ActiveSessionScope>().begin();

        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                // direct spawn
                commands.spawn_session_scoped(Name::new("root"));
                // a second direct spawn
                commands.spawn_session_scoped(Name::new("sibling"));
                // nested helper spawn
                nested_spawn_helper(&mut commands);
            })
            .unwrap();

        assert_eq!(
            count_scoped(&mut app, scope),
            3,
            "direct, sibling, and nested spawns must all inherit the active scope"
        );
    }

    /// #6: repeated session creation yields distinct ids; the counter never reuses
    /// a value even after the earlier scope is cleared.
    #[test]
    fn begin_mints_distinct_ids() {
        let mut app = session_app();
        let mut active = app.world_mut().resource_mut::<ActiveSessionScope>();
        let a = active.begin();
        active.clear();
        let b = active.begin();
        let c = active.begin();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_eq!((a.0, b.0, c.0), (0, 1, 2));
    }

    /// #4 + #5: retiring scope A removes exactly A's entities and leaves B intact.
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

        assert_eq!(count_scoped(&mut app, a), 2);
        assert_eq!(count_scoped(&mut app, b), 1);

        app.world_mut().write_message(SessionScopeRetired(a));
        app.update();

        assert_eq!(count_scoped(&mut app, a), 0, "A's entities are gone");
        assert_eq!(
            count_scoped(&mut app, b),
            1,
            "B's entity survived A's retire"
        );
    }

    /// #5 corollary: retiring A while B is the live ambient scope must not clear B
    /// (so B's later spawns keep inheriting their scope).
    #[test]
    fn retiring_a_stale_scope_leaves_the_live_one_active() {
        let mut app = session_app();
        let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        let b = app.world_mut().resource_mut::<ActiveSessionScope>().begin();

        app.world_mut().write_message(SessionScopeRetired(a));
        app.update();

        assert_eq!(
            app.world().resource::<ActiveSessionScope>().current(),
            Some(b),
            "retiring the stale scope A must not clear the live scope B"
        );
    }

    /// Retiring the live scope clears the ambient pointer, so subsequent unscoped
    /// spawns are genuinely unscoped rather than silently re-tagged.
    #[test]
    fn retiring_the_live_scope_clears_it() {
        let mut app = session_app();
        let a = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        app.world_mut().write_message(SessionScopeRetired(a));
        app.update();
        assert_eq!(app.world().resource::<ActiveSessionScope>().current(), None);
    }

    /// #7: an unscoped (frontend/process-resident) entity is never a cleanup
    /// candidate — no session retire can despawn it.
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

        assert!(
            app.world().get_entity(frontend).is_ok(),
            "the unscoped frontend entity must survive a session retire"
        );
        assert_eq!(count_scoped(&mut app, a), 0);
    }
}
