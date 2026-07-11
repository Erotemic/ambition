//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

fn tuning() -> BallDashTuning {
    BallDashTuning {
        rev_per_tap: 0.25,
        decay_per_s: 0.0, // isolate the rev arithmetic; decay has its own test
        ..Default::default()
    }
}

/// Four taps to full, and no further. A fifth tap is not a fifth quarter.
#[test]
fn four_revs_reach_a_full_charge_and_it_clamps() {
    let t = tuning();
    let mut s = BallDash::default();
    for expected in [0.25, 0.50, 0.75, 1.00] {
        let step = ball_dash_step(&mut s, true, true, true, 1.0 / 60.0, &t);
        assert_eq!(step, BallDashStep::Charging(expected));
    }
    assert_eq!(
        ball_dash_step(&mut s, true, true, true, 1.0 / 60.0, &t),
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
    ball_dash_step(&mut s, true, true, true, 0.0, &t); // one tap, no time
    assert_eq!(s.charge, 0.25);
    let step = ball_dash_step(&mut s, true, true, false, 0.2, &t);
    assert_eq!(step, BallDashStep::Charging(0.15));
    // ...and it floors at zero rather than going negative.
    ball_dash_step(&mut s, true, true, false, 10.0, &t);
    assert_eq!(s.charge, 0.0);
}

/// The launch edge is the crouch RELEASING, not the button being up.
#[test]
fn releasing_the_crouch_launches_once_and_only_once() {
    let t = tuning();
    let mut s = BallDash::default();
    ball_dash_step(&mut s, true, true, true, 0.0, &t);
    ball_dash_step(&mut s, true, true, true, 0.0, &t); // charge 0.5

    assert_eq!(
        ball_dash_step(&mut s, true, false, false, 0.0, &t),
        BallDashStep::Launch(0.5)
    );
    // Still standing, still not crouched: nothing more happens.
    assert_eq!(
        ball_dash_step(&mut s, true, false, false, 0.0, &t),
        BallDashStep::Idle
    );
    assert_eq!(s.charge, 0.0);
}

/// A bare crouch-and-stand is standing, not a limp dash. Without this floor
/// the verb fires on every duck and reads as a bug.
#[test]
fn a_crouch_below_the_launch_floor_just_stands_up() {
    let t = BallDashTuning {
        min_launch_charge: 0.3,
        ..tuning()
    };
    let mut s = BallDash::default();
    ball_dash_step(&mut s, true, true, true, 0.0, &t); // 0.25 — below the floor
    assert_eq!(
        ball_dash_step(&mut s, true, false, false, 0.0, &t),
        BallDashStep::Idle
    );
}

/// **You cannot bank a dash across a jump.** Going airborne mid-rev wipes the
/// charge, so the cost of building one is always paid on the ground where you
/// built it.
#[test]
fn leaving_the_ground_mid_rev_loses_the_charge() {
    let t = tuning();
    let mut s = BallDash::default();
    ball_dash_step(&mut s, true, true, true, 0.0, &t);
    ball_dash_step(&mut s, true, true, true, 0.0, &t);
    assert_eq!(s.charge, 0.5);

    assert_eq!(
        ball_dash_step(&mut s, false, true, true, 0.0, &t),
        BallDashStep::Idle,
        "airborne: no rev, no launch"
    );
    assert_eq!(s, BallDash::default(), "and the charge is gone");

    // Landing does not restore it.
    assert_eq!(
        ball_dash_step(&mut s, true, false, false, 0.0, &t),
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
        .spawn((ActorControl::default(), motion, kin, BallDash::default()))
        .id();
    (app, e)
}

fn set_control(app: &mut App, e: Entity, crouch: bool, jump: bool) {
    let mut c = app.world_mut().get_mut::<ActorControl>(e).unwrap();
    c.0.locomotion = ae::Vec2::new(0.0, if crouch { 1.0 } else { 0.0 });
    c.0.jump_pressed = jump;
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

/// The whole verb, end to end: rev twice, release, and the surface kernel's
/// tangential velocity is `facing × launch_speed × charge` — no conversion,
/// because `v_t` and `facing` share the kernel's own sign convention.
#[test]
fn releasing_a_half_charge_writes_half_the_launch_speed_into_v_t() {
    let (mut app, e) = body_app();
    set_control(&mut app, e, true, true);
    app.update();
    app.update(); // charge 0.5 (jump_pressed is held true here on purpose)
    assert_eq!(app.world().get::<BallDash>(e).unwrap().charge, 0.5);
    assert!(app.world().get::<Rolling>(e).is_none(), "not yet");

    set_control(&mut app, e, false, false);
    app.update();

    let t = BallDashTuning::default();
    assert_eq!(v_t(&app, e), 0.5 * t.launch_speed);
    assert!(app.world().get::<Rolling>(e).is_some(), "he is a ball now");
}

/// Facing left launches left. The kernel's `v_t += run * accel * dt` is what
/// makes this a one-liner instead of a tangent lookup.
#[test]
fn facing_decides_the_launch_direction() {
    let (mut app, e) = body_app();
    app.world_mut()
        .get_mut::<ae::BodyKinematics>(e)
        .unwrap()
        .facing = -1.0;
    set_control(&mut app, e, true, true);
    app.update();
    app.update();
    app.update();
    app.update(); // full charge
    set_control(&mut app, e, false, false);
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

    set_control(&mut app, e, true, true);
    for _ in 0..4 {
        app.update();
    }
    set_control(&mut app, e, false, false);
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
    set_control(&mut app, e, true, true);
    for _ in 0..4 {
        app.update();
    }
    set_control(&mut app, e, false, false);
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
