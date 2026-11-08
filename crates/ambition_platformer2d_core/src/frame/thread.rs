//! Where a volume is when portal pairs cut it
//! (`docs/planning/engine/portals-a-body-in-two-places.md`).
//!
//! A volume that crosses the plane of a portal inside its opening, coming from
//! the portal's front, is in two places: the part in front stays, and the part
//! past the plane is at the linked portal, mapped through the pair. A part past
//! the plane and beside the opening stays where it is. A part carried through
//! one pair can cross another, and is cut again ([`partition`]).
//!
//! A volume either moves through whole or reaches from a point. A body (a
//! hurtbox, a shot, a thrown thing) moves through square on: its part past
//! the plane between the opening's ends goes through, and the snag holds it
//! to fitting the opening ([`partition`]). A swing reaches from its owner
//! ([`partition_reaching`]): its part past the plane went through where the
//! line from the owner to it crosses the plane inside the opening, the wedge
//! from the owner through the ends ([`opening_from`]), as an eye sees through
//! a window. What it swept beside the opening met the host.
//!
//! A piece is the whole shape placed in its chart and the half planes that
//! bound the part of it that is there, so the test is exact for every
//! [`CombatVolume`]: a circle cut by a plane stays a cut circle.
//!
//! A pair is two portals of one live room (`ambition_portal2d::rooms`), so
//! every piece of a volume is in its root's room; its chart is that room's
//! world frame.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;

use bevy_math::DVec2;

use super::{Iso2, MapConvention, NumericFault, PortalAperture, PortalFrame, Sim2};
use crate::{AabbExt, CombatVolume, Vec2};

/// Slack (px) on a point's side of a plane: a point on the plane is on both
/// sides.
const EPS: f32 = 1e-3;

/// The points `p` with `(p − point) · inward ≥ 0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HalfPlane {
    pub point: Vec2,
    pub inward: Vec2,
}

impl HalfPlane {
    fn side(&self, p: Vec2) -> f32 {
        (p - self.point).dot(self.inward)
    }

    pub fn contains(&self, p: Vec2) -> bool {
        self.side(p) >= -EPS
    }

    /// The other side of the same line.
    pub fn flipped(&self) -> Self {
        Self { point: self.point, inward: -self.inward }
    }

    /// The half plane `map` takes this one to.
    pub fn mapped(&self, map: &Sim2) -> Result<Self, NumericFault> {
        if !self.inward.is_finite() || self.inward.length_squared() <= 0.0 {
            return Err(NumericFault::InvalidFrame);
        }
        Ok(Self { point: map.point(self.point)?, inward: map.direction(self.inward.normalize()) })
    }
}

/// A canonical cut: `inward.dot(point) >= offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialPlane {
    pub inward: DVec2,
    pub offset: f64,
}

impl From<HalfPlane> for MaterialPlane {
    fn from(plane: HalfPlane) -> Self {
        let inward = plane.inward.as_dvec2();
        Self { inward, offset: inward.dot(plane.point.as_dvec2()) }
    }
}

impl MaterialPlane {
    fn side(self, point: DVec2) -> f64 { point.dot(self.inward) - self.offset }

    fn error_at(self, point: DVec2) -> f64 {
        (point.abs().dot(self.inward.abs()) + self.offset.abs()) * f64::EPSILON * 8.0
    }

    fn validate(self) -> Result<Self, NumericFault> {
        if !self.inward.is_finite() || !self.offset.is_finite() { return Err(NumericFault::NonFinite); }
        if self.inward.length_squared() == 0.0 { return Err(NumericFault::InvalidFrame); }
        Ok(self)
    }

    fn mapped(self, map: &Sim2) -> Result<Self, NumericFault> {
        self.validate()?;
        self.projected(map).validate()
    }

    fn projected(self, map: &Sim2) -> Self {
        let inward = map.direction_wide(self.inward);
        Self { inward, offset: self.offset * map.scale_wide() + inward.dot(map.translation()) }
    }

    fn pulled_back(self, map: &Sim2) -> Result<Self, NumericFault> {
        self.validate()?;
        Self { inward: map.inverse_direction_wide(self.inward),
            offset: (self.offset - self.inward.dot(map.translation())) / map.scale_wide() }.validate()
    }

    fn in_world(self, map: &Sim2) -> Result<HalfPlane, NumericFault> {
        let plane = self.mapped(map)?;
        let point = plane.inward * (plane.offset / plane.inward.length_squared());
        Ok(HalfPlane { point: Sim2::IDENTITY.point_from_wide(point)?, inward: plane.inward.as_vec2() })
    }
}

/// One way through a pair: in at `enter`, out at `exit`. `key` orders the
/// links of a room (the passages give each end of each pair its own) and
/// names the link in a piece's path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalLink {
    pub enter: PortalAperture,
    pub exit: PortalAperture,
    pub key: u32,
    /// How deep past the entry plane what goes through goes; deeper stays
    /// where it is. `None`: all of it past the plane. A host's picture is cut
    /// by the pair its limb is in as deep as the limb reaches
    /// (`PortalHostReach`).
    pub depth: Option<f32>,
}

impl PortalLink {
    /// Both ways through the pair `(a, b)`: `a → b` keyed `2 * index`, `b → a`
    /// keyed `2 * index + 1`.
    pub fn both_ways(index: usize, (a, b): (PortalAperture, PortalAperture)) -> [PortalLink; 2] {
        let key = 2 * index as u32;
        [
            PortalLink { enter: a, exit: b, key, depth: None },
            PortalLink { enter: b, exit: a, key: key + 1, depth: None },
        ]
    }

    /// The map of the link.
    pub fn map(&self, convention: MapConvention) -> Result<Sim2, NumericFault> {
        Sim2::through(&self.enter, &self.exit, convention)
    }

    /// The half planes of what goes through, at the entry: past the plane,
    /// between the ends (in the wedge from `reach`, for a volume that reaches
    /// from it), and no deeper than [`Self::depth`].
    fn through_from(&self, reach: Option<Vec2>) -> impl Iterator<Item = HalfPlane> {
        entry_opening(&self.enter, reach).into_iter().chain(self.deepest(&self.enter))
    }

    /// The half plane no deeper than [`Self::depth`] past `ap`'s plane.
    fn deepest(&self, ap: &PortalAperture) -> Option<HalfPlane> {
        self.depth.map(|d| HalfPlane { point: ap.frame.origin - ap.frame.normal * d, inward: ap.frame.normal })
    }
}

/// How many times a volume may be cut and carried: the policy of the room or
/// the body (`docs/planning/engine/portals-a-body-in-two-places.md`, Policy).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PieceBudget {
    /// The cumulative scale stays in [2^-exponent, 2^exponent].
    pub max_scale_exponent: u8,
    /// Pairs a piece may be carried through, one after another.
    pub max_hops: u8,
    /// Refuse a candidate that needs more parts. Zero admits no partition.
    pub max_pieces: u16,
}

impl PieceBudget {
    /// One pair: a piece carried through a pair is not cut again.
    pub const SINGLE_HOP: Self = Self { max_hops: 1, max_pieces: 16, max_scale_exponent: 12 };
    /// Pairs in sequence and side by side: a chain of four, a body across a
    /// handful.
    pub const COMPOSED: Self = Self { max_hops: 4, max_pieces: 32, max_scale_exponent: 12 };
}

/// A canonical primitive and its root placement in the world.
#[derive(Clone, Copy)]
pub struct MaterialPrimitive<'a> {
    pub domain: &'a CombatVolume,
    pub to_world: Sim2,
}

impl<'a> MaterialPrimitive<'a> {
    /// World-authored effects use the world itself as their source chart.
    pub fn world(domain: &'a CombatVolume) -> Self {
        Self { domain, to_world: Sim2::IDENTITY }
    }
}

/// Canonical material in one chart. World geometry is a private projection.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumePiece {
    source: Arc<CombatVolume>,
    source_bounds: Vec<MaterialPlane>,
    to_piece: Sim2,
    root_map: Sim2,
    volume: CombatVolume,
    bounds: Vec<HalfPlane>,
    pub path: Vec<u32>,
}

impl VolumePiece {
    pub fn whole(volume: CombatVolume) -> Self {
        Self {
            source: Arc::new(volume.clone()), source_bounds: Vec::new(),
            to_piece: Sim2::IDENTITY, root_map: Sim2::IDENTITY, volume, bounds: Vec::new(), path: Vec::new(),
        }
    }

    pub fn from_material(source: MaterialPrimitive<'_>) -> Result<Self, NumericFault> {
        Self::projected(Arc::new(source.domain.clone()), Vec::new(), source.to_world, source.to_world, Vec::new())
    }

    /// Change the root chart while each material region stays in place.
    pub fn rebased_through(&self, link: &PortalLink, convention: MapConvention) -> Result<Self, NumericFault> {
        let mut rebased = self.clone();
        rebased.root_map = self.root_map.then(link.map(convention)?)?;
        if rebased.path.first() == Some(&link.key) {
            rebased.path.remove(0);
        } else {
            rebased.path.insert(0, link.key ^ 1);
        }
        Ok(rebased)
    }

    pub fn source(&self) -> &CombatVolume { &self.source }
    pub fn source_bounds(&self) -> &[MaterialPlane] { &self.source_bounds }
    pub fn transform(&self) -> Sim2 { self.to_piece }
    pub fn path_transform(&self) -> Result<Sim2, NumericFault> {
        self.root_map.inverse()?.then(self.to_piece)
    }
    pub fn root_bounds(&self) -> Result<Vec<HalfPlane>, NumericFault> {
        self.source_bounds.iter().map(|plane| plane.in_world(&self.root_map)).collect()
    }
    pub fn world_volume(&self) -> &CombatVolume { &self.volume }
    pub fn world_bounds(&self) -> &[HalfPlane] { &self.bounds }
    pub fn is_carried(&self) -> bool { !self.path.is_empty() }

    /// Points and vectors from a producer use its root world chart.
    pub fn carry(&self, p: Vec2) -> Result<Vec2, NumericFault> { self.path_transform()?.point(p) }
    pub fn carry_dir(&self, v: Vec2) -> Result<Vec2, NumericFault> { self.path_transform()?.vector(v) }
    pub fn uncarry(&self, p: Vec2) -> Result<Vec2, NumericFault> { self.path_transform()?.inverse()?.point(p) }
    pub fn uncarry_dir(&self, v: Vec2) -> Result<Vec2, NumericFault> { self.path_transform()?.inverse()?.vector(v) }

    pub fn is_empty(&self) -> bool {
        !self.source_bounds.is_empty() && !holds(&self.source, self.source_bounds.iter().copied())
    }

    /// Polygon regions are exact. A circle keeps its analytic contact test.
    pub fn source_region(&self) -> Option<Vec<Vec2>> {
        let outline = self.source.polygon().unwrap_or_else(|| box_outline(self.source.bounds()));
        cut(outline, &self.source_bounds)
    }

    pub fn region(&self) -> Option<Vec<Vec2>> {
        let outline = self.source.polygon().unwrap_or_else(|| box_outline(self.source.bounds()));
        cut_wide(outline, &self.source_bounds)?.into_iter().map(|p| self.to_piece.point_from_wide(p)).collect::<Result<Vec<_>, _>>().ok()
    }

    pub fn center(&self) -> Vec2 {
        self.region().map_or(self.volume.center(), |region| {
            region.iter().copied().sum::<Vec2>() / region.len() as f32
        })
    }

    pub fn meets(&self, other: &VolumePiece) -> bool {
        if let (CombatVolume::Circle { center: a, radius: ra }, CombatVolume::Circle { center: b, radius: rb }) = (&*self.source, &*other.source) {
            let a = (self.to_piece.point_wide(a.as_dvec2()), f64::from(*ra) * self.to_piece.scale_wide());
            let b = (other.to_piece.point_wide(b.as_dvec2()), f64::from(*rb) * other.to_piece.scale_wide());
            let planes: Vec<_> = self.source_bounds.iter().map(|plane| plane.projected(&self.to_piece))
                .chain(other.source_bounds.iter().map(|plane| plane.projected(&other.to_piece))).collect();
            return circles_meet(a, b, &planes);
        }
        if self.bounds.is_empty() && other.bounds.is_empty() {
            return self.volume.intersects(&other.volume);
        }
        let (a, b) = (self.volume.bounds(), other.volume.bounds());
        if a.min.x > b.max.x || b.min.x > a.max.x || a.min.y > b.max.y || b.min.y > a.max.y {
            return false;
        }
        let planes: Vec<HalfPlane> = self.bounds.iter().chain(&other.bounds).copied().collect();
        if let Some(outline) = other.volume.polygon() {
            return cut(outline, &planes).is_some_and(|p| self.volume.intersects(&CombatVolume::convex(p)));
        }
        if let Some(outline) = self.volume.polygon() {
            return cut(outline, &planes).is_some_and(|p| other.volume.intersects(&CombatVolume::convex(p)));
        }
        unreachable!("only a circle has no polygon")
    }

    pub fn active_bounds(&self) -> Vec<HalfPlane> {
        let outline = self.source.polygon().unwrap_or_else(|| box_outline(self.source.bounds()));
        let mut kept: Vec<usize> = (0..self.source_bounds.len()).collect();
        let mut i = 0;
        while i < kept.len() {
            let without: Vec<MaterialPlane> = kept.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, index)| self.source_bounds[*index]).collect();
            if cut_wide(outline.clone(), &without).is_some_and(|region| region.iter().any(|p| {
                let plane = self.source_bounds[kept[i]];
                plane.side(*p) < -plane.error_at(*p)
            })) {
                i += 1;
            } else {
                kept.remove(i);
            }
        }
        kept.into_iter().map(|index| self.bounds[index]).collect()
    }

    fn projected(
        source: Arc<CombatVolume>, source_bounds: Vec<MaterialPlane>, to_piece: Sim2, root_map: Sim2, path: Vec<u32>,
    ) -> Result<Self, NumericFault> {
        let volume = map_volume(&source, &to_piece)?;
        let bounds = source_bounds.iter().map(|plane| plane.in_world(&to_piece)).collect::<Result<Vec<_>, _>>()?;
        Ok(Self { source, source_bounds, to_piece, root_map, volume, bounds, path })
    }

    fn bounded(&self, more: &[HalfPlane]) -> Result<Self, NumericFault> {
        let mut source_bounds = self.source_bounds.clone();
        source_bounds.extend(more.iter().map(|plane| MaterialPlane::from(*plane).pulled_back(&self.to_piece)).collect::<Result<Vec<_>, _>>()?);
        Self::projected(self.source.clone(), source_bounds, self.to_piece, self.root_map, self.path.clone())
    }

    fn carried(&self, map: Sim2, link: u32) -> Result<Self, NumericFault> {
        let mut path = self.path.clone();
        path.push(link);
        Self::projected(self.source.clone(), self.source_bounds.clone(), self.to_piece.then(map)?, self.root_map, path)
    }
}

/// Derive world geometry from canonical material and one admitted map.
pub fn map_volume(volume: &CombatVolume, map: &Sim2) -> Result<CombatVolume, NumericFault> {
    let bounds = volume.bounds();
    if !bounds.min.is_finite() || !bounds.max.is_finite() {
        return Err(NumericFault::NonFinite);
    }
    for corner in box_corners(bounds) { map.point(corner)?; }
    let dimensions = bounds.max - bounds.min;
    let feature = dimensions.min_element();
    if feature > 0.0 { map.admits_resolution(volume.center(), feature)?; }
    Ok(match volume {
        CombatVolume::Aabb(a) => boxed(map.point(a.center())?, a.half_size() * map.scale(), map.direction(Vec2::X)),
        CombatVolume::Obb { center, half, rotation } => {
            boxed(map.point(*center)?, *half * map.scale(), map.direction(Vec2::from_angle(*rotation)))
        }
        CombatVolume::Circle { center, radius } => CombatVolume::circle(map.point(*center)?, map.length(*radius)?),
        CombatVolume::Convex { points, .. } => CombatVolume::convex(points.iter().map(|p| map.point(*p)).collect::<Result<Vec<_>, _>>()?),
    })
}

/// A rigid frame map for callers whose geometry has no aperture scale.
pub fn carry_volume(volume: &CombatVolume, enter: &PortalFrame, exit: &PortalFrame, convention: MapConvention) -> Result<CombatVolume, NumericFault> {
    let rigid = Iso2::through(enter, exit, convention);
    let map = Sim2::new(1.0, rigid.linear, rigid.translation)?;
    map_volume(volume, &map)
}

/// A box about `center` whose own x axis is `x_axis`: one along a world axis
/// stays an axis-aligned box (a floor, wall or ceiling pair maps it exactly).
fn boxed(center: Vec2, half: Vec2, x_axis: Vec2) -> CombatVolume {
    if x_axis.y.abs() < 1e-6 {
        CombatVolume::aabb(crate::Aabb::new(center, half))
    } else if x_axis.x.abs() < 1e-6 {
        CombatVolume::aabb(crate::Aabb::new(center, Vec2::new(half.y, half.x)))
    } else {
        CombatVolume::obb(center, half, x_axis.y.atan2(x_axis.x))
    }
}

/// The half planes of an opening: behind its plane, and between its ends.
pub fn opening(ap: &PortalAperture) -> [HalfPlane; 3] {
    let (o, n, t, h) = (ap.frame.origin, ap.frame.normal, ap.frame.tangent(), ap.half_length);
    [
        HalfPlane { point: o, inward: -n },
        HalfPlane { point: o - t * h, inward: t },
        HalfPlane { point: o + t * h, inward: -t },
    ]
}

/// The half planes of what goes through `ap` from `root`: past its plane, and
/// in the wedge from `root` through its ends (a line from `root` to a point
/// in it crosses the plane inside the opening). A root on or past the plane
/// sees the opening square on: the wedge is the opening's own width
/// ([`opening`]), as from a root far in front.
pub fn opening_from(ap: &PortalAperture, root: Vec2) -> [HalfPlane; 3] {
    let square = opening(ap);
    if ap.frame.to_local(root).y <= EPS {
        return square;
    }
    let (o, t, h) = (ap.frame.origin, ap.frame.tangent(), ap.half_length);
    let (lo, hi) = (o - t * h, o + t * h);
    // The side of the line from `root` through `end` that holds `other`.
    let side = |end: Vec2, other: Vec2| {
        let n = (end - root).perp().normalize_or_zero();
        HalfPlane { point: end, inward: if (other - end).dot(n) >= 0.0 { n } else { -n } }
    };
    [square[0], side(lo, hi), side(hi, lo)]
}

fn entry_opening(ap: &PortalAperture, reach: Option<Vec2>) -> [HalfPlane; 3] {
    reach.map_or_else(|| opening(ap), |from| opening_from(ap, from))
}

/// Does `volume` reach past `ap`'s plane inside its opening?
pub fn reaches_through(volume: &CombatVolume, ap: &PortalAperture) -> bool {
    let link = PortalLink { enter: *ap, exit: *ap, key: 0, depth: None };
    piece_reaches_through(&VolumePiece::whole(volume.clone()), &link, None).unwrap_or(false)
}

fn piece_reaches_through(piece: &VolumePiece, link: &PortalLink, reach: Option<Vec2>) -> Result<bool, NumericFault> {
    let ap = &link.enter;
    let origin = ap.frame.origin.as_dvec2();
    let normal = ap.frame.normal.as_dvec2();
    let tangent = ap.frame.tangent().as_dvec2();
    let corners = box_corners(piece.source.bounds()).map(|point| {
        let offset = piece.to_piece.point_wide(point.as_dvec2()) - origin;
        DVec2::new(offset.dot(tangent), offset.dot(normal))
    });
    if corners.iter().all(|p| p.y >= 0.0) { return Ok(false); }
    let half = f64::from(ap.half_length);
    if reach.is_none() && (corners.iter().all(|p| p.x <= -half) || corners.iter().all(|p| p.x >= half)) {
        return Ok(false);
    }
    let cuts = link.through_from(reach).map(|plane| MaterialPlane::from(plane).pulled_back(&piece.to_piece)).collect::<Result<Vec<_>, _>>()?;
    Ok(holds(&piece.source, piece.source_bounds.iter().copied().chain(cuts)))
}

/// The partitioner's working memory, kept between calls on a thread so that a
/// test of a box piece allocates nothing: the outline being clipped (and, in
/// `WORK`, the work list). What is in it never reaches a result.
/// Keep intersections in double precision until the area test. Rounding each
/// cut to world-space floats can turn a shared edge into another fragment.
#[derive(Default)]
struct Scratch {
    outline: Vec<DVec2>,
    kept: Vec<DVec2>,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::default();
    static WORK: RefCell<VecDeque<(VolumePiece, Vec2)>> = RefCell::default();
}

/// Does any of `volume` lie in every one of `planes`?
fn holds<P: Into<MaterialPlane>>(volume: &CombatVolume, planes: impl Iterator<Item = P>) -> bool {
    SCRATCH.with_borrow_mut(|Scratch { outline, kept }| {
        outline.clear();
        match volume {
            CombatVolume::Aabb(b) => outline.extend(box_corners(*b).map(Vec2::as_dvec2)),
            CombatVolume::Obb { center, half, rotation } => {
                let (x, y) = (Vec2::from_angle(*rotation) * half.x, Vec2::from_angle(*rotation).perp() * half.y);
                outline.extend([*center - x - y, *center + x - y, *center + x + y, *center - x + y].map(Vec2::as_dvec2));
            }
            CombatVolume::Circle { .. } => outline.extend(box_corners(volume.bounds()).map(Vec2::as_dvec2)),
            CombatVolume::Convex { .. } => outline.extend(volume.polygon().into_iter().flatten().map(Vec2::as_dvec2)),
        }
        for plane in planes {
            clip_into(outline, plane.into(), kept);
            std::mem::swap(outline, kept);
            if outline.len() < 3 {
                return false;
            }
        }
        if !has_area(outline) {
            return false;
        }
        match volume {
            // The box's part only bounds the disc's: test the disc itself.
            CombatVolume::Circle { .. } => volume.intersects(&CombatVolume::convex(outline.iter().map(|p| p.as_vec2()).collect())),
            _ => true,
        }
    })
}

/// Partition material at the finite aperture before moving the through region.
fn cut_by(piece: &VolumePiece, reach: Option<Vec2>, link: &PortalLink, convention: MapConvention) -> Result<Vec<VolumePiece>, NumericFault> {
    let [behind, lo, hi] = entry_opening(&link.enter, reach);
    let mut parts = Vec::with_capacity(5);
    let mut stay = |planes: &[HalfPlane]| -> Result<(), NumericFault> {
        let part = piece.bounded(planes)?;
        if !part.is_empty() { parts.push(part); }
        Ok(())
    };
    stay(&[behind.flipped()])?;
    stay(&[behind, lo.flipped()])?;
    stay(&[behind, hi.flipped()])?;
    if let Some(deepest) = link.deepest(&link.enter) {
        stay(&[lo, hi, deepest.flipped()])?;
    }
    let planes: Vec<_> = [behind, lo, hi].into_iter().chain(link.deepest(&link.enter)).collect();
    let through = piece.bounded(&planes)?;
    if !through.is_empty() { parts.push(through.carried(link.map(convention)?, link.key)?); }
    Ok(parts)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusalReason {
    HopLimit,
    PieceLimit,
    WorkLimit,
    ScaleLimit,
    Numeric(NumericFault),
}

/// A rejected candidate has no partial geometry to publish.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub path: Vec<u32>,
    pub link: Option<u32>,
    pub reason: RefusalReason,
}

pub type Partition = Result<Vec<VolumePiece>, Refusal>;

/// Admit a complete material partition, or refuse the source operation.
pub fn partition(
    source: MaterialPrimitive<'_>, root: Vec2, links: &[PortalLink], first: Option<u32>,
    budget: PieceBudget, convention: MapConvention,
) -> Partition {
    partition_from(source, root, false, links, first, budget, convention)
}

/// Reaching uses the wedge from the owner through the aperture endpoints.
pub fn partition_reaching(
    source: MaterialPrimitive<'_>, root: Vec2, links: &[PortalLink], first: Option<u32>,
    budget: PieceBudget, convention: MapConvention,
) -> Partition {
    partition_from(source, root, true, links, first, budget, convention)
}

fn partition_from(
    source: MaterialPrimitive<'_>, root: Vec2, reaching: bool, links: &[PortalLink], first: Option<u32>,
    budget: PieceBudget, convention: MapConvention,
) -> Partition {
    let initial = |reason| Refusal { path: Vec::new(), link: None, reason };
    if budget.max_pieces == 0 { return Err(initial(RefusalReason::PieceLimit)); }
    if !root.is_finite() { return Err(initial(RefusalReason::Numeric(NumericFault::NonFinite))); }
    if !source.to_world.admits_scale_exponent(budget.max_scale_exponent) {
        return Err(initial(RefusalReason::ScaleLimit));
    }
    let whole = VolumePiece::from_material(source).map_err(|fault| initial(RefusalReason::Numeric(fault)))?;
    for link in links {
        link.map(convention).map_err(|fault| Refusal {
            path: Vec::new(), link: Some(link.key), reason: RefusalReason::Numeric(fault),
        })?;
    }
    let mut work = WORK.take();
    work.clear();
    work.push_back((whole, root));
    let result = (|| {
        let mut out = Vec::new();
        let mut steps = 0usize;
        let step_limit = 8usize.saturating_mul(usize::from(budget.max_pieces)).saturating_mul(links.len().max(1));
        while let Some((piece, root)) = work.pop_front() {
            steps += 1;
            let forced = (steps == 1).then_some(first).flatten();
            let reach = reaching.then_some(root);
            let rank = |link: &PortalLink| if Some(link.key) == forced { f32::NEG_INFINITY }
                else { link.enter.frame.to_local(root).y };
            let mut next: Option<&PortalLink> = None;
            for link in links {
                if Some(link.key) != forced && link.enter.frame.to_local(root).y <= 0.0 { continue; }
                let reaches = piece_reaches_through(&piece, link, reach).map_err(|fault| Refusal {
                    path: piece.path.clone(), link: Some(link.key), reason: RefusalReason::Numeric(fault),
                })?;
                if reaches && next.is_none_or(|old| rank(link).total_cmp(&rank(old)).then(link.key.cmp(&old.key)).is_lt()) {
                    next = Some(link);
                }
            }
            let Some(link) = next else { out.push(piece); continue; };
            let refusal = |reason| Refusal { path: piece.path.clone(), link: Some(link.key), reason };
            if steps > step_limit { return Err(refusal(RefusalReason::WorkLimit)); }
            if piece.path.len() >= usize::from(budget.max_hops) { return Err(refusal(RefusalReason::HopLimit)); }
            let map = link.map(convention).map_err(|fault| refusal(RefusalReason::Numeric(fault)))?;
            if !piece.to_piece.admits_scale_product(map, budget.max_scale_exponent) {
                return Err(refusal(RefusalReason::ScaleLimit));
            }
            let parts = cut_by(&piece, reach, link, convention).map_err(|fault| refusal(RefusalReason::Numeric(fault)))?;
            if out.len() + work.len() + parts.len() > usize::from(budget.max_pieces) {
                return Err(refusal(RefusalReason::PieceLimit));
            }
            let local = link.enter.frame.to_local(root);
            let along = map.direction(link.enter.frame.tangent()).dot(link.exit.frame.tangent()) * map.scale();
            let exit_root = link.exit.frame.from_local(Vec2::new((local.x * along).clamp(-link.exit.half_length, link.exit.half_length), 0.0));
            for part in parts {
                let part_root = if part.path.len() > piece.path.len() { exit_root } else { root };
                work.push_back((part, part_root));
            }
        }
        Ok(out)
    })();
    work.clear();
    WORK.set(work);
    result
}

/// The aperture snapshot used by one body's movement operation.
#[derive(Clone, Copy)]
pub struct MotionAdmission<'a> {
    pub links: &'a [PortalLink],
    pub first: Option<u32>,
    pub budget: PieceBudget,
    pub convention: MapConvention,
}

impl MotionAdmission<'_> {
    /// Validate both occupancy and the complete translated collision sweep.
    pub fn admit(&self, from: crate::Aabb, to: crate::Aabb) -> Result<(), Refusal> {
        let source = CombatVolume::aabb(to);
        partition(crate::frame::thread::MaterialPrimitive::world(&source), from.center(), self.links, self.first, self.budget, self.convention)?;
        let mut points = box_corners(from).into_iter().chain(box_corners(to)).collect::<Vec<_>>();
        if points.iter().any(|p| !p.is_finite()) {
            return Err(Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(NumericFault::NonFinite) });
        }
        points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        points.dedup();
        let chain = |vertices: &[Vec2]| {
            let mut result: Vec<Vec2> = Vec::new();
            for point in vertices {
                while result.len() >= 2 {
                    let a = result[result.len() - 2];
                    let b = result[result.len() - 1];
                    if (b - a).perp_dot(*point - b) > 0.0 { break; }
                    result.pop();
                }
                result.push(*point);
            }
            result
        };
        let mut lower = chain(&points);
        points.reverse();
        let mut upper = chain(&points);
        lower.pop(); upper.pop(); lower.extend(upper);
        if lower.len() >= 3 {
            partition(crate::frame::thread::MaterialPrimitive::world(&CombatVolume::convex(lower)), from.center(), self.links, self.first, self.budget, self.convention)?;
        }
        Ok(())
    }
}

/// The links that lead back to themselves: what goes in at one comes out
/// facing the entry of the next, and so on round to the first (a pair facing
/// itself, a ring of pairs). A piece in such a ring is cut until its budget
/// stops it ([`partition`]); this names the ring, for a room that wants to
/// say so. A link leads to another when part of the other's opening is where
/// what comes out of its exit is (in front of the exit, between its ends) and
/// the other faces back at the exit.
pub fn recursive_links(links: &[PortalLink]) -> Vec<u32> {
    let leads_to = |from: &PortalLink, to: &PortalLink| {
        let [a, b] = [-1.0, 1.0].map(|s| from.exit.frame.to_local(to.enter.frame.from_local(Vec2::new(s * to.enter.half_length, 0.0))));
        // The part of the opening `a..b` (in the exit's local frame) where
        // `front > 0` and `|along| < half`.
        let h = from.exit.half_length;
        let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
        for (p0, dp, min, max) in [(a.y, b.y - a.y, 0.0, f32::INFINITY), (a.x, b.x - a.x, -h, h)] {
            if dp.abs() < 1e-9 {
                if p0 <= min || p0 >= max {
                    return false;
                }
                continue;
            }
            let (t0, t1) = ((min - p0) / dp, (max - p0) / dp);
            lo = lo.max(t0.min(t1));
            hi = hi.min(t0.max(t1));
        }
        hi > lo && to.enter.frame.to_local(from.exit.frame.origin).y > 0.0
    };
    let mut on_a_ring: Vec<u32> = links
        .iter()
        .filter(|start| {
            // Is `start` reachable from itself?
            let mut seen = vec![false; links.len()];
            let mut stack: Vec<&PortalLink> = links.iter().filter(|next| leads_to(start, next)).collect();
            while let Some(link) = stack.pop() {
                if link.key == start.key {
                    return true;
                }
                let at = links.iter().position(|l| l.key == link.key).expect("a link of the list");
                if !std::mem::replace(&mut seen[at], true) {
                    stack.extend(links.iter().filter(|next| leads_to(link, next)));
                }
            }
            false
        })
        .map(|link| link.key)
        .collect();
    on_a_ring.sort_unstable();
    on_a_ring
}

fn box_outline(b: crate::Aabb) -> Vec<Vec2> {
    box_corners(b).to_vec()
}

fn box_corners(b: crate::Aabb) -> [Vec2; 4] {
    [
        Vec2::new(b.min.x, b.min.y),
        Vec2::new(b.max.x, b.min.y),
        Vec2::new(b.max.x, b.max.y),
        Vec2::new(b.min.x, b.max.y),
    ]
}

/// The convex polygon `outline` cut by every half plane, or `None` when no
/// area is left.
pub fn clip_convex(outline: Vec<Vec2>, planes: &[HalfPlane]) -> Option<Vec<Vec2>> {
    cut(outline, planes)
}

/// The half planes whose common part is `rect`.
pub fn face_rect_planes(rect: &super::FaceRect) -> [HalfPlane; 4] {
    let (t, n) = (rect.tangent(), rect.normal);
    [
        HalfPlane { point: rect.center - n * rect.half_out, inward: n },
        HalfPlane { point: rect.center + n * rect.half_out, inward: -n },
        HalfPlane { point: rect.center - t * rect.half_along, inward: t },
        HalfPlane { point: rect.center + t * rect.half_along, inward: -t },
    ]
}

fn cut<P: Copy + Into<MaterialPlane>>(outline: Vec<Vec2>, planes: &[P]) -> Option<Vec<Vec2>> {
    cut_wide(outline, planes).map(|outline| outline.into_iter().map(|p| p.as_vec2()).collect())
}

fn cut_wide<P: Copy + Into<MaterialPlane>>(outline: Vec<Vec2>, planes: &[P]) -> Option<Vec<DVec2>> {
    let mut outline: Vec<DVec2> = outline.into_iter().map(Vec2::as_dvec2).collect();
    let mut kept = Vec::with_capacity(outline.len() + planes.len());
    for plane in planes {
        clip_into(&outline, (*plane).into(), &mut kept);
        std::mem::swap(&mut outline, &mut kept);
        if outline.len() < 3 { return None; }
    }
    has_area(&outline).then_some(outline)
}

fn has_area(outline: &[DVec2]) -> bool {
    let Some(&origin) = outline.first() else { return false; };
    let extent = outline.iter().map(|p| (*p - origin).abs().max_element()).fold(0.0_f64, f64::max);
    let magnitude = outline.iter().map(|p| p.abs().max_element()).fold(extent, f64::max);
    twice_area(outline).abs() > f64::EPSILON * magnitude * extent * outline.len() as f64 * 32.0
}

/// `outline`'s part on `plane`'s inward side, into `kept`.
fn clip_into(outline: &[DVec2], plane: MaterialPlane, kept: &mut Vec<DVec2>) {
    kept.clear();
    for i in 0..outline.len() {
        let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
        let (sa, sb) = (plane.side(a), plane.side(b));
        if sa >= 0.0 {
            kept.push(a);
        }
        if (sa >= 0.0) != (sb >= 0.0) {
            kept.push(a + (b - a) * (sa / (sa - sb)));
        }
    }
}

fn twice_area(outline: &[DVec2]) -> f64 {
    let Some(origin) = outline.first() else { return 0.0 };
    (1..outline.len().saturating_sub(1))
        .map(|i| (outline[i] - origin).perp_dot(outline[i + 1] - origin))
        .sum()
}

/// Do two discs and the half planes share a point? Their common part, when
/// there is one, is convex and has a point on its boundary that is a corner of
/// the planes' region, where an edge of it meets a circle, where the circles
/// meet, or any point of a circle that is wholly inside the rest; one of those
/// is in all of them.
fn circles_meet(a: (DVec2, f64), b: (DVec2, f64), planes: &[MaterialPlane]) -> bool {
    let error = (a.0.abs().max_element() + b.0.abs().max_element() + a.1 + b.1) * f64::EPSILON * 32.0;
    if a.0.distance(b.0) > a.1 + b.1 + error { return false; }
    if planes.is_empty() { return true; }
    let lo = a.0 - DVec2::splat(a.1);
    let hi = a.0 + DVec2::splat(a.1);
    let mut region = vec![lo, DVec2::new(hi.x, lo.y), hi, DVec2::new(lo.x, hi.y)];
    let mut kept = Vec::new();
    for plane in planes {
        clip_into(&region, *plane, &mut kept);
        std::mem::swap(&mut region, &mut kept);
        if region.is_empty() { return false; }
    }
    let in_disc = |p: DVec2, (c, r): (DVec2, f64)| {
        let error = (p.abs().max_element() + c.abs().max_element() + r) * f64::EPSILON * 32.0;
        p.distance(c) <= r + error
    };
    let mut candidates: Vec<DVec2> = region.clone();
    for i in 0..region.len() {
        let (p, q) = (region[i], region[(i + 1) % region.len()]);
        for disc in [a, b] {
            candidates.extend(segment_meets_circle(p, q, disc));
        }
    }
    candidates.extend(circles_cross(a, b));
    candidates.extend([a.0 + DVec2::new(a.1, 0.0), b.0 + DVec2::new(b.1, 0.0), a.0, b.0]);
    candidates
        .into_iter()
        .any(|p| in_disc(p, a) && in_disc(p, b) && planes.iter().all(|plane| plane.side(p) >= -plane.error_at(p)))
}

fn segment_meets_circle(p: DVec2, q: DVec2, (c, r): (DVec2, f64)) -> Vec<DVec2> {
    let d = q - p;
    let f = p - c;
    let (aa, bb, cc) = (d.dot(d), 2.0 * f.dot(d), f.dot(f) - r * r);
    let disc = bb * bb - 4.0 * aa * cc;
    if aa <= 0.0 || disc < 0.0 {
        return Vec::new();
    }
    let root = disc.sqrt();
    [(-bb - root) / (2.0 * aa), (-bb + root) / (2.0 * aa)]
        .into_iter()
        .filter(|u| (0.0..=1.0).contains(u))
        .map(|u| p + d * u)
        .collect()
}

fn circles_cross((a, ra): (DVec2, f64), (b, rb): (DVec2, f64)) -> Vec<DVec2> {
    let d = a.distance(b);
    if d <= 0.0 || d > ra + rb || d < (ra - rb).abs() {
        return Vec::new();
    }
    let along = (ra * ra - rb * rb + d * d) / (2.0 * d);
    let off = (ra * ra - along * along).max(0.0).sqrt();
    let u = (b - a) / d;
    let mid = a + u * along;
    vec![mid + u.perp() * off, mid - u.perp() * off]
}

#[cfg(test)]
mod tests;
