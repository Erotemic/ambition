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
//! ⛔ **it consumes no press and cancels no jump.** A footstool that had to
//! arbitrate with the air jump would need to run before movement and take the
//! edge away, which is a scheduling argument this does not have to win: it
//! writes both bodies' velocity AFTER the kernel has, so whatever the press also
//! produced, the footstool is what the tick ends with.
//!
//! ⚠ **a body whose [`ae::FootstoolTuning`] is `OFF` — the engine default — is
//! not a platform and cannot stand on one.** Heads are not platforms in the
//! exploration game, and a wandering enemy that could be jumped off would be a
//! different game for that side.

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
    pub faction: &'static crate::features::ActorFaction,
    pub team: Option<&'static ambition_combat::targeting::MatchTeam>,
    pub control: Option<&'static ambition_characters::brain::ActorControl>,
    pub frame: Option<&'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    pub brain: Option<&'static ambition_characters::brain::Brain>,
}

impl FootstoolBodyItem<'_, '_> {
    fn gravity_dir(&self) -> ae::Vec2 {
        self.frame
            .map(|f| f.basis().down)
            .unwrap_or(ae::DEFAULT_GRAVITY_DIR)
    }
}

/// **Every footstool that happens this tick.**
///
/// # Why the order is spelled out
///
/// Two bodies can be standing on the same head, and three can be stacked. Taking
/// whichever pair the query yields first makes the outcome depend on archetype
/// order — stable within a run and NOT stable across a rollback resimulation,
/// which is the definition of a desync. Pairs are therefore sorted by the two
/// bodies' stable [`SimId`]s, and a head that has already been jumped off this
/// tick is spent.
/// ⚠ **a [`ParamSet`], because the two passes want the same components two
/// ways.** The deciding pass reads `BodyKinematics` off every body and the
/// applying pass writes it on two of them; asking for both as plain queries is
/// an access conflict Bevy refuses at initialisation. The set is what makes the
/// two-pass shape legal, and the shape is what makes the outcome deterministic.
pub fn apply_footstools(
    mut bodies: ParamSet<(
        Query<FootstoolBody, Without<ambition_characters::brain::ScriptedControl>>,
        Query<(
            &mut ae::BodyKinematics,
            &mut ambition_characters::actor::BodyCombat,
            &mut ae::BodyComboTrace,
        )>,
    )>,
    tuning: Option<Res<ambition_combat::rules::ResolvedCombatTuning>>,
) {
    let friendly_fire = tuning.map(|t| t.friendly_fire()).unwrap_or_default();
    let mut pairs: Vec<(SimId, SimId, Entity, Entity, ae::FootstoolTuning, ae::Vec2)> = Vec::new();

    {
        let decide = bodies.p0();
        for stomper in decide.iter() {
            // The press is the whole trigger. Without it a fighter would footstool
            // every time an exchange put it briefly above somebody, which turns the
            // mechanic from a read into an accident.
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
            // Coming DOWN onto the head, or already resting on it. Rising into a
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
                if !ambition_combat::targeting::damage_lands_between(
                    *stomper.faction,
                    ambition_combat::targeting::effective_faction(*victim.faction, victim.brain),
                    stomper.team,
                    victim.team,
                    friendly_fire,
                    None,
                    victim.entity,
                ) {
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
    let mut effects = bodies.p1();

    let mut spent_heads: Vec<Entity> = Vec::new();
    for (_, _, stomper, victim, rules, gravity_dir) in pairs {
        if spent_heads.contains(&victim) {
            continue;
        }
        spent_heads.push(victim);

        // ⚠ SET, not add. A body arriving at terminal velocity and one barely
        // falling must leave a head at the same speed, or the hop a player
        // learns is a function of how far they fell to get there.
        if let Ok((mut kin, _, mut trace)) = effects.get_mut(stomper) {
            kin.vel = kin.vel - gravity_dir * (kin.vel.dot(gravity_dir) + rules.rise_speed);
            trace.combo.push(ae::ComboMark {
                op: ae::MovementOp::Footstool,
                age: 0.0,
            });
        }
        if let Ok((mut kin, mut combat, _)) = effects.get_mut(victim) {
            kin.vel = kin.vel - gravity_dir * (kin.vel.dot(gravity_dir) - rules.press_speed);
            // The stomped body's half of the cost, and it is a HARD lock rather
            // than hitstun: being stood on is not being hit, and the reason it
            // is dangerous is that you cannot answer it for a moment.
            combat.recoil_lock_timer = combat.recoil_lock_timer.max(rules.victim_stun);
        }
    }
}

#[cfg(test)]
mod tests;
