//! Portal-aware geometry and the surface-fit / aperture-crossing decision logic.
//!
//! Plain solid raycasts live in `ambition_platformer_runtime::world_query`; this
//! module keeps only the portal-specific traversal, the fit check, and the pure
//! `transit_step` decision machine shared by player + actor transit.

use bevy::prelude::*;

use crate::pieces as pp;
use ambition_engine_core::{self as ae, AabbExt};
use ambition_platformer_runtime::transit::rotate_velocity_between_normals as portal_transform_velocity;
use ambition_platformer_runtime::world_query::{ray_aabb, raycast_solids};

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
const TRANSIT_BEGIN_MARGIN: f32 = 6.0;

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
/// grounds the body, killing its momentum). Budget: the player clamps its sim
/// step to 1/30 s (`movement::update`), so 950 px/s terminal fall ⇒ ~32px/frame;
/// projectiles do NOT clamp, so a ~700 px/s shot on a 100ms hitch ⇒ ~70px.
/// 96px covers both with slack. Opening a few frames early is harmless: the
/// approach carve is gated on the body MOVING INTO the portal, and a hole only
/// ever opens where a placed, paired portal already is.
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
/// pair, it returns the action the caller applies. Shared by the player and
/// every non-player actor so they all cross a portal identically (the
/// unification the design calls for).
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
    /// player layer warps the held movement input by it so the held direction
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

/// Compute the transit step for a body. See [`TransitStep`]. `cooldown` is the
/// body's post-jump latch (player gun cooldown / actor [`super::types::PortalTransitCooldown`]);
/// `gravity_dir` selects whether a transit tumbles or just turns around.
pub fn transit_step(
    center: Vec2,
    size: Vec2,
    vel: Vec2,
    transit: Option<PortalTransit>,
    cooldown: f32,
    portals: &[PlacedPortal],
    gravity_dir: Vec2,
) -> TransitStep {
    transit_step_with_tuning(
        center,
        size,
        vel,
        transit,
        cooldown,
        portals,
        gravity_dir,
        &PortalTuning::default(),
    )
}

/// Compute the transit step with editable portal tuning.
pub fn transit_step_with_tuning(
    center: Vec2,
    size: Vec2,
    vel: Vec2,
    transit: Option<PortalTransit>,
    cooldown: f32,
    portals: &[PlacedPortal],
    gravity_dir: Vec2,
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
            // `straddles` bounds the rescue to the opening (the plane passes
            // THROUGH the body), so a body that is legitimately below the surface
            // is never teleported. The body must also be moving INTO the portal
            // (`vel · normal < 0`): that distinguishes a body falling THROUGH the
            // opening (rescue it) from one that JUST EMERGED from this portal and
            // is moving back out (do NOT re-grab it — the transfer maps the
            // centroid right onto the exit plane, so without the velocity gate the
            // rescue would immediately fire again and ping-pong).
            for enter in portals {
                if find_portal(portals, enter.channel.partner()).is_none() {
                    continue;
                }
                if !portal_fits(size, enter) {
                    continue;
                }
                let ef = enter.frame();
                if pp::straddles(body, &ef)
                    && pp::front_distance(center, &ef) <= 0.0
                    && vel.dot(enter.normal) < 0.0
                {
                    let exit = find_portal(portals, enter.channel.partner())
                        .expect("partner checked above");
                    return transfer_step(center, vel, *enter, exit, gravity_dir, tuning);
                }
            }
            if cooldown > 0.0 {
                return TransitStep::Idle;
            }
            // Begin into the first portal (across ALL pairs) the body is entering.
            for enter in portals {
                // Need the partner placed, or there's no exit to transit to.
                if find_portal(portals, enter.channel.partner()).is_none() {
                    continue;
                }
                if !portal_fits(size, enter) {
                    continue;
                }
                let frame = enter.frame();
                // Begin when the leading face reaches the opening, from the front
                // (centroid in front of the plane, or moving into it). The
                // capture box is the thin face plus a small margin; its
                // along-surface span is the opening, so this also gates laterally.
                let capture = capture_box(enter);
                let entering =
                    pp::front_distance(center, &frame) > 0.0 || vel.dot(enter.normal) < 0.0;
                if entering && body.strict_intersects(capture) {
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
