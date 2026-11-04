//! Live room claims and retention for an admitted body roster.
//!
//! A participant's driven body holds its room. A transfer releases source
//! rooms only when no unselected body or participant keeps a claim there.
//! Claims are derived from bodies and room stamps, not stored separately.

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

/// Source rooms with no claim outside the selected bodies and participants.
pub fn rooms_released_by_transfer(
    sources: impl IntoIterator<Item = LiveRoomInstance>,
    members: impl IntoIterator<Item = (Option<Entity>, Option<PlayerSlot>)>,
    drivers: impl IntoIterator<Item = (Entity, PlayerSlot, Option<LiveRoomInstance>)>,
) -> std::collections::BTreeSet<LiveRoomInstance> {
    let members: Vec<_> = members.into_iter().collect();
    let mut released: std::collections::BTreeSet<_> = sources.into_iter().collect();
    for (body, slot, room) in drivers {
        if !members.iter().any(|(selected, participant)| *selected == Some(body) || *participant == Some(slot)) {
            if let Some(room) = room { released.remove(&room); }
        }
    }
    released
}

/// Room fate shared by preparation and publication for one admitted roster.
pub struct RoomTransferPlan {
    pub departing: Option<LiveRoomInstance>,
    pub joins: Option<LiveRoomInstance>,
    pub retiring: std::collections::BTreeSet<LiveRoomInstance>,
    pub other_rooms_stay: bool,
}

pub fn room_transfer_plan(
    members: impl IntoIterator<Item = (Option<Entity>, Option<PlayerSlot>, Option<LiveRoomInstance>)>,
    drivers: impl IntoIterator<Item = (Entity, PlayerSlot, Option<LiveRoomInstance>)>,
    rooms: impl IntoIterator<Item = (LiveRoomInstance, Option<ambition_platformer2d_world::rooms::LiveRoomDefinition>)>,
    target: Option<ambition_platformer2d_world::rooms::LiveRoomDefinition>,
    rebuild_source: bool,
    restart: bool,
) -> RoomTransferPlan {
    let members: Vec<_> = members.into_iter().collect();
    let mut rooms: Vec<_> = rooms.into_iter().collect();
    rooms.sort_by_key(|(room, _)| *room);
    let departing = members.first().and_then(|(_, _, room)| *room)
        .or_else(|| members.is_empty().then(|| rooms.iter()
            .find(|(_, definition)| target.is_some() && *definition == target).map(|(room, _)| *room)).flatten())
        .or_else(|| (rooms.len() == 1).then(|| rooms[0].0));
    let sources = members.iter().filter_map(|(_, _, room)| *room).chain(departing);
    let mut retiring = rooms_released_by_transfer(sources,
        members.iter().map(|(body, slot, _)| (*body, *slot)), drivers);
    if rebuild_source { retiring.extend(departing); }
    let joins = (!restart && !members.is_empty()).then(|| rooms.iter()
        .find(|(room, definition)| (!rebuild_source || Some(*room) != departing) && target.is_some() && *definition == target)
        .map(|(room, _)| *room)).flatten();
    if let Some(joins) = joins { retiring.remove(&joins); }
    if restart { retiring.extend(rooms.iter().map(|(room, _)| *room)); }
    let other_rooms_stay = rooms.iter().any(|(room, _)| !retiring.contains(room));
    RoomTransferPlan { departing, joins, retiring, other_rooms_stay }
}

#[cfg(test)]
mod transfer_tests {
    use super::*;
    use ambition_platformer2d_world::rooms::LiveRoomDefinition;

    #[test]
    fn a_roster_releases_only_sources_without_an_unselected_claim() {
        let first = LiveRoomInstance::ACTIVATION;
        let (second, target) = (first.next(), first.next().next());
        let body = |index| Entity::from_raw_u32(index).unwrap();
        let (a, b, c) = (body(1), body(2), body(3));
        let definition = |index| LiveRoomDefinition::from_index(index);
        let plan = room_transfer_plan(
            [(Some(a), Some(PlayerSlot(0)), Some(first)), (Some(b), Some(PlayerSlot(1)), Some(second))],
            [(a, PlayerSlot(0), Some(first)), (b, PlayerSlot(1), Some(second)), (c, PlayerSlot(2), Some(second))],
            [(first, Some(definition(0))), (second, Some(definition(1))), (target, Some(definition(2)))],
            Some(definition(2)), false, false,
        );
        assert_eq!(plan.departing, Some(first));
        assert_eq!(plan.joins, Some(target));
        assert_eq!(plan.retiring, [first].into_iter().collect());
        assert!(plan.other_rooms_stay);
    }

    #[test]
    fn a_named_bodyless_rebuild_selects_its_room_among_several_live_rooms() {
        let first = LiveRoomInstance::ACTIVATION;
        let second = first.next();
        let plan = room_transfer_plan([], [],
            [(first, Some(LiveRoomDefinition::from_index(0))), (second, Some(LiveRoomDefinition::from_index(1)))],
            Some(LiveRoomDefinition::from_index(1)), true, false);
        assert_eq!(plan.departing, Some(second));
        assert_eq!(plan.retiring, [second].into_iter().collect());
        assert_eq!(plan.joins, None);
        assert!(plan.other_rooms_stay);
    }
    #[test]
    fn ordinary_same_room_travel_joins_but_an_explicit_rebuild_retires_it() {
        let room = LiveRoomInstance::ACTIVATION;
        let definition = ambition_platformer2d_world::rooms::LiveRoomDefinition::from_index(0);
        let body = Entity::from_raw_u32(1).unwrap();
        for rebuild in [false, true] {
            let plan = room_transfer_plan([(Some(body), Some(PlayerSlot(0)), Some(room))],
                [(body, PlayerSlot(0), Some(room))], [(room, Some(definition))], Some(definition), rebuild, false);
            assert_eq!(plan.joins, if rebuild { None } else { Some(room) });
            assert_eq!(plan.retiring.contains(&room), rebuild);
        }
    }

}
