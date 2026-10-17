//! Where a volume is when a portal pair cuts it
//! (`docs/planning/engine/portals-a-body-in-two-places.md`).
//!
//! A volume that crosses the plane of a portal inside its opening, coming from
//! the portal's front, is in two places: the part in front stays, and the part
//! past the plane is at the linked portal, mapped through the pair. A part past
//! the plane and beside the opening stays where it is.
//!
//! A piece is the whole shape placed in its chart and the half planes that
//! bound the part of it that is there, so the test is exact for every
//! [`CombatVolume`]: a circle cut by a plane stays a cut circle.

use super::{map_point, map_vec, MapConvention, PortalAperture, PortalFrame};
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
}

/// A volume's part in one place: the whole shape placed in that place's
/// chart, and the half planes that bound the part of it that is there.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumePiece {
    pub volume: CombatVolume,
    /// No half planes: all of `volume` is here.
    pub bounds: Vec<HalfPlane>,
    /// The pair the piece was carried through, entry first. `None`: it is in
    /// the volume's own chart.
    pub carried: Option<(PortalFrame, PortalFrame)>,
}

impl VolumePiece {
    pub fn whole(volume: CombatVolume) -> Self {
        Self { volume, bounds: Vec::new(), carried: None }
    }

    /// A point of the volume's own chart (its owner, the source of a strike),
    /// in this piece's chart.
    pub fn carry(&self, p: Vec2, convention: MapConvention) -> Vec2 {
        self.carried.map_or(p, |(enter, exit)| map_point(&enter, &exit, convention, p))
    }

    /// A direction of the volume's own chart, in this piece's chart.
    pub fn carry_dir(&self, v: Vec2, convention: MapConvention) -> Vec2 {
        self.carried.map_or(v, |(enter, exit)| map_vec(&enter, &exit, convention, v))
    }

    /// A point of this piece's chart, in the volume's own chart.
    pub fn uncarry(&self, p: Vec2, convention: MapConvention) -> Vec2 {
        self.carried.map_or(p, |(enter, exit)| map_point(&exit, &enter, convention, p))
    }

    /// A direction of this piece's chart, in the volume's own chart.
    pub fn uncarry_dir(&self, v: Vec2, convention: MapConvention) -> Vec2 {
        self.carried.map_or(v, |(enter, exit)| map_vec(&exit, &enter, convention, v))
    }

    /// Does the piece hold any of its volume?
    pub fn is_empty(&self) -> bool {
        if self.bounds.is_empty() {
            return false;
        }
        match self.volume.polygon() {
            Some(outline) => cut(outline, &self.bounds).is_none(),
            None => cut(box_outline(self.volume.bounds()), &self.bounds)
                .is_none_or(|region| !self.volume.intersects(&CombatVolume::convex(region))),
        }
    }

    /// The middle of the part that is here: where it touches what it meets.
    pub fn center(&self) -> Vec2 {
        let outline = self.volume.polygon().unwrap_or_else(|| box_outline(self.volume.bounds()));
        cut(outline, &self.bounds).map_or(self.volume.center(), |region| {
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
}

/// `volume` mapped through the pair: placed at `exit` as its part past
/// `enter`'s plane emerges there.
pub fn carry_volume(volume: &CombatVolume, enter: &PortalFrame, exit: &PortalFrame, convention: MapConvention) -> CombatVolume {
    let point = |p: Vec2| map_point(enter, exit, convention, p);
    let dir = |v: Vec2| map_vec(enter, exit, convention, v);
    match volume {
        CombatVolume::Aabb(a) => boxed(point(a.center()), a.half_size(), dir(Vec2::X)),
        CombatVolume::Obb { center, half, rotation } => boxed(point(*center), *half, dir(Vec2::from_angle(*rotation))),
        CombatVolume::Circle { center, radius } => CombatVolume::circle(point(*center), *radius),
        CombatVolume::Convex { points, .. } => CombatVolume::convex(points.iter().map(|p| point(*p)).collect()),
    }
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
fn opening(ap: &PortalAperture) -> [HalfPlane; 3] {
    let (o, n, t, h) = (ap.frame.origin, ap.frame.normal, ap.frame.tangent(), ap.half_length);
    [
        HalfPlane { point: o, inward: -n },
        HalfPlane { point: o - t * h, inward: t },
        HalfPlane { point: o + t * h, inward: -t },
    ]
}

/// Does `volume` reach past `ap`'s plane inside its opening?
pub fn reaches_through(volume: &CombatVolume, ap: &PortalAperture) -> bool {
    !VolumePiece { volume: volume.clone(), bounds: opening(ap).to_vec(), carried: None }.is_empty()
}

/// `volume` cut by the pair at `enter`: its part in front of the plane, its
/// parts past the plane and beside the opening, and its part through the
/// opening, at `exit`. Empty parts are left out.
pub fn cut_through(
    volume: &CombatVolume,
    enter: &PortalAperture,
    exit: &PortalAperture,
    convention: MapConvention,
) -> Vec<VolumePiece> {
    let (o, n, t, h) = (enter.frame.origin, enter.frame.normal, enter.frame.tangent(), enter.half_length);
    let behind = HalfPlane { point: o, inward: -n };
    // All in front: whole, and tested exactly as the volume is.
    if (VolumePiece { volume: volume.clone(), bounds: vec![behind], carried: None }).is_empty() {
        return vec![VolumePiece::whole(volume.clone())];
    }
    let here = |bounds: Vec<HalfPlane>| VolumePiece { volume: volume.clone(), bounds, carried: None };
    let [exit_front, exit_lo, exit_hi] = opening(exit);
    let through = VolumePiece {
        volume: carry_volume(volume, &enter.frame, &exit.frame, convention),
        bounds: vec![HalfPlane { point: exit_front.point, inward: -exit_front.inward }, exit_lo, exit_hi],
        carried: Some((enter.frame, exit.frame)),
    };
    [
        here(vec![HalfPlane { point: o, inward: n }]),
        here(vec![behind, HalfPlane { point: o - t * h, inward: -t }]),
        here(vec![behind, HalfPlane { point: o + t * h, inward: t }]),
        through,
    ]
    .into_iter()
    .filter(|piece| !piece.is_empty())
    .collect()
}

/// Where `volume`, which comes from `root` (its owner), is: cut by the first
/// opening of `pairs` it reaches through from that opening's front, or whole.
pub fn thread_volume(
    volume: &CombatVolume,
    root: Vec2,
    pairs: &[(PortalAperture, PortalAperture)],
    convention: MapConvention,
) -> Vec<VolumePiece> {
    for (a, b) in pairs {
        for (enter, exit) in [(a, b), (b, a)] {
            if enter.frame.to_local(root).y > 0.0 && reaches_through(volume, enter) {
                return cut_through(volume, enter, exit, convention);
            }
        }
    }
    vec![VolumePiece::whole(volume.clone())]
}

fn box_outline(b: crate::Aabb) -> Vec<Vec2> {
    vec![
        Vec2::new(b.min.x, b.min.y),
        Vec2::new(b.max.x, b.min.y),
        Vec2::new(b.max.x, b.max.y),
        Vec2::new(b.min.x, b.max.y),
    ]
}

/// The convex polygon `outline` cut by every half plane, or `None` when no
/// area is left.
fn cut(mut outline: Vec<Vec2>, planes: &[HalfPlane]) -> Option<Vec<Vec2>> {
    for plane in planes {
        let mut kept = Vec::with_capacity(outline.len() + 2);
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
        outline = kept;
        if outline.len() < 3 {
            return None;
        }
    }
    let twice_area: f32 = (0..outline.len())
        .map(|i| outline[i].perp_dot(outline[(i + 1) % outline.len()]))
        .sum();
    (twice_area.abs() > EPS).then_some(outline)
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
