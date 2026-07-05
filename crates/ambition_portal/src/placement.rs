//! Portal-aware geometry and the surface-fit / aperture-crossing decision logic.
//!
//! Plain solid raycasts live in `ambition_platformer_primitives::world_query`; this
//! module keeps only the portal-specific traversal, the fit check, and the pure
//! `transit_step` decision machine shared by all opted-in actor transit.

use bevy::prelude::*;

use crate::pieces::{self as pp, PortalFrame};
use ambition_engine_core::{self as ae, AabbExt};
use ambition_platformer_primitives::transit::rotate_velocity_between_normals as portal_transform_velocity;
use ambition_platformer_primitives::world_query::{ray_aabb, raycast_solids};

use super::color::PortalChannel;
use super::transit::PortalTransit;
use super::tuning::PortalTuning;
use super::types::{find_portal, PlacedPortal};

/// Recursive, portal-aware raycast: cast from `origin` along `dir`, and if the
/// ray crosses a portal face (entering from its front) before hitting a solid,
/// transform the remaining ray through the linked portal and continue — so line
/// of sight, beams, grapples, and aim traces "see through" a portal pair. The
/// returned `(hit, normal)` is in the chart where the ray finally lands. Bounded
/// by `max_depth` so two portals facing each other can't loop forever.
pub fn raycast_through_portals(
    world: &ae::World,
    portals: &[PlacedPortal],
    origin: Vec2,
    dir: Vec2,
    max_dist: f32,
    include_one_way: bool,
    max_depth: u32,
) -> Option<(Vec2, Vec2)> {
    let mut origin = origin;
    let mut dir = dir.normalize_or_zero();
    if dir == Vec2::ZERO {
        return None;
    }
    let mut budget = max_dist;
    for _ in 0..=max_depth {
        let solid = raycast_solids(world, origin, dir, budget, include_one_way);
        let solid_t = solid
            .map(|(hit, _)| (hit - origin).length())
            .unwrap_or(f32::INFINITY);
        // Nearest portal face the ray ENTERS (front side) before that solid —
        // across ALL placed pairs, each portal redirecting to its partner.
        let mut nearest: Option<(f32, PlacedPortal, PlacedPortal)> = None;
        for enter in portals {
            let Some(exit) = find_portal(portals, enter.channel.partner()) else {
                continue;
            };
            // Only enter through the front of the face (moving into it).
            if dir.dot(enter.normal) >= 0.0 {
                continue;
            }
            if let Some((t, _)) = ray_aabb(origin, dir, ae::Aabb::new(enter.pos, enter.half_extent))
            {
                if t <= budget && t < solid_t && nearest.map_or(true, |(bt, _, _)| t < bt) {
                    nearest = Some((t, *enter, exit));
                }
            }
        }
        match nearest {
            Some((t, enter, exit)) => {
                let entry = origin + dir * t;
                // Emerge just out of the exit face, redirected through the pair.
                origin = pp::map_point(entry, &enter.frame(), &exit.frame()) + exit.normal;
                dir = pp::portal_map_vec(dir, enter.normal, exit.normal).normalize_or_zero();
                budget -= t;
                if budget <= 0.0 || dir == Vec2::ZERO {
                    return None;
                }
            }
            None => return solid,
        }
    }
    None
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
    )
}

/// The render-space roll a body picks up traveling through a portal pair: the
/// signed on-screen angle its motion turns through — from "into the entry"
/// (`-n_in`) to "out of the exit" (`n_out`), measured in RENDER space (y
/// flipped). Computing it as the render-space turn directly (rather than a
/// world rotation we then conjugate) keeps the sign unambiguous. Fully general
/// for ANY two portal angles: floor↔floor = ±π, floor↔wall = ±π/2, slanted
/// pairs = whatever the normals give. A body entering feet-first leaves
/// feet-first along its new velocity.
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
/// Rotation convention (det +1) is a proper orientation map, so the body picks
/// up exactly the render-space rotation of the map. Reflection convention
/// (det -1) cannot be represented by roll alone; it keeps the historical
/// gravity-platformer accommodation where wall↔wall crossings stay upright and
/// express their mirror through [`portal_facing_flips_for_convention`].
pub fn somersault_roll_for_convention(
    rotation_convention: bool,
    n_in: Vec2,
    n_out: Vec2,
    gravity_dir: Vec2,
) -> f32 {
    if !rotation_convention && wall_to_wall(n_in, n_out, gravity_dir) {
        return 0.0;
    }
    portal_transit_roll(n_in, n_out)
}

/// The somersault roll a body picks up crossing a portal pair under the active
/// game-wide map convention.
pub fn somersault_roll(n_in: Vec2, n_out: Vec2, gravity_dir: Vec2) -> f32 {
    somersault_roll_for_convention(pp::portal_map_rotation(), n_in, n_out, gravity_dir)
}

/// Whether the body's horizontal FACING flips through this portal pair.
///
/// This is needed only under the reflection convention. A same-wall reflection
/// is a horizontal mirror, but the visual policy suppresses wall↔wall roll to
/// keep actors gravity-upright; the facing flip supplies the missing mirror so
/// the leading side still leads out. Under rotation convention the map is a
/// proper rotation, so facing is carried by roll and no separate mirror applies.
pub fn portal_facing_flips_for_convention(
    rotation_convention: bool,
    n_in: Vec2,
    n_out: Vec2,
    gravity_dir: Vec2,
) -> bool {
    !rotation_convention
        && wall_to_wall(n_in, n_out, gravity_dir)
        && portal_transit_roll(n_in, n_out).abs() > std::f32::consts::FRAC_PI_2
}

/// Whether the body's horizontal FACING flips through this portal pair under
/// the active game-wide map convention.
pub fn portal_facing_flips(n_in: Vec2, n_out: Vec2, gravity_dir: Vec2) -> bool {
    portal_facing_flips_for_convention(pp::portal_map_rotation(), n_in, n_out, gravity_dir)
}

/// Whether held horizontal movement should be temporarily mapped through the
/// portal after a transfer. This is an input-feel accommodation, but the gate is
/// mathematical: apply it only when the active map sends screen-horizontal input
/// to the opposite screen-horizontal direction. Floor↔wall turns map horizontal
/// input into vertical movement, which the platformer controller cannot express
/// as ordinary movement, so they stay on the emergence guard alone.
pub fn portal_input_warp_flips_horizontal_for_convention(
    rotation_convention: bool,
    n_in: Vec2,
    n_out: Vec2,
) -> bool {
    let mapped = if rotation_convention {
        pp::portal_map_vec_rotation(Vec2::X, n_in, n_out)
    } else {
        pp::portal_map_vec_reflection(Vec2::X, n_in, n_out)
    };
    mapped.x < -0.5 && mapped.y.abs() < 0.5
}

/// Whether held horizontal movement should be temporarily mapped through the
/// portal after a transfer under the active game-wide convention.
pub fn portal_input_warp_flips_horizontal(n_in: Vec2, n_out: Vec2) -> bool {
    portal_input_warp_flips_horizontal_for_convention(pp::portal_map_rotation(), n_in, n_out)
}

/// Does an actor of `size` fit through `portal`? The opening the actor must
/// pass through is the portal extent **perpendicular to its normal**: a wall
/// portal (horizontal normal) is a vertical doorway, so the actor's *height*
/// must fit; a floor / ceiling portal (vertical normal) gates on *width*. This
/// keeps big bosses out of small portals while staying fully general — make a
/// huge portal (or shrink the boss) and it passes.
pub fn portal_fits(size: Vec2, portal: &PlacedPortal) -> bool {
    let normal_is_horizontal = portal.normal.x.abs() >= portal.normal.y.abs();
    let (opening, cross) = if normal_is_horizontal {
        (portal.half_extent.y * 2.0, size.y)
    } else {
        (portal.half_extent.x * 2.0, size.x)
    };
    cross <= opening
}

/// Margin (px) added to a portal's thin face so a body resting against the
/// surface registers as "entering" before it has visibly sunk in (the carve
/// only opens once transit has begun, so begin must trigger on contact).
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

/// How much solid host material sits directly behind `frame`'s face along
/// `-normal`, probed at the aperture center: the merged extent of consecutive
/// solid intervals starting at (or within [`pp::SURFACE_GRACE`] of — the
/// authored face can sit a grid-snap off the collision edge) the face.
/// Exactly-adjacent blocks (merged tiles) extend the material; a real gap
/// behind the wall ends it. Returns `probe_depth` unclipped when no host
/// material is found (e.g. a portal on a one-way platform excluded from the
/// solid snapshot) — the clip only ever engages on measured geometry.
///
/// The HOST measures this each frame (it owns the collision world) and
/// publishes it via [`PortalHostDepths`](crate::types::PortalHostDepths); the
/// transit rescue, the carve, and the view-window depth all bound their
/// behind-the-face reach by it so a THIN wall's aperture volume ends where
/// the wall does.
pub fn measure_host_depth(occluders: &[ae::Aabb], frame: &PortalFrame, probe_depth: f32) -> f32 {
    if occluders.is_empty() {
        return probe_depth;
    }
    let dir = -frame.normal;
    let mut intervals: Vec<(f32, f32)> = occluders
        .iter()
        .filter_map(|a| ray_interval(frame.pos, dir, *a))
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
/// A body whose AABB intersects this box is "in the opening" — both the transit
/// `Begin` decision AND the host-surface carve key off it, so the floor opens the
/// exact frame transit begins (no one-frame lag where the body grounds on a
/// still-solid floor before the carve appears).
pub(crate) fn capture_box(portal: &PlacedPortal) -> ae::Aabb {
    ae::Aabb::new(
        portal.pos,
        portal.half_extent + Vec2::splat(TRANSIT_BEGIN_MARGIN),
    )
}

/// How far OUTWARD of a portal face (px) the carve's approach test reaches.
/// Must cover the largest distance any transiting body can travel in ONE frame,
/// so the host surface is already open on the frame a fast body crosses the
/// opening — **without knowing that frame's dt** (a dt-dependent sweep is
/// unfixably fragile: the carve publishes before the frame's clock refresh, so
/// any dt it reads is stale, and a frame hitch at re-entry under-sweeps and
/// grounds the body, killing its momentum). Budget: Ambition clamps controlled
/// actor sim steps to 1/30 s, so 1900 px/s terminal fall (`MAX_FALL_SPEED`)
/// ⇒ ~63px/frame — 96px covers it with slack. A body even faster on a hard
/// hitch may see the carve closed for ONE frame, but the carve-volume rescue
/// in `transit_step` recovers the crossing regardless. Opening a few frames
/// early is harmless: the approach carve is gated on the body MOVING INTO the
/// portal, and a hole only ever opens where a placed, paired portal already is.
pub(crate) const APPROACH_CARVE_REACH: f32 = 96.0;

/// The capture box extended [`APPROACH_CARVE_REACH`] px OUTWARD along the
/// portal's normal (into the room): the region in which an inbound body must
/// already see the surface open. Purely geometric — no dt, no velocity — so the
/// carve decision is immune to frame-time jitter; the caller pairs it with a
/// "moving into the portal" velocity gate.
pub(crate) fn approach_box(portal: &PlacedPortal) -> ae::Aabb {
    let capture = capture_box(portal);
    let n = portal.normal.normalize_or_zero();
    ae::Aabb::new(
        capture.center() + n * (APPROACH_CARVE_REACH * 0.5),
        capture.half_size() + n.abs() * (APPROACH_CARVE_REACH * 0.5),
    )
}

/// One step of the aperture / centroid-crossing transit machine for ANY body.
/// Pure: given the body's geometry + current transit/cooldown state + the portal
/// pair, it returns the action the caller applies. Shared by every opted-in
/// actor/body so portal crossings use one invariant path.
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
    /// `roll_delta` to its roll (the somersault), latch the cooldown, flip the
    /// straddled portal to `exit_channel`, mark crossed, play EXIT sfx. `warp_rot`
    /// is the `(cos, sin)` portal map (same rotation applied to velocity) — the
    /// host input layer warps held movement input by it so the held direction
    /// keeps carrying the body OUT instead of fighting the warped velocity.
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
        /// Outward normal of the exit portal — the direction the body emerges.
        /// Used by emission protection so held input can't cancel the emergence.
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
/// Shared by the mid-transit centroid crossing and the cooldown-bypassing rescue
/// so both emerge identically.
///
/// The exit position is the plain portal map of the centroid ([`pp::map_point`]):
/// a reversible topological glue — the depth the centroid has sunk PAST the entry
/// plane becomes the depth it emerges in FRONT of the exit plane, and the
/// along-surface offset is preserved. So a centroid that just barely crossed
/// emerges just barely in front of the exit, and an equal step back inverts the
/// move exactly (`map_point` is its own inverse with enter/exit swapped). No
/// artificial push-out: the centroid transfer (and the rescue) fire the frame the
/// centroid crosses, so the sink depth is small and the body emerges right at the
/// exit face rather than embedded behind it — the small-ε case is the common one,
/// and a large-dt crossing still maps to (a large) depth IN FRONT, never behind.
fn transfer_step(
    center: Vec2,
    vel: Vec2,
    enter: PlacedPortal,
    exit: PlacedPortal,
    gravity_dir: Vec2,
    tuning: &PortalTuning,
) -> TransitStep {
    let ef = enter.frame();
    let xf = exit.frame();
    let mut vel_out = portal_transform_velocity(vel, enter.normal, exit.normal);
    // Floor the exit speed along the exit normal so a slow walk-in still emerges
    // instead of stalling in the opening.
    if vel_out.dot(exit.normal) < tuning.min_exit_speed {
        let tangential = vel_out - vel_out.dot(exit.normal) * exit.normal;
        vel_out = tangential + exit.normal * tuning.min_exit_speed;
    }
    TransitStep::Transfer {
        pos: pp::map_point(center, &ef, &xf),
        vel: vel_out,
        // The body picks up the on-screen turn it travels through (a tumble for
        // floor/ceiling, nothing for a wall↔wall turn-around); `update_actor_roll`
        // then eases it back to gravity-upright (feet-in → reorient).
        roll_delta: somersault_roll(enter.normal, exit.normal, gravity_dir),
        facing_flip: portal_facing_flips(enter.normal, exit.normal, gravity_dir),
        input_warp: portal_input_warp_flips_horizontal(enter.normal, exit.normal),
        enter_normal: enter.normal,
        exit_normal: exit.normal,
        exit_channel: exit.channel,
        exit_pos: exit.pos,
    }
}

/// The body's PREVIOUS authoritative sample for the swept (CCD) transit tier:
/// where it was last frame and how fast it was moving then. The caller (the
/// transit system's `PortalSweepAnchor`) records the TRUE last-frame position —
/// not `pos - vel * dt` — because the very failure the sweep exists to fix
/// (a high-speed fall stopped/grounded at the carve bottom) zeroes the body's
/// live velocity, which would erase a reconstructed segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SweptSample {
    /// Authoritative body center at the previous transit step.
    pub pos: Vec2,
    /// Body velocity at the previous transit step (gates the sweep to
    /// segments that look like one frame of ballistic motion).
    pub vel: Vec2,
}

/// The largest sim step (s) one swept segment may represent. Mirrors the
/// Ambition 1/30 s controlled-body sim-step clamp (the same host budget
/// [`APPROACH_CARVE_REACH`] is sized against): a prev→now displacement longer
/// than `|prev_vel| * MAX_SWEPT_STEP_S` (+ slack) is NOT one frame of ballistic
/// motion — it is a respawn / reset / scripted teleport — and must never be
/// treated as travel that can cross a portal plane.
const MAX_SWEPT_STEP_S: f32 = 1.0 / 30.0;

/// Compute the transit step for a body. See [`TransitStep`]. `cooldown_pair`
/// is the body's post-jump latch, scoped to the pair it just crossed
/// ([`super::types::PortalTransitCooldown`]); `gravity_dir` selects whether a
/// transit tumbles or just turns around. The discrete convenience — no swept
/// sample, default depths/tuning.
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

/// The SWEPT (CCD) crossing scan shared by the unlatched and post-transfer
/// arms of [`transit_step_with_tuning`]: did the prev→now SEGMENT cross a
/// paired portal's plane front→behind through its opening? If so, the body
/// physically fell through the aperture this frame — build its Transfer.
///
/// Known bound (documented, not defended): the scan resolves ONE crossing per
/// step, so a body travelling more than a whole portal-loop's length in a
/// single frame (e.g. > 680px/frame on the c135↔c134 pair) can out-run one
/// transfer per frame. That is several times terminal velocity through a
/// gameplay loop; the regression pins correctness to ~600px/frame.
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
    // One frame of ballistic motion, or a teleport? (See MAX_SWEPT_STEP_S.)
    let max_step = prev.vel.length() * MAX_SWEPT_STEP_S * 1.5 + TRANSIT_BEGIN_MARGIN;
    if seg_len <= 1e-3 || seg_len > max_step {
        return None;
    }
    for enter in portals {
        let Some(exit) = find_portal(portals, enter.channel.partner()) else {
            continue;
        };
        if !portal_fits(size, enter) {
            continue;
        }
        let ef = enter.frame();
        let f0 = pp::front_distance(prev.pos, &ef);
        let f1 = pp::front_distance(center, &ef);
        // Crossed the plane INTO the wall this step.
        if f0 <= 0.0 || f1 > 0.0 {
            continue;
        }
        // Where along the segment the plane was crossed, and whether that
        // point is within the opening.
        let t = f0 / (f0 - f1);
        let at = prev.pos + seg * t;
        let along = Vec2::new(-ef.normal.y, ef.normal.x);
        let offset = (at - ef.pos).dot(along).abs();
        if offset <= ef.aperture_half() + TRANSIT_BEGIN_MARGIN {
            // Carry the velocity that PRODUCED the crossing: the live `vel`
            // when it still points into the portal (unobstructed fast
            // crossing), else the previous sample's (the integrator
            // stopped/zeroed the body at the carve bottom AFTER it crossed —
            // the exit must still get the entry momentum).
            let carried = if vel.dot(enter.normal) < 0.0 {
                vel
            } else {
                prev.vel
            };
            return Some(transfer_step(
                center,
                carried,
                *enter,
                exit,
                gravity_dir,
                tuning,
            ));
        }
    }
    None
}

/// Compute the transit step with editable portal tuning and the host-measured
/// wall depths (see [`PortalHostDepths`](super::types::PortalHostDepths) — the
/// rescue's aperture volume is bounded by the host material so a thin wall
/// never grabs a body in the open room behind it).
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
            // RESCUE / commit (runs EVEN on cooldown): if the body's centroid has
            // reached or passed a portal plane while the body still straddles its
            // opening, it is physically in the act of falling through — transfer it
            // NOW. The host-surface carve opens on geometric overlap (so the floor
            // is non-solid while a body is in the opening), but the gentle Begin
            // below is cooldown-blocked for a short window after a jump. Without
            // this rescue, a body that falls back into an open carve DURING that
            // cooldown (e.g. a quick floor↔floor bounce whose airtime is shorter
            // than the cooldown) sinks to the bottom of the open hole and grounds
            // there — "stuck in the middle of the floor", its momentum killed.
            // The gate is the OPEN aperture volume itself (the carve hole): the
            // body must intersect it with its centroid past the plane. This
            // bounds the rescue to the opening — a body legitimately below the
            // surface elsewhere is never teleported — while staying dt-robust:
            // the old `straddles` gate required the plane to pass THROUGH the
            // body on a sampled frame, which a fast fall (1900 px/s terminal ≈
            // 63 px at the 1/30 s sim-step clamp, vs a ~40 px body) can skip
            // entirely, grounding the body at the bottom of the open hole with
            // its momentum killed. Inside the carve volume the only way in was
            // through the aperture, so a deep crossing is still a crossing.
            // The body must also be moving INTO the portal (`vel · normal < 0`):
            // that distinguishes a body falling THROUGH the opening (rescue it)
            // from one that JUST EMERGED from this portal and is moving back out
            // (do NOT re-grab it — the transfer maps the centroid right onto the
            // exit plane, so without the velocity gate the rescue would
            // immediately fire again and ping-pong).
            for enter in portals {
                if find_portal(portals, enter.channel.partner()).is_none() {
                    continue;
                }
                if !portal_fits(size, enter) {
                    continue;
                }
                let ef = enter.frame();
                // The hole is bounded by the measured host material: on a
                // thin wall the aperture volume ends at the wall's far face,
                // so a body in the open room BEHIND it is never grabbed.
                let hole = pp::carve_hole_with_depth(&ef, host_depths.depth(enter.channel));
                if pp::front_distance(center, &ef) <= 0.0
                    && body.strict_intersects(hole)
                    && vel.dot(enter.normal) < 0.0
                {
                    let exit = find_portal(portals, enter.channel.partner())
                        .expect("partner checked above");
                    return transfer_step(center, vel, *enter, exit, gravity_dir, tuning);
                }
            }
            // SWEPT crossing (CCD — the §7.6 high-speed tier, runs EVEN on
            // cooldown like the rescue): at speeds past the carve budget
            // (`APPROACH_CARVE_REACH` / `CARVE_DEPTH` are sized for ~63 px/frame;
            // the relaxed fall cap on an accelerating portal loop exceeds that
            // without bound) one frame's step can jump the body from
            // in-front-of-plane to PAST the whole carve volume — the capture box
            // is never sampled (no Begin) and the body no longer intersects the
            // hole (no rescue), so the carve re-seals and, under the no-pushout
            // rule, the body grounds EMBEDDED with its momentum killed. Solid
            // blocks already sweep; this makes the transit TRIGGER swept too:
            // if the prev→now SEGMENT crossed the entry plane front→behind and
            // the crossing point lies within the aperture, the body physically
            // fell through the opening this frame — transfer it, however deep it
            // ended up. `transfer_step`'s `map_point` glue handles any depth
            // continuously (depth past the entry plane = depth in front of the
            // exit), so a deep crossing emerges correspondingly far along its
            // path — momentum preserved, which is the point ("speedy thing goes
            // in, speedy thing comes out").
            //
            // Two guards keep this honest:
            // * The crossing DIRECTION is the segment's own (front → behind);
            //   the live `vel` gate is deliberately NOT used — the integrator
            //   may already have stopped the body at the carve bottom and
            //   zeroed it, which is exactly the failure being fixed.
            // * The segment must look like ONE frame of ballistic motion:
            //   length ≤ `|prev_vel| * MAX_SWEPT_STEP_S` (+ slack). A respawn /
            //   reset / scripted teleport produces an arbitrary segment that
            //   must never read as travel through an aperture.
            if let Some(step) =
                swept_crossing_step(sweep, center, size, vel, portals, gravity_dir, tuning)
            {
                return step;
            }
            // Begin into the first portal (across ALL pairs) the body is
            // entering. The post-crossing cooldown latch is PAIR-scoped: it
            // only blocks re-Begin into the pair just crossed — entering a
            // different pair immediately is legitimate (chained rooms).
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
                // Begin when the leading face reaches the opening, FROM THE
                // FRONT: the centroid must be on the room side of the plane
                // (a dip of TRANSIT_BEGIN_MARGIN is tolerated — by then a
                // legit entry has already latched). Without the front-side
                // gate, a body pressed against the BACK of a thin host wall
                // could reach the capture box through the material and
                // "enter" a portal it cannot even see.
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
            // The CENTROID crossing the plane is the authoritative transfer —
            // the body jumps to the exit; gameplay sees no discontinuity because
            // every query uses the portal pieces.
            if !t.crossed && pp::front_distance(center, &ef) <= 0.0 {
                return transfer_step(center, vel, enter, exit, gravity_dir, tuning);
            }
            // SWEPT re-crossing while the POST-transfer latch is still clearing
            // (§7.6): on a fast portal loop the flight time between the exit and
            // the next entry can shrink BELOW one frame, so the body swept-crosses
            // the next aperture while `crossed` is still latched and the trailing
            // edge hasn't cleared. Without this arm the machine spends that frame
            // on Clear, the crossing is behind the plane by the time the None arm
            // sees it, and the body embeds. A pre-crossing latch (`!crossed`) is
            // NOT swept: its own centroid sign-test above already fires at any
            // depth.
            if t.crossed {
                if let Some(step) =
                    swept_crossing_step(sweep, center, size, vel, portals, gravity_dir, tuning)
                {
                    return step;
                }
            }
            // Stay engaged so the carve persists long enough to sink + cross —
            // clearing on "not straddling yet" would drop the carve every other
            // frame and the body would never sink in (it re-grounds on the solid
            // frame). Before the centroid crosses, stay while the body still
            // touches the opening (the capture box); after, stay while it still
            // straddles the exit plane (trailing edge not yet out). The cooldown
            // latch (set on transfer) stops a re-entry.
            let still_engaged = if t.crossed {
                pp::straddles(body, &enter.frame())
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
mod tests {
    use super::*;
    use crate::color::{PortalChannel, PortalChannelColor};
    use crate::types::portal_half_extent;

    fn floor(channel: PortalChannel, pos: Vec2) -> PlacedPortal {
        PlacedPortal {
            channel,
            pos,
            normal: Vec2::new(0.0, -1.0),
            half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
        }
    }

    const PURPLE: PortalChannel = PortalChannel::Authored(PortalChannelColor::Purple);
    const YELLOW: PortalChannel = PortalChannel::Authored(PortalChannelColor::Yellow);

    /// A fast fall can cross the whole straddle window between two sampled
    /// frames (1900 px/s terminal ≈ 63 px at the 1/30 s sim-step clamp vs a
    /// ~40 px body), leaving the body FULLY below the entry plane inside the
    /// open carve, with the Begin path cooldown-blocked. The rescue must still
    /// transfer it — the carve volume is the gate, not a same-frame straddle.
    #[test]
    fn rescue_transfers_a_deep_crossing_inside_the_carve_even_on_cooldown() {
        let portals = [
            floor(PURPLE, Vec2::new(100.0, 300.0)),
            floor(YELLOW, Vec2::new(500.0, 300.0)),
        ];
        // Body (24x40) entirely below the plane (top edge y=315 > 300) but
        // within the carve volume, still falling in, mid ping-pong cooldown.
        let step = transit_step(
            Vec2::new(100.0, 335.0),
            Vec2::new(24.0, 40.0),
            Vec2::new(0.0, 1600.0),
            None,
            Some(PURPLE), // cooldown latched — Begin blocked, only the rescue can act
            &portals,
            Vec2::new(0.0, 1.0),
        );
        match step {
            TransitStep::Transfer { pos, .. } => {
                assert!(
                    pos.y < 300.0,
                    "the transfer emerges in FRONT of the exit plane, got {pos:?}"
                );
            }
            other => panic!("a deep carve crossing must transfer, got {other:?}"),
        }
    }

    fn wall_portal(channel: PortalChannel, pos: Vec2, normal: Vec2) -> PlacedPortal {
        PlacedPortal {
            channel,
            pos,
            normal,
            half_extent: portal_half_extent(normal),
        }
    }

    /// Thin-wall geometric guard: with the host wall measured at 24px, the
    /// rescue's aperture volume ends at the wall's far face — a body standing
    /// in the open room BEHIND the wall (well within the unclipped 60px carve
    /// reach) is never teleported, while a genuine deep crossing inside the
    /// material still transfers.
    #[test]
    fn rescue_is_bounded_by_the_measured_host_depth() {
        use crate::types::PortalHostDepths;
        // Left face of a 24px wall spanning x ∈ [500, 524].
        let a = wall_portal(PURPLE, Vec2::new(500.0, 450.0), Vec2::new(-1.0, 0.0));
        let b = wall_portal(YELLOW, Vec2::new(100.0, 450.0), Vec2::new(-1.0, 0.0));
        let portals = [a, b];
        let depths = PortalHostDepths(vec![(PURPLE, 24.0), (YELLOW, 24.0)]);
        // A body in the room BEHIND the wall (centroid 40px past A's plane —
        // inside the UNCLIPPED 60px hole) moving deeper: must stay Idle.
        let step = transit_step_with_tuning(
            Vec2::new(540.0, 450.0),
            Vec2::new(24.0, 40.0),
            Vec2::new(80.0, 0.0), // moving +x = away from A's face = vel·n < 0
            None,
            None,
            None,
            &portals,
            Vec2::new(0.0, 1.0),
            &depths,
            &PortalTuning::default(),
        );
        assert!(
            matches!(step, TransitStep::Idle),
            "a body in the open room behind a thin wall must never be rescued, got {step:?}"
        );
    }

    /// A body pressed against the BACK of a thin host wall must not Begin a
    /// transit into a portal it cannot see — the capture box reaches through
    /// thin material, so Begin gates on the FRONT side of the plane.
    #[test]
    fn begin_requires_the_front_side_of_the_plane() {
        // Portal on the left face of a thin wall; body just BEHIND the face
        // (12px past the plane — within the capture box's through-reach),
        // moving away from the face (vel·n < 0 reads as "entering").
        let a = wall_portal(PURPLE, Vec2::new(500.0, 450.0), Vec2::new(-1.0, 0.0));
        let b = wall_portal(YELLOW, Vec2::new(100.0, 450.0), Vec2::new(-1.0, 0.0));
        let portals = [a, b];
        let step = transit_step(
            Vec2::new(512.0, 450.0),
            Vec2::new(4.0, 4.0), // small so it fits + overlaps the thin box
            Vec2::new(80.0, 0.0),
            None,
            None,
            &portals,
            Vec2::new(0.0, 1.0),
        );
        assert!(
            !matches!(step, TransitStep::Begin { .. }),
            "no Begin from behind the plane, got {step:?}"
        );
    }

    /// The post-crossing cooldown is scoped to the crossed pair: it blocks
    /// re-Begin into that pair but leaves a DIFFERENT pair enterable.
    #[test]
    fn cooldown_is_pair_scoped() {
        use crate::color::PortalChannelColor;
        const TEAL: PortalChannel = PortalChannel::Authored(PortalChannelColor::Teal);
        const RED: PortalChannel = PortalChannel::Authored(PortalChannelColor::Red);
        let portals = [
            floor(PURPLE, Vec2::new(100.0, 300.0)),
            floor(YELLOW, Vec2::new(500.0, 300.0)),
            floor(TEAL, Vec2::new(900.0, 300.0)),
            floor(RED, Vec2::new(1300.0, 300.0)),
        ];
        // Body resting on the TEAL portal, latched against the PURPLE pair.
        let step = transit_step(
            Vec2::new(900.0, 285.0),
            Vec2::new(24.0, 40.0),
            Vec2::new(0.0, 40.0),
            None,
            Some(PURPLE),
            &portals,
            Vec2::new(0.0, 1.0),
        );
        assert!(
            matches!(step, TransitStep::Begin { channel, .. } if channel == TEAL),
            "a different pair must stay enterable during the cooldown, got {step:?}"
        );
        // The latched pair itself (either end) is refused.
        let step = transit_step(
            Vec2::new(500.0, 285.0),
            Vec2::new(24.0, 40.0),
            Vec2::new(0.0, 40.0),
            None,
            Some(PURPLE),
            &portals,
            Vec2::new(0.0, 1.0),
        );
        assert!(
            matches!(step, TransitStep::Idle),
            "the crossed pair stays latched during the cooldown, got {step:?}"
        );
    }

    /// The carve volume bounds the rescue: a body genuinely below the surface
    /// (past the carve depth) is never teleported.
    #[test]
    fn rescue_never_grabs_a_body_past_the_carve_depth() {
        let portals = [
            floor(PURPLE, Vec2::new(100.0, 300.0)),
            floor(YELLOW, Vec2::new(500.0, 300.0)),
        ];
        let step = transit_step(
            Vec2::new(100.0, 420.0), // top edge y=400, past the 60px carve
            Vec2::new(24.0, 40.0),
            Vec2::new(0.0, 400.0),
            None,
            Some(PURPLE),
            &portals,
            Vec2::new(0.0, 1.0),
        );
        assert!(
            matches!(step, TransitStep::Idle),
            "a body below the carve volume must not be rescued, got {step:?}"
        );
    }

    fn ceiling(channel: PortalChannel, pos: Vec2) -> PlacedPortal {
        PlacedPortal {
            channel,
            pos,
            normal: Vec2::new(0.0, 1.0),
            half_extent: portal_half_extent(Vec2::new(0.0, 1.0)),
        }
    }

    /// §7.6 — the swept (CCD) transit tier, on the exact failing configuration:
    /// a floor→ceiling translation pair forming an ACCELERATING fall loop under
    /// a relaxed fall cap. The discrete tiers are sized for ~63 px/frame
    /// (`APPROACH_CARVE_REACH` / `CARVE_DEPTH`); past that, one frame's step
    /// jumps the body clean over the capture box AND the carve volume, no tier
    /// fires, and the body lands embedded in the floor with its momentum
    /// killed. The swept tier must transfer EVERY cycle, up past 800 px/frame,
    /// with the pair cooldown latched exactly as the live system latches it.
    #[test]
    fn swept_tier_transfers_the_accelerating_fall_loop_at_any_speed() {
        let floor_y = 300.0;
        let ceiling_y = floor_y - 680.0;
        let portals = [
            floor(PURPLE, Vec2::new(100.0, floor_y)),
            ceiling(YELLOW, Vec2::new(100.0, ceiling_y)),
        ];
        let size = Vec2::new(24.0, 40.0);
        let dt = 1.0 / 30.0;
        let gravity = 4000.0; // px/s², no fall cap — the loop accelerates forever
        let tuning = PortalTuning::default();
        let depths = crate::types::PortalHostDepths::default();

        let mut pos = Vec2::new(100.0, ceiling_y + 40.0);
        let mut vel = Vec2::new(0.0, 200.0);
        let mut prev = SweptSample { pos, vel };
        let mut transit: Option<PortalTransit> = None;
        let mut cooldown: Option<(PortalChannel, f32)> = None;
        let mut transfers = 0u32;
        let mut peak_step = 0.0f32;

        // 140 frames at g=4000 peaks ~630px/frame — past the ~500px/frame the
        // §7.6 report asked for, under the documented one-crossing-per-step
        // bound (a segment longer than the whole 680px loop can out-run one
        // transfer per frame; that regime is physically off the map).
        for frame in 0..140 {
            let step = transit_step_with_tuning(
                pos,
                size,
                vel,
                Some(prev),
                transit,
                cooldown.map(|(c, _)| c),
                &portals,
                Vec2::new(0.0, 1.0),
                &depths,
                &tuning,
            );
            match step {
                TransitStep::Begin { channel, .. } => {
                    transit = Some(PortalTransit {
                        straddling: channel,
                        crossed: false,
                    });
                }
                TransitStep::Transfer {
                    pos: p,
                    vel: v,
                    exit_channel,
                    ..
                } => {
                    pos = p;
                    vel = v;
                    transfers += 1;
                    cooldown = Some((exit_channel, tuning.teleport_cooldown_s));
                    transit = transit.map(|mut t| {
                        t.crossed = true;
                        t.straddling = exit_channel;
                        t
                    });
                }
                TransitStep::Clear => transit = None,
                TransitStep::Idle | TransitStep::Continue => {}
            }

            // The no-embed invariant: after the machine ran, the body may
            // overshoot the floor plane only within the frame it crossed it —
            // the NEXT machine call must have transferred it back out. A body
            // still below the plane here means every tier missed: embedded.
            assert!(
                pos.y <= floor_y + 1.0,
                "frame {frame}: body ended {}px past the floor plane at \
                 {:.0}px/frame — the transit trigger tunneled",
                pos.y - floor_y,
                vel.y * dt,
            );

            // Anchor + physics (the pure-machine mirror of the live system:
            // record post-step pos/vel, then integrate one ballistic frame).
            prev = SweptSample { pos, vel };
            vel.y += gravity * dt;
            pos.y += vel.y * dt;
            peak_step = peak_step.max(vel.y * dt);
            cooldown = cooldown.and_then(|(c, t)| {
                let t = t - dt;
                (t > 0.0).then_some((c, t))
            });
        }

        assert!(
            peak_step > 500.0,
            "the loop must actually reach tunneling speeds, peaked at {peak_step:.0}px/frame",
        );
        assert!(
            transfers > 40,
            "the loop must keep cycling (one transfer per crossing), got {transfers}",
        );
    }

    /// The swept tier's teleport guard: a prev→now segment far longer than one
    /// frame of the previous velocity's ballistic travel (a respawn / reset /
    /// scripted teleport) must NEVER read as travel through an aperture, even
    /// when the straight line between the two points crosses the portal plane
    /// inside the opening.
    #[test]
    fn swept_tier_ignores_teleport_sized_segments() {
        let portals = [
            floor(PURPLE, Vec2::new(100.0, 300.0)),
            floor(YELLOW, Vec2::new(500.0, 300.0)),
        ];
        // "Respawned" from far above the portal to far below it; the previous
        // velocity was a gentle 100 px/s — the 800px segment is two orders of
        // magnitude past one frame of that motion.
        let step = transit_step_with_tuning(
            Vec2::new(100.0, 700.0),
            Vec2::new(24.0, 40.0),
            Vec2::ZERO,
            Some(SweptSample {
                pos: Vec2::new(100.0, -100.0),
                vel: Vec2::new(0.0, 100.0),
            }),
            None,
            None,
            &portals,
            Vec2::new(0.0, 1.0),
            &crate::types::PortalHostDepths::default(),
            &PortalTuning::default(),
        );
        assert!(
            matches!(step, TransitStep::Idle),
            "a teleport-sized segment must not sweep through a portal, got {step:?}"
        );
    }

    /// The swept tier carries the ENTRY momentum even when the integrator
    /// already stopped the body (grounded at the carve bottom, velocity
    /// zeroed) after it crossed — the exact §7.6 embed: the previous sample
    /// proves the crossing and supplies the velocity the exit must emit.
    #[test]
    fn swept_tier_transfers_a_stopped_body_with_its_entry_momentum() {
        let portals = [
            floor(PURPLE, Vec2::new(100.0, 300.0)),
            ceiling(YELLOW, Vec2::new(100.0, -380.0)),
        ];
        // Last frame: 90px above the plane falling 15000 px/s (500 px/frame).
        // This frame: the integrator stopped it 110px past the plane (beyond
        // the 60px carve — the rescue can't see it) and zeroed its velocity.
        let step = transit_step_with_tuning(
            Vec2::new(100.0, 410.0),
            Vec2::new(24.0, 40.0),
            Vec2::ZERO,
            Some(SweptSample {
                pos: Vec2::new(100.0, 210.0),
                vel: Vec2::new(0.0, 15000.0),
            }),
            None,
            Some(PURPLE), // even mid ping-pong cooldown
            &portals,
            Vec2::new(0.0, 1.0),
            &crate::types::PortalHostDepths::default(),
            &PortalTuning::default(),
        );
        match step {
            TransitStep::Transfer { vel, .. } => {
                assert!(
                    vel.y > 10000.0,
                    "the exit must emit the ENTRY momentum, not the zeroed \
                     post-stop velocity; got {vel:?}"
                );
            }
            other => panic!("a swept crossing must transfer a stopped body, got {other:?}"),
        }
    }
}
