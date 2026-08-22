//! **Actors that stop THINKING when nobody is there to see them.**
//!
//! be dormant. This is important for maryo because ai slop will just walk off
//! the edge of the level before she even gets to that part of the level, so we
//! need to wake or sleep their brain depending on how close she is to them.
//! This sort of optimization will likely be generally important for any game
//! using the engine, although it's not something that should be inherent."*
//!
//! Three decisions follow from that last clause, and each is the reason this
//! module is shaped the way it is.
//!
//! ## 1. Dormancy is DECLARED, never assumed
//!
//! An actor with no [`DormancyPolicy`] is always awake. The engine attaches no
//! distance rule to anything, so "not inherent" is the default state rather
//! than an opt-out — and [`DormancyPolicy::Never`] exists so a character that
//! must keep simulating (a scripted patrol, a racing rival, a boss mid-phase)
//! can say so where a reader will find it.
//!
//! ## 2. The rule never names the player
//!
//! **"near the player" is the version that breaks on the couch**, and it would break for
//! netplay next. The wake test is *near any OBSERVER* — any body a view is composed around — so
//! one player, four on a sofa, and a remote peer are the same rule, and this module never
//! learns what a protagonist is.
//!
//! ## 3. It sleeps the BRAIN, not the body — and must CLEAR the brain's last word
//!
//! Freezing the BODY would be visibly wrong the moment it mattered: a falling enemy would hang
//! in the air at the edge of the wake radius, and a thrown one would stop mid-arc. Physics is
//! cheap; deciding is not.
//!
//! **but "a body with no control input already stands still" is FALSE here, and this module
//! shipped believing it for a few hours.** `ActorControl` is a COMPONENT, not a per-tick
//! message: `update_ecs_actors` writes it for every actor it ticks, and `integrate_sim_bodies`
//! reads it for every actor with no `Without<Dormant>` filter at all. So a sleeping actor does
//! not have "no control input" — it has a STALE one, and the body keeps integrating the last
//! thing the brain said.
//!
//! So going dormant CLEARS the frame. That is the whole difference between this
//! module doing what it says and doing nothing at all.
//!
//! ## Rollback
//!
//! [`Dormant`] is recomputed from positions every tick, before anything reads
//! it, and nothing carries across ticks — so it is derived state a restore
//! reproduces for free. That is deliberate: a memo that gates behaviour IS
//! rollback state, and the cheapest way to not get that wrong is to have no
//! memo.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;

/// **When this actor's brain may sleep.** Absent ⇒ always awake.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum DormancyPolicy {
    /// Never sleeps, whoever is watching. For an actor whose simulation IS the
    /// point: a scripted patrol that must arrive on time, a rival in a race, a
    /// boss whose phase timer is the fight.
    Never,
    /// Awake while any observer is within `radius` world units of this body.
    ///
    /// One number, not a rectangle: the wake test is a distance from a point,
    /// and a screen-shaped region would bake the camera's aspect into the
    /// simulation.
    AwakeNearObservers { radius: f32 },
}

/// **This actor's brain is asleep this tick.** Derived every tick; never
/// authored, never persisted.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dormant;

/// Recompute [`Dormant`] for every actor that declares a policy.
///
/// Runs before the brain tick, which filters `Without<Dormant>`.
///
/// The controlled body is whichever entity holds `DrivingParticipant(PlayerSlot::…)` — during
/// possession that is a DIFFERENT entity (the possessed actor). Input, abilities, camera,
/// portal viewer, and the melee lifecycle derive from"* it. Dormancy belongs on that list and
/// was not on it.
///
/// The wake radius was measuring the distance to a body nobody is looking through.
///
/// **[`DrivingParticipant`] covers every seat**, so a second player on the
/// couch still wakes the world around them with no extra wiring — which is what
/// the `PlayerEntity` version bought and is preserved here.
///
/// ⭐ **the predicate IS the query now.** This used to ask for every `Brain` and
/// then `matches!(.., Brain::Player(_))` — a filter written because *"somebody is
/// driving this body"* had nowhere of its own to live. It has one now:
/// [`ambition_characters::control::DrivingParticipant`] is authored at the seat and
/// moved by possession alone, so the observer set is a `With<>` and there is no
/// filter left to get wrong. The rule quoted above still holds — what changed is
/// that the fact it describes is no longer spelled inside an AI-policy enum.
pub fn assess_dormancy(
    mut commands: Commands,
    observers: Query<&ae::BodyKinematics, With<ambition_characters::control::DrivingParticipant>>,
    mut actors: Query<(
        Entity,
        &ae::BodyKinematics,
        &DormancyPolicy,
        Has<Dormant>,
        // The brain's last word, which must be RETRACTED when the brain sleeps.
        // `Option` because a body may carry a policy before it carries a brain.
        Option<&mut ambition_characters::control::ActorControl>,
    )>,
) {
    // Collected once rather than re-iterated per actor: the observer set is
    // tiny (one to four) and the actor set is not.
    let eyes: Vec<ae::Vec2> = observers.iter().map(|body| body.pos).collect();

    for (entity, body, policy, is_dormant, control) in &mut actors {
        let awake = match policy {
            DormancyPolicy::Never => true,
            DormancyPolicy::AwakeNearObservers { radius } => {
                // **no observers ⇒ AWAKE.** A world with nobody in it is a
                // world between activations, not a world to freeze: sleeping
                // every actor there would make a room's first frame after a
                // transition depend on which system ran first.
                eyes.is_empty()
                    || eyes
                        .iter()
                        .any(|eye| eye.distance(body.pos) <= radius.max(0.0))
            }
        };
        match (awake, is_dormant) {
            (true, true) => {
                commands.entity(entity).remove::<Dormant>();
            }
            (false, false) => {
                commands.entity(entity).insert(Dormant);
                // **RETRACT the brain's last word.** See the module doc: the
                // body integrates `ActorControl` whether or not the brain ran,
                // so a slop that fell asleep mid-stride would keep striding —
                // off the ledge this policy exists to keep it away from.
                //
                // On the TRANSITION only. Writing it every dormant tick would
                // touch a component for every sleeping actor every frame, which
                // is the cost this whole module exists to avoid, and it would
                // also overwrite anything that deliberately drives a dormant
                // body (a cutscene, a launch).
                if let Some(mut control) = control {
                    control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::markers::PlayerEntity;

    // **WHY EVERY FIXTURE BELOW SPAWNS A SEAT, NOT JUST A MARKER.**
    //
    // an observer is a body a participant is DRIVING, and that is
    // `DrivingParticipant`. A fixture that spawned `PlayerEntity` alone would
    // find NO OBSERVERS AT ALL — and "no observer nearby" is precisely this
    // system's dormancy condition. Every actor would fall asleep, so the tests
    // asserting sleep would pass for the wrong reason while only the ones
    // asserting wakefulness failed. A dead input road is half invisible from its
    // own failures.
    //
    // the possession reconcile is still chained in below and is now a NO-OP
    // here (no possession is in flight), kept so the fixtures keep running the
    // production ordering rather than a shape that only exists in a test.

    fn body_at(x: f32) -> ae::BodyKinematics {
        ae::BodyKinematics {
            pos: ae::Vec2::new(x, 0.0),
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(16.0, 32.0),
            facing: 1.0,
        }
    }

    fn app_with(policy: Option<DormancyPolicy>, actor_x: f32, observers: &[f32]) -> (App, Entity) {
        let mut app = App::new();
        // the derive runs AHEAD of its reader — see the note above `body_at`.
        app.init_resource::<crate::abilities::traversal::possession::PossessionState>();
        app.add_systems(
            Update,
            (crate::control::project_driving_participant, assess_dormancy).chain(),
        );
        for x in observers {
            // **an observer is a body being DRIVEN**, which is why this spawns a SEAT and not only
            // the `PlayerEntity` marker.
            app.world_mut().spawn((
                PlayerEntity,
                ambition_characters::control::DrivingParticipant(
                    ambition_characters::control::PlayerSlot::PRIMARY,
                ),
                body_at(*x),
            ));
        }
        let mut actor = app.world_mut().spawn(body_at(actor_x));
        if let Some(policy) = policy {
            actor.insert(policy);
        }
        let actor = actor.id();
        app.update();
        (app, actor)
    }

    fn is_dormant(app: &App, actor: Entity) -> bool {
        app.world().get::<Dormant>(actor).is_some()
    }

    /// **A POSSESSED body is the observer; the parked one is not.**
    ///
    /// Found while `ambition_content` adopted the seam: possess an actor, walk it
    /// away from the body you left behind, and the thing you are DRIVING falls
    /// asleep — and dormancy retracts its control frame, so it stops dead. The
    /// wake radius was measuring the distance to a body nobody is looking
    /// through.
    ///
    /// `markers.rs` already states the rule this test enforces: *"the controlled
    /// body is whichever entity holds `DrivingParticipant(…)` — during possession
    /// that is a DIFFERENT entity"*. Dormancy simply was not on the list of
    /// things that derive from it.
    #[test]
    fn the_driven_body_is_the_observer_not_the_parked_one() {
        let mut app = App::new();
        // the derive runs AHEAD of its reader — see the note above `body_at`.
        app.init_resource::<crate::abilities::traversal::possession::PossessionState>();
        app.add_systems(
            Update,
            (crate::control::project_driving_participant, assess_dormancy).chain(),
        );
        // The home avatar, parked at the origin and NOT being driven.
        app.world_mut().spawn((PlayerEntity, body_at(0.0)));
        // The possessed body, far away, holding the primary seat.
        app.world_mut().spawn((
            ambition_characters::control::DrivingParticipant(
                ambition_characters::control::PlayerSlot::PRIMARY,
            ),
            body_at(5_000.0),
        ));
        // An actor standing next to the possessed body.
        let actor = app
            .world_mut()
            .spawn((
                body_at(5_050.0),
                DormancyPolicy::AwakeNearObservers { radius: 400.0 },
            ))
            .id();
        app.update();
        assert!(
            !is_dormant(&app, actor),
            "an actor beside the body the player is DRIVING must be awake; \
             measuring to the parked home avatar instead is what put the \
             possessed player's own surroundings to sleep"
        );
    }

    /// The default is the one that matters: an actor that declares nothing is
    /// never touched, so adding this module changes no existing content.
    #[test]
    fn an_actor_with_no_policy_is_never_dormant() {
        let (app, actor) = app_with(None, 10_000.0, &[0.0]);
        assert!(
            !is_dormant(&app, actor),
            "no policy means the engine assumes nothing"
        );
    }

    #[test]
    fn a_far_actor_that_declared_a_radius_sleeps() {
        let (app, actor) = app_with(
            Some(DormancyPolicy::AwakeNearObservers { radius: 400.0 }),
            1_000.0,
            &[0.0],
        );
        assert!(is_dormant(&app, actor));
    }

    #[test]
    fn the_same_actor_wakes_when_an_observer_arrives() {
        let (mut app, actor) = app_with(
            Some(DormancyPolicy::AwakeNearObservers { radius: 400.0 }),
            1_000.0,
            &[0.0],
        );
        assert!(is_dormant(&app, actor), "asleep with the observer far away");

        let mut eyes = app
            .world_mut()
            .query_filtered::<&mut ae::BodyKinematics, With<PlayerEntity>>();
        for mut eye in eyes.iter_mut(app.world_mut()) {
            eye.pos = ae::Vec2::new(900.0, 0.0);
        }
        app.update();
        assert!(!is_dormant(&app, actor), "and awake once one is close");
    }

    /// **the couch case, which "near the player" cannot express.** Seat one is
    /// far away and seat two is next to the actor; the actor is awake because
    /// SOMEBODY is there, not because the protagonist is.
    #[test]
    fn a_second_observer_alone_is_enough_to_keep_an_actor_awake() {
        let (app, actor) = app_with(
            Some(DormancyPolicy::AwakeNearObservers { radius: 400.0 }),
            1_000.0,
            &[0.0, 950.0],
        );
        assert!(!is_dormant(&app, actor));
    }

    #[test]
    fn never_stays_awake_with_every_observer_across_the_level() {
        let (app, actor) = app_with(Some(DormancyPolicy::Never), 10_000.0, &[0.0]);
        assert!(!is_dormant(&app, actor));
    }

    /// `ActorControl` is a COMPONENT the body integrates every tick, and the
    /// brain tick that writes it filters `Without<Dormant>`. So falling asleep
    /// does not stop an actor — it FREEZES ITS LAST INTENT, and
    /// `integrate_sim_bodies` (which has no dormancy filter at all, deliberately,
    /// so a falling enemy keeps falling) goes on acting on it. A slop that dozed
    /// off mid-stride walks that direction forever, off the ledge this policy
    /// exists to keep it away from.
    ///
    /// **the first version of this module asserted only that the POLICY was
    /// attached**, which was true and useless: the actor was marked dormant and
    /// kept walking. The claim to test is what the actor DOES, not what it is
    /// labelled.
    #[test]
    fn falling_asleep_retracts_the_brains_last_intent() {
        use ambition_characters::actor::control::ActorControlFrame;
        use ambition_characters::control::ActorControl;
        use ambition_platformer2d_core::reference_frame::LocalAxes;

        let mut app = App::new();
        // the derive runs AHEAD of its reader — see the note above `body_at`.
        app.init_resource::<crate::abilities::traversal::possession::PossessionState>();
        app.add_systems(
            Update,
            (crate::control::project_driving_participant, assess_dormancy).chain(),
        );
        // Driven, not merely marked — see `app_with`.
        app.world_mut().spawn((
            PlayerEntity,
            ambition_characters::control::DrivingParticipant(
                ambition_characters::control::PlayerSlot::PRIMARY,
            ),
            body_at(0.0),
        ));

        let mut striding = ActorControlFrame::neutral();
        striding.locomotion = LocalAxes::new(-1.0, 0.0);
        let actor = app
            .world_mut()
            .spawn((
                body_at(1_000.0),
                DormancyPolicy::AwakeNearObservers { radius: 400.0 },
                ActorControl(striding),
            ))
            .id();

        app.update();

        assert!(is_dormant(&app, actor), "far from every observer");
        assert_eq!(
            app.world()
                .get::<ActorControl>(actor)
                .expect("the actor keeps its control component")
                .0
                .locomotion
                .vec(),
            ambition_platformer2d_core::Vec2::ZERO,
            "a sleeping brain must RETRACT its last word — otherwise the body \
             keeps integrating it and the actor walks off the level asleep, \
             which is the exact symptom the policy was added to stop"
        );
    }

    /// And an actor that stays AWAKE keeps its intent — the retraction is tied to
    /// the transition, not applied to everything the pass touches.
    #[test]
    fn a_waking_actor_keeps_the_intent_its_brain_just_wrote() {
        use ambition_characters::actor::control::ActorControlFrame;
        use ambition_characters::control::ActorControl;
        use ambition_platformer2d_core::reference_frame::LocalAxes;

        let mut app = App::new();
        // the derive runs AHEAD of its reader — see the note above `body_at`.
        app.init_resource::<crate::abilities::traversal::possession::PossessionState>();
        app.add_systems(
            Update,
            (crate::control::project_driving_participant, assess_dormancy).chain(),
        );
        app.world_mut().spawn((PlayerEntity, body_at(0.0)));

        let mut striding = ActorControlFrame::neutral();
        striding.locomotion = LocalAxes::new(1.0, 0.0);
        let actor = app
            .world_mut()
            .spawn((
                body_at(100.0),
                DormancyPolicy::AwakeNearObservers { radius: 400.0 },
                ActorControl(striding),
            ))
            .id();

        app.update();

        assert!(!is_dormant(&app, actor), "well inside the radius");
        assert_eq!(
            app.world()
                .get::<ActorControl>(actor)
                .unwrap()
                .0
                .locomotion
                .vec()
                .x,
            1.0,
            "an awake actor's intent is its brain's business, not this pass's"
        );
    }

    /// A world with nobody in it is between activations, not frozen.
    #[test]
    fn no_observers_at_all_leaves_everything_awake() {
        let (app, actor) = app_with(
            Some(DormancyPolicy::AwakeNearObservers { radius: 1.0 }),
            10_000.0,
            &[],
        );
        assert!(!is_dormant(&app, actor));
    }
}
