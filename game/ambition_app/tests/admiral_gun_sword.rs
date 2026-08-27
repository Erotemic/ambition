//! D254/R6: the Pirate Admiral's side-B fires a GUN-SWORD, and the game has to
//! agree that it does.
//!
//! ⛔ IN THE SHIPPED COMPOSITION, for the reason `smash_ride` gives: the demo
//! shell's catalog cannot seat `npc_pirate_admiral`.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

/// ⭐⭐ THE ADMIRAL'S DRAWN SIDEARM DISCHARGES LIKE A GUN-SWORD.
///
/// ⛔⛔ IT DID NOT, AND THE AUTHORING SAID IT DID. Four choices — the spinning
/// `lasersword` projectile, the muzzle at the hand, `weapon.lasersword.fire` and
/// the heavy recoil — were decided at the fire site by
/// `held_item_id == Some("gun_sword")`. The side-B draws `admiral_gun_sword`,
/// which is a different string, so it got the generic shot out of the midriff
/// with a 60px kick, while both the move's comment and the weapon's own row said
/// *"same art, same discharge, same hand"*.
///
/// ⭐ NOW `Discharge` IS AUTHORED ON THE WEAPON and the fire site knows no
/// weapon's name. Both gun-swords share the profile; each keeps its own damage,
/// speed and assist, which is exactly what this test asserts alongside it.
#[test]
fn the_admirals_side_b_fires_the_gun_swords_discharge() {
    use ambition_platformer2d::actor::MatchSeat;
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            "npc_pirate_admiral",
            "npc_pirate_admiral",
        ]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));
    for _ in 0..900 {
        app.update();
        let (seated, held) = {
            let world = app.world_mut();
            let mut all = world.query::<&MatchSeat>();
            let seated = all.iter(world).count();
            let mut q = world.query_filtered::<
                &MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
            >();
            (seated, q.iter(world).count())
        };
        if seated > 0 && held == 0 {
            break;
        }
    }
    let admiral = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the match seats a first fighter")
    };

    let side_b = ambition_platformer2d::engine_core::ControlFrame {
        axis_x: 1.0,
        special_pressed: true,
        special_held: true,
        ..Default::default()
    };
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), side_b);
    app.update();

    // Walk until the shot appears, keeping the velocity from the tick BEFORE it
    // so the recoil is a delta rather than a reading of whatever the move's
    // forward impulse left behind.
    let vel = |app: &App| {
        app.world()
            .get::<ambition_platformer2d::engine_core::BodyKinematics>(admiral)
            .map(|kin| kin.vel)
            .expect("the admiral has kinematics")
    };
    let hand = |app: &App| {
        let kin = app
            .world()
            .get::<ambition_platformer2d::engine_core::BodyKinematics>(admiral)
            .expect("the admiral has kinematics");
        ambition_platformer2d::mount::rider_hand_world_pos(kin.pos, kin.facing, kin.size.y)
    };
    let mut shot = None;
    for _ in 0..90 {
        let before = vel(&app);
        let hand_before = hand(&app);
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame {
                special_pressed: false,
                ..side_b
            },
        );
        app.update();
        let found = {
            let world = app.world_mut();
            let mut q = world.query::<(
                &ambition_platformer2d::projectiles::ProjectileOwner,
                &ambition_platformer2d::projectiles::ProjectileVisualId,
                &ambition_platformer2d::platformer::projectile::ProjectileGameplay,
                &ambition_platformer2d::engine_core::BodyKinematics,
            )>();
            q.iter(world)
                .find(|(owner, _, _, _)| owner.0 == admiral)
                .map(|(_, visual, gameplay, kin)| (visual.0.clone(), gameplay.damage, kin.pos))
        };
        if let Some(found) = found {
            shot = Some((found, before, vel(&app), hand_before));
            break;
        }
    }
    let ((visual, damage, origin), before, after, hand_before) =
        shot.expect("the admiral's side-B never produced a projectile he owns");

    assert_eq!(
        visual, "lasersword",
        "the side-B fired a `{visual}` — the drawn gun-sword's shot is the \
         spinning blade, and it was chosen by a compare against the OTHER \
         gun-sword's id"
    );
    assert_eq!(
        damage, 8,
        "the shot did {damage} — the admiral's sidearm is its own weapon and its \
         damage must not have been folded into a shared discharge"
    );
    assert!(
        origin.distance(hand_before) < 64.0,
        "the shot was born at {origin:?} and his hand was at {hand_before:?} — a \
         drawn weapon fires from the barrel a player can see"
    );
    // ⛔ A DELTA, and a big one. The generic kick is 60px/s and the gun-sword's
    // is 380, so a threshold between them is what tells "the profile applied"
    // from "something pushed him".
    let kick = after.x - before.x;
    assert!(
        kick < -200.0,
        "firing changed his x velocity by {kick} — the gun-sword's recoil is 380 \
         against a generic 60, so anything softer than this means the shot came \
         out of a body that did not know what it was holding"
    );
}
