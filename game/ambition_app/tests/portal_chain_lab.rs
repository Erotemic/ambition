#![cfg(all(feature = "portal", feature = "rl_sim"))]

use ambition_app::{AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions};
use ambition_platformer2d::engine_core::{self as ae, BodyKinematics, ControlFrame};
use ambition_platformer2d::characters::actor::CharacterForm;
use ambition_platformer2d::combat::{hitbox::strike_pieces, strike::{Hitbox, HitboxHits}};
use ambition_platformer2d::item::{GroundItem, ItemCustody};
use ambition_platformer2d::platformer::{body::geometry::BodyGeometry,
    lifecycle::InRoomInstance, markers::PrimaryPlayerOnly};
use ambition_platformer2d::portal::{EditablePortalTuning, PlacedPortal, PortalConvention, PortalGeometry,
    PortalLink, PortalTuning, link_hash};
use bevy::prelude::*;

/// The floor top of `portal_chain_lab`.
const FLOOR: f32 = 512.0;

fn place(sim: &mut Platformer2dSimHarness, who: Entity, at: Vec2) {
    let mut body = *sim.world().get::<BodyKinematics>(who).unwrap();
    body.pos = at;
    body.vel = Vec2::ZERO;
    body.facing = 1.0;
    sim.world_mut().entity_mut(who).insert((body, ae::SweepSample::at_rest(body, Vec2::Y)));
}

fn station(max_hops: u8) {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default().with_required_start_room("portal_chain_lab")
            .with_sync_test_rollback_settings(4, 10)).unwrap();
    for _ in 0..30 { sim.step(crate::common::base()); }
    let (player, axe, target) = {
        let world = sim.world_mut();
        let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
        let axe = world.query::<(Entity, &GroundItem)>().iter(world)
            .find(|(_, item)| item.spec.id == "axe").unwrap().0;
        let target = world.query::<(Entity, &CharacterForm)>().iter(world)
            .find(|(_, worn)| worn.id() == "sandbag_infinite").unwrap().0;
        (player, axe, target)
    };
    {
        let world = sim.world_mut();
        let mut tuning = world.resource_mut::<PortalTuning>();
        tuning.convention = PortalConvention::Reflection;
        tuning.geometry = PortalGeometry::Composed { max_hops, max_pieces: 32, max_scale_exponent: 12 };
        let tuning = *tuning;
        world.resource_mut::<EditablePortalTuning>().0 = tuning;
    }
    // The foot door's mouth that faces the axe: the body stands with its back
    // half through it, supported by the floor on both sides of the wall.
    let foot = {
        let world = sim.world_mut();
        world.query::<(&PlacedPortal, &PortalLink)>().iter(world)
            .find(|(portal, link)| link.0 == link_hash("portal_lab_j_other") && portal.normal.x > 0.5)
            .unwrap().0.clone()
    };
    let at = sim.world().get::<BodyKinematics>(axe).unwrap().pos;
    let half = sim.world().get::<BodyKinematics>(player).unwrap().half_extent();
    place(&mut sim, player, Vec2::new(at.x, FLOOR-half.y));
    let press = ControlFrame { attack_pressed: true, ..Default::default() };
    sim.rebase_rollback_history().unwrap();
    sim.step_frame(press);
    for _ in 0..6 { sim.step_frame(ControlFrame::default()); }
    assert!(matches!(sim.world().get::<ItemCustody>(axe), Some(ItemCustody::Held { holder }) if *holder == player));
    place(&mut sim, player, Vec2::new(foot.pos.x+4.0, FLOOR-half.y));
    sim.rebase_rollback_history().unwrap();
    for _ in 0..3 { sim.step_frame(ControlFrame::default()); }
    let mut simultaneous = false;
    let mut refused = false;
    let mut contacts = bevy::ecs::message::MessageCursor::<ambition_platformer2d::combat::HitEvent>::default();
    let mut target_hit = false;
    let mut one_hit_ledger = false;
    sim.step_frame(press);
    for _ in 0..60 {
        sim.step_frame(ControlFrame::default());
        for hit in contacts.read(sim.world().resource::<Messages<ambition_platformer2d::combat::HitEvent>>()) {
            if hit.attacker == Some(player) && hit.target == ambition_platformer2d::combat::HitTarget::Body(target) {
                target_hit = true;
                assert_eq!(hit.damage, 3);
            }
        }
        sim.rollback_health().unwrap();
        let supported = sim.observation().on_ground;
        let world = sim.world_mut();
        let body = *world.get::<BodyKinematics>(player).unwrap();
        let room = world.get::<InRoomInstance>(player).unwrap().0;
        let hits: Vec<_> = world.query::<(&Hitbox, &HitboxHits)>().iter(world)
            .filter(|(hit, _)| hit.owner == player).map(|(hit, ledger)| {
                if ledger.hit.contains(&target) {
                    assert_eq!(ledger.hit.len(), 1);
                    one_hit_ledger = true;
                }
                hit.clone()
            }).collect();
        let geometry = world.resource::<BodyGeometry>();
        for hit in hits {
            let strikes = match strike_pieces(Some(geometry), &hit, Some(&body), Some(room)) {
                Ok(parts) => parts,
                Err(refusal) => {
                    assert_eq!(refusal.reason, ae::frame::thread::RefusalReason::HopLimit);
                    refused = true;
                    continue;
                }
            };
            let domain = body.canonical_box().unwrap();
            let feet = geometry.body_pieces(ae::frame::thread::MaterialPrimitive {
                domain: &domain, to_world: body.material_map().unwrap() }, player, Some(room)).unwrap();
            let third = strikes.iter().find(|piece| piece.path.len() == 3);
            if let Some(third) = third {
                assert_eq!(third.path_transform().unwrap().scale(), 0.25);
                assert!(third.path_transform().unwrap().reflected());
                simultaneous |= supported && feet.iter().any(|piece| piece.path.len() == 1
                    && piece.path[0] != third.path[0]);
            }
        }
    }
    if max_hops >= 3 {
        assert!(simultaneous, "the held swing and foot must use independent paths");
        assert!(target_hit && one_hit_ledger, "one authored axe swing must reach the far target once");
        assert!(!refused);
    } else {
        assert!(refused, "the third cut must exceed the two-hop limit");
        assert!(!target_hit && !one_hit_ledger, "a refused swing must not damage the target");
        assert!(sim.observation().on_ground);
        assert_eq!(sim.world().get::<BodyKinematics>(player).unwrap().material.scale(), 1.0);
    }
    assert!(sim.rollback_execution_stats().unwrap().load_runs > 0);
}

#[test]
fn the_authored_axe_reaches_a_target_through_three_pairs_while_a_foot_uses_another() {
    station(4);
}

#[test]
fn a_two_hop_limit_refuses_the_authored_swing_and_keeps_the_body_supported() {
    station(2);
}
