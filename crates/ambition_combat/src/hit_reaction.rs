//! Body-generic hit reaction: knockback, directional influence, and reaction
//! timers. Avatar-specific damage application remains outside this module.

use crate::feel::Platformer2dFeelTuningMonolith;
use ambition_characters::actor::BodyCombat;
use ambition_platformer2d_core as ae;

/// **What the reaction DECIDED**, so a caller can explain it.
///
/// The magnitude and direction of a launch are the product of the authored
/// knockback, the victim's own DI, and the feel tuning — three inputs whose
/// individual contributions are invisible in the resulting velocity. "Why did
/// knockback have this magnitude and direction" is the inspector's question,
/// and it cannot be answered from the velocity alone.
///
/// Returned rather than published from in here: this function takes a `&mut
/// Vec2` and no writer, and threading one in would put an observer's parameter
/// through the reaction every body shares. The caller has the writer.
#[cfg(feature = "causal")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyReaction {
    /// The velocity the launch wrote.
    pub velocity: ae::Vec2,
    /// The DI the victim held, in its local frame. `ZERO` means it did not
    /// steer — which is a different finding from steering and being overruled.
    pub di_input_local: ae::Vec2,
    pub hitstun: f32,
    /// `false` when the hit carried no knockback at all: the body was hurt and
    /// not launched, and a reader looking for a launch should stop here.
    pub had_knockback: bool,
}

/// What [`apply_body_hit_reaction`] returns. A unit under no instrument, so a
/// build without `causal` pays nothing for a value nobody reads.
#[cfg(feature = "causal")]
pub type BodyReactionOutcome = BodyReaction;
#[cfg(not(feature = "causal"))]
pub type BodyReactionOutcome = ();

pub fn apply_body_hit_reaction(
    vel: &mut ae::Vec2,
    // Preserve launch momentum against hands-off air-stop assist.
    flight: &mut ae::BodyFlightState,
    combat: &mut BodyCombat,
    body_pos: ae::Vec2,
    body_facing: f32,
    gravity_dir: ae::Vec2,
    boss_hit: bool,
    knockback: Option<&crate::HitKnockback>,
    // The struck body's held control (local frame) for DI (CM2). `ZERO` = none.
    di_input_local: ae::Vec2,
    stance: VictimStance,
    feel: Platformer2dFeelTuningMonolith,
) -> BodyReactionOutcome {
    // ONE tuning row for the whole reaction, so the launch and the hitstun
    // cannot disagree about which feel numbers this hit uses (FB6b).
    let response = hit_response_tuning(&feel, boss_hit);
    // Crouching reduces launch magnitude; no separate damage threshold applies.
    let launch = ae::hit_response::knockback_velocity(
        body_pos,
        body_facing,
        gravity_dir,
        knockback,
        di_input_local,
        &response,
    ) * if stance.crouching {
        feel.crouch_cancel_scale
    } else {
        1.0
    };
    *vel = launch;
    // ⭐ **and PUBLISH it, because the write above is not authoritative for every
    // body.** `BodyKinematics::vel` is the authority for an axis-swept body and a
    // MIRROR for a riding surface-momentum one, whose velocity is derived from
    // the scalar `v_t` along its tangent and republished on the next step. Sanic
    // rides. So every knockback he took was applied faithfully to a field nothing
    // read, with hitstun 0.24s and knockback 360/260 all non-zero and no reaction
    // visible — reported as "no knockback", diagnosed for a long time as feel.
    //
    // `step_motion` drains this and hands it to the model, which is the only
    // thing that knows whether a launch means *leave the surface* or *override
    // the run*. Written here rather than applied here for the same reason: this
    // function has a `&mut Vec2` and no world and no `MotionModel`.
    flight.pending_launch = launch;
    combat.hitstun_timer = ae::hit_response::hitstun_duration(knockback, &response);
    // Brief hard control-lock at the front of the hitstun window: the body is thrown with no
    // authority, then regains the attack verb the instant it clears (while still in hitstun +
    // i-frames). The window ending IS the genre's "meteor cancel"; there is no second verb to
    // press.
    //
    // ⚠ a FLOOR under the ordinary recoil, never an addition, so a meteor is one
    // silence of a stated length rather than two stacked. And airborne only: a
    // body already standing on the floor is driven into a floor it is on, and
    // charging it a recovery window for that would be a free stun.
    let meteor = !stance.grounded
        && feel.meteor_lock_time > 0.0
        && launch.dot(gravity_dir) > 0.0
        && launch.length_squared() > 0.0;
    combat.recoil_lock_timer = if meteor {
        feel.knockback_recoil_lock_time.max(feel.meteor_lock_time)
    } else {
        feel.knockback_recoil_lock_time
    };
    combat.hitstop_timer = combat
        .hitstop_timer
        .max(ae::hit_response::hitlag_duration(knockback, &response));
    // CARRY THE LAUNCH, for exactly as long as the body cannot answer for it.
    //
    // The floor is the run-axis component of the velocity just written, in the
    // same frame the launch was resolved in. The window is HITSTUN, not the
    // recoil lock: the recoil lock is the short hard beat at the front, while
    // hitstun is the whole span in which the body has no authority over its own
    // trajectory — and "momentum you were given while you could not act" is a
    // statement about authority. Deliberately an existing number rather than a
    // new tuning knob; the carry is owed for exactly as long as the reaction it
    // belongs to.
    let side = ae::AccelerationFrame::new(gravity_dir).side;
    flight.carried_run = vel.dot(side);
    flight.carried_hold = combat.hitstun_timer;

    #[cfg(feature = "causal")]
    return BodyReaction {
        velocity: *vel,
        di_input_local,
        hitstun: combat.hitstun_timer,
        had_knockback: knockback.is_some(),
    };
    #[cfg(not(feature = "causal"))]
    {
        let _ = di_input_local;
    }
}

/// ⛔ **a struct rather than the `(bool, bool)` it would otherwise be**, and the
/// capture kit's own note is the argument: *"inserting it mid-list silently
/// shifted two positional arguments into the wrong slots and the compiler
/// reported it as a type error three parameters away."* Two adjacent booleans in
/// a twelve-argument list is that failure waiting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VictimStance {
    /// **Standing on something?** The meteor rule reads it: a spike on a
    /// grounded body is not a spike.
    pub grounded: bool,
    /// **Crouching?** CROUCH CANCEL — a crouching body takes less knockback, so
    /// ducking is a defensive option at low percent rather than only a shorter
    /// hurtbox. See [`ambition_combat::rules::DeclaredCombatRules::crouch_cancel_scale`].
    pub crouching: bool,
}

/// The kernel-tuning row for one struck body: the boss/enemy feel SELECTION is
/// this crate's business, the response math is the floor's
/// (`ae::hit_response`). One constructor, so the velocity and hitstun calls
/// cannot pick different rows for the same hit.
/// ⚠ **`feel.di_max_angle` must already be the RESOLVED match value** (AE6).
/// Directional influence is a rule of the match being played, not world tuning,
/// so the system that reads `Res<Platformer2dFeelTuningMonolith>` folds the resolved rules
/// into its LOCAL copy before the hit path sees it — see `resolved_feel`. The
/// row travels as one struct so the launch and the hitstun cannot disagree.
pub fn hit_response_tuning(
    feel: &Platformer2dFeelTuningMonolith,
    boss_hit: bool,
) -> ae::hit_response::HitResponseTuning {
    ae::hit_response::HitResponseTuning {
        knockback_x: if boss_hit {
            feel.boss_knockback_x
        } else {
            feel.enemy_knockback_x
        },
        knockback_y: if boss_hit {
            feel.boss_knockback_y
        } else {
            feel.enemy_knockback_y
        },
        hitstun_time: if boss_hit {
            feel.boss_hitstun_time
        } else {
            feel.enemy_hitstun_time
        },
        // ⭐ **hitstun scales with the LAUNCH now**, against this reference — the
        // mechanic that makes a follow-up possible after a big hit and
        // impossible after a jab. The two constants above stay what they were:
        // the duration a REFERENCE-strength hit arms.
        hitstun_reference_launch: ae::hit_response::STANDARD_LAUNCH_SPEED,
        hitstun_max_scale: ae::hit_response::MAX_HITSTUN_SCALE,
        hitlag_time: feel.hitlag_time,
        di_max_angle: feel.di_max_angle,
    }
}
