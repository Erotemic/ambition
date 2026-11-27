//! Body material geometry and optional spatial restrictions for each live room.
//!
//! The snapshot contains aperture and traversal policy. Each query supplies
//! material in its current chart.

use ambition_platformer2d_core::frame::thread::{
    MaterialPrimitive, Partition, PieceBudget, PortalLink, Refusal, RefusalReason, partition, partition_reaching,
};
use ambition_platformer2d_core::frame::{MapConvention, PortalAperture, Sim2};
use ambition_platformer2d_core::{CombatVolume, Vec2};
use bevy::prelude::{Component, Entity, Resource};

use crate::lifecycle::LiveRoomInstance;

/// A body's material budget overrides the room budget.
/// An exhausted budget refuses the operation.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialBudget(pub PieceBudget);

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
pub struct BodyGeometry {
    convention: MapConvention,
    /// How far the volumes of a room with no budget of its own are cut.
    default_budget: PieceBudget,
    /// The bodies with a policy of their own ([`MaterialBudget`]).
    bodies: Vec<(Entity, PieceBudget)>,
    /// Each live room's links (each pair both ways, in the pairs' order) and
    /// how far its volumes are cut ([`PieceBudget`]).
    rooms: Vec<PassageRoom>,
    /// Each body in transit, and the link of the pair it straddles: in at the
    /// end whose front its centre is on.
    straddling: Vec<(Entity, PortalLink)>,
    /// Whether the movement kernel moves a body's root through the links it
    /// crosses (portal M2), and the exit assistance it gives; `None` leaves
    /// that to a transit after the step.
    kernel_reroot: Option<f32>,
    /// Links closed to a body's root crossing: the pair whose plane it cuts
    /// from behind.
    closed: Vec<(Entity, Vec<u32>)>,
    /// The sole live room, when there is exactly one: where a caller that
    /// names no room is, as `LiveRooms::of` places an unstamped entity. The
    /// pairs are filed by that rule, so a body with no room stamp of its own
    /// (a seated Smash fighter) meets the pairs of the one room it is in.
    sole_room: Option<LiveRoomInstance>,
}

#[derive(Clone, Debug)]
struct PassageRoom {
    room: Option<LiveRoomInstance>,
    links: Vec<PortalLink>,
    /// The material budget of this room.
    budget: PieceBudget,
}

impl Default for BodyGeometry {
    fn default() -> Self {
        Self::new(MapConvention::default())
    }
}

impl BodyGeometry {
    pub fn new(convention: MapConvention) -> Self {
        Self {
            convention,
            default_budget: PieceBudget::SINGLE_HOP,
            bodies: Vec::new(),
            rooms: Vec::new(),
            straddling: Vec::new(),
            kernel_reroot: None,
            closed: Vec::new(),
            sole_room: None,
        }
    }

    /// The sole live room (`LiveRooms::sole`), or `None` when there are none
    /// or several: a lookup that names no room reads this one's links.
    pub fn set_sole_room(&mut self, room: Option<LiveRoomInstance>) {
        self.sole_room = room;
    }

    /// Let the movement kernel move a body's root through each admitted link
    /// it crosses, continuing its step in the exit's chart (portal M2), with
    /// `min_exit_speed` of exit assistance; `None` leaves the root to transit.
    pub fn set_kernel_reroot(&mut self, min_exit_speed: Option<f32>) {
        self.kernel_reroot = min_exit_speed;
    }

    /// Whether the movement kernel moves bodies' roots through the links.
    pub fn kernel_reroots(&self) -> bool {
        self.kernel_reroot.is_some()
    }

    /// Close `keys` to `body`'s root crossing this step.
    pub fn close_for(&mut self, body: Entity, keys: Vec<u32>) {
        match self.closed.iter_mut().find(|(b, _)| *b == body) {
            Some((_, closed)) => closed.extend(keys),
            None => self.closed.push((body, keys)),
        }
    }

    fn closed_for(&self, body: Entity) -> &[u32] {
        self.closed.iter().find(|(b, _)| *b == body).map_or(&[], |(_, keys)| keys.as_slice())
    }

    /// Replace the current aperture restrictions.
    pub fn reset_apertures(&mut self, convention: MapConvention, budget: PieceBudget) {
        self.reset_apertures_except(convention, budget, |_| false, |_| false);
    }

    /// Replace the aperture restrictions of the population a publication
    /// covers, and keep the rest: each room `keep_room` names, and each body
    /// `keep_body` names, with its budget, closed links and straddle.
    ///
    /// A room's replay hides the other rooms while it runs (portal T3): a
    /// publisher then sees only the replayed room, and clearing everything
    /// would erase the publication of rooms that did not run, which nothing
    /// republishes before the next full tick.
    pub fn reset_apertures_except(
        &mut self,
        convention: MapConvention,
        budget: PieceBudget,
        keep_room: impl Fn(Option<LiveRoomInstance>) -> bool,
        keep_body: impl Fn(Entity) -> bool,
    ) {
        self.convention = convention;
        self.default_budget = budget;
        self.bodies.retain(|(body, _)| keep_body(*body));
        self.rooms.retain(|room| keep_room(room.room));
        self.straddling.retain(|(body, _)| keep_body(*body));
        self.closed.retain(|(body, _)| keep_body(*body));
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
        let room = room.or(self.sole_room);
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
        let room = room.or(self.sole_room);
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

    /// `body` has a policy of its own ([`MaterialBudget`]).
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

    pub fn add_straddler(&mut self, body: Entity, room: Option<LiveRoomInstance>, front: PortalAperture, back: PortalAperture) {
        let link = self
            .links_in(room)
            .iter()
            .find(|link| link.enter == front && link.exit == back)
            .copied()
            .unwrap_or(PortalLink::new(front,back,u32::MAX));
        self.straddling.push((body, link));
    }

    pub fn clear_straddlers(&mut self) { self.straddling.clear(); }

    /// Drop every straddle but those of the bodies `keep` names.
    pub fn retain_straddlers(&mut self, keep: impl Fn(Entity) -> bool) {
        self.straddling.retain(|(body, _)| keep(*body));
    }

    /// Whether any body is through a pair.
    pub fn has_straddlers(&self) -> bool {
        !self.straddling.is_empty()
    }

    pub fn motion_for(&self, body: Entity, room: Option<LiveRoomInstance>) -> Option<ambition_platformer2d_core::frame::thread::MotionAdmission<'_>> {
        let budget = self.budget_for(Some(body), room);
        use ambition_platformer2d_core::frame::thread::{Reroot, StepSpan};
        let span = StepSpan {
            reroot: self.kernel_reroot.map(|min_exit_speed| Reroot { closed: self.closed_for(body), min_exit_speed }),
            ..StepSpan::WHOLE
        };
        Some(ambition_platformer2d_core::frame::thread::MotionAdmission {
            links: self.links_in(room), first: self.straddled_by(body).map(|link| link.key),
            budget, convention: self.convention, span,
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

    /// A volume can be cut by a pair in its room or by a pair its carrier
    /// straddles. A volume stays whole when this returns false.
    pub fn cuts_in(&self, room: Option<LiveRoomInstance>, carrier: Option<Entity>) -> bool {
        !self.links_in(room).is_empty() || carrier.is_some_and(|carrier| self.straddled_by(carrier).is_some())
    }

    /// Cut a volume supplied in world coordinates.
    pub fn world_pieces(
        &self,
        volume: &CombatVolume,
        owner: Option<Entity>,
        root: Vec2,
        room: Option<LiveRoomInstance>,
    ) -> Partition {
        self.pieces_from(volume, PartSource::World { owner }, Sim2::IDENTITY, root, room)
    }

    /// Cut a world volume that moves with a body. The body supplies its map.
    pub fn body_world_pieces(
        &self,
        volume: &CombatVolume,
        body: Entity,
        material_map: Sim2,
        root: Vec2,
        room: Option<LiveRoomInstance>,
    ) -> Partition {
        self.pieces_from(volume, PartSource::Body(body), material_map, root, room)
    }

    /// Partition accepted body material that reaches from its owner.
    pub fn body_pieces_reaching(&self, material: MaterialPrimitive<'_>, body: Entity, room: Option<LiveRoomInstance>) -> Partition {
        self.body_pieces_from(material, body, true, room)
    }

    /// [`ambition_platformer2d_core::frame::thread::short_of_refusal`] over
    /// the links a volume `carrier` carries in `room` is cut by: what of a
    /// refused reach acts in front of the mouth that refused it.
    pub fn short_of_refusal(
        &self, cut: Partition, material: MaterialPrimitive<'_>, carrier: Option<Entity>, room: Option<LiveRoomInstance>,
    ) -> Partition {
        let mut links = self.links_in(room).to_vec();
        links.extend(carrier.and_then(|carrier| self.straddled_by(carrier)));
        ambition_platformer2d_core::frame::thread::short_of_refusal(cut, material, &links)
    }

    /// Partition the body's physical box through the current room apertures.
    pub fn physical_body_pieces(&self, body: Entity, kinematics: &ambition_platformer2d_core::BodyKinematics,
        room: Option<LiveRoomInstance>) -> Partition {
        let numeric = |fault| Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(fault) };
        let domain = kinematics.canonical_box().map_err(numeric)?;
        self.body_pieces(MaterialPrimitive { domain: &domain,
            to_world: kinematics.material_map().map_err(numeric)? }, body, room)
    }

    /// Partition accepted body material without a reach wedge.
    pub fn body_pieces(&self, material: MaterialPrimitive<'_>, body: Entity, room: Option<LiveRoomInstance>) -> Partition {
        self.body_pieces_from(material, body, false, room)
    }

    fn body_pieces_from(&self, material: MaterialPrimitive<'_>, body: Entity, reaching: bool, room: Option<LiveRoomInstance>) -> Partition {
        let numeric = |fault| Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(fault) };
        let root = material.to_world.point(Vec2::ZERO).map_err(numeric)?;
        self.cut_material(material, PartSource::Body(body), root, reaching, room)
    }

    fn pieces_from(
        &self,
        volume: &CombatVolume,
        source: PartSource,
        root_map: Sim2,
        root: Vec2,
        room: Option<LiveRoomInstance>,
    ) -> Partition {
        let numeric = |fault| Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(fault) };
        let canonical = ambition_platformer2d_core::frame::thread::map_volume(volume, &root_map.inverse().map_err(numeric)?)
            .map_err(numeric)?;
        let material = MaterialPrimitive { domain: &canonical, to_world: root_map };
        self.cut_material(material, source, root, false, room)
    }

    fn cut_material(
        &self,
        material: MaterialPrimitive<'_>,
        source: PartSource,
        root: Vec2,
        reaching: bool,
        room: Option<LiveRoomInstance>,
    ) -> Partition {
        let cut_by = if reaching { partition_reaching } else { partition };
        let carrier = source.carrier();
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
    use ambition_platformer2d_core::Aabb;
    use ambition_platformer2d_core::frame::PortalFrame;
    use ambition_platformer2d_core::frame::thread::VolumePiece;

    #[test]
    fn a_straddler_uses_its_rooms_link_when_other_apertures_are_identical() {
        let front = PortalAperture { frame: PortalFrame::fixed(Vec2::ZERO, -Vec2::Y), half_length: 40.0 };
        let back = PortalAperture { frame: PortalFrame::fixed(Vec2::new(500.0, -200.0), Vec2::Y), half_length: 40.0 };
        let (a, b) = (LiveRoomInstance::ACTIVATION, LiveRoomInstance::from_ordinal(1));
        let body = Entity::from_raw_u32(1).unwrap();
        let mut passages = BodyGeometry::default();
        passages.add_pair(Some(a), (front, back));
        let other_front = PortalAperture { frame: PortalFrame::fixed(Vec2::splat(1000.0), -Vec2::Y), ..front };
        let other_back = PortalAperture { frame: PortalFrame::fixed(Vec2::splat(1500.0), Vec2::Y), ..back };
        passages.add_pair(Some(b), (other_front, other_back));
        passages.add_pair(Some(b), (front, back));
        passages.add_straddler(body, Some(b), front, back);
        let expected = passages.links_in(Some(b))[2];
        assert_eq!(passages.straddled_by(body), Some(expected));
        assert_eq!(passages.motion_for(body, Some(b)).unwrap().first, Some(expected.key));
    }

    /// A volume is cut only by its own room's pairs and the pair its own
    /// carrier straddles: a body in transit elsewhere cuts nothing of it. And
    /// where the predicate says no, the pieces are whole.
    #[test]
    fn only_the_room_and_the_carrier_can_cut_a_volume() {
        let floor = PortalAperture { frame: PortalFrame::fixed(Vec2::ZERO, Vec2::new(0.0, -1.0)), half_length: 40.0 };
        let ceiling = PortalAperture { frame: PortalFrame::fixed(Vec2::new(500.0, -200.0), Vec2::new(0.0, 1.0)), half_length: 40.0 };
        let (here, there) = (LiveRoomInstance::ACTIVATION, LiveRoomInstance::from_ordinal(1));
        let (traveller, other) = (Entity::from_raw_u32(1).unwrap(), Entity::from_raw_u32(2).unwrap());
        let mut passages = BodyGeometry::default();
        passages.add_pair(Some(there), (floor, ceiling));
        passages.add_straddler(traveller, Some(there), floor, ceiling);

        assert!(passages.cuts_in(Some(there), None), "a room with a pair");
        assert!(passages.cuts_in(Some(here), Some(traveller)), "the carrier straddles a pair");
        assert!(!passages.cuts_in(Some(here), None), "a body in transit in another room cut a volume here");
        assert!(!passages.cuts_in(Some(here), Some(other)), "another body in transit cut this carrier's volume");

        // Across the floor opening: a carrier that straddles it is cut.
        let volume = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::splat(12.0)));
        for carrier in [None, Some(other), Some(traveller)] {
            let root = Vec2::new(0.0, -1.0);
            let pieces = match carrier {
                Some(body) => passages.body_world_pieces(&volume, body, Sim2::IDENTITY, root, Some(here)),
                None => passages.world_pieces(&volume, None, root, Some(here)),
            }.unwrap();
            let whole = pieces == vec![VolumePiece::whole(volume.clone())];
            assert_eq!(!passages.cuts_in(Some(here), carrier), whole, "{carrier:?}: the predicate and the pieces disagree");
        }
    }
}


#[cfg(test)]
mod material_tests {
    use super::*;

    #[test]
    fn physical_partition_uses_the_accepted_body_material_and_refuses_invalid_dimensions() {
        let body = Entity::from_raw_u32(1).unwrap();
        let geometry = BodyGeometry::default();
        let mut kinematics = ambition_platformer2d_core::BodyKinematics {
            pos: Vec2::new(30.0, 40.0), canonical_size: Vec2::new(20.0, 10.0),
            ..Default::default()
        };
        kinematics.material = kinematics.material.turned(std::f64::consts::FRAC_PI_2).unwrap();
        let pieces = geometry.physical_body_pieces(body, &kinematics, None).unwrap();
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].world_volume().bounds(), kinematics.aabb());
        assert_eq!(pieces[0].source().bounds(), ambition_platformer2d_core::Aabb::new(Vec2::ZERO, Vec2::new(10.0, 5.0)));
        kinematics.canonical_size = Vec2::ZERO;
        assert_eq!(geometry.physical_body_pieces(body, &kinematics, None).unwrap_err().reason,
            RefusalReason::Numeric(ambition_platformer2d_core::frame::NumericFault::InvalidScale));
    }

    #[test]
    fn a_carrier_supplies_material_axes_without_moving_world_geometry() {
        let body = Entity::from_raw_u32(1).unwrap();
        let root = Sim2::new(2.0, bevy::math::Mat2::from_cols(-Vec2::X, Vec2::Y), Vec2::new(100.0, 200.0)).unwrap();
        let canonical = CombatVolume::aabb(ambition_platformer2d_core::Aabb::new(Vec2::ZERO, Vec2::new(10.0, 20.0)));
        let world = ambition_platformer2d_core::frame::thread::map_volume(&canonical, &root).unwrap();
        let passages = BodyGeometry::default();
        let pieces = passages.body_world_pieces(&world, body, root, Vec2::new(100.0, 200.0), None).unwrap();
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].source(), &canonical);
        assert!(pieces[0].transform().reflected());
        assert_eq!(pieces[0].world_volume(), &world);
        assert_eq!(pieces[0].carry(Vec2::new(103.0, 205.0)).unwrap(), Vec2::new(103.0, 205.0));
        let independent = passages.world_pieces(&world, None, Vec2::new(100.0, 200.0), None).unwrap();
        assert_eq!(independent[0].source(), &world);
        assert!(!independent[0].transform().reflected());
    }
}
