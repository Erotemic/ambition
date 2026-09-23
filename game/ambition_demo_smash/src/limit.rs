//! The game half of the Limit meter: who gains what, and when.
//!
//! ⭐⭐ FOUR SOURCES, ONE METER, AND NONE OF THEM IS A NEW AUTHORITY. Time comes
//! from `WorldTime`, damage from `ResolvedBodyHit` (which now carries the amount
//! that actually landed), and a move-driven fill from an ordinary technique. The
//! meter is the `smash_limit::LIMIT` slot of a body's `ActorResources`, which a
//! Limit-priced move spends. ⇒ This system decides nothing about what a meter
//! IS; it decides what goes into one.
//!
//! ⛔ AND IT DOES NOT DECIDE WHO HAS ONE. A body holds a Limit only because the
//! match declared one for its seats; every other body — Ambition's player, a
//! room's enemies — has no Limit slot, and nothing here can reach it.

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
/// a seat is BUILT with the meter this rule fills rather than adopted into it.
pub const SMASH_LIMIT: LimitMeterFill = LimitMeterFill::JONS_BASELINE;

/// Fill every seated fighter's meter from the clock and from the hits that
/// landed this tick.
///
/// ⛔⛔ ONE SYSTEM FOR THREE SOURCES, because they are one decision about one
/// tick's worth of meter. Splitting them would make "did this fighter cross the
/// cap this frame" a question with three answers, and the move that spends a
/// full meter reads exactly that.
/// Is guarding the SAFE option rather than the greedy one under this fill?
///
/// ⭐⭐ A SMASH BALANCE DOCTRINE, AND IT LIVES HERE BECAUSE OF WHAT IT IS. It was
/// briefly a validity rule inside `LimitMeterFill::problems()` — the generic
/// vocabulary of independent meter sources — where it would have refused to let a
/// future meter that deliberately rewards defensive play (parry 10, damage taken
/// 0) exist at all. ⇒ The mechanism answers "is this fill well formed"; whether
/// one source should outrank another is this ruleset's question.
///
/// In THIS game blocking is the safe option, so a meter paying more for guarding
/// than for eating the hit inverts the defensive read: the maximising play becomes
/// to guard, and taking damage stops being a cost.
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
        // ⛔ FILL FIRST, THEN DRAIN: it matters when the
        // two rates are equal: a meter authored to hold steady holds steady
        // instead of drifting by one frame's worth every tick.
        //
        // ⚠ FLOORED AT ZERO, NOT WRAPPED. A meter that went negative would need
        // to be refilled past zero before a priced move became reachable again,
        // which is a debt nobody authored.
        if dt > 0.0 && fill.decay_per_second > 0.0 {
            limit.drain(fill.decay_per_second * dt);
        }
    }

    for hit in hits.read() {
        // ⛔ THE VICTIM ALWAYS, THE ATTACKER ONLY IF THE ROAD KNOWS ONE. A blast
        // zone, a hazard and a stage spike all resolve with no attacker, and a
        // meter that credited "somebody" for those would pay a fighter for the
        // stage killing their opponent.
        if let Some(limit) = meters.get_mut(hit.victim).ok().and_then(|bank| limit_of(bank.into_inner())) {
            limit.refill(fill.taken(hit.damage));
        }
        if let Some(attacker) = hit.attacker {
            // ⛔ AND NOT FOR HITTING YOURSELF. A self-damaging move — a recoil, a
            // hazard the caster walked into — would otherwise pay twice.
            if attacker != hit.victim {
                if let Some(limit) = meters.get_mut(attacker).ok().and_then(|bank| limit_of(bank.into_inner())) {
                    limit.refill(fill.dealt(hit.damage));
                }
            }
        }
    }

    // ⭐⭐ A SUCCESSFUL BLOCK PAYS THE FIGHTER WHO BLOCKED, and until 2026-09-06
    // it paid nobody. `BlockedBodyHit` was read in exactly one place — to arm an
    // `OnBlock` cancel on the ATTACKER — so the defender's half of a defensive
    // exchange had no consequence at all.
    //
    // ⛔ THE DEFENDER ONLY, NEVER THE ATTACKER, and this is the one arm that
    // needs saying. The loop above pays an attacker through `dealt()` for damage
    // they actually did; a blocked strike did none. Paying them here would mean
    // throwing attacks INTO a shield charges your own meter, which rewards the
    // pressure this source exists to make costly.
    //
    // ⚠ AND NOT GATED ON KNOWING THE ATTACKER. `BlockedBodyHit::attacker` is an
    // `Option` because a hazard has no striker — but the guard still ate it, and
    // a fighter who blocks a stage spike blocked something. The defender is the
    // half this road always knows.
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
