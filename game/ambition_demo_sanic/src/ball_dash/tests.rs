//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

fn tuning() -> BallDashTuning {
    BallDashTuning {
        rev_per_tap: 0.4,
        decay_per_s: 0.0, // isolate the rev arithmetic; decay has its own test
        ..Default::default()
    }
}

/// One deliberate X tap produces a small launch; three taps reach full and clamp.
#[test]
fn one_rev_launches_and_three_revs_reach_a_full_charge() {
    let t = tuning();
    let mut s = BallDash::default();
    for expected in [0.4, 0.8, 1.0] {
        let step = ball_dash_step(&mut s, true, true, false, true, 1.0 / 60.0, &t);
        assert_eq!(step, BallDashStep::Charging(expected));
    }
    assert_eq!(
        ball_dash_step(&mut s, true, true, false, true, 1.0 / 60.0, &t),
        BallDashStep::Charging(1.0)
    );
}

/// Holding the crouch is not the same as revving it. Charge bleeds, so the
/// verb is a rhythm.
#[test]
fn charge_decays_while_crouched_without_revving() {
    let t = BallDashTuning {
        decay_per_s: 0.5,
        ..tuning()
    };
    let mut s = BallDash::default();
    ball_dash_step(&mut s, true, true, false, true, 0.0, &t); // one tap, no time
    assert_eq!(s.charge, 0.4);
    let step = ball_dash_step(&mut s, true, true, false, false, 0.2, &t);
    let BallDashStep::Charging(charge) = step else {
        panic!("holding crouch should remain in the charging state");
    };
    assert!((charge - 0.3).abs() < 1e-6);
    // ...and it floors at zero rather than going negative.
    ball_dash_step(&mut s, true, true, false, false, 10.0, &t);
    assert_eq!(s.charge, 0.0);
}

/// The launch edge is the crouch RELEASING, not the button being up.
#[test]
fn releasing_the_crouch_launches_once_and_only_once() {
    let t = tuning();
    let mut s = BallDash::default();
    ball_dash_step(&mut s, true, true, false, true, 0.0, &t);
    ball_dash_step(&mut s, true, true, false, true, 0.0, &t); // charge 0.8

    assert_eq!(
        ball_dash_step(&mut s, false, false, true, false, 0.0, &t),
        BallDashStep::Launch(0.8)
    );
    // Still standing, still not crouched: nothing more happens.
    assert_eq!(
        ball_dash_step(&mut s, true, false, false, false, 0.0, &t),
        BallDashStep::Idle
    );
    assert_eq!(s.charge, 0.0);
}

/// A bare crouch-and-stand is standing, not a limp dash. Without this floor
/// the verb fires on every duck and reads as a bug.
#[test]
fn a_crouch_below_the_launch_floor_just_stands_up() {
    let t = BallDashTuning {
        rev_per_tap: 0.2,
        min_launch_charge: 0.3,
        ..tuning()
    };
    let mut s = BallDash::default();
    ball_dash_step(&mut s, true, true, false, true, 0.0, &t); // 0.2 — below the floor
    assert_eq!(
        ball_dash_step(&mut s, true, false, false, false, 0.0, &t),
        BallDashStep::Idle
    );
}

/// The release is a latched semantic edge, not a reconstruction from the
/// later crouch/body-mode level. Even if another system already cleared that
/// level, an armed edge spends its charge exactly once.
#[test]
fn the_latched_release_edge_spends_charge_even_if_the_prior_level_was_lost() {
    let t = tuning();
    let mut s = BallDash {
        charge: 0.4,
        crouched: false,
        contact_grace: t.contact_grace_s,
    };

    assert_eq!(
        ball_dash_step(&mut s, false, false, true, false, 0.0, &t),
        BallDashStep::Launch(0.4),
        "the semantic falling edge, not a later body-mode level, owns release"
    );
    assert_eq!(s.charge, 0.0);
}

/// A one-tick block/chain hand-off must not eat a deliberate release. The
/// charge is built on a real contact, survives less than the authored grace,
/// and launches through the already-supported airborne branch.
#[test]
fn brief_contact_loss_still_releases_the_armed_dash() {
    let t = tuning();
    let mut s = BallDash::default();
    ball_dash_step(&mut s, true, true, false, true, 0.0, &t);
    assert_eq!(s.charge, 0.4);

    assert_eq!(
        ball_dash_step(
            &mut s,
            false,
            false,
            true,
            false,
            t.contact_grace_s * 0.5,
            &t,
        ),
        BallDashStep::Launch(0.4),
        "a brief surface seam must not erase the release edge"
    );
}

/// **You cannot bank a dash across a jump.** Staying airborne past the short
/// contact grace wipes the charge, so the cost of building one is still paid at
/// the surface where it was armed.
#[test]
fn leaving_the_ground_past_the_grace_loses_the_charge() {
    let t = tuning();
    let mut s = BallDash::default();
    ball_dash_step(&mut s, true, true, false, true, 0.0, &t);
    ball_dash_step(&mut s, true, true, false, true, 0.0, &t);
    assert_eq!(s.charge, 0.8);

    assert_eq!(
        ball_dash_step(
            &mut s,
            false,
            true,
            false,
            true,
            t.contact_grace_s + 0.01,
            &t,
        ),
        BallDashStep::Idle,
        "sustained airborne time: no rev, no launch"
    );
    assert_eq!(s, BallDash::default(), "and the charge is gone");

    // Landing does not restore it.
    assert_eq!(
        ball_dash_step(&mut s, true, false, false, false, 0.0, &t),
        BallDashStep::Idle
    );
}

// ── The ECS half: a real body, the real components, the real systems ──

use ae::surface::SurfaceMotion;
use ambition::actors::features::{MomentumMotion, MotionModel};
use ambition::characters::brain::ActorControl;

fn body_app() -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(BallDashTuning {
        decay_per_s: 0.0,
        ..Default::default()
    });
    app.insert_resource(ambition::time::WorldTime::default());
    app.add_message::<ambition::sfx::SfxMessage>();
    app.add_systems(bevy::app::Update, (tick_ball_dash, tick_rolling).chain());

    let mut kin = ae::BodyKinematics::default();
    kin.size = ae::Vec2::new(24.0, 40.0);
    kin.facing = 1.0;
    let mut motion = MotionModel::SurfaceMomentum(MomentumMotion::new(Default::default()));
    if let MotionModel::SurfaceMomentum(m) = &mut motion {
        m.state = SurfaceMotion::Riding {
            on: ae::surface::SurfaceRef::Chain(0),
            s: 100.0,
            v_t: 0.0,
        };
    }
    let e = app
        .world_mut()
        .spawn((
            ActorControl::default(),
            motion,
            kin,
            BallDash::default(),
            BallDashInput::default(),
            ambition::actors::actor::BodyAnimFacts::default(),
        ))
        .id();
    (app, e)
}

fn set_ball_dash_input(app: &mut App, e: Entity, crouch: bool, rev: bool) {
    set_ball_dash_input_with_contact(app, e, crouch, rev, true);
}

fn set_ball_dash_input_with_contact(
    app: &mut App,
    e: Entity,
    crouch: bool,
    rev: bool,
    grounded_at_capture: bool,
) {
    let previous_crouch = app
        .world()
        .get::<BallDashInput>(e)
        .map_or(false, |input| input.crouch_held);
    *app.world_mut().get_mut::<BallDashInput>(e).unwrap() = BallDashInput {
        crouch_held: crouch,
        crouch_released: previous_crouch && !crouch,
        rev_pressed: rev,
        grounded_at_capture,
    };
}

#[test]
fn attack_input_is_captured_as_the_sanic_rev_before_generic_gating() {
    let mut app = App::new();
    app.insert_resource(BallDashTuning::default());
    // Reproduce the flat-floor seam: the generic ground cluster still reports
    // support while the momentum state has transiently fallen back to Airborne.
    let motion = MotionModel::SurfaceMomentum(MomentumMotion::new(Default::default()));
    let entity = app
        .world_mut()
        .spawn((
            ActorControl::default(),
            motion,
            ambition::actors::actor::BodyGroundState {
                on_ground: true,
                ..Default::default()
            },
            BallDash::default(),
            BallDashInput::default(),
        ))
        .id();
    app.insert_resource(ambition::platformer::markers::ControlledSubject(Some(
        entity,
    )));
    app.add_systems(bevy::app::Update, capture_ball_dash_input);

    {
        let mut control = app.world_mut().get_mut::<ActorControl>(entity).unwrap();
        control.0.locomotion.y = 1.0;
        control.0.melee_pressed = true;
        control.0.jump_pressed = false;
    }
    app.update();

    assert_eq!(
        *app.world().get::<BallDashInput>(entity).unwrap(),
        BallDashInput {
            crouch_held: true,
            crouch_released: false,
            rev_pressed: true,
            grounded_at_capture: true,
        },
        "the default X/attack edge becomes the mode-local spin-dash rev with its pre-compaction contact"
    );

    {
        let mut control = app.world_mut().get_mut::<ActorControl>(entity).unwrap();
        control.0.locomotion.y = 0.0;
        control.0.melee_pressed = false;
    }
    app.update();
    assert!(
        app.world()
            .get::<BallDashInput>(entity)
            .unwrap()
            .crouch_released,
        "the input seam must latch Down's falling edge for the later gameplay phase"
    );
    app.update();
    assert!(
        !app.world()
            .get::<BallDashInput>(entity)
            .unwrap()
            .crouch_released,
        "the falling edge is one fixed tick, not a sticky level"
    );
}

/// Crouch compaction can make the momentum solver report Airborne later in the
/// same tick on a flat block. The technique must use the earlier PlayerInput
/// contact, both to arm the rev and to preserve its release across that seam.
#[test]
fn pre_compaction_floor_contact_arms_and_releases_after_detach() {
    let (mut app, e) = body_app();
    set_ball_dash_input_with_contact(&mut app, e, true, true, true);
    if let MotionModel::SurfaceMomentum(momentum) =
        &mut *app.world_mut().get_mut::<MotionModel>(e).unwrap()
    {
        momentum.state = SurfaceMotion::Airborne;
    }
    app.update();
    assert_eq!(app.world().get::<BallDash>(e).unwrap().charge, 0.4);

    set_ball_dash_input_with_contact(&mut app, e, false, false, false);
    app.update();

    assert!(
        app.world().get::<Rolling>(e).is_some(),
        "the captured floor contact plus grace must survive the compaction seam"
    );
    assert!(
        app.world().get::<ae::BodyKinematics>(e).unwrap().vel.x
            >= 0.4 * BallDashTuning::default().launch_speed,
        "the airborne release writes the charged speed into world velocity"
    );
}

fn v_t(app: &App, e: Entity) -> f32 {
    match app.world().get::<MotionModel>(e).unwrap() {
        MotionModel::SurfaceMomentum(m) => match m.state {
            SurfaceMotion::Riding { v_t, .. } => v_t,
            SurfaceMotion::Airborne => panic!("expected a riding body"),
        },
        _ => panic!("expected a momentum body"),
    }
}

/// The whole verb, end to end: rev once, release, and the surface kernel's
/// tangential velocity is `facing × launch_speed × charge` — no conversion,
/// because `v_t` and `facing` share the kernel's own sign convention.
#[test]
fn releasing_one_rev_writes_its_launch_speed_into_v_t() {
    let (mut app, e) = body_app();
    set_ball_dash_input(&mut app, e, true, true);
    app.update(); // one X edge is enough for a small launch
    assert_eq!(app.world().get::<BallDash>(e).unwrap().charge, 0.4);
    assert!(app.world().get::<Rolling>(e).is_none(), "not yet");

    set_ball_dash_input(&mut app, e, false, false);
    app.update();

    let t = BallDashTuning::default();
    assert_eq!(v_t(&app, e), 0.4 * t.launch_speed);
    assert!(app.world().get::<Rolling>(e).is_some(), "he is a ball now");
}

/// Facing left launches left. The kernel's `v_t += run * accel * dt` is what
/// makes this a one-liner instead of a tangent lookup.
#[test]
fn charging_reuses_the_shared_dash_startup_animation_fact() {
    let (mut app, e) = body_app();
    set_ball_dash_input(&mut app, e, true, true);
    app.update();

    let anim = app
        .world()
        .get::<ambition::actors::actor::BodyAnimFacts>(e)
        .expect("the body carries the shared presentation facts");
    assert!(
        anim.dash_startup_timer > 0.0,
        "revving must request an existing shared sprite row rather than a demo-local renderer"
    );
}

#[test]
fn facing_decides_the_launch_direction() {
    let (mut app, e) = body_app();
    app.world_mut()
        .get_mut::<ae::BodyKinematics>(e)
        .unwrap()
        .facing = -1.0;
    set_ball_dash_input(&mut app, e, true, true);
    app.update();
    app.update();
    app.update(); // full charge
    set_ball_dash_input(&mut app, e, false, false);
    app.update();
    assert_eq!(v_t(&app, e), -BallDashTuning::default().launch_speed);
}

/// **The hurtbox-resize seam.** Rolling shrinks the live body box (the kernel's
/// circle proxy is `size.min_element() * 0.5`, so the ball is physically
/// smaller); standing up restores exactly what he was, from the flag itself.
#[test]
fn rolling_narrows_the_body_and_standing_up_restores_it() {
    let (mut app, e) = body_app();
    let standing = app.world().get::<ae::BodyKinematics>(e).unwrap().size;
    assert_eq!(standing, ae::Vec2::new(24.0, 40.0));

    set_ball_dash_input(&mut app, e, true, true);
    for _ in 0..3 {
        app.update();
    }
    set_ball_dash_input(&mut app, e, false, false);
    app.update();

    let ball = app.world().get::<ae::BodyKinematics>(e).unwrap().size;
    assert_eq!(ball, BallDashTuning::default().ball_size);
    assert_eq!(
        app.world().get::<Rolling>(e).unwrap().restore_size,
        standing,
        "the flag remembers, so nothing has to re-derive his standing height"
    );

    // Kill the speed: he stands up on the next tick, at his old size.
    if let MotionModel::SurfaceMomentum(m) =
        &mut *app.world_mut().get_mut::<MotionModel>(e).unwrap()
    {
        m.state = SurfaceMotion::Riding {
            on: ae::surface::SurfaceRef::Chain(0),
            s: 100.0,
            v_t: 5.0,
        };
    }
    app.update();
    assert!(app.world().get::<Rolling>(e).is_none());
    assert_eq!(
        app.world().get::<ae::BodyKinematics>(e).unwrap().size,
        standing
    );
}

/// A ball that leaves a ramp keeps rolling: airborne, "speed" is the world
/// velocity, not a tangential one that no longer exists. Without this the ball
/// would pop back to standing size at the apex of every jump.
#[test]
fn a_ball_launched_off_a_ramp_stays_balled_while_airborne_and_fast() {
    let (mut app, e) = body_app();
    set_ball_dash_input(&mut app, e, true, true);
    for _ in 0..3 {
        app.update();
    }
    set_ball_dash_input(&mut app, e, false, false);
    app.update();
    assert!(app.world().get::<Rolling>(e).is_some());

    // Off the end of the ramp, carrying the launch speed as world velocity.
    if let MotionModel::SurfaceMomentum(m) =
        &mut *app.world_mut().get_mut::<MotionModel>(e).unwrap()
    {
        m.state = SurfaceMotion::Airborne;
    }
    app.world_mut()
        .get_mut::<ae::BodyKinematics>(e)
        .unwrap()
        .vel = ae::Vec2::new(700.0, -200.0);
    app.update();
    assert!(
        app.world().get::<Rolling>(e).is_some(),
        "still a ball — he is moving at 700px/s"
    );

    // Slow him to a crawl mid-air: now he unrolls.
    app.world_mut()
        .get_mut::<ae::BodyKinematics>(e)
        .unwrap()
        .vel = ae::Vec2::new(3.0, 1.0);
    app.update();
    assert!(app.world().get::<Rolling>(e).is_none());
}
