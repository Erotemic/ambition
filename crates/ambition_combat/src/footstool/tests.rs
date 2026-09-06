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
use ambition_characters::control::ActorControl;

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
    // a body whose `tumble_speed` is 0.0 never tumbles, and every body in
    // Ambition is that body. A fighter is not, so the fixture says so — without
    // it these tests would measure only the no-tumble fallback.
    tuning.tumble_speed = 500.0;
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
            crate::targeting::MatchTeam::new(team),
            ae::MotionModel::axis_swept(tuning.axis_swept_params()),
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

/// Seconds of tumble this body is carrying.
fn tumble_of(app: &App, entity: Entity) -> f32 {
    match app
        .world()
        .get::<ae::MotionModel>(entity)
        .expect("the body kept its motion model")
    {
        ae::MotionModel::AxisSwept(axis) => axis.state.tumble_timer,
        _ => 0.0,
    }
}

/// The hard control lock on this body's combat state.
fn lock_of(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<BodyCombat>(entity)
        .expect("the victim kept its combat state")
        .recoil_lock_timer
}

/// Did this body's press get claimed for a footstool?
fn claimed(app: &App, entity: Entity) -> bool {
    app.world()
        .get::<ae::BodyJumpState>(entity)
        .expect("the body kept its jump state")
        .footstool_claimed
}

/// A PRESSED JUMP ON A HEAD CLAIMS THE PRESS AND BURIES THE STOMPED.
///
/// both halves, because either alone is a different mechanic: a claim with no
/// shove is a free extra jump, and a shove with no claim is a spike you deliver
/// by falling. the stomper's RISE is the kernel's — it reads the claim ahead
/// of the air jump — so what this asserts here is the claim, not a velocity.
#[test]
fn a_footstool_claims_the_press_and_drives_the_stomped_down() {
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

    assert!(
        claimed(&app, stomper),
        "the press was not claimed, so the kernel would spend an air jump for it"
    );
    assert_eq!(
        fall_of(&app, victim),
        rules.press_speed,
        "the stomped body was not driven down"
    );
    assert_eq!(
        tumble_of(&app, victim),
        rules.air_tumble_time,
        "an airborne victim was not put into tumble"
    );
    assert_eq!(
        lock_of(&app, victim),
        0.0,
        "the tumble already owns control; a second lock beside it can only disagree"
    );
    assert_eq!(
        app.world()
            .get::<BodyCombat>(stomper)
            .expect("the stomper kept its combat state")
            .damage_invuln_timer,
        rules.stomper_invuln,
        "the bounce carried no i-frames, so the footstool is not an escape"
    );
}

/// A CLAIM THE KERNEL NEVER SPENT IS GONE BY THE NEXT TICK.
///
/// `footstool_claimed` means "THIS tick's jump edge was claimed", and
/// nothing but this clear makes that true. The kernel spends the claim inside
/// its footstool branch — and that branch is not first. A wall jump, a ground
/// jump, a coyote jump, a ladder jump and the one-way drop-through all resolve
/// the same press ahead of it, so a body that qualified for a footstool and
/// whose press went to a wall jump instead KEPT the claim, and the next airborne
/// press spent it over empty air: a free jump nobody stood on.
///
/// the second half is the poison and the test is worthless without it. A
/// clear that ran unconditionally after arbitration — or one that erased the
/// claim it had just granted — would satisfy the first assertion perfectly while
/// deleting the mechanic. The same tick has to do both.
#[test]
fn a_claim_the_kernel_never_spent_is_gone_by_the_next_tick() {
    let mut app = app();
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;

    // Nobody underneath this one: it is carrying a claim an earlier tick made
    // and some other jump branch took the press for.
    let stranded = fighter(&mut app, "stranded", ae::Vec2::new(600.0, 0.0), true, rules);
    app.world_mut()
        .get_mut::<ae::BodyJumpState>(stranded)
        .expect("the body kept its jump state")
        .footstool_claimed = true;

    // And a real pair, in the same tick, so the clear has to be able to tell
    // them apart.
    let victim = fighter(&mut app, "victim", ae::Vec2::ZERO, false, rules);
    let stomper = fighter(
        &mut app,
        "stomper",
        ae::Vec2::new(0.0, -SIZE.y),
        true,
        rules,
    );

    app.update();

    assert!(
        !claimed(&app, stranded),
        "a claim that lost input arbitration outlived the tick that made it, so \
         this body's next airborne press is a footstool off nothing"
    );
    assert!(
        claimed(&app, stomper),
        "the clear erased a footstool the same tick granted — the arbitration \
         has to run AFTER the sweep, not instead of it"
    );
    assert_eq!(
        fall_of(&app, victim),
        rules.press_speed,
        "the real pair stopped resolving once the sweep was added"
    );
}

/// A GROUNDED VICTIM FLINCHES; IT IS NOT SHOVED AND IT DOES NOT TUMBLE.
///
/// A body standing on a floor has nowhere to be driven, and Ultimate's grounded footstool is a
/// brief beat you follow up on, which is a different mechanic from the airborne tumble above
/// and not a shorter one.
#[test]
fn a_grounded_victim_flinches_instead_of_tumbling() {
    let mut app = app();
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;
    let victim = fighter(&mut app, "victim", ae::Vec2::ZERO, false, rules);
    app.world_mut()
        .get_mut::<ae::BodyGroundState>(victim)
        .expect("the victim kept its ground state")
        .on_ground = true;
    let stomper = fighter(
        &mut app,
        "stomper",
        ae::Vec2::new(0.0, -SIZE.y),
        true,
        rules,
    );

    app.update();

    assert!(claimed(&app, stomper), "a grounded head was not a platform");
    assert_eq!(
        fall_of(&app, victim),
        120.0,
        "a body already standing on a floor was driven into it"
    );
    assert_eq!(
        tumble_of(&app, victim),
        0.0,
        "a grounded victim tumbled; that is the airborne reaction"
    );
    assert_eq!(
        lock_of(&app, victim),
        rules.flinch_time,
        "a grounded victim owes the flinch"
    );
}

/// A BODY THAT NEVER TUMBLES STILL OWES THE SHOVE A BEAT.
///
/// `tumble_speed` is `0.0` for every body in Ambition, so without this
/// fallback an airborne victim there would be shoved with no lock at all — the
/// tumble branch returning zero would silently mean *no reaction*.
#[test]
fn an_airborne_victim_that_cannot_tumble_takes_the_flinch() {
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
    let mut tuning = ae::DEFAULT_TUNING;
    tuning.footstool = rules;
    tuning.tumble_speed = 0.0;
    app.world_mut()
        .entity_mut(victim)
        .insert(ae::MotionModel::axis_swept(
            tuning.axis_swept_params(),
        ));

    app.update();

    assert!(claimed(&app, stomper));
    assert_eq!(fall_of(&app, victim), rules.press_speed);
    assert_eq!(tumble_of(&app, victim), 0.0);
    assert_eq!(
        lock_of(&app, victim),
        rules.flinch_time,
        "a body that cannot tumble was shoved with no reaction at all"
    );
}

/// WITHOUT THE PRESS IT IS JUST TWO BODIES IN THE SAME PLACE.
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

    assert!(
        !claimed(&app, stomper),
        "an unpressed frame claimed a press"
    );
    assert_eq!(fall_of(&app, victim), 120.0, "an unpressed frame shoved");
}

/// A BODY WHOSE RULES ARE `OFF` IS NOT A PLATFORM, AT EITHER END.
///
/// this is the floor that keeps the exploration game unchanged: every body in
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

/// ONE HEAD, ONE FOOTSTOOL PER TICK.
///
/// Two bodies over one victim must not both claim a press off it, and the one
/// that gets it must be the same one on a resimulation — which is why the pairs
/// are sorted by `SimId` rather than taken in query order.
#[test]
fn a_head_is_spent_by_the_first_body_to_stand_on_it() {
    let mut app = app();
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;
    let victim = fighter(&mut app, "victim", ae::Vec2::ZERO, false, rules);
    let a = fighter(
        &mut app,
        "a_stomper",
        ae::Vec2::new(-4.0, -SIZE.y),
        true,
        rules,
    );
    let b = fighter(
        &mut app,
        "b_stomper",
        ae::Vec2::new(4.0, -SIZE.y),
        true,
        rules,
    );

    app.update();

    assert!(claimed(&app, a), "the head went out of SimId order");
    assert!(!claimed(&app, b), "one head was jumped off twice");
    assert_eq!(
        fall_of(&app, victim),
        rules.press_speed,
        "the victim was shoved twice, or not at all"
    );
}

/// ONE PRESS, ONE FOOTSTOOL — even standing over two heads.
///
/// An accepted pair spends both ends.
#[test]
fn a_stomper_over_two_heads_takes_exactly_one_of_them() {
    let mut app = app();
    let rules = ae::FootstoolTuning::PLATFORM_FIGHTER;
    // Two victims side by side, both within the stomper's footprint.
    let left = fighter(&mut app, "a_victim", ae::Vec2::new(-8.0, 0.0), false, rules);
    let right = fighter(&mut app, "b_victim", ae::Vec2::new(8.0, 0.0), false, rules);
    let stomper = fighter(
        &mut app,
        "stomper",
        ae::Vec2::new(0.0, -SIZE.y),
        true,
        rules,
    );

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

/// THE PHANTOM FOOTSTOOL: A COMMITTED VICTIM FOLLOWS THROUGH.
///
/// both halves, and the pair of them is the whole rule: the stomper still
/// gets the bounce (that is what the technique is FOR — farming height off a
/// committed opponent to escape disadvantage) while the victim's move is not
/// interrupted. Asserting only the second half would also pass if the footstool
/// had simply been refused.
#[test]
fn a_victim_in_the_middle_of_a_move_takes_no_reaction() {
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
    let mut melee = crate::components::BodyMelee::default();
    melee.begin(swing(), ae::Vec2::new(1.0, 0.0), 0.0);
    app.world_mut().entity_mut(victim).insert(melee);

    app.update();

    assert!(
        claimed(&app, stomper),
        "the bounce is the half a phantom footstool keeps"
    );
    assert_eq!(fall_of(&app, victim), 120.0, "a committed body was shoved");
    assert_eq!(tumble_of(&app, victim), 0.0, "a committed body was tumbled");
    assert_eq!(
        lock_of(&app, victim),
        0.0,
        "a committed body was interrupted"
    );
}

/// A swing in flight, which is all the phantom-footstool rule reads.
fn swing() -> crate::AttackSpec {
    crate::AttackSpec {
        intent: crate::AttackIntent::Neutral,
        startup_seconds: 0.1,
        active_seconds: 0.1,
        recovery_seconds: 0.1,
        hitbox_offset: ae::Vec2::ZERO,
        hitbox_half_size: ae::Vec2::new(8.0, 8.0),
        self_impulse: ae::Vec2::ZERO,
        knockback: ae::Vec2::ZERO,
        damage_kind: ambition_entity_catalog::placements::DamageKind::Slash,
        can_pogo: false,
        damage_override: None,
    }
}

/// A TEAMMATE IS NOT A PLATFORM UNTIL THE MATCH SAYS SO.
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

/// AND TEAM ATTACK FREES IT, so the test above measures the POLICY rather
/// than the absence of one.
#[test]
fn team_attack_lets_a_teammate_be_stood_on() {
    let mut app = app();
    app.world_mut()
        .insert_resource(crate::rules::ResolvedCombatTuning {
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

/// Bodies under different gravity frames have no shared notion of "head", so
/// the footstool relation is refused rather than evaluated in either frame.
#[test]
fn a_pair_that_disagrees_about_down_is_refused() {
    use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
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
