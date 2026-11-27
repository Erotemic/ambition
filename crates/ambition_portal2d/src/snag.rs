//! The snag's geometry: does a body go through an opening whole?
//!
//! A rigid body that straddles a portal cuts the portal's plane along one
//! segment, and that segment must be in the opening: past an end, the plane
//! is the host. A body cut by the plane past an end is like a rope half
//! through a hole and half around it, and it snags on the end (Jon
//! 2026-10-16).
//!
//! What holds such a body is its contact with the ends of the opening, met in
//! its own motion (the rims of the material cast,
//! `ambition_platformer2d_core::frame::material_cast` `rim.rs`), so every
//! movement system that casts its body meets it as a solid. What is left here
//! is the question the transit and the placement ask of a body as it stands:
//! does it fit ([`fits_through`]), is it in line with the opening
//! ([`in_line_with`]), does it cut the plane over the opening
//! ([`cuts_opening`]). The body is its material box ([`Outline`]: the
//! canonical box under its material map, so a turned body is tested as it is,
//! not by the world box around it).

use bevy::prelude::*;

use ambition_platformer2d_core::{self as ae, BodyKinematics};

use crate::pieces::PortalAperture;

/// A body's material box in the world: its canonical box's four corners under
/// its material map, in order. A turned body's outline is what fits or does
/// not fit an opening; the world box around it can be far wider (portal P3:
/// a 20 by 2 body at 45 degrees has a 22 px world box).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outline(pub [Vec2; 4]);

impl Outline {
    /// `body`'s material box, or its world box when its material map is not
    /// usable (a numeric fault the kernel refuses on its own road).
    pub fn of(body: BodyKinematics) -> Self {
        let half = body.canonical_size * 0.5;
        body.material_map()
            .ok()
            .and_then(|map| {
                let corners = [Vec2::new(-half.x, -half.y), Vec2::new(half.x, -half.y), half, Vec2::new(-half.x, half.y)];
                let mut out = [Vec2::ZERO; 4];
                for (slot, corner) in out.iter_mut().zip(corners) {
                    *slot = map.point(corner).ok().filter(|p| p.is_finite())?;
                }
                Some(Self(out))
            })
            .unwrap_or_else(|| body.aabb().into())
    }

    pub fn center(&self) -> Vec2 {
        self.0.iter().copied().sum::<Vec2>() * 0.25
    }
}

impl From<ae::Aabb> for Outline {
    fn from(b: ae::Aabb) -> Self {
        let (lo, hi) = (b.min, b.max);
        Self([Vec2::new(lo.x, lo.y), Vec2::new(hi.x, lo.y), Vec2::new(hi.x, hi.y), Vec2::new(lo.x, hi.y)])
    }
}

/// Does `body`, as it is, go through `opening` whole? True when it does not
/// cut the opening's plane (it is on one side of it), or cuts it within the
/// opening. A body that cuts the plane past an end is snagged there, part
/// through and part around, and is not taken through (the transit asks
/// this before every transfer).
pub fn fits_through(body: impl Into<Outline>, opening: &PortalAperture) -> bool {
    let h = opening.half_length;
    cut(&corners(body, opening)).is_none_or(|(t0, t1)| t0 >= -h && t1 <= h)
}

/// Is `body` in line with `opening`: does all of it lie within the opening's
/// span along its face? Only a body in line can pass whole, so only one in
/// line opens the hole ([`crate::transit`]): a body beside the opening, as
/// one standing on a ledge whose top is the opening's end, leaves the host
/// whole under it.
pub fn in_line_with(body: impl Into<Outline>, opening: &PortalAperture) -> bool {
    let h = opening.half_length;
    corners(body, opening).iter().all(|p| p.x.abs() <= h)
}

/// Does `body` cut `opening`'s plane over the opening (not only past its
/// ends)?
pub(crate) fn cuts_opening(body: impl Into<Outline>, opening: &PortalAperture) -> bool {
    let h = opening.half_length;
    cut(&corners(body, opening)).is_some_and(|(t0, t1)| t1 > -h && t0 < h)
}

/// The outline's corners in the opening's own axes: along its face, and out
/// of it.
fn corners(b: impl Into<Outline>, opening: &PortalAperture) -> [Vec2; 4] {
    b.into().0.map(|p| {
        let d = p - opening.frame.origin;
        Vec2::new(d.dot(opening.frame.tangent()), d.dot(opening.frame.normal))
    })
}

/// Where a convex polygon in the opening's axes cuts its plane (`y = 0`):
/// the span along the face, `None` when it lies on one side.
fn cut(poly: &[Vec2]) -> Option<(f32, f32)> {
    let mut span: Option<(f32, f32)> = None;
    for (i, &a) in poly.iter().enumerate() {
        let b = poly[(i + 1) % poly.len()];
        if (a.y < 0.0) != (b.y < 0.0) {
            let t = a.x + (b.x - a.x) * (a.y / (a.y - b.y));
            span = Some(span.map_or((t, t), |(lo, hi)| (lo.min(t), hi.max(t))));
        }
    }
    span
}

#[cfg(test)]
mod tests;
