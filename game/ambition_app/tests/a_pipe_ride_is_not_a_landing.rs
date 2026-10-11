//! A ride through a Mary-O pipe is not a landing, in the flagship app (Jon,
//! 2026-10-11).
//!
//! "When you go down a pipe, it plays some sort of collision sound many times
//! in quick succession (I also see dust, so I think its a landing sound)."
//!
//! The Mary-O host has the same arms
//! (`ambition_demo_mary_o_app/tests/a_pipe_ride_is_not_a_landing.rs`), and
//! each one is quiet there. This is the ride in the app a player has: the
//! shell, each game's rules, and the presentation of a window.

use bevy::ecs::message::{MessageCursor, Messages};
use bevy::prelude::*;

use ambition_demo_mary_o::ldtk_vocabulary::MaryOPipeMouth;
use ambition_demo_mary_o::pipe::PipeTransit;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxMessage};
use ambition_platformer2d::vfx::{VfxInRoom, VfxMessage};

/// A cue or a puff this near her is hers.
const HERS: f32 = 96.0;

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

#[test]
fn the_ride_down_a_pipe_writes_no_landing_cue_in_the_flagship() {
    let mut app = crate::mary_o_hud_surround::mary_o_hud_app(ae::Vec2::new(1600.0, 900.0));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    crate::mary_o_hud_surround::enter_mary_o(&mut app);
    for _ in 0..120 {
        step(&mut app, ControlFrame::default());
    }
    let tubes = ambition_demo_mary_o::tubes_for_room(ambition_demo_mary_o::LEVEL_1_1_ROOM_ID);
    let down = tubes.iter().find(|tube| tube.entrance.mouth == MaryOPipeMouth::Up).expect("1-1 has a tube down");
    let band = down.entrance.mouth_band();
    let (_, size) = player_body(&mut app);
    place_player(&mut app, Vec2::new(band.center().x, band.min.y - size.y * 0.5));
    for _ in 0..20 {
        step(&mut app, ControlFrame::default());
    }
    let mut sounds = MessageCursor::<OwnedSfxMessage>::default();
    let mut effects = MessageCursor::<VfxInRoom>::default();
    let _ = sounds.read(app.world().resource::<Messages<OwnedSfxMessage>>()).count();
    let _ = effects.read(app.world().resource::<Messages<VfxInRoom>>()).count();

    let (mut ticks, mut landings, mut puffs) = (0, 0, 0);
    let mut when: Vec<String> = Vec::new();
    let mut heard = std::collections::BTreeMap::<String, usize>::new();
    let mut press = ControlFrame::default();
    press.axis_y = 1.0;
    for _ in 0..400 {
        step(&mut app, press.clone());
        if in_a_pipe(&mut app) {
            let (at, _) = player_body(&mut app);
            ticks += 1;
            for message in sounds.read(app.world().resource::<Messages<OwnedSfxMessage>>()) {
                let text = format!("{:?}", message.request);
                *heard.entry(text.split(|c: char| !c.is_alphanumeric()).next().unwrap_or_default().to_string()).or_default() += 1;
                if matches!(message.request, SfxMessage::Land { pos } if pos.distance(at) < HERS) {
                    landings += 1;
                    when.push(format!("tick {ticks}, she is at {:.1},{:.1}: {:?} from {:?}", at.x, at.y, message.request, message.source));
                }
            }
            puffs += effects
                .read(app.world().resource::<Messages<VfxInRoom>>())
                .filter(|message| matches!(message.vfx, VfxMessage::Dust { pos, .. } if pos.distance(at) < HERS))
                .count();
        } else if ticks > 0 {
            break;
        }
    }
    assert!(ticks > 30, "premise: she rode the tube down ({ticks} ticks in the transit)");
    assert_eq!(
        (landings, puffs),
        (0, 0),
        "her landing cues and the puffs of dust at her in the {ticks} ticks of one ride (each cue of the room: {heard:?}; her landings: {when:?})"
    );
}
