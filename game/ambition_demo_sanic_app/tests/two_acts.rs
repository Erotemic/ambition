//! The two acts are one course: the speedway's goal leads to the highway, the
//! highway's back. A scripted headless run holds Right through both.
//!
//! Act 2 exercises what Act 1 cannot: four loops authored as LDtk data
//! (`SurfaceLoop` + `attach_to`), springs a runner at full speed can use, and
//! an upside-down tunnel where a `GravityZone` points gravity up and the
//! momentum solver rides the ceiling.

use ambition_demo_sanic::{SanicActPhase, SanicActState, HIGHWAY_ROOM_ID, SPEEDWAY_ROOM_ID};
use ambition_demo_sanic_app::build_demo_app;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

fn body(app: &mut App) -> (ae::BodyKinematics, Option<ae::SurfaceMotion>) {
    let mut q = app.world_mut().query_filtered::<(&ae::BodyKinematics, &ae::MotionModel), With<PrimaryPlayer>>();
    q.iter(app.world())
        .next()
        .map(|(kin, model)| {
            let motion = match model {
                ae::MotionModel::SurfaceMomentum(momentum) => Some(momentum.state),
                _ => None,
            };
            (*kin, motion)
        })
        .expect("the demo spawned Sanic")
}

fn act(app: &mut App) -> SanicActState {
    let mut q = app.world_mut().query::<&SanicActState>();
    q.iter(app.world()).next().copied().expect("the act owner exists")
}

fn room(app: &mut App) -> (String, Option<usize>) {
    let mut q = app.world_mut().query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let set = q.iter(app.world()).next().expect("the session's room set");
    (
        set.active_spec().id.clone(),
        set.active_world().chain_named("highway_tunnel_ceiling"),
    )
}

fn hold(app: &mut App, jump: bool) {
    let mut frame = ControlFrame::default();
    frame.axis_x = 1.0;
    frame.right_pressed = true;
    frame.jump_pressed = jump;
    frame.jump_held = jump;
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
}

fn release(app: &mut App) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = ControlFrame::default();
}

/// Run until the act clears, or panic naming how far he got.
fn run_to_clear(app: &mut App, act_name: &str, frames: usize, jump_at: impl Fn(f32) -> bool) -> usize {
    let mut furthest = f32::MIN;
    for frame in 0..frames {
        let x = body(app).0.pos.x;
        furthest = furthest.max(x);
        hold(app, jump_at(x));
        app.update();
        if matches!(act(app).phase, SanicActPhase::Cleared { .. }) {
            return frame;
        }
    }
    panic!("{act_name}: held Right for {frames} frames and never cleared; furthest x {furthest:.0}");
}

/// Release the stick and wait for the named room to become active.
fn arrive_in(app: &mut App, room_id: &str) {
    for _ in 0..900 {
        release(app);
        app.update();
        if room(app).0 == room_id {
            return;
        }
    }
    panic!("the cleared act never arrived in `{room_id}` (still in `{}`)", room(app).0);
}

#[test]
fn the_speedway_leads_to_the_highway_and_the_highway_back() {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(room(&mut app).0, SPEEDWAY_ROOM_ID, "the session enters at Act 1");

    // Act 1. It asks for a jump at its pit, as its own completion proof does.
    run_to_clear(&mut app, "Act 1", 2400, |x| {
        x > ambition_demo_sanic::PIT_LEFT_X - 220.0 && x < ambition_demo_sanic::PIT_RIGHT_X
    });
    arrive_in(&mut app, HIGHWAY_ROOM_ID);
    let arrived = act(&mut app);
    assert!(
        matches!(arrived.phase, SanicActPhase::Running) && arrived.elapsed < 0.5,
        "arriving in Act 2 starts a fresh act, not Act 1's results: {arrived:?}"
    );
    assert!(body(&mut app).0.pos.x < 400.0, "and at Act 2's start line");

    // Act 2, Right only: its springs carry a full-speed runner over both pits.
    let tunnel = room(&mut app).1.expect("the highway authors its tunnel ceiling");
    let mut rode_the_ceiling = false;
    let mut furthest = f32::MIN;
    let mut cleared = false;
    for _ in 0..3000 {
        let (kin, motion) = body(&mut app);
        furthest = furthest.max(kin.pos.x);
        if matches!(motion, Some(ae::SurfaceMotion::Riding { on: ae::SurfaceRef::Chain(chain), .. }) if chain == tunnel)
        {
            rode_the_ceiling = true;
        }
        hold(&mut app, false);
        app.update();
        if matches!(act(&mut app).phase, SanicActPhase::Cleared { .. }) {
            cleared = true;
            break;
        }
    }
    assert!(cleared, "Act 2: held Right and never cleared; furthest x {furthest:.0}");
    assert!(
        rode_the_ceiling,
        "inside the tunnel the GravityZone points gravity up, so he must ride its ceiling chain"
    );

    arrive_in(&mut app, SPEEDWAY_ROOM_ID);
    assert!(
        matches!(act(&mut app).phase, SanicActPhase::Running),
        "and the highway's goal leads back to a fresh Act 1"
    );
}
