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

    /// Can a volume in `room` be cut: has it a pair, or is a body in transit?
    /// (A body's own policy may still leave its volumes whole:
    /// [`Self::pieces`].)
    pub fn cuts_in(&self, room: Option<LiveRoomInstance>) -> bool {
        !self.links_in(room).is_empty() || !self.straddling.is_empty()
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
            tracing::debug!(
                target: "ambition_portal::refusal",
                "a volume of {owner:?} was not carried through link {} after {:?}: its opening is solid to it ({budget:?})",
                refusal.link,
                refusal.path
            );
        }
        cut.pieces
    }
}
