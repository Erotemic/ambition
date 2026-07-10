//! **The ball dash (spin dash)** — Track S's one new verb.
//!
//! `docs/planning/demos/sanic.md` §Design: *"a charge technique that on release
//! sets `v_t` (grounded) or velocity (airborne) along facing to
//! `dash_speed × charge`, with a rolling state flag that narrows the hurtbox
//! (BodyBaseSize seam). This is the S-track's one new technique; registered
//! content-side."*
//!
//! Content-side means content-side: this file adds **zero engine code**. It reads
//! `ActorControl` (the brain output every controlled body carries), writes
//! `MotionModel::SurfaceMomentum`'s `v_t`, and resizes `BodyKinematics::size`
//! against the `BodyBaseSize` reference the crouch seam already established. The
//! E9 oracle — *"could another platformer be built by ADDING a content crate
//! without editing core?"* — holds for a brand-new movement verb.
//!
//! ## The input, and why it needed no new binding
//!
//! Sonic 2's spin dash is *hold down, tap jump to rev, release down to launch*.
//! Every one of those already exists on `ActorControlFrame`:
//!
//! - **crouch** = `locomotion.y ≥ threshold`. `locomotion` is in the body's LOCAL
//!   frame, `+y` toward the feet — so this is gravity-relative for free, and a
//!   Sanic running the ceiling of a loop revs the same way he does on the floor.
//! - **rev** = `jump_pressed` (a rising edge) while crouched.
//! - **launch** = the crouch releasing while charge is above the launch floor.
//!
//! No new device binding, no new engine field. `locomotion.y` existing as a local
//! axis rather than a screen axis is what makes the loop case fall out.
//!
//! ## The sign of `v_t`
//!
//! The surface kernel integrates `v_t += run * accel * dt`, where `run` is the
//! same `locomotion.x` — so **`v_t`'s sign convention IS facing's**, and the
//! launch is `facing × speed × charge` with nothing to convert. Airborne, world
//! velocity has no such convention, so the launch resolves the local side axis
//! from gravity exactly as the kernel's airborne branch does.
//!
//! ## The ball is not a costume
//!
//! Rolling shrinks `BodyKinematics::size`. The momentum kernel derives its circle
//! proxy as `size.min_element() * 0.5`, so a balled-up Sanic is *physically*
//! smaller: he fits gaps he cannot walk through, and the hurtbox narrows because
//! the body did, not because a flag said so. `BodyBaseSize` is untouched — it is
//! the standing reference `pose_view` divides by for the stance ratio, the same
//! seam crouch uses.

use ambition::engine_core as ae;
use bevy::prelude::*;

/// Feel knobs. A `Resource` so a future act can retune per-zone without a
/// component write, and so a test can build an extreme one.
#[derive(Resource, Clone, Copy, Debug)]
pub struct BallDashTuning {
    /// Charge added per rev tap. `0.25` = four taps to full.
    pub rev_per_tap: f32,
    /// Charge bled off per second while crouched. Holding forever must not
    /// guarantee a max launch — the rev is a rhythm, not a timer.
    pub decay_per_s: f32,
    /// Launch speed (px/s) at `charge == 1.0`.
    pub launch_speed: f32,
    /// Below this charge a crouch-release is just standing up. Without a floor,
    /// every crouch would fire a limp dash and the verb would feel like a bug.
    pub min_launch_charge: f32,
    /// The balled-up body box. Square, so the kernel's circle proxy is exact.
    pub ball_size: ae::Vec2,
    /// Roll ends when tangential speed falls below this. Sanic stands up when he
    /// stops, not when a timer says so.
    pub exit_speed: f32,
    /// `locomotion.y` past this reads as a crouch (local down is `+y`).
    pub crouch_threshold: f32,
}

impl Default for BallDashTuning {
    fn default() -> Self {
        Self {
            rev_per_tap: 0.25,
            decay_per_s: 0.55,
            launch_speed: 900.0,
            min_launch_charge: 0.3,
            ball_size: ae::Vec2::new(24.0, 24.0),
            exit_speed: 90.0,
            crouch_threshold: 0.5,
        }
    }
}

/// Per-body charge state. Attached to whichever body is being driven; a possessed
/// Sanic revs, and the vacated body does not.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BallDash {
    /// `0..=1`.
    pub charge: f32,
    /// Was the body crouched at the end of last tick? The release edge.
    pub crouched: bool,
}

/// The rolling flag. Carries the size to restore, so nothing has to re-derive
/// "what was he before" — and a body that somehow rolls twice cannot lose its
/// standing height.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Rolling {
    pub restore_size: ae::Vec2,
}

/// What one tick of the charge machine decided. Pure, so the feel is testable
/// without a world: this is where the verb actually lives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BallDashStep {
    /// Nothing to do.
    Idle,
    /// Revving. Carries the charge after this tick, for a HUD or a rev SFX pitch.
    Charging(f32),
    /// Released above the floor. Carries the charge that was spent.
    Launch(f32),
}

/// One tick of the charge machine. `grounded` is "riding a surface" — you cannot
/// rev in mid-air, which is Sonic's rule and also the only one that makes the
/// launch's `v_t` meaningful.
///
/// A body that goes airborne mid-rev **loses the charge**. That is deliberate: an
/// airborne launch exists (the spec asks for it) but an airborne *rev* would let a
/// player bank a dash across a jump, and the whole point of the verb is that it
/// costs you your position while you build it.
pub fn ball_dash_step(
    state: &mut BallDash,
    grounded: bool,
    crouch: bool,
    rev_pressed: bool,
    dt: f32,
    tuning: &BallDashTuning,
) -> BallDashStep {
    if !grounded {
        *state = BallDash::default();
        return BallDashStep::Idle;
    }

    if crouch {
        if rev_pressed {
            state.charge = (state.charge + tuning.rev_per_tap).min(1.0);
        }
        state.charge = (state.charge - tuning.decay_per_s * dt).max(0.0);
        state.crouched = true;
        return BallDashStep::Charging(state.charge);
    }

    // Not crouching. Was he, last tick?
    let released = state.crouched;
    state.crouched = false;
    let charge = state.charge;
    state.charge = 0.0;

    if released && charge >= tuning.min_launch_charge {
        BallDashStep::Launch(charge)
    } else {
        BallDashStep::Idle
    }
}

/// Rev, launch, roll. Runs on the controlled body only — `ActorControl` is the
/// brain output, and a body with no brain has none.
#[allow(clippy::type_complexity)]
pub fn tick_ball_dash(
    mut commands: Commands,
    time: Res<ambition::time::WorldTime>,
    tuning: Res<BallDashTuning>,
    gravity: Option<Res<ambition::platformer::gravity::GravityField>>,
    mut bodies: Query<(
        Entity,
        &ambition::characters::brain::ActorControl,
        &mut ambition::actors::features::MotionModel,
        &mut ae::BodyKinematics,
        &mut BallDash,
        Option<&Rolling>,
    )>,
) {
    let gravity_down = ambition::platformer::gravity::gravity_dir_or_default(gravity.as_deref());
    let frame = ae::AccelerationFrame::new(gravity_down);

    for (entity, control, mut motion, mut kin, mut state, rolling) in &mut bodies {
        let ambition::actors::features::MotionModel::SurfaceMomentum(m) = &mut *motion else {
            // A Sanic on the AABB path is a Sanic in a different demo. Nothing
            // here reaches for `MotionModel::AxisSwept`, on purpose: the verb is
            // defined against the momentum kernel's `v_t`, and faking it with a
            // velocity write would produce a dash that ignores slopes.
            continue;
        };
        let c = control.0;
        let grounded = matches!(m.state, ae::surface::SurfaceMotion::Riding { .. });

        match ball_dash_step(
            &mut state,
            grounded,
            c.locomotion.y >= tuning.crouch_threshold,
            c.jump_pressed,
            time.scaled_dt,
            &tuning,
        ) {
            BallDashStep::Idle | BallDashStep::Charging(_) => {}
            BallDashStep::Launch(charge) => {
                let speed = tuning.launch_speed * charge;
                let facing = if kin.facing == 0.0 { 1.0 } else { kin.facing };
                match &mut m.state {
                    ae::surface::SurfaceMotion::Riding { v_t, .. } => {
                        // The kernel integrates `v_t += run * accel * dt` with
                        // `run = locomotion.x`, so `v_t` and facing share a sign.
                        *v_t = facing * speed;
                    }
                    ae::surface::SurfaceMotion::Airborne => {
                        // No tangent to speak of; the local side axis is the
                        // kernel's own airborne convention.
                        kin.vel = frame.side * facing * speed;
                    }
                }
                if rolling.is_none() {
                    commands.entity(entity).insert(Rolling {
                        restore_size: kin.size,
                    });
                    kin.size = tuning.ball_size;
                }
            }
        }
    }
}

/// Stand back up when the roll runs out of speed. Separate from the launch so a
/// body that gains speed some other way (a booster, a slope) keeps rolling, and
/// so the exit reads off ONE quantity.
pub fn tick_rolling(
    mut commands: Commands,
    mut bodies: Query<(
        Entity,
        &ambition::actors::features::MotionModel,
        &mut ae::BodyKinematics,
        &Rolling,
    )>,
    tuning: Res<BallDashTuning>,
) {
    for (entity, motion, mut kin, rolling) in &mut bodies {
        let ambition::actors::features::MotionModel::SurfaceMomentum(m) = motion else {
            continue;
        };
        // Airborne, speed is the world velocity's magnitude; riding, it is |v_t|.
        // A ball flying off a ramp must not un-ball at the apex just because its
        // tangential speed no longer exists.
        let speed = match m.state {
            ae::surface::SurfaceMotion::Riding { v_t, .. } => v_t.abs(),
            ae::surface::SurfaceMotion::Airborne => kin.vel.length(),
        };
        if speed < tuning.exit_speed {
            kin.size = rolling.restore_size;
            commands.entity(entity).remove::<Rolling>();
        }
    }
}

/// Give the controlled body its charge state. Idempotent; runs every tick because
/// possession can hand control to a body that has never revved.
pub fn attach_ball_dash(
    mut commands: Commands,
    subject: Option<Res<ambition::platformer::markers::ControlledSubject>>,
    without: Query<(), (With<ae::BodyKinematics>, Without<BallDash>)>,
) {
    let Some(entity) = subject.and_then(|s| s.0) else {
        return;
    };
    if without.get(entity).is_ok() {
        commands.entity(entity).insert(BallDash::default());
    }
}

#[cfg(test)]
mod tests {
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
}
