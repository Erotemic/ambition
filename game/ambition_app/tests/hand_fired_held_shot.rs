//! K2: a gun-sword or fireball fired FROM THE HAND is a projectile on the one
//! projectile road — the same entity, spawner and stepper an Admiral's side-B or
//! a sentry's volley uses — not the parallel `HeldProjectile` simulation the
//! fold deleted.
//!
//! Every other held-shot fact is pinned at the unit level
//! (`items::pickup::tests`), where the system under test stops at the
//! `ActionRequest::Ranged` it writes. This is the only place that proves the
//! request is CONSUMED in the shipped composition: a press on a held weapon
//! produces a projectile owned by the presser, wearing the weapon's look and
//! carrying the weapon's authored burst.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

/// `(item id, expected visual, expected splash half-extent)`.
fn fire_from_the_hand(item: &str) -> (String, f32, f32) {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::characters::control::PlayerSlot;
    use ambition_platformer2d::engine_core::{BodyKinematics, ControlFrame, Vec2};
    use ambition_platformer2d::item::{GroundItem, ItemCustody};
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster(["mary_o", "mary_o"]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));
    for _ in 0..900 {
        app.update();
        let (seated, scripted) = {
            let world = app.world_mut();
            let mut all = world.query::<&MatchSeat>();
            let seated = all.iter(world).count();
            let mut q = world.query_filtered::<
                &MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
            >();
            (seated, q.iter(world).count())
        };
        if seated > 1 && scripted == 0 {
            break;
        }
    }
    let shooter = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, s)| s.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the match seats a first fighter")
    };

    // The weapon under the shooter's feet; one press picks it up.
    let spec = ambition_platformer2d::character::held_item_by_id(item)
        .unwrap_or_else(|| panic!("{item} is a registered held item"));
    let at = app
        .world()
        .get::<BodyKinematics>(shooter)
        .expect("a seated fighter has kinematics")
        .pos;
    let weapon = app
        .world_mut()
        .spawn((
            GroundItem {
                spec,
                pos: at,
                vel: Vec2::ZERO,
                half_extent: Vec2::splat(12.0),
            },
            ItemCustody::InWorld,
        ))
        .id();
    let press = ControlFrame {
        attack_pressed: true,
        ..Default::default()
    };
    ambition_platformer2d::sim::drive_slot_frame(app.world_mut(), PlayerSlot(0), press);
    app.update();
    for _ in 0..6 {
        ambition_platformer2d::sim::drive_slot_frame(
            app.world_mut(),
            PlayerSlot(0),
            ControlFrame::default(),
        );
        app.update();
    }
    assert!(
        matches!(
            app.world().get::<ItemCustody>(weapon),
            Some(ItemCustody::Held { holder }) if *holder == shooter
        ),
        "the shooter picked the {item} up"
    );

    // A second press fires it. Walk until the shot exists, holding the
    // velocity from the tick before so recoil would read as a delta.
    let vel_x = |app: &App| {
        app.world()
            .get::<BodyKinematics>(shooter)
            .map(|kin| kin.vel.x)
            .expect("the shooter has kinematics")
    };
    ambition_platformer2d::sim::drive_slot_frame(app.world_mut(), PlayerSlot(0), press);
    let mut shot = None;
    for tick in 0..30 {
        let before = vel_x(&app);
        app.update();
        if tick == 0 {
            ambition_platformer2d::sim::drive_slot_frame(
                app.world_mut(),
                PlayerSlot(0),
                ControlFrame::default(),
            );
        }
        let found = {
            let world = app.world_mut();
            let mut q = world.query::<(
                &ambition_platformer2d::projectiles::ProjectileOwner,
                &ambition_platformer2d::projectiles::ProjectileVisualId,
                &ambition_platformer2d::platformer::projectile::ProjectileGameplay,
            )>();
            q.iter(world)
                .find(|(owner, _, _)| owner.0 == shooter)
                .map(|(_, visual, gameplay)| (visual.0.clone(), gameplay.splash_half_extent))
        };
        if let Some((visual, splash)) = found {
            shot = Some((visual, splash, vel_x(&app) - before));
            break;
        }
    }
    shot.unwrap_or_else(|| {
        panic!("a press on the held {item} never produced a projectile the shooter owns")
    })
}

#[test]
fn a_hand_fired_gun_sword_bolt_flies_the_one_projectile_road() {
    let (visual, splash, kick) = fire_from_the_hand("gun_sword");
    assert_eq!(
        visual,
        ambition_platformer2d::characters::brain::action_set::LASERSWORD_VISUAL
    );
    assert_eq!(splash, 0.0, "a bolt hits only what it touches");
    // The deleted held-shot path kicked nobody; the fold must not start to
    // (awaiting-maintainer-decision.md records whether it SHOULD).
    assert!(
        kick.abs() < 1.0,
        "a hand-fired bolt carried {kick} px/s of recoil"
    );
}

#[test]
fn a_hand_fired_fireball_carries_its_burst() {
    use ambition_platformer2d::characters::brain::action_set::{
        FIREBALL_SPLASH_HALF, GAUNTLET_FIREBALL_VISUAL,
    };
    let (visual, splash, _) = fire_from_the_hand("fireball");
    assert_eq!(visual, GAUNTLET_FIREBALL_VISUAL);
    assert_eq!(
        splash, FIREBALL_SPLASH_HALF,
        "the fireball bursts where it lands"
    );
}
