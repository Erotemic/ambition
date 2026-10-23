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
use ambition_characters::brain::ActorActionMessage;
use ambition_combat_port::{AnswerParryPort, Parried, ParriedPort, Struck};
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
    super::status::install_for_hit_reaction(app);
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
    mut parries: MessageReader<ParriedBodyHit>,
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
    mut strikes: MessageReader<OnHitEffectMessage>,
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
