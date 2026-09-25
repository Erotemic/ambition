//! Act 2's roads and secrets, played: each one is reached the way a player
//! reaches it, in the real headless app.
//!
//! - The valley (the slow road under the chasm) leads back to the course.
//! - A spin dash into the cliff at the valley's west end breaks the wall and
//!   rolls on through the cave's monitors.
//! - A jump to the ledge over the plateau finds a spring that throws him onto
//!   the sky island and its ring monitor.

use ambition_demo_sanic::monitors::SpentMonitors;
use ambition_demo_sanic::{SanicActPhase, SanicActState, HIGHWAY_ROOM_ID};
use ambition_demo_sanic_app::build_demo_app;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

fn player(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<PrimaryPlayer>>();
    q.iter(app.world()).next().expect("Sanic is in the world")
}

fn pos(app: &mut App) -> Vec2 {
    let body = player(app);
    app.world().get::<ae::BodyKinematics>(body).unwrap().pos
}

fn act(app: &mut App) -> SanicActState {
    let mut q = app.world_mut().query::<&SanicActState>();
    q.iter(app.world()).next().copied().expect("the act owner exists")
}

fn active_room(app: &mut App) -> String {
    let mut q = app.world_mut().query::<&ambition_platformer2d::world::rooms::RoomSet>();
    q.iter(app.world()).next().expect("the room set").active_spec().id.clone()
}

fn spent(app: &mut App) -> Vec<String> {
    app.world().resource::<SpentMonitors>().0.clone()
}

fn press(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
}

fn stick(x: f32, jump: bool, down: bool, rev: bool) -> ControlFrame {
    ControlFrame {
        axis_x: x,
        axis_y: if down { 1.0 } else { 0.0 },
        right_pressed: x > 0.0,
        left_pressed: x < 0.0,
        down_pressed: down,
        jump_pressed: jump,
        jump_held: jump,
        attack_pressed: rev,
        attack_held: rev,
        ..Default::default()
    }
}

/// Boot the demo and play Act 1 to its clear, arriving in Act 2.
fn arrive_in_act_two() -> App {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..630 {
        app.update();
    }
    for _ in 0..2400 {
        let x = pos(&mut app).x;
        let jump = x > ambition_demo_sanic::PIT_LEFT_X - 220.0 && x < ambition_demo_sanic::PIT_RIGHT_X;
        press(&mut app, stick(1.0, jump, false, false));
        app.update();
        if matches!(act(&mut app).phase, SanicActPhase::Cleared { .. }) {
            break;
        }
    }
    for _ in 0..900 {
        press(&mut app, ControlFrame::default());
        app.update();
        if active_room(&mut app) == HIGHWAY_ROOM_ID {
            break;
        }
    }
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(active_room(&mut app), HIGHWAY_ROOM_ID, "Act 1 led to Act 2");
    app
}

/// Put Sanic somewhere in Act 2 through the discrete-transit authority.
fn place(app: &mut App, at: Vec2) {
    let mut q = app.world_mut().query_filtered::<(
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
    ), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let (mut clusters, mut model) = q.iter_mut(world).next().unwrap();
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

#[test]
fn the_valley_road_leads_back_to_the_course() {
    let mut app = arrive_in_act_two();
    place(&mut app, Vec2::new(5400.0, 1400.0));
    let mut furthest = f32::MIN;
    for _ in 0..600 {
        press(&mut app, stick(1.0, false, false, false));
        app.update();
        let at = pos(&mut app);
        furthest = furthest.max(at.x);
        if at.x > 8000.0 && at.y < 1250.0 {
            return;
        }
    }
    panic!("from the valley, holding Right never reached the course past the tunnel mouth (furthest x {furthest:.0})");
}

#[test]
fn a_spin_dash_into_the_cliff_opens_the_cave() {
    let mut app = arrive_in_act_two();
    place(&mut app, Vec2::new(5000.0, 1400.0));
    for frame in 0..240 {
        let input = match frame {
            // Face the cliff.
            0..12 => stick(-1.0, false, false, false),
            // Crouch and rev three times.
            12..60 => stick(0.0, false, true, frame % 8 == 0),
            // Release: the dash.
            _ => ControlFrame::default(),
        };
        press(&mut app, input);
        app.update();
    }
    let spent = spent(&mut app);
    for block in ["breakable_cave_wall", "monitor_rings_cave", "monitor_speed_cave"] {
        assert!(
            spent.iter().any(|name| name == block),
            "the dash breaks `{block}`: spent {spent:?}, Sanic at {:?}",
            pos(&mut app)
        );
    }
    assert!(
        pos(&mut app).x < 4400.0 && pos(&mut app).y > 1300.0,
        "and he is in the cave, under the rock: {:?}",
        pos(&mut app)
    );
}

#[test]
fn the_ledge_spring_throws_him_onto_the_sky_island() {
    let mut app = arrive_in_act_two();
    place(&mut app, Vec2::new(12960.0, 960.0));
    for _ in 0..240 {
        let at = pos(&mut app);
        // An easy run at the ledge, and a jump up to it.
        let jump = at.x > 13090.0 && at.x < 13130.0 && at.y > 960.0;
        press(&mut app, stick(if at.x < 13300.0 { 0.4 } else { 0.0 }, jump, false, false));
        app.update();
        if spent(&mut app).iter().any(|name| name == "monitor_rings_sky") {
            return;
        }
    }
    panic!(
        "the ledge's spring never put him on the sky island's monitor: spent {:?}, at {:?}",
        spent(&mut app),
        pos(&mut app)
    );
}
