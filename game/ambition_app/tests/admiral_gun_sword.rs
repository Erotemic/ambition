//! D255/R6: the Pirate Admiral's side-B fires a GUN-SWORD, and the game has to
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
    let SideB {
        visual,
        damage,
        origin,
        before,
        after,
        hand_before,
        ..
    } = fire_the_side_b(false);

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
    //
    // ⚠ SAMPLED AFTER THE LAUNCH GATEWAY RUNS — see the `app.update()` above.
    // A -10.8 delta here means the sample is early, not that the kick is soft.
    let kick = after.x - before.x;
    assert!(
        kick < -200.0,
        "firing changed his x velocity by {kick} — the gun-sword's recoil is 380 \
         against a generic 60, so anything softer than this means the shot came \
         out of a body that did not know what it was holding"
    );
}

/// What one side-B discharge left behind.
struct SideB {
    visual: String,
    damage: i32,
    /// Where the shot was when it was first seen, and its unit direction.
    origin: bevy::math::Vec2,
    direction: bevy::math::Vec2,
    /// The admiral's x velocity before the shot, and after its recoil landed.
    before: bevy::math::Vec2,
    after: bevy::math::Vec2,
    /// The fixed rider hand, on the tick before the shot.
    hand_before: bevy::math::Vec2,
    /// The rig's weapon hand in the world, on the tick before the shot, when
    /// the admiral has a rig.
    rig_hand_before: Option<bevy::math::Vec2>,
    /// Whether the admiral wore a rig.
    rigged: bool,
}

/// Seat two admirals and fire the first one's side-B. `admit_rigs` is the
/// articulated-rig trial switch (`BodyRigAdmission`); the shipped game is off.
fn fire_the_side_b(admit_rigs: bool) -> SideB {
    use ambition_platformer2d::actor::MatchSeat;
    use bevy::prelude::*;

    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
        |app| {
            app.insert_resource(ambition_platformer2d::characters::actor::BodyRigAdmission {
                admit: admit_rigs,
            });
        },
    );
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
                With<ambition_platformer2d::characters::control::ControlHolds>,
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
    // The rig's weapon hand in the world: the same placement the hand muzzle
    // makes, from the pose the simulation resolved.
    let rig_hand = |app: &App| {
        use ambition_platformer2d::combat::body_rig::{BodyRig, BodyRigPose};
        let world = app.world();
        let kin = world.get::<ambition_platformer2d::engine_core::BodyKinematics>(admiral)?;
        let rig = world.get::<BodyRig>(admiral)?;
        let hand = world
            .get::<BodyRigPose>(admiral)?
            .attachment(&rig.0, ambition_platformer2d::characters::actor::body_rig::HAND_NEAR)?;
        let down = Vec2::Y;
        Some(kin.pos + down * (kin.size.y * 0.5) + BodyRigPose::to_body(hand, kin.facing, down))
    };
    let rigged = app
        .world()
        .get::<ambition_platformer2d::combat::body_rig::BodyRig>(admiral)
        .is_some();
    let mut shot = None;
    for _ in 0..90 {
        let before = vel(&app);
        let hand_before = hand(&app);
        let rig_hand_before = rig_hand(&app);
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
                .map(|(_, visual, gameplay, kin)| {
                    (visual.0.clone(), gameplay.damage, kin.pos, kin.vel.normalize_or_zero())
                })
        };
        if let Some(found) = found {
            // ⛔⛤ **ONE FRAME LATER, AND THAT IS A CONTRACT CHANGE THIS TEST HAD
            // NOT FOLLOWED.** `Q112` (`246cab797`) moved ranged recoil off a
            // direct `kin.vel += kick` and onto
            // `BodyFlightState::stage_launch`, because a bare `vel` write is
            // INERT for a surface-momentum body — the launch gateway is the one
            // authority that survives the movement model. Its own comment says
            // *"the recoil no longer writes here"*.
            //
            // ⇒ So sampling `kin.vel` on the frame the projectile appears reads
            // the body BEFORE the gateway has consumed the staged launch.
            // MEASURED: the shot frame reads 90.8 px/s (friction alone, a
            // -10.8 delta) and the NEXT frame reads -369.2, a -470.8 delta —
            // the gun-sword's 380 kick, landed.
            //
            // ⚠ THE RECOIL WAS NEVER LOST, which is the finding that matters: a
            // red here read as "the profile did not apply" and the truth was
            // "this test measures a retired contract". One `update()` is the
            // whole fix.
            app.update();
            shot = Some((found, before, vel(&app), hand_before, rig_hand_before));
            break;
        }
    }
    let ((visual, damage, origin, direction), before, after, hand_before, rig_hand_before) =
        shot.expect("the admiral's side-B never produced a projectile he owns");
    SideB {
        visual,
        damage,
        origin,
        direction,
        before,
        after,
        hand_before,
        rig_hand_before,
        rigged,
    }
}

/// With the rig trial on, the admiral's drawn gun-sword fires from the hand his
/// rig puts it in this tick, not from the fixed rider hand.
///
/// The shot is seen one tick after it is born, so it has flown one tick past
/// the muzzle, and the muzzle is `ahead` past the hand: the band is those two.
#[test]
fn with_rigs_admitted_the_gun_sword_fires_from_the_rig_hand() {
    let SideB {
        origin,
        direction,
        hand_before,
        rig_hand_before,
        rigged,
        ..
    } = fire_the_side_b(true);
    assert!(rigged, "the rig trial is on and the admiral wears no rig");
    let rig_hand = rig_hand_before.expect("a rigged admiral resolved no weapon hand");
    // The shot flies along its fire line from the muzzle, so it is on the line
    // through the hand it was fired from: nothing to the side, and between
    // `ahead` (18) and `ahead` plus one tick of flight along it.
    let along = direction.dot(origin - rig_hand);
    let aside = direction.perp_dot(origin - rig_hand).abs();
    assert!(
        aside < 0.5 && (18.0..48.0).contains(&along),
        "the shot at {origin:?} flying {direction:?} is {aside} to the side of \
         and {along} along from the rig hand {rig_hand:?}: it was not fired from it"
    );
    // The fixed rider hand is off that line, so this test tells the two apart.
    assert!(
        direction.perp_dot(origin - hand_before).abs() > 5.0,
        "the rider hand {hand_before:?} is on the shot's line too, so the rig \
         hand was not shown to be the muzzle"
    );
}
