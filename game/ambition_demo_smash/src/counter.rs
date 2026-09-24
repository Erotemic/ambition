//! The counter, assembled from parts the engine already had.
//!
//! No new defensive mechanic. The perfect shield already denies a qualifying
//! attack and names who it denied; a move can hold an authored window open;
//! and any published technique can be fired by key. A counter is those three
//! in sequence.
//!
//! So the response is a technique key: `smash.capture_attempt` gives a
//! parry-into-grab; a resource technique could give a Revenge gauge; a slow
//! gives Witch Time.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_counter::{CounterParams, COUNTER};
use ambition_platformer2d::combat::hitbox::ParriedBodyHit;
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core as ae;

/// Hold the parry window open for as long as a counter stance is authored.
///
/// Every frame, not once: `parry_window_timer` decays, so a stance that opened
/// the window only on its first frame would stop catching part-way through.
/// The sustained effect fires every live frame, so re-arming each frame fits.
///
/// It does not raise the shield. `parrying()` reads the timer alone; setting
/// `active` would add a held shield's blocking, guard cost and shieldstun. A
/// counter is not a guard.
pub fn hold_counter_parry_windows(
    mut actions: MessageReader<ActorActionMessage>,
    mut guards: Query<&mut ae::BodyShieldState>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != COUNTER {
            continue;
        }
        let params: CounterParams = match params.hydrate() {
            Ok(p) => p,
            Err(err) => {
                warn!("smash counter params did not hydrate: {err}");
                continue;
            }
        };
        let Ok(mut shield) = guards.get_mut(message.actor) else {
            continue;
        };
        shield.parry_window_timer = shield.parry_window_timer.max(params.window_s);
        // The absorb mode is armed on the same heartbeat, so a stance that
        // stops re-arming stops absorbing and parrying on the same tick.
        if params.absorbs_projectiles {
            shield.absorb_window_timer = shield.absorb_window_timer.max(params.window_s);
        }
    }
}

/// A parry that a counter stance caught answers with the stance's authored
/// technique.
///
/// The stance is read back off the move, not remembered in a component: the
/// live window is the stance. This asks which window is under the playback
/// clock and whether it sustains `smash.counter`. `MovePlayback` is already
/// rollback state, so there is nothing extra to register.
///
/// A parry without a stance is silent: an ordinary parry is complete on its
/// own.
pub fn answer_a_parry_with_the_authored_counter(
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
        let target = if stance.answers_the_attacker {
            parry.attacker
        } else {
            parry.defender
        };
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
fn live_counter_stance(playback: &MovePlayback) -> Option<CounterParams> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::entity_catalog::smash_capture::CAPTURE_ATTEMPT;

    /// The riposte the shipped contract authors, not a fixture built here.
    ///
    /// Taken from the shipped contract on purpose, so the move under test is
    /// the move a player presses.
    fn shipped_riposte() -> ambition_platformer2d::entity_catalog::MoveSpec {
        crate::moveset::fighter_moveset()
            .moves
            .into_iter()
            .find(|m| m.id == "riposte")
            .expect("the shared contract authors a riposte")
    }

    fn app_with_counter_systems() -> App {
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_message::<ParriedBodyHit>();
        app.add_systems(
            Update,
            (
                hold_counter_parry_windows,
                answer_a_parry_with_the_authored_counter,
            ),
        );
        app
    }

    /// A parry caught while standing in the riposte answers with the authored
    /// response.
    ///
    /// No counter mechanic exists: a successful defence emits the technique
    /// the author named, here the command grab.
    #[test]
    fn a_parry_in_the_riposte_stance_answers_with_the_authored_technique() {
        let riposte = shipped_riposte();
        // Mid-stance: the Active window is the second of three.
        let stance = riposte.windows[1].clone();
        let mut playback = MovePlayback::new(riposte, 1.0);
        playback.t = (stance.start_s + stance.end_s) * 0.5;

        let mut app = app_with_counter_systems();
        let defender = app.world_mut().spawn(playback).id();
        let attacker = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(ParriedBodyHit {
            defender,
            attacker,
            hitbox: attacker,
            contact: ae::Vec2::ZERO,
        });
        app.update();

        let answers: Vec<String> = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .drain()
            .filter_map(|m| match m.request {
                ActionRequest::Special { spec, .. } if m.actor == defender => {
                    let SpecialActionSpec::Special(key) = spec;
                    Some(key)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            answers,
            vec![CAPTURE_ATTEMPT.to_string()],
            "a parry caught inside the riposte's stance did not answer with the \
             authored technique, so the counter is a move that stands there"
        );
    }

    /// A Witch-Time stance answers the fighter who swung, not its owner.
    ///
    /// `ParriedBodyHit::attacker` carries the attacker. Both arms: a dispatcher
    /// that aimed everything at the attacker would send every riposte to the
    /// wrong body.
    #[test]
    fn a_stance_that_answers_the_attacker_aims_there_and_the_others_still_do_not() {
        let aimed_at = |answers_the_attacker: bool| -> Vec<Entity> {
            let mut app = app_with_counter_systems();
            let defender = app.world_mut().spawn_empty().id();
            let attacker = app.world_mut().spawn_empty().id();
            let mut spec = shipped_riposte();
            for window in &mut spec.windows {
                if let Some(effect) = window.sustain_effect.as_mut() {
                    if effect.key == COUNTER {
                        let mut params = effect
                            .params
                            .hydrate::<CounterParams>()
                            .expect("the shipped riposte hydrates");
                        params.answers_the_attacker = answers_the_attacker;
                        effect.params =
                            ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params)
                                .expect("re-serialize");
                    }
                }
            }
            // Mid-stance: at `t = 0` the playback is in startup, and both arms
            // would agree on an empty list.
            let stance = spec.windows[1].clone();
            let mut playback = MovePlayback::new(spec, 1.0);
            playback.t = (stance.start_s + stance.end_s) * 0.5;
            app.world_mut().entity_mut(defender).insert(playback);
            let hitbox = app.world_mut().spawn_empty().id();
            app.world_mut().write_message(ParriedBodyHit {
                defender,
                attacker,
                hitbox,
                contact: ae::Vec2::ZERO,
            });
            app.update();
            let messages = app
                .world()
                .resource::<bevy::ecs::message::Messages<ActorActionMessage>>();
            let mut cursor = messages.get_cursor();
            cursor.read(messages).map(|m| m.actor).collect()
        };

        let owner_run = aimed_at(false);
        assert!(
            !owner_run.is_empty(),
            "poison: this fixture dispatches nothing at all, so neither arm below \
             says anything about targeting"
        );
        let attacker_run = aimed_at(true);
        assert_ne!(
            owner_run, attacker_run,
            "the stance aimed at the same body either way ({owner_run:?}) — the \
             flag is not being read, so a Witch-Time slows its own caster"
        );
    }

    /// A parry caught outside the stance answers with nothing. Without this, a
    /// scan of every window would pass the test above and counter from frames
    /// the author closed.
    #[test]
    fn a_parry_outside_the_stance_answers_with_nothing() {
        let riposte = shipped_riposte();
        let recovery = riposte.windows[2].clone();
        let mut playback = MovePlayback::new(riposte, 1.0);
        playback.t = (recovery.start_s + recovery.end_s) * 0.5;

        let mut app = app_with_counter_systems();
        let defender = app.world_mut().spawn(playback).id();
        let attacker = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(ParriedBodyHit {
            defender,
            attacker,
            hitbox: attacker,
            contact: ae::Vec2::ZERO,
        });
        app.update();

        let answered = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .drain()
            .any(|m| m.actor == defender);
        assert!(
            !answered,
            "a parry landed during the riposte's RECOVERY still fired its \
             answer, so the authored stance window means nothing"
        );
    }

    /// The stance opens a parry window and does not raise a shield.
    ///
    /// A raised shield would add blocking, guard cost and shieldstun. An
    /// assertion on `parrying()` alone cannot tell a default guard from a
    /// raised one, so this checks `active` too.
    #[test]
    fn the_stance_arms_a_parry_without_raising_a_shield() {
        let mut app = app_with_counter_systems();
        let body = app
            .world_mut()
            .spawn(ae::BodyShieldState::default())
            .id();
        let params = ambition_platformer2d::entity_catalog::smash_counter::CounterParams {
            window_s: 0.05,
            // Its own answer, as for every counter except the clerk's.
            answers_the_attacker: false,
            response: CAPTURE_ATTEMPT.to_string(),
            response_params: Default::default(),
            absorbs_projectiles: false,
        };
        app.world_mut().write_message(ActorActionMessage {
            actor: body,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::Special(COUNTER.to_string()),
                params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params)
                    .expect("counter params serialize"),
            },
            move_instance: None,
        });
        app.update();

        let shield = app
            .world()
            .get::<ae::BodyShieldState>(body)
            .expect("the body carries a guard");
        assert!(
            shield.parrying(),
            "the counter stance did not put the body into a parry, so the move \
             is a pose: timer={}",
            shield.parry_window_timer
        );
        assert!(
            !shield.active,
            "the counter stance RAISED THE SHIELD. `parrying()` is the timer \
             alone, so this buys the catch nothing and costs the move a held \
             guard's blocking, integrity and shieldstun"
        );
    }
}
