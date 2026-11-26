//! A world with no home avatar still moves.

use super::{MovingPlatformStep, install_moving_platform_simulation};
use crate::collision::MovingPlatformSet;
use crate::platforms::{LiftStage, MovingPlatformState};
use ambition_platformer2d_core as ae;
use bevy_app::{App, Update};
use bevy_ecs::prelude::*;

fn one_platform() -> MovingPlatformSet {
    MovingPlatformSet(vec![MovingPlatformState::from_authored(
        ae::Vec2::new(400.0, 800.0),
        ae::Vec2::new(155.0, 18.0),
        240.0,
        130.0,
    )])
}

fn app_with_one_platform(scaled_dt: f32) -> App {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime::new(1.0 / 60.0, scaled_dt));
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        one_platform(),
    );
    install_moving_platform_simulation(&mut app, Update);
    app
}

fn platform_x(app: &App) -> f32 {
    ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<MovingPlatformSet>(
        app.world(),
    )
    .expect("the fixture's live room has platforms")
    .0[0]
        .pos
        .x
}

/// Every live room advances, including rooms with no body or view.
#[test]
fn every_live_rooms_platforms_advance() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        LiveRoomInstance, RoomInstanceRoot, SessionRoot, activation_room_root,
        session_world_component,
    };
    let mut app = app_with_one_platform(1.0 / 60.0);
    let scope = session_world_component::<SessionRoot>(app.world())
        .expect("the fixture has a session root")
        .0;
    let elsewhere = (0..7).fold(LiveRoomInstance::ACTIVATION, |room, _| room.next());
    app.world_mut()
        .spawn((activation_room_root(scope), one_platform()))
        .insert(elsewhere);

    app.update();

    let world = app.world_mut();
    let mut rooms: Vec<(String, f32)> = world
        .query_filtered::<(&LiveRoomInstance, &MovingPlatformSet), With<RoomInstanceRoot>>()
        .iter(world)
        .map(|(live, platforms)| (live.to_string(), platforms.0[0].pos.x))
        .collect();
    rooms.sort_by(|a, b| a.0.cmp(&b.0));
    let moved = 400.0 + 130.0 / 60.0;
    assert_eq!(rooms.len(), 2, "the fixture has two live rooms: {rooms:?}");
    for (room, x) in &rooms {
        assert!(
            (x - moved).abs() < 1e-3,
            "live room {room}'s platform did not advance ({x}, expected {moved}): {rooms:?}"
        );
    }
}

/// Platform movement does not require a participant body.
#[test]
fn platforms_advance_in_a_world_with_no_player_body() {
    let mut app = app_with_one_platform(1.0 / 60.0);
    let before = platform_x(&app);
    app.update();
    let after = platform_x(&app);

    assert!(
        (after - before - 130.0 / 60.0).abs() < 1e-3,
        "a platform in a world with no home avatar must travel its own speed \
         ({before} -> {after}); anything that consults a player body to decide \
         whether the WORLD may move is frozen in every match"
    );
    let world = app.world_mut();
    let mut primaries = world.query_filtered::<Entity, With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>>();
    assert_eq!(
        primaries.iter(world).count(),
        0,
        "the fixture must contain no primary player, or it is not testing the \
         condition that broke"
    );
}

/// The simulation clock controls platform movement.
#[test]
fn a_stopped_clock_stops_them() {
    let mut app = app_with_one_platform(0.0);
    let before = platform_x(&app);
    app.update();
    assert_eq!(
        platform_x(&app),
        before,
        "a zero sim dt must hold the platform still — that is how hitstop, pause \
         and bullet-time reach world geometry"
    );
}

#[test]
fn an_ordinary_grounded_body_starts_a_lift() {
    let mut app = app_with_one_platform(1.0 / 60.0);
    let lift = MovingPlatformState::lift(
        "lift",
        "Lift",
        ae::Vec2::new(400.0, 800.0),
        ae::Vec2::new(155.0, 18.0),
        -100.0,
        60.0,
    );
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        MovingPlatformSet(vec![lift]),
    );
    for _ in 0..2 {
        app.update();
    }
    let height = |app: &App| {
        ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<MovingPlatformSet>(app.world())
            .unwrap().0[0].pos.y
    };
    assert_eq!(height(&app), 800.0, "an empty lift waits for a body");
    app.world_mut().spawn((
        ae::BodyKinematics {
            pos: ae::Vec2::new(400.0, 767.0),
            canonical_size: ae::Vec2::new(30.0, 48.0),
            ..Default::default()
        },
        ae::BodyGroundState {
            on_ground: true,
            ..Default::default()
        },
    ));
    for _ in 0..2 {
        app.update();
    }
    assert!(
        height(&app) < 800.0,
        "a lift must respond to a grounded body without a participant controller"
    );
}

fn lift_app() -> App {
    let mut app = app_with_one_platform(1.0 / 60.0);
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        MovingPlatformSet(vec![MovingPlatformState::lift(
            "lift",
            "Lift",
            ae::Vec2::new(400.0, 800.0),
            ae::Vec2::new(155.0, 18.0),
            -100.0,
            60.0,
        )]),
    );
    app
}

fn lift_y(app: &App) -> f32 {
    ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<MovingPlatformSet>(
        app.world(),
    )
    .unwrap()
    .0[0]
        .pos
        .y
}

#[test]
fn lift_contacts_use_the_body_frame_and_collision_plane() {
    for depth in [ae::DepthPlane::PLAYABLE, ae::DepthPlane(1)] {
        let mut app = lift_app();
        let kin = ae::BodyKinematics {
            material: ae::frame::BodyMaterial::IDENTITY.turned(-std::f64::consts::FRAC_PI_2).unwrap(),
            pos: ae::Vec2::new(298.5, 800.0),
            canonical_size: ae::Vec2::new(30.0, 48.0),
            ..Default::default()
        };
        app.world_mut().spawn((
            kin,
            ae::SweepSample::at_rest(kin, ae::Vec2::X),
            ae::BodyGroundState {
                on_ground: true,
                ..Default::default()
            },
            depth,
        ));
        for _ in 0..2 {
            app.update();
        }
        assert_eq!(lift_y(&app) < 800.0, depth.is_playable(), "depth {depth:?}");
    }
}

#[test]
fn a_body_starts_only_its_own_live_rooms_lift() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        InRoomInstance, LiveRoomInstance, RoomInstanceRoot, spawn_live_room,
    };
    let mut app = lift_app();
    let elsewhere = LiveRoomInstance::ACTIVATION.next();
    spawn_live_room(
        app.world_mut(),
        elsewhere,
        MovingPlatformSet(vec![MovingPlatformState::lift(
            "other_lift",
            "Other lift",
            ae::Vec2::new(400.0, 800.0),
            ae::Vec2::new(155.0, 18.0),
            -100.0,
            60.0,
        )]),
    );
    app.world_mut().spawn((
        ae::BodyKinematics {
            pos: ae::Vec2::new(400.0, 767.0),
            canonical_size: ae::Vec2::new(30.0, 48.0),
            ..Default::default()
        },
        ae::BodyGroundState {
            on_ground: true,
            ..Default::default()
        },
        InRoomInstance(LiveRoomInstance::ACTIVATION),
    ));
    for _ in 0..2 {
        app.update();
    }
    let world = app.world_mut();
    let mut rooms =
        world.query_filtered::<(&LiveRoomInstance, &MovingPlatformSet), With<RoomInstanceRoot>>();
    for (room, platforms) in rooms.iter(world) {
        assert_eq!(
            platforms.0[0].pos.y < 800.0,
            *room == LiveRoomInstance::ACTIVATION,
            "room {room:?}"
        );
    }
}

#[derive(Resource, Default)]
struct ObservedPlatformPositions(Vec<f32>);

fn observe_platform_geometry(
    collision: crate::collision::CollisionWorld,
    mut observed: ResMut<ObservedPlatformPositions>,
) {
    let world = collision.solids().expect("the fixture has one live room");
    observed.0.push(
        world
            .blocks
            .first()
            .expect("the room has a platform")
            .aabb
            .min
            .x,
    );
}

#[test]
fn the_world_step_retires_old_walls_before_contact_readers() {
    let mut app = app_with_one_platform(1.0 / 60.0);
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new(
            "room",
            ae::Vec2::splat(1000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        )),
    );
    app.init_resource::<ObservedPlatformPositions>();
    app.add_systems(
        Update,
        (
            observe_platform_geometry.before(MovingPlatformStep),
            observe_platform_geometry.after(MovingPlatformStep),
        ),
    );
    app.update();
    let positions = &app.world().resource::<ObservedPlatformPositions>().0;
    assert_eq!(positions.len(), 2);
    assert!(
        positions[1] > positions[0],
        "readers after the step see the platform's new pose"
    );
    assert_eq!(
        app.world()
            .resource::<crate::collision::ComposedSolids>()
            .kept(),
        1,
        "the cache keeps the current room geometry and releases its old pose"
    );
}

/// A switch-called lift waits at both stops: a rider does not start it, a
/// send takes it to the far stop and it parks there, the next send brings it
/// back, and a send while it travels is refused.
#[test]
fn a_called_lift_goes_only_when_sent_and_waits_at_each_stop() {
    let mut lift = MovingPlatformState::lift(
        "lift",
        "Lift",
        ae::Vec2::new(400.0, 800.0),
        ae::Vec2::new(155.0, 18.0),
        -100.0,
        600.0,
    )
    .sent_by_switch();
    let dt = 1.0 / 60.0;
    lift.set_ridden(true);
    for _ in 0..30 {
        lift.update(dt);
    }
    assert_eq!(lift.pos.y, 800.0, "a rider started a lift only a switch sends");
    assert!(lift.send(), "a resting called lift refused a send");
    lift.update(dt);
    assert!(!lift.send(), "a travelling lift took a second send");
    for _ in 0..30 {
        lift.update(dt);
    }
    assert_eq!((lift.pos.y, lift.lift_stage()), (700.0, Some(LiftStage::Parked)), "the lift did not park at its far stop");
    assert!(lift.send(), "a parked lift refused a send");
    for _ in 0..30 {
        lift.update(dt);
    }
    assert_eq!((lift.pos.y, lift.lift_stage()), (800.0, Some(LiftStage::Resting)), "the lift did not come back to rest");
    let mut rider_lift =
        MovingPlatformState::lift("l", "L", ae::Vec2::new(0.0, 0.0), ae::Vec2::new(10.0, 10.0), -100.0, 60.0);
    assert!(!rider_lift.send(), "a rider's lift took a switch's send");
}

/// Crushing C3: a press descending at 6 px a tick onto a body that stands on
/// a floor. `authority` is the press's own.
fn press_app(authority: Option<ae::crush::SurfaceAuthority>) -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime::new(1.0 / 60.0, 1.0 / 60.0));
    // The press: 200 wide, 20 tall, its underside at y 100, falling 360 px/s.
    let mut press = MovingPlatformState::from_vertical_loop("press", "Press",
        ae::Vec2::new(400.0, 90.0), ae::Vec2::new(200.0, 20.0), 0.0, 1000.0, 360.0, true);
    press.authority = authority;
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(),
        MovingPlatformSet(vec![press]));
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(),
        ae::RoomGeometry(ae::World::new("press room", ae::Vec2::new(1600.0, 900.0), ae::Vec2::ZERO,
            vec![ae::Block::solid("floor", ae::Vec2::new(0.0, 160.0), ae::Vec2::new(1600.0, 40.0))])));
    install_moving_platform_simulation(&mut app, Update);
    // A 48-tall body on the floor: its head at y 112, 12 px under the press.
    let body = app.world_mut().spawn(ae::BodyKinematics {
        pos: ae::Vec2::new(400.0, 136.0), canonical_size: ae::Vec2::new(24.0, 48.0), ..Default::default()
    }).id();
    (app, body)
}

fn press(app: &App) -> MovingPlatformState {
    ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<MovingPlatformSet>(app.world())
        .unwrap().0[0].clone()
}

/// A yielding press stops where it meets the body, which has no room to give:
/// two ticks of 6 px, then 0 px of the third (its underside on the body's
/// head at y 112), and it holds there. What it records is the move it made,
/// so a rider on it is carried by that. The body is untouched.
#[test]
fn a_yielding_press_stops_on_a_body_under_it() {
    let (mut app, body) = press_app(Some(ae::crush::SurfaceAuthority::Yielding));
    let mut deltas = Vec::new();
    for _ in 0..5 {
        app.update();
        deltas.push(press(&app).last_delta().y);
    }
    let underside = press(&app).aabb().max.y;
    assert!((underside - 112.0).abs() < 1e-3, "it rests on the body's head: {underside}, deltas {deltas:?}");
    assert!((deltas[0] - 6.0).abs() < 1e-3 && (deltas[1] - 6.0).abs() < 1e-3, "{deltas:?}");
    assert!(deltas[3].abs() < 1e-3 && deltas[4].abs() < 1e-3, "it holds: {deltas:?}");
    assert_eq!(app.world().get::<ae::BodyKinematics>(body).unwrap().pos, ae::Vec2::new(400.0, 136.0));
}

/// Control: an unstoppable press (the default) keeps its schedule.
#[test]
fn an_unstoppable_press_keeps_its_schedule() {
    let (mut app, _) = press_app(None);
    for _ in 0..5 { app.update(); }
    assert!((press(&app).aabb().max.y - 130.0).abs() < 1e-3, "{:?}", press(&app).aabb());
}

/// A limited press pushes up to its allowance past the body's room, then
/// holds.
#[test]
fn a_limited_press_pushes_to_its_limit_then_holds() {
    let (mut app, _) = press_app(Some(ae::crush::SurfaceAuthority::Limited { max_overconstraint_px: 4.0, max_force: 1.0 }));
    for _ in 0..6 { app.update(); }
    assert!((press(&app).aabb().max.y - 116.0).abs() < 1e-3, "4 px into the body: {:?}", press(&app).aabb());
}

/// A stalled press goes on when the body leaves, and a rewind of the platform
/// set replays the same stall.
#[test]
fn a_stalled_press_resumes_when_the_body_leaves_and_a_rewind_replays_it() {
    let (mut app, body) = press_app(Some(ae::crush::SurfaceAuthority::Yielding));
    app.update();
    let saved = ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<MovingPlatformSet>(app.world())
        .unwrap().to_snapshot_ron();
    for _ in 0..4 { app.update(); }
    let stalled = press(&app);
    // Rewind the platforms to the first tick and replay: the same stall.
    let restored = MovingPlatformSet::from_snapshot_ron(&saved).unwrap();
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), restored);
    for _ in 0..4 { app.update(); }
    assert_eq!(press(&app), stalled, "the rewind replays the stall");
    app.world_mut().despawn(body);
    app.update();
    assert!((press(&app).last_delta().y - 6.0).abs() < 1e-3, "it goes on: {:?}", press(&app).last_delta());
}
