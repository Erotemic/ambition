//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use ambition_portal::arm_portal_pickups;

fn spawn_player(app: &mut App, pos: Vec2, facing: f32) -> Entity {
    app.world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos,
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            PortalGun::default(),
            ActionSet::default(),
        ))
        .id()
}

#[test]
fn picking_up_the_portal_gun_activates_it() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.add_message::<PickUpPortalGun>();
    app.add_message::<PortalGunEquipped>();
    app.add_systems(Update, pickup_portal_gun_system);
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos: Vec2::new(50.0, 50.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            ActionSet::default(),
            // No PortalGun yet — the single pickup item grants it.
        ))
        .id();
    app.world_mut().spawn(PortalGunPickup {
        pos: Vec2::new(50.0, 50.0),
        half_extent: Vec2::splat(20.0),
        arm_timer: 0.0,
    });
    assert!(app.world().get::<PortalGun>(player).is_none());

    app.world_mut().write_message(PickUpPortalGun);
    app.update();
    assert!(
        app.world()
            .get::<PortalGun>(player)
            .is_some_and(|g| g.active),
        "a pickup intent while overlapping grants the active gun"
    );
    let remaining = {
        let mut q = app.world_mut().query::<&PortalGunPickup>();
        q.iter(app.world()).count()
    };
    assert_eq!(remaining, 0, "the pickup is consumed");
}

#[test]
fn dropped_portal_gun_arms_before_it_can_be_regrabbed() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.add_message::<DropPortalGun>();
    app.add_message::<PickUpPortalGun>();
    app.add_message::<PortalGunEquipped>();
    app.insert_resource(ambition_platformer_primitives::time::SimDt { dt: 1.0 / 60.0 });
    app.add_systems(
        Update,
        (
            drop_portal_gun_system,
            arm_portal_pickups,
            pickup_portal_gun_system,
        )
            .chain(),
    );
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0), 1.0);

    // Drop intent drops the gun.
    app.world_mut().write_message(DropPortalGun);
    app.update();
    assert!(
        app.world().get::<PortalGun>(player).is_none(),
        "a drop intent should drop the portal gun"
    );

    // Move the player directly onto the dropped pickup so only the arm
    // timer (not distance) guards against a re-grab.
    let pickup_pos = {
        let mut q = app.world_mut().query::<&PortalGunPickup>();
        q.iter(app.world())
            .next()
            .expect("a pickup was dropped")
            .pos
    };
    app.world_mut()
        .get_mut::<BodyKinematics>(player)
        .unwrap()
        .pos = pickup_pos;

    // Immediately a pickup intent while overlapping — the freshly-dropped
    // pickup is still arming, so it must NOT be re-grabbed (the bug).
    app.world_mut().write_message(PickUpPortalGun);
    app.update();
    assert!(
        app.world().get::<PortalGun>(player).is_none(),
        "an armed (just-dropped) pickup can't be re-grabbed on the next intent"
    );

    // Let it disarm, then a pickup intent picks it back up.
    for _ in 0..30 {
        app.update();
    }
    app.world_mut().write_message(PickUpPortalGun);
    app.update();
    assert!(
        app.world().get::<PortalGun>(player).is_some(),
        "once disarmed, a pickup intent while overlapping re-grabs the gun"
    );
}
