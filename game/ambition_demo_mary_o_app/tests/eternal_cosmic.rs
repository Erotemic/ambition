//! Eternal Cosmic Mary-O, in the shipped demo: the Utility control puts her in
//! the form and takes her out of it, and in the form each press of jump in
//! the air is a jump.
//!
//! Jon, 2026-10-11: "You need to be invincible, and have infinite jumps (so
//! you can effectively fly), and be faster ... we can just let me use the 'd'
//! key to make the transform happen toggle it on or off."

use bevy::prelude::*;

use ambition_demo_mary_o::cosmic::COSMIC_CHARACTER_ID;
use ambition_demo_mary_o::test_course::TEST_COURSE_ROOM_ID;
use ambition_platformer2d::actors::features::transform_beat::TransformBeat;
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}

fn idle(app: &mut App, frames: usize) {
    for _ in 0..frames {
        step(app, ControlFrame::default());
    }
}

/// One press of the Utility control: the device verb her transformation is on.
fn press_utility(app: &mut App) {
    step(
        app,
        ControlFrame {
            fly_toggle_pressed: true,
            ..Default::default()
        },
    );
}

fn press_jump(app: &mut App) {
    step(
        app,
        ControlFrame {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        },
    );
}

/// The demo, in the flat fixture course, with her seated and settled.
fn seated() -> (App, Entity) {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(TEST_COURSE_ROOM_ID.to_string()));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    let mut body = None;
    for _ in 0..600 {
        app.update();
        let world = app.world_mut();
        let mut players = world.query_filtered::<Entity, With<PrimaryPlayer>>();
        if let Some(found) = players.iter(world).next() {
            body = Some(found);
            break;
        }
    }
    let body = body.expect("Mary-O never took a seat in her own demo");
    idle(&mut app, 30);
    (app, body)
}

fn worn(app: &App, body: Entity) -> String {
    app.world().get::<WornCharacter>(body).expect("she wears a character").id().to_string()
}

fn kin(app: &App, body: Entity) -> BodyKinematics {
    app.world().get::<BodyKinematics>(body).expect("she has a body").clone()
}

/// She is in the form, and its transformation is done.
fn in_the_form() -> (App, Entity) {
    let (mut app, body) = seated();
    press_utility(&mut app);
    idle(&mut app, 4);
    // The second press ends the transformation.
    press_utility(&mut app);
    idle(&mut app, 8);
    assert_eq!(worn(&app, body), COSMIC_CHARACTER_ID);
    assert!(app.world().get::<TransformBeat>(body).is_none(), "the second press did not end her transformation");
    (app, body)
}

#[test]
fn the_utility_control_puts_her_in_the_form_and_takes_her_out_of_it() {
    let (mut app, body) = seated();
    let before = worn(&app, body);
    assert_ne!(before, COSMIC_CHARACTER_ID);

    press_utility(&mut app);
    idle(&mut app, 4);
    assert_eq!(worn(&app, body), COSMIC_CHARACTER_ID, "one press of the Utility control is the form");
    let beat = *app
        .world()
        .get::<TransformBeat>(body)
        .expect("her way into the form has its transformation");
    assert_eq!(beat.policy.anim, CharacterAnim::Transform);
    // The row of her sheet: 36 frames of 95 ms. A beat of the fallback length
    // is a sheet the demo did not read.
    assert!(
        (beat.policy.duration - 3.42).abs() < 0.05,
        "the transformation is as long as its row takes to draw, and it is {} s",
        beat.policy.duration
    );

    // A press while she changes ends the change, and she stays in the form.
    press_utility(&mut app);
    idle(&mut app, 8);
    assert!(app.world().get::<TransformBeat>(body).is_none());
    assert_eq!(worn(&app, body), COSMIC_CHARACTER_ID);

    // The next press is her way out, to the form her equipment gives her.
    press_utility(&mut app);
    idle(&mut app, 40);
    assert_eq!(worn(&app, body), before, "she comes back to the form she had");
}

#[test]
fn in_the_form_each_press_of_jump_in_the_air_is_a_jump() {
    let (mut app, body) = in_the_form();
    let ground = kin(&app, body).pos.y;
    press_jump(&mut app);
    let mut highest = ground;
    // Ten more presses, each one when she starts to come down.
    let mut jumps_in_the_air = 0;
    for _ in 0..600 {
        let now = kin(&app, body);
        highest = highest.min(now.pos.y);
        if now.vel.y > 0.0 && jumps_in_the_air < 10 {
            press_jump(&mut app);
            if kin(&app, body).vel.y < 0.0 {
                jumps_in_the_air += 1;
            }
        } else {
            idle(&mut app, 1);
        }
        if jumps_in_the_air == 10 {
            break;
        }
    }
    assert_eq!(jumps_in_the_air, 10, "each press of jump in the air must be a jump");
    // One jump of this form is under five tiles. Ten of them are a flight.
    assert!(ground - highest > 400.0, "ten jumps in the air took her {} px up", ground - highest);
}

#[test]
fn out_of_the_form_a_press_of_jump_in_the_air_is_nothing() {
    let (mut app, body) = seated();
    press_jump(&mut app);
    let mut tried = false;
    for _ in 0..200 {
        let now = kin(&app, body);
        if now.vel.y > 0.0 {
            press_jump(&mut app);
            assert!(kin(&app, body).vel.y > 0.0, "Mary-O Classic has no jump in the air");
            tried = true;
            break;
        }
        idle(&mut app, 1);
    }
    assert!(tried, "she never came down from her jump");
}
