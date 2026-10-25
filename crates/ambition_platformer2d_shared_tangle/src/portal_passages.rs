//! The portal pairs of each live room this tick, for a domain that is not the
//! portal's: where a volume is when a pair cuts it
//! (`docs/planning/engine/portals-a-body-in-two-places.md`).
//!
//! The portal domain writes it each tick after its transit; a composition with
//! no portals leaves it empty, and every volume is whole.

use ambition_platformer2d_core::frame::thread::{partition, PieceBudget, PortalLink, VolumePiece};
use ambition_platformer2d_core::frame::{MapConvention, PortalAperture};
use ambition_platformer2d_core::{CombatVolume, Vec2};
use bevy::prelude::{Component, Entity, Resource};

use crate::lifecycle::LiveRoomInstance;

/// A body's own portal geometry policy, over its room's: how far its volumes
/// are cut (`None`: not at all; it is moved whole at a crossing, and meets
/// one side). For a body or an ability that asks for more, or less, than the
/// room gives every body (`docs/planning/engine/portals-a-body-in-two-places.md`,
/// Policy).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortalPieceBudget(pub Option<PieceBudget>);

/// A drawable that reaches out from a point that is not its own centre: a
/// swing drawn in front of the body that swings it. The point is in engine
/// world coordinates. A portal pair carries the drawable when it reaches past
/// the entry from there, as the swing's volume is cut from its owner. Without
/// it, a drawable reaches out from its own centre.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ReachesFrom(pub Vec2);

#[derive(Resource, Clone, Debug)]
pub struct PortalPassages {
    convention: MapConvention,
    /// How far the volumes of a room with no budget of its own are cut.
    default_budget: Option<PieceBudget>,
    /// The bodies with a policy of their own ([`PortalPieceBudget`]).
    bodies: Vec<(Entity, Option<PieceBudget>)>,
    /// Each live room's links (each pair both ways, in the pairs' order) and
    /// how far its volumes are cut ([`PieceBudget`]).
    rooms: Vec<PassageRoom>,
    /// Each body in transit, and the link of the pair it straddles: in at the
    /// end whose front its centre is on.
    straddling: Vec<(Entity, PortalLink)>,
}

#[derive(Clone, Debug)]
struct PassageRoom {
    room: Option<LiveRoomInstance>,
    links: Vec<PortalLink>,
    /// `None`: the volumes of the room are not cut (each is whole where it is).
    budget: Option<PieceBudget>,
}

impl Default for PortalPassages {
    fn default() -> Self {
        Self::new(MapConvention::default())
    }
}

impl PortalPassages {
    pub fn new(convention: MapConvention) -> Self {
        Self {
            convention,
            default_budget: Some(PieceBudget::SINGLE_HOP),
            bodies: Vec::new(),
            rooms: Vec::new(),
            straddling: Vec::new(),
        }
    }

    /// How far the volumes of every room are cut, unless a room says
    /// otherwise ([`Self::set_budget`]).
    pub fn with_default_budget(mut self, budget: Option<PieceBudget>) -> Self {
        self.default_budget = budget;
        self
    }

    pub fn convention(&self) -> MapConvention {
        self.convention
    }

    fn room_mut(&mut self, room: Option<LiveRoomInstance>) -> &mut PassageRoom {
        let at = match self.rooms.iter().position(|r| r.room == room) {
            Some(at) => at,
            None => {
                let budget = self.default_budget;
                self.rooms.push(PassageRoom { room, links: Vec::new(), budget });
                self.rooms.len() - 1
            }
        };
        &mut self.rooms[at]
    }

    fn room(&self, room: Option<LiveRoomInstance>) -> Option<&PassageRoom> {
        self.rooms.iter().find(|r| r.room == room)
    }

    pub fn add_pair(&mut self, room: Option<LiveRoomInstance>, pair: (PortalAperture, PortalAperture)) {
        let links = &mut self.room_mut(room).links;
        let index = links.len() / 2;
        links.extend(PortalLink::both_ways(index, pair));
    }

    /// How far the volumes of `room` are cut; `None` leaves each whole.
    pub fn set_budget(&mut self, room: Option<LiveRoomInstance>, budget: Option<PieceBudget>) {
        self.room_mut(room).budget = budget;
    }

    pub fn budget_in(&self, room: Option<LiveRoomInstance>) -> Option<PieceBudget> {
        self.room(room).map_or(self.default_budget, |r| r.budget)
    }

    /// `body` has a policy of its own ([`PortalPieceBudget`]).
    pub fn set_body_budget(&mut self, body: Entity, budget: Option<PieceBudget>) {
        match self.bodies.iter_mut().find(|(b, _)| *b == body) {
            Some((_, b)) => *b = budget,
            None => self.bodies.push((body, budget)),
        }
    }

    /// How far the volumes of `owner` in `room` are cut: its own policy, else
    /// its room's.
    pub fn budget_for(&self, owner: Option<Entity>, room: Option<LiveRoomInstance>) -> Option<PieceBudget> {
        owner
            .and_then(|owner| self.bodies.iter().find(|(b, _)| *b == owner))
            .map_or_else(|| self.budget_in(room), |(_, budget)| *budget)
    }

    pub fn add_straddler(&mut self, body: Entity, front: PortalAperture, back: PortalAperture) {
        let link = self
            .rooms
            .iter()
            .flat_map(|r| &r.links)
            .find(|link| link.enter == front && link.exit == back)
            .copied()
            .unwrap_or(PortalLink { enter: front, exit: back, key: u32::MAX, depth: None });
        self.straddling.push((body, link));
    }

    /// The links of `room`: each pair of it both ways.
    pub fn links_in(&self, room: Option<LiveRoomInstance>) -> &[PortalLink] {
        self.room(room).map_or(&[], |r| &r.links)
    }

    /// The pairs of `room`, each once, from the end that orders first.
    pub fn pairs_in(&self, room: Option<LiveRoomInstance>) -> Vec<(PortalAperture, PortalAperture)> {
        self.links_in(room).iter().step_by(2).map(|link| (link.enter, link.exit)).collect()
    }

    /// The link of the pair `body` straddles: in at the end whose front its
    /// centre is on.
    pub fn straddled_by(&self, body: Entity) -> Option<PortalLink> {
        self.straddling.iter().find(|(b, _)| *b == body).map(|(_, link)| *link)
    }

    /// Can a volume in `room` that `carrier` carries be cut: has the room a
    /// pair, or does the carrier straddle one? These are the links
    /// [`Self::pieces`] cuts by, so a `false` here is a whole volume there. A
    /// body in transit in another room, or another body in transit, cuts
    /// nothing of this volume. (A body's own policy may still leave its
    /// volumes whole.)
    pub fn cuts_in(&self, room: Option<LiveRoomInstance>, carrier: Option<Entity>) -> bool {
        !self.links_in(room).is_empty() || carrier.is_some_and(|carrier| self.straddled_by(carrier).is_some())
    }

    /// Where `volume`, which comes from `root`, is in `room`: cut first by the
    /// pair its `owner` straddles (a volume the owner carries with it), and by
    /// every opening of the room it reaches through from that opening's
    /// front, as far as the owner's policy, else the room's, lets it; or
    /// whole. A cut the budget refuses leaves that opening solid to the
    /// piece: what would have gone through is not anywhere.
    pub fn pieces(
        &self,
        volume: &CombatVolume,
        owner: Option<Entity>,
        root: Vec2,
        room: Option<LiveRoomInstance>,
    ) -> Vec<VolumePiece> {
        let Some(budget) = self.budget_for(owner, room) else {
            return vec![VolumePiece::whole(volume.clone())];
        };
        let straddled = owner.and_then(|owner| self.straddled_by(owner));
        let links = self.links_in(room);
        let cut = match straddled {
            Some(link) if !links.contains(&link) => {
                let mut with: Vec<PortalLink> = links.to_vec();
                with.push(link);
                partition(volume, root, &with, Some(link.key), budget, self.convention)
            }
            _ => partition(volume, root, links, straddled.map(|l| l.key), budget, self.convention),
        };
        for refusal in &cut.refused {
            let kept = if refusal.whole {
                "it stays whole where it is, its part through the opening with it"
            } else {
                "its opening is solid to it"
            };
            tracing::debug!(
                target: "ambition_portal::refusal",
                "a volume of {owner:?} was not carried through link {} after {:?}: {kept} ({budget:?})",
                refusal.link,
                refusal.path
            );
        }
        cut.pieces
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::frame::PortalFrame;
    use ambition_platformer2d_core::Aabb;

    /// A volume is cut only by its own room's pairs and the pair its own
    /// carrier straddles: a body in transit elsewhere cuts nothing of it. And
    /// where the predicate says no, the pieces are whole.
    #[test]
    fn only_the_room_and_the_carrier_can_cut_a_volume() {
        let floor = PortalAperture { frame: PortalFrame::fixed(Vec2::ZERO, Vec2::new(0.0, -1.0)), half_length: 40.0 };
        let ceiling = PortalAperture { frame: PortalFrame::fixed(Vec2::new(500.0, -200.0), Vec2::new(0.0, 1.0)), half_length: 40.0 };
        let (here, there) = (LiveRoomInstance::ACTIVATION, LiveRoomInstance::from_ordinal(1));
        let (traveller, other) = (Entity::from_raw_u32(1).unwrap(), Entity::from_raw_u32(2).unwrap());
        let mut passages = PortalPassages::default();
        passages.add_pair(Some(there), (floor, ceiling));
        passages.add_straddler(traveller, floor, ceiling);

        assert!(passages.cuts_in(Some(there), None), "a room with a pair");
        assert!(passages.cuts_in(Some(here), Some(traveller)), "the carrier straddles a pair");
        assert!(!passages.cuts_in(Some(here), None), "a body in transit in another room cut a volume here");
        assert!(!passages.cuts_in(Some(here), Some(other)), "another body in transit cut this carrier's volume");

        // Across the floor opening: a carrier that straddles it is cut.
        let volume = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::splat(12.0)));
        for carrier in [None, Some(other), Some(traveller)] {
            let pieces = passages.pieces(&volume, carrier, Vec2::new(0.0, -1.0), Some(here));
            let whole = pieces == vec![VolumePiece::whole(volume.clone())];
            assert_eq!(!passages.cuts_in(Some(here), carrier), whole, "{carrier:?}: the predicate and the pieces disagree");
        }
    }
}
