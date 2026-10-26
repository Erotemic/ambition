//! Read the live room selected by the primary body.

use crate::lifecycle::{LiveRoomInstance, LiveRooms};
use bevy::prelude::{Entity, With};

/// Read the primary body's room, or the sole live room when selection fails.
/// Developer gravity controls use this room. Music uses it to break a tie
/// between rooms with the same authored priority (Q150).
#[derive(bevy::ecs::system::SystemParam)]
pub struct PrimaryLiveRoom<'w, 's> {
    live: LiveRooms<'w, 's>,
    primary: bevy::prelude::Query<'w, 's, Entity, With<super::PrimaryBody>>,
}

impl PrimaryLiveRoom<'_, '_> {
    pub fn get(&self) -> Option<LiveRoomInstance> {
        self.primary
            .single()
            .ok()
            .and_then(|body| self.live.of(body))
            .or_else(|| self.live.sole())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::{InRoomInstance, RoomInstanceRoot};
    use bevy::ecs::system::SystemState;
    use bevy::prelude::World;

    fn selected(world: &mut World) -> Option<LiveRoomInstance> {
        let mut reader = SystemState::<PrimaryLiveRoom>::new(world);
        reader.get(world).expect("room query is valid").get()
    }

    #[test]
    fn primary_room_selection_refuses_ambiguity_and_falls_back_to_the_sole_room() {
        let mut world = World::new();
        let first = LiveRoomInstance::ACTIVATION;
        let second = first.next();
        assert_eq!(selected(&mut world), None);
        let first_root = world.spawn((RoomInstanceRoot, first)).id();
        assert_eq!(selected(&mut world), Some(first));
        world.spawn((RoomInstanceRoot, second));
        assert_eq!(selected(&mut world), None);
        let primary = world
            .spawn((super::super::PrimaryBody, InRoomInstance(second)))
            .id();
        assert_eq!(selected(&mut world), Some(second));
        world.entity_mut(primary).remove::<InRoomInstance>();
        assert_eq!(selected(&mut world), None);
        world.entity_mut(primary).insert(InRoomInstance(first));
        let duplicate = world
            .spawn((super::super::PrimaryBody, InRoomInstance(second)))
            .id();
        assert_eq!(selected(&mut world), None);
        world.despawn(duplicate);
        assert_eq!(selected(&mut world), Some(first));
        world.despawn(primary);
        assert_eq!(selected(&mut world), None);
        world.despawn(first_root);
        assert_eq!(selected(&mut world), Some(second));
    }
}
