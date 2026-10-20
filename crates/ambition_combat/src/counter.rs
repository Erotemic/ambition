//! A parry caught in a counter stance answers with the stance's authored
//! technique (`smash.counter`).
//!
//! No new defensive mechanic. A perfect shield already denies a qualifying
//! attack and names who it denied (`ParriedBodyHit`); a stance holds its parry
//! window open (`ambition.combat.parry_window`); and any published technique
//! can be fired by key. A counter is those three in sequence, so the answer is
//! a technique key: `smash.capture_attempt` gives a parry into a grab, a slow
//! gives Witch Time.
//!
//! The vocabulary is the engine's, as capture's is: what a stance answers
//! with, and on whom, is the stance's authored data (`CounterParams`).

use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_characters::brain::ActorActionMessage;
use ambition_entity_catalog::smash_counter::{CounterParams, COUNTER};
use bevy::prelude::*;

use crate::hitbox::ParriedBodyHit;
use crate::moveset::MovePlayback;

/// A parry that a counter stance caught answers with the stance's technique.
///
/// The stance is read back off the move, not remembered in a component: the
/// live window is the stance. This asks which window is under the playback
/// clock and whether it sustains `smash.counter`. `MovePlayback` is already
/// rollback state, so there is nothing extra to register.
///
/// A parry without a stance is silent: an ordinary parry is complete on its
/// own.
pub fn answer_parries_from_counter_stances(
    mut parries: MessageReader<ParriedBodyHit>,
    playbacks: Query<&MovePlayback>,
    mut actions: MessageWriter<ActorActionMessage>,
) {
    for parry in parries.read() {
        let Ok(playback) = playbacks.get(parry.defender) else {
            continue;
        };
        let Some(stance) = live_counter_stance(playback) else {
            continue;
        };
        // Whom the answer is aimed at, as the stance says: a riposte answers on
        // its owner; a Witch-Time slow answers on the fighter who swung.
        let target = if stance.answers_the_attacker { parry.attacker } else { parry.defender };
        actions.write(ActorActionMessage {
            actor: target,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::Special(stance.response.clone()),
                params: stance.response_params.clone(),
            },
            move_instance: None,
        });
    }
}

/// The counter stance a body is standing in right now, if any.
///
/// The window must be the one under the clock. Scanning every window would
/// also counter during startup and recovery.
pub fn live_counter_stance(playback: &MovePlayback) -> Option<CounterParams> {
    playback
        .spec
        .windows
        .iter()
        .filter(|w| w.start_s <= playback.t && playback.t < w.end_s)
        .find_map(|w| {
            let effect = w.sustain_effect.as_ref()?;
            if effect.key != COUNTER {
                return None;
            }
            match effect.params.hydrate::<CounterParams>() {
                Ok(params) => Some(params),
                Err(err) => {
                    warn!("a live counter stance did not hydrate its params: {err}");
                    None
                }
            }
        })
}
