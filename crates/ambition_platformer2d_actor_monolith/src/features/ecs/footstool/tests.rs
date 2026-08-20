//! The footstool's ARBITRATION, on a hand-built app.
//!
//! The geometry is guarded in `ambition_platformer2d_core`
//! (`collision_semantics::tests`, four cardinal frames plus a poison for the
//! hover case) and the kernel's jump chain owns the stomper's rise. What is
//! left to prove here is everything neither can: that the press is CLAIMED
//! rather than overwritten, that an accepted pair spends BOTH ends, that a
//! disagreeing gravity frame and a protected teammate are refused, and that a
//! body whose tuning is `OFF` is not a platform.

use super::*;
use ambition_characters::actor::{BodyCombat, BodyHealth, Health};
use ambition_characters::brain::ActorControl;

const SIZE: ae::Vec2 = ae::Vec2::new(24.0, 40.0);

fn app() -> App {
    let mut app = App::new();
    app.add_systems(Update, claim_footstools);
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
    fighter_on_team(app, id, pos, jump, rules, id)
}

/// The same, with the team named — for the two tests that are about teams.
fn fighter_on_team(
    app: &mut App,
    id: &str,
    pos: ae::Vec2,
    jump: bool,
    rules: ae::FootstoolTuning,
    team: &str,
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
            ae::BodyJumpState::default(),
            ambition_combat::targeting::MatchTeam::new(team),
            crate::features::MotionModel::axis_swept(tuning.axis_swept_params()),
            BodyHealth::new(Health::new(100)),
            ActorControl(control),
            BodyCombat::default(),
            ae::BodyComboTrace::default(),
        ))
        .id()
}

fn fall_of(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<ae::BodyKinematics>(entity)
        .expect("the body kept its kinematics")
        .vel
        .y
}

/// Did this body's press get claimed for a footstool?
fn claimed(app: &App, entity: Entity) -> bool {
    app.world()
        .get::<ae::BodyJumpState>(entity)
        .expect("the body kept its jump state")
        .footstool_claimed
}

/// **A PRESSED JUMP ON A HEAD CLAIMS THE PRESS AND BURIES THE STOMPED.**
///
/// ⛔ both halves, because either alone is a different mechanic: a claim with no
/// shove is a free extra jump, and a shove with no claim is a spike you deliver
/// by falling. ⚠ the stomper's RISE is the kernel's — it reads the claim ahead
/// of the air jump — so what this asserts here is the claim, not a velocity.
#[test]
fn a_footstool_claims_the_press_and_drives_the_stomped_down() {
    let mut app = app();
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;
    let victim = fighter(&mut app, "victim", ae::Vec2::ZERO, false, rules);
    let stomper = fighter(&mut app, "stomper", ae::Vec2::new(0.0, -SIZE.y), true, rules);

    app.update();

    assert!(
        claimed(&app, stomper),
        "the press was not claimed, so the kernel would spend an air jump for it"
    );
    assert_eq!(
        fall_of(&app, victim),
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
    let stomper = fighter(&mut app, "stomper", ae::Vec2::new(0.0, -SIZE.y), false, rules);

    app.update();

    assert!(!claimed(&app, stomper), "an unpressed frame claimed a press");
    assert_eq!(fall_of(&app, victim), 120.0, "an unpressed frame shoved");
}

/// **A BODY WHOSE RULES ARE `OFF` IS NOT A PLATFORM, AT EITHER END.**
///
/// ⛔ this is the floor that keeps the exploration game unchanged: every body in
/// it carries the default tuning, and a footstool that ignored it would make
/// every enemy's head a platform on the day this system was registered.
#[test]
fn a_body_with_no_footstool_rules_cannot_be_stood_on() {
    let mut app = app();
    let victim = fighter(
        &mut app,
        "victim",
        ae::Vec2::ZERO,
        false,
        ae::FootstoolTuning::OFF,
    );
    let stomper = fighter(
        &mut app,
        "stomper",
        ae::Vec2::new(0.0, -SIZE.y),
        true,
        ae::FootstoolTuning::PLATFORM_FIGHTER,
    );

    app.update();

    assert!(!claimed(&app, stomper), "an OFF head was a platform");
    assert_eq!(fall_of(&app, victim), 120.0, "an OFF head was shoved");
}

/// **ONE HEAD, ONE FOOTSTOOL PER TICK.**
///
/// Two bodies over one victim must not both claim a press off it, and the one
/// that gets it must be the same one on a resimulation — which is why the pairs
/// are sorted by `SimId` rather than taken in query order.
#[test]
fn a_head_is_spent_by_the_first_body_to_stand_on_it() {
    let mut app = app();
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;
    let victim = fighter(&mut app, "victim", ae::Vec2::ZERO, false, rules);
    let a = fighter(&mut app, "a_stomper", ae::Vec2::new(-4.0, -SIZE.y), true, rules);
    let b = fighter(&mut app, "b_stomper", ae::Vec2::new(4.0, -SIZE.y), true, rules);

    app.update();

    assert!(claimed(&app, a), "the head went out of SimId order");
    assert!(!claimed(&app, b), "one head was jumped off twice");
    assert_eq!(
        fall_of(&app, victim),
        rules.press_speed,
        "the victim was shoved twice, or not at all"
    );
}

/// **ONE PRESS, ONE FOOTSTOOL — even standing over two heads.**
///
/// ⛔ the mirror of the test above and the half the first version was missing:
/// it spent only the VICTIM, so a stomper whose feet reached two bodies shoved
/// them both and took two combo marks off one jump press. An accepted pair
/// spends both ends.
#[test]
fn a_stomper_over_two_heads_takes_exactly_one_of_them() {
    let mut app = app();
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;
    // Two victims side by side, both within the stomper's footprint.
    let left = fighter(&mut app, "a_victim", ae::Vec2::new(-8.0, 0.0), false, rules);
    let right = fighter(&mut app, "b_victim", ae::Vec2::new(8.0, 0.0), false, rules);
    let stomper = fighter(&mut app, "stomper", ae::Vec2::new(0.0, -SIZE.y), true, rules);

    app.update();

    assert!(claimed(&app, stomper));
    let shoved = [left, right]
        .into_iter()
        .filter(|e| fall_of(&app, *e) == rules.press_speed)
        .count();
    assert_eq!(
        shoved, 1,
        "one press shoved {shoved} bodies; a footstool takes one head"
    );
}

/// **A TEAMMATE IS NOT A PLATFORM UNTIL THE MATCH SAYS SO.**
///
/// ⛔ the genre gates a teammate footstool on Team Attack, and the first version
/// asked NO team question at all — it had asked `damage_lands_between`, found
/// that wrong (a footstool is not damage), and removed the question instead of
/// replacing it.
#[test]
fn a_teammate_cannot_be_stood_on_while_team_attack_is_off() {
    let mut app = app();
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;
    let victim = fighter_on_team(&mut app, "victim", ae::Vec2::ZERO, false, rules, "red");
    let stomper = fighter_on_team(
        &mut app,
        "stomper",
        ae::Vec2::new(0.0, -SIZE.y),
        true,
        rules,
        "red",
    );

    app.update();

    assert!(!claimed(&app, stomper), "a teammate was a platform");
    assert_eq!(fall_of(&app, victim), 120.0, "a teammate was shoved");
}

/// **AND TEAM ATTACK FREES IT**, so the test above measures the POLICY rather
/// than the absence of one.
#[test]
fn team_attack_lets_a_teammate_be_stood_on() {
    let mut app = app();
    app.world_mut()
        .insert_resource(ambition_combat::rules::ResolvedCombatTuning {
            friendly_fire: true,
            ..Default::default()
        });
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;
    let victim = fighter_on_team(&mut app, "victim", ae::Vec2::ZERO, false, rules, "red");
    let stomper = fighter_on_team(
        &mut app,
        "stomper",
        ae::Vec2::new(0.0, -SIZE.y),
        true,
        rules,
        "red",
    );

    app.update();

    assert!(
        claimed(&app, stomper),
        "Team Attack did not free the footstool"
    );
    assert_eq!(fall_of(&app, victim), rules.press_speed);
}

/// **TWO BODIES UNDER DIFFERENT GRAVITY HAVE NO SHARED "HEAD".**
///
/// ⛔ the first version read the VICTIM's box in the STOMPER's frame, so under
/// mixed gravity it judged a head that the victim does not have. Refused, rather
/// than answered in one of the two frames.
#[test]
fn a_pair_that_disagrees_about_down_is_refused() {
    use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
    let mut app = app();
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;
    let victim = fighter(&mut app, "victim", ae::Vec2::ZERO, false, rules);
    let stomper = fighter(&mut app, "stomper", ae::Vec2::new(0.0, -SIZE.y), true, rules);
    // The victim falls sideways; the stomper still falls down.
    let mut sideways = ResolvedMotionFrame::default();
    sideways.publish_resolved_frame(ae::MotionFrame::from_direction(
        ae::Vec2::new(1.0, 0.0),
        1600.0,
    ));
    app.world_mut().entity_mut(victim).insert(sideways);

    app.update();

    assert!(
        !claimed(&app, stomper),
        "a body under sideways gravity was read as having a head above it"
    );
}
