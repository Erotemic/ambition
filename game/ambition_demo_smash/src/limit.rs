//! The game half of the Limit meter: who gains what, and when.
//!
//! ⭐⭐ FOUR SOURCES, ONE METER, AND NONE OF THEM IS A NEW AUTHORITY. Time comes
//! from `WorldTime`, damage from `ResolvedBodyHit` (which now carries the amount
//! that actually landed), and a move-driven fill from an ordinary technique. The
//! meter is `BodyMana`, which was already rollback-canonical and already what
//! `MoveGates::meter_cost` spends. ⇒ This system decides nothing about what a
//! meter IS; it decides what goes into one.
//!
//! ⛔ AND IT DOES NOT DECIDE WHO HAS ONE. `LimitMeterFill::default()` fills
//! nothing, so a match that declares no Limit gets exactly the behaviour every
//! match had before this existed.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_limit::{
    FillMeterParams, LimitMeterFill, FILL_METER,
};
use ambition_platformer2d::engine_core as ae;

/// The match's Limit rule. A game that never inserts one fills nothing.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct SmashLimitFill(pub LimitMeterFill);

/// Fill every seated fighter's meter from the clock and from the hits that
/// landed this tick.
///
/// ⛔⛔ ONE SYSTEM FOR THREE SOURCES, because they are one decision about one
/// tick's worth of meter. Splitting them would make "did this fighter cross the
/// cap this frame" a question with three answers, and the move that spends a
/// full meter reads exactly that.
pub fn fill_limit_meters(
    rule: Option<Res<SmashLimitFill>>,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut hits: MessageReader<ambition_platformer2d::combat::hitbox::ResolvedBodyHit>,
    mut meters: Query<&mut ae::BodyMana>,
) {
    let Some(rule) = rule else {
        return;
    };
    let fill = rule.0;
    if fill.cap <= 0.0 {
        return;
    }
    let dt = time.sim_dt();

    // ⭐ THE CAP IS THE RULE'S, NOT THE COMPONENT'S DEFAULT. `BodyMana::default()`
    // is a 100-point meter; a match that declares a 60-point Limit means 60, and
    // a fighter whose meter still says 100 would need 100 to spend a move priced
    // at the cap. ⇒ Set once, here, where the rule is known.
    for mut mana in &mut meters {
        if mana.meter.max != fill.cap {
            mana.meter.max = fill.cap;
            // ⛔⛔ AND A LIMIT STARTS EMPTY. `BodyMana::default()` is a MANA POOL
            // and a mana pool starts FULL — `ResourceMeter::new` sets `current`
            // to `max` — so adopting the cap without this would open every match
            // with every fighter's Limit already spendable. ⇒ Found by a guard
            // that expected two seconds of clock to read 1.0 and got 60.0.
            //
            // ⚠ ONCE, on adoption, which is what the `max != cap` test buys: a
            // fighter mid-match is not re-emptied by a system that runs every
            // tick.
            mana.meter.current = 0.0;
        }
        if dt > 0.0 && fill.per_second > 0.0 {
            mana.meter.refill(fill.per_second * dt);
        }
        // ⛔ FILL FIRST, THEN DRAIN, which is the order `ResourceMeter::tick`
        // itself documents ("regen first, then decay") and it matters when the
        // two rates are equal: a meter authored to hold steady holds steady
        // instead of drifting by one frame's worth every tick.
        //
        // ⚠ FLOORED AT ZERO, NOT WRAPPED. A meter that went negative would need
        // to be refilled past zero before a priced move became reachable again,
        // which is a debt nobody authored.
        if dt > 0.0 && fill.decay_per_second > 0.0 {
            mana.meter.current = (mana.meter.current - fill.decay_per_second * dt).max(0.0);
        }
    }

    for hit in hits.read() {
        // ⛔ THE VICTIM ALWAYS, THE ATTACKER ONLY IF THE ROAD KNOWS ONE. A blast
        // zone, a hazard and a stage spike all resolve with no attacker, and a
        // meter that credited "somebody" for those would pay a fighter for the
        // stage killing their opponent.
        if let Ok(mut mana) = meters.get_mut(hit.victim) {
            mana.meter.refill(fill.taken(hit.damage));
        }
        if let Some(attacker) = hit.attacker {
            // ⛔ AND NOT FOR HITTING YOURSELF. A self-damaging move — a recoil, a
            // hazard the caster walked into — would otherwise pay twice.
            if attacker != hit.victim {
                if let Ok(mut mana) = meters.get_mut(attacker) {
                    mana.meter.refill(fill.dealt(hit.damage));
                }
            }
        }
    }
}

/// The *"cloud like meter, where a move fills it"* case: an authored technique
/// that charges its own owner.
pub fn apply_authored_meter_fills(
    mut actions: MessageReader<ActorActionMessage>,
    mut meters: Query<&mut ae::BodyMana>,
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
        if let Ok(mut mana) = meters.get_mut(message.actor) {
            mana.meter.refill(params.amount);
        }
    }
}

#[cfg(test)]
mod tests;
