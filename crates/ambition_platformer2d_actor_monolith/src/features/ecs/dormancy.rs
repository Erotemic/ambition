//! **Actors that stop THINKING when nobody is there to see them.**
//!
//! Jon, 2026-08-03: *"We probably need an engine concept that allows actors to
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
//! ⛔ **"near the player" is the version that breaks on the couch**, and it
//! would break for netplay next. The wake test is *near any OBSERVER* — any
//! body a view is composed around — so one player, four on a sofa, and a remote
//! peer are the same rule, and this module never learns what a protagonist is.
//! (`magnetize_pickups` had the other version and pulled every coin toward seat
//! one; this is the same lesson applied before the mistake instead of after.)
//!
//! ## 3. It sleeps the BRAIN, not the body
//!
//! The reported defect is an actor ACTING off-screen — walking off a ledge
//! before the player arrives. A body with no control input already stands
//! still, so gating the decision is sufficient. Freezing the BODY would be
//! visibly wrong the moment it mattered: a falling enemy would hang in the air
//! at the edge of the wake radius, and a thrown one would stop mid-arc. Physics
//! is cheap; deciding is not.
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
/// Runs before the brain tick, which filters `Without<Dormant>`. Observers are
/// the same population the pickup magnet attracts toward and collection claims
/// with — bodies marked [`PlayerEntity`](crate::actor::PlayerEntity) — so a
/// second seat's presence wakes the world around it with no extra wiring.
pub fn assess_dormancy(
    mut commands: Commands,
    observers: Query<
        &ae::BodyKinematics,
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
    actors: Query<(
        Entity,
        &ae::BodyKinematics,
        &DormancyPolicy,
        Has<Dormant>,
    )>,
) {
    // Collected once rather than re-iterated per actor: the observer set is
    // tiny (one to four) and the actor set is not.
    let eyes: Vec<ae::Vec2> = observers.iter().map(|body| body.pos).collect();

    for (entity, body, policy, is_dormant) in &actors {
        let awake = match policy {
            DormancyPolicy::Never => true,
            DormancyPolicy::AwakeNearObservers { radius } => {
                // ⚠ **no observers ⇒ AWAKE.** A world with nobody in it is a
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
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::markers::PlayerEntity;

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
        app.add_systems(Update, assess_dormancy);
        for x in observers {
            app.world_mut().spawn((PlayerEntity, body_at(*x)));
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

    /// The default is the one that matters: an actor that declares nothing is
    /// never touched, so adding this module changes no existing content.
    #[test]
    fn an_actor_with_no_policy_is_never_dormant() {
        let (app, actor) = app_with(None, 10_000.0, &[0.0]);
        assert!(!is_dormant(&app, actor), "no policy means the engine assumes nothing");
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

        let mut eyes = app.world_mut().query_filtered::<&mut ae::BodyKinematics, With<PlayerEntity>>();
        for mut eye in eyes.iter_mut(app.world_mut()) {
            eye.pos = ae::Vec2::new(900.0, 0.0);
        }
        app.update();
        assert!(!is_dormant(&app, actor), "and awake once one is close");
    }

    /// ⭐ **the couch case, which "near the player" cannot express.** Seat one is
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
