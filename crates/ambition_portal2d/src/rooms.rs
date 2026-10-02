//! Which live room a portal is in, and the portals of each live room (OW1).
//!
//! A portal pair is two portals in ONE live room. A body meets only the
//! portals of its own live room: transit, carving and the free-body teleport
//! group the placed portals by room before they pair them, so a portal in one
//! room is not a partner of a portal in another, and a body in one room does
//! not cross a portal of another room at the same coordinates.
//!
//! The room of a portal or a body is `LiveRooms::of`: its `InRoomInstance`
//! stamp, else the sole live room. With one live room every portal and every
//! body is in it, so the groups are the old single list.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms};

use crate::types::{PlacedPortal, PortalHostDepths};

/// The live room of a portal or a body. `None` is no live room that can be
/// told: with two live rooms, an unstamped entity. Such portals pair only with
/// each other, and such bodies meet only them.
pub type PortalRoom = Option<LiveRoomInstance>;

/// The placed portals of each live room, each room's list in
/// [`crate::stable_portal_order`].
#[derive(Debug, Default)]
pub struct PortalsByRoom(Vec<(PortalRoom, Vec<PlacedPortal>)>);

impl PortalsByRoom {
    /// Group `portals` by the room `live` gives each of them.
    pub fn collect<'a>(portals: impl IntoIterator<Item = (Entity, &'a PlacedPortal)>, live: &LiveRooms) -> Self {
        let mut rooms: Vec<(PortalRoom, Vec<PlacedPortal>)> = Vec::new();
        for (entity, portal) in portals {
            let room = live.of(entity);
            match rooms.iter_mut().find(|(seen, _)| *seen == room) {
                Some((_, list)) => list.push(portal.clone()),
                None => rooms.push((room, vec![portal.clone()])),
            }
        }
        // Sort here: downstream loops take the first match, and `Query` order
        // is archetype order, which a rollback resimulation may not reproduce.
        rooms.sort_by_key(|(room, _)| *room);
        for (_, list) in &mut rooms {
            list.sort_by(crate::stable_portal_order);
        }
        Self(rooms)
    }

    /// The portals of `room`, empty when it has none.
    pub fn in_room(&self, room: PortalRoom) -> &[PlacedPortal] {
        self.0
            .iter()
            .find(|(seen, _)| *seen == room)
            .map_or(&[], |(_, list)| list.as_slice())
    }

    /// Each room and its portals, in room order.
    pub fn rooms(&self) -> impl Iterator<Item = (PortalRoom, &[PlacedPortal])> {
        self.0.iter().map(|(room, list)| (*room, list.as_slice()))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The measured host depth of each portal, by live room: two rooms can each
/// hold a portal of one channel. Written by the host adapter, read by transit
/// and the carve. A room with no entry reads an empty table, which is
/// "unmeasured" for every channel.
#[derive(Resource, Clone, Debug, Default)]
pub struct PortalHostDepthsByRoom(pub Vec<(PortalRoom, PortalHostDepths)>);

impl PortalHostDepthsByRoom {
    /// The depth table of `room`.
    pub fn in_room(&self, room: PortalRoom) -> &PortalHostDepths {
        static UNMEASURED: PortalHostDepths = PortalHostDepths(Vec::new());
        self.0
            .iter()
            .find(|(seen, _)| *seen == room)
            .map_or(&UNMEASURED, |(_, depths)| depths)
    }

    /// Record `depth` for `channel` in `room`.
    pub fn push(&mut self, room: PortalRoom, channel: crate::PortalChannel, depth: f32) {
        match self.0.iter_mut().find(|(seen, _)| *seen == room) {
            Some((_, depths)) => depths.0.push((channel, depth)),
            None => self.0.push((room, PortalHostDepths(vec![(channel, depth)]))),
        }
    }
}

#[cfg(test)]
mod tests;
