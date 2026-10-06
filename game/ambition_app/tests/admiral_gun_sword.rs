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
        direction,
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
    // A match seat states no art scale (no `SpritePosedBody`, no
    // `ActorRenderSize`), so the landmark query has no answer for it from the
    // art, and with no rig the shot leaves the fixed hand: the named fallback
    // (`ambition_held_items::holding_hand_world`). The shot is on the line
    // through that hand. A seat that learns to state its scale moves this
    // shot to the hip, where the rigged admiral below fires from.
    assert!(
        direction.perp_dot(origin - hand_before).abs() < 0.5,
        "the shot at {origin:?} flying {direction:?} is off the line through the fixed hand \
         {hand_before:?}: an unrigged seat that states no art scale fires from the fixed hand"
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
    /// The admiral's feet line (world y, +y down), on the tick before the shot.
    feet_before: f32,
    /// Half the shot's height.
    shot_half_height: f32,
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
        let feet_before = {
            let kin = app
                .world()
                .get::<ambition_platformer2d::engine_core::BodyKinematics>(admiral)
                .expect("the admiral has kinematics");
            kin.pos.y + kin.size.y * 0.5
        };
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
                    (visual.0.clone(), gameplay.damage, kin.pos, kin.vel.normalize_or_zero(), kin.size.y * 0.5)
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
            let body = app
                .world()
                .get::<ambition_platformer2d::engine_core::BodyKinematics>(admiral)
                .expect("the admiral has kinematics")
                .clone();
            app.update();
            let after = vel(&app);
            // A measurement, not an assertion: what the place a shot is born
            // costs or gains in flight. See Q158.
            let other_health = |app: &mut App| {
                let world = app.world_mut();
                let mut q = world.query::<(
                    &MatchSeat,
                    &ambition_platformer2d::characters::actor::BodyHealth,
                    &ambition_platformer2d::engine_core::BodyKinematics,
                )>();
                q.iter(world)
                    .find(|(seat, _, _)| seat.0 == 1)
                    .map(|(_, health, kin)| (health.damage_percent(), kin.pos))
            };
            let health_before = other_health(&mut app);
            let mut path = vec![found.2];
            for _ in 0..180 {
                ambition_platformer2d::sim::drive_control_frame(
                    app.world_mut(),
                    ambition_platformer2d::engine_core::ControlFrame::default(),
                );
                app.update();
                let world = app.world_mut();
                let mut q = world.query::<(
                    &ambition_platformer2d::projectiles::ProjectileOwner,
                    &ambition_platformer2d::engine_core::BodyKinematics,
                )>();
                match q.iter(world).find(|(owner, _)| owner.0 == admiral) {
                    Some((_, kin)) => path.push(kin.pos),
                    None => break,
                }
            }
            eprintln!(
                "side-B (rigs admitted: {admit_rigs}): shot first seen {:?} from the body centre (body size {:?}, \
                 facing {}), {:.1} above the feet line, flying {:?}; the fixed hand {:?} and the rig hand {:?} from \
                 the centre; alive {} ticks, travelled {:.0}; the other fighter (percent, place, from this body {:?}) {:?} -> {:?}",
                found.2 - body.pos,
                body.size,
                body.facing,
                feet_before - found.2.y,
                found.3,
                hand_before - body.pos,
                rig_hand_before.map(|hand| hand - body.pos),
                path.len(),
                (*path.last().expect("one place") - found.2).length(),
                health_before.map(|(_, place)| place - body.pos),
                health_before,
                other_health(&mut app),
            );
            shot = Some((found, before, after, hand_before, rig_hand_before, feet_before));
            break;
        }
    }
    let ((visual, damage, origin, direction, shot_half_height), before, after, hand_before, rig_hand_before, feet_before) =
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
        feet_before,
        shot_half_height,
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
        feet_before,
        shot_half_height,
        rigged,
        ..
    } = fire_the_side_b(true);
    assert!(
        rigged,
        "the rig trial is on and the admiral wears no rig: `pirate_admiral_body_rig.ron` is a \
         published asset (gitignored), so publish it with `scripts/regen/sprites.sh`"
    );
    let rig_hand = rig_hand_before.expect("a rigged admiral resolved no weapon hand");
    // The shot flies along its fire line from the muzzle, so it is on the line
    // through the hand it was fired from, between `ahead` (18) and `ahead`
    // plus one tick of flight along it. A hand nearer the feet than the shot is
    // tall (the admiral holds the gun-sword at the hip) lifts the shot clear of
    // the feet line: up from the hand's line by at most its half height and
    // the 1 px clearance, never down.
    let along = direction.dot(origin - rig_hand);
    let lifted = rig_hand.y - origin.y;
    assert!(
        (-0.5..=shot_half_height + 1.0).contains(&lifted) && (18.0..48.0).contains(&along),
        "the shot at {origin:?} flying {direction:?} is {lifted} above and \
         {along} along from the rig hand {rig_hand:?}: it was not fired from it"
    );
    // ⛔ A SHOT BORN TOUCHING THE GROUND DIES ON ITS FIRST TICK, and this test
    // saw no shot at all when the redrawn admiral's hand sat 2 px lower
    // (2026-10-04).
    assert!(
        origin.y + shot_half_height < feet_before,
        "the shot at {origin:?} reaches the admiral's feet line {feet_before}: \
         it was born touching the ground"
    );
    // The fixed rider hand is off that line, so this test tells the two apart.
    assert!(
        direction.perp_dot(origin - hand_before).abs() > 5.0,
        "the rider hand {hand_before:?} is on the shot's line too, so the rig \
         hand was not shown to be the muzzle"
    );
}
