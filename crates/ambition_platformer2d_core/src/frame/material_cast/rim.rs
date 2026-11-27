//! The rim of an opening: the contact a body meets at the ends of the hole
//! it goes through (portal P3, `portal-parts-similarity-and-chirality.md`).
//!
//! A rigid body that crosses a portal's plane goes through only whole: the
//! segment along which it cuts the plane must lie in the opening, because
//! past an end the plane is the host. For a convex body that rule is one
//! fact: an end of the opening never lies inside the body. A body that cuts
//! the plane over the opening and past an end holds that end inside it, and
//! a body that cuts the plane within the opening, or wholly beside it, does
//! not. So each end of each opening the body may go through is a point
//! obstacle, moving with its aperture, and a body meets it as it meets any
//! other solid: sliding along the face it stops at the end, coming in from
//! the front past an end it lands on the end, and a body wider than the
//! opening rests on both. A turned body is tested as it is.
//!
//! A thick host gives the same answer from its own solid beside the hole.
//! A line host, a plank, and the far side of the seam (the exit's plane,
//! which the part not yet through has behind it) give none, and the rim is
//! what holds the body there.
//!
//! One case keeps an older rule. A body whose box already cuts the plane
//! past an end when the motion starts (a box whose feet stand into a slope
//! beside the opening: the kernel's feet rule) is not pushed out: the part
//! past the end may go no deeper into the side it was not on. That is a
//! floor beside the opening at the depth it is at now, moving with the
//! aperture.
//!
//! The rims are the cast's own contacts: a movement system meets them in
//! the same query as the room's solids ([`crate::world::MaterialCollider::Rim`]),
//! and its response to them is its response to a solid.

use bevy_math::DVec2;

use super::super::thread::{MotionAdmission, Refusal, RefusalReason};
use super::{MaterialCast, MaterialCastCells, MaterialCastHit, MaterialCell};
use crate::Vec2;

/// How far (px) a part past an end may go deeper in a step before the floor
/// holds it: float noise, not motion.
const NO_WORSE: f64 = 1.0e-3;

/// What a rim is to the body that meets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RimKind {
    /// An end of the opening: a point the body may not take inside it.
    End,
    /// The depth a part past an end had when the motion started: a segment
    /// beside the opening it may not go past.
    Floor,
}

/// One rim of one opening for one cast, where it is at the end of the
/// world tick and how it travels over the tick (a surface chain's
/// convention: per-tick displacement, turn about `pivot`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RimObstacle {
    /// The link whose entry aperture this is a rim of.
    pub link: u32,
    /// Which end, along the aperture's tangent: `1` or `-1`.
    pub end: i8,
    pub kind: RimKind,
    points: [Vec2; 2],
    pub displacement: Vec2,
    pub turn: f32,
    pub pivot: Vec2,
}

impl RimObstacle {
    /// The obstacle at the end of the tick: one point, or a floor segment.
    pub fn points(&self) -> &[Vec2] {
        match self.kind {
            RimKind::End => &self.points[..1],
            RimKind::Floor => &self.points,
        }
    }

    /// The order two rims met at one instant are taken in.
    pub(crate) fn cmp_identity(&self, other: &Self) -> std::cmp::Ordering {
        (self.link, self.end, self.kind).cmp(&(other.link, other.end, other.kind))
    }
}

impl MaterialCast<'_> {
    /// The rims this cast's material may meet under `admission`: for each
    /// admitted link whose entry the body is in front of or straddles at the
    /// start, both ends of its opening (see the module docs). A link closed
    /// to this body (it cuts that plane from behind) gives none, nor does an
    /// opening the body is behind or out of reach of.
    pub(super) fn rims(&self, admission: MotionAdmission<'_>) -> Result<Vec<RimObstacle>, Refusal> {
        let closed = admission.span.reroot.map_or(&[][..], |reroot| reroot.closed);
        let numeric = |link: u32| move |reason| Refusal { path: vec![], link: Some(link), reason: RefusalReason::Numeric(reason) };
        let outline = self.outline().map_err(super::numeric)?;
        let center = outline.iter().copied().sum::<DVec2>() / outline.len() as f64;
        let size = outline.iter().map(|p| p.distance(center)).fold(0.0, f64::max);
        let mut rims = Vec::new();
        for link in admission.links {
            if closed.contains(&link.key) {
                continue;
            }
            let start = link.enter_step.sample(link.enter, self.time.start).map_err(numeric(link.key))?;
            let frame = start.frame;
            let (origin, normal) = (frame.origin.as_dvec2(), frame.normal.as_dvec2().normalize());
            let tangent = DVec2::new(-normal.y, normal.x);
            let h = f64::from(start.half_length);
            let reach = size + self.delta.as_dvec2().length() + link.enter_step.displacement.as_dvec2().length()
                + f64::from(link.enter_step.turn.abs()) * (h + size) + 1.0;
            let local: Vec<DVec2> = outline.iter().map(|p| { let d = *p - origin; DVec2::new(d.dot(tangent), d.dot(normal)) }).collect();
            let c = DVec2::new((center - origin).dot(tangent), (center - origin).dot(normal));
            // Out of reach, or behind the opening: the host's business.
            if c.y.abs() > reach || c.x.abs() > h + reach || c.y < 0.0 {
                continue;
            }
            let cut = cut(&local);
            for end in [1i8, -1] {
                let side = f64::from(end);
                let past = cut.map(|(t0, t1)| if end > 0 { t1 - h } else { -h - t0 });
                // At the end of the tick, in the aperture's own axes at
                // `start`, carried to where the aperture ends.
                let place = |x: f64, y: f64| -> Result<Vec2, Refusal> {
                    let end_frame = link.enter.frame;
                    let n = end_frame.normal.as_dvec2().normalize();
                    let t = DVec2::new(-n.y, n.x);
                    crate::frame::Sim2::IDENTITY.point_from_wide(end_frame.origin.as_dvec2() + t * x + n * y)
                        .map_err(numeric(link.key))
                };
                let (kind, points) = match past {
                    // In front of the plane, or cutting it within the
                    // opening: the end itself.
                    None => (RimKind::End, [place(side * h, 0.0)?; 2]),
                    Some(past) if past <= 0.0 => (RimKind::End, [place(side * h, 0.0)?; 2]),
                    // Past this end already: no deeper.
                    Some(_) => {
                        let beyond = clip(&local, |p| side * p.x - h);
                        let depth = beyond.iter().map(|p| -p.y).fold(0.0, f64::max);
                        let height = beyond.iter().map(|p| p.y).fold(0.0, f64::max);
                        let y = if height >= depth { -(depth + NO_WORSE) } else { height + NO_WORSE };
                        (RimKind::Floor, [place(side * h, y)?, place(side * (h + reach + size), y)?])
                    }
                };
                rims.push(RimObstacle {
                    link: link.key, end, kind, points,
                    displacement: link.enter_step.displacement, turn: link.enter_step.turn, pivot: link.enter.frame.pivot,
                });
            }
        }
        Ok(rims)
    }

    /// The body's outline in the world at the start of the cast: its
    /// polygon, or its bounding box for a round body.
    fn outline(&self) -> Result<Vec<DVec2>, crate::frame::NumericFault> {
        let map = self.material.to_world;
        let points = match self.material.domain.polygon() {
            Some(points) => points,
            None => {
                let b = self.material.domain.bounds();
                vec![b.min, Vec2::new(b.max.x, b.min.y), b.max, Vec2::new(b.min.x, b.max.y)]
            }
        };
        points.into_iter().map(|p| map.point(p).map(|p| p.as_dvec2())).collect()
    }
}

impl MaterialCastCells<'_> {
    /// The body's first contact with one of its rims, met by the whole body
    /// in its root chart over the cast. `part` is that whole body.
    pub(crate) fn first_rim_contact(&self) -> Result<Option<(RimObstacle, &MaterialCell, MaterialCastHit)>, Refusal> {
        let Some(rims) = self.rims.as_ref() else { return Ok(None) };
        let part = &rims.whole;
        let mut best: Option<(RimObstacle, MaterialCastHit)> = None;
        for rim in &rims.obstacles {
            let hit = rims.cells.against_rigid(part, rim.points(), rim.displacement, rim.turn, rim.pivot)?;
            if let Some(hit) = hit {
                let better = best.as_ref().is_none_or(|(old_rim, old)| {
                    hit.toi.total_cmp(&old.toi).then(rim.cmp_identity(old_rim)).is_lt()
                });
                if better {
                    best = Some((*rim, hit));
                }
            }
        }
        Ok(best.map(|(rim, hit)| (rim, part, hit)))
    }
}

/// The rims of one admitted cast and the whole body that meets them.
pub(super) struct CastRims<'a> {
    pub(super) obstacles: Vec<RimObstacle>,
    /// The cast with no portal cut: the body as one rigid outline.
    pub(super) cells: Box<MaterialCastCells<'a>>,
    pub(super) whole: MaterialCell,
}

/// Where a convex polygon in an opening's axes cuts its plane (`y = 0`):
/// the span along the face, `None` when it lies on one side.
fn cut(poly: &[DVec2]) -> Option<(f64, f64)> {
    let mut span: Option<(f64, f64)> = None;
    for (i, &a) in poly.iter().enumerate() {
        let b = poly[(i + 1) % poly.len()];
        if (a.y < 0.0) != (b.y < 0.0) {
            let t = a.x + (b.x - a.x) * (a.y / (a.y - b.y));
            span = Some(span.map_or((t, t), |(lo, hi)| (lo.min(t), hi.max(t))));
        }
    }
    span
}

/// The part of the convex polygon `poly` where `keep(p) >= 0`, for a `keep`
/// that is linear in `p`.
fn clip(poly: &[DVec2], keep: impl Fn(DVec2) -> f64) -> Vec<DVec2> {
    let mut out = Vec::with_capacity(poly.len() + 2);
    for (i, &a) in poly.iter().enumerate() {
        let b = poly[(i + 1) % poly.len()];
        let (ka, kb) = (keep(a), keep(b));
        if ka >= 0.0 {
            out.push(a);
        }
        if (ka >= 0.0) != (kb >= 0.0) {
            out.push(a + (b - a) * (ka / (ka - kb)));
        }
    }
    out
}

#[cfg(test)]
mod tests;
