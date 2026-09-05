//! The counter, assembled from parts the engine already had.
//!
//! ⭐⭐ NOTHING HERE IS A DEFENSIVE MECHANIC. The perfect shield already denies a
//! qualifying attack and now names who it denied; a move can already hold an
//! authored window open; and every technique the game publishes can already be
//! fired by key. A counter is those three facts in sequence, and the whole of
//! this module is the sequence.
//!
//! ⇒ Which is why the RESPONSE is a technique key rather than a retaliation
//! built in here. Naming `smash.capture_attempt` gives a parry-into-grab; naming
//! a future resource technique gives a Revenge gauge; naming a slow gives Witch
//! Time. None of those is a case in this file.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_counter::{CounterParams, COUNTER};
use ambition_platformer2d::combat::hitbox::ParriedBodyHit;
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core as ae;

/// Hold the parry window open for as long as a counter stance is authored.
///
/// ⛔⛔ EVERY FRAME, NOT ONCE. `BodyShieldState::parrying()` is
/// `active && parry_window_timer > 0.0`, and that timer counts down like any
/// other. A stance that opened the window on its first frame would stop
/// catching part-way through its own authored window, and the move would look
/// like it had a timing bug rather than a decaying grant. The sustained effect
/// fires every frame the window is live, so re-arming per frame is the natural
/// shape rather than a workaround.
///
/// ⛔⛔ AND IT DOES NOT RAISE THE SHIELD, WHICH THE FIRST VERSION DID.
/// `parrying()` is the timer ALONE, so `active` buys the catch nothing — while
/// setting it hands the counter a held shield's other half: ordinary hits get
/// BLOCKED rather than passing, guard integrity is spent, and the body owes
/// shieldstun. A counter is not a guard, and the stance says so by leaving
/// `active` where it found it.
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
    }
}

/// A parry that a counter stance caught answers with the stance's authored
/// technique.
///
/// ⭐⭐ THE STANCE IS READ BACK OFF THE MOVE, NOT REMEMBERED. The obvious
/// implementation arms a `CounterStance` component when the sustained effect
/// fires and consumes it here — which makes the authored window and the
/// component two homes for one fact, and puts a second thing into the rollback
/// wire that only ever mirrors the first. The move's own live window IS the
/// stance, so this asks it: which window is under the playback clock right now,
/// and does it sustain `smash.counter`? A rewind restores `MovePlayback`
/// already, so the answer rewinds with it and there is nothing extra to
/// register.
///
/// ⚠ A PARRY WITHOUT A STANCE IS SILENT, AND MUST BE. An ordinary shield parry
/// is a complete mechanic on its own; only a fighter standing in an authored
/// counter answers with anything.
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
        actions.write(ActorActionMessage {
            actor: parry.defender,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::Special(stance.response.clone()),
                params: stance.response_params.clone(),
            },
        });
    }
}

/// The counter stance a body is standing in right now, if any.
///
/// ⛔ THE WINDOW MUST BE THE ONE UNDER THE CLOCK. Scanning every window of the
/// move would answer "this move has a counter somewhere in it", which is true
/// during its startup and its recovery too — so a fighter would counter with a
/// stance that had already closed.
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
    use ambition_platformer2d::characters::smash_capture::CAPTURE_ATTEMPT;

    /// The riposte the shipped contract authors, not a fixture built here.
    ///
    /// ⛔ TAKEN FROM THE CONTRACT ON PURPOSE. A test that authored its own
    /// counter would prove the systems work on a counter this test invented,
    /// which is the shape that let a boss id agree with itself under a key
    /// production never writes. The move a player presses is the move under
    /// test.
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
    /// ⭐ THE POINT OF THE WHOLE SLICE, in one assertion: no counter mechanic
    /// exists anywhere: a defence that succeeded emits a technique the author
    /// named, and that technique happens to be the command grab.
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

    /// A parry caught OUTSIDE the stance answers with nothing.
    ///
    /// ⛔ THE HALF THAT MAKES THE OTHER ONE MEAN SOMETHING. Scanning the move's
    /// windows without asking which is under the clock would pass the test above
    /// and also fire during startup and recovery — so a fighter would counter
    /// out of frames the author closed, and the stance's timing would be
    /// decoration.
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

    /// The stance opens a parry window, and does NOT raise a shield.
    ///
    /// ⛔⛔ THE SECOND HALF IS THE ONE THAT CAUGHT A REAL BUG. The first version
    /// of this system set `shield.active = true` on the belief that
    /// `parrying()` was `active && timer > 0.0` — a sentence copied from a
    /// `body_vulnerable` doc comment that was itself wrong. Poisoning the line
    /// away did not fail this test, because a defaulted guard and a raised one
    /// are indistinguishable to an assertion that only asks `parrying()`.
    /// ⇒ Raising the shield would have given every counter a held shield's
    /// blocking, its integrity cost and its shieldstun, and nothing in the
    /// fixture would have said so.
    #[test]
    fn the_stance_arms_a_parry_without_raising_a_shield() {
        let mut app = app_with_counter_systems();
        let body = app
            .world_mut()
            .spawn(ae::BodyShieldState::default())
            .id();
        let params = ambition_platformer2d::characters::smash_counter::CounterParams {
            window_s: 0.05,
            response: CAPTURE_ATTEMPT.to_string(),
            response_params: Default::default(),
        };
        app.world_mut().write_message(ActorActionMessage {
            actor: body,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::Special(COUNTER.to_string()),
                params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params)
                    .expect("counter params serialize"),
            },
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
