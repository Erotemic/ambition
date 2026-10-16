//! The castle, in the shipped demo, ends on its switch and not on a pole.
//!
//! Jon, 2026-10-11: "We also need the switch as the mechanism to kill the
//! boss. and that has to be what ends that level, not the flag. We can have
//! robot v3 be the reward at the end and you get a cut scene where the robot
//! says 'thank you mary-o'."
//!
//! Jon, 2026-10-12: the switch is a button she lands on, a gate is past it,
//! and the end has beats: "After the void lizard falls and you see the death,
//! then the wall to the right of the button opens, and you walk over to the
//! robot ..., and then the robot thanks you, there is a pause so the player
//! can take in the victory, and then it transitions to the next level".

use bevy::prelude::*;

use ambition_demo_mary_o::castle_end::{
    room_switch, rescue_line, CastleBeat, GOAL_GATE, RESCUED_CHARACTER_ID, RESCUE_CUTSCENE,
};
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

    let gate = room
        .world
        .blocks
        .iter()
        .find(|block| block.name == GOAL_GATE)
        .expect("the castle authors no `goal_gate` block: the way past the switch is open")
        .aabb;
    assert!(gate.min.x >= switch.max.x, "the gate is not past the switch");

    // A body in the air over the switch does not press it.
    let place = |app: &mut App, pos: ae::Vec2| {
        let world = app.world_mut();
        let mut bodies = world.query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>();
        let mut body = bodies.iter_mut(world).next().expect("her body");
        body.pos = pos;
        body.vel = ae::Vec2::ZERO;
    };
    let her_half = {
        let world = app.world_mut();
        let mut bodies = world.query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
        bodies.iter(world).next().expect("her body").size.y * 0.5
    };
    let over = ae::Vec2::new((switch.min.x + switch.max.x) * 0.5, switch.min.y - her_half);
    place(&mut app, over - ae::Vec2::new(0.0, 40.0));
    app.update();
    assert_eq!(beat(&mut app), None, "a body in the air over the switch pressed it");

    // She comes down on it.
    let mut pressed_at = None;
    let mut seen = vec![];
    let mut heard = None;
    for frame in 0..2400 {
        app.update();
        if let Some(now) = beat(&mut app) {
            pressed_at.get_or_insert(frame);
            if seen.last() != Some(&now) {
                // What is true of the world when a beat starts.
                match now {
                    CastleBeat::Aftermath => {
                        assert_eq!(living_lizards(&mut app), 0, "the Void Lizard is alive after its fall");
                    }
                    // The gate is a wall of solid rock to the ceiling: she is
                    // past it only if it opened.
                    CastleBeat::Thanks => {
                        let at = her_x(&mut app);
                        assert!(at > gate.max.x, "he thanks her and she is not past the gate (x {at}, gate {gate:?})");
                    }
                    beat if beat < CastleBeat::Walks => {
                        let at = her_x(&mut app);
                        assert!(at < gate.min.x, "she is at the gate in {beat:?} (x {at})");
                    }
                    _ => {}
                }
                seen.push(now);
            }
        }
        if heard.is_none() {
            if let Some(text) = line_on_screen(&app) {
                heard = Some((frame, text));
            }
        }
        if room_id(&mut app).as_deref() != Some(CASTLE) {
            use CastleBeat::*;
            assert_eq!(seen, [Pressed, Charges, BridgeFalls, BossFalls, Aftermath, GateOpens, Walks, Thanks, Victory]);
            let (line_frame, text) = heard.expect("the level went and the robot did not say his line");
            assert_eq!(text, rescue_line(), "the line on the screen is not the pack's rescue banner");
            let pressed_at = pressed_at.expect("a beat was seen");
            assert!(
                line_frame - pressed_at > 240,
                "his line came {} frames after the switch: the end of the castle is a blink",
                line_frame - pressed_at
            );
            assert_eq!(room_id(&mut app).as_deref(), Some("mary_o_2_1"), "the castle names 2-1 as its next room");
            return;
        }
    }
    panic!(
        "she landed on the switch of the castle and the level did not go in 2400 frames \
         (beats: {seen:?}, his line: {heard:?}, lizards alive: {})",
        living_lizards(&mut app)
    );
}

/// The beat of the end of the castle.
fn beat(app: &mut App) -> Option<CastleBeat> {
    let world = app.world_mut();
    let mut sequences = world.query::<&ambition_demo_mary_o::flag::FlagSequence>();
    sequences.iter(world).next().and_then(|sequence| sequence.castle_beat()).map(|(beat, _)| beat)
}

fn her_x(app: &mut App) -> f32 {
    let world = app.world_mut();
    let mut bodies = world.query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    bodies.iter(world).next().expect("her body").pos.x
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

/// The boss gauge names the lizard as her cast does.
///
/// The name was a Rust constant beside the catalog row, so a rename in
/// `character_catalog.ron` left the gauge on the old name. The gauge now reads
/// the body's own name, which the cast gives it.
#[test]
fn the_boss_gauge_names_the_boss_as_the_cast_does() {
    use ambition_platformer2d::presentation::{HudReadouts, HudSlotId};

    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(CASTLE.to_string()));
    for _ in 0..600 {
        app.update();
        let world = app.world_mut();
        let mut players = world.query_filtered::<Entity, With<PrimaryPlayer>>();
        if players.iter(world).next().is_some() {
            break;
        }
    }
    for _ in 0..30 {
        app.update();
    }
    let lizard = {
        let world = app.world_mut();
        let mut bodies = world.query::<(&ambition_platformer2d::combat::actor_tuning::ActorConfig, &ae::BodyKinematics)>();
        bodies
            .iter(world)
            .find(|(config, _)| ambition_demo_mary_o::void_lizard::is_void_lizard_brain(&config.brain))
            .map(|(_, kin)| kin.pos)
            .expect("the castle has its Void Lizard")
    };
    // Beside it, inside its sight: the gauge shows.
    let gauge = (0..60).find_map(|_| {
        {
            let world = app.world_mut();
            let mut bodies = world.query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>();
            let mut body = bodies.iter_mut(world).next().expect("her body");
            body.pos = ae::Vec2::new(lizard.x - 40.0, lizard.y);
            body.vel = ae::Vec2::ZERO;
        }
        app.update();
        app.world()
            .resource::<HudReadouts>()
            .get(&HudSlotId::from(ambition_demo_mary_o::provider::BOSS_HUD_SLOT))
            .map(|readout| readout.text())
    });
    let named = ambition_demo_mary_o::pack::PACK.display_name("void_lizard").to_uppercase();
    let gauge = gauge.expect("no boss gauge came up beside the Void Lizard");
    assert!(gauge.starts_with(&format!("{named} ")), "the boss gauge reads {gauge:?}, not the cast's name {named:?}");
}
