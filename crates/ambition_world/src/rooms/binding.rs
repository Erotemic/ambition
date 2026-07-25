//! Room-graph references: does every authored link actually land somewhere?
//!
//! `RoomSet::from_parts` builds the transition graph from `RoomLink`s, and each
//! link names four things: a source room, a source zone, a target room, and a
//! target zone. Only the two ROOMS were ever checked, by an `eprintln!` that
//! nothing reads, and the two ZONES were not checked at all — the link's zone
//! strings went straight into the edge weight.
//!
//! What that costs shows up much later, at
//! [`transition_from_zone`](super::RoomSet): the edge matches by
//! `LoadingZone::id`, and a `to_zone` naming no zone in the target room makes
//! `zone_by_id` return `None`, so the function returns `None`, so the player
//! walks into the door and nothing happens. No panic, no log, no transition —
//! a door that goes nowhere, indistinguishable from a door you have not reached
//! the trigger box of yet.
//!
//! This is the cross-content graph check that `game/ambition_content` performs
//! on raw LDtk JSON. Doing it on the room IR instead means every provider gets
//! it, including the ones with no `.ldtk` file at all.

use ambition_platformer_primitives::binding::{
    BindingLedger, BindingReport, Namespace, Ref, Resolver,
};

use super::{RoomLink, RoomSpec};

/// The rooms a room set contains.
pub struct RoomId;

impl Namespace for RoomId {
    const NAME: &'static str = "room";
}

/// The loading zones of one room. Room-scoped: `east_gate` in one room is a
/// different zone from `east_gate` in another, which is exactly why a link
/// carries both a room and a zone.
pub struct LoadingZoneId;

impl Namespace for LoadingZoneId {
    const NAME: &'static str = "loading zone";
}

/// Resolve every endpoint of every link against the rooms it claims to join.
///
/// One report for the whole graph: an author who renamed a zone sees every link
/// that still points at the old name, rather than discovering them one dead door
/// at a time.
pub fn sweep_room_links(rooms: &[RoomSpec], links: &[RoomLink]) -> BindingReport {
    let room_ids: Resolver<RoomId> = Resolver::new(rooms.iter().map(|room| room.id.as_str()));
    let mut ledger = BindingLedger::new();

    for link in links {
        // Each end is resolved independently: a link with a bad room AND a bad
        // zone at the other end should report both, not stop at the first.
        for (room, zone, end) in [
            (&link.from_room, &link.from_zone, "from"),
            (&link.to_room, &link.to_zone, "to"),
        ] {
            let declared_by = format!(
                "link {}:{} -> {}:{} ({end})",
                link.from_room, link.from_zone, link.to_room, link.to_zone,
            );
            let Some(bound) = ledger.resolve(&room_ids, &Ref::new(room), declared_by.clone())
            else {
                // No room means no zone namespace to check the zone against;
                // reporting "unknown zone in a room that does not exist" would be
                // noise on top of the real defect.
                continue;
            };
            ledger.resolve(
                &zones_of(&rooms[bound.slot()]),
                &Ref::new(zone),
                declared_by,
            );
        }
    }

    ledger.finish()
}

/// The loading zones one room offers, by the `id` the graph edges match on.
fn zones_of(room: &RoomSpec) -> Resolver<LoadingZoneId> {
    Resolver::new(room.loading_zones.iter().map(|zone| zone.id.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::{LoadingZone, LoadingZoneActivation};
    use ambition_engine_core as ae;

    fn room(id: &str, zones: &[&str]) -> RoomSpec {
        let mut room = RoomSpec::new(
            id,
            ae::World::new(id, ae::Vec2::splat(500.0), ae::Vec2::ZERO, Vec::new()),
        );
        room.loading_zones = zones
            .iter()
            .map(|zone| LoadingZone {
                id: (*zone).to_owned(),
                name: format!("{zone} display"),
                activation: LoadingZoneActivation::Walk,
                aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)),
            })
            .collect();
        room
    }

    /// A link may be wrong at either end and in either half, and each way is a
    /// different fix. The zone half is the one that had no check at all.
    #[test]
    fn a_link_that_lands_nowhere_is_reported() {
        let rooms = vec![room("hall", &["east_gate"]), room("cellar", &["stair_top"])];
        let links = vec![
            // Good.
            RoomLink {
                from_room: "hall".into(),
                from_zone: "east_gate".into(),
                to_room: "cellar".into(),
                to_zone: "stair_top".into(),
                bidirectional: true,
            },
            // The zone was renamed; the link was not.
            RoomLink {
                from_room: "hall".into(),
                from_zone: "east_gate".into(),
                to_room: "cellar".into(),
                to_zone: "stair".into(),
                bidirectional: false,
            },
            // The room does not exist at all.
            RoomLink {
                from_room: "attic".into(),
                from_zone: "hatch".into(),
                to_room: "hall".into(),
                to_zone: "east_gate".into(),
                bidirectional: false,
            },
        ];

        let report = sweep_room_links(&rooms, &links);
        assert_eq!(report.len(), 2, "{report}");

        let found: Vec<_> = report
            .unresolved()
            .iter()
            .map(|u| (u.namespace, u.id.as_str()))
            .collect();
        assert_eq!(found, vec![("loading zone", "stair"), ("room", "attic")]);
        assert_eq!(
            report.unresolved()[0].did_you_mean.as_deref(),
            Some("stair_top"),
        );
    }
}
