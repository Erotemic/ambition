//! **The footstool: jumping off another body's head.**
//!
//! The other body-vs-body interaction beside [`super::capture`], and it is built
//! the same way for the same reasons — one read-only pass that DECIDES, a
//! deterministic order over the decisions, then one pass that applies them.
//!
//! ```text
//! a CAPTURE    volume overlap        -> a relationship that outlives the move
//! a FOOTSTOOL  feet on a head + jump -> two impulses and a stun, over at once
//! ```
//!
//! ## ⭐⭐ It CLAIMS the press; it does not overwrite the result
//!
//! This runs BEFORE the movement kernel and writes
//! [`ae::BodyJumpState::footstool_claimed`], which the kernel's jump chain
//! consumes AHEAD of the air jump. So a footstool costs no air jump, and a body
//! that has spent every midair jump can still take one.
//!
//! ⛔ **the first version ran in `Settle` and merely wrote the bounce velocity
//! afterwards**, on the argument that whoever wrote velocity first, the
//! footstool was what the tick ended with. That argument is true about the
//! VELOCITY and false about everything else: the kernel had already spent an air
//! jump and emitted `MovementOp::DoubleJump`, so the identical footstool cost a
//! charge when you had one and nothing when you did not. One input edge, two
//! meanings. Arbitration has to happen before the commit, not after it.
//!
//! ⚠ **the price is that contact is judged from the PREVIOUS tick's resolved
//! positions**, because a claim must be made before this tick's movement. At
//! 60Hz that is the standard trade in this genre, and it is the correct half to
//! give up: a frame of latency on the read is invisible, and a jump charge spent
//! or not spent is not.
//!
//! ## What both ends must agree to
//!
//! ⚠ **a body whose [`ae::FootstoolTuning`] is `OFF` — the engine default — is
//! not a platform and cannot stand on one.** Heads are not platforms in the
//! exploration game, and a wandering enemy that could be jumped off would be a
//! different game for that side.
//!
//! ⚠ **the two bodies must share a gravity frame.** Each carries its own
//! resolved frame, so *whose head* is only a question with an answer when both
//! agree which way is down; a pair that disagrees is refused rather than judged
//! in the stomper's frame, which is what the first version did.
//!
//! ⚠ **and the match's team policy decides whether a teammate may be stood on.**
//! Not through `damage_lands_between` — a footstool deals no damage, and
//! borrowing the damage question made the mechanic's reachability depend on a
//! rule about hurting people. The friendly-fire flag is read directly, so a
//! teams match with Team Attack off refuses the pair, which is the genre's rule.
//!
//! ## The victim's reaction is TWO reactions, or NONE
//!
//! ⭐ **the PHANTOM footstool**: a victim who is in the middle of a move takes
//! no reaction at all and follows through, while the stomper still gets the
//! bounce. It is the genre's rule and a real technique in it — Ultimate players
//! farm the bounce off a committed opponent to escape disadvantage — and
//! without it a footstool would be a free interrupt of any attack it landed on.
//!
//!
//! ⭐ **when there IS a reaction, grounded and airborne are different mechanics, and Ultimate treats them
//! that way.** A grounded target has nowhere to be shoved and takes a brief
//! flinch — which is what makes a grounded footstool a combo STARTER — while an
//! airborne one is driven down into a tumble that cannot be cancelled early
//! (a footstool produces no real knockback, so there is nothing to meteor-cancel
//! out of). Both are techable on landing. The split itself lives in
//! [`ae::footstool_victim`], because a tumble is model-private state.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// **What a body must BE to take either end of a footstool.**
///
/// Narrower than a capture's participant list on purpose: a footstool is over
/// inside the tick that starts it, so it needs the two bodies' geometry, the
/// stomper's press, and somewhere to put the stun — and nothing that only a
/// lasting relationship would need.
#[derive(bevy::ecs::query::QueryData)]
pub struct FootstoolBody {
    pub entity: Entity,
    pub id: &'static SimId,
    pub kin: &'static ae::BodyKinematics,
    pub ground: &'static ae::BodyGroundState,
    pub model: &'static crate::features::MotionModel,
    pub health: &'static ambition_characters::actor::BodyHealth,
    pub team: Option<&'static ambition_combat::targeting::MatchTeam>,
    pub control: Option<&'static ambition_characters::brain::ActorControl>,
    /// The swing clock, read ONLY to ask whether this body is mid-move — see
    /// the phantom footstool in the module doc.
    pub melee: Option<&'static ambition_combat::components::BodyMelee>,
    pub frame: Option<&'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
}

impl FootstoolBodyItem<'_, '_> {
    fn gravity_dir(&self) -> ae::Vec2 {
        self.frame
            .map(|f| f.basis().down)
            .unwrap_or(ae::DEFAULT_GRAVITY_DIR)
    }
}

/// **Two bodies agree which way is down**, closely enough to say whose head is
/// whose. A pair under materially different gravity is refused rather than
/// judged in one of the two frames.
fn frames_agree(a: ae::Vec2, b: ae::Vec2) -> bool {
    a.dot(b) > 0.999
}

/// **May `stomper` stand on `victim`?** — the TEAM question, asked directly.
///
/// ⛔ deliberately not `damage_lands_between`. What the genre gates a teammate
/// footstool on is Team Attack, which is this same flag; asking the damage
/// question instead would make a mechanic that deals no damage depend on a
/// policy about dealing it.
fn team_permits(
    stomper: Option<&ambition_combat::targeting::MatchTeam>,
    victim: Option<&ambition_combat::targeting::MatchTeam>,
    friendly_fire: bool,
) -> bool {
    match (stomper, victim) {
        (Some(a), Some(b)) if a == b => friendly_fire,
        _ => true,
    }
}

/// **Claim the press for every footstool that is about to happen.**
///
/// # Why the order is spelled out
///
/// Two bodies can be standing on one head and one body can be over two heads.
/// Taking whichever pair the query yields first makes the outcome depend on
/// archetype order — stable within a run and NOT stable across a rollback
/// resimulation, which is the definition of a desync. Pairs are therefore sorted
/// by the two bodies' stable [`SimId`]s, and an accepted pair SPENDS BOTH ENDS:
/// one press is one footstool, and one head is jumped off once.
pub fn claim_footstools(
    mut bodies: ParamSet<(
        Query<FootstoolBody, Without<ambition_characters::brain::ScriptedControl>>,
        Query<(
            &mut ae::BodyKinematics,
            &mut ae::BodyJumpState,
            &mut ambition_characters::actor::BodyCombat,
            &mut crate::features::MotionModel,
        )>,
    )>,
    tuning: Option<Res<ambition_combat::rules::ResolvedCombatTuning>>,
) {
    let friendly_fire = tuning.is_some_and(|t| t.friendly_fire().enabled);
    type Pair = (
        SimId,
        SimId,
        Entity,
        Entity,
        ae::FootstoolTuning,
        ae::Vec2,
        bool,
        bool,
    );
    let mut pairs: Vec<Pair> = Vec::new();

    {
        let decide = bodies.p0();
        for stomper in decide.iter() {
            // The press is the whole trigger. Without it a fighter would
            // footstool every time an exchange put it briefly above somebody,
            // which turns the mechanic from a read into an accident.
            if !stomper.control.is_some_and(|c| c.0.jump_pressed) {
                continue;
            }
            if stomper.ground.on_ground
                || ambition_combat::util::body_is_corpse(Some(stomper.health))
            {
                continue;
            }
            let rules = stomper.model.footstool_tuning();
            if !rules.is_enabled() {
                continue;
            }
            let gravity_dir = stomper.gravity_dir();
            // Coming DOWN onto the head, or already resting on it. Rising INTO a
            // body from below is not a footstool; it is being under somebody.
            if !ae::collision_semantics::moving_toward_feet(stomper.kin.vel, gravity_dir) {
                continue;
            }
            let stomper_box = stomper.kin.aabb_oriented(gravity_dir);

            for victim in decide.iter() {
                if victim.entity == stomper.entity
                    || ambition_combat::util::body_is_corpse(Some(victim.health))
                {
                    continue;
                }
                // A body whose own rules say it is not a platform cannot be one,
                // whatever the stomper's rules say. Both ends opt in.
                if !victim.model.footstool_tuning().is_enabled() {
                    continue;
                }
                if !frames_agree(gravity_dir, victim.gravity_dir()) {
                    continue;
                }
                if !team_permits(stomper.team, victim.team, friendly_fire) {
                    continue;
                }
                if !ae::collision_semantics::feet_on_head(
                    stomper_box,
                    victim.kin.aabb_oriented(gravity_dir),
                    gravity_dir,
                    rules.band,
                ) {
                    continue;
                }
                pairs.push((
                    stomper.id.clone(),
                    victim.id.clone(),
                    stomper.entity,
                    victim.entity,
                    rules,
                    gravity_dir,
                    // Read HERE, from the decide pass, because the effects pass
                    // has already begun changing the answer for other pairs.
                    victim.ground.on_ground,
                    victim.melee.is_some_and(|m| m.phase().is_some()),
                ));
            }
        }
    }

    pairs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    let mut spent: Vec<Entity> = Vec::new();
    let mut effects = bodies.p1();

    // ⛔⛔ **THE CLAIM IS THIS TICK'S JUMP EDGE, and clearing it HERE is the only
    // thing that makes that true.** The kernel spends the claim inside its
    // footstool branch — but that branch is not first: a wall jump, a ground
    // jump, a coyote jump, a ladder jump and the one-way drop-through all
    // resolve the same press ahead of it. A body that qualified for a footstool
    // and whose press went to a wall jump instead kept the claim, and the NEXT
    // airborne press spent it with nobody underneath.
    //
    // ⭐ clearing before the accepted pairs are stamped makes the lifetime
    // structural instead of a discipline: a claim that loses input arbitration
    // cannot outlive the tick that made it, whatever new branch is added above.
    //
    // ⚠ guarded rather than written flat, because an unconditional `false` would
    // mark every body's `BodyJumpState` changed every tick.
    for (_, mut jump, _, _) in effects.iter_mut() {
        if jump.footstool_claimed {
            jump.footstool_claimed = false;
        }
    }

    for (_, _, stomper, victim, rules, gravity_dir, victim_grounded, victim_mid_move) in pairs {
        // ⛔ BOTH ends. A stomper over two heads takes ONE footstool, and a head
        // under two stompers is jumped off ONCE. The first version spent only
        // the victim, so one press shoved every body it happened to overlap.
        if spent.contains(&stomper) || spent.contains(&victim) {
            continue;
        }
        spent.push(stomper);
        spent.push(victim);

        // The stomper's half is a CLAIM on its own jump press. The kernel writes
        // the rise, ahead of the air jump, and emits the op.
        // ⭐ and the bounce carries i-frames. Ultimate gives four frames, and
        // they are what make a footstool an ESCAPE from disadvantage rather than
        // only a way to gain height — without them the body that just committed
        // to standing on somebody is a stationary target at head height.
        if let Ok((_, mut jump, mut combat, _)) = effects.get_mut(stomper) {
            jump.footstool_claimed = true;
            combat.damage_invuln_timer = combat.damage_invuln_timer.max(rules.stomper_invuln);
        }
        // The victim's half is written now, so the kernel integrates it this
        // tick rather than a tick late. The reaction itself is the movement
        // side's — a shove plus a tumble is model-private state, and the split
        // between a grounded flinch and an airborne tumble is a movement fact.
        // ⛔ the PHANTOM footstool: a committed body follows through. The
        // stomper's claim above still stands — that is the whole point of the
        // technique — so this skips the reaction and not the pair.
        if victim_mid_move {
            continue;
        }
        if let Ok((mut kin, _, mut combat, mut model)) = effects.get_mut(victim) {
            let flinch =
                ae::footstool_victim(&mut model, &mut kin, victim_grounded, gravity_dir, rules);
            // A HARD lock rather than hitstun: being stood on is not being hit,
            // and what makes it dangerous is that you cannot answer it. ⚠ zero
            // when a tumble started, which owns control for longer than this
            // would have.
            combat.recoil_lock_timer = combat.recoil_lock_timer.max(flinch);
        }
    }
}

#[cfg(test)]
mod tests;
