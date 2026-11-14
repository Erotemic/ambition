//! The portals of a room, for geometry observation (GOBS-3,
//! `docs/planning/engine/geometry-observability-and-f1.md`).
//!
//! `portal.apertures`: each placed portal's opening, the rectangle along its
//! face (`half_length` along the tangent, `half_thickness` out of it) turned
//! to its normal. It is active when transit would find its partner: the
//! partner channel placed in the same live room ([`find_portal`] over
//! [`PortalsByRoom`], as transit and the carve pair them). A lone end, and a
//! portal in a room whose rules do not use portals, are inactive with that
//! reason.

use std::borrow::Cow;

use ambition_geometry_observation::{
    GeometryObservation, GeometryObserverAppExt as _, ObservationQuery, ObservedShape, ObservedState, ObserverInfo,
};
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
use bevy::prelude::*;

use crate::rooms::{PortalRooms, PortalsByRoom};
use crate::types::{find_portal, PlacedPortal};

pub(crate) fn install(app: &mut App) {
    app.add_geometry_observer(
        ObserverInfo {
            name: "portal.apertures",
            description: "each placed portal's opening; a lone end, or one in a room without portals, is inactive",
            needs_subject: false,
        },
        apertures,
    );
}

fn apertures(
    query: In<ObservationQuery>,
    portals: Query<(Entity, &PlacedPortal)>,
    live: PortalRooms,
    roots: Query<&LiveRoomInstance>,
    bodies: Query<&InRoomInstance>,
) -> Vec<GeometryObservation> {
    let named = match (query.room, query.subject) {
        (Some(root), _) => Some(roots.get(root).ok().copied()),
        (None, Some(subject)) => Some(live.of(subject).or_else(|| bodies.get(subject).ok().map(|room| room.0))),
        (None, None) => None,
    };
    let paired = PortalsByRoom::collect(portals.iter(), &live);
    let mut rooms_seen = Vec::new();
    let mut out = Vec::new();
    for (entity, portal) in &portals {
        let room = live.of(entity);
        if named.is_some_and(|named| named != room) {
            continue;
        }
        if !rooms_seen.contains(&room) {
            rooms_seen.push(room);
        }
        let state = if !live.allows(entity) {
            ObservedState::Inactive { reason: Cow::Borrowed("the room's rules do not use portals") }
        } else if find_portal(paired.in_room(room), portal.channel.partner()).is_none() {
            ObservedState::Inactive { reason: Cow::Borrowed("its partner is not placed in this room") }
        } else {
            ObservedState::Active
        };
        out.push((room, portal, state));
    }
    let many_rooms = rooms_seen.len() > 1;
    out.into_iter()
        .map(|(room, portal, state)| {
            let key = match (many_rooms, room) {
                (true, Some(room)) => format!("{} in room {room:?}", portal.channel.name()),
                (true, None) => format!("{} in no told room", portal.channel.name()),
                (false, _) => portal.channel.name(),
            };
            GeometryObservation {
                key,
                roles: vec![Cow::Borrowed("portal"), Cow::Borrowed("aperture")],
                shape: opening(portal),
                state,
            }
        })
        .collect()
}

/// The opening's rectangle, its corners in turn.
fn opening(portal: &PlacedPortal) -> ObservedShape {
    let along = ambition_platformer2d_core::frame::tangent_of(portal.normal) * portal.opening_half_length();
    let out = portal.normal * portal.opening_half_thickness();
    let c = portal.pos;
    ObservedShape::Polygon {
        points: [c - along - out, c + along - out, c + along + out, c - along + out]
            .map(|p| [p.x, p.y])
            .to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_geometry_observation::{observe_all, ObserverAnswer};
    use ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot;

    use crate::color::{PortalChannel, PortalGunColor};

    const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
    const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::ORANGE);

    fn placed(channel: PortalChannel, x: f32) -> PlacedPortal {
        PlacedPortal::fixed(channel, Vec2::new(x, 200.0), Vec2::X, 24.0).unwrap()
    }

    fn observed(app: &mut App, root: Entity) -> Vec<GeometryObservation> {
        let layers = observe_all(app.world_mut(), ObservationQuery { room: Some(root), subject: None }, |name| {
            name == "portal.apertures"
        });
        match &layers[..] {
            [layer] => match &layer.answer {
                ObserverAnswer::Observed { geometry } => geometry.clone(),
                other => panic!("portal.apertures answered {other:?}"),
            },
            other => panic!("one layer was asked for, {} came back", other.len()),
        }
    }

    /// A pair in one room is two active openings, each a rectangle around its
    /// portal; an end alone in the other room is listed inactive, and the
    /// first room's list does not hold it.
    #[test]
    fn a_rooms_portals_are_observed_as_openings_and_a_lone_end_is_inactive() {
        let mut app = App::new();
        let live = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        let roots = live.map(|room| app.world_mut().spawn((RoomInstanceRoot, room)).id());
        install(&mut app);
        app.world_mut().spawn((placed(BLUE, 20.0), InRoomInstance(live[0])));
        app.world_mut().spawn((placed(ORANGE, 380.0), InRoomInstance(live[0])));
        app.world_mut().spawn((placed(BLUE, 20.0), InRoomInstance(live[1])));

        let first = observed(&mut app, roots[0]);
        assert_eq!(first.len(), 2, "the first room has its pair and nothing else: {first:?}");
        assert!(first.iter().all(|seen| seen.state == ObservedState::Active), "a pair is active: {first:?}");
        let blue = first.iter().find(|seen| seen.key == "blue").expect("the blue end, by its channel");
        let ObservedShape::Polygon { points } = &blue.shape else { panic!("an opening is a polygon: {blue:?}") };
        let xs = points.iter().map(|p| p[0]);
        let ys = points.iter().map(|p| p[1]);
        let (lo_y, hi_y) = ys.fold((f32::MAX, f32::MIN), |(lo, hi), y| (lo.min(y), hi.max(y)));
        assert_eq!((lo_y, hi_y), (176.0, 224.0), "a wall portal's opening runs 24 px each way along the wall");
        assert!(xs.clone().all(|x| (x - 20.0).abs() < 20.0), "the opening is at the portal: {points:?}");

        let second = observed(&mut app, roots[1]);
        assert_eq!(second.len(), 1, "{second:?}");
        assert!(
            matches!(second[0].state, ObservedState::Inactive { .. }),
            "a blue whose orange is in another room is a lone end: {second:?}"
        );
    }
}
