//! Optional distance-based brain dormancy for actors.
//!
//! A game states one rule, [`DormancyRule`], for the rooms it governs: how near
//! an observer must be for a hostile to keep thinking. [`assess_dormancy`]
//! applies the rule of the active room each tick to
//! facts the body already has (its faction, and whether an encounter owns it
//! or another body drives it). Thus no body waits for a stance on a later tick,
//! and a body that no pass tagged cannot think for the whole level. Without the
//! rule, no brain sleeps.
//!
//! Dormancy sleeps only the brain; body physics continues. Entering dormancy
//! clears the persistent `ActorControl` frame so stale input cannot keep moving
//! the body. [`Dormant`] is derived from current positions each tick.

use bevy::prelude::*;

use ambition_characters::actor::limb::Limb;
use ambition_combat::components::{ActorFaction, EncounterMob};
use ambition_mount::Mountable;
use ambition_platformer2d_core as ae;

/// One game's dormancy rule, declared for the rooms it governs with
/// [`DeclareRulesExt::declare_rules`](ambition_combat::scoped_rules::DeclareRulesExt::declare_rules).
/// In a room that no game gave a rule, no brain sleeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DormancyRule {
    /// A hostile's brain sleeps while every observer is farther than this, in
    /// world units.
    ///
    /// One number, not a rectangle: the wake test is a distance from a point,
    /// and a screen-shaped region would bake the camera's aspect into the
    /// simulation.
    pub hostile_wake_radius: f32,
}

/// The distance within which an observer keeps this body's brain awake, or
/// `None` when the brain never sleeps.
///
/// Only a hostile ([`ActorFaction::Enemy`]) sleeps:
/// - A boss never sleeps. Its phase machine is the fight, it has its own wake
///   (`BossEncounterPhase::Dormant`, which the encounter drives), and the brain
///   tick does not tick a boss.
/// - A placed NPC never sleeps. The placed cast thinks whoever watches it (the
///   Hall is a load test as much as an exhibition).
/// - A player or a neutral prop has no brain to sleep.
///
/// Two hostiles never sleep either. The encounter, not a distance, decides
/// when an encounter mob appears and when it is done. And another body drives
/// a mount or a limb: dormancy retracts `ActorControl`, which that driver
/// writes. A mount does not sleep while it has no rider, because boarding can
/// occur at any time.
pub fn wake_radius(
    rule: Option<&DormancyRule>,
    faction: ActorFaction,
    is_encounter_mob: bool,
    is_driven_by_another_body: bool,
) -> Option<f32> {
    let rule = rule?;
    (faction == ActorFaction::Enemy && !is_encounter_mob && !is_driven_by_another_body)
        .then_some(rule.hostile_wake_radius)
}

/// This actor's brain is asleep this tick. Derived every tick; never
/// authored, never persisted.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dormant;

/// Recompute [`Dormant`] before the brain tick from distance to every
/// [`DrivingParticipant`](ambition_characters::control::DrivingParticipant).
/// This naturally follows possession and multiple local seats.
pub fn assess_dormancy(
    mut commands: Commands,
    // Each actor's own room's rule (OW1): with two rooms live, THE live
    // room's rule was the rule of no room, so no actor slept.
    rule: crate::session::governing_rules::RulesOf<DormancyRule>,
    observers: Query<(Entity, &ae::BodyKinematics), With<ambition_characters::control::DrivingParticipant>>,
    mut actors: Query<(
        Entity,
        &ae::BodyKinematics,
        &ActorFaction,
        Has<EncounterMob>,
        Has<Mountable>,
        Has<Limb>,
        Has<Dormant>,
        // The brain's last word, which must be RETRACTED when the brain sleeps.
        // `Option` because a body may be a candidate before it carries a brain.
        Option<&mut ambition_characters::control::ActorControl>,
    )>,
) {
    // Collected once rather than re-iterated per actor: the observer set is
    // tiny (one to four) and the actor set is not. Each eye carries its live
    // room: an observer in another live room at the same place does not see
    // this actor (OW1).
    let eyes: Vec<_> = observers
        .iter()
        .map(|(observer, body)| (rule.room_of(observer), body.pos))
        .collect();
    // One rule per live room, resolved the first time an actor of that room
    // asks: the actor set is large and the room set is one or two.
    let mut rules: Vec<(Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>, Option<DormancyRule>)> =
        Vec::new();

    for (entity, body, faction, is_encounter_mob, is_mount, is_limb, is_dormant, control) in
        &mut actors
    {
        let room = rule.room_of(entity);
        let room_rule = match rules.iter().find(|(seen, _)| *seen == room) {
            Some((_, resolved)) => *resolved,
            None => {
                let resolved = rule.in_live_room(room);
                rules.push((room, resolved));
                resolved
            }
        };
        let awake = match wake_radius(room_rule.as_ref(), *faction, is_encounter_mob, is_mount || is_limb) {
            None => true,
            Some(radius) => {
                // no observers  AWAKE. A world with nobody in it is a
                // world between activations, not a world to freeze: sleeping
                // every actor there would make a room's first frame after a
                // transition depend on which system ran first.
                // Per room: an actor whose room holds no observer is awake,
                // for the same reason.
                let mut eyes_here = eyes.iter().filter(|(eye_room, _)| *eye_room == room).peekable();
                eyes_here.peek().is_none()
                    || eyes_here.any(|(_, eye)| eye.distance(body.pos) <= radius.max(0.0))
            }
        };
        match (awake, is_dormant) {
            (true, true) => {
                commands.entity(entity).remove::<Dormant>();
            }
            (false, false) => {
                commands.entity(entity).insert(Dormant);
                // RETRACT the brain's last word. See the module doc: the
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

    // WHY EVERY FIXTURE BELOW SPAWNS A SEAT, NOT JUST A MARKER.
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

    /// A hostile's wake radius in these fixtures.
    const RADIUS: f32 = 400.0;

    /// A standalone game's rule: these fixtures have no rooms.
    fn declare_rule(app: &mut App, radius: f32) {
        use ambition_combat::scoped_rules::{DeclareRulesExt, RulesScope};
        app.declare_rules(
            RulesScope::EveryRoom,
            DormancyRule {
                hostile_wake_radius: radius,
            },
        );
    }

    fn app_with(
        rule: Option<f32>,
        faction: ActorFaction,
        actor_x: f32,
        observers: &[f32],
    ) -> (App, Entity) {
        let mut app = App::new();
        if let Some(radius) = rule {
            declare_rule(&mut app, radius);
        }
        // the derive runs AHEAD of its reader — see the note above `body_at`.
        app.init_resource::<crate::control::possession::PossessionState>();
        app.add_systems(
            Update,
            (crate::control::project_driving_participant, assess_dormancy).chain(),
        );
        for x in observers {
            // an observer is a body being DRIVEN, which is why this spawns a SEAT and not only
            // the `PlayerEntity` marker.
            app.world_mut().spawn((
                PlayerEntity,
                ambition_characters::control::DrivingParticipant(
                    ambition_characters::control::PlayerSlot::PRIMARY,
                ),
                body_at(*x),
            ));
        }
        let actor = app.world_mut().spawn((body_at(actor_x), faction)).id();
        app.update();
        (app, actor)
    }

    fn is_dormant(app: &App, actor: Entity) -> bool {
        app.world().get::<Dormant>(actor).is_some()
    }

    /// A POSSESSED body is the observer; the parked one is not.
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
        declare_rule(&mut app, RADIUS);
        // the derive runs AHEAD of its reader — see the note above `body_at`.
        app.init_resource::<crate::control::possession::PossessionState>();
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
                ActorFaction::Enemy,
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

    /// The default is the one that matters: a game that states no rule is
    /// never touched, so adding this module changes no existing content.
    #[test]
    fn a_game_with_no_rule_never_sleeps_a_brain() {
        let (app, actor) = app_with(None, ActorFaction::Enemy, 10_000.0, &[0.0]);
        assert!(
            !is_dormant(&app, actor),
            "no rule means the engine assumes nothing"
        );
    }

    #[test]
    fn a_far_hostile_sleeps() {
        let (app, actor) = app_with(Some(RADIUS), ActorFaction::Enemy, 1_000.0, &[0.0]);
        assert!(is_dormant(&app, actor));
    }

    #[test]
    fn the_same_actor_wakes_when_an_observer_arrives() {
        let (mut app, actor) = app_with(Some(RADIUS), ActorFaction::Enemy, 1_000.0, &[0.0]);
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

    /// the couch case, which "near the player" cannot express. Seat one is
    /// far away and seat two is next to the actor; the actor is awake because
    /// SOMEBODY is there, not because the protagonist is.
    #[test]
    fn a_second_observer_alone_is_enough_to_keep_an_actor_awake() {
        let (app, actor) =
            app_with(Some(RADIUS), ActorFaction::Enemy, 1_000.0, &[0.0, 950.0]);
        assert!(!is_dormant(&app, actor));
    }

    /// Only a hostile sleeps. A boss, the placed cast, a player and a prop
    /// think whoever watches them.
    #[test]
    fn a_far_body_that_is_not_hostile_stays_awake() {
        for faction in [
            ActorFaction::Boss,
            ActorFaction::Npc,
            ActorFaction::Player,
            ActorFaction::Neutral,
        ] {
            let (app, actor) = app_with(Some(RADIUS), faction, 10_000.0, &[0.0]);
            assert!(!is_dormant(&app, actor), "{faction:?} fell asleep");
        }
    }

    /// A far hostile that an encounter owns, or that another body drives,
    /// stays awake: the encounter decides when a mob is done, and sleep would
    /// retract the control that the driver writes.
    #[test]
    fn a_far_hostile_that_something_else_decides_for_stays_awake() {
        let owned: [(&str, fn(&mut EntityWorldMut)); 3] = [
            ("an encounter mob", |body| {
                body.insert(EncounterMob::new("wave"));
            }),
            ("a mount", |body| {
                body.insert(Mountable::at(ae::Vec2::ZERO));
            }),
            ("a limb", |body| {
                let host = body.id();
                body.insert(Limb {
                    of: host,
                    slot: ambition_characters::actor::limb::LimbSlot::HAND_LEFT,
                    home_offset: ae::Vec2::ZERO,
                });
            }),
        ];
        for (what, own) in owned {
            let (mut app, actor) = app_with(Some(RADIUS), ActorFaction::Enemy, 10_000.0, &[0.0]);
            own(&mut app.world_mut().entity_mut(actor));
            app.update();
            assert!(!is_dormant(&app, actor), "{what} fell asleep");
        }
    }

    /// Entering dormancy retracts the brain's last `ActorControl` intent. Physics
    /// integration continues for dormant bodies, so stale locomotion intent must
    /// not remain active.
    #[test]
    fn falling_asleep_retracts_the_brains_last_intent() {
        use ambition_characters::actor::control::ActorControlFrame;
        use ambition_characters::control::ActorControl;
        use ambition_platformer2d_core::reference_frame::LocalAxes;

        let mut app = App::new();
        declare_rule(&mut app, RADIUS);
        // the derive runs AHEAD of its reader — see the note above `body_at`.
        app.init_resource::<crate::control::possession::PossessionState>();
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
                ActorFaction::Enemy,
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
        declare_rule(&mut app, RADIUS);
        // the derive runs AHEAD of its reader — see the note above `body_at`.
        app.init_resource::<crate::control::possession::PossessionState>();
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
                ActorFaction::Enemy,
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
        let (app, actor) = app_with(Some(1.0), ActorFaction::Enemy, 10_000.0, &[]);
        assert!(!is_dormant(&app, actor));
    }

    /// OW1: each actor sleeps by its own room's rule and is seen only by the
    /// observers in its own room. Ambition's hall (#0) has the rule, Smash's
    /// stage (#1) has none. Bob observes the hall at x = 0 and Alice the stage
    /// at x = 1000. A hall hostile at x = 1000 is far from Bob, so it sleeps,
    /// although Alice stands at the same place in the other room. A stage
    /// hostile far from everyone stays awake, because its room has no rule.
    /// With THE live room's rule, both rooms had none and every actor was awake.
    #[test]
    fn an_actor_sleeps_by_its_own_rooms_rule_and_observers() {
        use ambition_combat::scoped_rules::{DeclareRulesExt, RulesScope};
        use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
        let mut app = App::new();
        app.declare_rules(
            RulesScope::UntaggedRooms,
            DormancyRule {
                hostile_wake_radius: RADIUS,
            },
        );
        let hall = crate::session::governing_rules::tests::two_game_session(&mut app, true);
        let stage = hall.next();
        app.init_resource::<crate::control::possession::PossessionState>();
        app.add_systems(
            Update,
            (crate::control::project_driving_participant, assess_dormancy).chain(),
        );
        for (slot, x, room) in [(1, 0.0, hall), (0, 1000.0, stage)] {
            app.world_mut().spawn((
                PlayerEntity,
                ambition_characters::control::DrivingParticipant(
                    ambition_characters::control::PlayerSlot(slot),
                ),
                body_at(x),
                InRoomInstance(room),
            ));
        }
        let in_hall = app
            .world_mut()
            .spawn((body_at(1000.0), ActorFaction::Enemy, InRoomInstance(hall)))
            .id();
        let on_stage = app
            .world_mut()
            .spawn((body_at(5000.0), ActorFaction::Enemy, InRoomInstance(stage)))
            .id();
        app.update();
        assert_eq!(
            (is_dormant(&app, in_hall), is_dormant(&app, on_stage)),
            (true, false),
            "(the hall hostile far from Bob sleeps, the stage hostile with no rule wakes)"
        );
    }
}
