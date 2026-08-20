//! The footstool's WIRING, on a hand-built app.
//!
//! The geometry is guarded in `ambition_platformer2d_core`
//! (`collision_semantics::tests`, four cardinal frames plus a poison for the
//! hover case). What is left to prove here is everything a pure predicate
//! cannot: that the system runs, that BOTH bodies feel it, that a press is
//! required, and that a body whose tuning is `OFF` is not a platform.

use super::*;
use ambition_characters::actor::{BodyCombat, BodyHealth, Health};
use ambition_characters::brain::ActorControl;
use crate::features::ActorFaction;

const SIZE: ae::Vec2 = ae::Vec2::new(24.0, 40.0);

fn app() -> App {
    let mut app = App::new();
    app.add_systems(Update, apply_footstools);
    app
}

/// A fighter at `pos`, falling, with the platform-fighter footstool rules.
fn fighter(
    app: &mut App,
    id: &str,
    pos: ae::Vec2,
    jump: bool,
    rules: ae::FootstoolTuning,
) -> Entity {
    let mut tuning = ae::DEFAULT_TUNING;
    tuning.footstool = rules;
    let mut control = ambition_characters::actor::control::ActorControlFrame::neutral();
    control.jump_pressed = jump;
    app.world_mut()
        .spawn((
            SimId::placement(id),
            ae::BodyKinematics {
                pos,
                // Falling: the footstool refuses a body on the way up.
                vel: ae::Vec2::new(0.0, 120.0),
                size: SIZE,
                facing: 1.0,
            },
            ae::BodyGroundState::default(),
            crate::features::MotionModel::axis_swept(tuning.axis_swept_params()),
            BodyHealth::new(Health::new(100)),
            ActorFaction::Player,
            ActorControl(control),
            BodyCombat::default(),
            ae::BodyComboTrace::default(),
        ))
        .id()
}

fn rise_of(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<ae::BodyKinematics>(entity)
        .expect("the body kept its kinematics")
        .vel
        .y
}

/// **A PRESSED JUMP ON A HEAD BOUNCES THE STOMPER AND BURIES THE STOMPED.**
///
/// ⛔ both halves, because either alone is a different mechanic: a bounce with
/// no shove is a free double jump, and a shove with no bounce is a spike you
/// deliver by falling.
#[test]
fn a_footstool_lifts_the_stomper_and_drives_the_stomped_down() {
    let mut app = app();
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;
    let victim = fighter(&mut app, "victim", ae::Vec2::ZERO, false, rules);
    let stomper = fighter(
        &mut app,
        "stomper",
        ae::Vec2::new(0.0, -SIZE.y),
        true,
        rules,
    );

    app.update();

    assert_eq!(
        rise_of(&app, stomper),
        -rules.rise_speed,
        "the stomper did not leave the head at the authored speed"
    );
    assert_eq!(
        rise_of(&app, victim),
        rules.press_speed,
        "the stomped body was not driven down"
    );
    assert!(
        app.world()
            .get::<BodyCombat>(victim)
            .expect("the victim kept its combat state")
            .recoil_lock_timer
            > 0.0,
        "being stood on cost the stomped body nothing"
    );
}

/// **WITHOUT THE PRESS IT IS JUST TWO BODIES IN THE SAME PLACE.**
#[test]
fn standing_over_somebody_without_pressing_jump_does_nothing() {
    let mut app = app();
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;
    let victim = fighter(&mut app, "victim", ae::Vec2::ZERO, false, rules);
    let stomper = fighter(
        &mut app,
        "stomper",
        ae::Vec2::new(0.0, -SIZE.y),
        false,
        rules,
    );

    app.update();

    assert_eq!(rise_of(&app, stomper), 120.0, "an unpressed frame bounced");
    assert_eq!(rise_of(&app, victim), 120.0, "an unpressed frame shoved");
}

/// **A BODY WHOSE RULES ARE `OFF` IS NOT A PLATFORM, AT EITHER END.**
///
/// ⛔ this is the floor that keeps the exploration game unchanged: every body in
/// it carries the default tuning, and a footstool that ignored it would make
/// every enemy's head a platform on the day this system was registered.
#[test]
fn a_body_with_no_footstool_rules_cannot_be_stood_on() {
    let mut app = app();
    let victim = fighter(&mut app, "victim", ae::Vec2::ZERO, false, ae::FootstoolTuning::OFF);
    let stomper = fighter(
        &mut app,
        "stomper",
        ae::Vec2::new(0.0, -SIZE.y),
        true,
        ae::FootstoolTuning::PLATFORM_FIGHTER,
    );

    app.update();

    assert_eq!(rise_of(&app, stomper), 120.0, "an OFF head was a platform");
    assert_eq!(rise_of(&app, victim), 120.0, "an OFF head was shoved");
}

/// **ONE HEAD, ONE FOOTSTOOL PER TICK.**
///
/// Two bodies stacked on one victim must not both take a bounce off it, and the
/// one that gets it must be the same one on a resimulation — which is why the
/// pairs are sorted by `SimId` rather than taken in query order.
#[test]
fn a_head_is_spent_by_the_first_body_to_stand_on_it() {
    let mut app = app();
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;
    let victim = fighter(&mut app, "victim", ae::Vec2::ZERO, false, rules);
    let a = fighter(&mut app, "a_stomper", ae::Vec2::new(-4.0, -SIZE.y), true, rules);
    let b = fighter(&mut app, "b_stomper", ae::Vec2::new(4.0, -SIZE.y), true, rules);

    app.update();

    let bounced = [a, b]
        .into_iter()
        .filter(|e| rise_of(&app, *e) == -rules.rise_speed)
        .collect::<Vec<_>>();
    assert_eq!(
        bounced,
        vec![a],
        "the head was either shared or awarded out of SimId order"
    );
    assert_eq!(
        rise_of(&app, victim),
        rules.press_speed,
        "the victim was shoved twice, or not at all"
    );
}
