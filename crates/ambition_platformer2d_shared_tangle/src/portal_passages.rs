//! The portal pairs of each live room this tick, for a domain that is not the
//! portal's: where a volume is when a pair cuts it
//! (`docs/planning/engine/portals-a-body-in-two-places.md`).
//!
//! The portal domain writes it each tick after its transit; a composition with
//! no portals leaves it empty, and every volume is whole.

use ambition_platformer2d_core::frame::thread::{partition, PieceBudget, PortalLink, VolumePiece};
use ambition_platformer2d_core::frame::{MapConvention, PortalAperture};
use ambition_platformer2d_core::{CombatVolume, Vec2};
use bevy::prelude::{Entity, Resource};

use crate::lifecycle::LiveRoomInstance;

#[derive(Resource, Clone, Debug, Default)]
pub struct PortalPassages {
    convention: MapConvention,
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

impl PortalPassages {
    pub fn new(convention: MapConvention) -> Self {
        Self { convention, ..Self::default() }
    }

    pub fn convention(&self) -> MapConvention {
        self.convention
    }

    fn room_mut(&mut self, room: Option<LiveRoomInstance>) -> &mut PassageRoom {
        let at = match self.rooms.iter().position(|r| r.room == room) {
            Some(at) => at,
            None => {
                self.rooms.push(PassageRoom { room, links: Vec::new(), budget: Some(PieceBudget::SINGLE_HOP) });
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
        self.room(room).map_or(Some(PieceBudget::SINGLE_HOP), |r| r.budget)
    }

    pub fn add_straddler(&mut self, body: Entity, front: PortalAperture, back: PortalAperture) {
        let link = self
            .rooms
            .iter()
            .flat_map(|r| &r.links)
            .find(|link| link.enter == front && link.exit == back)
            .copied()
            .unwrap_or(PortalLink { enter: front, exit: back, key: u32::MAX });
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

    /// Can a volume in `room` be cut: has it a pair, or is a body in transit?
    pub fn cuts_in(&self, room: Option<LiveRoomInstance>) -> bool {
        self.budget_in(room).is_some() && (!self.links_in(room).is_empty() || !self.straddling.is_empty())
    }

    /// Where `volume`, which comes from `root`, is in `room`: cut first by the
    /// pair its `owner` straddles (a volume the owner carries with it), and by
    /// every opening of the room it reaches through from that opening's
    /// front, as far as the room's budget lets it; or whole.
    pub fn pieces(
        &self,
        volume: &CombatVolume,
        owner: Option<Entity>,
        root: Vec2,
        room: Option<LiveRoomInstance>,
    ) -> Vec<VolumePiece> {
        let Some(budget) = self.budget_in(room) else {
            return vec![VolumePiece::whole(volume.clone())];
        };
        let straddled = owner.and_then(|owner| self.straddled_by(owner));
        let links = self.links_in(room);
        match straddled {
            Some(link) if !links.contains(&link) => {
                let mut with: Vec<PortalLink> = links.to_vec();
                with.push(link);
                partition(volume, root, &with, Some(link.key), budget, self.convention).pieces
            }
            _ => partition(volume, root, links, straddled.map(|l| l.key), budget, self.convention).pieces,
        }
    }
}
