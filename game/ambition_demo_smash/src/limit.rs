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

/// Give every meter the match's cap, and empty it, BEFORE anything prices a move
/// against it.
///
/// ⛔⛔ THIS EXISTS BECAUSE DYING WOULD OTHERWISE GRANT THE LIMIT. A stock loss
/// keeps the body entity and calls `reset_body_clusters`, which does
/// `*clusters.mana = BodyMana::default()` — a 100-point pool that starts FULL,
/// since `ResourceMeter::new` sets `current` to `max`. That runs in
/// `CombatSet::Settle`. The emptying used to live only in `fill_limit_meters`,
/// which runs in `CombatSet::ContentFlavor` — AFTER `CombatSet::Trigger`, where a
/// move's cost is checked. ⇒ For one frame a respawned fighter read 100/100
/// against a Limit priced at the match's cap, and respawn protection permits a
/// swing, so the comeback move was free on the frame after dying.
///
/// ⭐ IT FIXES THE CLASS, NOT THE RESPAWN. Any path that hands a body a fresh
/// `BodyMana` — a respawn today, a possession or a spawn tomorrow — is corrected
/// before costs are read, because this asks the same question `fill_limit_meters`
/// asked and asks it a phase earlier.
///
/// ⚠ AND IT DOES NOT SPLIT THE THREE FILL SOURCES, which `fill_limit_meters`'
/// own doc argues against: the clock, the hits and the authored fill stay
/// together there. This is a one-time NORMALISATION of a meter wearing another
/// rule's shape, and it answers no question about crossing the cap this frame.
pub fn adopt_the_limit_cap(
    rule: Option<Res<SmashLimitFill>>,
    mut meters: Query<&mut ae::BodyMana>,
) {
    let Some(rule) = rule else {
        return;
    };
    let cap = rule.0.cap;
    if cap <= 0.0 {
        return;
    }
    for mut mana in &mut meters {
        if mana.meter.max != cap {
            mana.meter.max = cap;
            // A Limit starts EMPTY; a mana pool starts full. See the note in
            // `fill_limit_meters` for the guard that found this the first time.
            mana.meter.current = 0.0;
        }
    }
}

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
    mut blocks: MessageReader<ambition_platformer2d::combat::hitbox::BlockedBodyHit>,
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
        // ⛔ THE ADOPTION MOVED OUT — see `adopt_the_limit_cap`. This branch is
        // kept as a SAFETY NET for a meter that arrives between the two systems,
        // not as the place adoption happens: by the time this runs, `Trigger`
        // has already priced a move against the meter.
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
        if let Ok(mut mana) = meters.get_mut(block.victim) {
            mana.meter.refill(fill.blocked());
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
