//! Publish this tick's pairs and straddlers in core geometry
//! ([`PortalPassages`]), for a domain that is not the portal's (combat) to
//! tell where a volume is.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use ambition_platformer2d_shared_tangle::portal_passages::PortalPassages;

use crate::rooms::PortalsByRoom;
use crate::transit::PortalTransit;
use crate::tuning::PortalTuning;
use crate::types::{find_portal, PlacedPortal};

/// Rebuilt after the transit, so it says where each body is this tick: each
/// linked pair of each live room once, and the pair each body in transit
/// straddles, the end its centre is in front of first.
pub fn publish_portal_passages(
    portals: Query<(Entity, &PlacedPortal)>,
    transits: Query<(Entity, &PortalTransit)>,
    live: LiveRooms,
    tuning: Option<Res<PortalTuning>>,
    mut passages: ResMut<PortalPassages>,
) {
    let convention = tuning.map(|t| t.convention.map_convention()).unwrap_or_default();
    *passages = PortalPassages::new(convention);
    let by_room = PortalsByRoom::collect(&portals, &live);
    for (room, all) in by_room.rooms() {
        for p in all {
            let partner = p.channel.partner();
            // Each pair once: from the end whose channel orders first.
            if partner < p.channel {
                continue;
            }
            if let Some(q) = find_portal(all, partner) {
                passages.add_pair(room, (p.aperture(), q.aperture()));
            }
        }
    }
    let mut straddlers: Vec<(Entity, &PortalTransit)> = transits.iter().collect();
    // Query order is archetype order, which a resimulation may not repeat.
    straddlers.sort_by_key(|(body, _)| *body);
    for (body, transit) in straddlers {
        let all = by_room.in_room(live.of(body));
        if let (Some(front), Some(back)) =
            (find_portal(all, transit.straddling), find_portal(all, transit.straddling.partner()))
        {
            passages.add_straddler(body, front.aperture(), back.aperture());
        }
    }
}
