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
#[derive(Resource, Default)]
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
        if d >= 0.0 {
            let push = half_n - d + EVICT_MARGIN;
            if push > 0.0 {
                kin.pos += n * push;
            }
        } else {
            let push = half_n + d + EVICT_MARGIN;
            if push > 0.0 {
                kin.pos -= n * push;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{PortalChannelColor, PortalGunColor};
    use crate::transit::PortalPolicy;
    use crate::types::portal_half_extent;

    fn floor_portal(channel: PortalChannel, pos: Vec2) -> PlacedPortal {
        PlacedPortal::fixed(
            channel,
            pos,
            Vec2::new(0.0, -1.0),
            portal_half_extent(Vec2::new(0.0, -1.0)),
        )
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<PortalFrameHistory>();
        app.add_systems(Update, evict_straddlers_on_portal_change);
        app
    }

    fn straddling_body(app: &mut App, pos: Vec2) -> Entity {
        app.world_mut()
            .spawn((
                BodyKinematics {
                    pos,
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                PortalBody,
                PortalPolicy {
                    reorient: true,
                    carry_velocity: true,
                },
            ))
            .id()
    }

    const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
    const PURPLE: PortalChannel = PortalChannel::Authored(PortalChannelColor::Purple);

    #[test]
    fn vanished_portal_evicts_a_straddler_to_its_centroids_side() {
        let mut app = app();
        // Floor portal at y=300; body centered just ABOVE the plane (centroid
        // front, feet dipped through).
        let portal = app
            .world_mut()
            .spawn(floor_portal(BLUE, Vec2::new(100.0, 300.0)))
            .id();
        let body = straddling_body(&mut app, Vec2::new(100.0, 290.0));
        // Frame 1: history records the portal; body still straddles.
        app.update();
        assert!(pp::straddles(
            ae::Aabb::new(Vec2::new(100.0, 290.0), Vec2::new(12.0, 20.0)),
            &floor_portal(BLUE, Vec2::new(100.0, 300.0)).aperture()
        ));
        // Portal vanishes; frame 2 evicts the straddler upward (centroid front).
        app.world_mut().entity_mut(portal).despawn();
        app.update();
        let kin = app.world().get::<BodyKinematics>(body).unwrap();
        // Pushed UP (—y) fully clear of the old plane: bottom edge above y=300.
        assert!(
            kin.pos.y + kin.size.y * 0.5 <= 300.0 + 1e-3,
            "evicted clear: {:?}",
            kin.pos
        );
    }

    #[test]
    fn teleported_portal_evicts_straddler_but_stable_portal_does_not() {
        let mut app = app();
        let portal = app
            .world_mut()
            .spawn(floor_portal(BLUE, Vec2::new(100.0, 300.0)))
            .id();
        let body = straddling_body(&mut app, Vec2::new(100.0, 290.0));
        // A different, stable channel a SECOND body straddles — must be left alone.
        app.world_mut()
            .spawn(floor_portal(PURPLE, Vec2::new(500.0, 300.0)));
        let stable_body = straddling_body(&mut app, Vec2::new(500.0, 290.0));
        app.update();
        // Teleport BLUE far away; PURPLE unchanged.
        app.world_mut()
            .entity_mut(portal)
            .insert(floor_portal(BLUE, Vec2::new(900.0, 300.0)));
        app.update();
        let moved = app.world().get::<BodyKinematics>(body).unwrap().pos;
        let stable = app.world().get::<BodyKinematics>(stable_body).unwrap().pos;
        assert!(
            moved.y < 290.0 - 1.0,
            "BLUE straddler evicted up: {moved:?}"
        );
        assert!(
            (stable.y - 290.0).abs() < 1e-3,
            "PURPLE straddler untouched: {stable:?}"
        );
    }

    /// CC6: a HOSTED aperture riding its face is the same portal in motion,
    /// not a close — a straddling body must NOT be evicted (the dynamic
    /// straddle re-evaluates; eviction stays a CLOSE-only pushout). A
    /// teleport of the same channel still evicts (covered above).
    #[test]
    fn host_carried_motion_does_not_evict_a_straddler() {
        let mut app = app();
        let portal = app
            .world_mut()
            .spawn(floor_portal(BLUE, Vec2::new(100.0, 300.0)))
            .id();
        let body = straddling_body(&mut app, Vec2::new(100.0, 290.0));
        app.update(); // history primes

        // The host refresh carried the aperture up 8px this frame.
        {
            let mut p = app.world_mut().get_mut::<PlacedPortal>(portal).unwrap();
            p.host = Some(ambition_engine_core::GeoFaceRef::new(
                ambition_engine_core::GeoId::anon(),
                ambition_engine_core::Face::Top,
                0.0,
            ));
            p.prev_pos = p.pos;
            p.pos += Vec2::new(0.0, -8.0);
        }
        let before = app.world().get::<BodyKinematics>(body).unwrap().pos;
        app.update();
        let after = app.world().get::<BodyKinematics>(body).unwrap().pos;
        assert_eq!(before, after, "host-carried motion is not a close");
    }
}
