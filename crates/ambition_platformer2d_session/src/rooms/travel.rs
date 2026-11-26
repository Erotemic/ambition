//! Requests to move a body to a room's spawn point.

use bevy::prelude::*;

use ambition_platformer2d_core::{
    replay_messages::ReplayMessageReader, BodyKinematics, ConfirmedFrameBoundary,
};
use ambition_platformer2d_shared_tangle::lifecycle::{LiveBodies, LiveBodyId};
use ambition_platformer2d_world::rooms::LiveRoomSpecs;

use crate::lifecycle_commit::{LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent};

/// Request travel for the specified body. A host submits this with a host intent writer.
#[derive(Clone, Message)]
pub struct RoomTravelRequested {
    pub body: LiveBodyId,
    pub destination: String,
}

/// Send a valid request to the room transition loader.
pub fn admit_room_travel(
    mut requests: ReplayMessageReader<RoomTravelRequested>,
    identities: LiveBodies,
    rooms: LiveRoomSpecs,
    bodies: Query<
        Option<&ambition_characters::control::DrivingParticipant>,
        (
            With<BodyKinematics>,
            Without<ambition_combat::death_rules::OutOfPlay>,
            Without<super::passage::ZonePassage>,
        ),
    >,
    boundary: Option<Res<ConfirmedFrameBoundary>>,
    mut pending: ResMut<PendingLifecycleCommit>,
) {
    for request in requests.read() {
        if pending.peek().is_some() || request.body.room.is_none() {
            continue;
        }
        let Some(entity) = identities.entity_of(&request.body) else {
            continue;
        };
        let Ok(driver) = bodies.get(entity) else {
            continue;
        };
        if rooms.spec_of(entity).is_none() {
            continue;
        }
        let Some(destination) = rooms
            .rooms()
            .rooms
            .iter()
            .find(|room| room.id == request.destination)
        else {
            continue;
        };
        let intent = RoomTransitionIntent::to_room_spawn(
            request.body.clone(),
            driver.map(|driver| driver.0),
            destination,
        );
        let _ = pending.record(
            boundary.as_ref().map_or(0, |boundary| boundary.current),
            LifecycleIntent::Transition(intent),
        );
    }
}
