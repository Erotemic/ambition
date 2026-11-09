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

#[test]
fn a_projectile_hits_far_side_geometry_before_its_root_transits() {
    far_side_contact(ContactMotion::Fixed);
}

#[test]
fn a_projectile_meets_a_wall_that_crosses_its_far_part_between_endpoint_poses() {
    far_side_contact(ContactMotion::Wall);
}

#[test]
fn a_hosted_exit_carries_material_into_the_lab_target_during_the_tick() {
    far_side_contact(ContactMotion::Exit);
}

enum ContactMotion { Fixed, Wall, Exit }

fn far_side_contact(motion:ContactMotion) {
    use ambition_platformer2d::engine_core::{Aabb,RoomGeometry};
    use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
    use ambition_platformer2d::portal::{link_hash, PlacedPortal, PortalBodyTransited, PortalHost, PortalLink};
    use ambition_platformer2d::projectiles::{ProjectileOwner, ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart, ProjectileVisualId};
    use ambition_platformer2d::vfx::{VfxInRoom, VfxMessage};
    use ambition_platformer2d::world::{collision::MovingPlatformSet, platforms::MovingPlatformState};

    let mut sim = fixed_60hz_room_sim("portal_lab");
    sim.step(base());
    let owner = {
        let world = sim.world_mut();
        world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap()
    };
    let (enter, exit) = {
        let world = sim.world_mut();
        let mut pair: Vec<_> = world.query::<(&PlacedPortal, &PortalLink)>().iter(world)
            .filter(|(_, link)| link.0 == link_hash("portal_lab_k_contact"))
            .map(|(portal, _)| portal.clone()).collect();
        assert_eq!(pair.len(), 2, "station K must load both linked endpoints");
        pair.sort_by(|a, b| a.pos.x.total_cmp(&b.pos.x));
        (pair.remove(0), pair.remove(0))
    };
    let tangent = Vec2::new(-exit.normal.y, exit.normal.x);
    sim.world_mut().write_message(ProjectileSpawnRequest::open(owner, ProjectileSpawn {
        origin: enter.pos + enter.normal * 40.0,
        dir: -enter.normal,
        speed: 600.0,
        damage: 1,
        max_lifetime: 2.0,
        half_extent: Vec2::splat(5.0),
        gravity: 0.0,
        visual_id: "far_contact_gauge".into(),
        bounces: 0,
        bounce_on_world_contact: false,
        splash_half_extent: 0.0,
        boomerang_return_s: None,
    }, ProjectileStart::StepNextTick));
    sim.step(base());
    let shot = {
        let world = sim.world_mut();
        world.query::<(Entity, &ProjectileOwner, &ProjectileVisualId)>().iter(world)
            .find(|(_, by, visual)| by.0 == owner && visual.0 == "far_contact_gauge")
            .map(|(entity, _, _)| entity).expect("the production materializer creates the shot")
    };
    let mut transited = false;
    let mut impact = false;
    let mut expired = false;
    let mut history = Vec::new();
    let mut moved_target=false;
    for _ in 0..12 {
        if !matches!(motion,ContactMotion::Fixed) && !moved_target && sim.world().get::<BodyKinematics>(shot)
            .is_some_and(|body|(body.pos-enter.pos).dot(enter.normal)<=10.01) {
            let world=sim.world_mut();
            if matches!(motion,ContactMotion::Wall) {
                let mut room=world.query::<&mut RoomGeometry>().single_mut(world).unwrap();
                let target=room.0.blocks.to_mut().iter_mut().find(|block|block.name=="K far-side target").unwrap();
                target.aabb=Aabb::new(exit.pos+exit.normal*6.0+tangent*20.0,Vec2::splat(4.0));
                target.velocity=tangent*100.0;
            } else {
                let platform=MovingPlatformState::from_sweep("k_exit_motion", "K exit motion",
                    exit.pos-exit.normal*10.0, Vec2::new(20.0,80.0), 400.0, 1200.0);
                let surfaces=ambition_platformer2d::engine_core::World::new("host setup",
                    Vec2::splat(2000.0),Vec2::ZERO,vec![platform.as_collision_block()]);
                let face=surfaces.attribute_face(exit.pos,exit.normal,0.1).expect("the exit lies on its host face");
                ambition_platformer2d::session::sole_live_room_component_mut::<MovingPlatformSet>(world)
                    .unwrap().0.push(platform);
                let mut portals=world.query::<&mut PlacedPortal>();
                let mut portal=portals.iter_mut(world).find(|p|p.channel==exit.channel).unwrap();
                portal.host=PortalHost::Face { face,lift:0.0 };
            }
            moved_target=true;
        }
        sim.step(base());
        history.push(sim.world().get::<BodyKinematics>(shot).copied());
        transited |= sim.world().resource::<Messages<PortalBodyTransited>>()
            .iter_current_update_messages().any(|event| event.body == shot);
        impact |= sim.world().resource::<Messages<VfxInRoom>>()
            .iter_current_update_messages().any(|event| match event.vfx {
                VfxMessage::Impact { pos, .. } => {
                    let offset = pos-exit.pos;
                    match motion {
                        ContactMotion::Wall=>(2.0..=2.2).contains(&offset.dot(exit.normal)) && (offset.dot(tangent)+5.0).abs()<0.01,
                        ContactMotion::Fixed=>(offset.dot(exit.normal)-2.0).abs() < 0.01 && offset.dot(tangent).abs() <= 5.01,
                        ContactMotion::Exit=>(offset.dot(exit.normal)-10.0).abs() < 0.05 && offset.dot(tangent).abs()<=5.01,
                    }
                }
                _ => false,
            });
        if sim.world().get::<BodyKinematics>(shot).is_none() {
            expired = true;
            break;
        }
    }
    assert!(expired, "the far-side wall must stop the shot; enter={enter:?}, exit={exit:?}, poses={history:?}");
    assert!(!transited, "the material must meet the far wall before its root crosses");
    assert!(impact, "the impact must be at the physical far-side contact");
}
