//! Shared movement step for every controllable body.
//! Callers resolve input and tuning; this seam applies body-generic hitlag and out-of-play rules
//! before dispatching to the movement kernel.

use ambition_platformer2d_core as ae;

use super::BodyCombat;

/// Step one body through the movement kernel with shared hitlag and out-of-play handling.
/// Axis tuning is refreshed here so refresh and integration cannot diverge between body roads.
pub fn step_body(
    model: &mut ae::movement::MotionModel,
    clusters: &mut ae::BodyClustersMut<'_>,
    // MUTABLE because the automatic displacement is BANKED here: hitlag is the
    // only place that knows the freeze is running, and the step after it is the
    // only place that knows it just lifted.
    combat: &mut BodyCombat,
    axis_tuning: ae::MovementTuning,
    // Out-of-play is enforced here so controller-specific roads cannot define different dead-body motion.
    out_of_play: bool,
    // SOMETHING ELSE OWNS THIS BODY'S POSE — a saddle, a lift, a grab.
    //
    // ⛔⛔ NOT THE SAME THING AS `out_of_play`, AND NOT ROUTED THROUGH IT. The
    // out-of-play arm halts velocity, zeroes `dt` and leans on a control hold
    // having already neutralised a dead body's input. A held body is not dead:
    // a rider steers its mount and swings from the saddle, so its combat and
    // action controls must keep working. What it must NOT do is drive itself.
    //
    // ⭐ SO THIS TAKES THE LOCOMOTION AND LEAVES EVERYTHING ELSE. The stick is
    // zeroed and the movement verbs are cleared, because a jump or a dodge
    // consumed here changes state the external constraint CANNOT undo — a snap
    // fixes a position, not a spent double-jump or an armed dodge. Position is
    // the constraint's to own and it is welcome to it.
    pose_owned_externally: bool,
    mut ctx: ae::MotionStepContext<'_>,
) -> ae::MotionStepResult {
    if pose_owned_externally {
        // ⛔ THE UNDAMPED STICK GOES TOO. Edge detection compares this tick's
        // stick against last tick's, so leaving it live would hand a rider who
        // simply HELD a direction through a ride a fresh press the moment they
        // got off — the same trap `undamped_axes`' own note describes.
        ctx.input.axes = Default::default();
        ctx.input.undamped_axes = None;
        ctx.input.movement = Default::default();
    }
    if let ae::movement::MotionModel::AxisSwept(axis) = model {
        axis.params = axis_tuning.axis_swept_params();
    }
    // Clear retained velocity while out of play so it cannot be spent after respawn or reset.
    if out_of_play {
        ae::movement::halt_body(clusters.kinematics);
        ctx.dt = 0.0;
        return ae::step_motion(model, clusters, ctx);
    }
    if combat.is_in_hitlag() {
        // SDI is allowed during hitlag, but `step_motion` does not sweep when
        // `dt == 0`; sweep the displacement here so it cannot enter geometry.
        ae::movement::shift_frozen_body(
            ctx.world,
            clusters.kinematics,
            ctx.frame.down(),
            ae::hit_response::smash_di_shift(
                ctx.input.axes.vec(),
                ctx.frame.down(),
                axis_tuning.sdi_step,
            ),
        );
        // Bank the automatic displacement while the freeze runs; it is spent
        // below, on the first step after it lifts.
        combat.asdi_owed = true;
        ctx.dt = 0.0;
    } else if combat.asdi_owed {
        // AUTOMATIC SDI — one displacement per HIT, in whatever direction the
        // stick is held now. Paid HERE, on the far side of the freeze, because
        // the defender has the whole of it to choose; paid at the start it
        // would be indistinguishable from one more SDI tick.
        //
        // Swept like the SDI shift for the same reason: a displacement applied
        // straight to the position could put a body inside geometry.
        combat.asdi_owed = false;
        ae::movement::shift_frozen_body(
            ctx.world,
            clusters.kinematics,
            ctx.frame.down(),
            ae::hit_response::smash_di_shift(
                ctx.input.axes.vec(),
                ctx.frame.down(),
                axis_tuning.asdi_step,
            ),
        );
    }
    ae::step_motion(model, clusters, ctx)
}

#[cfg(test)]
mod tests;
