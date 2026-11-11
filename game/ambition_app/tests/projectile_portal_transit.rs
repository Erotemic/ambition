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
fn a_projectile_retains_its_pose_when_a_recipients_partition_is_refused() {
    use ambition_platformer2d::engine_core as ae;
    use ambition_platformer2d::characters::actor::ActorFaction;
    use ambition_platformer2d::combat::components::DamageableVolumes;
    use ambition_platformer2d::platformer::{body::geometry::MaterialBudget, lifecycle::InRoomInstance,
        markers::PrimaryPlayerOnly};
    use ambition_platformer2d::projectiles::{ProjectileGameplay, ProjectileSpawn, ProjectileSpawnRequest,
        ProjectileStart, ProjectileVisualId};
    let mut sim = fixed_60hz_room_sim("portal_lab");
    sim.step(base());
    let (owner, room) = {
        let world = sim.world_mut();
        let entity = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
        (entity, *world.get::<InRoomInstance>(entity).unwrap())
    };
    let center = Vec2::new(1000.0, 400.0);
    let mut hurt = DamageableVolumes::default();
    hurt.publish_body(vec![ae::CombatVolume::aabb(ae::Aabb::new(Vec2::ZERO, Vec2::splat(10.0)))]);
    let target = sim.world_mut().spawn((ActorFaction::Enemy, room,
        ambition_platformer2d::observation::SimId::placement("recipient_refusal_target"),
        ae::BodyKinematics { pos: center, canonical_size: Vec2::splat(20.0), ..Default::default() },
        ae::CenteredAabb::new(center, Vec2::splat(10.0)), hurt,
        MaterialBudget(ae::frame::thread::PieceBudget { max_pieces: 0,
            ..ae::frame::thread::PieceBudget::SINGLE_HOP }))).id();
    let remote = Vec2::new(100_000.0, -100_000.0);
    let remote_target = sim.world_mut().spawn((ActorFaction::Enemy, room,
        ambition_platformer2d::observation::SimId::placement("remote_recipient_refusal_target"),
        ae::BodyKinematics { pos: remote, canonical_size: Vec2::splat(20.0), ..Default::default() },
        ae::CenteredAabb::new(remote, Vec2::splat(10.0)), DamageableVolumes::body_box(Vec2::splat(10.0)),
        MaterialBudget(ae::frame::thread::PieceBudget { max_pieces: 0,
            ..ae::frame::thread::PieceBudget::SINGLE_HOP }))).id();
    let ally=sim.world_mut().spawn((ActorFaction::Player,room,
        ambition_platformer2d::observation::SimId::placement("ineligible_recipient_refusal_target"),
        ae::BodyKinematics {pos:center,canonical_size:Vec2::splat(20.0),..Default::default()},
        ae::CenteredAabb::new(center,Vec2::splat(10.0)),DamageableVolumes::body_box(Vec2::splat(10.0)),
        MaterialBudget(ae::frame::thread::PieceBudget {max_pieces:0,..ae::frame::thread::PieceBudget::SINGLE_HOP}))).id();
    sim.world_mut().write_message(ProjectileSpawnRequest::open(owner, ProjectileSpawn {
        origin: center - Vec2::X * 20.0, dir: Vec2::X, speed: 1200.0,
        damage: 1, max_lifetime: 3.0, half_extent: Vec2::splat(2.0), gravity: 0.0,
        visual_id: "recipient_refusal_gauge".into(), bounces: 0, bounce_on_world_contact: false,
        splash_half_extent: 0.0, boomerang_return_s: None,
    }, ProjectileStart::StepNextTick));
    sim.step(base());
    let shot = {
        let world = sim.world_mut();
        world.query::<(Entity, &ProjectileVisualId)>().iter(world)
            .find(|(_, visual)| visual.0 == "recipient_refusal_gauge").unwrap().0
    };
    let before = *sim.world().get::<BodyKinematics>(shot).unwrap();
    let age = sim.world().get::<ProjectileGameplay>(shot).unwrap().age;
    let mut hits = bevy::ecs::message::MessageCursor::<ambition_platformer2d::combat::HitEvent>::default();
    for _ in 0..3 {
        sim.step(base());
        assert_eq!(*sim.world().get::<BodyKinematics>(shot).unwrap(), before);
        assert_eq!(sim.world().get::<ae::SweepSample>(shot).unwrap().delta(), Vec2::ZERO);
        assert!(hits.read(sim.world().resource::<Messages<ambition_platformer2d::combat::HitEvent>>()).next().is_none());
    }
    assert!(sim.world().get::<ProjectileGameplay>(shot).unwrap().age > age);
    sim.world_mut().entity_mut(target).remove::<MaterialBudget>();
    sim.step(base());
    assert!(hits.read(sim.world().resource::<Messages<ambition_platformer2d::combat::HitEvent>>())
        .any(|hit| hit.target == ambition_platformer2d::combat::HitTarget::Body(target)),
        "an admitted contact must resume: {:?}", sim.world().get::<BodyKinematics>(shot));
    assert!(sim.world().get::<MaterialBudget>(remote_target).is_some());
    assert!(sim.world().get::<MaterialBudget>(ally).is_some());
}

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
    // INTO its face at 400 px/s. Its recorded step reaches the entry plane.
    let proj = sim
        .world_mut()
        .spawn((
            BodyKinematics { material: Default::default(),
                pos: entry_pos,
                vel: -entry_normal * 400.0,
                canonical_size: Vec2::new(8.0, 8.0),
                facing: 1.0,
            },
            ambition_platformer2d::engine_core::SweepSample { turn: None, material: Default::default(), travel: Default::default(),
                prev: entry_pos + entry_normal * 4.0, curr: entry_pos,
                vel: -entry_normal * 400.0, half: Vec2::splat(4.0), down: Vec2::ZERO,
                time: [0.0, 1.0],
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
    use ambition_platformer2d::platformer::body::geometry::MaterialBudget;
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
    sim.world_mut().entity_mut(shot).insert(MaterialBudget(PieceBudget {
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
    sim.world_mut().entity_mut(shot).remove::<MaterialBudget>();
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
fn a_projectile_bounces_on_its_far_material_and_retires_the_contact() {
    for reverse in [false,true] {far_side_contact(ContactMotion::Bounce(reverse));}
}

#[test]
fn a_projectile_damages_a_body_with_its_scaled_tip_before_root_transit() {
    for reverse in [false,true] {
        for launch in [false,true] {scaled_tip_contact(reverse,TipResponse::Damage {launch});}
    }
}

#[test]
fn a_portal_carried_tip_is_parried_at_its_physical_contact_before_root_transit() {
    for reverse in [false,true] {
        for absorb in [false,true] {scaled_tip_contact(reverse,TipResponse::Parry {absorb});}
    }
}

#[derive(Clone,Copy)]
enum TipResponse { Damage {launch:bool}, Parry {absorb:bool} }

fn scaled_tip_contact(reverse:bool,response:TipResponse) {
    use ambition_platformer2d::engine_core as ae;
    use ambition_platformer2d::characters::actor::{BodyHealth,BodyCombat,Health};
    use ambition_platformer2d::combat::{HitEvent, HitTarget, components::FeatureId};
    use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
    use ambition_platformer2d::portal::{PlacedPortal, PortalBodyTransited};
    use ambition_platformer2d::projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart, ProjectileVisualId};
    let launch=matches!(response,TipResponse::Damage {launch:true});

    let mut sim=fixed_60hz_room_sim("portal_lab");
    sim.step(base());
    let (owner,mut pair)={
        let world=sim.world_mut();
        let owner=world.query_filtered::<Entity,PrimaryPlayerOnly>().single(world).unwrap();
        let pair=world.query::<&PlacedPortal>().iter(world)
            .filter(|p| p.pos==Vec2::new(1904.0,856.0) || p.pos==Vec2::new(2448.0,876.0))
            .cloned().collect::<Vec<_>>();
        (owner,pair)
    };
    assert_eq!(pair.len(),2,"H must keep both authored openings");
    pair.sort_by(|a,b|a.pos.x.total_cmp(&b.pos.x));
    if reverse { pair.reverse(); }
    let (entry,exit)=(&pair[0],&pair[1]);
    let ratio=exit.half_length/entry.half_length;
    assert_eq!(ratio,if reverse {2.0} else {0.5});
    sim.spawn_enemy_character_at("carried_tip_recipient","Carried tip recipient",
        (exit.pos.x,exit.pos.y),(1.0,1.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,"npc_raptor_stalker");
    let target={
        let world=sim.world_mut();
        world.query::<(Entity,&FeatureId)>().iter(world)
            .find(|(_,id)|id.as_str()=="carried_tip_recipient").unwrap().0
    };
    let mut target_body=*sim.world().get::<ae::BodyKinematics>(target).unwrap();
    target_body.material=ae::frame::BodyMaterial::new(0.01,bevy::math::Mat2::IDENTITY).unwrap();
    let half=target_body.half_extent();
    let center=exit.pos+exit.normal*(half.dot(exit.normal.abs())+0.5*ratio);
    target_body.pos=center;
    target_body.vel=Vec2::ZERO;
    let weight=sim.world().get::<ambition_platformer2d::combat::components::CombatTuning>(target)
        .map_or(1.0,|tuning|tuning.weight);
    let mut health=BodyHealth::new(Health::new(100));
    if matches!(response,TipResponse::Parry {..}) {health.health.current=90;}
    sim.world_mut().entity_mut(target).insert((target_body,health,
        BodyCombat::default(),ae::PoseOwnedExternally::only(ae::PoseHold::Conductor)));
    if let TipResponse::Parry {absorb}=response {
        // The authored base must permit the open counter window.
        sim.world_mut().get_mut::<ae::AbilityBase>(target).unwrap().abilities.shield=true;
        sim.world_mut().entity_mut(target).insert(ae::BodyShieldState {parry_window_timer:0.2,
            absorb_window_timer:if absorb {0.2} else {0.0},..Default::default()});
    }
    sim.world_mut().write_message(ProjectileSpawnRequest::open(owner,ProjectileSpawn {
        origin:entry.pos+entry.normal*10.0,dir:-entry.normal,speed:600.0,
        damage:1,max_lifetime:2.0,half_extent:Vec2::splat(5.0),gravity:0.0,
        visual_id:"scaled_tip_gauge".into(),bounces:0,bounce_on_world_contact:false,
        splash_half_extent:0.0,boomerang_return_s:None,
    },ProjectileStart::StepNextTick).with_launch(launch.then_some(
        ambition_platformer2d::projectiles::ProjectileLaunch {base:100.0,growth:0.0})));
    sim.step(base());
    let shot={
        let world=sim.world_mut();
        world.query::<(Entity,&ProjectileVisualId)>().iter(world)
            .find(|(_,visual)|visual.0=="scaled_tip_gauge").unwrap().0
    };
    let mut transited=false;
    let initial=*sim.world().get::<ae::BodyKinematics>(shot).unwrap();
    let mut contacts=Vec::new();
    let mut intercepted=false;
    let mut cue=None;
    for _ in 0..4 {
        sim.step(base());
        transited|=sim.world().resource::<Messages<PortalBodyTransited>>()
            .iter_current_update_messages().any(|event|event.body==shot);
        contacts.extend(sim.world().resource::<Messages<HitEvent>>().iter_current_update_messages()
            .filter(|event|event.target==HitTarget::Body(target)).filter_map(|event|event.contact.clone()
                .map(|contact|(contact,event.knockback.as_ref().unwrap().magnitude))));
        intercepted=sim.world().get::<ambition_platformer2d::projectiles::ProjectileOwner>(shot)
            .is_some_and(|owner|owner.0==target);
        cue=cue.or_else(||sim.world().resource::<Messages<ambition_platformer2d::vfx::VfxInRoom>>()
            .iter_current_update_messages().find_map(|event|match event.vfx {
                ambition_platformer2d::vfx::VfxMessage::Impact {pos,..}
                    if pos.distance(center)<=half.length()+0.001=>Some(pos),_=>None,
            }));
        if intercepted || sim.world().get::<ae::BodyKinematics>(shot).is_none() { break; }
    }
    if let TipResponse::Parry {absorb}=response {
        assert!(contacts.is_empty(),"an intercepted contact cannot also deal damage");
        assert!(!transited,"a carried-tip parry precedes root transit");
        if absorb {
            assert!(sim.world().get::<ae::BodyKinematics>(shot).is_none());
        } else {
            assert!(intercepted,"the returned shot changes combat owner");
            let returned=*sim.world().get::<ae::BodyKinematics>(shot).unwrap();
            assert_eq!(returned.material,initial.material);
            assert!((returned.pos-entry.pos).dot(entry.normal)>0.0,"{returned:?}");
            assert!(returned.vel.distance(-initial.vel*1.3)<0.001,"{returned:?}");
            let sample=sim.world().get::<ae::SweepSample>(shot).unwrap();
            assert!(matches!(sample.travel,ae::LinearTravel::Prefix {until,..} if until>0.0 && until<1.0),"{sample:?}");
            assert!(cue.is_some(),"the parry cue belongs at the physical recipient");
            sim.step(base());
            let next=sim.world().get::<ae::BodyKinematics>(shot).unwrap();
            assert!((next.pos-returned.pos).dot(entry.normal)>0.0,"the shot retreats in its source chart");
        }
        sim.step(base());
        assert_eq!(sim.world().get::<BodyHealth>(target).unwrap().current(),91,
            "the catch heals exactly once and deals no damage");
        return;
    }
    assert_eq!(contacts.len(),1,"the carried tip must name one recipient: reverse={reverse}");
    assert!(!transited,"contact precedes the root crossing: reverse={reverse}");
    let (contact,magnitude)=&contacts[0];
    use ambition_platformer2d::combat::HitKnockbackMagnitude;
    let expected=if launch {
        let rules=ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<
            ambition_platformer2d::combat::rules::RoomCombatTuning>(sim.world()).unwrap().0;
        let HitKnockbackMagnitude::LaunchSpeed(speed)=ambition_platformer2d::combat::hitbox::projectile_launch_magnitude(
            100.0,0.0,0,weight,&rules) else { panic!("an authored launch has speed units"); };
        HitKnockbackMagnitude::LaunchSpeed(speed*ratio)
    } else {HitKnockbackMagnitude::FeelScale(0.85)};
    assert_eq!(*magnitude,expected,"convert speed, preserve feel: reverse={reverse}, launch={launch}");
    assert!(contact.time>0.0 && contact.time<1.0,"{contact:?}");
    assert!((contact.point-center).length()<=half.length()+0.001,"{contact:?}");
    sim.step(base());
    assert_eq!(sim.world().get::<BodyHealth>(target).unwrap().current(),99,
        "the real actor consumes exactly one point of damage");
    assert!(sim.world().get::<ae::BodyKinematics>(shot).is_none(),"contact spends the shot");

}

#[test]
fn a_projectile_meets_a_wall_that_crosses_its_far_part_between_endpoint_poses() {
    far_side_contact(ContactMotion::Wall);
}

#[test]
fn a_hosted_exit_carries_material_into_the_lab_target_during_the_tick() {
    far_side_contact(ContactMotion::Exit);
}

#[test]
fn a_spinning_plank_hits_the_far_material_between_matching_endpoint_poses() {
    far_side_contact(ContactMotion::TurningWall);
}

#[test]
fn a_spinning_exit_carries_material_to_a_target_outside_both_endpoint_poses() {
    far_side_contact(ContactMotion::TurningExit);
}

#[test]
fn a_spinning_exit_carries_a_projectile_into_a_hurt_body_between_matching_poses() {
    far_side_contact(ContactMotion::TurningExitBody);
}

#[test]
fn a_projectile_keeps_the_opening_when_its_tip_first_arrives_at_tick_end() {
    far_side_contact(ContactMotion::Endpoint);
}

enum ContactMotion { Fixed, Bounce(bool), Wall, Exit, TurningWall, TurningExit, TurningExitBody, Endpoint }

#[test]
fn the_authored_moving_exit_hits_its_target_before_the_shots_root_transfers() {
    use ambition_platformer2d::engine_core::RoomGeometry;
    use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
    use ambition_platformer2d::portal::{link_hash, PlacedPortal, PortalBodyTransited, PortalHost, PortalLink};
    use ambition_platformer2d::projectiles::{ProjectileOwner, ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart, ProjectileVisualId};
    use ambition_platformer2d::vfx::{VfxInRoom, VfxMessage};

    let mut sim = fixed_60hz_room_sim("portal_lab");
    sim.step(base());
    sim.step(base());
    let owner = {
        let world = sim.world_mut();
        world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap()
    };
    let (entry, exit_channel, target) = {
        let world = sim.world_mut();
        let mut pair = world.query::<(&PlacedPortal, &PortalLink)>().iter(world)
            .filter(|(_, link)| link.0 == link_hash("portal_lab_l_motion"))
            .map(|(portal, _)| portal.clone()).collect::<Vec<_>>();
        assert_eq!(pair.len(), 2);
        pair.sort_by(|a, b| a.pos.y.total_cmp(&b.pos.y));
        let exit = pair.remove(0);
        assert!(matches!(exit.host, PortalHost::Face { .. }), "L must use its authored moving host: {exit:?}");
        let target = world.query::<&RoomGeometry>().single(world).unwrap().0.blocks.iter()
            .find(|block| block.name == "L far-side target").unwrap().aabb;
        (pair.remove(0), exit.channel, target)
    };

    // Fire while the exit approaches the target. The next tick creates the
    // shot; the following tick carries its leading material into the target.
    let mut ready = false;
    for _ in 0..90 {
        let exit = {
            let world = sim.world_mut();
            world.query::<&PlacedPortal>().iter(world).find(|p| p.channel == exit_channel).unwrap().clone()
        };
        let gap = target.min.x - exit.pos.x;
        if (8.0..=12.0).contains(&gap) && exit.step.displacement.x > 0.0 {
            ready = true;
            break;
        }
        sim.step(base());
    }
    assert!(ready, "the authored host must sweep toward the target");
    sim.world_mut().write_message(ProjectileSpawnRequest::open(owner, ProjectileSpawn {
        origin: entry.pos + entry.normal * 6.0,
        dir: -entry.normal,
        speed: 1200.0,
        damage: 1,
        max_lifetime: 2.0,
        half_extent: Vec2::splat(5.0),
        gravity: 0.0,
        visual_id: "authored_moving_exit_gauge".into(),
        bounces: 0,
        bounce_on_world_contact: false,
        splash_half_extent: 0.0,
        boomerang_return_s: None,
    }, ProjectileStart::StepNextTick));
    sim.step(base());
    let shot = {
        let world = sim.world_mut();
        world.query::<(Entity, &ProjectileOwner, &ProjectileVisualId)>().iter(world)
            .find(|(_, by, visual)| by.0 == owner && visual.0 == "authored_moving_exit_gauge")
            .map(|(entity, _, _)| entity).unwrap()
    };
    let mut impact = false;
    let mut transited = false;
    for _ in 0..4 {
        sim.step(base());
        transited |= sim.world().resource::<Messages<PortalBodyTransited>>()
            .iter_current_update_messages().any(|event| event.body == shot);
        impact |= sim.world().resource::<Messages<VfxInRoom>>()
            .iter_current_update_messages().any(|event| matches!(event.vfx,
                VfxMessage::Impact { pos, .. } if (pos.x-target.min.x).abs()<0.01
                    && (target.min.y..=target.max.y).contains(&pos.y)));
        if sim.world().get::<BodyKinematics>(shot).is_none() { break; }
    }
    assert!(impact, "the shot must hit L's authored target");
    assert!(!transited, "leading material must contact before the root transfers");
    assert!(sim.world().get::<BodyKinematics>(shot).is_none(), "the contact expires the shot");
}

fn far_side_contact(motion:ContactMotion) {
    use ambition_platformer2d::engine_core::{Aabb,AabbExt,RoomGeometry};
    use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
    use ambition_platformer2d::portal::{link_hash, PlacedPortal, PortalBodyTransited, PortalHost, PortalLink};
    use ambition_platformer2d::projectiles::{ProjectileOwner, ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart, ProjectileVisualId};
    use ambition_platformer2d::vfx::{VfxInRoom, VfxMessage};
    use ambition_platformer2d::world::{collision::MovingPlatformSet, platforms::{MovingPlatformState, Plank, PlatformTurn}};

    let mut sim = fixed_60hz_room_sim("portal_lab");
    sim.step(base());
    let owner = {
        let world = sim.world_mut();
        world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap()
    };
    let (enter, exit) = {
        let world = sim.world_mut();
        let mut pair: Vec<_> = world.query::<(&PlacedPortal, &PortalLink)>().iter(world)
            .filter(|(portal, link)| if matches!(motion,ContactMotion::Bounce(_)) {
                portal.pos==Vec2::new(1904.0,856.0) || portal.pos==Vec2::new(2448.0,876.0)
            } else {link.0 == link_hash("portal_lab_k_contact")})
            .map(|(portal, _)| portal.clone()).collect();
        assert_eq!(pair.len(), 2, "the contact station must load both linked endpoints");
        pair.sort_by(|a, b| a.pos.x.total_cmp(&b.pos.x));
        if matches!(motion,ContactMotion::Bounce(true)) {pair.reverse();}
        (pair.remove(0), pair.remove(0))
    };
    let tangent = Vec2::new(-exit.normal.y, exit.normal.x);
    if matches!(motion,ContactMotion::Bounce(_)) {
        let room=*sim.world().get::<ambition_platformer2d::platformer::lifecycle::InRoomInstance>(owner).unwrap();
        assert_eq!(exit.half_length/enter.half_length,if matches!(motion,ContactMotion::Bounce(true)) {2.0} else {0.5});
        let wall=Aabb::new(exit.pos+exit.normal*202.0,
            exit.normal.abs()*200.0+tangent.abs()*40.0);
        let world=sim.world_mut();
        world.query::<&mut RoomGeometry>().single_mut(world).unwrap().0.blocks.push(
            ambition_platformer2d::engine_core::Block::solid("H bounce support",wall.min,wall.max-wall.min));
        world.spawn((room,ambition_platformer2d::world::GravityZone {
            aabb:Aabb::new(enter.pos,Vec2::splat(100.0)),dir:-enter.normal,
        }));
    }
    sim.world_mut().write_message(ProjectileSpawnRequest::open(owner, ProjectileSpawn {
        origin: enter.pos + enter.normal * if matches!(motion,ContactMotion::Endpoint) {45.0} else {40.0},
        dir: -enter.normal,
        speed: 600.0,
        damage: 1,
        max_lifetime: 2.0,
        half_extent: Vec2::splat(5.0),
        gravity: 0.0,
        visual_id: "far_contact_gauge".into(),
        bounces: if matches!(motion,ContactMotion::Bounce(_)) {2} else {0},
        bounce_on_world_contact: matches!(motion,ContactMotion::Bounce(_)),
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
    if matches!(motion,ContactMotion::Bounce(_)) {
        sim.world_mut().get_mut::<ambition_platformer2d::projectiles::ProjectileGameplay>(shot).unwrap().rebound=
            ambition_platformer2d::projectiles::Rebound {restitution:0.5,min_speed:0.0};
    }
    let initial_material=sim.world().get::<BodyKinematics>(shot).unwrap().material;
    let mut transited = false;
    let mut impact = false;
    let mut expired = false;
    let mut history = Vec::new();
    let mut moved_target=false;
    let mut hurt_body=None;
    for _ in 0..12 {
        if matches!(motion,ContactMotion::Wall|ContactMotion::Exit|ContactMotion::TurningWall|ContactMotion::TurningExit|ContactMotion::TurningExitBody) && !moved_target && sim.world().get::<BodyKinematics>(shot)
            .is_some_and(|body|(body.pos-enter.pos).dot(enter.normal)<=10.01) {
            let world=sim.world_mut();
            if matches!(motion,ContactMotion::Exit|ContactMotion::TurningExit|ContactMotion::TurningExitBody) {
                // The moving host replaces the fixed backing.
                world.query::<&mut RoomGeometry>().single_mut(world).unwrap().0.blocks
                    .retain(|block| block.name != "K exit backing");
            }
            if matches!(motion,ContactMotion::TurningWall) {
                world.query::<&mut RoomGeometry>().single_mut(world).unwrap().0.blocks
                    .retain(|block| block.name!="K far-side target");
                let mut platform=MovingPlatformState::from_sweep("k_spinning_target", "K spinning target",
                    exit.pos+exit.normal*6.0-tangent*10.0, Vec2::new(40.0,12.0), 0.0, 0.0);
                platform.plank=Some(Plank {turn:PlatformTurn::Spin {deg_per_s:21600.0},..Default::default()});
                ambition_platformer2d::session::sole_live_room_component_mut::<MovingPlatformSet>(world)
                    .unwrap().0.push(platform);
            } else if matches!(motion,ContactMotion::TurningExit|ContactMotion::TurningExitBody) {
                let mut platform=MovingPlatformState::from_sweep("k_spinning_exit","K spinning exit",
                    exit.pos,Vec2::new(80.0,12.0),0.0,0.0);
                platform.plank=Some(Plank {turn:PlatformTurn::Spin {deg_per_s:21600.0},
                    turned_s:1.0/240.0,angle:std::f32::consts::FRAC_PI_2,..Default::default()});
                ambition_platformer2d::session::sole_live_room_component_mut::<MovingPlatformSet>(world)
                    .unwrap().0.push(platform);
                world.query::<&mut RoomGeometry>().single_mut(world).unwrap().0.blocks.to_mut().iter_mut()
                    .find(|block|block.name=="K far-side target").unwrap().aabb=
                    Aabb::new(exit.pos-tangent*6.0,Vec2::new(5.0,0.5));
                let mut portals=world.query::<&mut PlacedPortal>();
                portals.iter_mut(world).find(|p|p.channel==exit.channel).unwrap().host=
                    PortalHost::Line {line:"k_spinning_exit".into(),along:40.0,lift:0.0};
                if matches!(motion,ContactMotion::TurningExitBody) {
                    let aabb=Aabb::new(exit.pos-tangent*6.0,Vec2::new(5.0,0.5));
                    world.query::<&mut RoomGeometry>().single_mut(world).unwrap().0.blocks
                        .retain(|block|block.name!="K far-side target");
                    let room=*world.get::<ambition_platformer2d::platformer::lifecycle::InRoomInstance>(owner).unwrap();
                    hurt_body=Some(world.spawn((room,ambition_platformer2d::characters::actor::ActorFaction::Enemy,
                        ambition_platformer2d::observation::SimId::placement("turning_exit_hurt_body"),
                        ambition_platformer2d::engine_core::CenteredAabb::new(aabb.center(),aabb.half_size()),
                        ambition_platformer2d::combat::components::DamageableVolumes::world_box(aabb))).id());
                }
            } else if matches!(motion,ContactMotion::Wall) {
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
        if matches!(motion,ContactMotion::Bounce(_)) && sim.world().get::<ambition_platformer2d::projectiles::ProjectileGameplay>(shot)
            .is_some_and(|game|game.bounces_remaining==1) {
            let bounced=*sim.world().get::<BodyKinematics>(shot).unwrap();
            assert!(!transited);
            assert_eq!(bounced.material,initial_material);
            assert!((bounced.pos-enter.pos).dot(enter.normal)>0.0,"{bounced:?}");
            assert!(bounced.vel.distance(enter.normal*300.0)<0.01,"{bounced:?}");
            assert!(sim.world().resource::<Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>()
                .iter_current_update_messages().any(|event|matches!(event.request,
                    ambition_platformer2d::sfx::SfxMessage::Hit {pos}
                        if ((pos-exit.pos).dot(exit.normal)-2.0).abs()<0.01)),
                "the bounce cue belongs at the far contact");
            sim.step(base());
            let next=sim.world().get::<BodyKinematics>(shot).expect("the rebound must clear its contact on the next tick");
            assert!((next.pos-bounced.pos).dot(enter.normal)>0.0,"{next:?}");
            assert_eq!(sim.world().get::<ambition_platformer2d::projectiles::ProjectileGameplay>(shot).unwrap().bounces_remaining,1);
            return;
        }
        if let Some(target)=hurt_body {
            impact|=sim.world().resource::<Messages<ambition_platformer2d::combat::HitEvent>>()
                .iter_current_update_messages().any(|event|event.target==ambition_platformer2d::combat::HitTarget::Body(target)
                    && event.contact.as_ref().is_some_and(|contact| {
                        let offset=contact.point-exit.pos;
                        (-5.01..=5.01).contains(&offset.dot(exit.normal))
                            && (-6.51..=-5.49).contains(&offset.dot(tangent))
                    }));
        }
        impact |= sim.world().resource::<Messages<VfxInRoom>>()
            .iter_current_update_messages().any(|event| match event.vfx {
                VfxMessage::Impact { pos, .. } => {
                    let offset = pos-exit.pos;
                    match motion {
                        ContactMotion::Wall=>(2.0..=2.2).contains(&offset.dot(exit.normal)) && (offset.dot(tangent)+5.0).abs()<0.01,
                        ContactMotion::Fixed|ContactMotion::Bounce(_)|ContactMotion::Endpoint=>(offset.dot(exit.normal)-2.0).abs() < 0.01 && offset.dot(tangent).abs() <= 5.01,
                        ContactMotion::Exit=>(offset.dot(exit.normal)-10.0).abs() < 0.05 && offset.dot(tangent).abs()<=5.01,
                        ContactMotion::TurningWall=>(-0.01..=5.01).contains(&offset.dot(exit.normal)) && offset.dot(tangent).abs()<=5.01,
                        ContactMotion::TurningExit|ContactMotion::TurningExitBody=>(-5.01..=5.01).contains(&offset.dot(exit.normal))
                            && (-6.51..=-5.49).contains(&offset.dot(tangent)),
                    }
                }
                _ => false,
            });
        if sim.world().get::<BodyKinematics>(shot).is_none() {
            expired = true;
            break;
        }
    }
    assert!(!matches!(motion,ContactMotion::Bounce(_)),"the shot did not produce a rebound: {history:?}");
    if matches!(motion,ContactMotion::Endpoint) {
        assert!(history.iter().flatten().any(|body| ((body.pos-enter.pos).dot(enter.normal)-5.0).abs()<0.01),
            "the leading tip must reach the opening at tick end without a false wall hit: {history:?}");
    }
    assert!(expired, "the far-side wall must stop the shot; enter={enter:?}, exit={exit:?}, poses={history:?}");
    assert!(!transited, "the material must meet the far wall before its root crosses");
    assert!(impact, "the impact must be at the physical far-side contact");
}


#[test]
fn the_lab_g_spinner_takes_a_still_root_during_a_full_turn() {
    use ambition_platformer2d::engine_core::SweepSample;
    use ambition_platformer2d::portal::{link_hash, PlacedPortal, PortalBodyTransited, PortalLink};
    use ambition_platformer2d::world::{collision::MovingPlatformSet, platforms::PlatformTurn};
    let mut sim = fixed_60hz_room_sim("portal_lab");
    sim.step(base());
    let enter = {
        let world = sim.world_mut();
        world.query::<(&PlacedPortal, &PortalLink)>().iter(world)
            .filter(|(_, link)| link.0 == link_hash("lab_g"))
            .map(|(portal, _)| portal.clone()).max_by(|a, b| a.pos.x.total_cmp(&b.pos.x)).unwrap()
    };
    let root = enter.pos + Vec2::new(-enter.normal.y, enter.normal.x) * 30.0 + enter.normal * 4.0;
    let body = BodyKinematics { pos: root, vel: Vec2::ZERO, canonical_size: Vec2::splat(4.0), ..Default::default() };
    let entity = sim.world_mut().spawn((body, SweepSample::at_rest(body, Vec2::ZERO))).id();
    let mut host = ambition_platformer2d::session::sole_live_room_component_mut::<MovingPlatformSet>(sim.world_mut())
        .expect("the lab has moving hosts");
    let plank = host.0.iter_mut().find(|host| host.name == "G spinning plank").unwrap()
        .plank.as_mut().unwrap();
    plank.turned_s = plank.angle.to_degrees() / 21600.0;
    plank.turn = PlatformTurn::Spin { deg_per_s: 21600.0 };
    drop(host);
    sim.step(base());
    let event = sim.world_mut().resource_mut::<Messages<PortalBodyTransited>>().drain()
        .find(|event| event.body == entity).expect("the recorded full turn crosses the still root");
    assert!(event.transfer.crossing_time > 0.0 && event.transfer.crossing_time < 1.0);
    assert_eq!(event.transfer.crossing_point, root);
    let frame = event.transfer.enter.frame;
    assert!((root - frame.origin).dot(frame.normal).abs() < 1.0e-4);
    assert!(frame.normal.distance(enter.normal) > 0.5);
    assert!(sim.world().get::<BodyKinematics>(entity).unwrap().pos.distance(root) > 100.0);
}


#[test]
fn a_body_wider_than_the_opening_stays_on_the_host() {
    use ambition_platformer2d::engine_core::{frame::BodyMaterial, SweepSample};
    use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
    let mut sim = fixed_60hz_room_sim("portal_lab");
    for _ in 0..4 { sim.step(base()); }
    let (enter, _) = crate::common::first_floor_authored_portal_pair(&mut sim);
    let body = {
        let world = sim.world_mut();
        world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap()
    };
    let host_y = {
        let world = sim.world_mut();
        let room = world.query::<&ambition_platformer2d::engine_core::RoomGeometry>().single(world).unwrap();
        room.0.blocks.iter().filter(|block| block.aabb.min.x <= enter.pos.x && enter.pos.x < block.aabb.max.x)
            .min_by(|a, b| (a.aabb.min.y - enter.pos.y).abs().total_cmp(&(b.aabb.min.y - enter.pos.y).abs()))
            .expect("the floor portal has a host").aabb.min.y
    };
    let width = enter.aperture().half_length * 2.0 + 1.0;
    let mut kin = sim.world_mut().get_mut::<BodyKinematics>(body).unwrap();
    kin.material = BodyMaterial::new(width / kin.canonical_size.x, bevy::math::Mat2::IDENTITY).unwrap();
    kin.pos = enter.pos + enter.normal * (kin.size().y * 0.5 + 60.0);
    kin.vel = -enter.normal * 600.0;
    let record = SweepSample::at_rest(*kin, Vec2::Y);
    drop(kin);
    sim.world_mut().entity_mut(body).insert(record);
    for _ in 0..24 {
        sim.step(base());
        let kin = sim.world().get::<BodyKinematics>(body).unwrap();
        assert!(kin.size().x > enter.aperture().half_length * 2.0, "the body must remain wider than the opening");
        assert!(kin.pos.y + kin.size().y * 0.5 <= host_y + 0.01,
            "the host must hold the complete body: host_y={host_y}, body={kin:?}");
    }
}


#[test]
fn the_lab_g_spinner_cannot_transfer_a_body_whose_source_motion_was_refused() {
    use ambition_platformer2d::engine_core::{frame::thread::PieceBudget, SweepSample};
    use ambition_platformer2d::platformer::{markers::PrimaryPlayerOnly, body::geometry::MaterialBudget};
    use ambition_platformer2d::portal::{link_hash, PlacedPortal, PortalBodyTransited, PortalLink};
    use ambition_platformer2d::world::{collision::MovingPlatformSet, platforms::PlatformTurn};
    let mut sim = fixed_60hz_room_sim("portal_lab");
    sim.step(base());
    let (enter, entity) = {
        let world = sim.world_mut();
        let enter = world.query::<(&PlacedPortal, &PortalLink)>().iter(world)
            .filter(|(_, link)| link.0 == link_hash("lab_g"))
            .map(|(portal, _)| portal.clone()).max_by(|a, b| a.pos.x.total_cmp(&b.pos.x)).unwrap();
        let entity = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
        (enter, entity)
    };
    let root = enter.pos + Vec2::new(-enter.normal.y, enter.normal.x) * 10.0 + enter.normal * 4.0;
    let before = BodyKinematics { pos: root, vel: Vec2::ZERO, ..*sim.world().get::<BodyKinematics>(entity).unwrap() };
    sim.world_mut().entity_mut(entity).insert((before, SweepSample::at_rest(before, Vec2::Y),
        MaterialBudget(PieceBudget { max_hops: 0, ..PieceBudget::COMPOSED })));
    let mut host = ambition_platformer2d::session::sole_live_room_component_mut::<MovingPlatformSet>(sim.world_mut()).unwrap();
    let plank = host.0.iter_mut().find(|host| host.name == "G spinning plank").unwrap().plank.as_mut().unwrap();
    plank.turned_s = plank.angle.to_degrees() / 21600.0;
    plank.turn = PlatformTurn::Spin { deg_per_s: 21600.0 };
    drop(host);
    sim.step(base());
    assert_eq!(*sim.world().get::<BodyKinematics>(entity).unwrap(), before,
        "source refusal must retain the body through the final transit pass");
    assert!(!sim.world_mut().resource_mut::<Messages<PortalBodyTransited>>().drain()
        .any(|event| event.body == entity), "refused motion must not publish a transfer");
    let sample = sim.world().get::<SweepSample>(entity).unwrap();
    assert_eq!(sample.time, [1.0, 1.0], "a refused operation has no earlier travel interval");
    sim.world_mut().entity_mut(entity).remove::<MaterialBudget>();
    sim.step(base());
    assert!(sim.world_mut().resource_mut::<Messages<PortalBodyTransited>>().drain()
        .any(|event| event.body == entity), "the same body can cross when the source operation is admitted");
    assert!(sim.world().get::<BodyKinematics>(entity).unwrap().pos.distance(root) > 100.0);
}
