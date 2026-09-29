//! Which live room this is, which is not which room definition is selected.
//!
//! `RoomSet` answers "which definition is live" with an index that is the
//! same every time the session stands in that room. `RoomConstructionPlanId`
//! is a content hash that excludes commit-time facts, so two constructions of
//! one room share it. Neither can tell two visits apart. OW1 in
//! `docs/planning/engine/open-world-runtime-and-residency.md` needs that.

use bevy::ecs::query::With;
use bevy::ecs::world::Mut;
use bevy::prelude::{Component, Entity, Name, Ref, Single, World};
use bevy::ecs::component::Mutable;

use super::{session_world_entity, SessionRoot, SessionScopeId, SessionScopedEntity};

/// Which live room a session is standing in, as an ordinal of that session's
/// room publications.
///
/// A session starts at `0` — the room it was activated in — and every
/// publication that seats it in a room mints the next one, INCLUDING a
/// publication of the room it is already in. That is the whole point: leaving
/// `blink_run` for `portal_lab` and coming back gives three instances of two
/// definitions, and `RoomSet` alone reports the same index for the first and
/// the third.
///
/// This identity assumes one live room, so it lives on the session root. With
/// simultaneous instances (OW1), each instance's owner carries one; the
/// ordinal stays the same, only its carrier moves. It must never become a way
/// to select a definition. An entity that lives in a room carries the same
/// value as [`InRoomInstance`].
///
/// It is defined here, below the room crate, because the spawn scope that
/// stamps [`InRoomInstance`] is defined here.
///
/// It is rollback state (`root.live_room_instance`). A rewind across a room
/// publication returns to the previous live room, so the identity must rewind
/// too.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LiveRoomInstance(u32);

impl LiveRoomInstance {
    /// The activation room: a session that has published nothing is standing in
    /// its first live room, not in none.
    pub const ACTIVATION: Self = Self(0);

    /// How many rooms this session has been seated in before this one.
    pub fn ordinal(self) -> u32 {
        self.0
    }

    /// Rebuild from an ordinal, for snapshot decode only. It restores an
    /// identity this session already minted; it does not choose a new one.
    pub fn from_ordinal(ordinal: u32) -> Self {
        Self(ordinal)
    }

    /// Mint the next instance, because a room was just published into this
    /// session.
    ///
    /// Saturates instead of wrapping. A wrap would silently reuse an old
    /// room's identity; a stuck ordinal is visible in the census. Neither is
    /// reachable at 60Hz (a publication per tick for two years), so there is
    /// no refusal.
    pub fn advance(&mut self) {
        *self = self.next();
    }

    /// The instance the next publication into this session will mint. A room
    /// staged for that publication is stamped with it before `advance` runs,
    /// so its occupants never carry the room they replace.
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// The live room whose construction built an entity.
///
/// Stamped at spawn by [`crate::lifecycle::SessionSpawnScope::apply_to`] when
/// the scope carries an instance. A room that is staged for a publication
/// carries the instance that publication will mint, not the instance that is
/// live while it is staged. Whether a room's retirement sweeps the entity is
/// still [`crate::lifecycle::RoomScopedEntity`]'s question; this says which
/// live room that is.
///
/// It is a value, not an `Entity`, so a snapshot restores it without entity
/// mapping. It is rollback state (`scope.room_instance`): a rewind that
/// re-creates an occupant must give it back its room.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InRoomInstance(pub LiveRoomInstance);

/// The entity that owns one live room instance: its identity
/// ([`LiveRoomInstance`]) and the geometry a body in it collides with.
///
/// The session root keeps the room DEFINITIONS (`RoomSet`); each live
/// instance of one of them has a root of its own. A session has exactly one in
/// the one-room profile. Two live rooms at once (the Alice/Bob world) is two
/// roots, and the same code reads each through the instance an entity carries
/// ([`InRoomInstance`]).
///
/// It is owned by its session (`SessionScopedEntity`), so a retired session
/// takes it with it, and a candidate session's root is hidden with the rest of
/// the candidate. It is a rollback carrier (`root:room_instance`) with the
/// identity [`Self::sim_id`].
///
/// ⚠ One root per session is re-seated in place by each publication: its
/// `LiveRoomInstance` advances and its geometry is replaced. A second live
/// instance needs its own identity (OW1 cut 5); until then the identity is a
/// constant.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoomInstanceRoot;

impl RoomInstanceRoot {
    /// The peer-compared identity of the session's live room root.
    pub fn sim_id() -> crate::sim_id::SimId {
        crate::sim_id::SimId::singleton("session", "room_instance")
    }
}

/// A system parameter for the component `T` of THE live room.
///
/// ⚠ **THE ONE-LIVE-ROOM READ, NAMED AS THE DEBT IT IS.** It is a `Single`
/// over [`RoomInstanceRoot`], so it answers while a session has one live room
/// and the system does not run when there are two. That is right for a reader
/// that has not said WHICH room it means. ⚠ Today that is every reader,
/// `CollisionWorld` included: it reads the sole live room's geometry and
/// platforms until it takes the instance of the body it moves (OW1 cut 3c).
pub type SoleLiveRoom<'w, 's, T> = Single<'w, 's, Ref<'static, T>, With<RoomInstanceRoot>>;

/// The one-live-room WRITE: [`SoleLiveRoom`]'s mutable twin, and the same debt.
pub type SoleLiveRoomMut<'w, 's, T> = Single<'w, 's, &'static mut T, With<RoomInstanceRoot>>;

/// The live session's sole live room root.
///
/// `None` when there is no live session, no room root, or more than one live
/// room: a caller that asks for "the" room gets no answer rather than one of
/// them.
pub fn sole_live_room_entity(world: &World) -> Option<Entity> {
    let scope = session_world_entity(world)
        .and_then(|root| world.get::<SessionRoot>(root))
        .map(|root| root.0);
    let mut query =
        world.try_query_filtered::<(Entity, Option<&SessionScopedEntity>), With<RoomInstanceRoot>>()?;
    let mut roots = query
        .iter(world)
        .filter(|(_, owner)| match (owner, scope) {
            (Some(owner), Some(scope)) => owner.0 == scope,
            _ => true,
        })
        .map(|(entity, _)| entity);
    let root = roots.next()?;
    if roots.next().is_some() {
        return None;
    }
    Some(root)
}

/// Read one component of the sole live room, at an exclusive-world boundary.
pub fn sole_live_room_component<T: Component>(world: &World) -> Option<&T> {
    world.get::<T>(sole_live_room_entity(world)?)
}

/// Mutate one component of the sole live room, at an exclusive-world
/// boundary.
pub fn sole_live_room_component_mut<T: Component<Mutability = Mutable>>(
    world: &mut World,
) -> Option<Mut<'_, T>> {
    let entity = sole_live_room_entity(world)?;
    world.get_mut::<T>(entity)
}

/// The root of live room `instance` in the session `scope`, a hidden
/// candidate's included. This is the publication's question: which instance
/// root does THIS transaction replace.
pub fn live_room_root_for(
    world: &World,
    scope: SessionScopeId,
    instance: LiveRoomInstance,
) -> Option<Entity> {
    let matches = |(entity, live, owner): (Entity, &LiveRoomInstance, Option<&SessionScopedEntity>)| {
        (*live == instance && owner.is_none_or(|owner| owner.0 == scope)).then_some(entity)
    };
    // `try_query` refuses a filter on a component the world never registered,
    // so a world with no candidate marker asks without `Allow`: it has nothing
    // hidden.
    let roots: Vec<Entity> = match world.try_query_filtered::<
        (Entity, &LiveRoomInstance, Option<&SessionScopedEntity>),
        (
            With<RoomInstanceRoot>,
            bevy::ecs::query::Allow<crate::construction::InactiveCandidate>,
        ),
    >() {
        Some(mut query) => query.iter(world).filter_map(matches).collect(),
        None => world
            .try_query_filtered::<
                (Entity, &LiveRoomInstance, Option<&SessionScopedEntity>),
                With<RoomInstanceRoot>,
            >()?
            .iter(world)
            .filter_map(matches)
            .collect(),
    };
    let mut roots = roots.into_iter();
    let root = roots.next()?;
    debug_assert!(
        roots.next().is_none(),
        "two roots of live room {instance} in session {scope:?}"
    );
    Some(root)
}

/// The bundle a session's activation room root is spawned with.
pub fn activation_room_root(scope: SessionScopeId) -> impl bevy::prelude::Bundle {
    (
        Name::new("live room"),
        RoomInstanceRoot,
        RoomInstanceRoot::sim_id(),
        LiveRoomInstance::ACTIVATION,
        SessionScopedEntity(scope),
    )
}

/// Insert one component into the sole live room root of the direct/test
/// session, spawning the session root and its activation room root if they do
/// not exist yet. The live-room twin of
/// [`super::insert_session_world_component`], for small direct hosts and
/// focused tests.
pub fn insert_live_room_component<T: Component>(world: &mut World, component: T) -> Entity {
    let entity = match sole_live_room_entity(world) {
        Some(entity) => entity,
        None => {
            let session_root = match session_world_entity(world) {
                Some(root) => root,
                None => super::insert_session_world_component(world, Name::new("direct session world")),
            };
            let scope = world
                .get::<SessionRoot>(session_root)
                .map_or(SessionScopeId(0), |root| root.0);
            world.spawn(activation_room_root(scope)).id()
        }
    };
    world.entity_mut(entity).insert(component);
    entity
}

impl InRoomInstance {
    /// Does a room resident stamped `stamp` leave with the live room
    /// `departing`? This is the one rule both room-replacement roads (the
    /// transition and the hot reload) use to build their outgoing roster.
    ///
    /// A resident of another live room stays: that is the point of the stamp.
    ///
    /// ⚠ **AN UNSTAMPED RESIDENT LEAVES WITH ANY DEPARTING ROOM.** That is the
    /// rule from before rooms had instances, and it is exact while a session has
    /// one live room. The roads that still spawn room residents without a stamp
    /// are named: portal shots (the intent names no shooter), match and world
    /// items, and presentation `RoomVisual`s. A census over every shipped room
    /// (`every_room_resident_carries_its_live_room_after_combat`) holds the
    /// simulated population at zero. A second live instance must stamp them
    /// before this arm can be deleted.
    ///
    /// `departing` is `None` for a session root that carries no instance;
    /// then every resident leaves, as before.
    pub fn leaves_with(stamp: Option<&Self>, departing: Option<LiveRoomInstance>) -> bool {
        match (stamp, departing) {
            (Some(stamp), Some(departing)) => stamp.0 == departing,
            _ => true,
        }
    }
}

impl std::fmt::Display for LiveRoomInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Re-entering a room is not re-entering the same live room.
    ///
    /// This is the unit half. The composed half (a crossing back mints a
    /// third instance while `RoomSet` reports the first index again) is
    /// tested with the crossing.
    #[test]
    fn every_publication_is_a_different_live_room() {
        let mut instance = LiveRoomInstance::ACTIVATION;
        assert_eq!(instance.ordinal(), 0);

        let first = instance;
        instance.advance();
        let second = instance;
        instance.advance();
        let third = instance;

        assert_ne!(first, second);
        assert_ne!(second, third);
        assert_ne!(
            first, third,
            "coming back to a room you have already been in must not reuse the \
             identity of the live room you left"
        );
        assert_eq!(third.ordinal(), 2);
    }

    /// A resident of the departing room leaves with it, a resident of another
    /// live room stays, and an unstamped resident leaves as before.
    #[test]
    fn a_resident_leaves_only_with_its_own_live_room() {
        let departing = LiveRoomInstance::ACTIVATION;
        let other = departing.next().next();
        assert!(InRoomInstance::leaves_with(Some(&InRoomInstance(departing)), Some(departing)));
        assert!(!InRoomInstance::leaves_with(Some(&InRoomInstance(other)), Some(departing)));
        assert!(InRoomInstance::leaves_with(None, Some(departing)));
        assert!(InRoomInstance::leaves_with(Some(&InRoomInstance(other)), None));
    }
}
