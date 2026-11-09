// Portal integration test: only built with the portal mechanic + RL stepping API.
#![cfg(all(feature = "portal", feature = "rl_sim"))]
//! Verifies free-flying portal transit in the real app schedule.
//!
//! A projectile-shaped body carries only the shared `BodyKinematics`: the portal
//! core drives every body. The test uses a live authored portal pair and the
//! normal `PortalSet::Transit` schedule.

use crate::common::{base, first_authored_portal_pair, fixed_60hz_room_sim};

use ambition_platformer2d::engine_core::BodyKinematics;
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
            BodyKinematics { material: Default::default(),
                pos: entry_pos,
                vel: -entry_normal * 400.0,
                canonical_size: Vec2::new(8.0, 8.0),
                facing: 1.0,
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

#[test]
fn a_projectile_refuses_a_portal_cut_before_motion_and_resumes_when_admitted() {
    use ambition_platformer2d::engine_core::frame::thread::PieceBudget;
    use ambition_platformer2d::platformer::portal_passages::PortalPieceBudget;
    use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
    use ambition_platformer2d::projectiles::{ProjectileOwner, ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart, ProjectileVisualId};
    let mut sim = fixed_60hz_room_sim("portal_lab");
    sim.step(base());
    let owner = {
        let world = sim.world_mut();
        world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap()
    };
    let (enter, exit) = crate::common::first_floor_authored_portal_pair(&mut sim);
    sim.world_mut().write_message(ProjectileSpawnRequest::open(owner, ProjectileSpawn {
        origin: enter.pos + enter.normal * 80.0, dir: -enter.normal, speed: 1200.0,
        damage: 1, max_lifetime: 3.0, half_extent: Vec2::splat(5.0), gravity: 0.0,
        visual_id: "seam_gauge".into(), bounces: 0, bounce_on_world_contact: false,
        splash_half_extent: 0.0, boomerang_return_s: None,
    }, ProjectileStart::StepNextTick));
    sim.step(base());
    let shot = {
        let world = sim.world_mut();
        world.query::<(Entity, &ProjectileOwner, &ProjectileVisualId)>().iter(world)
            .find(|(_, by, visual)| by.0 == owner && visual.0 == "seam_gauge")
            .map(|(entity, _, _)| entity).expect("the production materializer creates the shot")
    };
    sim.world_mut().entity_mut(shot).insert(PortalPieceBudget(PieceBudget {
        max_hops: 0, max_pieces: 1, max_scale_exponent: 12,
    }));
    let initial = sim.world().get::<BodyKinematics>(shot).unwrap().pos;
    let mut previous = initial;
    let mut refused_ticks = 0;
    let mut previous_age = sim.world().get::<ambition_platformer2d::projectiles::ProjectileGameplay>(shot).unwrap().age;
    for _ in 0..12 {
        sim.step(base());
        let body = sim.world().get::<BodyKinematics>(shot).expect("refusal keeps the projectile");
        let age = sim.world().get::<ambition_platformer2d::projectiles::ProjectileGameplay>(shot).unwrap().age;
        assert!(age > previous_age, "refused motion must not make a projectile live forever");
        previous_age = age;
        assert!((body.pos - enter.pos).dot(enter.normal) >= 5.0,
            "the source body crossed a cut it cannot represent");
        if body.pos == previous {
            refused_ticks += 1;
            let sweep = sim.world().get::<ambition_platformer2d::engine_core::SweepSample>(shot).unwrap();
            assert_eq!(sweep.delta(), Vec2::ZERO, "refusal must retire the prior travel segment");
        }
        previous = body.pos;
    }
    assert!(initial.distance(previous) > 20.0, "the shot must actually approach the aperture");
    assert!(refused_ticks >= 3, "the shot must reach and remain at the refused boundary");
    sim.world_mut().entity_mut(shot).remove::<PortalPieceBudget>();
    let mut arrived = false;
    let mut history = Vec::new();
    for _ in 0..12 {
        sim.step(base());
        history.push(sim.world().get::<BodyKinematics>(shot).copied());
        if sim.world().get::<BodyKinematics>(shot).is_some_and(|body| body.pos.distance(exit.pos) < 70.0) {
            arrived = true;
            break;
        }
    }
    assert!(arrived, "the same shot must cross when the cut is admitted; entry={:?}, exit={:?}, poses={history:?}", enter.pos, exit.pos);
}
