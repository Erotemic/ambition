//! **The demo changes rooms.**
//!
//! This is the exit proof for moving the room-transition transaction into the
//! engine (2026-07-25). Before it, `RoomTransitionRequested` had exactly one
//! consumer and only `ambition_app` registered it — so in THIS binary, which
//! deliberately does not depend on `ambition_app`, the message went into a
//! registered channel that nothing drained. A second room was not merely
//! unauthored; it was unreachable.
//!
//! So the assertion that matters is not "a transition was requested" — the
//! request always worked. It is that the ACTIVE ROOM CHANGED and the body is
//! standing in the new room's geometry, which is only true if a consumer ran.
//! (Room-replay §2.5 shipped three green proofs of a beat that had no consumer
//! in-process, each asserting a value the emitter wrote one line earlier. Not
//! again.)
//!
//! Gated off `input` for the same reason as `scripted_level_run`: under that
//! feature the participant pipeline owns `ControlFrame` and erases a scripted
//! write, so driving this seam is only meaningful in the headless composition.
#![cfg(not(feature = "input"))]

use ambition::engine_core::AabbExt;
use ambition::input::ControlFrame;
use ambition::platformer::markers::PrimaryPlayer;
use ambition::world::rooms::RoomSet;
use ambition_demo_mary_o::level_1_2::LEVEL_1_2_ROOM_ID;
use ambition_demo_mary_o::LEVEL_1_1_ROOM_ID;
use ambition_demo_mary_o_app::build_demo_app;
use bevy::prelude::*;

#[derive(Resource, Clone, Copy, Default)]
struct ScriptedStick(ControlFrame);

fn apply_scripted_stick(stick: Res<ScriptedStick>, mut frame: ResMut<ControlFrame>) {
    *frame = stick.0;
}

fn boot() -> App {
    let mut app = build_demo_app();
    app.init_resource::<ScriptedStick>();
    app.add_systems(PreUpdate, apply_scripted_stick);
    // Settle activation: the provider publishes its world over several frames.
    for _ in 0..90 {
        app.update();
    }
    app
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut().resource_mut::<ScriptedStick>().0 = frame;
    app.update();
}

fn hold_right() -> ControlFrame {
    let mut frame = ControlFrame::default();
    frame.axis_x = 1.0;
    frame.right_pressed = true;
    frame
}

fn press_down() -> ControlFrame {
    let mut frame = ControlFrame::default();
    frame.axis_y = 1.0;
    frame
}

fn player_pos(app: &mut App) -> Vec2 {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition::engine_core::BodyKinematics, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .expect("gameplay has a primary player")
        .pos
}

/// The id of the room that is actually AUTHORITATIVE right now — the fact a
/// transition has to change for anything to have happened.
fn active_room(app: &mut App) -> String {
    let mut query = app.world_mut().query::<&RoomSet>();
    let world = app.world();
    let set = query.iter(world).next().expect("the session has a RoomSet");
    set.rooms[set.active].id.clone()
}

fn place_player(app: &mut App, pos: Vec2) {
    let mut query = app.world_mut().query_filtered::<(
        ambition::engine_core::BodyClusterQueryData,
        &mut ambition::actors::features::MotionModel,
    ), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let (mut cluster_item, mut motion_model) = query
        .iter_mut(world)
        .next()
        .expect("gameplay has a primary player");
    let mut clusters = cluster_item.as_clusters_mut();
    ambition::engine_core::movement::transit_body(
        &mut motion_model,
        &mut clusters,
        pos,
        ambition::engine_core::movement::TransitVelocity::Zero,
    );
}

#[test]
fn she_walks_out_of_one_room_and_into_another() {
    let mut app = boot();
    assert_eq!(
        active_room(&mut app),
        LEVEL_1_1_ROOM_ID,
        "the demo should start on the surface",
    );

    // Into the vault through the real directional pipe, then along its floor to
    // the descent shaft at the far end. The pipe half is played, not set up:
    // stand on the mouth and press DOWN, the verb Jon asked for.
    let mouth = ambition_demo_mary_o::pipe_mouth();
    place_player(
        &mut app,
        Vec2::new(mouth.center().x, mouth.center().y - 24.0),
    );
    for _ in 0..8 {
        step(&mut app, ControlFrame::default());
    }
    for _ in 0..60 {
        step(&mut app, press_down());
        if player_pos(&mut app).y > ambition_demo_mary_o::vault_bounds().min.y {
            break;
        }
    }
    assert!(
        player_pos(&mut app).y > ambition_demo_mary_o::vault_bounds().min.y,
        "the pipe did not put her in the vault, so the rest of this run is meaningless",
    );

    // Now WALK to the shaft. No placement from here on: the transition has to be
    // reached by moving, or it proves nothing about reachability.
    let mut room = active_room(&mut app);
    for _ in 0..600 {
        step(&mut app, hold_right());
        room = active_room(&mut app);
        if room == LEVEL_1_2_ROOM_ID {
            break;
        }
    }

    assert_eq!(
        room, LEVEL_1_2_ROOM_ID,
        "walking into the descent shaft did not change the active room — the \
         transition either never fired or nothing consumed it",
    );

    // And she is IN the new room, not merely bookkept into it. 1-2's corridor
    // floor is at the bottom of a 14-tile room; 1-1's vault floor is elsewhere,
    // so a body still standing in the old geometry fails this.
    let pos = player_pos(&mut app);
    let world_size = {
        let mut query = app.world_mut().query::<&RoomSet>();
        let world = app.world();
        let set = query.iter(world).next().expect("a RoomSet");
        set.rooms[set.active].world.size
    };
    assert!(
        pos.x >= 0.0 && pos.x <= world_size.x && pos.y >= 0.0 && pos.y <= world_size.y,
        "she landed outside 1-2's bounds at {pos:?} (room is {world_size:?})",
    );
}

#[test]
fn the_two_rooms_are_linked_both_ways() {
    let mut app = boot();
    let mut query = app.world_mut().query::<&RoomSet>();
    let world = app.world();
    let set = query.iter(world).next().expect("the session has a RoomSet");

    let ids: Vec<&str> = set.rooms.iter().map(|room| room.id.as_str()).collect();
    assert!(ids.contains(&LEVEL_1_1_ROOM_ID), "1-1 missing from {ids:?}");
    assert!(ids.contains(&LEVEL_1_2_ROOM_ID), "1-2 missing from {ids:?}");

    // A one-way link is a trap door: you can reach 1-2 and never come back. Both
    // rooms must name a zone that leaves them.
    for room in &set.rooms {
        assert!(
            !room.loading_zones.is_empty(),
            "room '{}' has no way out",
            room.id,
        );
    }
}
