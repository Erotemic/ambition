//! The game half of the Limit meter: who gains what, and when.
//!
//! Four sources fill one meter, and none of them is a new authority. Time comes
//! from `WorldTime`, damage from `ResolvedBodyHit` (the amount that landed), and
//! a move-driven fill from an ordinary technique. The meter is the
//! `smash_limit::LIMIT` slot of a body's `ActorResources`. This module decides
//! what goes into a meter, not what a meter is.
//!
//! It also does not decide who has a meter. Only the match's seats get a Limit
//! slot, so other bodies (Ambition's player, room enemies) are not reachable.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::engine_core::resources::{ActorResources, ResourceLevel};
use ambition_platformer2d::entity_catalog::smash_limit::{
    FillMeterParams, LimitMeterFill, FILL_METER, LIMIT,
};

fn limit_of(bank: &mut ActorResources) -> Option<&mut ResourceLevel> {
    bank.level_of_mut(&LIMIT)
}

/// The match's Limit rule. A game that never inserts one fills nothing.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct SmashLimitFill(pub LimitMeterFill);

/// This ruleset's Limit. Its declaration is also the match's seat resource, so
/// a seat is built with the meter this rule fills.
pub const SMASH_LIMIT: LimitMeterFill = LimitMeterFill::JONS_BASELINE;

/// Fill every seated fighter's meter from the clock and from the hits that
/// landed this tick.
///
/// One system for all sources, because they are one decision about one tick of
/// meter. Split systems would give three answers to "did this fighter cross the
/// cap this frame", and the move that spends a full meter reads that.
/// Is guarding the safe option rather than the greedy one under this fill?
///
/// This is a smash balance rule, so it lives here and not in
/// `LimitMeterFill::problems()`. That check only answers "is this fill well
/// formed"; a future meter can reward defensive play on purpose.
///
/// In this game blocking is the safe option. A meter that pays more for guarding
/// than for taking the hit makes guarding the best play, and damage stops being
/// a cost.
pub fn guarding_is_the_safe_option(fill: &LimitMeterFill) -> bool {
    fill.on_block <= 0.0 || fill.on_block < fill.on_damage_taken
}

pub fn fill_limit_meters(
    rule: Option<Res<SmashLimitFill>>,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut hits: MessageReader<ambition_platformer2d::combat::hitbox::ResolvedBodyHit>,
    mut blocks: MessageReader<ambition_platformer2d::combat::hitbox::BlockedBodyHit>,
    mut meters: Query<&mut ActorResources>,
) {
    let Some(rule) = rule else {
        return;
    };
    let fill = rule.0;
    if fill.cap <= 0.0 {
        return;
    }
    let dt = time.sim_dt();

    for mut bank in &mut meters {
        let Some(limit) = limit_of(&mut bank) else {
            continue;
        };
        if dt > 0.0 && fill.per_second > 0.0 {
            limit.refill(fill.per_second * dt);
        }
        // Fill first, then drain. When the two rates are equal, a meter
        // authored to hold steady does not drift by one frame each tick.
        //
        // Floored at zero, not wrapped. A negative meter would need a refill
        // past zero before a priced move is available again.
        if dt > 0.0 && fill.decay_per_second > 0.0 {
            limit.drain(fill.decay_per_second * dt);
        }
    }

    for hit in hits.read() {
        // Always the victim; the attacker only if one is known. A blast zone,
        // a hazard or a stage spike has no attacker, and the stage must not
        // pay a fighter for killing their opponent.
        if let Some(limit) = meters.get_mut(hit.victim).ok().and_then(|bank| limit_of(bank.into_inner())) {
            limit.refill(fill.taken(hit.damage));
        }
        if let Some(attacker) = hit.attacker {
            // Not for hitting yourself: a self-damaging move would pay twice.
            if attacker != hit.victim {
                if let Some(limit) = meters.get_mut(attacker).ok().and_then(|bank| limit_of(bank.into_inner())) {
                    limit.refill(fill.dealt(hit.damage));
                }
            }
        }
    }

    // A successful block pays the fighter who blocked.
    //
    // The defender only, never the attacker. The loop above pays an attacker
    // through `dealt()` for damage done; a blocked strike did none. Paying here
    // would let attacks into a shield charge the attacker's meter.
    //
    // Not gated on a known attacker. `BlockedBodyHit::attacker` is `None` for a
    // hazard, but the defender still blocked it.
    for block in blocks.read() {
        if let Some(limit) = meters.get_mut(block.victim).ok().and_then(|bank| limit_of(bank.into_inner())) {
            limit.refill(fill.blocked());
        }
    }
}

/// The *"cloud like meter, where a move fills it"* case: an authored technique
/// that charges its own owner.
pub fn apply_authored_meter_fills(
    mut actions: MessageReader<ActorActionMessage>,
    mut meters: Query<&mut ActorResources>,
) {
    for message in actions.read() {
        let ambition_platformer2d::characters::brain::action_set::ActionRequest::Special {
            spec,
            params,
        } = &message.request
        else {
            continue;
        };
        let ambition_platformer2d::characters::brain::action_set::SpecialActionSpec::Special(key) =
            spec;
        if key != FILL_METER {
            continue;
        }
        let Ok(params) = params.hydrate::<FillMeterParams>() else {
            warn!("a meter fill did not hydrate its params");
            continue;
        };
        // A body that holds no Limit gains nothing: the technique names the
        // Limit, not whichever meter its user happens to carry.
        if let Some(limit) = meters
            .get_mut(message.actor)
            .ok()
            .and_then(|bank| limit_of(bank.into_inner()))
        {
            limit.refill(params.amount);
        }
    }
}

#[cfg(test)]
mod tests;
