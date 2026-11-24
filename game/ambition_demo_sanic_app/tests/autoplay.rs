//! The Act 3 profile crosses a portal and replays the same act after its goal.

use ambition_demo_sanic::{DARKNESS_ROOM_ID, autoplay::ACT3_PROFILE};
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
        if seen.portals > 0 && seen.clears > 0 && seen.replays > 0 {
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
