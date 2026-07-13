//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::abilities::test_support::spawn_primary_player_holding;
use crate::enemy_projectile::test_support::enemy_projectile_bodies;
use crate::enemy_projectile::EnemyProjectileState;
use crate::projectile::ProjectileSeqCounter;

fn test_app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_vfx::EffectRequest>();
    app.init_resource::<EnemyProjectileState>();
    app.init_resource::<ProjectileSeqCounter>();
    // fire emits Effect::Projectiles; apply_projectile_effects spawns the entity.
    app.add_systems(
        Update,
        (
            fire_meteor_system,
            crate::enemy_projectile::apply_projectile_effects,
        )
            .chain(),
    );
    app
}

#[test]
fn attack_rains_player_faction_meteors() {
    let mut app = test_app();
    let player = spawn_primary_player_holding(&mut app, METEOR_ID);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    let bodies = enemy_projectile_bodies(&mut app);
    assert_eq!(
        bodies.len(),
        METEOR_COUNT,
        "one volley = METEOR_COUNT meteors"
    );
}

#[test]
fn no_meteor_without_attack_or_item() {
    let mut app = test_app();
    spawn_primary_player_holding(&mut app, METEOR_ID);
    app.update(); // no attack pressed
    assert!(enemy_projectile_bodies(&mut app).is_empty());
}

#[test]
fn meteor_costs_mana_and_is_blocked_when_empty() {
    let mut app = test_app();
    let player = spawn_primary_player_holding(&mut app, METEOR_ID);
    app.world_mut()
        .get_mut::<BodyMana>(player)
        .unwrap()
        .meter
        .current = 5.0;
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    assert!(
        enemy_projectile_bodies(&mut app).is_empty(),
        "no meteors when mana < cost"
    );
    app.world_mut()
        .get_mut::<BodyMana>(player)
        .unwrap()
        .meter
        .current = 100.0;
    app.update();
    assert_eq!(
        enemy_projectile_bodies(&mut app).len(),
        METEOR_COUNT,
        "fires once there's mana"
    );
}

#[test]
fn meteors_spawn_above_the_player_and_spread_horizontally() {
    let player_pos = ae::Vec2::new(100.0, 100.0);
    let origins = meteor_strike_origins(
        player_pos,
        ae::Vec2::new(1.0, 0.0),
        1.0,
        ae::Vec2::new(0.0, 1.0),
    );
    // All spawn above the player (smaller y, engine y-down).
    assert!(
        origins.iter().all(|o| o.y < player_pos.y),
        "meteors spawn above the player to fall down: {origins:?}"
    );
    // The zone centers ahead of the player (+x for a rightward aim).
    let mean_x = origins.iter().map(|o| o.x).sum::<f32>() / METEOR_COUNT as f32;
    assert!(mean_x > player_pos.x, "strike zone is ahead of the player");
    // They spread horizontally (not a single column).
    let min_x = origins.iter().map(|o| o.x).fold(f32::INFINITY, f32::min);
    let max_x = origins
        .iter()
        .map(|o| o.x)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        (max_x - min_x) > 100.0,
        "meteors are spread across a band: {min_x}..{max_x}"
    );
}

#[test]
fn meteor_origins_are_frame_equivalent() {
    let player_pos = ae::Vec2::new(100.0, 100.0);
    let local_aim = ae::Vec2::new(1.0, 0.0);
    let down = meteor_strike_origins(player_pos, local_aim, 1.0, ae::Vec2::new(0.0, 1.0));
    for gravity_dir in [
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(-1.0, 0.0),
    ] {
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let rotated = meteor_strike_origins(player_pos, local_aim, 1.0, gravity_dir);
        for (reference, actual) in down.iter().zip(rotated.iter()) {
            let expected_local = ae::AccelerationFrame::new(ae::Vec2::new(0.0, 1.0))
                .to_local(*reference - player_pos);
            let actual_local = frame.to_local(*actual - player_pos);
            assert!((expected_local - actual_local).length() < 1e-3);
        }
    }
}

#[test]
fn meteor_aims_with_the_left_stick_facing_on_a_null_aim() {
    // Aiming left (negative facing, no directional hold) puts the zone to the left.
    let left = meteor_strike_origins(
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::ZERO,
        -1.0,
        ae::Vec2::new(0.0, 1.0),
    );
    let mean_x = left.iter().map(|o| o.x).sum::<f32>() / METEOR_COUNT as f32;
    assert!(
        mean_x < 100.0,
        "a left-facing null-aim cast strikes to the left"
    );
}
