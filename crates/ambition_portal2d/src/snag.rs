//! The snag: a body passes through an opening only as a whole.
//!
//! A rigid body that straddles a portal cuts the portal's plane along one
//! segment, and that segment must be in the opening: past an end, the plane
//! is the host. A body cut by the plane past an end is like a rope half
//! through a hole and half around it, and it snags on the end (Jon
//! 2026-10-16). It holds before the body's centre crosses and after: the
//! plane of the exit is the same seam, seen from the other side.
//!
//! A thick host makes this true by itself while the body comes in from the
//! front: its solid stands beside the hole. A line, a plank, or a slope a body
//! stands into by its feet does not, and from the far side of the seam no host
//! does, so the portal core states it. It is a constraint, the portal core's
//! second position authority beside eviction ([`crate::eviction`]): it moves
//! the body back to where it may be, and takes from its velocity what carried
//! it there.
//!
//! The body is a box, and the rule is a no-worse rule: a body is never moved
//! further past an end than it was, so a box standing into a slope beside an
//! opening (the kernel's feet rule) is left as it is. A body with a shape of
//! its own (a physics body) would answer the same question with its own
//! contact; [`snag_push`] is the rule for a box.

use bevy::prelude::*;

use ambition_platformer2d_core::{self as ae, AabbExt, BodyKinematics};
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;

use crate::pieces::PortalAperture;
use crate::rooms::PortalsByRoom;
use crate::transit::{FreePortalBody, PortalBackside};
use crate::types::{find_portal, PlacedPortal};

/// How a body that cuts `opening`'s plane past one of its ends moves back,
/// and the normal of what it is against. `None` when it does not cut the
/// plane over the opening (it is not going through: the host is what it is
/// against), when it cuts the plane within the opening, or when there is no
/// start (`came_from`, the same box at the start of the step) to say how it
/// got there.
///
/// How it got past the end says what it is against:
///
/// - it did not cut the plane before and came at it from the front: the
///   host beyond the end, so the part past the end goes back out along the
///   normal (from behind it is at the back of the portal: the host's
///   business, and [`PortalBackside`]'s);
/// - it cut the plane within the opening before (it slid along it): the end
///   of the opening, so it goes back along the face to the end;
/// - it cut the plane past the end before (a box standing into a slope
///   beside the opening, or one the snag holds on the host): the part past
///   the end goes no deeper into the side it was not on.
pub fn snag_push(body: ae::Aabb, came_from: Option<ae::Aabb>, opening: &PortalAperture) -> Option<(Vec2, Vec2)> {
    let came_from = came_from?;
    let h = opening.half_length;
    let here = corners(body, opening);
    let before = corners(came_from, opening);
    let (t0, t1) = cut(&here)?;
    if t1 <= -h || t0 >= h {
        return None;
    }
    let normal = opening.frame.normal;
    let tangent = opening.frame.tangent();
    for side in [1.0f32, -1.0] {
        let past = |(t0, t1): (f32, f32)| if side > 0.0 { t1 - h } else { -h - t0 };
        let over = past((t0, t1));
        if over <= 0.0 {
            continue;
        }
        return match cut(&before) {
            // It came at the plane from the front: the part past the end goes
            // back out. From behind it is at the back of the portal, which is
            // the host's (`PortalBackside`).
            None => {
                let beyond = clip(&here, |p| side * p.x - h);
                if !before.iter().all(|p| p.y >= 0.0) {
                    return None;
                }
                let deep = beyond.iter().map(|p| -p.y).fold(0.0, f32::max);
                (deep > 0.0).then_some((normal * deep, normal))
            }
            // It slid along the plane, from within the opening: back to the
            // end.
            Some(was) if past(was) <= 0.0 => Some((-tangent * side * over, -tangent * side)),
            // It cut the plane past the end already (its feet stand into a
            // slope beside the opening; the snag holds it on the host): the
            // part past the end goes no deeper into the side it is not on.
            Some(_) => {
                let beyond_now = clip(&here, |p| side * p.x - h);
                let beyond_was = clip(&before, |p| side * p.x - h);
                let depth = |poly: &[Vec2]| poly.iter().map(|p| -p.y).fold(0.0, f32::max);
                let height = |poly: &[Vec2]| poly.iter().map(|p| p.y).fold(0.0, f32::max);
                if height(&beyond_was) >= depth(&beyond_was) {
                    let more = depth(&beyond_now) - depth(&beyond_was);
                    (more > NO_WORSE).then_some((normal * more, normal))
                } else {
                    let more = height(&beyond_now) - height(&beyond_was);
                    (more > NO_WORSE).then_some((-normal * more, -normal))
                }
            }
        };
    }
    None
}

/// How much (px) a part past an end may go deeper in a step before it is
/// held: float noise, not motion.
const NO_WORSE: f32 = 1.0e-3;

/// How far (px) where a body cuts a plane may run past an end and still go
/// through: float noise, not a body part.
const FITS_SLACK: f32 = 0.5;

/// Does `body`, as it is, go through `opening` whole? True when it does not
/// cut the opening's plane (it is on one side of it), or cuts it within the
/// opening. A body that cuts the plane past an end is snagged there, part
/// through and part around, and is not taken through (the transit asks
/// this before every transfer).
pub fn fits_through(body: ae::Aabb, opening: &PortalAperture) -> bool {
    let h = opening.half_length + FITS_SLACK;
    cut(&corners(body, opening)).is_none_or(|(t0, t1)| t0 >= -h && t1 <= h)
}

/// Does `body` cut `opening`'s plane over the opening (not only past its
/// ends)?
pub(crate) fn cuts_opening(body: ae::Aabb, opening: &PortalAperture) -> bool {
    let h = opening.half_length;
    cut(&corners(body, opening)).is_some_and(|(t0, t1)| t1 > -h && t0 < h)
}

/// The box's corners in the opening's own axes: along its face, and out of
/// it.
fn corners(b: ae::Aabb, opening: &PortalAperture) -> [Vec2; 4] {
    let (lo, hi) = (b.min, b.max);
    [Vec2::new(lo.x, lo.y), Vec2::new(hi.x, lo.y), Vec2::new(hi.x, hi.y), Vec2::new(lo.x, hi.y)].map(|p| {
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

/// The part of the convex polygon `poly` where `keep(p) >= 0`, for a `keep`
/// that is linear in `p`.
fn clip(poly: &[Vec2], keep: impl Fn(Vec2) -> f32) -> Vec<Vec2> {
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

/// Velocity with what carried the body into `against` taken out, relative to
/// the portal's own motion.
fn stop_into(portal: &PlacedPortal, at: Vec2, vel: Vec2, against: Vec2) -> Vec2 {
    let into = portal.relative_velocity(at, vel).dot(against);
    if into < 0.0 { vel - against * into } else { vel }
}

/// The paired portals of `all` a body of `size` at `at` may be cutting the
/// plane of over the opening: the portals it could be going through.
fn openings_near<'a>(all: &'a [PlacedPortal], at: Vec2, size: Vec2) -> impl Iterator<Item = &'a PlacedPortal> {
    let reach = size.length() * 0.5;
    all.iter().filter(move |p| {
        let d = at - p.pos;
        d.dot(p.normal).abs() < reach
            && d.dot(p.aperture().frame.tangent()).abs() < p.opening_half_length() + reach
            && find_portal(all, p.channel.partner()).is_some()
            && crate::portal_fits(size, p)
    })
}

/// Snag every kernel body that cuts a portal's plane past an end of its
/// opening (see the module docs). Runs after eviction and before transit, so
/// the body the transit reads is one that fits. A body is read whether or
/// not it straddles yet: one step can take it from in front of the plane to
/// across it.
pub fn snag_straddlers(
    portals: Query<(Entity, &PlacedPortal)>,
    mut bodies: Query<(Entity, &mut BodyKinematics, Option<&mut ae::SweepSample>, Option<&PortalBackside>)>,
    live: LiveRooms,
) {
    let by_room = PortalsByRoom::collect(&portals, &live);
    if by_room.is_empty() {
        return;
    }
    for (entity, mut kin, mut sweep, backside) in &mut bodies {
        let all = by_room.in_room(live.of(entity));
        if all.is_empty() {
            continue;
        }
        let body = kin.collision_box(sweep.as_deref());
        for portal in openings_near(all, body.center(), body.half_size() * 2.0) {
            if backside.is_some_and(|back| back.of_pair(portal.channel)) {
                continue;
            }
            let body = kin.collision_box(sweep.as_deref());
            // The step's start, when the record is of this step.
            let came_from = sweep
                .as_deref()
                .and_then(|s| s.ending_at(kin.pos))
                .map(|s| ae::Aabb::new(body.center() + (s.prev - s.curr), body.half_size()));
            let Some((push, against)) = snag_push(body, came_from, &portal.aperture()) else {
                continue;
            };
            // A measurement trace (`RUST_LOG=ambition_portal::transit_trace=debug`).
            bevy::log::debug!(
                target: "ambition_portal::transit_trace",
                "snag body={entity} box={:?}..{:?} from={:?} push={push:?} against={against:?} portal={:?} half={} vel={:?}",
                body.min,
                body.max,
                came_from.map(|b| b.center()),
                portal.pos,
                portal.opening_half_length(),
                kin.vel
            );
            ambition_platformer2d_core::movement::carry_body(&mut kin, sweep.as_deref_mut(), push);
            kin.vel = stop_into(portal, kin.pos, kin.vel, against);
        }
    }
}

/// [`snag_straddlers`] for free bodies `B` (a thrown item).
pub fn snag_free_straddlers<B: FreePortalBody>(
    portals: Query<(Entity, &PlacedPortal)>,
    mut bodies: Query<(Entity, &mut B, Option<&B::Presence>, Option<&PortalBackside>)>,
    live: LiveRooms,
) {
    let by_room = PortalsByRoom::collect(&portals, &live);
    if by_room.is_empty() {
        return;
    }
    for (entity, mut body, presence, backside) in &mut bodies {
        let all = by_room.in_room(live.of(entity));
        if all.is_empty() || !B::present(presence) {
            continue;
        }
        let half = body.half_extent();
        for portal in openings_near(all, body.pos(), half * 2.0) {
            if backside.is_some_and(|back| back.of_pair(portal.channel)) {
                continue;
            }
            let (pos, vel) = (body.pos(), body.vel());
            // Its last step moves with it, so a second portal reads the step
            // from where this one left it.
            let came_from = body
                .last_step()
                .filter(|(_, to)| *to == pos)
                .map(|(from, _)| ae::Aabb::new(from, half));
            let Some((push, against)) = snag_push(ae::Aabb::new(pos, half), came_from, &portal.aperture()) else {
                continue;
            };
            let at = pos + push;
            body.emerge(at, stop_into(portal, at, vel, against));
        }
    }
}

#[cfg(test)]
mod tests;
