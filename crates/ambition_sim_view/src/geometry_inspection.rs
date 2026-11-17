//! Room and subject context for read-only geometry reports.

use ambition_geometry_observation::{
    GeometryReport, ObservationContext, ObservationQuery, observe_all,
};
use ambition_platformer2d_core::{BodyKinematics, RoomGeometry};
use ambition_platformer2d_shared_tangle::{
    lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot, SessionScopedEntity},
    sim_id::SimId,
};
use bevy::prelude::*;

/// Bodies that can be selected, in stable identity order.
pub fn geometry_subjects(world: &mut World) -> Vec<(String, Entity)> {
    let mut subjects: Vec<_> = world
        .query_filtered::<(Entity, &SimId), With<BodyKinematics>>()
        .iter(world)
        .map(|(entity, id)| (id.as_str().to_owned(), entity))
        .collect();
    subjects.sort_by(|a, b| a.0.cmp(&b.0));
    subjects
}

/// Reject an absent room or a foreign subject before a provider can fall back
/// to another room. All geometry in the report uses this one context.
pub fn inspect_geometry(
    world: &mut World,
    query: ObservationQuery,
    wanted: impl Fn(&str) -> bool,
) -> Result<GeometryReport, String> {
    let root = query.room.ok_or("select a live room")?;
    if world.get::<RoomInstanceRoot>(root).is_none() {
        return Err("the selected room is not live".into());
    }
    let room = world
        .get::<LiveRoomInstance>(root)
        .copied()
        .ok_or("the room has no live identity")?;
    let geometry = world
        .get::<RoomGeometry>(root)
        .ok_or("the room has no collision geometry")?;
    let room_name = ambition_platformer2d_world::rooms::live_room_spec_of(world, root)
        .map_or_else(|| geometry.0.name.clone(), |spec| spec.id.clone());
    let subject = if let Some(subject) = query.subject {
        if world.get::<InRoomInstance>(subject).map(|at| at.0) != Some(room) {
            return Err("the selected subject is not in this live room".into());
        }
        Some(
            world
                .get::<SimId>(subject)
                .map_or_else(|| format!("{subject}"), |id| id.as_str().to_owned()),
        )
    } else {
        None
    };
    let context = ObservationContext {
        session: world
            .get::<SessionScopedEntity>(root)
            .map(|scope| scope.0.0),
        room: room_name,
        room_instance: room.ordinal(),
        subject,
        sim_tick: world
            .get_resource::<ambition_time::SimTick>()
            .map(|tick| tick.0),
    };
    Ok(GeometryReport {
        context,
        layers: observe_all(world, query, wanted),
    })
}
