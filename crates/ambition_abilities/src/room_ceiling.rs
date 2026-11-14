//! A room's withheld verbs, as a ceiling on each player's body (LEVEL-1-KIT).
//!
//! A level names the verbs a player does without there (`withheld_verbs`,
//! `RoomMetadata`). This source writes that ceiling into the body's
//! `AbilityContributions` from the room the body is in, each tick, and
//! withdraws it in a room that withholds nothing: the body's kit is untouched,
//! so it has every verb again when it leaves.

use bevy::prelude::*;

use ambition_platformer2d_core::{AbilityContribution, AbilityContributions, AbilitySet};

/// The room's key in a body's `AbilityContributions`.
pub const ROOM_WITHHELD_VERBS: &str = "room.withheld_verbs";

/// Each player's body takes its room's ceiling. A player only: the room's
/// own cast is authored for it.
pub fn withhold_the_rooms_verbs(
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mut bodies: Query<(Entity, &mut AbilityContributions), With<ambition_characters::control::DrivingParticipant>>,
) {
    for (body, mut contributions) in &mut bodies {
        // The LDtk owner refuses a list that names no verb, so `Err` is a
        // room built some other way; it withholds nothing.
        let wanted = rooms
            .spec_of(body)
            .and_then(|spec| spec.metadata.withheld_verbs.as_deref())
            .and_then(|withheld| AbilitySet::all_but_listed(withheld).ok())
            .map(AbilityContribution::Ceiling);
        if contributions.get(ROOM_WITHHELD_VERBS) == wanted {
            continue;
        }
        match wanted {
            Some(ceiling) => contributions.set(ROOM_WITHHELD_VERBS, ceiling),
            None => contributions.clear(ROOM_WITHHELD_VERBS),
        }
    }
}
