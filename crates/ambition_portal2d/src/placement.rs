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

/// Cast through placed portals. A refusal publishes no partial ray.
#[allow(clippy::too_many_arguments)]
pub fn raycast_through_portals(
    world: &ae::World,
    portals: &[PlacedPortal],
    origin: Vec2,
    dir: Vec2,
    max_dist: f32,
    include_one_way: bool,
    budget: ae::cast::RayBudget,
    convention: MapConvention,
) -> Result<Option<(Vec2, Vec2)>, ae::frame::thread::Refusal> {
    let mut ordered = portals.iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| crate::stable_portal_order(a, b));
    let links = ordered.into_iter().enumerate().filter_map(|(key, enter)| {
        let exit = find_portal(portals, enter.channel.partner())?;
        Some(ae::frame::thread::PortalLink::new(enter.aperture(),exit.aperture(),key as u32))
    }).collect::<Vec<_>>();
    ae::cast::trace_through_apertures(
        world, &links, ae::cast::PortalRay { origin, direction: dir, reach: max_dist, width: 0.0 },
        include_one_way, budget, convention,
    ).map(|trace| trace.hit)
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
) -> Result<Option<(Vec2, Vec2)>, ae::frame::thread::Refusal> {
    raycast_through_portals(
        world,
        portals,
        origin,
        dir,
        max_dist,
        include_one_way,
        ae::cast::RayBudget {
            max_hops: tuning.raycast_recursion_depth,
            max_scale_exponent: tuning.geometry.budget().max_scale_exponent,
            max_pieces: tuning.geometry.budget().max_pieces,
        },
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

/// Measure the body's width along the authored opening.
pub fn portal_fits(size: Vec2, portal: &PlacedPortal) -> bool {
    let normal = portal.normal.as_dvec2().normalize();
    let along = bevy::math::DVec2::new(-normal.y, normal.x);
    let cross = f64::from(size.x) * along.x.abs() + f64::from(size.y) * along.y.abs();
    cross <= f64::from(portal.aperture().half_length) * 2.0
}

/// Project the canonical box onto the opening's tangent.
fn material_fits(body: ae::BodyKinematics, down: Vec2, portal: &PlacedPortal) -> Result<bool, ae::frame::NumericFault> {
    let normal = portal.normal.as_dvec2().normalize();
    let tangent = bevy::math::DVec2::new(-normal.y, normal.x);
    Ok(body.projected_half_extent(down, tangent)? <= f64::from(portal.aperture().half_length))
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
    /// Commit the checked root crossing and its input policy.
    Transfer {
        transfer: PortalTransfer,
        roll_delta: f32,
        facing_flip: bool,
        input_warp: bool,
    },
    /// Leave the current body unchanged when the map cannot be represented.
    Refused {
        channel: Option<PortalChannel>,
        reason: ae::frame::thread::RefusalReason,
    },
    /// The body fully cleared the plane — remove [`PortalTransit`].
    Clear,
    /// Mid-transit, nothing to apply this frame.
    Continue,
}

/// The admitted geometric map and velocity samples for one root crossing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalTransfer {
    pub map: ae::frame::Sim2,
    pub enter: ae::frame::PortalAperture,
    pub exit: ae::frame::PortalAperture,
    pub entry_channel: PortalChannel,
    pub exit_channel: PortalChannel,
    pub crossing_time: f64,
    pub crossing_point: Vec2,
    pub entry_pos: Vec2,
    pub entry_vel: Vec2,
    /// Velocity after the surface-relative map, before exit-speed assistance.
    pub mapped_vel: Vec2,
    pub exit_pos: Vec2,
    pub exit_vel: Vec2,
}

impl PortalTransfer {
    /// Keep controller motion separate from motion supplied by the world.
    pub fn carried_run(self, side: Vec2, previous: f32) -> Result<f32, ae::frame::NumericFault> {
        use ae::frame::NumericFault;
        if !side.is_finite() || !previous.is_finite() {
            return Err(NumericFault::NonFinite);
        }
        let side = side.as_dvec2();
        let controller = self.entry_vel.as_dvec2().dot(side) - f64::from(previous);
        let mapped_side = self.map.orthogonal().as_dmat2() * side * f64::from(self.map.scale());
        let carried = self.exit_vel.as_dvec2().dot(side) - controller * mapped_side.dot(side);
        let narrowed = carried as f32;
        if !narrowed.is_finite() { return Err(NumericFault::Overflow); }
        if carried != 0.0 && narrowed == 0.0 { return Err(NumericFault::Underflow); }
        Ok(narrowed)
    }

    pub fn new(
        center: Vec2,
        velocity: Vec2,
        enter: &PlacedPortal,
        exit: &PlacedPortal,
        tuning: &PortalTuning,
    ) -> Result<Self, ae::frame::NumericFault> {
        Self::from_apertures(center, velocity, enter.channel, exit.channel,
            enter.aperture(), exit.aperture(), tuning)
    }

    fn from_apertures(
        center: Vec2, velocity: Vec2, entry_channel: PortalChannel, exit_channel: PortalChannel,
        entry: ae::frame::PortalAperture, leaving: ae::frame::PortalAperture, tuning: &PortalTuning,
    ) -> Result<Self, ae::frame::NumericFault> {
        use ae::frame::{NumericFault, Sim2};
        let map = Sim2::through(&entry, &leaving, tuning.convention.map_convention())?;
        let exit_pos = map.point(center)?;
        let mapped_vel = map.velocity(&entry.frame, &leaving.frame, center, velocity)?;
        if !tuning.min_exit_speed.is_finite() {
            return Err(NumericFault::NonFinite);
        }
        let frame = leaving.frame;
        let arm = exit_pos.as_dvec2() - frame.pivot.as_dvec2();
        let surface = frame.velocity.as_dvec2()
            + f64::from(frame.spin) * bevy::math::DVec2::new(-arm.y, arm.x);
        let normal = frame.normal.as_dvec2().normalize();
        let relative_speed = (mapped_vel.as_dvec2() - surface).dot(normal);
        let assisted = mapped_vel.as_dvec2()
            + normal * (f64::from(tuning.min_exit_speed) - relative_speed).max(0.0);
        let exit_vel = assisted.as_vec2();
        if !exit_vel.is_finite() {
            return Err(NumericFault::Overflow);
        }
        if (assisted.x != 0.0 && exit_vel.x == 0.0)
            || (assisted.y != 0.0 && exit_vel.y == 0.0)
        {
            return Err(NumericFault::Underflow);
        }
        Ok(Self {
            map, enter: entry, exit: leaving,
            entry_channel, exit_channel,
            crossing_time: 1.0, crossing_point: center,
            entry_pos: center, entry_vel: velocity, mapped_vel, exit_pos, exit_vel,
        })
    }
}

/// The first admitted root crossing and the sampled partner pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalCrossing {
    pub root: ae::frame::RootCrossing,
    pub exit: ae::frame::PortalAperture,
    pub entry_channel: PortalChannel,
    pub exit_channel: PortalChannel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrossingRefusal {
    pub channel: PortalChannel,
    pub reason: ae::frame::thread::RefusalReason,
}

fn transfer_step(
    crossing: PortalCrossing, center: Vec2, vel: Vec2, gravity_dir: Vec2, tuning: &PortalTuning,
) -> TransitStep {
    let enter = crossing.root.aperture;
    let exit = crossing.exit;
    let mut transfer = match PortalTransfer::from_apertures(crossing.root.point, vel,
        crossing.entry_channel, crossing.exit_channel, enter, exit, tuning) {
        Ok(transfer) => transfer,
        Err(reason) => return TransitStep::Refused {
            channel: Some(crossing.entry_channel), reason: ae::frame::thread::RefusalReason::Numeric(reason),
        },
    };
    transfer.crossing_time = crossing.root.world_time;
    transfer.crossing_point = crossing.root.point;
    transfer.entry_pos = center;
    transfer.exit_pos = match transfer.map.point(center) {
        Ok(point) => point,
        Err(reason) => return TransitStep::Refused {
            channel: Some(crossing.entry_channel), reason: ae::frame::thread::RefusalReason::Numeric(reason),
        },
    };
    let convention = tuning.convention.map_convention();
    TransitStep::Transfer {
        transfer,
        roll_delta: somersault_roll_for_convention(convention, enter.frame.normal, exit.frame.normal, gravity_dir),
        facing_flip: portal_facing_flips_for_convention(convention, enter.frame.normal, exit.frame.normal, gravity_dir),
        input_warp: portal_input_warp_flips_horizontal_for_convention(convention, enter.frame.normal, exit.frame.normal),
    }
}

/// Compute the transit step for a body. See [`TransitStep`]. `cooldown_pair`
/// is the body's post-jump latch, scoped to the pair it just crossed
/// ([`super::types::PortalTransitCooldown`]); `gravity_dir` selects whether a
/// transit tumbles or just turns around. This form updates overlap state.
/// It has no recorded travel and cannot transfer a root.
pub fn transit_step(
    body: ae::BodyKinematics,
    transit: Option<PortalTransit>,
    cooldown_pair: Option<PortalChannel>,
    portals: &[PlacedPortal],
    gravity_dir: Vec2,
) -> TransitStep {
    transit_step_with_tuning(
        body,
        None,
        transit,
        cooldown_pair,
        portals,
        gravity_dir,
        &PortalTuning::default(),
    )
}

/// The first crossing of an axis-aligned body over a complete world tick.
pub fn first_portal_crossing(
    from: Vec2, to: Vec2, size: Vec2, portals: &[PlacedPortal],
    backside: Option<crate::PortalBackside>,
) -> Result<Option<f64>, CrossingRefusal> {
    let domain = ae::CombatVolume::aabb(ae::Aabb::new(from, size * 0.5));
    let cast = ae::frame::material_cast::MaterialCast::new(
        ae::frame::thread::MaterialPrimitive::world(&domain),
        from,
        to - from,
        0.0..1.0,
    );
    first_portal_crossing_through(&cast, portals, backside, 1.0).map(|hit| hit.map(|hit| hit.root.toi))
}

/// Follow the recorded host motion and return the pair at the crossing time.
/// A refusal prevents the source operation from committing its candidate.
pub fn first_portal_crossing_through(
    cast: &ae::frame::material_cast::MaterialCast<'_>, portals: &[PlacedPortal],
    backside: Option<crate::PortalBackside>, until: f64,
) -> Result<Option<PortalCrossing>, CrossingRefusal> {
    let usable = crate::transit::usable_portals(portals, backside);
    let mut ordered = usable.iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| crate::stable_portal_order(a, b));
    let mut first: Option<PortalCrossing> = None;
    for enter in ordered {
        let Some(exit) = find_portal(usable.iter(), enter.channel.partner()) else { continue; };
        let refusal = |reason| CrossingRefusal { channel: enter.channel, reason };
        let Some(root) = cast.crossing(enter.step, enter.aperture(), until).map_err(refusal)? else { continue; };
        if first.is_some_and(|hit| hit.root.toi <= root.toi) { continue; }
        let exit_pose = exit.step.sample(exit.aperture(), root.world_time)
            .map_err(|reason| refusal(ae::frame::thread::RefusalReason::Numeric(reason)))?;
        first = Some(PortalCrossing {
            root, exit: exit_pose, entry_channel: enter.channel, exit_channel: exit.channel,
        });
    }
    Ok(first)
}

fn swept_crossing_step(
    sweep: ae::SweepSample, body: ae::BodyKinematics,
    portals: &[PlacedPortal], gravity_dir: Vec2, tuning: &PortalTuning,
) -> Option<TransitStep> {
    let center = body.pos;
    let vel = body.vel;
    let domain = ae::CombatVolume::aabb(ae::Aabb::new(Vec2::ZERO, body.canonical_size * 0.5));
    let cast = match sweep.material_cast(&domain) {
        Ok(cast) => cast,
        Err(refusal) => return Some(TransitStep::Refused { channel: None, reason: refusal.reason }),
    };
    match first_portal_crossing_through(&cast, portals, None, 1.0) {
        Ok(Some(hit)) => {
            let frame = hit.root.aperture.frame;
            let carried = if (vel - frame.velocity_at(hit.root.point)).dot(frame.normal) < 0.0 {
                vel
            } else { sweep.vel };
            Some(transfer_step(hit, center, carried, gravity_dir, tuning))
        }
        Ok(None) => None,
        Err(refusal) => Some(TransitStep::Refused { channel: Some(refusal.channel), reason: refusal.reason }),
    }
}

/// Compute transit from recorded travel. An overlap can begin or clear a
/// straddle, but only a directed root crossing can transfer the body.
#[allow(clippy::too_many_arguments)]
pub fn transit_step_with_tuning(
    kinematics: ae::BodyKinematics,
    sweep: Option<ae::SweepSample>,
    transit: Option<PortalTransit>,
    cooldown_pair: Option<PortalChannel>,
    portals: &[PlacedPortal],
    gravity_dir: Vec2,
    tuning: &PortalTuning,
) -> TransitStep {
    let center = kinematics.pos;
    let vel = kinematics.vel;
    let sweep = sweep.filter(|sample| sample.ending_at(center).is_some());
    let down = sweep.map_or(Vec2::ZERO, |sweep| sweep.down);
    let body = kinematics.aabb_oriented(down);
    // Resolve `(straddled, its linked exit)` for a color — both must be placed.
    let pair_for = |c: PortalChannel| -> Option<(PlacedPortal, PlacedPortal)> {
        Some((find_portal(portals, c)?, find_portal(portals, c.partner())?))
    };
    if let Some(sweep) = sweep {
        if let Some(step) = swept_crossing_step(sweep, kinematics, portals, gravity_dir, tuning) {
            return step;
        }
    }
    match transit {
        None => {
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
                let fits = match material_fits(kinematics, down, enter) {
                    Ok(fits) => fits,
                    Err(reason) => return TransitStep::Refused { channel: Some(enter.channel), reason: ae::frame::thread::RefusalReason::Numeric(reason) },
                };
                if !fits { continue; }
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
            let Some((enter, _exit)) = pair_for(t.straddling) else {
                return TransitStep::Clear;
            };
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
