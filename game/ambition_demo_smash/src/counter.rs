//! The counter, end to end in this game.
//!
//! The stance is the module `ambition_demo_smash_modules::counter`: each
//! tick it is live, it holds its fighter's parry window open
//! (`ambition.combat.parry_window`). A parry caught in the stance answers with
//! the stance's authored technique (`ambition_combat::counter`), so the
//! response is a technique key: `smash.capture_attempt` gives a
//! parry-into-grab; a slow gives Witch Time.
//!
//! What stays here are the tests of the counter this game authors.

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::module_road::{app_on, runner_of, Road};
    use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
    use ambition_platformer2d::characters::brain::ActorActionMessage;
    use ambition_platformer2d::combat::counter::answer_parries_from_counter_stances;
    use ambition_platformer2d::combat::hitbox::ParriedBodyHit;
    use ambition_platformer2d::combat::moveset::MovePlayback;
    use ambition_platformer2d::engine_core as ae;
    use ambition_platformer2d::entity_catalog::smash_capture::CAPTURE_ATTEMPT;
    use ambition_platformer2d::entity_catalog::smash_counter::{CounterParams, COUNTER};

    /// The riposte the shipped contract authors, not a fixture built here.
    ///
    /// Taken from the shipped contract on purpose, so the move under test is
    /// the move a player presses.
    fn shipped_riposte() -> ambition_platformer2d::entity_catalog::MoveSpec {
        crate::smash_pack::PACK.moveset(crate::SMASH_CHARACTER_ID)
            .moves
            .into_iter()
            .find(|m| m.id == "riposte")
            .expect("the shared contract authors a riposte")
    }

    /// The answer to a caught parry, as the engine gives it.
    fn app_with_counter_systems() -> App {
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_message::<ParriedBodyHit>();
        app.add_message::<ambition_platformer2d::combat::guard::DeflectedBodyHit>();
        app.add_systems(Update, answer_parries_from_counter_stances);
        app
    }

    /// The stance through its module on `road`.
    fn stance_app(road: Road) -> App {
        app_on(road, ambition_demo_smash_modules::counter::module)
    }

    fn guard(app: &mut App) -> Entity {
        app.world_mut().spawn((ae::BodyShieldState::default(), ae::BodyKinematics::default())).id()
    }

    /// One tick of a stance of `window_s` on `body`.
    fn stand(app: &mut App, body: Entity, window_s: f32, absorbs_projectiles: bool) {
        let params = CounterParams {
            window_s,
            // Its own answer, as for every counter except the clerk's.
            answers_the_attacker: false,
            response: CAPTURE_ATTEMPT.to_string(),
            response_params: Default::default(),
            absorbs_projectiles,
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
    }

    fn shield(app: &App, body: Entity) -> ae::BodyShieldState {
        app.world().get::<ae::BodyShieldState>(body).expect("the body carries a guard").clone()
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
        let mut app = stance_app(Road::Linked);
        let body = guard(&mut app);
        stand(&mut app, body, 0.05, false);

        let shield = shield(&app, body);
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

    /// The loaded file's stance is the linked one's: the same window and
    /// absorb. Its runner is the loaded file.
    #[test]
    fn the_stance_from_its_module_file_parries_as_the_linked_one_does() {
        let mut windows_on = Vec::new();
        for road in [Road::Linked, Road::Loaded] {
            let mut app = stance_app(road);
            let body = guard(&mut app);
            stand(&mut app, body, 0.05, true);
            let runner = runner_of(&app, COUNTER);
            assert_eq!(runner.starts_with("Loaded"), road == Road::Loaded, "{road:?}: {runner}");
            let shield = shield(&app, body);
            windows_on.push((shield.parry_window_timer, shield.absorb_window_timer));
        }
        assert_eq!(windows_on[1], windows_on[0], "the loaded stance held a window unlike the linked one");
        assert!(windows_on[0].0 > 0.0 && windows_on[0].1 > 0.0, "premise: the linked stance parries and absorbs");
    }

    /// A stance's re-arm never cuts a longer window short, and a stance that
    /// does not absorb opens no absorb.
    #[test]
    fn a_short_re_arm_keeps_the_longer_window_and_absorbs_only_when_authored() {
        let mut app = stance_app(Road::Linked);
        let body = guard(&mut app);
        stand(&mut app, body, 0.5, false);
        stand(&mut app, body, 0.05, false);
        let shield = shield(&app, body);
        assert!(
            shield.parry_window_timer > 0.05,
            "a short re-arm cut the window to {}",
            shield.parry_window_timer
        );
        assert_eq!(shield.absorb_window_timer, 0.0, "a stance that does not absorb opened an absorb");
    }

    /// A window with no end is refused by the engine, even from a module that
    /// a move's admission did not check: it would parry for the rest of the
    /// match. (A negative window needs no refusal to be harmless: the longer
    /// of it and the guard's is the guard's.)
    #[test]
    fn a_window_with_no_end_is_refused() {
        let mut app = stance_app(Road::Linked);
        let body = guard(&mut app);
        stand(&mut app, body, f32::INFINITY, true);
        let shield = shield(&app, body);
        assert_eq!((shield.parry_window_timer, shield.absorb_window_timer), (0.0, 0.0), "an endless window was held");
    }
}
