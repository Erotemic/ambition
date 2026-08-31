use super::*;
use crate::abilities::test_support::spawn_primary_player_holding;
use crate::enemy_projectile::test_support::live_projectile_bodies;
use ambition_projectiles::ProjectileSeqCounter;

fn test_app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_projectiles::ProjectileSpawnRequest>();
    app.init_resource::<ProjectileSeqCounter>();
    // Fire emits a request; the immediate materializer creates the live projectile.
    app.add_systems(
        Update,
        (
            fire_meteor_system,
            ambition_projectiles::materialize_projectiles_for_this_tick,
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
    let bodies = live_projectile_bodies(&mut app);
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
    assert!(live_projectile_bodies(&mut app).is_empty());
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
        live_projectile_bodies(&mut app).is_empty(),
        "no meteors when mana < cost"
    );
    app.world_mut()
        .get_mut::<BodyMana>(player)
        .unwrap()
        .meter
        .current = 100.0;
    app.update();
    assert_eq!(
        live_projectile_bodies(&mut app).len(),
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

/// ⭐⭐ A SECOND DRIVEN BODY RAINS ITS OWN METEORS.
/// Same singular-`ControlledSubject` defect as the volley — see
/// `crate::abilities::ranged::volley`'s twin of this test for the why.
#[test]
fn two_driven_bodies_each_rain_their_own_meteors() {
    use crate::abilities::test_support::spawn_seated_body_holding;
    let mut app = test_app();
    app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
    let a = spawn_seated_body_holding(
        &mut app,
        METEOR_ID,
        0,
        "seat_a",
        ae::Vec2::new(100.0, 100.0),
    );
    let b = spawn_seated_body_holding(
        &mut app,
        METEOR_ID,
        1,
        "seat_b",
        ae::Vec2::new(600.0, 100.0),
    );
    for body in [a, b] {
        app.world_mut()
            .get_mut::<ActorControl>(body)
            .unwrap()
            .0
            .melee_pressed = true;
    }
    app.update();

    let owners: Vec<_> = app
        .world_mut()
        .query::<&ambition_projectiles::ProjectileOwner>()
        .iter(app.world())
        .map(|o| o.0)
        .collect();
    assert!(
        owners.iter().any(|&o| o == a),
        "seat a rained nothing: {owners:?}"
    );
    assert!(
        owners.iter().any(|&o| o == b),
        "seat b rained nothing: {owners:?}"
    );
    assert_eq!(
        owners.iter().filter(|&&o| o == a).count(),
        owners.iter().filter(|&&o| o == b).count(),
        "the two seats fired the same ability, so their strike counts must match: {owners:?}"
    );
}
