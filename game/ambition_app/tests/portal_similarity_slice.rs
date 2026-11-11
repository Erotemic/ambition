#![cfg(all(feature = "portal", feature = "rl_sim"))]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions};
use ambition_platformer2d::engine_core::{self as ae, BodyKinematics, RoomGeometry};
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d::portal::{PlacedPortal, PortalConvention, PortalTuning, EditablePortalTuning};
use bevy::prelude::*;

fn body(sim: &mut Platformer2dSimHarness) -> (Entity, BodyKinematics) {
    let world = sim.world_mut();
    let (entity, body) = world.query_filtered::<(Entity, &BodyKinematics), (With<PlayerEntity>, With<PrimaryPlayer>)>()
        .single(world).unwrap();
    (entity, *body)
}

fn slice() -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_required_start_room("portal_lab")
            .with_sync_test_rollback_settings(4, 10),
    ).unwrap();
    sim.world_mut().resource_mut::<PortalTuning>().convention = PortalConvention::Reflection;
    sim.world_mut().resource_mut::<EditablePortalTuning>().convention = PortalConvention::Reflection;
    sim.rebase_rollback_history().unwrap();
    for _ in 0..12 { sim.step(crate::common::base()); }
    let (entity, mut pose) = body(&mut sim);
    assert_eq!(sim.world().get::<ambition_platformer2d::characters::actor::WornCharacter>(entity).unwrap().id(), "player_robot_v3");
    let world = sim.world_mut();
    let mut portals = world.query::<&PlacedPortal>();
    let endpoints: Vec<_> = portals.iter(world).filter(|p| p.half_length == 40.0 || p.half_length == 20.0)
        .filter(|p| p.pos.x > 1800.0 && p.pos.y > 800.0).collect();
    assert!(endpoints.iter().any(|p| p.pos == ae::Vec2::new(1904.0, 856.0) && p.opening_half_length() == 40.0));
    assert!(endpoints.iter().any(|p| p.pos == ae::Vec2::new(2448.0, 876.0) && p.opening_half_length() == 20.0));
    pose.pos = ae::Vec2::new(1860.0, 896.0 - pose.half_extent().y);
    pose.vel = ae::Vec2::ZERO;
    world.entity_mut(entity).insert((pose, ae::SweepSample::at_rest(pose, ae::Vec2::Y)));
    world.query::<&mut RoomGeometry>().single_mut(world).unwrap().0.blocks.push(
        ae::Block::solid("slice far contact", ae::Vec2::new(2441.0, 881.0), ae::Vec2::new(2.0, 14.0)));
    sim.rebase_rollback_history().unwrap();
    sim
}

fn run() -> Vec<Vec<u8>> {
    let mut sim = slice();
    let (_, initial) = body(&mut sim);
    let mut trace = Vec::new();
    let mut shrunk = 0;
    let mut restored = 0;
    let mut stationary = None;
    let mut far_clear_at = None;
    let mut returned = false;
    for frame in 0..220 {
        if frame == 60 {
            let (_, pose) = body(&mut sim);
            assert_eq!(pose.material.scale(), 1.0, "the far-side wall must stop the near-side root");
            assert!(pose.pos.x < 1904.0 && pose.pos.x + pose.half_extent().x > 1904.0, "{pose:?}");
            let far_edge = 2448.0 - 0.5 * (pose.pos.x + pose.half_extent().x - 1904.0);
            assert!((far_edge - 2443.0).abs() < 0.002, "physical cap stops at {far_edge}, wall face is 2443");
            assert!(sim.observation().on_ground);
            let world = sim.world_mut();
            world.query::<&mut RoomGeometry>().single_mut(world).unwrap().0.blocks.retain(|b| b.name != "slice far contact");
            sim.rebase_rollback_history().unwrap();
        }
        let moving = if frame < 60 { frame < 50 }
            else { !returned && far_clear_at.is_none_or(|at| frame > at + 30) };
        sim.step(AgentAction { move_x: if moving { 1.0 } else { 0.0 }, ..crate::common::base() });
        sim.rollback_health().unwrap_or_else(|error| panic!("frame {frame}: {error}"));
        let (_, pose) = body(&mut sim);
        if frame == 50 { stationary = Some(pose.pos); }
        if (51..60).contains(&frame) {
            assert_eq!(Some(pose.pos), stationary, "a stationary partial body must keep its pose");
            assert!(sim.observation().on_ground, "a stationary partial body must keep its support");
        }
        if pose.material.scale() == 0.5 {
            if far_clear_at.is_none() && pose.pos.x + pose.half_extent().x < 2448.0 {
                far_clear_at = Some(frame);
            }
            shrunk += 1;
            assert!(pose.material.reflected(), "frame {frame}: {pose:?}, convention {:?}",
                sim.world().resource::<PortalTuning>().convention);
            if frame < 90 { assert_eq!(pose.facing, initial.facing, "held steering must retain material facing through a reflection"); }
            assert_eq!(pose.canonical_size, initial.canonical_size);
            assert_eq!(pose.size(), initial.size() * 0.5);
            assert!(pose.material.orthogonal().abs_diff_eq(Mat2::from_cols(Vec2::NEG_X, Vec2::Y), 1.0e-5));
        }
        if shrunk > 0 && pose.material.scale() == 1.0 {
            if pose.pos.x + pose.half_extent().x < 1904.0 { returned = true; }
            restored += 1;
            assert!(!pose.material.reflected());
            assert!(pose.material.orthogonal().abs_diff_eq(Mat2::IDENTITY, 1.0e-5));
        }
        trace.push(ae::snapshot::encode_state(&pose));
    }
    assert!(shrunk > 10, "the body did not remain small after leaving the opening");
    assert!(restored > 10, "the reverse crossing did not restore physical material");
    let stats = sim.rollback_execution_stats().unwrap();
    assert!(stats.load_runs > 0, "the slice must exercise a real rewind");
    trace
}

#[test]
fn the_authored_80_40_pair_carries_partial_contact_scale_parity_and_reverse_travel() {
    assert_eq!(run(), run(), "the same input and scene edits must replay bit for bit");
}

#[cfg(feature = "visible")]
#[test]
fn the_shipped_renderer_keeps_the_crossed_robots_physical_scale_and_reflection() {
    use ambition_platformer2d::game_shell::ShellCommand;
    use ambition_platformer2d::platformer::lifecycle::PlayerVisual;
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::NoWindow, true, |app| {
            app.insert_resource(ambition_app::app::StartRoomOverride("portal_lab".into()));
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f32(1.0 / 60.0)));
        },
    );
    for _ in 0..30 { app.update(); }
    app.world_mut().write_message(ShellCommand::GoTo(ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into()));
    let mut player = None;
    for _ in 0..1200 {
        app.update();
        let world = app.world_mut();
        player = world.query_filtered::<Entity, (With<PlayerVisual>, With<BodyKinematics>)>().iter(world).next();
        if player.is_some() { break; }
    }
    let player = player.expect("the visible route creates the robot");
    for _ in 0..30 { app.update(); }
    app.world_mut().resource_mut::<PortalTuning>().convention = PortalConvention::Reflection;
    app.world_mut().resource_mut::<EditablePortalTuning>().convention = PortalConvention::Reflection;
    let mut pose = *app.world().get::<BodyKinematics>(player).unwrap();
    pose.pos = Vec2::new(1903.0, 896.0 - pose.half_extent().y);
    pose.vel = Vec2::new(600.0, 0.0);
    app.world_mut().entity_mut(player).insert((pose, ae::SweepSample::at_rest(pose, Vec2::Y)));
    let mut samples = 0;
    for _ in 0..40 {
        app.update();
        let pose = *app.world().get::<BodyKinematics>(player).unwrap();
        if pose.material.scale() != 0.5 { continue; }
        assert!(pose.material.reflected());
        let root = app.world().get::<Transform>(player).unwrap();
        let local = Vec2::new(3.0, -7.0);
        let physical = pose.material.at(Vec2::ZERO).unwrap().vector(local).unwrap();
        let rendered = root.rotation * (root.scale * Vec3::new(local.x, -local.y, 0.0));
        assert!(rendered.truncate().distance(Vec2::new(physical.x, -physical.y)) < 1.0e-5,
            "the rendered asymmetric point must use the collision material: {pose:?} {root:?}");
        samples += 1;
    }
    assert!(samples > 20, "the rendered body must stay small and reflected after crossing");
}

#[test]
fn a_carried_javelin_leaves_the_scaled_holder_with_its_physical_material() {
    use ambition_platformer2d::item::{GroundItem, ItemCustody};
    use ambition_platformer2d::observation::SimId;
    fn run_held() -> Vec<Vec<u8>> {
        let mut sim = slice();
        let (holder, mut initial) = body(&mut sim);
        {
            use bevy::ecs::system::SystemState;
            use ambition_platformer2d::combat::hand::RepertoireQuery;
            let world = sim.world_mut();
            let mut state = SystemState::<(Commands, Query<RepertoireQuery>)>::new(world);
            {
                let (mut commands, mut bodies) = state.get_mut(world).unwrap();
                ambition_platformer2d::held_items::empty_hand(&mut commands, holder, &mut bodies.get_mut(holder).unwrap());
            }
            state.apply(world);
        }
        initial.material = ae::frame::BodyMaterial::new(0.75, Mat2::IDENTITY).unwrap();
        initial.pos.y = 896.0 - initial.half_extent().y;
        sim.world_mut().entity_mut(holder).insert((initial, ae::SweepSample::at_rest(initial, ae::Vec2::Y)));
        {
            let world = sim.world_mut();
            world.query::<&mut RoomGeometry>().single_mut(world).unwrap().0.blocks.retain(|b| b.name != "slice far contact");
        }
        let (item, id, mut item_body) = {
            let world = sim.world_mut();
            world.query::<(Entity, &GroundItem, &SimId, &BodyKinematics)>().iter(world)
                .find(|(_, item, _, kin)| item.spec.id == "javelin" && (1800.0..1900.0).contains(&kin.pos.x))
                .map(|(entity, _, id, kin)| (entity, id.clone(), *kin)).expect("station H has its authored javelin")
        };
        let item_size = item_body.canonical_size;
        item_body.pos = initial.pos;
        item_body.vel = ae::Vec2::ZERO;
        item_body.material = ae::frame::BodyMaterial::new(1.25, Mat2::from_angle(0.2)).unwrap();
        sim.world_mut().entity_mut(item).insert((item_body, ae::SweepSample::at_rest(item_body, ae::Vec2::Y)));
        sim.rebase_rollback_history().unwrap();
        sim.step_frame(ae::ControlFrame { attack_pressed: true, ..Default::default() });
        for _ in 0..6 { sim.step_frame(ae::ControlFrame::default()); }
        let (holder, _) = body(&mut sim);
        let world = sim.world_mut();
        let custody = world.query::<(&ItemCustody, &SimId)>().iter(world).find(|(_, candidate)| **candidate == id).unwrap().0;
        assert!(custody.held_by(holder), "pickup premise: {custody:?}, holder={holder:?}, body={:?}, item={:?}, hand={:?}, gun={:?}, driven={:?}",
            world.get::<BodyKinematics>(holder), world.get::<BodyKinematics>(item),
            world.get::<ambition_platformer2d::combat::held_items::HeldItem>(holder).map(|held| held.id()),
            world.get::<ambition_platformer2d::portal::PortalGenerator>(holder),
            world.get_resource::<ambition_platformer2d::platformer::markers::ControlledSubject>());
        let mut dropped = false;
        let mut released = 0;
        let mut trace = Vec::new();
        for _ in 0..130 {
            let (_, pose) = body(&mut sim);
            if !dropped && pose.material.scale() == 0.375 && pose.pos.x + pose.half_extent().x < 2448.0 {
                sim.step_frame(ae::ControlFrame { grab_pressed: true, ..Default::default() });
                dropped = true;
            } else {
                sim.step(AgentAction { move_x: if dropped { 0.0 } else { 1.0 }, ..crate::common::base() });
            }
            sim.rollback_health().unwrap();
            let world = sim.world_mut();
            let (item, kin, custody) = world.query::<(Entity, &BodyKinematics, &ItemCustody, &SimId)>().iter(world)
                .find(|(_, _, _, candidate)| **candidate == id).map(|(entity, kin, custody, _)| (entity, *kin, *custody)).unwrap();
            if custody.in_world() {
                assert!(dropped, "custody must end only after the drop request");
                assert_eq!(kin.material.scale(), 0.625);
                assert!(kin.material.reflected());
                assert_eq!(kin.canonical_size, item_size);
                assert!(world.get::<ae::BodyInStorage>(item).is_none());
                released += 1;
            }
            trace.push(ae::snapshot::encode_state(&kin));
        }
        assert!(released > 10, "the item must remain a scaled, reflected free body");
        assert!(sim.rollback_execution_stats().unwrap().load_runs > 0);
        trace
    }
    assert_eq!(run_held(), run_held());
}
