//! Portal-aware geometry and the surface-fit / aperture-crossing decision logic.
//!
//! Plain solid raycasts live in `crate::platformer_runtime::collision`; this
//! module keeps only the portal-specific traversal, the fit check, and the pure
//! `transit_step` decision machine shared by player + actor transit.

use bevy::prelude::*;

use crate::engine_core::{self as ae, AabbExt};
use crate::platformer_runtime::collision::{ray_aabb, raycast_solids};
use crate::platformer_runtime::transit::rotate_velocity_between_normals as portal_transform_velocity;
use crate::portal::pieces as pp;

use super::color::PortalChannel;
use super::transit::PortalTransit;
use super::types::{find_portal, PlacedPortal, MIN_EXIT_SPEED};

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

/// The somersault roll a body picks up crossing a portal pair. It is the
/// on-screen turn from [`portal_transit_roll`] — EXCEPT a pure turn-around in
/// the gravity-perpendicular plane (wall↔wall under normal gravity) imparts NO
/// tumble: the body stays gravity-upright and just reverses facing, so it comes
/// out the far wall already correctly oriented. Crossing a floor / ceiling
/// (normal along gravity) keeps the genuine tumble (feet-in → reorient).
pub fn somersault_roll(n_in: Vec2, n_out: Vec2, gravity_dir: Vec2) -> f32 {
    let g = gravity_dir.normalize_or_zero();
    // A portal whose normal is perpendicular to gravity sits on a wall; the body
    // enters/leaves it moving horizontally, so the transit is a turn-around, not
    // a tumble.
    let in_wall = n_in.normalize_or_zero().dot(g).abs() < 0.5;
    let out_wall = n_out.normalize_or_zero().dot(g).abs() < 0.5;
    if in_wall && out_wall {
        return 0.0;
    }
    portal_transit_roll(n_in, n_out)
}

/// Whether the body's horizontal FACING flips through this portal pair.
///
/// A 180° somersault rotation inherently mirrors the sprite left↔right. For a
/// wall↔wall turn-around we SUPPRESS that rotation (to keep the body upright —
/// see [`somersault_roll`]), which would lose the mirror and emerge the body
/// back-first ("face in, back out"). So in exactly that suppressed-180° case the
/// mirror is re-applied as a facing flip, giving the wanted "face in, face out"
/// (really: X-in, X-out). Every other case carries its orientation in the
/// rotation, so facing is left alone.
pub fn portal_facing_flips(n_in: Vec2, n_out: Vec2, gravity_dir: Vec2) -> bool {
    let g = gravity_dir.normalize_or_zero();
    let in_wall = n_in.normalize_or_zero().dot(g).abs() < 0.5;
    let out_wall = n_out.normalize_or_zero().dot(g).abs() < 0.5;
    // Suppressed (both walls) AND the would-be turn is a ~180° flip (same-wall),
    // not a 0° straight-through (facing-each-other walls).
    in_wall && out_wall && portal_transit_roll(n_in, n_out).abs() > std::f32::consts::FRAC_PI_2
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
        /// Also the gate for the held-input warp: it's exactly the case where
        /// warping held movement stays horizontally expressible.
        facing_flip: bool,
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
    let body = ae::Aabb::new(center, size * 0.5);
    // Resolve `(straddled, its linked exit)` for a color — both must be placed.
    let pair_for = |c: PortalChannel| -> Option<(PlacedPortal, PlacedPortal)> {
        Some((find_portal(portals, c)?, find_portal(portals, c.partner())?))
    };
    match transit {
        None => {
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
                let capture = ae::Aabb::new(
                    enter.pos,
                    enter.half_extent + Vec2::splat(TRANSIT_BEGIN_MARGIN),
                );
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
                let xf = exit.frame();
                let mut vel_out = portal_transform_velocity(vel, enter.normal, exit.normal);
                // Floor the exit speed along the exit normal so a slow walk-in
                // still emerges instead of stalling in the opening.
                if vel_out.dot(exit.normal) < MIN_EXIT_SPEED {
                    let tangential = vel_out - vel_out.dot(exit.normal) * exit.normal;
                    vel_out = tangential + exit.normal * MIN_EXIT_SPEED;
                }
                return TransitStep::Transfer {
                    pos: pp::map_point(center, &ef, &xf),
                    vel: vel_out,
                    // The body picks up the on-screen turn it travels through
                    // (a tumble for floor/ceiling, nothing for a wall↔wall
                    // turn-around); `update_actor_roll` then eases it back to
                    // gravity-upright (feet-in → reorient).
                    roll_delta: somersault_roll(enter.normal, exit.normal, gravity_dir),
                    facing_flip: portal_facing_flips(enter.normal, exit.normal, gravity_dir),
                    enter_normal: enter.normal,
                    exit_normal: exit.normal,
                    exit_channel: exit.channel,
                    exit_pos: exit.pos,
                };
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
                let capture = ae::Aabb::new(
                    enter.pos,
                    enter.half_extent + Vec2::splat(TRANSIT_BEGIN_MARGIN),
                );
                body.strict_intersects(capture)
            };
            if still_engaged {
                TransitStep::Continue
            } else {
                TransitStep::Clear
            }
        }
    }
}
