//! Sing: an area that takes the floor away from whoever stood too close.
//!
//! ⭐⭐ NO STATUS SYSTEM WAS WRITTEN FOR THIS. `attack_support`'s
//! `hard_lock_timer` is already a `max()` over named causes of "this body cannot
//! act", and `BodyCombat::sleep_timer` is a fifth one. This module finds the
//! bodies in range and sets that timer; everything downstream — control
//! stripping, the shared decay, the wake a real hit buys — was already there.
//!
//! ⛔ THE SINGER IS NEVER CAUGHT BY THEIR OWN SONG. Not politeness: the move
//! puts everyone else to sleep and then the singer acts, which IS the move. A
//! version that slept its own caster would be a very slow suicide.

use bevy::math::bounding::IntersectsVolume as _;
use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_sleep::{SleepParams, SLEEP};
use ambition_platformer2d::engine_core as ae;

/// Put every eligible body inside the pulse to sleep.
pub fn apply_authored_sleep(
    mut actions: MessageReader<ActorActionMessage>,
    singers: Query<&ae::BodyKinematics>,
    mut victims: Query<(
        Entity,
        &ae::CenteredAabb,
        &mut ambition_platformer2d::characters::actor::BodyCombat,
    )>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != SLEEP {
            continue;
        }
        let params: SleepParams = match params.hydrate() {
            Ok(p) => p,
            Err(err) => {
                warn!("smash sleep params did not hydrate: {err}");
                continue;
            }
        };
        let Ok(kin) = singers.get(message.actor) else {
            continue;
        };
        // ⭐ CENTRED ON THE SINGER AND SYMMETRIC, so the move does not care
        // which way they are facing. A directional sing would be a strike with a
        // status attached, which is a different move and a worse one.
        let reach = ae::CenteredAabb::from_center_size(
            kin.pos,
            ae::Vec2::new(params.half_extents.0 * 2.0, params.half_extents.1 * 2.0),
        )
        .aabb();
        for (body, aabb, mut combat) in &mut victims {
            if body == message.actor {
                continue;
            }
            if !reach.intersects(&aabb.aabb()) {
                continue;
            }
            // ⛔ A FLOOR, NOT AN ADDITION. Two overlapping pulses must not stack
            // into a sleep nobody can wake from; the longer one simply wins.
            combat.sleep_timer = combat.sleep_timer.max(params.duration_s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::characters::actor::BodyCombat;

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_systems(Update, apply_authored_sleep);
        app
    }

    fn body(app: &mut App, at: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                ae::BodyKinematics {
                    pos: at,
                    size: ae::Vec2::new(28.0, 46.0),
                    ..Default::default()
                },
                ae::CenteredAabb::from_center_size(at, ae::Vec2::new(28.0, 46.0)),
                BodyCombat::default(),
            ))
            .id()
    }

    fn sing(app: &mut App, singer: Entity, duration_s: f32) {
        app.world_mut().write_message(ActorActionMessage {
            actor: singer,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::Special(SLEEP.to_string()),
                params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                    &SleepParams {
                        duration_s,
                        half_extents: (70.0, 40.0),
                    },
                )
                .expect("sleep params serialize"),
            },
        });
    }

    fn slept(app: &App, body: Entity) -> f32 {
        app.world().get::<BodyCombat>(body).unwrap().sleep_timer
    }

    /// The song catches whoever is near and never the singer.
    ///
    /// ⛔⛔ THE SELF-EXCLUSION IS THE MOVE, NOT POLITENESS. Sing puts everyone
    /// else to sleep and then the singer acts — that IS the payoff. A version
    /// that slept its own caster would be an elaborate way to lose, and nothing
    /// about the area or the duration would reveal it.
    #[test]
    fn the_song_catches_the_room_and_never_the_singer() {
        let mut app = app();
        let singer = body(&mut app, ae::Vec2::new(0.0, 0.0));
        let near = body(&mut app, ae::Vec2::new(40.0, 0.0));
        let far = body(&mut app, ae::Vec2::new(400.0, 0.0));
        sing(&mut app, singer, 1.4);
        app.update();

        assert_eq!(slept(&app, singer), 0.0, "the singer slept through their own song");
        assert!(
            slept(&app, near) > 0.0,
            "a body inside the pulse was not caught"
        );
        assert_eq!(
            slept(&app, far),
            0.0,
            "a body well outside the pulse was caught, so the area means nothing"
        );
    }

    /// Two songs do not STACK; the longer one wins.
    ///
    /// ⛔ ADDITION WOULD BE UNBOUNDED. Two singers, or one singer twice, would
    /// compound into a sleep nobody wakes from — and the wake a real hit buys
    /// would stop being the counterplay it is meant to be.
    #[test]
    fn overlapping_songs_take_the_longer_one_rather_than_the_sum() {
        let mut app = app();
        let singer = body(&mut app, ae::Vec2::new(0.0, 0.0));
        let victim = body(&mut app, ae::Vec2::new(40.0, 0.0));
        sing(&mut app, singer, 1.4);
        app.update();
        sing(&mut app, singer, 0.6);
        app.update();
        assert_eq!(
            slept(&app, victim),
            1.4,
            "overlapping songs did not take the longer one — a shorter song \
             either extended the sleep or cut it short"
        );
    }
}
