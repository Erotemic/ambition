//! The portal pairs of each live room this tick, for a domain that is not the
//! portal's: where a volume is when a pair cuts it
//! (`docs/planning/engine/portals-a-body-in-two-places.md`).
//!
//! The portal domain publishes apertures before movement and refreshes body
//! material maps after transit. A composition with no portals has no links.

use ambition_platformer2d_core::frame::thread::{partition, partition_reaching, MaterialPrimitive, Partition, PieceBudget, PortalLink, Refusal, RefusalReason};
use ambition_platformer2d_core::frame::{MapConvention, NumericFault, PortalAperture, Sim2};
use ambition_platformer2d_core::{CombatVolume, Vec2};
use bevy::prelude::{Component, Entity, Resource};

use crate::lifecycle::LiveRoomInstance;

/// A body's material budget overrides the room budget.
/// An exhausted budget refuses the operation.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortalPieceBudget(pub PieceBudget);

/// The material chart and the owner of its traversal policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartSource {
    /// Use the body's material map, active transit and policy.
    Body(Entity),
    /// Use world material. The optional owner supplies only the policy.
    World { owner: Option<Entity> },
}

impl Default for PartSource {
    fn default() -> Self { Self::World { owner: None } }
}

impl PartSource {
    /// The body whose material map and active transit apply.
    pub fn carrier(self) -> Option<Entity> {
        match self { Self::Body(body) => Some(body), Self::World { .. } => None }
    }

    fn owner(self) -> Option<Entity> {
        match self { Self::Body(body) => Some(body), Self::World { owner } => owner }
    }
}

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
    materials: std::collections::BTreeMap<Entity, Result<Sim2, NumericFault>>,
    /// How far the volumes of a room with no budget of its own are cut.
    default_budget: PieceBudget,
    /// The bodies with a policy of their own ([`PortalPieceBudget`]).
    bodies: Vec<(Entity, PieceBudget)>,
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
    /// The material budget of this room.
    budget: PieceBudget,
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
            materials: Default::default(),
            default_budget: PieceBudget::SINGLE_HOP,
            bodies: Vec::new(),
            rooms: Vec::new(),
            straddling: Vec::new(),
        }
    }

    pub fn clear_materials(&mut self) { self.materials.clear(); }

    pub fn set_body_material(&mut self, body: Entity, material: Result<Sim2, NumericFault>) {
        self.materials.insert(body, material);
    }

    /// How far the volumes of every room are cut, unless a room says
    /// otherwise ([`Self::set_budget`]).
    pub fn with_default_budget(mut self, budget: PieceBudget) -> Self {
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
        let (a,b)=pair;
        self.add_pair_with_steps(room,[(a,ambition_platformer2d_core::frame::ApertureStep::fixed(a.frame.origin,a.frame.normal)),
            (b,ambition_platformer2d_core::frame::ApertureStep::fixed(b.frame.origin,b.frame.normal))]);
    }

    pub fn add_pair_with_steps(&mut self,room:Option<LiveRoomInstance>,pair:[(PortalAperture,ambition_platformer2d_core::frame::ApertureStep);2]) {
        let links = &mut self.room_mut(room).links;
        let index = links.len() / 2;
        links.extend(PortalLink::both_ways_with_steps(index,pair));
    }

    /// The material budget of `room`.
    pub fn set_budget(&mut self, room: Option<LiveRoomInstance>, budget: PieceBudget) {
        self.room_mut(room).budget = budget;
    }

    pub fn budget_in(&self, room: Option<LiveRoomInstance>) -> PieceBudget {
        self.room(room).map_or(self.default_budget, |r| r.budget)
    }

    /// `body` has a policy of its own ([`PortalPieceBudget`]).
    pub fn set_body_budget(&mut self, body: Entity, budget: PieceBudget) {
        match self.bodies.iter_mut().find(|(b, _)| *b == body) {
            Some((_, b)) => *b = budget,
            None => self.bodies.push((body, budget)),
        }
    }

    /// How far the volumes of `owner` in `room` are cut: its own policy, else
    /// its room's.
    pub fn budget_for(&self, owner: Option<Entity>, room: Option<LiveRoomInstance>) -> PieceBudget {
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
            .unwrap_or(PortalLink::new(front,back,u32::MAX));
        self.straddling.push((body, link));
    }

    pub fn clear_straddlers(&mut self) { self.straddling.clear(); }

    pub fn motion_for(&self, body: Entity, room: Option<LiveRoomInstance>) -> Option<ambition_platformer2d_core::frame::thread::MotionAdmission<'_>> {
        let budget = self.budget_for(Some(body), room);
        Some(ambition_platformer2d_core::frame::thread::MotionAdmission {
            links: self.links_in(room), first: self.straddled_by(body).map(|link| link.key),
            budget, convention: self.convention,
        })
    }

    /// The links of `room`: each pair of it both ways.
    pub fn links_in(&self, room: Option<LiveRoomInstance>) -> &[PortalLink] {
        self.room(room).map_or(&[], |r| &r.links)
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
    /// nothing of this volume.
    pub fn cuts_in(&self, room: Option<LiveRoomInstance>, carrier: Option<Entity>) -> bool {
        !self.links_in(room).is_empty() || carrier.is_some_and(|carrier| self.straddled_by(carrier).is_some())
    }

    /// Cut `volume` from `root` in `room`. A body source uses its material
    /// map and active transit. A world source uses world material. Both use
    /// their owner's policy, or the room policy when no owner is set.
    /// A refusal returns no partial geometry.
    pub fn pieces(
        &self,
        volume: &CombatVolume,
        source: PartSource,
        root: Vec2,
        room: Option<LiveRoomInstance>,
    ) -> Partition {
        self.pieces_from(volume, source, root, false, room)
    }

    /// [`Self::pieces`] for a volume that reaches from `root` (a strike from
    /// its owner): it goes through an opening in the wedge from `root`
    /// through the opening's ends ([`partition_reaching`]).
    pub fn pieces_reaching(
        &self,
        volume: &CombatVolume,
        source: PartSource,
        root: Vec2,
        room: Option<LiveRoomInstance>,
    ) -> Partition {
        self.pieces_from(volume, source, root, true, room)
    }

    fn pieces_from(
        &self,
        volume: &CombatVolume,
        source: PartSource,
        root: Vec2,
        reaching: bool,
        room: Option<LiveRoomInstance>,
    ) -> Partition {
        let cut_by = if reaching { partition_reaching } else { partition };
        let numeric = |fault| Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(fault) };
        let carrier = source.carrier();
        let root_map = carrier.and_then(|owner| self.materials.get(&owner)).copied()
            .unwrap_or(Ok(Sim2::IDENTITY)).map_err(numeric)?;
        let canonical = ambition_platformer2d_core::frame::thread::map_volume(volume, &root_map.inverse().map_err(numeric)?)
            .map_err(numeric)?;
        let material = MaterialPrimitive { domain: &canonical, to_world: root_map };
        let budget = self.budget_for(source.owner(), room);
        let straddled = carrier.and_then(|owner| self.straddled_by(owner));
        let links = self.links_in(room);
        let cut = match straddled {
            Some(link) if !links.contains(&link) => {
                let mut with: Vec<PortalLink> = links.to_vec();
                with.push(link);
                cut_by(material, root, &with, Some(link.key), budget, self.convention)
            }
            _ => cut_by(material, root, links, straddled.map(|l| l.key), budget, self.convention),
        };
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::frame::thread::VolumePiece;
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
            let pieces = passages.pieces(&volume, match carrier { Some(body) => PartSource::Body(body), None => PartSource::World { owner: None } }, Vec2::new(0.0, -1.0), Some(here)).unwrap();
            let whole = pieces == vec![VolumePiece::whole(volume.clone())];
            assert_eq!(!passages.cuts_in(Some(here), carrier), whole, "{carrier:?}: the predicate and the pieces disagree");
        }
    }
}


#[cfg(test)]
mod material_tests {
    use super::*;

    #[test]
    fn a_carrier_supplies_material_axes_without_moving_world_geometry() {
        let body = Entity::from_raw_u32(1).unwrap();
        let root = Sim2::new(2.0, bevy::math::Mat2::from_cols(-Vec2::X, Vec2::Y), Vec2::new(100.0, 200.0)).unwrap();
        let canonical = CombatVolume::aabb(ambition_platformer2d_core::Aabb::new(Vec2::ZERO, Vec2::new(10.0, 20.0)));
        let world = ambition_platformer2d_core::frame::thread::map_volume(&canonical, &root).unwrap();
        let mut passages = PortalPassages::default();
        passages.set_body_material(body, Ok(root));
        let pieces = passages.pieces(&world, PartSource::Body(body), Vec2::new(100.0, 200.0), None).unwrap();
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].source(), &canonical);
        assert!(pieces[0].transform().reflected());
        assert_eq!(pieces[0].world_volume(), &world);
        assert_eq!(pieces[0].carry(Vec2::new(103.0, 205.0)).unwrap(), Vec2::new(103.0, 205.0));
        let independent = passages.pieces(&world, PartSource::World { owner: None }, Vec2::new(100.0, 200.0), None).unwrap();
        assert_eq!(independent[0].source(), &world);
        assert!(!independent[0].transform().reflected());
    }
}
