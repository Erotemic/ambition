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
use ambition_platformer2d::actors::body::transform_beat::TransformBeat;
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

    // The next press is her way out. She wears the form while its row plays:
    // 18 frames of 95 ms, and a hold on the last.
    press_utility(&mut app);
    idle(&mut app, 8);
    assert_eq!(worn(&app, body), COSMIC_CHARACTER_ID, "she left the form before the row of her way out played");
    let leave = *app
        .world()
        .get::<TransformBeat>(body)
        .expect("her way out of the form has its row");
    assert_eq!(leave.policy.anim, CharacterAnim::Shrink);
    assert!(
        (leave.policy.duration - 1.91).abs() < 0.05,
        "her way out is as long as its row takes to draw and its hold, and it is {} s",
        leave.policy.duration
    );
    // Then she is the form her equipment gives her.
    idle(&mut app, 240);
    assert_eq!(worn(&app, body), before, "she comes back to the form she had");
    assert!(app.world().get::<TransformBeat>(body).is_none(), "a beat is left on her after she came back");
}

/// A press while she changes out of the form ends the change.
#[test]
fn a_press_on_her_way_out_takes_her_out_at_once() {
    let (mut app, body) = seated();
    let before = worn(&app, body);
    press_utility(&mut app);
    idle(&mut app, 4);
    press_utility(&mut app);
    idle(&mut app, 8);
    assert_eq!(worn(&app, body), COSMIC_CHARACTER_ID);
    // Her way out, and a press four ticks into it.
    press_utility(&mut app);
    idle(&mut app, 4);
    assert_eq!(worn(&app, body), COSMIC_CHARACTER_ID);
    press_utility(&mut app);
    idle(&mut app, 4);
    assert_eq!(worn(&app, body), before, "the press on her way out did not take her out of the form");
}

#[test]
fn in_the_form_each_press_of_jump_in_the_air_is_a_jump() {
    let (mut app, body) = in_the_form();
    let ground = kin(&app, body).pos.y;
    press_jump(&mut app);
    let mut highest = ground;
    // Ten more presses, each one when she starts to come down, or as many as
    // there are before she is at the top of the room: the sky has a top
    // (`the_sky_has_a_top`), and a press there is no flight.
    let (mut presses, mut jumps_in_the_air) = (0, 0);
    let mut at_the_top = false;
    for _ in 0..600 {
        let now = kin(&app, body);
        highest = highest.min(now.pos.y);
        at_the_top = now.pos.y - now.size.y * 0.5 < 48.0;
        if presses == 10 || at_the_top {
            break;
        }
        if now.vel.y > 0.0 {
            presses += 1;
            press_jump(&mut app);
            // The jump is in her speed on the tick after the press.
            idle(&mut app, 1);
            if kin(&app, body).vel.y < 0.0 {
                jumps_in_the_air += 1;
            }
        } else {
            idle(&mut app, 1);
        }
    }
    assert!(presses >= 2, "she made {presses} press(es) in the air, so this measured nothing");
    assert_eq!(jumps_in_the_air, presses, "each press of jump in the air must be a jump");
    // One jump of this form is under five tiles. More of them are a flight:
    // 400 px up, or to the top of the room when the room is not that high.
    assert!(
        ground - highest > 400.0 || at_the_top,
        "{presses} jumps in the air took her {} px up, and not to the top of the room",
        ground - highest
    );
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

/// The sky has a top. With a jump for each press she went up with no end,
/// out of the room and out of the view (Jon, 2026-10-11: "a camera bug when
/// you are above the intended play zone, it keeps snapping up and down").
#[test]
fn in_the_form_she_does_not_fly_over_the_top_of_the_room() {
    let (mut app, body) = in_the_form();
    let mut highest = f32::MAX;
    for tick in 0..600 {
        if tick % 8 == 0 {
            press_jump(&mut app);
        } else {
            idle(&mut app, 1);
        }
        let kin = kin(&app, body);
        highest = highest.min(kin.pos.y - kin.size.y * 0.5);
    }
    assert!(highest < 40.0, "she did not get near the top of the room, so this measured nothing (top {highest})");
    assert!(highest >= -0.5, "her head went {} px over the top of the room", -highest);
}
