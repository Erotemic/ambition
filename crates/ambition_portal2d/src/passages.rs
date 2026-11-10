//! Publish this tick's pairs and straddlers in core geometry
//! ([`BodyGeometry`]), for a domain that is not the portal's (combat) to
//! tell where a volume is.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use ambition_platformer2d_shared_tangle::body::geometry::{BodyGeometry, MaterialBudget};

use crate::rooms::PortalsByRoom;
use crate::transit::PortalTransit;
use crate::tuning::PortalTuning;
use crate::types::{find_portal, PlacedPortal};

/// Each linked pair of `portals` (one room's, in their stable order) once,
/// from the end whose channel orders first: the pairs every reader of
/// [`BodyGeometry`] and the drawing of pieces cut by.
pub fn linked_pairs(portals: &[PlacedPortal]) -> Vec<(ae::frame::PortalAperture, ae::frame::PortalAperture)> {
    linked_pairs_iter(portals).map(|(a,b)|(a.aperture(),b.aperture())).collect()
}

fn linked_pairs_iter(portals:&[PlacedPortal]) -> impl Iterator<Item=(&PlacedPortal,PlacedPortal)> {
    portals
        .iter()
        .filter(|p| p.channel <= p.channel.partner())
        .filter_map(move |p| find_portal(portals, p.channel.partner()).map(|q| (p,q)))
}

/// Publish current apertures and occupancy before body integration.
pub fn publish_portal_apertures(
    portals: Query<(Entity, &PlacedPortal)>,
    transits: Query<(Entity, &PortalTransit)>,
    bodies: Query<(Entity, &ae::BodyKinematics, Option<&ae::SweepSample>)>,
    own_policies: Query<(Entity, &MaterialBudget)>,
    live: LiveRooms,
    tuning: Option<Res<PortalTuning>>,
    mut passages: ResMut<BodyGeometry>,
    // The rings last said, by room, so each is said once.
    mut rings_said: Local<Vec<(crate::rooms::PortalRoom, Vec<u32>)>>,
) {
    let tuning = tuning.as_deref().copied().unwrap_or_default();
    *passages = BodyGeometry::new(tuning.convention.map_convention()).with_default_budget(tuning.geometry.budget());
    for (body, own) in &own_policies {
        passages.set_body_budget(body, own.0);
    }
    write_materials(&mut passages, &bodies);
    let by_room = PortalsByRoom::collect(&portals, &live);
    let mut rings: Vec<(crate::rooms::PortalRoom, Vec<u32>)> = Vec::new();
    for (room, all) in by_room.rooms() {
        for (a,b) in linked_pairs_iter(all) {
            passages.add_pair_with_steps(room,[(a.aperture(),a.step),(b.aperture(),b.step)]);
        }
        // A ring of pairs (a pair facing itself, two doors in line) cuts a
        // volume in it again and again until the budget stops it: said once,
        // where pieces are composed.
        if tuning.geometry.budget().max_hops > 1 {
            let ring = ae::frame::thread::recursive_links(passages.links_in(room));
            if !ring.is_empty() {
                if !rings_said.iter().any(|(r, said)| *r == room && *said == ring) {
                    bevy::log::info!(
                        target: "ambition_portal::rings",
                        "the portals of room {room:?} make a ring (links {ring:?}): a volume in it is cut until {:?} stops it",
                        tuning.geometry
                    );
                }
                rings.push((room, ring));
            }
        }
    }
    *rings_said = rings;
    write_straddlers(&mut passages, &by_room, transits.iter().map(|(body, transit)| (body, live.of(body), *transit)).collect());
}

/// Refresh body occupancy after transit without changing the admitted apertures.
pub fn publish_portal_passages(
    portals: Query<(Entity, &PlacedPortal)>,
    transits: Query<(Entity, &PortalTransit)>,
    bodies: Query<(Entity, &ae::BodyKinematics, Option<&ae::SweepSample>)>,
    live: LiveRooms,
    mut passages: ResMut<BodyGeometry>,
) {
    write_materials(&mut passages, &bodies);
    let by_room = PortalsByRoom::collect(&portals, &live);
    write_straddlers(&mut passages, &by_room, transits.iter().map(|(body, transit)| (body, live.of(body), *transit)).collect());
}

fn write_straddlers(
    passages: &mut BodyGeometry,
    by_room: &PortalsByRoom,
    mut straddlers: Vec<(Entity, crate::rooms::PortalRoom, PortalTransit)>,
) {
    passages.clear_straddlers();
    straddlers.sort_by_key(|(body, _, _)| *body);
    for (body, room, transit) in straddlers {
        let all = by_room.in_room(room);
        if let (Some(front), Some(back)) = (
            find_portal(all, transit.straddling), find_portal(all, transit.straddling.partner()),
        ) { passages.add_straddler(body, front.aperture(), back.aperture()); }
    }
}


fn write_materials(
    passages: &mut BodyGeometry,
    bodies: &Query<(Entity, &ae::BodyKinematics, Option<&ae::SweepSample>)>,
) {
    passages.clear_materials();
    for (body, kin, sweep) in bodies {
        passages.set_body_material(body, kin.material_map(ae::SweepSample::down_or(sweep, ae::Vec2::Y)));
    }
}
