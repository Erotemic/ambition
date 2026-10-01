//! Who holds a live room live (OW4, first slice).
//!
//! A live room stays live while a participant's driven body is in it. That is
//! the rule a crossing uses to choose whether the room it leaves stays whole
//! (`another_player_stays` in the room transition commit) or is retired. This
//! module holds that rule once, as [`claims_on`], and derives from it the
//! read-only answer "which owners hold each live room" ([`live_room_claims`]),
//! which the `[census] rooms` instrument prints.
//!
//! ⛔ DERIVED AND READ-ONLY. A claim is not stored: it is read from the driven
//! bodies and their `InRoomInstance` stamps every time it is asked. So a
//! claim is released when its body leaves the room or stops being driven,
//! even when the producer that put it there no longer ticks, and a refused
//! crossing, which moves no body, releases nothing.

use bevy::prelude::Entity;

use ambition_characters::control::PlayerSlot;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;

/// One participant's claim on a live room: the body its slot drives there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoomClaim {
    pub slot: PlayerSlot,
    pub body: Entity,
}

/// The claims on live room `room`: each driven body stamped into it, with
/// the slot that drives it. `drivers` are the driven bodies now: each entity,
/// its slot, and its stamped live room (`None`: no stamp).
///
/// ⛔ THE STAMP, NOT `LiveRooms::of`. An unstamped body is not counted as the
/// sole live room's: the crossing rule has always asked for the stamp, and
/// this is that rule.
pub fn claims_on(
    room: LiveRoomInstance,
    drivers: impl IntoIterator<Item = (Entity, PlayerSlot, Option<LiveRoomInstance>)>,
) -> impl Iterator<Item = RoomClaim> {
    drivers
        .into_iter()
        .filter(move |(_, _, stamped)| *stamped == Some(room))
        .map(|(body, slot, _)| RoomClaim { slot, body })
}

/// Each live room in `rooms` and the claims on it, in room order, each list
/// sorted by slot and then body. A live room with no claim is listed with an
/// empty list: nothing holds it live.
pub fn live_room_claims(
    rooms: impl IntoIterator<Item = LiveRoomInstance>,
    drivers: impl IntoIterator<Item = (Entity, PlayerSlot, Option<LiveRoomInstance>)>,
) -> Vec<(LiveRoomInstance, Vec<RoomClaim>)> {
    let drivers: Vec<_> = drivers.into_iter().collect();
    let mut rooms: Vec<LiveRoomInstance> = rooms.into_iter().collect();
    rooms.sort();
    rooms.dedup();
    rooms
        .into_iter()
        .map(|room| {
            let mut claims: Vec<RoomClaim> = claims_on(room, drivers.iter().copied()).collect();
            claims.sort();
            (room, claims)
        })
        .collect()
}

/// [`live_room_claims`] of the live room roots and driven bodies in `world`.
pub fn live_room_claims_in(world: &mut bevy::prelude::World) -> Vec<(LiveRoomInstance, Vec<RoomClaim>)> {
    use ambition_characters::control::DrivingParticipant;
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, RoomInstanceRoot};
    let rooms: Vec<LiveRoomInstance> = world
        .query_filtered::<&LiveRoomInstance, bevy::prelude::With<RoomInstanceRoot>>()
        .iter(world)
        .copied()
        .collect();
    let drivers: Vec<(Entity, PlayerSlot, Option<LiveRoomInstance>)> = world
        .query::<(Entity, &DrivingParticipant, Option<&InRoomInstance>)>()
        .iter(world)
        .map(|(body, driver, room)| (body, driver.0, room.map(|room| room.0)))
        .collect();
    live_room_claims(rooms, drivers)
}
