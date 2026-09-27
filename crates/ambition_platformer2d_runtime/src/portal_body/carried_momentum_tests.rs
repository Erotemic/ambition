//! The carried run a portal transfer leaves on a body is measured along THAT
//! body's own run axis.

use bevy::prelude::*;

use ambition_platformer2d_core::{BodyFlightState, BodyKinematics, MotionFrame};
use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
use ambition_portal2d::PortalBodyTransited;

/// Fall into a floor portal (outward normal up), leave a wall portal facing +x:
/// the classic fling. The exit velocity is the entry speed turned onto +x.
const ENTER_NORMAL: Vec2 = Vec2::new(0.0, -1.0);
const EXIT_NORMAL: Vec2 = Vec2::new(1.0, 0.0);
const SPEED: f32 = 500.0;

fn carried_run_after_fling(body_down: Vec2) -> f32 {
    let mut app = App::new();
    app.add_message::<PortalBodyTransited>();
    app.insert_resource(ambition_portal2d::PortalTuning::default());
    // The primary body's gravity is the default (down). The body under test
    // must not be measured against it.
    app.init_resource::<ambition_platformer2d_shared_tangle::gravity::GravityField>();
    app.add_systems(Update, super::apply_portal_carried_momentum);

    let mut frame = ResolvedMotionFrame::default();
    frame.publish_resolved_frame(MotionFrame::from_direction(body_down, 1000.0));
    let body = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: Vec2::new(100.0, 100.0),
                vel: EXIT_NORMAL * SPEED,
                size: Vec2::new(16.0, 32.0),
                facing: 1.0,
            },
            frame,
            BodyFlightState::default(),
        ))
        .id();
    app.world_mut().write_message(PortalBodyTransited {
        body,
        enter_normal: ENTER_NORMAL,
        exit_normal: EXIT_NORMAL,
        facing_flip: false,
        input_warp: false,
        exit_pos: Vec2::new(100.0, 100.0),
    });
    app.update();
    app.world().get::<BodyFlightState>(body).unwrap().carried_run
}

/// Control: under ordinary down gravity the fall speed turned sideways IS run
/// the body did not steer, so it is carried in full.
#[test]
fn a_fling_under_ordinary_gravity_carries_the_whole_exit_speed() {
    let carried = carried_run_after_fling(Vec2::new(0.0, 1.0));
    assert!(
        (carried.abs() - SPEED).abs() < 1e-3,
        "the fling was not conserved: carried_run = {carried}"
    );
}

/// A body whose own gravity points +x leaves the wall portal travelling along
/// ITS down axis: that is fall, not run, so nothing is carried. Measured on the
/// primary body's axis instead, the same exit reads as a full-speed fling.
#[test]
fn a_body_under_sideways_gravity_carries_no_run_from_a_fall_along_its_own_down() {
    let carried = carried_run_after_fling(Vec2::new(1.0, 0.0));
    assert!(
        carried.abs() < 1e-3,
        "a fall along the body's own gravity became carried run: {carried}"
    );
}

/// A game with portals gets the carried momentum from `PortalSchedulePlugin`
/// alone: a body that falls into a floor portal and leaves a wall portal
/// carries the fling as run it did not steer.
#[test]
fn portal_schedule_plugin_carries_a_fling_for_any_game() {
    use ambition_platformer2d_shared_tangle::schedule::{GameMode, SimScheduleExt};
    use ambition_portal2d::{portal_half_extent, PlacedPortal, PortalChannel, PortalGunColor};

    let mut app = App::new();
    app.set_sim_schedule(Update);
    app.init_resource::<ambition_platformer2d_shared_tangle::time::SimDt>();
    app.init_resource::<ambition_characters::control::SeatRawFrames>();
    app.init_resource::<ambition_characters::control::SlotControls>();
    app.insert_resource(State::new(GameMode::Playing));
    app.add_plugins(crate::PortalSchedulePlugin);
    for (color, pos, normal) in [
        (PortalGunColor::BLUE, Vec2::new(100.0, 300.0), ENTER_NORMAL),
        (PortalGunColor::ORANGE, Vec2::new(20.0, 600.0), EXIT_NORMAL),
    ] {
        app.world_mut().spawn(PlacedPortal::fixed(
            PortalChannel::Gun(color),
            pos,
            normal,
            portal_half_extent(normal),
        ));
    }
    let mut frame = ResolvedMotionFrame::default();
    frame.publish_resolved_frame(MotionFrame::from_direction(Vec2::new(0.0, 1.0), 1000.0));
    let body = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: Vec2::new(100.0, 300.0),
                vel: Vec2::new(0.0, SPEED),
                size: Vec2::new(16.0, 32.0),
                facing: 1.0,
            },
            frame,
            BodyFlightState::default(),
        ))
        .id();

    let mut arrived = None;
    for _ in 0..4 {
        app.update();
        let kin = *app.world().get::<BodyKinematics>(body).unwrap();
        if kin.pos.x < 60.0 {
            arrived = Some(kin);
            break;
        }
    }
    let kin = arrived.expect("the body must fall through the floor portal and leave the wall portal");
    assert!(kin.vel.x > 0.0, "it leaves the wall portal moving out of it, vel={:?}", kin.vel);
    let carried = app.world().get::<BodyFlightState>(body).unwrap().carried_run;
    assert!(
        (carried - kin.vel.x).abs() < 1e-3,
        "the fling is carried run it did not steer: carried_run = {carried}, exit vel = {:?}",
        kin.vel
    );
}
