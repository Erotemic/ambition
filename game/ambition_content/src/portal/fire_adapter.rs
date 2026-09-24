//! Ambition fire-intent resolver: gesture → generic portal fire intent.
//!
//! The input adapter recognizes the gesture and emits a [`FirePortalGun`] naming
//! its body. This resolver reads `FirePortalGun`, resolves the origin (that
//! body's position), the direction (the gesture's aim) and the channel (the
//! held gun's current color), and emits the generic intent. Anything (a replay,
//! an AI) can also place a portal by emitting `PortalFireIntent` directly.

use bevy::prelude::*;

use ambition_platformer2d_core::BodyKinematics;
use ambition_portal2d::{FirePortalGun, PortalFireIntent, PortalGun};

/// Resolve a [`FirePortalGun`] gesture into a generic [`PortalFireIntent`] fired
/// from the body holding the gun.
///
/// The gesture names its body, so this resolver re-derives nothing. With a
/// seatless gesture, a resolver that looped driven bodies would have to guess
/// whose press it was, and would fire one shot per body for one press
/// (D-PORTAL-GESTURE-SEAT).
///
/// The other two portal readers are correctly singular:
/// `sync_portal_viewer`'s eye and `tag_portal_affordance_body`'s drawn gun are
/// presentation, and a view has one viewpoint. Origin = the body's position,
/// dir = the gesture's aim, channel = the held gun's `next_color`. If the body
/// is not holding a `PortalGun`, no intent is emitted (no fallback to the home
/// avatar). Gun-active gating lives here, so the generic intent is emitted only
/// for an armed fire. The core fire system drops a zero aim.
pub fn resolve_portal_fire_intent(
    mut fires: MessageReader<FirePortalGun>,
    mut holders: Query<(
        &BodyKinematics,
        &PortalGun,
        &mut ambition_characters::control::ActorControl,
        // The shot's identity is minted here, the only place that knows who fired.
        // A portal shot is a rollback anchor (`require_rollback::<PortalShot>`), so
        // it must not rewind by entity index. Every other mid-match spawner mints
        // from its own counter; this is the gun's road.
        //
        // `Option`, so a body with no identity still reaches the refusal below. A
        // gun in a hand with no `SimId` is a fixture, not a session, and the
        // timeline's identity census refuses the anonymous shot instead of this
        // query silently dropping the press.
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
        Option<&mut ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
    )>,
    mut intents: MessageWriter<PortalFireIntent>,
) {
    // Every gesture, each from its own body, so two seats each holding a gun
    // get two shots.
    for fire in fires.read() {
        let Ok((kin, gun, mut actor_control, firer, counter)) = holders.get_mut(fire.body) else {
            continue;
        };
        if !gun.active {
            continue;
        }
        // Refuse instead of firing a shot that cannot be named (ADR 0030).
        //
        // The refusal must not swallow the press. `melee_pressed` is cleared at the
        // bottom of this loop so the wearer's jab does not answer the same press; a
        // refusal that skipped the arm would leave it set and the body would jab.
        // So the press is consumed here, before the `continue`.
        let (Some(firer), Some(mut counter)) = (firer, counter) else {
            warn!(
                "a portal shot was refused: the firer carries no SimId or no \
                 SimIdCounter, so the shot could not be named"
            );
            actor_control.0.melee_pressed = false;
            continue;
        };
        // Minted from the firer's own counter, like every production spawner, so a
        // resimulated tick re-mints the same id and two seats firing on one tick
        // cannot collide.
        let id = Some(
            ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(firer, counter.next()),
        );
        intents.write(PortalFireIntent {
            origin: kin.pos,
            dir: fire.aim,
            channel: gun.next_color.channel(),
            id,
        });
        // The gun answered the press, so the wearer's jab must not answer it too.
        actor_control.0.melee_pressed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::BodyBaseSize;
    use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

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

    /// The portal fire originates from the body holding the gun, not the vacated
    /// home avatar. Give the gun to a non-home controlled body and assert the fire
    /// origin is that body's position.
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
                // An identity and its mint stream, as a production body has: the shot
                // mints under the firer. Without them the fixture would hit the refusal
                // (ADR 0030).
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement("test_holder"),
                ambition_platformer2d_shared_tangle::sim_id::SimIdCounter::default(),
            ))
            .id();
        app.world_mut().write_message(FirePortalGun {
            body: holder,
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

    /// The fire spends the Attack press where it is accepted.
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
            app.world_mut().write_message(FirePortalGun {
                body: holder,
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
