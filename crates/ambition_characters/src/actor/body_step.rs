//! **The one body step.** Every controllable body — the home avatar, an AI
//! actor, a seated fighter, a boss — reaches the movement kernel through
//! [`step_body`], so a rule about how a body integrates cannot reach one road
//! and miss another.
//!
//! ⛔⛔ **that asymmetry is not hypothetical; it is what D114 WAS.** Hitstop is
//! armed on the victim and the attacker alike, because a landed hit is one
//! event — but only the avatar road ever branched on it. So the freeze was a
//! property of *who got hit* rather than of the hit, a player felt every
//! connect, and an exchange between two AI bodies froze neither of them. On a
//! platform fighter that is every CPU match and every seat past the first. The
//! fix was one branch; the DEFECT was that there were two places to put it and
//! only one of them had it.
//!
//! ⭐ so this function exists to make the rule structural rather than
//! remembered. It lives here, in the actor-behaviour crate, because this crate's
//! stated job is *"the same brain + control-frame contract drives players, NPCs,
//! enemies, and bosses"* — and because the rule needs [`BodyCombat`], which
//! [`ambition_platformer2d_core`] deliberately cannot see: `step_motion`'s own
//! contract is *"model dispatch happens inside the trusted kernel, while
//! body/controller identity remains outside."* A body's hitlag is body identity.
//!
//! ⚠ **what stays with the CALLER, on purpose**: resolving which
//! [`MovementTuning`] this body moves under (authored feel, a live inspector
//! slider, a flyer's derived chase speed), and building the [`InputState`]. Those
//! differ legitimately between roads. What must not differ is what happens once
//! they are resolved.

use ambition_platformer2d_core as ae;

use super::BodyCombat;

/// Step one body through the movement kernel, spending whatever hitlag it is in.
///
/// Takes the body's [`BodyCombat`] rather than a pre-computed `dt` so that no
/// caller can spell the freeze differently — or forget it, which is the failure
/// this replaces. The `dt` on `ctx` is the tick's ordinary simulation delta; a
/// body in hitlag steps at zero and everything else about the step is unchanged.
///
/// ⭐ **`axis_tuning` is applied here rather than by the caller** because the
/// live-tuning refresh and the step are one operation in practice: a caller that
/// steps without refreshing runs a body on last session's authored feel, and a
/// caller that refreshes without stepping has done nothing. Both roads already
/// wrote these two lines adjacent; this is that pair, named once.
///
/// ⚠ the refresh touches ONLY the axis policy's parameters. The environmental
/// acceleration frame rides `ctx.frame` and cannot be frozen into, or reset
/// with, movement-model configuration.
pub fn step_body(
    model: &mut ae::movement::MotionModel,
    clusters: &mut ae::BodyClustersMut<'_>,
    combat: &BodyCombat,
    axis_tuning: ae::MovementTuning,
    // **Has this body's attempt already ended?** (`OutOfPlay`, ADR 0033.) A bool
    // for the same reason the caller takes one: the rule is applied HERE so a
    // second road cannot invent a slightly different "does a dead body move".
    out_of_play: bool,
    mut ctx: ae::MotionStepContext<'_>,
) -> ae::MotionStepResult {
    if let ae::movement::MotionModel::AxisSwept(axis) = model {
        axis.params = axis_tuning.axis_swept_params();
    }
    // ⭐ ONE named rule, asked of the BODY. See this module's header for why it
    // is not a `dt` parameter: a parameter is something a caller can compute
    // wrongly, and one of them did for months.
    // ⭐ **A DEAD BODY STOPS WHERE IT DIED.** Jon, 2026-08-21: *"when you die,
    // the camera will still keep panning. Her death should stop her velocity to
    // play her death animation, so the camera should stop too as a side
    // effect."* — and the causal order in that sentence is the design: the
    // camera follows the body, so stopping the body is the whole fix and pinning
    // the camera would be the same bug wearing a hat.
    //
    // ⛔ **`OutOfPlay`'s own doc already CLAIMED this** — *"it makes 'she dies
    // where she died' free … nothing moves her now, so there is nothing to
    // pin"* — while the flag only ever gated a `BodyReset`. Gravity and carried
    // momentum went on integrating, so she slid or fell through her own death
    // animation. The doc was right about the intent and nothing implemented it.
    //
    // Velocity is cleared rather than merely frozen because the window ends in a
    // respawn or a level reset; a retained velocity would be spent the instant
    // the body came back. Idempotent, so it is safe under rollback re-simulation.
    if out_of_play {
        clusters.kinematics.vel = ae::Vec2::ZERO;
        ctx.dt = 0.0;
        return ae::step_motion(model, clusters, ctx);
    }
    if combat.is_in_hitlag() {
        // ⭐ **SDI: the ONE thing a body may still do while frozen.** Hitlag is
        // a WINDOW rather than merely a pause, and this is what makes it one —
        // the victim shifts itself out of the next hit's way while the current
        // one is still stopped. Its offensive twin, DI, already rides the launch
        // this same freeze precedes.
        //
        // ⚠ written straight to `pos` because `dt` is about to be zero and
        // nothing will integrate it. A shift into geometry is small and bounded
        // (px per tick), so the kernel's own contact correction resolves it out
        // the near face on the next moving tick — which is also the honest
        // answer to "can I SDI through a wall": no.
        clusters.kinematics.pos += ae::hit_response::smash_di_shift(
            ctx.input.axes.vec(),
            ctx.frame.down(),
            axis_tuning.sdi_step,
        );
        ctx.dt = 0.0;
    }
    ae::step_motion(model, clusters, ctx)
}

#[cfg(test)]
mod tests;
