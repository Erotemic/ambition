// Portal integration test: only built with the portal mechanic + RL stepping
// API. Compiled out (empty test binary) when `portal` is disabled.
#![cfg(all(feature = "portal", feature = "rl_sim"))]
//! Stage 19 Phase 4 — the projectile-transit DEMO, proven against the REAL app
//! schedule.
//!
//! A projectile is a free-flying body: it carries the shared `BodyKinematics`
//! and is opted into the ONE generic `portal_transit` core with a free-flying
//! `PortalPolicy { reorient: false, carry_velocity: true }`. The unit suite in
//! `ambition_content::portal::transit_body_adapter` proves the Phase-4 tagging
//! adapter (`ensure_projectile_portal_bodies`) + the core transit in isolation;
//! THIS test proves the same thing end-to-end inside a live `SandboxSim` app —
//! the real `PortalSet::Transit` schedule processes a projectile-shaped body and
//! transits it through a placed pair.
//!
//! Setup: read a live authored portal pair, spawn a projectile-shaped body
//! (small `BodyKinematics`, no `ActorRoll`, the free-flying `PortalPolicy`)
//! flying into the entry portal, then step the real app and assert the body
//! emerges on the far side travelling through the exit (keeps flying — not
//! stopped, not re-oriented).
//!
//! The body is spawned already carrying `PortalBody` + `PortalPolicy` (both
//! public) so the test needs no `pub(crate)` projectile types; the projectile
//! *tagging* path is covered by the in-crate unit suite. What this guards is the
//! real-app wiring: `portal_transit` runs in the live schedule against a
//! free-flying body and carries it through the pair.

mod common;
use common::{base, first_authored_portal_pair, fixed_60hz_room_sim};

use ambition_sandbox::platformer_runtime::body::BodyKinematics;
use ambition_sandbox::portal::{PortalBody, PortalPolicy};
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
