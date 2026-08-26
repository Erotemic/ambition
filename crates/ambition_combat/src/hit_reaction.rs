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
    /// Seconds of hitstun THIS REACTION CHARGED — not the body's remaining
    /// timer.
    ///
    /// ⛔ `0.0` on the two paths that launch nothing (armor ate it, or the hit
    /// carried no knockback), because the body may still be serving hitstun from
    /// an EARLIER hit and reporting that here would credit this reaction with
    /// somebody else's stun.
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
    // ⭐⭐ AND THE HELPLESS EPISODE THIS HIT ENDS — which is NOT a resource, and
    // that distinction is the whole reason it is a separate parameter beside the
    // dodge rather than folded into it.
    //
    // A fighter that spent its last recovery is helpless until something answers
    // for it. Being hit answers for it: the hit above hands the air dodge back
    // precisely so a launched fighter has an answer to the follow-up, and a body
    // still forbidden to act cannot use it. ⛔ CLEARING THE EPISODE REFUNDS
    // NOTHING — `recovery_charges` stays spent, and so does the double jump.
    jump: Option<&mut ae::BodyJumpState>,
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
    // …and the fighter is allowed to use it. See the parameter's own note for
    // why this gives back no charge.
    if let Some(jump) = jump {
        jump.post_recovery_helpless = false;
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
            // ⛔ NONE, and that is the paragraph above in one number: this path
            // is *"no launch, no carry, no hitstun, no recoil lock"*. The body's
            // own `hitstun_timer` may be non-zero from an earlier hit, and
            // reporting it here would credit this reaction with that stun.
            hitstun: 0.0,
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
    // ⭐ AN AUTOLINK PULSE HOLDS INSTEAD OF LAUNCHING, and it is the same road:
    // one velocity, written once, under the victim's own body authority. The
    // genre's multi-hit moves work because their intermediate pulses keep the
    // victim inside the next hitbox and only the LAST one launches, so this is a
    // property of the HIT rather than a mode the victim is put into — no
    // relationship, no hold clock, no escape, and the victim keeps every verb it
    // had. ⛔ crouch-cancel does not scale it: crouching shortens a LAUNCH, and
    // there is nothing here to shorten.
    let launch = match knockback.follow.as_ref() {
        // ⭐ THE VICTIM SIDE ONLY CLOSES A GAP NOW. Where the anchor IS was
        // resolved by the producer, against the attacker's own facing and frame —
        // this body has neither, and reconstructing another body's coordinate
        // system from its own facts is what made a back-side hit mirror the wrong
        // way.
        Some(follow) => ae::hit_response::autolink_velocity(follow, body_pos),
        // Crouching reduces launch magnitude; no separate damage threshold applies.
        None => {
            ae::hit_response::knockback_velocity(
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
            }
        }
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
    // …with the KIND it is, because the gateway that drains this decides the
    // floor game from it: a gust must neither pin a prone body nor start a
    // tumble, and speed alone cannot tell it from a hit.
    flight.stage_launch(launch, knockback.flinchless);
    // ⭐⭐ A GUST MOVES YOU AND LEAVES YOU IN CONTROL. The launch above is
    // computed exactly as a strike's is — a windbox authors its strength and
    // direction the ordinary way — and the ONE difference is here: no hitstun,
    // so a body being blown off a ledge can still act on the way. That is the
    // whole reason the genre has windboxes rather than weak hits.
    //
    // ⛔ `= 0.0` WOULD BE WRONG. A body already reeling from a real hit that
    // then drifts through a gust must not have its stun CLEARED by it — that
    // would make a windbox the best combo breaker in the game. A flinchless
    // pulse declines to charge stun; it does not discharge it.
    if !knockback.flinchless {
        combat.hitstun_timer = ae::hit_response::hitstun_duration(Some(knockback), &response);
    }
    // Brief hard control-lock at the front of the hitstun window: the body is thrown with no
    // authority, then regains the attack verb the instant it clears (while still in hitstun +
    // i-frames). The window ending IS the genre's "meteor cancel"; there is no second verb to
    // press.
    //
    //  a FLOOR under the ordinary recoil, never an addition, so a meteor is one
    // silence of a stated length rather than two stacked. And airborne only: a
    // body already standing on the floor is driven into a floor it is on, and
    // charging it a recovery window for that would be a free stun.
    // ⛔ AND AN AUTOLINK IS NEVER A METEOR. The lock keys on "the velocity points
    // toward the feet", which is true of any follow anchor placed BELOW the
    // attacker — a spinning move that gathers its victim under itself would
    // charge the genre's meteor silence for holding somebody. A meteor is an
    // authored LAUNCH downward; this pulse authored a hold.
    let meteor = knockback.follow.is_none()
        && !stance.grounded
        && feel.meteor_lock_time > 0.0
        && launch.dot(gravity_dir) > 0.0
        && launch.length_squared() > 0.0;
    // ⛔⛔ AND A FLINCHLESS PULSE DECLINES THIS TOO, for the reason stated four
    // lines above about hitstun: a WINDBOX pushes you and leaves you in control.
    // This was assigned unconditionally, so a gust that authored no stun still
    // took the hard control lock — several frames of no authority from a volume
    // whose whole contract is "moves you and leaves you playing", and a
    // REPEATING gust refreshed it every pulse.
    //
    // ⭐ SAME ASYMMETRY AS THE STUN: it declines to CHARGE the lock, it does not
    // DISCHARGE one. A body already reeling from a real hit that drifts through
    // a gust keeps the beat it was already owed.
    if !knockback.flinchless {
        combat.recoil_lock_timer = if meteor {
            feel.knockback_recoil_lock_time.max(feel.meteor_lock_time)
        } else {
            feel.knockback_recoil_lock_time
        };
    }
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
            flinchless: false,
            dir: 1.0,
            magnitude: ae::hit_response::HitKnockbackMagnitude::LaunchSpeed(600.0),
            source_pos: ae::Vec2::new(0.0, 0.0),
            impact_pos: ae::Vec2::new(10.0, 0.0),
            launch_dir: None,
            follow: None,
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

    /// ⭐⭐ A GUST MOVES YOU AND LEAVES YOU IN CONTROL.
    ///
    /// The whole of the parity inventory's *"Windboxes / flinchless push"* row:
    /// hit reaction may apply velocity without hitstun. ⛔ AND THE LAUNCH HALF
    /// IS ASSERTED FIRST — a windbox that quietly stopped launching would
    /// satisfy "no hitstun" perfectly while doing nothing at all, which is the
    /// shape a mechanic ships green and inert in.
    #[test]
    fn a_windbox_launches_the_body_without_stunning_it() {
        let gust = crate::HitKnockback {
            flinchless: true,
            ..hard_knockback()
        };
        let mut vel = ae::Vec2::new(120.0, 0.0);
        let mut flight = ae::BodyFlightState::default();
        let mut combat = BodyCombat::default();
        apply_body_hit_reaction(
            &mut vel,
            &mut flight,
            &mut combat,
            ae::Vec2::new(20.0, 0.0),
            1.0,
            ae::Vec2::new(0.0, 1.0),
            false,
            Some(&gust),
            // A gust deals no damage — the authored zero the damage floor was
            // taught to preserve.
            0,
            ae::Vec2::ZERO,
            VictimStance::default(),
            None,
            None,
            None,
            feel(),
        );

        assert_ne!(
            vel,
            ae::Vec2::new(120.0, 0.0),
            "the gust did not move the body at all, so 'it pushes without \
             stunning' is only half true and the half that matters is missing"
        );
        assert_eq!(
            combat.hitstun_timer, 0.0,
            "the gust stunned its victim, which is what makes it a weak hit \
             rather than a windbox"
        );
    }

    /// ⛔⛔ AND IT DECLINES TO CHARGE STUN, IT DOES NOT DISCHARGE IT.
    ///
    /// A body already reeling from a real hit that then drifts through a gust
    /// keeps the stun it was already in. `hitstun_timer = 0.0` on the flinchless
    /// path would have made a windbox the best combo breaker in the game — and
    /// it is the obvious way to write this, which is why it is pinned.
    #[test]
    fn a_windbox_does_not_clear_stun_the_victim_was_already_in() {
        let gust = crate::HitKnockback {
            flinchless: true,
            ..hard_knockback()
        };
        let mut vel = ae::Vec2::new(120.0, 0.0);
        let mut flight = ae::BodyFlightState::default();
        let mut combat = BodyCombat {
            hitstun_timer: 0.4,
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
            Some(&gust),
            0,
            ae::Vec2::ZERO,
            VictimStance::default(),
            None,
            None,
            None,
            feel(),
        );
        assert_eq!(
            combat.hitstun_timer, 0.4,
            "a gust wiped the stun a real hit had already charged"
        );
    }

    /// A WINDBOX PUSHES YOU AND LEAVES YOU PLAYING.
    ///
    /// ⛔⛔ IT TOOK THE HARD CONTROL LOCK ANYWAY. The flinchless arm correctly
    /// declines to charge `hitstun_timer` — and `recoil_lock_timer` was assigned
    /// four lines later UNCONDITIONALLY, so a volume whose whole authored
    /// contract is "moves you and leaves you in control" removed all authority
    /// for several frames, and a REPEATING gust refreshed that every pulse.
    ///
    /// ⭐ AND IT MUST NOT DISCHARGE ONE EITHER, which is the same asymmetry the
    /// stun arm states: a body already reeling from a real hit that drifts
    /// through a gust keeps the beat it was already owed. Arm two pins that, so
    /// the fix cannot become "a windbox clears your stagger" — which would make
    /// it the best combo breaker in the game.
    #[test]
    fn a_windbox_neither_charges_nor_clears_the_control_lock() {
        let gust = || crate::HitKnockback {
            flinchless: true,
            ..hard_knockback()
        };
        let push = |already_locked: f32| -> BodyCombat {
            let mut vel = ae::Vec2::new(120.0, 0.0);
            let mut flight = ae::BodyFlightState::default();
            let mut combat = BodyCombat {
                recoil_lock_timer: already_locked,
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
                Some(&gust()),
                0,
                ae::Vec2::ZERO,
                VictimStance::default(),
                None,
                None,
                None,
                feel(),
            );
            combat
        };

        // ⛔ THE CONTROL FIRST: an ordinary hit must still charge the lock, or
        // the arms below would pass on a rule that stopped locking anything.
        let (_, _, struck) = react(false);
        assert!(
            struck.recoil_lock_timer > 0.0,
            "an ordinary knockback charged no control lock, so this fixture \
             cannot tell a windbox from a hit"
        );

        assert_eq!(
            push(0.0).recoil_lock_timer,
            0.0,
            "a WINDBOX took the hard control lock — its authored contract is that \
             it moves you and leaves you playing, and a repeating one would \
             refresh that every pulse"
        );
        assert!(
            (push(0.25).recoil_lock_timer - 0.25).abs() < 1e-6,
            "a windbox CLEARED a lock the body was already owed — declining to \
             charge is not the same as discharging, and this direction would make \
             a gust the best combo breaker in the game"
        );
    }

    /// ⭐⭐ A HIT ENDS THE HELPLESS EPISODE AND REFUNDS NOTHING.
    ///
    /// The reaction already hands the air dodge back — *"a launched fighter that
    /// could not dodge would have no answer to the follow-up"* — and until
    /// 2026-08-25 helplessness was `recovery_charges == 0`, a resource reading
    /// nothing but a landing-shaped refresh could end. So the dodge came back to
    /// a fighter still forbidden to use it.
    ///
    /// ⛔ THE SECOND ASSERTION IS THE LOAD-BEARING ONE. Ending the episode must
    /// not restore the CHARGE — a spent recovery stays spent through an
    /// edge-guard hit, exactly as the spent double jump does, and that is a
    /// deliberate correction this must not undo.
    #[test]
    fn a_hit_ends_the_helpless_episode_without_giving_the_recovery_back() {
        let mut vel = ae::Vec2::new(120.0, 0.0);
        let mut flight = ae::BodyFlightState::default();
        let mut combat = BodyCombat::default();
        let mut dodge = ae::BodyDodgeState {
            air_dodge_spent: true,
            ..Default::default()
        };
        let mut jump = ae::BodyJumpState {
            recovery_charges: 0,
            post_recovery_helpless: true,
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
            Some(&mut dodge),
            Some(&mut jump),
            None,
            feel(),
        );

        assert!(
            !dodge.air_dodge_spent,
            "the hit did not hand the air dodge back, so the fixture is not the \
             one the rule is about"
        );
        assert!(
            !jump.post_recovery_helpless,
            "the fighter kept its helpless episode through an accepted hit, so \
             the dodge it was just handed is one it may not use"
        );
        assert_eq!(
            jump.recovery_charges, 0,
            "the hit gave the RECOVERY back — a spent recovery stays spent \
             through an edge-guard hit, exactly as the double jump does"
        );
    }
}
