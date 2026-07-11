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
mod tests;
