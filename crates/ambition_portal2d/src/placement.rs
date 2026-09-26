//! Portal-aware geometry and the surface-fit / aperture-crossing decision logic.
//!
//! Plain solid raycasts live in `ambition_platformer2d_core::cast`; this
//! module keeps only the portal-specific traversal, the fit check, and the pure
//! `transit_step` decision machine shared by all body transit.

use bevy::prelude::*;

use crate::pieces::{self as pp, MapConvention, PortalFrame};
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_shared_tangle::transit::rotate_velocity_between_normals as portal_transform_velocity;

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

/// Does an actor of `size` fit through `portal`? The opening is the portal
/// extent perpendicular to its normal: a wall portal checks the actor's
/// height, and a floor or ceiling portal checks its width.
pub fn portal_fits(size: Vec2, portal: &PlacedPortal) -> bool {
    let normal_is_horizontal = portal.normal.x.abs() >= portal.normal.y.abs();
    let (opening, cross) = if normal_is_horizontal {
        (portal.half_extent.y * 2.0, size.y)
    } else {
        (portal.half_extent.x * 2.0, size.x)
    };
    cross <= opening
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
    if occluders.is_empty() {
        return probe_depth;
    }
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
    if found {
        depth.min(probe_depth)
    } else {
        probe_depth
    }
}

/// The capture box for a portal: the thin face grown by [`TRANSIT_BEGIN_MARGIN`].
pub(crate) fn capture_box(portal: &PlacedPortal) -> ae::Aabb {
    ae::Aabb::new(
        portal.pos,
        portal.half_extent + Vec2::splat(TRANSIT_BEGIN_MARGIN),
    )
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
pub(crate) fn approach_box(portal: &PlacedPortal) -> ae::Aabb {
    let capture = capture_box(portal);
    let n = portal.normal.normalize_or_zero();
    ae::Aabb::new(
        capture.center() + n * (APPROACH_CARVE_REACH * 0.5),
        capture.half_size() + n.abs() * (APPROACH_CARVE_REACH * 0.5),
    )
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
    // Galilean composition: v_out = map(v − v_enter) + v_exit. Static portals
    // have zero velocities.
    let mut vel_out =
        portal_transform_velocity(vel - ef.velocity, enter.normal, exit.normal, convention)
            + xf.velocity;
    // Floor the exit speed along the exit normal so a slow walk-in still
    // emerges. The floor applies in the exit aperture's rest frame.
    let rel_out = vel_out - xf.velocity;
    if rel_out.dot(exit.normal) < tuning.min_exit_speed {
        let tangential = rel_out - rel_out.dot(exit.normal) * exit.normal;
        vel_out = tangential + exit.normal * tuning.min_exit_speed + xf.velocity;
    }
    TransitStep::Transfer {
        pos: pp::map_point(center, &ef, &xf, convention),
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
        if !portal_fits(size, enter) {
            continue;
        }
        // Relative sweep: shift the body's start sample by the aperture's
        // frame displacement, then test against the aperture's end-of-frame
        // plane. A moving aperture over a still body gives a nonzero segment
        // and transits it (the "scoop").
        let rel_prev = prev.pos + enter.frame_delta();
        let rel_seg = center - rel_prev;
        if rel_seg.length_squared() <= 1e-6 {
            continue;
        }
        let ap = enter.aperture();
        let f0 = pp::front_distance(rel_prev, &ap.frame);
        let f1 = pp::front_distance(center, &ap.frame);
        // Crossed the plane INTO the wall this step.
        if f0 <= 0.0 || f1 > 0.0 {
            continue;
        }
        // Where along the (relative) segment the plane was crossed, and
        // whether that point is within the opening.
        let t = f0 / (f0 - f1);
        let at = rel_prev + rel_seg * t;
        let offset = (at - ap.frame.origin).dot(ap.frame.tangent()).abs();
        if offset <= ap.half_length + TRANSIT_BEGIN_MARGIN {
            // Carry the velocity that caused the crossing: the live `vel` if
            // it still points into the portal, else the previous sample (the
            // integrator may have stopped the body at the carve bottom).
            let carried = if vel.dot(enter.normal) < 0.0 {
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
                let hole = pp::carve_hole_with_depth(&ap, host_depths.depth(enter.channel));
                if pp::front_distance(center, &ap.frame) <= 0.0
                    && body.strict_intersects(hole)
                    && vel.dot(enter.normal) < 0.0
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
                // thin wall could reach the capture box through it.
                let capture = capture_box(enter);
                let front = pp::front_distance(center, &frame);
                let entering = front > 0.0 || vel.dot(enter.normal) < 0.0;
                if front >= -TRANSIT_BEGIN_MARGIN && entering && body.strict_intersects(capture) {
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
            if !t.crossed && pp::front_distance(center, &ef) <= 0.0 {
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
                body.strict_intersects(capture_box(&enter))
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
