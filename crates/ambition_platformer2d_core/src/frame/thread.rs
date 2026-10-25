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

use super::{Iso2, MapConvention, PortalAperture, PortalFrame};
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
    pub fn mapped(&self, map: &Iso2) -> Self {
        Self { point: map.point(self.point), inward: map.vec(self.inward) }
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
    pub fn map(&self, convention: MapConvention) -> Iso2 {
        Iso2::through(&self.enter.frame, &self.exit.frame, convention)
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
    /// Pairs a piece may be carried through, one after another.
    pub max_hops: u8,
    /// A cut that would make the partition larger than this is refused, so a
    /// partition is never larger than this (or than one piece, for none).
    pub max_pieces: u16,
}

impl PieceBudget {
    /// One pair: a piece carried through a pair is not cut again.
    pub const SINGLE_HOP: Self = Self { max_hops: 1, max_pieces: 16 };
    /// Pairs in sequence and side by side: a chain of four, a body across a
    /// handful.
    pub const COMPOSED: Self = Self { max_hops: 4, max_pieces: 32 };
}

/// A volume's part in one place: the whole shape placed in that place's
/// chart, and the half planes that bound the part of it that is there.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumePiece {
    pub volume: CombatVolume,
    /// No half planes: all of `volume` is here.
    pub bounds: Vec<HalfPlane>,
    /// The volume's own chart (its root's) into the piece's: the pairs it was
    /// carried through, composed.
    pub to_piece: Iso2,
    /// The links it was carried through, first first. Empty: it is in the
    /// volume's own chart.
    pub path: Vec<u32>,
}

impl VolumePiece {
    pub fn whole(volume: CombatVolume) -> Self {
        Self { volume, bounds: Vec::new(), to_piece: Iso2::IDENTITY, path: Vec::new() }
    }

    /// Was it carried through a pair?
    pub fn is_carried(&self) -> bool {
        !self.path.is_empty()
    }

    /// A point of the volume's own chart (its owner, the source of a strike),
    /// in this piece's chart.
    pub fn carry(&self, p: Vec2) -> Vec2 {
        self.to_piece.point(p)
    }

    /// A direction of the volume's own chart, in this piece's chart.
    pub fn carry_dir(&self, v: Vec2) -> Vec2 {
        self.to_piece.vec(v)
    }

    /// A point of this piece's chart, in the volume's own chart.
    pub fn uncarry(&self, p: Vec2) -> Vec2 {
        self.to_piece.inverse().point(p)
    }

    /// A direction of this piece's chart, in the volume's own chart.
    pub fn uncarry_dir(&self, v: Vec2) -> Vec2 {
        self.to_piece.inverse().vec(v)
    }

    /// Does the piece hold any of its volume?
    pub fn is_empty(&self) -> bool {
        !self.bounds.is_empty() && !holds(&self.volume, self.bounds.iter().copied())
    }

    /// The part that is here, as a convex polygon in the piece's chart: exact
    /// for a polygon volume; for a circle, the part of its bounding box (the
    /// circle's own test stays exact in [`Self::meets`]).
    pub fn region(&self) -> Option<Vec<Vec2>> {
        let outline = self.volume.polygon().unwrap_or_else(|| box_outline(self.volume.bounds()));
        cut(outline, &self.bounds)
    }

    /// The middle of the part that is here: where it touches what it meets.
    pub fn center(&self) -> Vec2 {
        self.region().map_or(self.volume.center(), |region| {
            region.iter().copied().sum::<Vec2>() / region.len() as f32
        })
    }

    /// Do the two pieces share a point? Two whole pieces are tested exactly
    /// as their volumes are ([`CombatVolume::intersects`]).
    pub fn meets(&self, other: &VolumePiece) -> bool {
        if self.bounds.is_empty() && other.bounds.is_empty() {
            return self.volume.intersects(&other.volume);
        }
        let (a, b) = (self.volume.bounds(), other.volume.bounds());
        if a.min.x > b.max.x || b.min.x > a.max.x || a.min.y > b.max.y || b.min.y > a.max.y {
            return false;
        }
        let planes: Vec<HalfPlane> = self.bounds.iter().chain(&other.bounds).copied().collect();
        // Cut the one that is a polygon by every plane, and test the other
        // against what is left.
        if let Some(outline) = other.volume.polygon() {
            return cut(outline, &planes).is_some_and(|p| self.volume.intersects(&CombatVolume::convex(p)));
        }
        if let Some(outline) = self.volume.polygon() {
            return cut(outline, &planes).is_some_and(|p| other.volume.intersects(&CombatVolume::convex(p)));
        }
        match (&self.volume, &other.volume) {
            (CombatVolume::Circle { center: a, radius: ra }, CombatVolume::Circle { center: b, radius: rb }) => {
                circles_meet((*a, *ra), (*b, *rb), &planes)
            }
            _ => unreachable!("only a circle has no polygon"),
        }
    }

    /// The half planes of [`Self::bounds`] that cut the volume: without one
    /// of them the part here would be larger. What a drawing clips by.
    pub fn active_bounds(&self) -> Vec<HalfPlane> {
        let outline = self.volume.polygon().unwrap_or_else(|| box_outline(self.volume.bounds()));
        let mut kept = self.bounds.clone();
        let mut i = 0;
        while i < kept.len() {
            let without: Vec<HalfPlane> = kept.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, p)| *p).collect();
            let cuts = cut(outline.clone(), &without).is_some_and(|region| region.iter().any(|p| kept[i].side(*p) < -EPS));
            if cuts {
                i += 1;
            } else {
                kept.remove(i);
            }
        }
        kept
    }

    fn bounded(&self, more: impl IntoIterator<Item = HalfPlane>) -> Self {
        let mut piece = self.clone();
        piece.bounds.extend(more);
        piece
    }
}

/// `volume` placed where `map` takes it.
pub fn map_volume(volume: &CombatVolume, map: &Iso2) -> CombatVolume {
    match volume {
        CombatVolume::Aabb(a) => boxed(map.point(a.center()), a.half_size(), map.vec(Vec2::X)),
        CombatVolume::Obb { center, half, rotation } => {
            boxed(map.point(*center), *half, map.vec(Vec2::from_angle(*rotation)))
        }
        CombatVolume::Circle { center, radius } => CombatVolume::circle(map.point(*center), *radius),
        CombatVolume::Convex { points, .. } => CombatVolume::convex(points.iter().map(|p| map.point(*p)).collect()),
    }
}

/// `volume` mapped through the pair: placed at `exit` as its part past
/// `enter`'s plane emerges there.
pub fn carry_volume(volume: &CombatVolume, enter: &PortalFrame, exit: &PortalFrame, convention: MapConvention) -> CombatVolume {
    map_volume(volume, &Iso2::through(enter, exit, convention))
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
    piece_reaches_through(&VolumePiece::whole(volume.clone()), &link, None)
}

fn piece_reaches_through(piece: &VolumePiece, link: &PortalLink, reach: Option<Vec2>) -> bool {
    let ap = &link.enter;
    // The box around the volume first: most openings are nowhere near.
    let b = piece.volume.bounds();
    let corners = [b.min, Vec2::new(b.max.x, b.min.y), b.max, Vec2::new(b.min.x, b.max.y)].map(|c| ap.frame.to_local(c));
    let h = ap.half_length;
    if corners.iter().all(|l| l.y >= 0.0) {
        return false;
    }
    // Square on, a box wholly past an end is beside the opening.
    if reach.is_none() && (corners.iter().all(|l| l.x <= -h) || corners.iter().all(|l| l.x >= h)) {
        return false;
    }
    holds(&piece.volume, piece.bounds.iter().copied().chain(link.through_from(reach)))
}

/// The partitioner's working memory, kept between calls on a thread so that a
/// test of a box piece allocates nothing: the outline being clipped (and, in
/// `WORK`, the work list). What is in it never reaches a result.
#[derive(Default)]
struct Scratch {
    outline: Vec<Vec2>,
    kept: Vec<Vec2>,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::default();
    static WORK: RefCell<VecDeque<(VolumePiece, Vec2)>> = RefCell::default();
}

/// Does any of `volume` lie in every one of `planes`?
fn holds(volume: &CombatVolume, planes: impl Iterator<Item = HalfPlane>) -> bool {
    SCRATCH.with_borrow_mut(|Scratch { outline, kept }| {
        outline.clear();
        match volume {
            CombatVolume::Aabb(b) => outline.extend(box_corners(*b)),
            CombatVolume::Obb { center, half, rotation } => {
                let (x, y) = (Vec2::from_angle(*rotation) * half.x, Vec2::from_angle(*rotation).perp() * half.y);
                outline.extend([*center - x - y, *center + x - y, *center + x + y, *center - x + y]);
            }
            CombatVolume::Circle { .. } => outline.extend(box_corners(volume.bounds())),
            CombatVolume::Convex { .. } => outline.extend(volume.polygon().into_iter().flatten()),
        }
        for plane in planes {
            clip_into(outline, plane, kept);
            std::mem::swap(outline, kept);
            if outline.len() < 3 {
                return false;
            }
        }
        if twice_area(outline).abs() <= EPS {
            return false;
        }
        match volume {
            // The box's part only bounds the disc's: test the disc itself.
            CombatVolume::Circle { .. } => volume.intersects(&CombatVolume::convex(outline.clone())),
            _ => true,
        }
    })
}

/// `piece` cut by `link`: its part in front of the entry plane, its parts past
/// the plane and beside the opening (seen from `reach`, for a piece that
/// reaches from it), its part deeper than the link goes, and (unless
/// `refused`) its part through the opening, at the exit. Empty parts are left
/// out.
fn cut_by(piece: &VolumePiece, reach: Option<Vec2>, link: &PortalLink, convention: MapConvention, refused: bool) -> Vec<VolumePiece> {
    let [behind, lo, hi] = entry_opening(&link.enter, reach);
    let mut parts = Vec::with_capacity(5);
    let mut stay = |more: &[HalfPlane]| {
        if holds(&piece.volume, piece.bounds.iter().chain(more).copied()) {
            parts.push(piece.bounded(more.iter().copied()));
        }
    };
    stay(&[behind.flipped()]);
    stay(&[behind, lo.flipped()]);
    stay(&[behind, hi.flipped()]);
    if let Some(deepest) = link.deepest(&link.enter) {
        stay(&[lo, hi, deepest.flipped()]);
    }
    if !refused {
        let map = link.map(convention);
        let [exit_behind, _, _] = opening(&link.exit);
        let mut path = piece.path.clone();
        path.push(link.key);
        // Out of the exit, as deep as it went in.
        let shallow = link.depth.map(|d| HalfPlane {
            point: link.exit.frame.origin + link.exit.frame.normal * d,
            inward: -link.exit.frame.normal,
        });
        let through = VolumePiece {
            volume: map_volume(&piece.volume, &map),
            bounds: piece
                .bounds
                .iter()
                .map(|plane| plane.mapped(&map))
                .chain([exit_behind.flipped(), lo.mapped(&map), hi.mapped(&map)])
                .chain(shallow)
                .collect(),
            to_piece: piece.to_piece.then(&map),
            path,
        };
        if !through.is_empty() {
            parts.push(through);
        }
    }
    parts
}

/// A cut the budget refused: the piece whose path is `path` would have been
/// carried through `link`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub path: Vec<u32>,
    pub link: u32,
    /// How the piece was kept. False: the opening was solid to it, and its
    /// part through the opening is not anywhere. True: the budget could not
    /// pay even for the parts in front and beside (or the step limit was
    /// reached), so the piece stayed whole where it is, its part through the
    /// opening with it.
    pub whole: bool,
}

/// Where a volume is: its pieces, and the cuts its budget refused.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Partition {
    pub pieces: Vec<VolumePiece>,
    pub refused: Vec<Refusal>,
}

/// Where `volume`, which comes from `root` (its owner), is among `links`.
///
/// A piece is cut by the link it reaches through from that link's front;
/// among several, the one whose plane is nearest the piece's root, then the
/// lowest key — so the result does not depend on the order of `links`. A
/// piece carried through a link has its root on that link's exit. The part
/// through is cut again, and the parts that stay are too, until no piece
/// reaches through a link from its front. `first` is a link the root piece is
/// cut by first whatever side its root is on: the pair a body straddles.
///
/// The budget bounds the work. A cut it refuses leaves the opening solid to
/// that piece: the part that would have gone through is not anywhere, and the
/// refusal says so; the parts in front and beside stay. When even those parts
/// would make the partition larger than the budget, the piece stays whole
/// where it is, as at the step limit.
pub fn partition(
    volume: &CombatVolume,
    root: Vec2,
    links: &[PortalLink],
    first: Option<u32>,
    budget: PieceBudget,
    convention: MapConvention,
) -> Partition {
    partition_from(volume, root, false, links, first, budget, convention)
}

/// [`partition`] for a volume that reaches from `root` (a swing from its
/// owner): what it reaches past a plane goes through in the wedge from its
/// root through the opening's ends ([`opening_from`]), and what is beside it
/// stays. A piece carried through reaches from where its root's line came
/// out of the exit.
pub fn partition_reaching(
    volume: &CombatVolume,
    root: Vec2,
    links: &[PortalLink],
    first: Option<u32>,
    budget: PieceBudget,
    convention: MapConvention,
) -> Partition {
    partition_from(volume, root, true, links, first, budget, convention)
}

fn partition_from(
    volume: &CombatVolume,
    root: Vec2,
    reaching: bool,
    links: &[PortalLink],
    first: Option<u32>,
    budget: PieceBudget,
    convention: MapConvention,
) -> Partition {
    let mut out = Partition::default();
    let mut work = WORK.take();
    work.clear();
    work.push_back((VolumePiece::whole(volume.clone()), root));
    // A partition that stops growing ends: each cut bounds its parts by the
    // link's planes, and no part of it reaches through that link again. This
    // stops one that the budget lets grow without end, should one exist.
    let mut steps = 0usize;
    let step_limit = 8 * usize::from(budget.max_pieces).max(1) * links.len().max(1);
    while let Some((piece, root)) = work.pop_front() {
        steps += 1;
        let forced = (steps == 1).then_some(first).flatten();
        let reach = reaching.then_some(root);
        let reaching = links
            .iter()
            .filter(|link| {
                (Some(link.key) == forced || link.enter.frame.to_local(root).y > 0.0)
                    && piece_reaches_through(&piece, link, reach)
            })
            .min_by(|a, b| {
                let rank = |link: &PortalLink| {
                    if Some(link.key) == forced {
                        f32::NEG_INFINITY
                    } else {
                        link.enter.frame.to_local(root).y
                    }
                };
                rank(a).total_cmp(&rank(b)).then(a.key.cmp(&b.key))
            });
        let Some(link) = reaching else {
            out.pieces.push(piece);
            continue;
        };
        if steps > step_limit {
            out.refused.push(Refusal { path: piece.path.clone(), link: link.key, whole: true });
            out.pieces.push(piece);
            continue;
        }
        // The budget is checked on the parts a cut makes, not on a guess of
        // their number: a link with a depth makes five, and a refused cut
        // makes up to four. The parts take the place of the piece.
        let kept = out.pieces.len() + work.len();
        let fits = |parts: &[VolumePiece]| kept + parts.len() <= usize::from(budget.max_pieces);
        let through = (piece.path.len() < usize::from(budget.max_hops))
            .then(|| cut_by(&piece, reach, link, convention, false))
            .filter(|parts| fits(parts));
        let parts = match through {
            Some(parts) => parts,
            None => {
                let parts = cut_by(&piece, reach, link, convention, true);
                let whole = !fits(&parts);
                out.refused.push(Refusal { path: piece.path.clone(), link: link.key, whole });
                if whole {
                    out.pieces.push(piece);
                    continue;
                }
                parts
            }
        };
        // The exit point the root's line meets: where a carried piece came
        // from, for the next link's front.
        let exit_root = |link: &PortalLink| {
            let local = link.enter.frame.to_local(root);
            let along = link.map(convention).vec(link.enter.frame.tangent()).dot(link.exit.frame.tangent());
            let h = link.exit.half_length;
            link.exit.frame.from_local(Vec2::new((local.x * along).clamp(-h, h), 0.0))
        };
        for part in parts {
            let part_root = if part.path.len() > piece.path.len() { exit_root(link) } else { root };
            work.push_back((part, part_root));
        }
    }
    WORK.set(work);
    out
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

fn cut(mut outline: Vec<Vec2>, planes: &[HalfPlane]) -> Option<Vec<Vec2>> {
    let mut kept = Vec::with_capacity(outline.len() + planes.len());
    for plane in planes {
        clip_into(&outline, *plane, &mut kept);
        std::mem::swap(&mut outline, &mut kept);
        if outline.len() < 3 {
            return None;
        }
    }
    (twice_area(&outline).abs() > EPS).then_some(outline)
}

/// `outline`'s part on `plane`'s inward side, into `kept`.
fn clip_into(outline: &[Vec2], plane: HalfPlane, kept: &mut Vec<Vec2>) {
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

fn twice_area(outline: &[Vec2]) -> f32 {
    (0..outline.len()).map(|i| outline[i].perp_dot(outline[(i + 1) % outline.len()])).sum()
}

/// Do two discs and the half planes share a point? Their common part, when
/// there is one, is convex and has a point on its boundary that is a corner of
/// the planes' region, where an edge of it meets a circle, where the circles
/// meet, or any point of a circle that is wholly inside the rest; one of those
/// is in all of them.
fn circles_meet(a: (Vec2, f32), b: (Vec2, f32), planes: &[HalfPlane]) -> bool {
    let bounds = crate::Aabb::new(a.0, Vec2::splat(a.1));
    let Some(region) = cut(box_outline(bounds), planes) else {
        return false;
    };
    let in_disc = |p: Vec2, (c, r): (Vec2, f32)| p.distance(c) <= r + EPS;
    let mut candidates: Vec<Vec2> = region.clone();
    for i in 0..region.len() {
        let (p, q) = (region[i], region[(i + 1) % region.len()]);
        for disc in [a, b] {
            candidates.extend(segment_meets_circle(p, q, disc));
        }
    }
    candidates.extend(circles_cross(a, b));
    candidates.extend([a.0 + Vec2::new(a.1, 0.0), b.0 + Vec2::new(b.1, 0.0), a.0, b.0]);
    candidates
        .into_iter()
        .any(|p| in_disc(p, a) && in_disc(p, b) && planes.iter().all(|plane| plane.contains(p)))
}

fn segment_meets_circle(p: Vec2, q: Vec2, (c, r): (Vec2, f32)) -> Vec<Vec2> {
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

fn circles_cross((a, ra): (Vec2, f32), (b, rb): (Vec2, f32)) -> Vec<Vec2> {
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
