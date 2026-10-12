//! The castle, in the shipped demo, ends on its switch and not on a pole.
//!
//! Jon, 2026-10-11: "We also need the switch as the mechanism to kill the
//! boss. and that has to be what ends that level, not the flag. We can have
//! robot v3 be the reward at the end and you get a cut scene where the robot
//! says 'thank you mary-o'."

use bevy::prelude::*;

use ambition_demo_mary_o::castle_end::{room_switch, RESCUED_CHARACTER_ID, RESCUE_CUTSCENE, RESCUE_LINE};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;

const CASTLE: &str = "mary_o_1_4";

fn room_id(app: &mut App) -> Option<String> {
    ambition_platformer2d::world::rooms::sole_live_room_spec(app.world()).map(|room| room.id.clone())
}

fn living_lizards(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut bodies = world.query::<(
        &ambition_platformer2d::combat::actor_tuning::ActorConfig,
        &ambition_platformer2d::characters::actor::BodyHealth,
    )>();
    bodies
        .iter(world)
        .filter(|(config, health)| {
            ambition_demo_mary_o::void_lizard::is_void_lizard_brain(&config.brain) && health.alive()
        })
        .count()
}

fn line_on_screen(app: &App) -> Option<String> {
    let active = app.world().resource::<ambition_platformer2d::cutscene::ActiveCutscene>();
    let playing = active.runtime.as_ref().is_some_and(|runtime| runtime.script.id == RESCUE_CUTSCENE);
    playing.then(|| active.presentation.banner.clone().map(|(text, _)| text)).flatten()
}

#[test]
fn the_switch_drops_the_boss_and_the_robot_thanks_her_before_the_level_goes() {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(CASTLE.to_string()));
    let mut seated = false;
    for _ in 0..600 {
        app.update();
        let world = app.world_mut();
        let mut players = world.query_filtered::<Entity, With<PrimaryPlayer>>();
        if players.iter(world).next().is_some() {
            seated = true;
            break;
        }
    }
    assert!(seated, "Mary-O never took a seat in the castle");
    for _ in 0..30 {
        app.update();
    }

    let room = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
        .expect("the castle is live")
        .clone();
    assert_eq!(room.id, CASTLE);
    assert!(
        !room.world.blocks.iter().any(|block| block.name == ambition_demo_mary_o::GOAL_POLE_PREFIX),
        "the castle authors a pole. Its goal is its switch."
    );
    let switch = room_switch(&room).expect("the castle authors no `goal_switch` block");
    assert!(
        format!("{room:?}").contains(RESCUED_CHARACTER_ID),
        "nobody waits past the bridge: the castle places no `{RESCUED_CHARACTER_ID}`"
    );
    assert_eq!(living_lizards(&mut app), 1, "the castle has one Void Lizard, alive, before the switch");

    // Put her on the floor at the switch.
    {
        let world = app.world_mut();
        let mut bodies = world.query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>();
        let mut body = bodies.iter_mut(world).next().expect("her body");
        let half = body.size.y * 0.5;
        body.pos = ae::Vec2::new((switch.min.x + switch.max.x) * 0.5, switch.max.y - half);
        body.vel = ae::Vec2::ZERO;
    }

    let mut heard = None;
    let mut dead_at_the_line = None;
    for frame in 0..900 {
        app.update();
        if heard.is_none() {
            if let Some(text) = line_on_screen(&app) {
                heard = Some((frame, text));
                dead_at_the_line = Some(living_lizards(&mut app) == 0);
            }
        }
        if room_id(&mut app).as_deref() != Some(CASTLE) {
            let (_, text) = heard.expect("the level went and the robot did not say his line");
            assert_eq!(text, RESCUE_LINE);
            assert_eq!(dead_at_the_line, Some(true), "the Void Lizard was alive when the robot thanked her");
            assert_eq!(room_id(&mut app).as_deref(), Some("mary_o_1_1"), "the castle names 1-1 as its next room");
            return;
        }
    }
    panic!(
        "she took the switch of the castle and the level did not go in 900 frames \
         (his line: {heard:?}, lizards alive: {})",
        living_lizards(&mut app)
    );
}

/// Where the living Void Lizards have their feet.
fn lizard_feet(app: &mut App) -> Vec<f32> {
    let world = app.world_mut();
    let mut bodies = world.query::<(
        &ambition_platformer2d::combat::actor_tuning::ActorConfig,
        &ambition_platformer2d::characters::actor::BodyHealth,
        &ae::BodyKinematics,
    )>();
    bodies
        .iter(world)
        .filter(|(config, health, _)| {
            ambition_demo_mary_o::void_lizard::is_void_lizard_brain(&config.brain) && health.alive()
        })
        .map(|(_, _, kin)| kin.pos.y + kin.size.y * 0.5)
        .collect()
}

/// Jon, 2026-10-11: "I get to the bridge and there is nothing there ... and
/// when I jump onto it, it indeed has no collision." The sequence of the flag
/// stays in its tally while she goes from 1-3 to the castle, and the bridge
/// fell for that tally on the tick she came in. A test that starts in the
/// castle does not see it: this one comes from 1-3, by its pole.
#[test]
fn the_bridge_and_its_boss_are_there_for_a_body_that_comes_from_the_last_level() {
    const BEFORE: &str = "mary_o_1_3";
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(BEFORE.to_string()));
    for _ in 0..120 {
        app.update();
    }
    let pole = ambition_demo_mary_o::pole_for_room(BEFORE);
    {
        let world = app.world_mut();
        let mut bodies = world.query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>();
        let mut body = bodies.iter_mut(world).next().expect("her body in 1-3");
        body.pos = ae::Vec2::new(pole.x, pole.base_y - 8.0);
        body.vel = ae::Vec2::ZERO;
    }
    let mut arrived = false;
    for _ in 0..900 {
        app.update();
        if room_id(&mut app).as_deref() == Some(CASTLE) {
            arrived = true;
            break;
        }
    }
    assert!(arrived, "the pole of 1-3 did not lead to the castle");
    // Time for a boss with no bridge under it to fall out of the room.
    for _ in 0..240 {
        app.update();
    }
    let room = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
        .expect("the castle is live")
        .clone();
    let floor = room_switch(&room).expect("the switch of the castle").max.y;
    let feet = lizard_feet(&mut app);
    assert_eq!(feet.len(), 1, "the Void Lizard is not alive four seconds after she came in from 1-3");
    assert!(
        feet[0] <= floor + 2.0,
        "the Void Lizard is under its bridge (feet {}, floor {floor}): the bridge fell when she came in",
        feet[0]
    );
}
