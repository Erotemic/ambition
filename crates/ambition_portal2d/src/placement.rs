//! Portal-aware geometry and the surface-fit / aperture-crossing decision logic.
//!
//! Plain solid raycasts live in `ambition_platformer2d_core::cast`; this
//! module keeps only the portal-specific traversal, the fit check, and the pure
//! `transit_step` decision machine shared by all body transit.

use bevy::prelude::*;

use crate::pieces::{self as pp, MapConvention, PortalFrame};
use ambition_platformer2d_core as ae;

use super::color::PortalChannel;
use super::transit::PortalTransit;
use super::tuning::PortalTuning;
use super::types::{find_portal, PlacedPortal};

/// Portal-aware raycast. If the ray enters a portal aperture from its front
/// before it hits a solid, the rest of the ray continues from the linked
/// portal. Line of sight, beams, grapples, and aim traces use this. The
/// returned `(hit, normal)` is in the chart where the ray ends. `max_depth`
/// stops two facing portals from looping forever.
///
/// The traversal is [`ae::cast::ray_through_apertures`]. This wrapper supplies
/// the aperture pairs from the placed portals; the caller supplies the map
/// convention.
#[allow(clippy::too_many_arguments)]
pub fn raycast_through_portals(
    world: &ae::World,
    portals: &[PlacedPortal],
    origin: Vec2,
    dir: Vec2,
    max_dist: f32,
    include_one_way: bool,
    max_depth: u32,
    convention: MapConvention,
) -> Option<(Vec2, Vec2)> {
    let pairs: Vec<(ae::frame::PortalAperture, ae::frame::PortalAperture)> = portals
        .iter()
        .filter_map(|enter| {
            let exit = find_portal(portals, enter.channel.partner())?;
            Some((enter.aperture(), exit.aperture()))
        })
        .collect();
    ae::cast::ray_through_apertures(
        world,
        &pairs,
        origin,
        dir,
        max_dist,
        include_one_way,
        max_depth,
        convention,
    )
}

/// Portal-aware raycast using the editable portal recursion budget.
pub fn raycast_through_portals_tuned(
    world: &ae::World,
    portals: &[PlacedPortal],
    origin: Vec2,
    dir: Vec2,
    max_dist: f32,
    include_one_way: bool,
    tuning: &PortalTuning,
) -> Option<(Vec2, Vec2)> {
    raycast_through_portals(
        world,
        portals,
        origin,
        dir,
        max_dist,
        include_one_way,
        tuning.raycast_recursion_depth,
        tuning.convention.map_convention(),
    )
}

pub fn portal_transit_roll(n_in: Vec2, n_out: Vec2) -> f32 {
    // Approach direction (-n_in) and exit direction (n_out), each flipped into
    // render space; the body turns by the signed angle between them.
    let into_render = Vec2::new(-n_in.x, n_in.y);
    let out_render = Vec2::new(n_out.x, -n_out.y);
    let dot = into_render.dot(out_render);
    let cross = into_render.x * out_render.y - into_render.y * out_render.x;
    cross.atan2(dot)
}

fn wall_to_wall(n_in: Vec2, n_out: Vec2, gravity_dir: Vec2) -> bool {
    let g = gravity_dir.normalize_or_zero();
    let in_wall = n_in.normalize_or_zero().dot(g).abs() < 0.5;
    let out_wall = n_out.normalize_or_zero().dot(g).abs() < 0.5;
    in_wall && out_wall
}

/// Convention-aware somersault policy.
///
/// Rotation convention (det +1): the body rolls by the render-space rotation
/// of the map. Reflection convention (det -1) cannot be a roll alone:
/// wall↔wall crossings stay upright and the mirror comes from
/// [`portal_facing_flips_for_convention`].
pub fn somersault_roll_for_convention(
    convention: MapConvention,
    n_in: Vec2,
    n_out: Vec2,
    gravity_dir: Vec2,
) -> f32 {
    if convention == MapConvention::Reflection && wall_to_wall(n_in, n_out, gravity_dir) {
        return 0.0;
    }
    portal_transit_roll(n_in, n_out)
}

/// Whether the body's horizontal facing flips through this portal pair.
///
/// Only under the reflection convention: wall↔wall roll is suppressed to keep
/// actors upright, so the facing flip supplies the mirror. Under the rotation
/// convention, roll carries the facing.
pub fn portal_facing_flips_for_convention(
    convention: MapConvention,
    n_in: Vec2,
    n_out: Vec2,
    gravity_dir: Vec2,
) -> bool {
    convention == MapConvention::Reflection
        && wall_to_wall(n_in, n_out, gravity_dir)
        && portal_transit_roll(n_in, n_out).abs() > std::f32::consts::FRAC_PI_2
}

/// Whether held horizontal input is mapped through the portal for a short
/// time after a transfer. True only when the map sends screen-horizontal to
/// the opposite horizontal direction. Floor↔wall turns map horizontal to
/// vertical, which the controller cannot express, so they rely on the
/// emergence guard only.
pub fn portal_input_warp_flips_horizontal_for_convention(
    convention: MapConvention,
    n_in: Vec2,
    n_out: Vec2,
) -> bool {
    let mapped = pp::portal_map_vec(Vec2::X, n_in, n_out, convention);
    mapped.x < -0.5 && mapped.y.abs() < 0.5
}

/// Does an actor of `size` fit through `portal`? The actor's box measured
/// along the face against the opening: a wall portal checks the actor's
/// height, a floor or ceiling portal its width, and a portal on a slope the
/// box's reach along the slope.
pub fn portal_fits(size: Vec2, portal: &PlacedPortal) -> bool {
    let along = portal.aperture().frame.tangent();
    let cross = size.x * along.x.abs() + size.y * along.y.abs();
    cross <= portal.aperture().half_length * 2.0 + 1.0e-3
}

/// Margin (px) added to a portal's thin face so a body resting on the surface
/// begins transit on contact. The carve opens only after transit begins.
pub(crate) const TRANSIT_BEGIN_MARGIN: f32 = 6.0;

/// The ray-parameter interval where `origin + t*dir` is inside `aabb` (slab
/// method), or `None` if the ray never enters it.
fn ray_interval(origin: Vec2, dir: Vec2, aabb: ae::Aabb) -> Option<(f32, f32)> {
    let inv = Vec2::new(1.0 / dir.x, 1.0 / dir.y);
    let t1 = (aabb.min - origin) * inv;
    let t2 = (aabb.max - origin) * inv;
    let near = t1.min(t2);
    let far = t1.max(t2);
    let t_near = near.x.max(near.y);
    let t_far = far.x.min(far.y);
    (t_near <= t_far).then_some((t_near, t_far))
}

/// How much solid host material is directly behind `frame`'s face along
/// `-normal`, probed at the aperture center. Consecutive solid intervals that
/// start at the face (within [`pp::SURFACE_GRACE`], for grid-snap error) are
/// merged; adjacent tiles extend the material and a gap ends it.
///
/// The host measures this each frame and publishes it through
/// [`PortalHostDepths`](crate::types::PortalHostDepths). The transit rescue,
/// the carve, and the view-window depth use it, so a thin wall's aperture
/// volume ends where the wall ends.
pub fn measure_host_depth(occluders: &[ae::Aabb], frame: &PortalFrame, probe_depth: f32) -> f32 {
    host_material_depth(occluders, frame, probe_depth).unwrap_or(probe_depth)
}

/// [`measure_host_depth`], `None` when no solid starts at the face: what is
/// behind the face is not known from these occluders (a portal on a line, or
/// one placed in the open).
pub fn host_material_depth(occluders: &[ae::Aabb], frame: &PortalFrame, probe_depth: f32) -> Option<f32> {
    let dir = -frame.normal;
    let mut intervals: Vec<(f32, f32)> = occluders
        .iter()
        .filter_map(|a| ray_interval(frame.origin, dir, *a))
        .filter(|(near, far)| *far > 0.0 && *near < probe_depth)
        .collect();
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut depth: f32 = 0.0;
    let mut found = false;
    for (near, far) in intervals {
        let reach = if found {
            depth + 0.5
        } else {
            pp::SURFACE_GRACE
        };
        if near <= reach {
            depth = depth.max(far);
            found = true;
        } else {
            break;
        }
    }
    found.then(|| depth.min(probe_depth))
}

/// The capture region for a portal: the thin face grown by
/// [`TRANSIT_BEGIN_MARGIN`], turned to the face.
pub(crate) fn capture_face(portal: &PlacedPortal) -> ae::frame::FaceRect {
    ae::frame::FaceRect {
        center: portal.pos,
        normal: portal.normal.normalize_or_zero(),
        half_along: portal.aperture().half_length + TRANSIT_BEGIN_MARGIN,
        half_out: portal.opening_half_thickness() + TRANSIT_BEGIN_MARGIN,
    }
}

/// Sized for terminal fall: sim steps are clamped to 1/30 s, and
/// `MAX_FALL_SPEED` (1900 px/s) is about 63 px per step. A faster body may see
/// the carve closed for one frame; the rescue in `transit_step` still
/// recovers the crossing. Opening early is harmless: the approach carve needs
/// the body to move into a placed, paired portal.
pub(crate) const APPROACH_CARVE_REACH: f32 = 96.0;

/// The capture box extended [`APPROACH_CARVE_REACH`] px outward along the
/// portal normal: where an inbound body must already see the surface open.
/// Geometric only, so frame time does not affect it; the caller adds a
/// "moving into the portal" velocity check.
pub(crate) fn approach_face(portal: &PlacedPortal) -> ae::frame::FaceRect {
    let capture = capture_face(portal);
    ae::frame::FaceRect {
        center: capture.center + capture.normal * (APPROACH_CARVE_REACH * 0.5),
        half_out: capture.half_out + APPROACH_CARVE_REACH * 0.5,
        ..capture
    }
}

/// One step of the transit machine for any body. Pure: from the body geometry,
/// transit and cooldown state, and the portals, it returns the action the
/// caller applies. Every body uses this one path.
#[derive(Clone, Copy, Debug)]
pub enum TransitStep {
    /// Not touching a portal (or latched) — do nothing.
    Idle,
    /// Begin transit into this portal: insert [`PortalTransit`], play ENTER sfx.
    Begin {
        channel: PortalChannel,
        portal_pos: Vec2,
    },
    /// The centroid crossed: move the body to `pos`, set velocity `vel`, add
    /// `roll_delta` to its roll, latch the cooldown, switch the straddled
    /// portal to `exit_channel`, mark crossed, and play the exit sfx.
    /// `warp_rot` is the `(cos, sin)` portal map; the input layer warps held
    /// movement by it so held input keeps carrying the body out.
    Transfer {
        pos: Vec2,
        vel: Vec2,
        roll_delta: f32,
        /// Mirror the body's horizontal facing (the wall↔wall "face out" rule).
        facing_flip: bool,
        /// Whether the held-input warp maps horizontal movement to the opposite
        /// horizontal direction for this transfer.
        input_warp: bool,
        /// Entry + exit portal normals — the held-input warp maps through them.
        enter_normal: Vec2,
        /// Outward normal of the exit portal. Emission protection uses it so
        /// held input cannot cancel the emergence.
        exit_normal: Vec2,
        exit_channel: PortalChannel,
        exit_pos: Vec2,
    },
    /// The body fully cleared the plane — remove [`PortalTransit`].
    Clear,
    /// Mid-transit, nothing to apply this frame.
    Continue,
}

/// Build the [`TransitStep::Transfer`] for a body crossing `enter` → `exit`.
/// The centroid crossing and the rescue both use it.
///
/// The exit position is the portal map of the centroid ([`pp::map_point`]).
/// Depth past the entry plane becomes the same depth in front of the exit
/// plane, and the along-surface offset is kept. The map is its own inverse
/// with enter and exit swapped. There is no push-out: the transfer fires on
/// the crossing frame, so the depth is small, and a large depth still maps
/// to the front of the exit.
fn transfer_step(
    center: Vec2,
    vel: Vec2,
    enter: PlacedPortal,
    exit: PlacedPortal,
    gravity_dir: Vec2,
    tuning: &PortalTuning,
) -> TransitStep {
    let convention = tuning.convention.map_convention();
    let ef = enter.frame();
    let xf = exit.frame();
    let exit_pos = pp::map_point(center, &ef, &xf, convention);
    // The body keeps its velocity relative to the surface it crosses: its
    // velocity relative to the entry's surface at `center`, mapped, plus the
    // exit's surface velocity where it comes out. A portal that moves or
    // turns gives what crosses it its motion there; static portals have none.
    let mut vel_out = ae::frame::map_velocity(&ef, &xf, convention, center, vel);
    // Floor the exit speed along the exit normal so a slow walk-in still
    // emerges. The floor applies in the exit surface's rest frame.
    let exit_surface = xf.velocity_at(exit_pos);
    let rel_out = vel_out - exit_surface;
    if rel_out.dot(exit.normal) < tuning.min_exit_speed {
        let tangential = rel_out - rel_out.dot(exit.normal) * exit.normal;
        vel_out = tangential + exit.normal * tuning.min_exit_speed + exit_surface;
    }
    TransitStep::Transfer {
        pos: exit_pos,
        vel: vel_out,
        // The body takes the on-screen turn (a tumble for floor/ceiling, none
        // for wall↔wall). `update_actor_roll` then eases it back upright. The
        // convention comes from `PortalTuning::convention`, not a global.
        roll_delta: somersault_roll_for_convention(
            convention,
            enter.normal,
            exit.normal,
            gravity_dir,
        ),
        facing_flip: portal_facing_flips_for_convention(
            convention,
            enter.normal,
            exit.normal,
            gravity_dir,
        ),
        input_warp: portal_input_warp_flips_horizontal_for_convention(
            convention,
            enter.normal,
            exit.normal,
        ),
        enter_normal: enter.normal,
        exit_normal: exit.normal,
        exit_channel: exit.channel,
        exit_pos: exit.pos,
    }
}

/// The movement-kernel sample for swept (CCD) transit: where the sim step
/// started and the body velocity then.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SweptSample {
    /// Authoritative body center at the previous transit step.
    pub pos: Vec2,
    /// Body velocity at the previous transit step (gates the sweep to
    /// segments that look like one frame of ballistic motion).
    pub vel: Vec2,
}

/// The largest sim step (s) one swept segment may represent. Matches the
/// 1/30 s sim-step clamp (also used for [`APPROACH_CARVE_REACH`]). A prev→now
/// displacement longer than `|prev_vel| * MAX_SWEPT_STEP_S` (plus slack) is a
/// respawn, reset, or teleport, and must not cross a portal plane.
const MAX_SWEPT_STEP_S: f32 = 1.0 / 30.0;

/// Compute the transit step for a body. See [`TransitStep`]. `cooldown_pair`
/// is the body's post-jump latch, scoped to the pair it just crossed
/// ([`super::types::PortalTransitCooldown`]); `gravity_dir` selects whether a
/// transit tumbles or just turns around. Discrete form: no swept sample,
/// default depths and tuning.
pub fn transit_step(
    center: Vec2,
    size: Vec2,
    vel: Vec2,
    transit: Option<PortalTransit>,
    cooldown_pair: Option<PortalChannel>,
    portals: &[PlacedPortal],
    gravity_dir: Vec2,
) -> TransitStep {
    transit_step_with_tuning(
        center,
        size,
        vel,
        None,
        transit,
        cooldown_pair,
        portals,
        gravity_dir,
        &super::types::PortalHostDepths::default(),
        &PortalTuning::default(),
    )
}

/// Where along `from -> to` a centre crosses `enter`'s plane front to behind
/// through its opening, as a fraction of the segment: the crossing every
/// swept transfer is made of.
fn plane_crossing(from: Vec2, to: Vec2, enter: &PlacedPortal) -> Option<f32> {
    if (to - from).length_squared() <= 1e-6 {
        return None;
    }
    let ap = enter.aperture();
    let f0 = pp::front_distance(from, &ap.frame);
    let f1 = pp::front_distance(to, &ap.frame);
    if f0 <= 0.0 || f1 > 0.0 {
        return None;
    }
    let t = f0 / (f0 - f1);
    let at = from + (to - from) * t;
    let offset = (at - ap.frame.origin).dot(ap.frame.tangent()).abs();
    (offset <= ap.half_length + TRANSIT_BEGIN_MARGIN).then_some(t)
}

/// Does a body of `size` that moved `from -> to` cross `enter` and get taken
/// by it, and where along the step (a fraction)? THE swept crossing: the
/// transit takes a body by it ([`swept_crossing_step`]), and a shot's
/// contacts stop where it says ([`first_portal_crossing`]), so the two can
/// not disagree. The step's start is carried with the aperture as it moved
/// and turned this tick (a moving portal's scoop); the centre crosses the
/// plane front to behind through the opening; and the body fits through the
/// opening whole where its centre crosses (`crate::snag`: a body that cuts
/// the plane past an end is snagged, not taken). Asked where the step ends,
/// a fast body is wholly past the plane there and fits trivially, though its
/// box went through the host beside the opening.
pub(crate) fn swept_crossing(from: Vec2, to: Vec2, size: Vec2, enter: &PlacedPortal, portals: &[PlacedPortal]) -> Option<f32> {
    find_portal(portals, enter.channel.partner())?;
    if !portal_fits(size, enter) {
        return None;
    }
    let rel_from = enter.frame().from_local(enter.prev_frame().to_local(from));
    let t = plane_crossing(rel_from, to, enter)?;
    let at = rel_from + (to - rel_from) * t;
    crate::snag::fits_through(ae::Aabb::new(at, size * 0.5), &enter.aperture()).then_some(t)
}

/// The first point of `from -> to` where a body's centre enters a paired
/// portal that takes it ([`swept_crossing`]): the fraction of the segment
/// still in the room it left. Past it the body is the pair's to carry, so
/// whatever it would touch on the far side of the plane, in the room it
/// left, it does not touch. A body on the back of a pair (`backside`) is not
/// taken by it.
pub fn first_portal_crossing(
    from: Vec2,
    to: Vec2,
    size: Vec2,
    portals: &[PlacedPortal],
    backside: Option<crate::PortalBackside>,
) -> Option<f32> {
    let usable = crate::transit::usable_portals(portals, backside);
    usable
        .iter()
        .filter_map(|enter| swept_crossing(from, to, size, enter, &usable))
        .min_by(f32::total_cmp)
}

/// The swept (CCD) crossing scan used by two arms of
/// [`transit_step_with_tuning`]. If the prev→now segment crossed a paired
/// portal's plane front→behind through its opening, build its Transfer.
#[allow(clippy::too_many_arguments)]
fn swept_crossing_step(
    sweep: Option<SweptSample>,
    center: Vec2,
    size: Vec2,
    vel: Vec2,
    portals: &[PlacedPortal],
    gravity_dir: Vec2,
    tuning: &PortalTuning,
) -> Option<TransitStep> {
    let prev = sweep?;
    let seg = center - prev.pos;
    let seg_len = seg.length();
    // Reject teleports (see `MAX_SWEPT_STEP_S`). Check the body segment only:
    // a moving aperture over a still body is not a teleport.
    let max_step = prev.vel.length() * MAX_SWEPT_STEP_S * 1.5 + TRANSIT_BEGIN_MARGIN;
    if seg_len > max_step {
        return None;
    }
    for enter in portals {
        let Some(exit) = find_portal(portals, enter.channel.partner()) else {
            continue;
        };
        // Relative sweep, in the aperture's own frame (a moving or turning
        // aperture over a still body gives a nonzero segment and transits
        // it: the "scoop"), whole or not at all: [`swept_crossing`].
        if swept_crossing(prev.pos, center, size, enter, portals).is_some() {
            // Carry the velocity that caused the crossing: the live `vel` if
            // it still points into the portal, else the previous sample (the
            // integrator may have stopped the body at the carve bottom).
            let carried = if enter.relative_velocity(center, vel).dot(enter.normal) < 0.0 {
                vel
            } else {
                prev.vel
            };
            return Some(transfer_step(
                center,
                carried,
                enter.clone(),
                exit,
                gravity_dir,
                tuning,
            ));
        }
    }
    None
}

/// Compute the transit step with editable portal tuning and host-measured
/// wall depths ([`PortalHostDepths`](super::types::PortalHostDepths)), so a
/// thin wall's rescue volume does not reach the room behind it.
#[allow(clippy::too_many_arguments)]
pub fn transit_step_with_tuning(
    center: Vec2,
    size: Vec2,
    vel: Vec2,
    sweep: Option<SweptSample>,
    transit: Option<PortalTransit>,
    cooldown_pair: Option<PortalChannel>,
    portals: &[PlacedPortal],
    gravity_dir: Vec2,
    host_depths: &super::types::PortalHostDepths,
    tuning: &PortalTuning,
) -> TransitStep {
    let body = ae::Aabb::new(center, size * 0.5);
    // Resolve `(straddled, its linked exit)` for a color — both must be placed.
    let pair_for = |c: PortalChannel| -> Option<(PlacedPortal, PlacedPortal)> {
        Some((find_portal(portals, c)?, find_portal(portals, c.partner())?))
    };
    match transit {
        None => {
            // Rescue (runs even on cooldown). The carve opens on overlap, but
            // Begin is blocked by the cooldown after a jump. Without this, a
            // quick floor↔floor bounce sinks into the open hole and grounds
            // there.
            //
            // Gate: the body intersects the carve hole with its centroid past
            // the plane. The only way into the hole is through the aperture, so
            // this is safe at any depth and at any dt (a fast fall can skip a
            // frame where the plane cuts the body). The body must also move
            // into the portal (`vel · normal < 0`), so a body that just emerged
            // is not grabbed again.
            for enter in portals {
                if find_portal(portals, enter.channel.partner()).is_none() {
                    continue;
                }
                if !portal_fits(size, enter) {
                    continue;
                }
                let ap = enter.aperture();
                // Bounded by the measured host depth, so a body behind a thin
                // wall is not grabbed.
                let hole = pp::carve_face_with_depth(&ap, host_depths.depth(enter.channel));
                // In line with the opening: only a body that came through it
                // is in its hole whole (one beside it went through the host,
                // and is not taken; one past the plane fits trivially).
                if pp::front_distance(center, &ap.frame) <= 0.0
                    && hole.strict_intersects(body)
                    && enter.relative_velocity(center, vel).dot(enter.normal) < 0.0
                    && crate::snag::in_line_with(body, &ap)
                {
                    let exit = find_portal(portals, enter.channel.partner())
                        .expect("partner checked above");
                    return transfer_step(center, vel, enter.clone(), exit, gravity_dir, tuning);
                }
            }
            // Swept trigger: if the prev→now segment crossed the entry plane
            // front→behind within the aperture, transfer at any depth.
            // `map_point` keeps depth and momentum.
            //
            // Guards:
            // * Direction comes from the segment, not the live `vel`, which
            //   the integrator may have zeroed at the carve bottom.
            // * The segment must be one frame of ballistic motion (see
            //   `MAX_SWEPT_STEP_S`), not a respawn or teleport.
            if let Some(step) =
                swept_crossing_step(sweep, center, size, vel, portals, gravity_dir, tuning)
            {
                return step;
            }
            // Begin into the first portal (across all pairs) the body enters.
            // The cooldown blocks only the pair just crossed.
            for enter in portals {
                if cooldown_pair.is_some_and(|c| c == enter.channel || c == enter.channel.partner())
                {
                    continue;
                }
                // Need the partner placed, or there's no exit to transit to.
                if find_portal(portals, enter.channel.partner()).is_none() {
                    continue;
                }
                if !portal_fits(size, enter) {
                    continue;
                }
                let frame = enter.frame();
                // Begin when the leading face reaches the opening from the
                // front: the centroid must be on the room side of the plane
                // (within `TRANSIT_BEGIN_MARGIN`). Otherwise a body behind a
                // thin wall could reach the capture box through it. And in
                // line with the opening, as for its hole (`body_opens`): a body
                // beside it, past an end, is not going through.
                let capture = capture_face(enter);
                let front = pp::front_distance(center, &frame);
                let entering = front > 0.0 || enter.relative_velocity(center, vel).dot(enter.normal) < 0.0;
                if front >= -TRANSIT_BEGIN_MARGIN
                    && entering
                    && crate::snag::in_line_with(body, &enter.aperture())
                    && capture.strict_intersects(body)
                {
                    return TransitStep::Begin {
                        channel: enter.channel,
                        portal_pos: enter.pos,
                    };
                }
            }
            TransitStep::Idle
        }
        Some(t) => {
            // The straddled portal or its partner was removed → end transit.
            let Some((enter, exit)) = pair_for(t.straddling) else {
                return TransitStep::Clear;
            };
            let ef = enter.frame();
            // The centroid crossing the plane triggers the transfer. Gameplay
            // sees no discontinuity because queries use the portal pieces.
            // Whole, or not at all (`crate::snag`): a body that cuts the plane
            // past an end stays straddling until it fits.
            if !t.crossed
                && pp::front_distance(center, &ef) <= 0.0
                && crate::snag::fits_through(body, &enter.aperture())
            {
                return transfer_step(center, vel, enter, exit, gravity_dir, tuning);
            }
            // Swept re-crossing while the post-transfer latch clears. On a fast
            // portal loop, the body can cross the next aperture in under one
            // frame, while `crossed` is still set. Without this, the frame is
            // spent on Clear and the body embeds. The `!crossed` case needs no
            // sweep: the centroid test above works at any depth.
            if t.crossed {
                if let Some(step) =
                    swept_crossing_step(sweep, center, size, vel, portals, gravity_dir, tuning)
                {
                    return step;
                }
            }
            // Stay engaged so the carve stays open while the body sinks and
            // crosses. The cooldown latch (set on transfer) stops re-entry.
            let still_engaged = if t.crossed {
                pp::straddles(body, &enter.aperture())
            } else {
                capture_face(&enter).strict_intersects(body)
            };
            if still_engaged {
                TransitStep::Continue
            } else {
                TransitStep::Clear
            }
        }
    }
}

#[cfg(test)]
mod tests;
