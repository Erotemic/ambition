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
    // still forbidden to act cannot use it.
    //
    // ⭐ AND THE RECOVERY COMES BACK WITH THE EPISODE — see the call site. The
    // DOUBLE JUMP still does not, which is the distinction the two rules turn
    // on: a spent midair jump is a resource an edge-guard took from you, and
    // freefall is a punishment for having spent the recovery. Lifting the
    // punishment without returning the thing is neither rule.
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

    // ⛔⛔ …EXCEPT FOR A GUST, AND THAT IS THE WHOLE OF THIS BLOCK'S EXCEPTION.
    // A windbox is authored as *"pushes its victim and does nothing else"*, and
    // every fact below is an INJURY the victim is owed for having been hurt. A
    // gust did not hurt anybody. Reducing the pulse to `flinchless` lost that:
    // the wind still refunded an air dodge, still cleared post-recovery
    // helplessness and still charged hitlag, so blowing on a recovering fighter
    // handed it a fresh dodge and a way out of freefall — a windbox was the
    // best rescue tool in the game and nobody had authored it to be.
    let gust = knockback.is_some_and(|k| k.is_windbox());

    // ⛔ BEFORE any launch, so the hang is gone by the time a velocity is
    // written: dropping it afterwards would let the ledge constraint eat the
    // launch the same hit just handed out.
    //
    // ⭐⭐ AND A GUST DOES THIS ONE TOO — BY ITS OWN RULE, not by inheritance.
    // Blowing a fighter off the edge it is holding is the POINT of a stage
    // windbox, and the genre's gusts do exactly that. It is stated here rather
    // than left to fall out of the injury block above, because the reason is
    // different: the others are things a body is owed for being HURT, and this
    // one is a thing that happens to a body that is PUSHED.
    if let Some((model, ledge)) = ledge {
        ae::movement::knock_off_ledge(model, ledge);
        // ⛔⛔ AND THE WIRE GOES WITH THE HANG, for the reason the hang does: a
        // constraint that survived the hit would eat the launch the hit just
        // handed out. A body on a flyline has its position written from
        // `(anchor, length, angle)` every tick, so a knockback velocity applied
        // underneath it is overwritten on the very next frame and the fighter
        // rides serenely on up — the trapdoor's deleted leap, in a mode that
        // lasts long enough to see.
        //
        // ⭐ THE CUT WRITES NO VELOCITY. Whatever hit her owns her motion now;
        // see `cut_the_wire`, which is deliberately not the wire's own release.
        ae::movement::cut_the_wire(model);
    }
    // The air dodge is spent per airtime, and a hit is a new airtime's worth of
    // trouble — a launched fighter that could not dodge would have no answer to
    // the follow-up. A body without the ability never spent it, so this is a
    // no-op for one.
    if let Some(dodge) = dodge {
        if !gust {
            dodge.air_dodge_spent = false;
        }
    }
    // …and the fighter is allowed to use it. THE RECOVERY IT GETS BACK IS
    // BELOW, past the launch filter, because it is owed to a FLINCH and this
    // block is everything an accepted hit owes whatever it launches.
    //
    // ⛔⛔ THIS USED TO GIVE BACK NO CHARGE, under a comment saying so was
    // deliberate and must not be undone. It was wrong, and Jon named the rule it
    // was wrong about (2026-08-26): *"In Smash Ultimate, the normal rule is that
    // if you have used an up-B that puts you into special fall / helplessness,
    // then an opponent hits you hard enough to cause flinch, that hit clears
    // helplessness. Once hitstun ends, you can act again, INCLUDING using your
    // up-B again."*
    //
    // ⭐ THE OLD COMMENT WAS RIGHT ABOUT THE DOUBLE JUMP AND GENERALISED FROM
    // IT. A spent midair jump genuinely does stay spent through an edge-guard
    // hit — that is what makes taking somebody's second jump worth doing, and it
    // is still true two lines up. The recovery is the opposite case: freefall is
    // a PUNISHMENT for having spent it, and a hit that lifts the punishment
    // while withholding the thing punished for leaves a fighter free to act with
    // nothing to act with. The genre gives both back together or neither.
    //
    // ⭐ AND THE EPISODE FLAG IS NOW REDUNDANT HERE RATHER THAN LOAD-BEARING —
    // cleared anyway, because a body whose charge came back must not be reading
    // as helpless for even one tick, and because the flag is what
    // `body_is_helpless` actually asks.
    // ONE tuning row for the whole reaction, so the launch and the hitstun
    // cannot disagree about which feel numbers this hit uses (FB6b).
    let response = hit_response_tuning(&feel, boss_hit);
    // HITLAG IS THE HIT, not the launch. The freeze on contact is what makes a
    // hit read as a hit at all: an armoured trade or a damage-only tick that
    // passed through in silence would look like a whiff to both players.
    //
    // ⭐⭐ AND A GUST EARNS NONE, WHICH IS ALSO WHAT RETIRES THE MATCH FREEZE.
    // `request_impact_hitstop_on_resolved_hits` arms the global freeze off the
    // hitlag the resolver reports and returns early when that is zero — so the
    // windbox stops stopping the world as a CONSEQUENCE of owing no beat,
    // rather than by a second rule taught to the freeze about wind. There is
    // one fact here, not two, and `is_a_connect` never had to learn about it.
    if !gust {
        combat.hitstop_timer = combat
            .hitstop_timer
            .max(ae::hit_response::hitlag_duration(damage, &response));
    }

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
    // ⭐⭐ THE FLINCH REFUND, AND ITS PLACE IS THE RULE. Everything above is what
    // an ACCEPTED hit owes; this is what a FLINCH owes, and Jon named the
    // difference: *"an opponent hits you hard enough to cause FLINCH, that hit
    // clears helplessness"*. Two hits get this far and should not: a damage-only
    // tick (a hazard, a poison, a chip) authors no knockback at all, and a
    // strike eaten by SUPER ARMOR is one the body did not answer for. Neither
    // flinches anybody, and the early return above is precisely the line between
    // them and a hit that does.
    //
    // ⛔⛔ IT WAS ABOVE THAT LINE and therefore fired for both — so a poison tick
    // handed a helpless fighter its recovery back, and so did a hit its own
    // armour ate. The first draft of this rule put it beside the air dodge
    // because they read as one paragraph; they are not, and the reviewer caught
    // it. The DODGE is owed to any accepted hit by its own contract (*"a
    // launched fighter that could not dodge would have no answer to the
    // follow-up"*) and stays where it was.
    //
    // ⛔ STILL NOT A GUST. A windbox authors real knockback and so reaches this
    // line, and *"what a windbox declines is the INJURY, not the physics"* — a
    // fighter blown out of freefall by wind was the best rescue tool in the game
    // and nobody authored it to be.
    //
    // ⛔ AND STILL NOT THE DOUBLE JUMP. A spent midair jump stays spent through
    // an edge-guard; freefall is a punishment for spending the RECOVERY, and
    // lifting the punishment without returning the thing punished for is neither
    // rule.
    if let Some(jump) = jump {
        if !gust {
            jump.post_recovery_helpless = false;
            jump.recovery_charges = ae::DEFAULT_RECOVERY_CHARGES;
        }
    }
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
    flight.stage_launch(launch, knockback.is_windbox());
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
    if !knockback.is_windbox() {
        combat.hitstun_timer = ae::hit_response::hitstun_duration(Some(knockback), &response);
        // ⭐⭐ AND A REAL HIT WAKES A SLEEPING BODY. A status that outlasted
        // being struck would let one Sing carry a whole stock: the victim
        // cannot act, and hitting them does not change that, so there is no
        // counterplay at all. Waking here rather than in the status's own tick
        // puts it next to the hitstun it replaces — the body stops being ASLEEP
        // and starts being HIT, which is a different helplessness with an end
        // the attacker earned.
        //
        // ⛔ INSIDE THE WINDBOX GUARD, on that comment's own reasoning one line
        // up: a flinchless gust declines to charge stun, so it must not
        // discharge a sleep either. Blowing a sleeping fighter across the stage
        // should not wake them — the wake is what a real hit buys.
        combat.sleep_timer = 0.0;
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
    if !knockback.is_windbox() {
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
    /// hurtbox. See [`ambition_platformer2d::combat::rules::DeclaredCombatRules::crouch_cancel_scale`].
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
            reaction: ae::hit_response::HitReaction::Strike,
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
            reaction: ae::hit_response::HitReaction::Windbox,
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
            reaction: ae::hit_response::HitReaction::Windbox,
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

    /// ⛔⛔ A GUST OWES NOTHING AN INJURY OWES — the four facts of being HIT,
    /// none of which a windbox may hand out.
    ///
    /// The authored contract is *"pushes its victim and does nothing else — no
    /// damage, no hitstun, no shield"*, and it was true of exactly one of them.
    /// The pulse reached the victim as a `flinchless: bool`, which named the
    /// stun and nothing more, so `apply_body_hit_reaction`'s accepted-hit block
    /// still ran in full: the gust REFUNDED the air dodge, CLEARED post-recovery
    /// helplessness and CHARGED hitlag before the flinchless branch was reached.
    ///
    /// ⭐⭐ THE VICTIM HERE IS THE ONE THAT MAKES IT MATTER: a fighter deep in a
    /// recovery, dodge spent and helpless, is exactly who a windbox is aimed at
    /// off the ledge — and blowing on them gave them their dodge back and let
    /// them act out of freefall. A windbox was the best rescue tool in the game
    /// and nobody authored it to be one.
    ///
    /// ⭐ THE STRIKE ARM IS THE PREMISE GUARD. Without it every assertion below
    /// would pass against a function that had simply stopped granting these to
    /// ANYBODY, which is a different bug wearing the same green.
    #[test]
    fn a_gust_refunds_no_dodge_clears_no_helplessness_and_earns_no_hitlag() {
        /// Push a fully-spent recovering victim with `reaction` and report
        /// (dodge spent, still helpless, hitlag charged, moved).
        fn push(reaction: ae::hit_response::HitReaction) -> (bool, bool, f32, bool) {
            let pulse = crate::HitKnockback {
                reaction,
                ..hard_knockback()
            };
            let mut vel = ae::Vec2::new(120.0, 0.0);
            let mut flight = ae::BodyFlightState::default();
            let mut combat = BodyCombat::default();
            let mut dodge = ae::BodyDodgeState {
                air_dodge_spent: true,
                ..Default::default()
            };
            let mut jump = ae::BodyJumpState {
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
                Some(&pulse),
                // ⭐ NONZERO, because hitlag is charged off the DAMAGE. A gust
                // authors zero, but asking with zero would make the hitlag
                // assertion pass for the wrong reason — the arm has to prove the
                // KIND refuses the beat, not that the number was small.
                9,
                ae::Vec2::ZERO,
                VictimStance::default(),
                Some(&mut dodge),
                Some(&mut jump),
                None,
                feel(),
            );
            (
                dodge.air_dodge_spent,
                jump.post_recovery_helpless,
                combat.hitstop_timer,
                vel != ae::Vec2::new(120.0, 0.0),
            )
        }

        let (gust_dodge, gust_helpless, gust_lag, gust_moved) =
            push(ae::hit_response::HitReaction::Windbox);
        let (hit_dodge, hit_helpless, hit_lag, hit_moved) =
            push(ae::hit_response::HitReaction::Strike);

        // The premise: a real blow DOES grant all three. Without this the arms
        // below are satisfied by a function that grants them to nobody.
        assert!(!hit_dodge, "a real hit stopped refunding the air dodge");
        assert!(
            !hit_helpless,
            "a real hit stopped clearing post-recovery helplessness"
        );
        assert!(hit_lag > 0.0, "a real hit stopped charging hitlag");
        assert!(
            hit_moved && gust_moved,
            "one of the two pulses did not push at all"
        );

        assert!(
            gust_dodge,
            "the gust handed the victim its air dodge back — a windbox is not an \
             injury and owes no recovery from one"
        );
        assert!(
            gust_helpless,
            "the gust let a helpless fighter act again, which is the rescue \
             nobody authored"
        );
        assert_eq!(
            gust_lag, 0.0,
            "the gust charged hitlag. That is also what armed the whole-match \
             freeze: `request_impact_hitstop_on_resolved_hits` reads the \
             resolver's hitlag and returns early at zero, so a gust that earns \
             no beat stops the world for nobody"
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
            reaction: ae::hit_response::HitReaction::Windbox,
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

    /// ⭐⭐ A FLINCHING HIT ENDS THE HELPLESS EPISODE **AND GIVES THE RECOVERY
    /// BACK** — BUT NOT THE DOUBLE JUMP.
    ///
    /// ⛔⛔ THIS TEST USED TO ASSERT THE OPPOSITE, by name, calling it *"a
    /// deliberate correction this must not undo"*. It was the wrong rule, and it
    /// is worth recording that the wrongness was invisible from inside: the
    /// reasoning — a spent resource stays spent through an edge-guard — is
    /// exactly right about the double jump and was generalised one resource too
    /// far. Jon named the genre rule, 2026-08-26: *"if you have used an up-B
    /// that puts you into special fall / helplessness, then an opponent hits you
    /// hard enough to cause flinch, that hit clears helplessness. Once hitstun
    /// ends, you can act again, INCLUDING using your up-B again."*
    ///
    /// ⭐ THE THREE ASSERTIONS ARE THREE DIFFERENT RESOURCES AND ALL THREE ARE
    /// LOAD-BEARING, because the failure this replaces was one rule swallowing
    /// its neighbour. Freefall is a PUNISHMENT for spending the recovery, so
    /// lifting it while withholding the recovery leaves a fighter free to act
    /// with nothing to act with; a midair jump is not a punishment for anything
    /// and taking somebody's second jump has to stay worth doing.
    #[test]
    fn a_flinching_hit_gives_the_recovery_back_but_not_the_double_jump() {
        let mut vel = ae::Vec2::new(120.0, 0.0);
        let mut flight = ae::BodyFlightState::default();
        let mut combat = BodyCombat::default();
        let mut dodge = ae::BodyDodgeState {
            air_dodge_spent: true,
            ..Default::default()
        };
        let mut jump = ae::BodyJumpState {
            recovery_charges: 0,
            // SPENT, so the assertion below has somewhere to fail: a fixture
            // that arrived with a jump left could not tell "the hit kept its
            // hands off it" from "the hit handed one back".
            air_jumps_available: 0,
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
            jump.recovery_charges,
            ae::DEFAULT_RECOVERY_CHARGES,
            "the flinching hit did NOT give the recovery back, so the fighter is \
             out of freefall and still has nothing to recover with"
        );
        assert_eq!(
            jump.air_jumps_available, 0,
            "the hit handed back a spent DOUBLE JUMP — taking somebody's second \
             jump has to stay worth doing, and that half of the old rule was right"
        );
    }

    /// ⛔⛔ AND THREE HITS THAT DO **NOT** FLINCH GIVE IT BACK.
    ///
    /// The first draft of the refund sat beside the air-dodge refund, above the
    /// launch filter, because the two read as one paragraph. They are not one
    /// paragraph, and the difference is exactly Jon's threshold: the dodge is
    /// owed to any ACCEPTED hit, the recovery to a FLINCH.
    ///
    /// Three arms, because there are three distinct ways past "accepted" that
    /// are not a flinch, and a single arm would leave the other two open:
    /// a damage-only tick authors no knockback at all; super armor means the
    /// body did not answer for the hit it took; and a windbox declines the
    /// injury while keeping the physics.
    ///
    /// ⚠ THE POSITIVE ARM ABOVE CANNOT PROVE THIS. It shows a flinch refunds,
    /// which is equally true of an implementation that refunds on ANYTHING —
    /// which is what the first draft did.
    ///
    /// PROBED RED: with the refund moved back above the launch filter, the
    /// damage-only arm fails first.
    #[test]
    fn a_hit_that_does_not_flinch_gives_no_recovery_back() {
        /// One reaction over a spent, helpless fighter. Returns what its budget
        /// looked like afterwards.
        fn spent_fighter_after(
            knockback: Option<ae::hit_response::HitKnockback>,
            armored: bool,
        ) -> ae::BodyJumpState {
            let mut vel = ae::Vec2::new(120.0, 0.0);
            let mut flight = ae::BodyFlightState::default();
            let mut combat = BodyCombat {
                armored,
                ..Default::default()
            };
            let mut dodge = ae::BodyDodgeState {
                air_dodge_spent: true,
                ..Default::default()
            };
            let mut jump = ae::BodyJumpState {
                recovery_charges: 0,
                air_jumps_available: 0,
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
                knockback.as_ref(),
                12,
                ae::Vec2::ZERO,
                VictimStance::default(),
                Some(&mut dodge),
                Some(&mut jump),
                None,
                feel(),
            );
            jump
        }

        // A DAMAGE-ONLY TICK — a hazard, a poison, a chip. It authors no
        // knockback, so nothing throws the body and nothing flinches it.
        let poison = spent_fighter_after(None, false);
        assert_eq!(
            poison.recovery_charges, 0,
            "a damage-only tick handed a helpless fighter its recovery back"
        );
        assert!(
            poison.post_recovery_helpless,
            "and it let the fighter out of freefall, which is the same bug \
             pointed the other way — free to act with nothing to act with"
        );

        // SUPER ARMOR — the hit landed and the body did not answer for it.
        let armored = spent_fighter_after(Some(hard_knockback()), true);
        assert_eq!(
            armored.recovery_charges, 0,
            "a hit the body's own armour ate refreshed its recovery"
        );
        assert!(armored.post_recovery_helpless);

        // A GUST — real knockback, declined injury. It reaches the launch filter
        // and must still be refused, which is why `!gust` survives beside it.
        let mut wind = hard_knockback();
        wind.reaction = ae::hit_response::HitReaction::Windbox;
        let blown = spent_fighter_after(Some(wind), false);
        assert_eq!(
            blown.recovery_charges, 0,
            "wind refreshed a recovery — a windbox would be the best rescue \
             tool in the game and nobody authored it to be"
        );
        assert!(blown.post_recovery_helpless);
    }
}
