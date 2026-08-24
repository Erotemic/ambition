//! Body-generic hit reaction: knockback, directional influence, and reaction
//! timers. Avatar-specific damage application remains outside this module.

use crate::feel::Platformer2dFeelTuningMonolith;
use ambition_characters::actor::BodyCombat;
use ambition_platformer2d_core as ae;

/// What the reaction DECIDED, so a caller can explain it.
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
    // THE DAMAGE THIS HIT DEALT, staled, which is what the freeze is computed
    // from. Threaded rather than read off the knockback because knockback is
    // what the freeze is deliberately NOT allowed to depend on any more — see
    // `ae::hit_response::hitlag_duration`.
    damage: i32,
    // The struck body's held control (local frame) for DI (CM2). `ZERO` = none.
    di_input_local: ae::Vec2,
    stance: VictimStance,
    // ⭐ THE ONE AIR RESOURCE A HIT GIVES BACK: the air dodge.
    //
    // ⛔⛔ AND IT IS ONE, NOT THREE. This took a whole `AirBudget` — jumps,
    // dash charges and the dodge together — because the LANDING helper refreshes
    // them together, and "the player road did X" was read as "X is a fact of
    // being hit". It is not. In the genre a spent double jump STAYS spent
    // through an ordinary edge-guard hit; that is what makes taking somebody's
    // second jump worth doing. The jump comes back from a cause that has
    // re-seated the body — landing, catching the ledge, being grabbed, a
    // respawn — and Ambition's traversal dash is its own capability that no
    // Smash-shaped hit reaction has any business recharging.
    //
    // ⇒ the parameter is the resource the rule actually names. `None` for a body
    // with no dodge state to hand over.
    dodge: Option<&mut ae::BodyDodgeState>,
    // ⭐ THE LEDGE HANG A HIT TAKES, for the same reason the budget is here: it
    // is a fact about being HIT, not about which road resolved the hit. It was
    // the player road's alone until `c3d7cdba7` and the actor road's separately
    // after — two copies of one rule, which is what D203 is about.
    //
    // `None` where a hang is impossible by construction: a captive being thrown
    // is not holding an edge.
    ledge: Option<(&mut ae::MotionModel, &mut ae::BodyLedgeState)>,
    feel: Platformer2dFeelTuningMonolith,
) -> BodyReactionOutcome {
    // ═══ THE FACTS OF BEING HIT ═══════════════════════════════════════════
    //
    // Everything in this block is true of an ACCEPTED hit whatever it launches,
    // which is the line this function exists to draw: a damaging hit that
    // authored no knockback is still a hit, and an armoured body that keeps its
    // trajectory was still hit too. Below the divider is what a LAUNCH does,
    // and only a launch reaches it.

    // ⛔ BEFORE any launch, so the hang is gone by the time a velocity is
    // written: dropping it afterwards would let the ledge constraint eat the
    // launch the same hit just handed out.
    if let Some((model, ledge)) = ledge {
        ae::movement::knock_off_ledge(model, ledge);
    }
    // The air dodge is spent per airtime, and a hit is a new airtime's worth of
    // trouble — a launched fighter that could not dodge would have no answer to
    // the follow-up. A body without the ability never spent it, so this is a
    // no-op for one.
    if let Some(dodge) = dodge {
        dodge.air_dodge_spent = false;
    }
    // ONE tuning row for the whole reaction, so the launch and the hitstun
    // cannot disagree about which feel numbers this hit uses (FB6b).
    let response = hit_response_tuning(&feel, boss_hit);
    // HITLAG IS THE HIT, not the launch. The freeze on contact is what makes a
    // hit read as a hit at all: an armoured trade or a damage-only tick that
    // passed through in silence would look like a whiff to both players.
    combat.hitstop_timer = combat
        .hitstop_timer
        .max(ae::hit_response::hitlag_duration(damage, &response));

    // ═══ THE FACTS OF A LAUNCH ═════════════════════════════════════════════
    //
    // Two hits stop here, and they stop for the same reason: nothing is going to
    // throw this body.
    //
    // SUPER ARMOR — the hit landed and the body does not answer for it. Armor is
    // about AUTHORITY, not damage (the caller has already decided the percent and
    // this function never touched it), so what an armoured body keeps is its
    // trajectory and its control: no launch, no carry, no hitstun, no recoil lock.
    //
    // ⛔⛔ NO KNOCKBACK IS NOT ZERO KNOCKBACK. `knockback_velocity(None)` returns
    // a zero launch, and writing a zero launch ERASES the velocity the body
    // already had — so a damage-only tick (a hazard, a poison, a chip) stopped a
    // running player dead. The actor road had dodged this by skipping the whole
    // reaction for a `None` knockback, which cost it the hit facts above instead.
    // Both roads were wrong in opposite directions, and that is D203's whole
    // subject.
    //
    // ⇒ the velocity is not written at all rather than written as zero. A body
    // nothing launched is still going wherever it was going.
    let Some(knockback) = knockback.filter(|_| !combat.armored) else {
        #[cfg(feature = "causal")]
        return BodyReaction {
            velocity: *vel,
            di_input_local,
            // TRUE when a launch was authored and armor ate it, FALSE when the
            // hit authored none: a reader looking for a launch needs to tell
            // "absorbed" from "never existed".
            had_knockback: knockback.is_some(),
        };
        #[cfg(not(feature = "causal"))]
        {
            let _ = (
                di_input_local,
                body_pos,
                body_facing,
                gravity_dir,
                stance,
                flight,
            );
            return;
        }
    };
    // Crouching reduces launch magnitude; no separate damage threshold applies.
    let launch = ae::hit_response::knockback_velocity(
        body_pos,
        body_facing,
        gravity_dir,
        Some(knockback),
        di_input_local,
        &response,
    ) * if stance.crouching {
        feel.crouch_cancel_scale
    } else {
        1.0
    };
    *vel = launch;
    //  and PUBLISH it, because the write above is not authoritative for every
    // body. `BodyKinematics::vel` is the authority for an axis-swept body and a
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
    combat.hitstun_timer = ae::hit_response::hitstun_duration(Some(knockback), &response);
    // Brief hard control-lock at the front of the hitstun window: the body is thrown with no
    // authority, then regains the attack verb the instant it clears (while still in hitstun +
    // i-frames). The window ending IS the genre's "meteor cancel"; there is no second verb to
    // press.
    //
    //  a FLOOR under the ordinary recoil, never an addition, so a meteor is one
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
        had_knockback: true,
    };
    #[cfg(not(feature = "causal"))]
    {
        let _ = di_input_local;
    }
}

///  a struct rather than the `(bool, bool)` it would otherwise be, and the
/// capture kit's own note is the argument: *"inserting it mid-list silently
/// shifted two positional arguments into the wrong slots and the compiler
/// reported it as a type error three parameters away."* Two adjacent booleans in
/// a twelve-argument list is that failure waiting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VictimStance {
    /// Standing on something? The meteor rule reads it: a spike on a
    /// grounded body is not a spike.
    pub grounded: bool,
    /// Crouching? CROUCH CANCEL — a crouching body takes less knockback, so
    /// ducking is a defensive option at low percent rather than only a shorter
    /// hurtbox. See [`ambition_combat::rules::DeclaredCombatRules::crouch_cancel_scale`].
    pub crouching: bool,
}

/// The kernel-tuning row for one struck body: the boss/enemy feel SELECTION is
/// this crate's business, the response math is the floor's
/// (`ae::hit_response`). One constructor, so the velocity and hitstun calls
/// cannot pick different rows for the same hit.
///  `feel.di_max_angle` must already be the RESOLVED match value (AE6).
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
        //  hitstun scales with the LAUNCH now, against this reference — the
        // mechanic that makes a follow-up possible after a big hit and
        // impossible after a jab. The two constants above stay what they were:
        // the duration a REFERENCE-strength hit arms.
        hitstun_reference_launch: ae::hit_response::STANDARD_LAUNCH_SPEED,
        hitstun_max_scale: ae::hit_response::MAX_HITSTUN_SCALE,
        hitlag_time: feel.hitlag_time,
        di_max_angle: feel.di_max_angle,
    }
}

#[cfg(test)]
mod super_armor_tests {
    use super::*;

    fn feel() -> Platformer2dFeelTuningMonolith {
        Platformer2dFeelTuningMonolith::default()
    }

    fn hard_knockback() -> crate::HitKnockback {
        crate::HitKnockback {
            dir: 1.0,
            magnitude: ae::hit_response::HitKnockbackMagnitude::LaunchSpeed(600.0),
            source_pos: ae::Vec2::new(0.0, 0.0),
            impact_pos: ae::Vec2::new(10.0, 0.0),
            launch_dir: None,
        }
    }

    fn react(armored: bool) -> (ae::Vec2, ae::BodyFlightState, BodyCombat) {
        let mut vel = ae::Vec2::new(120.0, 0.0);
        let mut flight = ae::BodyFlightState::default();
        let mut combat = BodyCombat {
            armored,
            ..Default::default()
        };
        apply_body_hit_reaction(
            &mut vel,
            &mut flight,
            &mut combat,
            ae::Vec2::new(20.0, 0.0),
            1.0,
            ae::Vec2::new(0.0, 1.0),
            false,
            Some(&hard_knockback()),
            12,
            ae::Vec2::ZERO,
            VictimStance::default(),
            // No budget and no ledge: this fixture is about the launch and
            // the stagger.
            None,
            None,
            feel(),
        );
        (vel, flight, combat)
    }

    /// Armor is about AUTHORITY, not about damage: the hit lands (damage is the
    /// caller's business and this function never touched it) and the body keeps
    /// its trajectory and its control.
    #[test]
    fn an_armoured_body_is_neither_launched_nor_stunned() {
        let (plain_vel, plain_flight, plain_combat) = react(false);
        assert_ne!(
            plain_vel,
            ae::Vec2::new(120.0, 0.0),
            "the fixture's knockback must move an UNARMOURED body, or nothing \
             below is a comparison"
        );
        assert!(plain_combat.hitstun_timer > 0.0);

        let (vel, flight, combat) = react(true);
        assert_eq!(
            vel,
            ae::Vec2::new(120.0, 0.0),
            "armor stopped the launch by zeroing the body's velocity — an \
             armoured body is still going where it was going"
        );
        assert_eq!(flight.pending_launch, ae::Vec2::ZERO);
        assert_eq!(combat.hitstun_timer, 0.0, "armor must not leave hitstun");
        assert_eq!(combat.recoil_lock_timer, 0.0);
        assert_eq!(flight.carried_hold, 0.0);
        assert!(
            combat.hitstop_timer > 0.0,
            "the hit still has to READ: an armoured trade that passed through in \
             silence looks like a whiff to both players"
        );
        // ... and the launch the plain body took is exactly what was refused.
        let _ = (plain_flight, plain_combat);
    }
}
