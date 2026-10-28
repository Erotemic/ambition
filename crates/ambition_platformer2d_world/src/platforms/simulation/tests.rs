//! A world with no home avatar still moves.

use super::{MovingPlatformStep, install_moving_platform_simulation};
use crate::collision::MovingPlatformSet;
use crate::platforms::MovingPlatformState;
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
            size: ae::Vec2::new(30.0, 48.0),
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
            pos: ae::Vec2::new(298.5, 800.0),
            size: ae::Vec2::new(30.0, 48.0),
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
            size: ae::Vec2::new(30.0, 48.0),
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
