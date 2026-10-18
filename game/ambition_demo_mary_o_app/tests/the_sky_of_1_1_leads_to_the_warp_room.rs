//! The secret of 1-1: its sky leads to a room of pipes, and a pipe of that
//! room leads to the start of a level.
//!
//! Jon, 2026-10-12: "if I fly too high in world 1-1 I trigger a special
//! loading zone to a special level with warp pipes that lets me to into any
//! starting world. This will be a nice secret in the game and also let me
//! debug easier."

use ambition_demo_mary_o::pipe::PipeTransit;
use ambition_demo_mary_o::{warp_pipes_for_room, LEVEL_1_1_ROOM_ID};
use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

const WARP_ROOM: &str = "mary_o_warp";

fn boot() -> App {
    let mut app = build_demo_app();
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..90 {
        app.update();
    }
    app
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}

fn active_room(app: &mut App) -> String {
    ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
        .expect("the session has a live room")
        .id
        .clone()
}

fn body(app: &mut App) -> ae::BodyKinematics {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    *query.iter(app.world()).next().expect("gameplay has a primary player")
}

fn in_a_pipe(app: &mut App) -> bool {
    let mut query = app
        .world_mut()
        .query_filtered::<(), (With<PrimaryPlayer>, With<PipeTransit>)>();
    query.iter(app.world()).next().is_some()
}

fn place(app: &mut App, pos: Vec2, velocity: Vec2) {
    let mut query = app.world_mut().query_filtered::<(
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
    ), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let (mut cluster_item, mut motion_model) = query
        .iter_mut(world)
        .next()
        .expect("gameplay has a primary player");
    let mut clusters = cluster_item.as_clusters_mut();
    ae::movement::transit_body(
        &mut motion_model,
        &mut clusters,
        pos,
        ae::movement::TransitVelocity::Set(velocity),
    );
}

/// Step until the active room is `room`, for `frames` at most.
fn arrives_in(app: &mut App, room: &str, frames: usize, frame: ControlFrame) -> bool {
    for _ in 0..frames {
        step(app, frame.clone());
        if active_room(app) == room {
            return true;
        }
    }
    false
}

#[test]
fn she_goes_up_out_of_1_1_and_takes_a_pipe_to_a_level() {
    let mut app = boot();
    assert_eq!(active_room(&mut app), LEVEL_1_1_ROOM_ID);

    // The top of the view is not the zone: a body that is at the top of the
    // level, and no more than a tile over it, stays in 1-1.
    place(&mut app, Vec2::new(400.0, 70.0), Vec2::new(0.0, -600.0));
    for _ in 0..120 {
        step(&mut app, ControlFrame::default());
    }
    assert_eq!(active_room(&mut app), LEVEL_1_1_ROOM_ID, "the top line of 1-1 took her to the warp room");

    // She goes up out of the view, as the form with a jump in the air does.
    place(&mut app, Vec2::new(400.0, 70.0), Vec2::new(0.0, -1400.0));
    assert!(
        arrives_in(&mut app, WARP_ROOM, 240, ControlFrame::default()),
        "she went up to the top of 1-1 and the room is `{}`",
        active_room(&mut app)
    );

    // She is in the room, and she comes down to its floor.
    let size = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
        .expect("a live room")
        .world
        .size;
    for _ in 0..180 {
        step(&mut app, ControlFrame::default());
    }
    let at = body(&mut app).pos;
    assert!(
        at.x > 0.0 && at.x < size.x && at.y > 0.0 && at.y < size.y,
        "she is at {at:?}, out of the warp room ({size:?})"
    );
    assert_eq!(active_room(&mut app), WARP_ROOM, "she did not stay in the warp room");

    // A pipe for each level of the file that is not this room.
    let pipes = warp_pipes_for_room(WARP_ROOM);
    for level in ["mary_o_1_1", "mary_o_1_2", "mary_o_1_3", "mary_o_1_4"] {
        assert!(
            pipes.iter().any(|pipe| pipe.room == level),
            "the warp room has no pipe to `{level}`"
        );
    }

    // She stands on the pipe to 1-3 and presses down.
    let pipe = pipes.iter().find(|pipe| pipe.room == "mary_o_1_3").unwrap();
    let half = body(&mut app).size.y * 0.5;
    let top = Vec2::new(
        (pipe.mouth.aabb.min.x + pipe.mouth.aabb.max.x) * 0.5,
        pipe.mouth.aabb.min.y - half,
    );
    place(&mut app, top, Vec2::ZERO);
    for _ in 0..20 {
        step(&mut app, ControlFrame::default());
    }
    assert_eq!(active_room(&mut app), WARP_ROOM, "to stand on a pipe is not to go into it");

    let mut down = ControlFrame::default();
    down.axis_y = 1.0;
    // The press is of the control of the body, some ticks after the device.
    for _ in 0..10 {
        step(&mut app, down.clone());
        if in_a_pipe(&mut app) {
            break;
        }
    }
    assert!(in_a_pipe(&mut app), "she pressed down on the pipe and did not go into it");
    assert!(
        arrives_in(&mut app, "mary_o_1_3", 300, ControlFrame::default()),
        "she went down the pipe to 1-3 and the room is `{}`",
        active_room(&mut app)
    );

    // The ride ends with the room: she is at the start of 1-3 and can move.
    for _ in 0..30 {
        step(&mut app, ControlFrame::default());
    }
    assert!(!in_a_pipe(&mut app), "the pipe transit of the warp room went with her to 1-3");
    let start = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
        .expect("a live room")
        .world
        .spawn;
    let at = body(&mut app).pos;
    assert!(
        (at.x - start.x).abs() < 48.0,
        "she is at {at:?}, and the start of 1-3 is {start:?}"
    );
    let mut right = ControlFrame::default();
    right.axis_x = 1.0;
    right.right_pressed = true;
    for _ in 0..40 {
        step(&mut app, right.clone());
    }
    assert!(
        body(&mut app).pos.x > at.x + 16.0,
        "she can not walk after the pipe: {} -> {}",
        at.x,
        body(&mut app).pos.x
    );
}

/// Each pipe of the warp room has the name of its level over it, in dark words
/// with a white edge (Jon, 2026-10-12: the white words could not be seen on
/// the pale sky).
#[cfg(feature = "visible")]
#[test]
fn each_pipe_of_the_warp_room_has_the_name_of_its_level_over_it() {
    use ambition_demo_mary_o_app::{build_windowed_demo_app_entering, RenderMode};
    use ambition_platformer2d::render::rendering::WorldLabel;

    let mut app = build_windowed_demo_app_entering(
        RenderMode::Headless,
        ambition_demo_mary_o::MARY_O_GAMEPLAY_ROUTE,
        WARP_ROOM,
    );
    for _ in 0..120 {
        app.update();
    }
    // The words and their edge are the colours her rules file states
    // (`pipe_signs`).
    let look = ambition_demo_mary_o::rules::mary_o_rules().pipe_signs;
    let rgb = |[r, g, b]: [f32; 3]| Color::srgb(r, g, b).to_srgba();
    let mut signs = app.world_mut().query::<(&Text2d, &TextColor, &WorldLabel, &Children)>();
    let mut words = Vec::new();
    for (text, ink, label, edge) in signs.iter(app.world()) {
        if !label.owner_id.starts_with("mary_o_pipe_sign:") {
            continue;
        }
        words.push(text.0.clone());
        let ink = ink.0.to_srgba();
        assert!(ink.red + ink.green + ink.blue < 1.0, "the words `{}` are not dark: {ink:?}", text.0);
        assert_eq!(ink, rgb(look.ink), "the words `{}` are not the file's ink", text.0);
        assert_eq!(edge.len(), 8, "the words `{}` have no edge on each side", text.0);
        for part in edge.iter() {
            let colour = app.world().entity(part).get::<TextColor>().expect("a part of the edge is words").0;
            assert_eq!(colour.to_srgba(), rgb(look.edge), "the edge of `{}` is not the file's edge", text.0);
        }
        // The copies of the edge are the file's distance from the words.
        let farthest = edge
            .iter()
            .map(|part| app.world().entity(part).get::<Transform>().expect("a part of the edge is placed").translation)
            .map(|at| at.x.abs().max(at.y.abs()))
            .fold(0.0_f32, f32::max);
        assert!((farthest - look.edge_px).abs() < 1e-4, "the edge of `{}` is {farthest} from it, not {}", text.0, look.edge_px);
    }
    words.sort();
    assert_eq!(words, ["1-1", "1-2", "1-3", "1-4", "2-1", "2-2", "2-3", "2-4"]);
}
