//! One resolver for live occurrence references, independent of object facets.

use super::{ObjectOccurrence, SessionScopeId, SessionScopedEntity};
use ambition_extension_sdk::ObjectRef;
use bevy::{ecs::system::SystemParam, prelude::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectLookupError { Unbound, ForeignSession, Retired, Ambiguous }

fn address(session: &SessionScopedEntity, occurrence: &ObjectOccurrence) -> ObjectRef {
    occurrence.reference(session.0)
}

fn resolve<'a>(session: SessionScopeId, reference: ObjectRef,
    objects: impl Iterator<Item = (Entity, &'a SessionScopedEntity, &'a ObjectOccurrence)>)
    -> Result<Entity, ObjectLookupError> {
    if !reference.is_bound() { return Err(ObjectLookupError::Unbound); }
    if reference.session != session.0 { return Err(ObjectLookupError::ForeignSession); }
    let mut found = objects.filter(|(_, owner, occurrence)| address(owner, occurrence) == reference);
    let entity = found.next().ok_or(ObjectLookupError::Retired)?.0;
    if found.next().is_some() { return Err(ObjectLookupError::Ambiguous); }
    Ok(entity)
}

/// Object facets do not change the resolution population. A missing mailbox,
/// body, or interaction capability is a separate domain refusal.
#[derive(SystemParam)]
pub struct LiveObjects<'w, 's> {
    objects: Query<'w, 's, (Entity, &'static SessionScopedEntity, &'static ObjectOccurrence)>,
}

impl LiveObjects<'_, '_> {
    pub fn reference(&self, entity: Entity) -> Option<ObjectRef> {
        let (_, session, occurrence) = self.objects.get(entity).ok()?;
        Some(address(session, occurrence))
    }
    pub fn resolve(&self, session: SessionScopeId, reference: ObjectRef) -> Result<Entity, ObjectLookupError> {
        resolve(session, reference, self.objects.iter())
    }
}

pub fn object_reference(world: &World, entity: Entity) -> Option<ObjectRef> {
    let mut objects = world.try_query::<(&SessionScopedEntity, &ObjectOccurrence)>()?;
    let (session, occurrence) = objects.get(world, entity).ok()?;
    Some(address(session, occurrence))
}

pub fn resolve_object(world: &World, session: SessionScopeId, reference: ObjectRef) -> Result<Entity, ObjectLookupError> {
    if !reference.is_bound() { return Err(ObjectLookupError::Unbound); }
    if reference.session != session.0 { return Err(ObjectLookupError::ForeignSession); }
    let Some(mut objects) = world.try_query::<(Entity, &SessionScopedEntity, &ObjectOccurrence)>() else {
        return Err(ObjectLookupError::Retired);
    };
    resolve(session, reference, objects.iter(world))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lifecycle::*, sim_id::SimId};
    use bevy::ecs::system::SystemState;

    #[test]
    fn references_resolve_one_lifetime_independent_of_room_and_facets() {
        let mut world = World::new();
        let session = SessionScopeId(7);
        let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        for room in rooms { world.spawn((RoomInstanceRoot, room, SessionScopedEntity(session))); }
        let mut spawn = |room| world.spawn((SessionScopedEntity(session), InRoomInstance(room),
            SimId::placement("same_authored_id"))).id();
        let a = spawn(rooms[0]);
        let b = spawn(rooms[1]);
        ensure_object_occurrence(&mut world, a).unwrap();
        ensure_object_occurrence(&mut world, b).unwrap();
        let reference = object_reference(&world, a).unwrap();
        assert_ne!(reference, object_reference(&world, b).unwrap());
        world.entity_mut(a).insert(InRoomInstance(rooms[1]));
        assert_eq!(resolve_object(&world, session, reference), Ok(a));
        let mut state = SystemState::<LiveObjects>::new(&mut world);
        assert_eq!(state.get(&world).unwrap().resolve(session, reference), Ok(a));
        assert_eq!(resolve_object(&world, SessionScopeId(8), reference), Err(ObjectLookupError::ForeignSession));
        let stamp = *world.get::<ObjectOccurrence>(a).unwrap();
        world.despawn(a);
        assert_eq!(resolve_object(&world, session, reference), Err(ObjectLookupError::Retired));
        let rebuilt = world.spawn((SessionScopedEntity(session), InRoomInstance(rooms[1]), stamp)).id();
        assert_eq!(resolve_object(&world, session, reference), Ok(rebuilt));
        let duplicate = world.spawn((SessionScopedEntity(session), stamp)).id();
        assert_eq!(resolve_object(&world, session, reference), Err(ObjectLookupError::Ambiguous));
        assert_eq!(state.get(&world).unwrap().resolve(session, reference), Err(ObjectLookupError::Ambiguous));
        world.despawn(duplicate);
        world.despawn(rebuilt);
        let replacement = world.spawn((SessionScopedEntity(session), InRoomInstance(rooms[0]),
            SimId::placement("same_authored_id"))).id();
        ensure_object_occurrence(&mut world, replacement).unwrap();
        assert_ne!(reference, object_reference(&world, replacement).unwrap());
        assert_eq!(resolve_object(&world, session, reference), Err(ObjectLookupError::Retired));
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;
    use crate::{action_results::*, lifecycle::*};
    use ambition_extension_sdk::action::{ActionStage, ReplyTo};
    use bevy::ecs::system::SystemState;

    #[test]
    fn a_session_recipient_does_not_borrow_a_room_lifetime() {
        let mut world = World::new();
        let session = SessionScopeId(1);
        let root = world.spawn((SessionRoot(session), SessionScopedEntity(session))).id();
        let room = world.spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION, SessionScopedEntity(session))).id();
        let reply = ReplyTo {request: 0, selector: "test::events/result".into()};
        let session_action = begin_action(&mut world, root, "test::events/ask", "test", Some(&reply), 0).reply.unwrap();
        let room_action = begin_action(&mut world, room, "test::events/ask", "test", Some(&reply), 0).reply.unwrap();
        assert_ne!(session_action.recipient.reference(), room_action.recipient.reference());
        assert_eq!(session_action.ticket.room, None);
        assert_eq!(room_action.ticket.room, Some(0));
        world.despawn(room);
        let mut access = SystemState::<ActionReplies>::new(&mut world);
        assert!(access.get_mut(&mut world).unwrap().publish(Some(&session_action), ActionStage::Applied, true,
            "survived", "", Vec::new()));
        assert_eq!(session_action.recipient.resolve(&world), Some(root));
        assert_eq!(room_action.recipient.resolve(&world), None);
        world.despawn(root);
        assert_eq!(session_action.recipient.resolve(&world), None);
    }
}
