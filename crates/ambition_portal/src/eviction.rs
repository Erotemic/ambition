//! Straddle eviction — the ONE sanctioned pushout.
//!
//! **JON'S RULE: AVOID PUSHOUT.** A transited body emerges right at the exit
//! face ([`map_point`](crate::pieces::map_point)) and lets velocity carry it
//! clear — it is never artificially shoved out of geometry. Pushout corrupts
//! position/reversibility and papers over real bugs.
//!
//! The lone exception is here: a portal **moves, closes, or teleports while a
//! body straddles its plane** (re-firing the gun to reposition a portal out
//! from under a straddler; a room reset clearing portals; the partner of a
//! transiting body vanishing). Physically the body's two halves are in two
//! different places, and the closing aperture would **rip the body in half**.
//! We model a world-force that instead shoves the straddling body fully to the
//! side its centroid is on, so it lands intact in open space rather than
//! embedded in the now-solid wall. This is the only displacement the portal
//! mechanic performs.
//!
//! ## Disabling this (the rip-in-half mechanic)
//! A game that WANTS lethal portal-close — severing or killing anything caught
//! straddling a vanishing portal — simply does NOT register
//! [`evict_straddlers_on_portal_change`], and instead reacts to the same event
//! (a body straddling a portal that just changed) by killing/splitting it.
//! The detection (frame history → straddle test) is the reusable half; the
//! *response* (evict vs. rip) is the game's choice. Ambition currently evicts;
//! the rip may become a real mechanic later.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use ambition_engine_core as ae;

use ambition_platformer_primitives::body::BodyKinematics;

use crate::color::PortalChannel;
use crate::pieces::{self as pp, PortalAperture};
use crate::transit::PortalBody;
use crate::types::PlacedPortal;

/// Last frame's placed-portal frame per channel, so
/// [`evict_straddlers_on_portal_change`] can detect a portal that MOVED or
/// VANISHED under a straddling body. Crate-owned; the [`PortalPlugin`](crate::PortalPlugin)
/// initialises it.
#[derive(Resource, Default, Clone)]
pub struct PortalFrameHistory(HashMap<PortalChannel, PortalAperture>);

/// Small clearance past the closing plane so the evicted body is unambiguously
/// on one side (not resting exactly on it).
const EVICT_MARGIN: f32 = 1.0;

/// Detect portals that moved / vanished since last frame and shove any body
/// straddling the OLD plane fully to its centroid's side (the sanctioned
/// pushout — see the module docs; the alternative is to rip the body in half).
pub fn evict_straddlers_on_portal_change(
    mut history: ResMut<PortalFrameHistory>,
    portals: Query<&PlacedPortal>,
    mut bodies: Query<&mut BodyKinematics, With<PortalBody>>,
) {
    // A HOSTED aperture riding its face (CC6) is the same portal in motion,
    // not a close: compare against where host-carried motion says it should
    // be. Unhosted portals have zero frame_delta, so this is byte-identical
    // to the pre-CC6 rule for them. A refire/teleport still evicts — its
    // displacement never matches the host delta.
    let current: HashMap<PortalChannel, (PortalAperture, Vec2)> = portals
        .iter()
        .map(|p| (p.channel, (p.aperture(), p.frame_delta())))
        .collect();

    for (channel, old) in history.0.iter() {
        // The plane is unchanged only if a portal of the same channel still
        // sits at the same pos + normal (host-carried motion included);
        // otherwise its old plane is closing.
        let unchanged = current.get(channel).is_some_and(|(now, delta)| {
            now.frame.origin.distance(old.frame.origin + *delta) < 1.0
                && now.frame.normal == old.frame.normal
        });
        if unchanged {
            continue;
        }
        evict_for_plane(*old, &mut bodies);
    }

    history.0 = current.into_iter().map(|(c, (ap, _))| (c, ap)).collect();
}

/// Shove every [`PortalBody`] straddling `plane` to the side its centroid is
/// on, just past the plane.
fn evict_for_plane(
    plane: PortalAperture,
    bodies: &mut Query<&mut BodyKinematics, With<PortalBody>>,
) {
    let n = plane.frame.normal;
    for mut kin in bodies.iter_mut() {
        let body = ae::Aabb::new(kin.pos, kin.size * 0.5);
        if !pp::straddles(body, &plane) {
            continue;
        }
        // Signed centroid distance (+ in front), and the body's half-extent
        // along the normal: push so the trailing edge clears the plane on the
        // centroid's side.
        let d = pp::front_distance(kin.pos, &plane.frame);
        let half_n = (kin.size * 0.5).dot(n.abs());
        // Jon's ONE pushout exception, expressed as the external-constraint
        // authority (ADR 0024): the closing portal carries the straddling body
        // clear of the plane.
        if d >= 0.0 {
            let push = half_n - d + EVICT_MARGIN;
            if push > 0.0 {
                ambition_engine_core::movement::carry_body(&mut kin, n * push);
            }
        } else {
            let push = half_n + d + EVICT_MARGIN;
            if push > 0.0 {
                ambition_engine_core::movement::carry_body(&mut kin, -n * push);
            }
        }
    }
}

#[cfg(test)]
mod tests;
