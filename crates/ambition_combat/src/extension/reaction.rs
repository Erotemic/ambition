//! The adapters of a reaction to a hit: `ambition.combat.parried`,
//! `ambition.combat.answer_parry` and `ambition.combat.struck`. See the port
//! cards in `ambition_combat_port::reaction`.
//!
//! A counter is a parry caught inside a stance. The engine says which parry
//! was caught in which stance, and presses the technique a module answers
//! with; what a stance answers with, and on whom, is the module's. A strike
//! that carries an effect is said to the effect's module on the body it
//! struck, which holds what the effect leaves there.

use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d_core::replay_messages::ReplayMessageReader;
use ambition_characters::brain::ActorActionMessage;
use ambition_combat_port::{AnswerParryPort, BuryPort, CrumplePort, FreezePort, MendPort, ParalyzePort, RecoilPort, SeizeStruckPort, Parried, ParriedPort, Struck};
pub use ambition_combat_port::StruckPort;
use ambition_entity_catalog::ParamValue;
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::HIT_REACTION;
use ambition_extension_sdk::Port as _;
use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::sim_id::SimId;

use crate::hitbox::ParriedBodyHit;
use crate::moveset::MovePlayback;
use crate::on_hit::OnHitEffectMessage;

/// Install the parry trigger and the answer in `hit_reaction`.
pub fn install_for_hit_reaction(app: &mut App) {
    // The adapters read the parries and write the presses, so a composition
    // without combat's own schedule (a module harness) still has both.
    app.add_message::<ParriedBodyHit>();
    app.add_message::<OnHitEffectMessage>();
    app.add_message::<ActorActionMessage>();
    app.init_resource::<TickParries>();
    app.install_extension_trigger::<ParriedPort, _>(
        HIT_REACTION,
        "ambition_combat",
        queue_parries.run_if(ambition_extension_host::trigger_is_bound::<ParriedPort>),
    );
    app.install_extension_submitted_request::<AnswerParryPort, _>(HIT_REACTION, "ambition_combat", lower_parry_answers);
    app.install_extension_trigger::<StruckPort, _>(
        HIT_REACTION,
        "ambition_combat",
        queue_strikes.run_if(ambition_extension_host::trigger_is_bound::<StruckPort>),
    );
    app.install_extension_submitted_request::<ParalyzePort, _>(HIT_REACTION, "ambition_combat", lower_paralyses);
    app.install_extension_submitted_request::<BuryPort, _>(HIT_REACTION, "ambition_combat", lower_burials);
    app.install_extension_submitted_request::<FreezePort, _>(HIT_REACTION, "ambition_combat", lower_freezes);
    app.install_extension_submitted_request::<CrumplePort, _>(HIT_REACTION, "ambition_combat", lower_crumples);
    app.install_extension_submitted_request::<MendPort, _>(HIT_REACTION, "ambition_combat", lower_mends);
    app.install_extension_submitted_request::<RecoilPort, _>(HIT_REACTION, "ambition_combat", lower_recoils);
    app.add_message::<crate::capture::CaptureAttemptRequested>();
    app.install_extension_submitted_request::<SeizeStruckPort, _>(HIT_REACTION, "ambition_combat", lower_struck_seizes);
    super::status::install_for_hit_reaction(app);
}

/// Bury each grounded body a module buried (see the port card): the burial
/// replaces the reaction of the hit that asked for it, so that hit's staged
/// launch, velocity, stun and recoil lock are dropped. A body in the air is
/// left as its hit left it.
pub fn lower_burials(
    mut outbox: ResMut<ExtensionOutbox>,
    mut bodies: Query<(
        &mut ambition_characters::actor::BodyCombat,
        &mut ambition_platformer2d_core::BodyKinematics,
        &mut ambition_platformer2d_core::BodyFlightState,
        &ambition_platformer2d_core::BodyGroundState,
    )>,
) {
    for submitted in outbox.drain::<BuryPort>(&HIT_REACTION) {
        let duration_s = submitted.value.duration_s;
        if !duration_s.is_finite() || duration_s <= 0.0 {
            warn!("extension entry {} asked for a burial of {duration_s} s; refused", submitted.entry);
            continue;
        }
        let Ok((mut combat, mut kin, mut flight, ground)) = bodies.get_mut(submitted.scope) else {
            warn!("extension entry {} asked to bury a body that has no ground to be buried in; refused", submitted.entry);
            continue;
        };
        if !ground.on_ground {
            continue;
        }
        flight.take_launch();
        kin.vel = ambition_platformer2d_core::Vec2::ZERO;
        combat.hitstun_timer = 0.0;
        combat.recoil_lock_timer = 0.0;
        if combat.bury_timer < duration_s {
            combat.bury_timer = duration_s;
        }
    }
}

/// Crumple each grounded body a module crumpled (see the port card): like a
/// burial it replaces the hit's reaction, so the hit's staged launch,
/// velocity, stun and recoil lock are dropped, and the body owes the collapse
/// the body step spends when the sag ends. A body in the air is left as its
/// hit left it.
pub fn lower_crumples(
    mut outbox: ResMut<ExtensionOutbox>,
    mut bodies: Query<(
        &mut ambition_characters::actor::BodyCombat,
        &mut ambition_platformer2d_core::BodyKinematics,
        &mut ambition_platformer2d_core::BodyFlightState,
        &ambition_platformer2d_core::BodyGroundState,
    )>,
) {
    for submitted in outbox.drain::<CrumplePort>(&HIT_REACTION) {
        let duration_s = submitted.value.duration_s;
        if !duration_s.is_finite() || duration_s <= 0.0 {
            warn!("extension entry {} asked for a crumple of {duration_s} s; refused", submitted.entry);
            continue;
        }
        let Ok((mut combat, mut kin, mut flight, ground)) = bodies.get_mut(submitted.scope) else {
            warn!("extension entry {} asked to crumple a body that has no ground to crumple on; refused", submitted.entry);
            continue;
        };
        if !ground.on_ground {
            continue;
        }
        flight.take_launch();
        kin.vel = ambition_platformer2d_core::Vec2::ZERO;
        combat.hitstun_timer = 0.0;
        combat.recoil_lock_timer = 0.0;
        if combat.crumple_timer < duration_s {
            combat.crumple_timer = duration_s;
        }
        combat.crumple_collapse_owed = true;
    }
}

/// Seize each struck body for the striker a module named (see the port card):
/// a grab aimed at that one body, judged by the grab's rules in this tick's
/// acquisition. When both bodies stand where a grab is possible, the hit's
/// reaction is dropped, as a burial drops it, because the hold replaces it; a
/// seize the grab rules would refuse anyway leaves the hit as it was.
pub fn lower_struck_seizes(
    mut outbox: ResMut<ExtensionOutbox>,
    strikers: Query<(Entity, &ambition_platformer2d_shared_tangle::sim_id::SimId)>,
    held: Query<(Entity, &crate::capture::CapturedBy)>,
    grounds: Query<&ambition_platformer2d_core::BodyGroundState>,
    mut bodies: Query<(
        &mut ambition_characters::actor::BodyCombat,
        &mut ambition_platformer2d_core::BodyKinematics,
        &mut ambition_platformer2d_core::BodyFlightState,
    )>,
    mut attempts: MessageWriter<crate::capture::CaptureAttemptRequested>,
) {
    for submitted in outbox.drain::<SeizeStruckPort>(&HIT_REACTION) {
        let ambition_combat_port::SeizeStruck { striker, hold_offset } = submitted.value;
        let victim = submitted.scope;
        let Some((captor, _)) = strikers.iter().find(|(_, id)| super::blast::credit_of(id) == striker) else {
            warn!("extension entry {} asked a credit that names no body to seize; refused", submitted.entry);
            continue;
        };
        let standing = |body: Entity| grounds.get(body).is_ok_and(|ground| ground.on_ground);
        // In a hold already, either way round: a captive, or a captor.
        let in_a_hold = |body: Entity| held.iter().any(|(captive, by)| captive == body || by.captor == body);
        if captor == victim || !standing(captor) || !standing(victim) || in_a_hold(captor) || in_a_hold(victim) {
            continue;
        }
        let Ok((mut combat, mut kin, mut flight)) = bodies.get_mut(victim) else {
            continue;
        };
        flight.take_launch();
        kin.vel = ambition_platformer2d_core::Vec2::ZERO;
        combat.hitstun_timer = 0.0;
        combat.recoil_lock_timer = 0.0;
        attempts.write(crate::capture::CaptureAttemptRequested {
            captor,
            offset: ambition_platformer2d_core::Vec2::ZERO,
            half_extents: ambition_platformer2d_core::Vec2::ZERO,
            hold_offset: ambition_platformer2d_core::Vec2::from(hold_offset),
            move_instance: None,
            target: Some(victim),
        });
    }
}

/// Mend each body a module named by its credit (see the port card): its pool
/// and its damage meter, as `BodyHealth::heal` repays them. A body out of
/// play is not mended.
pub fn lower_mends(
    mut outbox: ResMut<ExtensionOutbox>,
    mut bodies: Query<(
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        &mut ambition_characters::actor::BodyHealth,
        Has<crate::death_rules::OutOfPlay>,
    )>,
) {
    for submitted in outbox.drain::<MendPort>(&HIT_REACTION) {
        let ambition_combat_port::Mend { body, amount } = submitted.value;
        if amount <= 0 {
            warn!("extension entry {} asked to mend {amount}; refused", submitted.entry);
            continue;
        }
        let Some((_, mut health, out_of_play)) =
            bodies.iter_mut().find(|(id, _, _)| super::blast::credit_of(id) == body)
        else {
            warn!("extension entry {} asked to mend a credit that names no body; refused", submitted.entry);
            continue;
        };
        if !out_of_play {
            health.heal(amount);
        }
    }
}

/// Charge each body a module named by its credit its strike's cost (see the
/// port card): a price paid through `BodyHealth::spend`, which advances the
/// meter and leaves the pool at least 1. A body out of play does not pay.
pub fn lower_recoils(
    mut outbox: ResMut<ExtensionOutbox>,
    mut bodies: Query<(
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        &mut ambition_characters::actor::BodyHealth,
        Has<crate::death_rules::OutOfPlay>,
    )>,
) {
    for submitted in outbox.drain::<RecoilPort>(&HIT_REACTION) {
        let ambition_combat_port::Recoil { body, amount } = submitted.value;
        if amount <= 0 {
            warn!("extension entry {} asked a recoil of {amount}; refused", submitted.entry);
            continue;
        }
        let Some((_, mut health, out_of_play)) =
            bodies.iter_mut().find(|(id, _, _)| super::blast::credit_of(id) == body)
        else {
            warn!("extension entry {} asked a credit that names no body to pay a recoil; refused", submitted.entry);
            continue;
        };
        if !out_of_play {
            health.spend(amount, 1);
        }
    }
}

/// Encase each body a module froze for at least the asked time (see the port
/// card). Its motion is left alone: the block keeps the hit's launch.
pub fn lower_freezes(
    mut outbox: ResMut<ExtensionOutbox>,
    mut bodies: Query<&mut ambition_characters::actor::BodyCombat>,
) {
    for submitted in outbox.drain::<FreezePort>(&HIT_REACTION) {
        let duration_s = submitted.value.duration_s;
        if !duration_s.is_finite() || duration_s <= 0.0 {
            warn!("extension entry {} asked for a freeze of {duration_s} s; refused", submitted.entry);
            continue;
        }
        let Ok(mut combat) = bodies.get_mut(submitted.scope) else {
            warn!("extension entry {} asked to freeze a body with no combat state; refused", submitted.entry);
            continue;
        };
        if combat.freeze_timer < duration_s {
            combat.freeze_timer = duration_s;
        }
    }
}

/// Hold each body a module paralyzed for at least the asked time (see the
/// port card): a longer paralysis it already has stands.
pub fn lower_paralyses(
    mut outbox: ResMut<ExtensionOutbox>,
    mut bodies: Query<&mut ambition_characters::actor::BodyCombat>,
) {
    for submitted in outbox.drain::<ParalyzePort>(&HIT_REACTION) {
        let duration_s = submitted.value.duration_s;
        if !duration_s.is_finite() || duration_s <= 0.0 {
            warn!("extension entry {} asked for a paralysis of {duration_s} s; refused", submitted.entry);
            continue;
        }
        let Ok(mut combat) = bodies.get_mut(submitted.scope) else {
            warn!("extension entry {} asked to paralyze a body with no combat state; refused", submitted.entry);
            continue;
        };
        if combat.paralysis_timer < duration_s {
            combat.paralysis_timer = duration_s;
        }
    }
}

/// The parries of this tick that a bound stance caught: the defender, the
/// attacker and the stance's params. An answer names a parry by its index
/// here.
///
/// Derived: written again in each tick's collect from that tick's parries,
/// and read only by the lowering of the same tick.
#[derive(Resource, Default, Debug)]
pub struct TickParries(pub Vec<(Entity, Entity, ParamValue)>);

/// The sustained effects of the windows under a move's clock, in window
/// order. A window outside the clock does not count: a stance counters only
/// while it is live, never in its startup or recovery.
fn live_sustained(playback: &MovePlayback) -> impl Iterator<Item = &ambition_entity_catalog::EffectRef> {
    playback
        .spec
        .windows
        .iter()
        .filter(|w| w.start_s <= playback.t && playback.t < w.end_s)
        .filter_map(|w| w.sustain_effect.as_ref())
}

/// One invocation for each parry caught in a live window whose sustained
/// effect a module binds, in the order the parries were caught.
pub fn queue_parries(
    admitted: Res<AdmittedExtensions>,
    mut parries: ReplayMessageReader<ParriedBodyHit>,
    playbacks: Query<&MovePlayback>,
    mut table: ResMut<TickParries>,
    mut invocations: ResMut<ExtensionInvocations>,
) {
    table.0.clear();
    let bound: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.trigger.port == ParriedPort::KEY)
        .map(|e| e.descriptor.trigger.selector.as_ref())
        .collect();
    for parry in parries.read() {
        let Ok(playback) = playbacks.get(parry.defender) else {
            continue;
        };
        let Some(stance) = live_sustained(playback).find(|effect| bound.contains(&effect.key.as_str())) else {
            continue;
        };
        let index = table.0.len() as u32;
        table.0.push((parry.defender, parry.attacker, stance.params.clone()));
        invocations.trigger::<ParriedPort>(
            &HIT_REACTION,
            stance.key.clone(),
            parry.defender,
            None,
            false,
            Parried {
                parry: index,
                contact: [parry.contact.x, parry.contact.y],
                params: super::special::special_params(&stance.params),
            },
        );
    }
}

/// One invocation for each strike whose on-hit effect a module binds, on the
/// body it struck, in the order the strikes landed.
pub fn queue_strikes(
    admitted: Res<AdmittedExtensions>,
    mut strikes: ReplayMessageReader<OnHitEffectMessage>,
    identities: Query<&SimId>,
    mut invocations: ResMut<ExtensionInvocations>,
) {
    let bound: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.trigger.port == StruckPort::KEY)
        .map(|e| e.descriptor.trigger.selector.as_ref())
        .collect();
    for strike in strikes.read() {
        // The host routes a selector no entry binds to nothing anyway; this
        // spares reading the params of every other on-hit effect.
        if !bound.contains(&strike.effect.key.as_str()) {
            continue;
        }
        invocations.trigger::<StruckPort>(
            &HIT_REACTION,
            strike.effect.key.clone(),
            strike.victim,
            None,
            false,
            Struck {
                contact: [strike.contact.x, strike.contact.y],
                striker: identities.get(strike.owner).map(super::blast::credit_of).unwrap_or(0),
                params: super::special::special_params(&strike.effect.params),
            },
        );
    }
}

/// Press each answer a module asked for, on the body it named, with the part
/// of the stance's params it named.
pub fn lower_parry_answers(
    mut outbox: ResMut<ExtensionOutbox>,
    table: Res<TickParries>,
    mut actions: MessageWriter<ActorActionMessage>,
) {
    for submitted in outbox.drain::<AnswerParryPort>(&HIT_REACTION) {
        let answer = submitted.value;
        let Some((defender, attacker, stance)) = table.0.get(answer.parry as usize) else {
            warn!("extension entry {} answered parry {}, which this tick did not catch; refused", submitted.entry, answer.parry);
            continue;
        };
        // A module names only a parry of its own scope (see the port card).
        if *defender != submitted.scope {
            warn!("extension entry {} answered a parry another body caught; refused", submitted.entry);
            continue;
        }
        let params = if answer.params_at.is_empty() {
            ParamValue::default()
        } else {
            match stance.at(&answer.params_at) {
                Some(params) => params,
                None => {
                    warn!(
                        "extension entry {} answered with the params at `{}`, which the stance does not state; refused",
                        submitted.entry, answer.params_at
                    );
                    continue;
                }
            }
        };
        actions.write(ActorActionMessage {
            actor: if answer.on_attacker { *attacker } else { *defender },
            request: ActionRequest::Special { spec: SpecialActionSpec::Special(answer.key), params },
            move_instance: None,
        });
    }
}

#[cfg(test)]
mod tests;
