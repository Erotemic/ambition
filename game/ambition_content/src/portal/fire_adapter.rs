//! Ambition fire-intent resolver: gesture → generic portal fire intent.
//!
//! The input adapter recognizes the *gesture* and emits a [`FirePortalGun`] (implying "the
//! primary player, holding the gun, aiming this way"). This resolver bridges the two: it reads
//! `FirePortalGun`, resolves the origin (the primary player's body position), the direction
//! (the gesture's aim), and the channel (the held gun's current color), and emits the generic
//! intent — behavior identical to the old in-core `portal_fire_system`, but now anything (a
//! replay, an AI) can place a portal by emitting `PortalFireIntent` directly.

use bevy::prelude::*;

use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_portal2d::{FirePortalGun, PortalFireIntent, PortalGun};

/// Resolve a [`FirePortalGun`] gesture into a generic [`PortalFireIntent`] fired
/// from the body HOLDING the gun — the controlled subject. Origin = that body's
/// position, dir = the gesture's aim, channel = the held gun's `next_color`. If the
/// controlled body isn't holding a `PortalGun`, no intent is emitted (no fallback to
/// the home avatar). Gun-active gating lives here so the generic intent is only
/// emitted for a genuine, armed fire. A zero aim is dropped by the core fire system.
pub fn resolve_portal_fire_intent(
    mut fires: MessageReader<FirePortalGun>,
    controlled: Option<Res<ControlledSubject>>,
    mut holders: Query<(
        &BodyKinematics,
        &PortalGun,
        &mut ambition_characters::control::ActorControl,
    )>,
    primary_fallback: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    mut intents: MessageWriter<PortalFireIntent>,
) {
    let Some(fire) = fires.read().last().copied() else {
        return;
    };
    let Some(subject) = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary_fallback.single().ok())
    else {
        return;
    };
    let Ok((kin, gun, mut actor_control)) = holders.get_mut(subject) else {
        return;
    };
    if !gun.active {
        return;
    }
    intents.write(PortalFireIntent {
        origin: kin.pos,
        dir: fire.aim,
        channel: gun.next_color.channel(),
    });
    // The gun answered the press, so the wearer's jab must not answer it too.
    actor_control.0.melee_pressed = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::BodyBaseSize;

    #[derive(Resource, Default)]
    struct CapturedOrigin(Option<Vec2>);

    fn capture_origin(
        mut intents: MessageReader<PortalFireIntent>,
        mut captured: ResMut<CapturedOrigin>,
    ) {
        if let Some(intent) = intents.read().last() {
            captured.0 = Some(intent.origin);
        }
    }

    /// The portal fire originates from the body HOLDING the gun — the controlled
    /// subject — not the vacated home avatar. Give the gun to a non-home controlled
    /// body and assert the fire origin is that body's position.
    #[test]
    fn portal_fire_origin_comes_from_the_holding_controlled_body() {
        let home_pos = Vec2::new(0.0, 0.0);
        let holder_pos = Vec2::new(500.0, 40.0);

        let mut app = App::new();
        app.add_message::<FirePortalGun>();
        app.add_message::<PortalFireIntent>();
        app.init_resource::<CapturedOrigin>();
        app.add_systems(Update, (resolve_portal_fire_intent, capture_origin).chain());

        // Home avatar: primary player, NO gun.
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos: home_pos,
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
        ));
        // The body the player is DRIVING, holding an active portal gun.
        let holder = app
            .world_mut()
            .spawn((
                BodyKinematics {
                    pos: holder_pos,
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                PortalGun {
                    active: true,
                    ..PortalGun::default()
                },
                // Every production body carries an intent frame, and this system
                // spends the Attack press on it when the gun answers.
                ambition_characters::control::ActorControl::default(),
            ))
            .id();
        app.world_mut()
            .insert_resource(ControlledSubject(Some(holder)));

        app.world_mut().write_message(FirePortalGun {
            aim: Vec2::new(1.0, 0.0),
        });
        app.update();

        let origin = app
            .world()
            .resource::<CapturedOrigin>()
            .0
            .expect("a fire intent should be emitted for the holder");
        assert_eq!(
            origin, holder_pos,
            "portal fires from the holding controlled body, not the home avatar",
        );
    }

    /// THE FIRE SPENDS THE ATTACK PRESS — HERE, WHERE IT IS ACCEPTED.
    #[test]
    fn an_accepted_fire_spends_the_press_and_a_refused_one_does_not() {
        use ambition_characters::control::ActorControl;

        let press_survived = |active: bool| -> bool {
            let mut app = App::new();
            app.add_message::<FirePortalGun>();
            app.add_message::<PortalFireIntent>();
            app.add_systems(Update, resolve_portal_fire_intent);
            let mut control = ActorControl::default();
            control.0.melee_pressed = true;
            let holder = app
                .world_mut()
                .spawn((
                    BodyKinematics {
                        pos: Vec2::ZERO,
                        vel: Vec2::ZERO,
                        size: Vec2::new(24.0, 40.0),
                        facing: 1.0,
                    },
                    PortalGun {
                        active,
                        ..PortalGun::default()
                    },
                    control,
                ))
                .id();
            app.world_mut()
                .insert_resource(ControlledSubject(Some(holder)));
            app.world_mut().write_message(FirePortalGun {
                aim: Vec2::new(1.0, 0.0),
            });
            app.update();
            app.world()
                .entity(holder)
                .get::<ActorControl>()
                .unwrap()
                .0
                .melee_pressed
        };

        assert!(
            !press_survived(true),
            "⛔ the press survived a fire the gun accepted, so the wearer's jab \
             answers it too"
        );
        assert!(
            press_survived(false),
            "⛔ an INACTIVE gun refused the fire and the press was spent anyway — \
             that is the whole defect, one system further along"
        );
    }
}
