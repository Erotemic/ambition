//! A ride through a pipe is not a landing (Jon, 2026-10-11).
//!
//! "When you go down a pipe, it plays some sort of collision sound many times
//! in quick succession (I also see dust, so I think its a landing sound)."
//!
//! The transit puts the body at a place on its path each tick. The body is
//! not in the air between two of those ticks: it is carried. A landing cue is
//! for a body that fell and touched down.

use bevy::ecs::message::{MessageCursor, Messages};
use bevy::prelude::*;

use ambition_demo_mary_o::ldtk_vocabulary::MaryOPipeMouth;
use ambition_demo_mary_o::pipe::PipeTransit;
use ambition_demo_mary_o::PipeTube;
use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxMessage};
use ambition_platformer2d::vfx::{VfxInRoom, VfxMessage};

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut().resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>().0 = frame;
    app.update();
}

fn player_body(app: &mut App) -> (Vec2, Vec2) {
    let mut query = app.world_mut().query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    let kin = query.iter(app.world()).next().expect("gameplay has a primary player");
    (kin.pos, kin.size)
}

fn in_a_pipe(app: &mut App) -> bool {
    let mut query = app.world_mut().query_filtered::<(), (With<PrimaryPlayer>, With<PipeTransit>)>();
    query.iter(app.world()).next().is_some()
}

fn place_player(app: &mut App, pos: Vec2) {
    let mut query = app
        .world_mut()
        .query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let (mut cluster_item, mut motion_model) = query.iter_mut(world).next().expect("gameplay has a primary player");
    let mut clusters = cluster_item.as_clusters_mut();
    ae::movement::transit_body(&mut motion_model, &mut clusters, pos, ae::movement::TransitVelocity::Zero);
}

/// A cue or a puff this near her is hers: the other bodies of the room land
/// and die in their own places while she rides.
const HERS: f32 = 96.0;

/// Her landing cues since the cursor last read.
fn landings(app: &App, cursor: &mut MessageCursor<OwnedSfxMessage>, at: Vec2) -> usize {
    cursor
        .read(app.world().resource::<Messages<OwnedSfxMessage>>())
        .filter(|message| matches!(message.request, SfxMessage::Land { pos } if pos.distance(at) < HERS))
        .count()
}

/// The puffs of dust at her since the cursor last read.
fn dust(app: &App, cursor: &mut MessageCursor<VfxInRoom>, at: Vec2) -> usize {
    cursor
        .read(app.world().resource::<Messages<VfxInRoom>>())
        .filter(|message| matches!(message.vfx, VfxMessage::Dust { pos, .. } if pos.distance(at) < HERS))
        .count()
}

/// What one ride through `tube` wrote: the ticks of the transit, her landing
/// cues and the puffs of dust at her.
struct Ride {
    ticks: usize,
    landings: usize,
    puffs: usize,
}

fn up_or_down(mouth: MaryOPipeMouth) -> ControlFrame {
    let mut frame = ControlFrame::default();
    // A mouth that opens up is entered by Down, and one that opens down by Up.
    frame.axis_y = if mouth == MaryOPipeMouth::Up { 1.0 } else { -1.0 };
    frame
}

fn ride(app: &mut App, tube: &PipeTube) -> Ride {
    let band = tube.entrance.mouth_band();
    let (_, size) = player_body(app);
    // At the mouth: on it when it opens up, under it when it opens down.
    let at = match tube.entrance.mouth {
        MaryOPipeMouth::Up => Vec2::new(band.center().x, band.min.y - size.y * 0.5),
        MaryOPipeMouth::Down => Vec2::new(band.center().x, band.max.y + size.y * 0.5 - 2.0),
    };
    place_player(app, at);
    // The tick that puts her on a pipe is not a tick of the ride. Under a
    // pipe there is no such tick: she would fall from its mouth in it.
    if tube.entrance.mouth == MaryOPipeMouth::Up {
        step(app, ControlFrame::default());
    }
    let mut cursor = MessageCursor::<OwnedSfxMessage>::default();
    let mut dust_cursor = MessageCursor::<VfxInRoom>::default();
    let _ = (landings(app, &mut cursor, at), dust(app, &mut dust_cursor, at));
    let mut ride = Ride { ticks: 0, landings: 0, puffs: 0 };
    for _ in 0..600 {
        step(app, up_or_down(tube.entrance.mouth));
        if in_a_pipe(app) {
            let (at, _) = player_body(app);
            ride.ticks += 1;
            ride.landings += landings(app, &mut cursor, at);
            ride.puffs += dust(app, &mut dust_cursor, at);
        } else if ride.ticks > 0 {
            break;
        }
    }
    ride
}

fn boot() -> App {
    let mut app = build_demo_app();
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..90 {
        app.update();
    }
    app
}

/// Ride each tube of 1-1, down and then up, and say what each ride wrote.
fn each_ride_of_1_1_is_quiet(app: &mut App, form: &str) {
    let tubes = ambition_demo_mary_o::tubes_for_room(ambition_demo_mary_o::LEVEL_1_1_ROOM_ID);
    assert_eq!(tubes.len(), 2, "premise: 1-1 has a tube down and a tube up");
    for tube in tubes {
        let ride = ride(app, tube);
        let (link, ticks, landed, puffs) = (&tube.link, ride.ticks, ride.landings, ride.puffs);
        assert!(ticks > 15, "premise: the {form} Mary-O rode `{link}` ({ticks} frames in the transit)");
        assert_eq!(landed, 0, "{form}, `{link}`: {landed} landing cues in the {ticks} ticks of one ride");
        assert_eq!(puffs, 0, "{form}, `{link}`: {puffs} puffs of dust in the {ticks} ticks of one ride");
        // She comes out and stands: the next ride starts from rest.
        for _ in 0..40 {
            step(app, ControlFrame::default());
        }
    }
}

/// A window does not draw one frame for each tick: a frame of a fast display
/// has no tick in it or one, and a slow frame has two or three. The tube down
/// only: a body put under the tube up falls from its mouth in a frame of two
/// ticks, before the press is read.
#[test]
fn the_ride_down_is_quiet_when_a_frame_is_not_a_tick() {
    for frame_s in [1.0 / 144.0, 1.0 / 30.0, 1.0 / 17.0] {
        let mut app = boot();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(frame_s)));
        for _ in 0..30 {
            step(&mut app, ControlFrame::default());
        }
        let tubes = ambition_demo_mary_o::tubes_for_room(ambition_demo_mary_o::LEVEL_1_1_ROOM_ID);
        let down = tubes.iter().find(|tube| tube.entrance.mouth == MaryOPipeMouth::Up).expect("1-1 has a tube down");
        let ride = ride(&mut app, down);
        let rate = (1.0 / frame_s).round();
        assert!(ride.ticks > 5, "premise: she rode the tube down at {rate} frames a second ({} frames in the transit)", ride.ticks);
        assert_eq!((ride.landings, ride.puffs), (0, 0), "landing cues and puffs of dust in one ride at {rate} frames a second");
    }
}

/// The same rides with the published body rigs admitted, which give a body
/// the box of the pose it is in.
#[test]
fn a_ride_is_quiet_with_body_rigs() {
    let mut app = ambition_demo_mary_o_app::build_demo_app_with_body_rigs();
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..90 {
        app.update();
    }
    each_ride_of_1_1_is_quiet(&mut app, "small, with body rigs");
}

#[test]
fn a_ride_through_a_pipe_writes_no_landing_cue() {
    let mut app = boot();
    each_ride_of_1_1_is_quiet(&mut app, "small");
}

#[test]
fn a_grown_mary_o_rides_a_pipe_with_no_landing_cue() {
    let mut app = boot();
    crate::two_rooms::earn_the_star_wand(&mut app);
    for _ in 0..120 {
        step(&mut app, ControlFrame::default());
    }
    each_ride_of_1_1_is_quiet(&mut app, "grown");
}
