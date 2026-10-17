//! The portal pairs of each live room this tick, for a domain that is not the
//! portal's: where a volume is when a pair cuts it
//! (`docs/planning/engine/portals-a-body-in-two-places.md`).
//!
//! The portal domain writes it each tick after its transit; a composition with
//! no portals leaves it empty, and every volume is whole.

use ambition_platformer2d_core::frame::thread::{cut_through, thread_volume, VolumePiece};
use ambition_platformer2d_core::frame::{MapConvention, PortalAperture};
use ambition_platformer2d_core::{CombatVolume, Vec2};
use bevy::prelude::{Entity, Resource};

use crate::lifecycle::LiveRoomInstance;

#[derive(Resource, Clone, Debug, Default)]
pub struct PortalPassages {
    convention: MapConvention,
    /// Each linked pair once, by the live room it is in.
    rooms: Vec<(Option<LiveRoomInstance>, Vec<(PortalAperture, PortalAperture)>)>,
    /// Each body in transit, and the pair it straddles: the end whose front
    /// its centre is on first.
    straddling: Vec<(Entity, PortalAperture, PortalAperture)>,
}

impl PortalPassages {
    pub fn new(convention: MapConvention) -> Self {
        Self { convention, ..Self::default() }
    }

    pub fn convention(&self) -> MapConvention {
        self.convention
    }

    pub fn add_pair(&mut self, room: Option<LiveRoomInstance>, pair: (PortalAperture, PortalAperture)) {
        match self.rooms.iter_mut().find(|(r, _)| *r == room) {
            Some((_, pairs)) => pairs.push(pair),
            None => self.rooms.push((room, vec![pair])),
        }
    }

    pub fn add_straddler(&mut self, body: Entity, front: PortalAperture, back: PortalAperture) {
        self.straddling.push((body, front, back));
    }

    pub fn pairs_in(&self, room: Option<LiveRoomInstance>) -> &[(PortalAperture, PortalAperture)] {
        self.rooms.iter().find(|(r, _)| *r == room).map_or(&[], |(_, pairs)| pairs)
    }

    /// The pair `body` straddles: the end whose front its centre is on, then
    /// its partner.
    pub fn straddled_by(&self, body: Entity) -> Option<(PortalAperture, PortalAperture)> {
        self.straddling.iter().find(|(b, ..)| *b == body).map(|(_, front, back)| (*front, *back))
    }

    /// Can a volume in `room` be cut: has it a pair, or is a body in transit?
    pub fn cuts_in(&self, room: Option<LiveRoomInstance>) -> bool {
        !self.pairs_in(room).is_empty() || !self.straddling.is_empty()
    }

    /// Where `volume`, which comes from `root`, is in `room`: cut by the pair
    /// its `owner` straddles (a volume the owner carries with it), or by the
    /// first opening of the room it reaches through from that opening's front,
    /// or whole.
    pub fn pieces(
        &self,
        volume: &CombatVolume,
        owner: Option<Entity>,
        root: Vec2,
        room: Option<LiveRoomInstance>,
    ) -> Vec<VolumePiece> {
        match owner.and_then(|owner| self.straddled_by(owner)) {
            Some((front, back)) => cut_through(volume, &front, &back, self.convention),
            None => thread_volume(volume, root, self.pairs_in(room), self.convention),
        }
    }
}
