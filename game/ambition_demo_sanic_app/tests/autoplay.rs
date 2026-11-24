//! The profiling routes use the normal input and goal rules.

use ambition_demo_sanic::{
    DARKNESS_ROOM_ID, HIGHWAY_ROOM_ID, SPEEDWAY_ROOM_ID,
    autoplay::{ACT3_PROFILE, ALL_ACTS_PROFILE},
};
use ambition_platformer2d::{engine_core as ae, scripted_input};
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Progress {
    portals: usize,
    clears: usize,
    replays: usize,
}

fn observe(
    mut seen: ResMut<Progress>,
    mut portals: MessageReader<ambition_platformer2d::portal::PortalBodyTransited>,
    mut replays: MessageReader<ambition_platformer2d::combat::RoomReplayAdmitted>,
    acts: Query<&ambition_demo_sanic::SanicActState>,
) {
    seen.portals += portals.read().count();
    seen.replays += replays.read().count();
    if acts.iter().any(|act| {
        matches!(
            act.phase,
            ambition_demo_sanic::SanicActPhase::Cleared { .. }
        )
    }) {
        seen.clears += 1;
    }
}

#[test]
fn the_full_profile_clears_each_act_in_order() {
    let mut app = ambition_demo_sanic_app::build_demo_app();
    scripted_input::install_autoplay(&mut app, ALL_ACTS_PROFILE);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 30.0),
    ));
    app.init_resource::<Progress>();
    app.add_systems(Last, observe);
    let mut cleared = Vec::new();
    let mut acts = app.world_mut().query::<&ambition_demo_sanic::SanicActState>();
    for _ in 0..3600 {
        app.update();
        let Some(room) = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world()) else {
            continue;
        };
        if matches!(acts.single(app.world()).map(|act| act.phase),
            Ok(ambition_demo_sanic::SanicActPhase::Cleared { .. }))
            && cleared.last() != Some(&room.id)
        {
            cleared.push(room.id.clone());
            if cleared.len() == 3 {
                break;
            }
        }
    }
    assert_eq!(cleared, [SPEEDWAY_ROOM_ID, HIGHWAY_ROOM_ID, DARKNESS_ROOM_ID]);
    assert!(app.world().resource::<Progress>().portals > 0);
}

#[test]
fn the_act_three_profile_runs_through_a_portal_and_repeats_act_three() {
    let mut app = ambition_demo_sanic_app::build_demo_app();
    scripted_input::install_autoplay(&mut app, ACT3_PROFILE);
    app.init_resource::<Progress>();
    app.add_systems(Last, observe);
    let mut entered = false;
    for _ in 0..900 {
        app.update();
        if ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
            .is_some_and(|room| room.id == DARKNESS_ROOM_ID)
        {
            entered = true;
            break;
        }
        let mut modes = app.world_mut().query_filtered::<&mut ambition_platformer2d::session::Departure,
            With<ambition_demo_sanic::SanicActState>>();
        for mut departure in modes.iter_mut(app.world_mut()) {
            departure.leave(ambition_platformer2d::session::Destination::Room(
                DARKNESS_ROOM_ID.into(),
            ));
        }
    }
    assert!(entered, "the profile fixture must enter Act 3");
    let mut finished = false;
    for _ in 0..9000 {
        app.update();
        let Some(room) = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
        else {
            continue;
        };
        assert_eq!(room.id, DARKNESS_ROOM_ID);
        let seen = app.world().resource::<Progress>();
        if seen.portals >= 2 && seen.clears > 0 && seen.replays >= 2 {
            finished = true;
            break;
        }
    }
    let seen = app.world().resource::<Progress>();
    assert!(
        finished,
        "the profile did not complete: portals={} clears={} replays={}",
        seen.portals, seen.clears, seen.replays
    );
    for _ in 0..120 {
        app.update();
    }
    let room = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world()).unwrap();
    assert_eq!(room.id, DARKNESS_ROOM_ID);
    let spawn = room.world.spawn;
    let mut bodies = app.world_mut().query_filtered::<&ae::BodyKinematics,
        With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    let body = bodies.single(app.world()).unwrap();
    assert!(
        body.pos.is_finite() && body.vel.is_finite(),
        "body after replay: {body:?}"
    );
    assert!(
        body.pos.x > spawn.x + body.size().x,
        "the replay must resume the route: {body:?}"
    );
    assert_eq!(
        app.world()
            .resource::<scripted_input::ScriptedControls>()
            .0
            .axis_x,
        1.0
    );
}
