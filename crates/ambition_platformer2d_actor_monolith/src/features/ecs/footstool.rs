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
//! ⚠ **PARTIAL against the genre, and named rather than implied**: every victim
//! takes the same shove and the same lock, while Ultimate distinguishes a
//! grounded target (a brief freeze) from an airborne one (a tumble), and has no
//! phantom-footstool rule here — a target executing a move is interrupted, and
//! in Ultimate it would not be.

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
        )>,
    )>,
    tuning: Option<Res<ambition_combat::rules::ResolvedCombatTuning>>,
) {
    let friendly_fire = tuning.is_some_and(|t| t.friendly_fire().enabled);
    let mut pairs: Vec<(SimId, SimId, Entity, Entity, ae::FootstoolTuning, ae::Vec2)> = Vec::new();

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
                ));
            }
        }
    }

    pairs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    let mut spent: Vec<Entity> = Vec::new();
    let mut effects = bodies.p1();
    for (_, _, stomper, victim, rules, gravity_dir) in pairs {
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
        if let Ok((_, mut jump, _)) = effects.get_mut(stomper) {
            jump.footstool_claimed = true;
        }
        // The victim's half is written now, so the kernel integrates it this
        // tick rather than a tick late.
        //
        // ⚠ SET, not add: a body arriving at terminal velocity and one barely
        // falling must be driven down at the same speed, or being stood on costs
        // more the further your attacker fell to reach you.
        if let Ok((mut kin, _, mut combat)) = effects.get_mut(victim) {
            let along = kin.vel.dot(gravity_dir);
            kin.vel -= gravity_dir * (along - rules.press_speed);
            // A HARD lock rather than hitstun: being stood on is not being hit,
            // and what makes it dangerous is that you cannot answer it.
            combat.recoil_lock_timer = combat.recoil_lock_timer.max(rules.victim_stun);
        }
    }
}

#[cfg(test)]
mod tests;
