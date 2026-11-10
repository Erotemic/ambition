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
        let moving = frame < 50 || (60..90).contains(&frame) || frame >= 120;
        sim.step(AgentAction { move_x: if moving { 1.0 } else { 0.0 }, ..crate::common::base() });
        sim.rollback_health().unwrap_or_else(|error| panic!("frame {frame}: {error}"));
        let (_, pose) = body(&mut sim);
        if frame == 50 { stationary = Some(pose.pos); }
        if (51..60).contains(&frame) {
            assert_eq!(Some(pose.pos), stationary, "a stationary partial body must keep its pose");
            assert!(sim.observation().on_ground, "a stationary partial body must keep its support");
        }
        if pose.material.scale() == 0.5 {
            shrunk += 1;
            assert!(pose.material.reflected(), "frame {frame}: {pose:?}, convention {:?}",
                sim.world().resource::<PortalTuning>().convention);
            if frame < 90 { assert_eq!(pose.facing, initial.facing, "held steering must retain material facing through a reflection"); }
            assert_eq!(pose.canonical_size, initial.canonical_size);
            assert_eq!(pose.size(), initial.size() * 0.5);
            assert!(pose.material.orthogonal().abs_diff_eq(Mat2::from_cols(Vec2::NEG_X, Vec2::Y), 1.0e-5));
        }
        if shrunk > 0 && pose.material.scale() == 1.0 {
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
    let room = app.world().get::<ambition_platformer2d::platformer::lifecycle::InRoomInstance>(player).map(|room| room.0);
    let links = app.world().resource::<ambition_platformer2d::platformer::body::geometry::BodyGeometry>().links_in(room).len();
    let probe = ambition_platformer2d::world::collision::with_room_in_world(app.world(),
        app.world().get::<ambition_platformer2d::platformer::lifecycle::InRoomInstance>(player), |scene| {
            let walls = scene.solids()?;
            let kin = *app.world().get::<BodyKinematics>(player)?;
            let domain = kin.canonical_box().ok()?;
            let geometry = app.world().resource::<ambition_platformer2d::platformer::body::geometry::BodyGeometry>();
            let cast = ae::frame::material_cast::MaterialCast::new(ae::frame::thread::MaterialPrimitive {
                domain: &domain, to_world: kin.material_map().ok()? }, kin.pos, Vec2::X * 10.0, 0.0..1.0);
            Some(format!("{:?}", walls.first_material_sweep(cast, geometry.motion_for(player, room), |_, _, _| Ok(true))
                .map(|hit| hit.map(|hit| (hit.collider.source(), hit.contact)))))
        });
    assert!(samples > 20, "probe {probe:?}; contacts {:?}; the rendered body must stay small and reflected after crossing: {:?}, room {room:?}, links {links}, refusal {:?}",
        app.world().get::<ambition_platformer2d::actors::body::BodyFrameOutput>(player).map(|output| &output.events.contacts),
        app.world().get::<BodyKinematics>(player), app.world().get::<ambition_platformer2d::actors::body::BodyFrameOutput>(player).map(|output| &output.events.motion_refusal));
}
