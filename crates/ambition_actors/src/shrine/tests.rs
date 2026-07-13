//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::actor::BodyBaseSize;
use crate::actor::{PlayerEntity, PrimaryPlayer};

#[test]
fn interacting_at_the_shrine_heals_to_full() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.init_resource::<ambition_persistence::save::SandboxSave>();
    app.init_resource::<ShrineActivationPulse>();
    app.add_systems(Update, heal_save_shrine_system);

    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActorControl::default(),
            BodyKinematics {
                pos: Vec2::new(100.0, 100.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: false,
            }),
            BodyMana::default(),
        ))
        .id();
    // Drain mana so we can see it refill.
    app.world_mut()
        .get_mut::<BodyMana>(player)
        .unwrap()
        .meter
        .try_spend(40.0);
    app.world_mut().spawn(HealShrine {
        pos: Vec2::new(100.0, 100.0),
        half_extent: Vec2::new(22.0, 40.0),
    });

    // Interact while overlapping → heal to full.
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .interact_pressed = true;
    app.update();

    let health = *app.world().get::<BodyHealth>(player).unwrap();
    assert_eq!(health.current(), health.max(), "health should be full");
    let mana = app.world().get::<BodyMana>(player).unwrap().meter;
    assert!(
        mana.is_full(),
        "mana should be refilled, got {}",
        mana.current
    );
}

#[test]
fn no_heal_without_interact_or_when_not_touching() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.init_resource::<ambition_persistence::save::SandboxSave>();
    app.init_resource::<ShrineActivationPulse>();
    app.add_systems(Update, heal_save_shrine_system);
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActorControl::default(),
            BodyKinematics {
                pos: Vec2::new(100.0, 100.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: false,
            }),
            BodyMana::default(),
        ))
        .id();
    // A shrine far away.
    app.world_mut().spawn(HealShrine {
        pos: Vec2::new(900.0, 900.0),
        half_extent: Vec2::new(22.0, 40.0),
    });

    // Interact pressed but not touching → no heal.
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .interact_pressed = true;
    app.update();
    assert_eq!(
        app.world().get::<BodyHealth>(player).unwrap().current(),
        1,
        "no heal when not at the shrine"
    );
}
