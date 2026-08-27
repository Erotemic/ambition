// Portal integration test: only built with the portal mechanic + RL stepping API.
#![cfg(all(feature = "portal", feature = "rl_sim"))]
//! Verifies free-flying portal transit in the real app schedule.
//!
//! A projectile-shaped body carries shared `BodyKinematics`, `PortalBody`, and a
//! non-reorienting velocity-preserving `PortalPolicy`. The test uses a live
//! authored portal pair and the normal `PortalSet::Transit` schedule; projectile
//! tagging itself is covered by the in-crate adapter tests.

use crate::common::{base, first_authored_portal_pair, fixed_60hz_room_sim};

use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::portal::{PortalBody, PortalPolicy};
use bevy::prelude::*;

#[test]
fn a_free_flying_projectile_body_transits_a_portal_pair_in_the_real_app() {
    let mut sim = fixed_60hz_room_sim("portal_lab");

    // Step once so the world + schedule are fully initialized.
    sim.step(base());

    // Read a live authored pair after link resolution. Link-authored portals
    // get generated channels in the app, so this test should not depend on a
    // particular authoring color surviving as the runtime channel.
    let (entry_pos, entry_normal, exit_pos) = {
        let (entry, exit) = first_authored_portal_pair(&mut sim);
        (entry.pos, entry.normal, exit.pos)
    };

    // A free-flying projectile-shaped body straddling the entry portal, moving
    // INTO its face (against the outward normal) at 400 px/s — well above
    // MIN_EXIT_SPEED so the rotation is pure. No projectile-step integration is
    // needed because it already sits in the opening.
    let proj = sim
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: entry_pos,
                vel: -entry_normal * 400.0,
                size: Vec2::new(8.0, 8.0),
                facing: 1.0,
            },
            PortalBody,
            PortalPolicy {
                reorient: false,
                carry_velocity: true,
            },
            Name::new("test projectile body"),
        ))
        .id();

    // Distance from the entry portal at spawn — the body must end up FAR from
    // here (transited) and NEAR the exit portal.
    let entry_to_exit = entry_pos.distance(exit_pos);

    // Step the real app a few frames: the live `PortalSet::Transit` schedule runs
    // `portal_transit`. The body is spawned straddling the entry portal (centroid
    // on its plane, moving in), so the aperture machine begins immediately and
    // transfers within a couple of transit frames — no projectile-step
    // integration needed (the body sits in the opening from the start).
    let mut emerged = false;
    for _ in 0..8 {
        sim.step(base());
        let kin = *sim
            .world()
            .get::<BodyKinematics>(proj)
            .expect("the projectile body entity is still alive");
        // Emerged when it has jumped close to the EXIT portal and far from entry.
        if kin.pos.distance(exit_pos) < entry_to_exit * 0.5
            && kin.pos.distance(entry_pos) > entry_to_exit * 0.5
        {
            // Travels ALONG the exit normal: keeps flying out, rotated velocity.
            // The exit portal's outward normal is the emergence direction.
            assert!(
                kin.vel.length() > 100.0,
                "the projectile keeps flying out of the exit portal, vel={:?}",
                kin.vel,
            );
            assert_eq!(
                kin.facing, 1.0,
                "a free-flying projectile is not re-oriented by transit \
                 (reorient:false), facing={}",
                kin.facing,
            );
            emerged = true;
            break;
        }
    }

    assert!(
        emerged,
        "the free-flying body should have transited the authored portal pair and \
         emerged on the far side in the real app schedule",
    );
}
