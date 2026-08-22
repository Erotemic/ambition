//! Round-scoped entity lifetime.
//!
//! Projectiles, strike volumes, temporary summons, and other effects that must
//! not cross a round boundary carry [`RoundScopedEntity`]. Scope is explicit
//! lifetime policy, not spawn provenance: dynamically spawned entities may have
//! longer lifetimes. The active round id and allocator are rollback state.

use bevy::prelude::*;

/// Stable identity of one round.
///
/// Minted from a deterministic monotonic counter, so the same sequence of rounds
/// produces the same identities in a replay (ADR 0023: nothing in the sim may
/// depend on a clock or a hash seed).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoundScopeId(pub u64);

/// Lifetime-scope marker: despawn when this round ends.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoundScopedEntity(pub RoundScopeId);

/// Current round plus deterministic allocator. Both fields are rollback state;
/// rewinding across a round boundary must reproduce the same minted id.
#[derive(Resource, Default, Debug, Clone)]
pub struct ActiveRoundScope {
    current: Option<RoundScopeId>,
    next_raw: u64,
}

/// Snapshot the actual round id and allocator value, not just resource
/// presence. `Option<RoundScopeId>` encodes as a presence flag plus raw id so
/// absence stays distinct from the valid first id `0`.
impl ambition_platformer2d_core::snapshot::SnapshotState for ActiveRoundScope {
    fn encode(&self, out: &mut Vec<u8>) {
        ambition_platformer2d_core::snapshot::put_bool(out, self.current.is_some());
        ambition_platformer2d_core::snapshot::put_u64(out, self.current.map_or(0, |id| id.0));
        ambition_platformer2d_core::snapshot::put_u64(out, self.next_raw);
    }

    fn decode(r: &mut ambition_platformer2d_core::snapshot::Reader<'_>) -> Option<Self> {
        let present = r.bool()?;
        let raw = r.u64()?;
        let next_raw = r.u64()?;
        Some(Self {
            current: present.then_some(RoundScopeId(raw)),
            next_raw,
        })
    }
}

impl ActiveRoundScope {
    /// Mint a fresh round, make it current, and return it.
    pub fn begin(&mut self) -> RoundScopeId {
        let id = RoundScopeId(self.next_raw);
        self.next_raw += 1;
        self.current = Some(id);
        id
    }

    /// The active round, when a match currently has one.
    pub fn current(&self) -> Option<RoundScopeId> {
        self.current
    }

    /// End the current round without starting another — the match is over.
    /// Entities scoped to it are culled by [`despawn_departed_round_entities`]
    /// exactly as they are at a round boundary.
    pub fn end(&mut self) {
        self.current = None;
    }

    /// Capture the current round for spawn work requested now.
    ///
    /// Captured rather than read later, for the same reason session scope is: a
    /// spawn deferred across a round boundary belongs to the round that ASKED,
    /// not to whichever round happens to be current when the command flushes.
    pub fn spawn_scope(&self) -> RoundSpawnScope {
        RoundSpawnScope { id: self.current }
    }
}

/// The round a spawn belongs to, captured at request time.
///
/// [`RoundSpawnScope::UNSCOPED`] means "outlives rounds", which is the right
/// answer for a fighter's body, the stage, and anything a match owns rather than
/// a round.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoundSpawnScope {
    id: Option<RoundScopeId>,
}

impl RoundSpawnScope {
    /// Belongs to no round; survives every round boundary.
    pub const UNSCOPED: Self = Self { id: None };

    pub const fn scoped(id: RoundScopeId) -> Self {
        Self { id: Some(id) }
    }

    /// Attach this ownership to an already-created entity command.
    pub fn apply_to(self, entity: &mut EntityCommands<'_>) {
        if let Some(id) = self.id {
            entity.insert(RoundScopedEntity(id));
        }
    }
}

impl From<RoundScopeId> for RoundSpawnScope {
    fn from(id: RoundScopeId) -> Self {
        Self::scoped(id)
    }
}

/// Despawn every entity scoped to a round that is no longer current.
///
/// The whole point: this names no transient family. A round boundary asks "who
/// belonged to the round that just ended" and the entities answer, instead of
/// the boundary listing the kinds of thing that might exist.
///
///  compares against the CURRENT id rather than despawning everything scoped,
/// so an entity spawned during the same frame the round turned over — a
/// projectile fired on the KO frame — carries the OLD id and is culled, while
/// one spawned by the new round's opening is not. Despawning "all scoped
/// entities" at the boundary would race that.
pub fn despawn_departed_round_entities(
    mut commands: Commands,
    active: Option<Res<ActiveRoundScope>>,
    scoped: Query<(Entity, &RoundScopedEntity)>,
) {
    let current = active.and_then(|active| active.current());
    for (entity, owner) in &scoped {
        if Some(owner.0) != current {
            commands.entity(entity).try_despawn();
        }
    }
}

/// Installs the round lifetime: the allocator and the culler.
///
/// Separate from `SessionScopePlugin` because the lifetimes nest — a session
/// contains many rounds — and a match that never starts a round still has a
/// session. Installed by whatever composes a MATCH, not by the engine at large:
/// a single-player platformer has no rounds and should carry no round culler.
pub struct RoundScopePlugin;

impl Plugin for RoundScopePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveRoundScope>()
            // In `Update` and not the sim schedule: a round boundary is a rules
            // decision, and the entities it culls are already gone from the sim's
            // point of view the moment the rules said so. Matching
            // `despawn_retired_session_entities`, which is the same shape.
            .add_systems(Update, despawn_departed_round_entities);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct Persistent;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<ActiveRoundScope>();
        app.add_systems(Update, despawn_departed_round_entities);
        app
    }

    #[test]
    fn a_round_boundary_culls_what_that_round_created_and_nothing_else() {
        let mut app = app();
        let first = app.world_mut().resource_mut::<ActiveRoundScope>().begin();

        let shot = app.world_mut().spawn(RoundScopedEntity(first)).id();
        let body = app.world_mut().spawn(Persistent).id();
        app.update();
        assert!(
            app.world().get_entity(shot).is_ok(),
            "the round is still live; nothing should have been culled"
        );

        app.world_mut().resource_mut::<ActiveRoundScope>().begin();
        app.update();
        assert!(
            app.world().get_entity(shot).is_err(),
            "the round ended and its projectile outlived it"
        );
        assert!(
            app.world().get_entity(body).is_ok(),
            "an unscoped entity is a MATCH-lifetime thing — a fighter's body — \
             and a round boundary must not touch it"
        );
    }

    #[test]
    fn a_spawn_requested_before_the_boundary_belongs_to_the_round_that_asked() {
        // The reason the scope is CAPTURED and not read at flush time: a
        // projectile fired on the KO frame belongs to the round it was fired in,
        // whichever round is current by the time the command applies.
        let mut app = app();
        let first = app.world_mut().resource_mut::<ActiveRoundScope>().begin();
        let captured = app.world().resource::<ActiveRoundScope>().spawn_scope();
        app.world_mut().resource_mut::<ActiveRoundScope>().begin();

        let late = app.world_mut().spawn(()).id();
        {
            let world = app.world_mut();
            let mut commands = world.commands();
            let mut entity = commands.entity(late);
            captured.apply_to(&mut entity);
        }
        app.world_mut().flush();
        assert_eq!(
            app.world().get::<RoundScopedEntity>(late),
            Some(&RoundScopedEntity(first)),
            "the capture has to remember the round that ASKED"
        );
        app.update();
        assert!(
            app.world().get_entity(late).is_err(),
            "and being late does not grant it a reprieve"
        );
    }

    #[test]
    fn ending_a_match_culls_the_last_rounds_entities_too() {
        let mut app = app();
        let last = app.world_mut().resource_mut::<ActiveRoundScope>().begin();
        let shot = app.world_mut().spawn(RoundScopedEntity(last)).id();
        app.world_mut().resource_mut::<ActiveRoundScope>().end();
        app.update();
        assert!(
            app.world().get_entity(shot).is_err(),
            "a match that ends leaves the same debris a round boundary does"
        );
    }
}
