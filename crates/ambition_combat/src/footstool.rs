//! Footstool interaction: jumping off another body.
//!
//! Contact is therefore judged from the previous tick's resolved positions.
//!
//! Both bodies must enable footstools, share a gravity frame, and satisfy the
//! match's teammate policy. A victim already committed to a move receives no
//! reaction; otherwise grounded victims flinch and airborne victims tumble.
//! Decisions are collected read-only, ordered deterministically, then applied.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// What a body must BE to take either end of a footstool.
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
    pub model: &'static ae::MotionModel,
    pub health: &'static ambition_characters::actor::BodyHealth,
    /// The world's hands are off this body. NOT redundant with `health`: a
    /// fighter waiting out its death beat has already had `health.reset()`
    /// called on it, so it reads ALIVE for the whole interlude.
    pub out_of_play: bevy::prelude::Has<crate::death_rules::OutOfPlay>,
    pub team: Option<&'static crate::targeting::MatchTeam>,
    pub control: Option<&'static ambition_characters::control::ActorControl>,
    /// The swing clock, read ONLY to ask whether this body is mid-move — see
    /// the phantom footstool in the module doc.
    pub melee: Option<&'static crate::components::BodyMelee>,
    pub frame: Option<&'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
}

impl FootstoolBodyItem<'_, '_> {
    fn gravity_dir(&self) -> ae::Vec2 {
        self.frame
            .map(|f| f.basis().down)
            .unwrap_or(ae::DEFAULT_GRAVITY_DIR)
    }
}

/// Two bodies agree which way is down, closely enough to say whose head is
/// whose. A pair under materially different gravity is refused rather than
/// judged in one of the two frames.
fn frames_agree(a: ae::Vec2, b: ae::Vec2) -> bool {
    a.dot(b) > 0.999
}

/// May `stomper` stand on `victim`? — the TEAM question, asked directly.
///
///  deliberately not `damage_lands_between`. What the genre gates a teammate
/// footstool on is Team Attack, which is this same flag; asking the damage
/// question instead would make a mechanic that deals no damage depend on a
/// policy about dealing it.
fn team_permits(
    stomper: Option<&crate::targeting::MatchTeam>,
    victim: Option<&crate::targeting::MatchTeam>,
    friendly_fire: bool,
) -> bool {
    match (stomper, victim) {
        (Some(a), Some(b)) if a == b => friendly_fire,
        _ => true,
    }
}

/// Claim the press for every footstool that is about to happen.
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
        Query<FootstoolBody, Without<ambition_characters::control::ScriptedControl>>,
        Query<(
            &mut ae::BodyKinematics,
            &mut ae::BodyJumpState,
            &mut ambition_characters::actor::BodyCombat,
            &mut ae::MotionModel,
        )>,
    )>,
    tuning: Option<Res<crate::rules::ResolvedCombatTuning>>,
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
                || crate::util::body_is_untouchable(Some(stomper.health), stomper.out_of_play)
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
                    || crate::util::body_is_untouchable(Some(victim.health), victim.out_of_play)
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

    //  THE CLAIM IS THIS TICK'S JUMP EDGE, and clearing it HERE is the only
    // thing that makes that true. The kernel spends the claim inside its
    // footstool branch — but that branch is not first: a wall jump, a ground
    // jump, a coyote jump, a ladder jump and the one-way drop-through all
    // resolve the same press ahead of it. A body that qualified for a footstool
    // and whose press went to a wall jump instead kept the claim, and the NEXT
    // airborne press spent it with nobody underneath.
    //
    //  clearing before the accepted pairs are stamped makes the lifetime
    // structural instead of a discipline: a claim that loses input arbitration
    // cannot outlive the tick that made it, whatever new branch is added above.
    //
    //  guarded rather than written flat, because an unconditional `false` would
    // mark every body's `BodyJumpState` changed every tick.
    for (_, mut jump, _, _) in effects.iter_mut() {
        if jump.footstool_claimed {
            jump.footstool_claimed = false;
        }
    }

    for (_, _, stomper, victim, rules, gravity_dir, victim_grounded, victim_mid_move) in pairs {
        // Spend both participants: one stomper can footstool once, and one victim
        // can be footstooled once per resolution pass.
        if spent.contains(&stomper) || spent.contains(&victim) {
            continue;
        }
        spent.push(stomper);
        spent.push(victim);

        // The stomper's half is a CLAIM on its own jump press. The kernel writes
        // the rise, ahead of the air jump, and emits the op.
        //  and the bounce carries i-frames. Ultimate gives four frames, and
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
        //  the PHANTOM footstool: a committed body follows through. The
        // stomper's claim above still stands — that is the whole point of the
        // technique — so this skips the reaction and not the pair.
        if victim_mid_move {
            continue;
        }
        if let Ok((mut kin, _, mut combat, mut model)) = effects.get_mut(victim) {
            let flinch =
                ae::footstool_victim(&mut model, &mut kin, victim_grounded, gravity_dir, rules);
            // A HARD lock rather than hitstun: being stood on is not being hit,
            // and what makes it dangerous is that you cannot answer it.  zero
            // when a tumble started, which owns control for longer than this
            // would have.
            combat.recoil_lock_timer = combat.recoil_lock_timer.max(flinch);
        }
    }
}

#[cfg(test)]
mod tests;
