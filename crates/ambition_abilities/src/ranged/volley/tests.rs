use super::*;
use crate::test_support::spawn_primary_player_holding;
use crate::test_support::live_projectile_bodies;
use ambition_projectiles::ProjectileSeqCounter;

fn test_app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_projectiles::ProjectileSpawnRequest>();
    app.init_resource::<ProjectileSeqCounter>();
    // Chain the immediate projectile materializer after the fire system.
    app.add_systems(
        Update,
        (
            fire_volley_system,
            ambition_projectiles::materialize_projectiles_for_this_tick,
        )
            .chain(),
    );
    app
}

#[test]
fn attack_with_the_volley_spawns_a_fan_of_player_faction_bolts() {
    let mut app = test_app();
    let player = spawn_primary_player_holding(&mut app, VOLLEY_ID);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    let bodies = live_projectile_bodies(&mut app);
    assert_eq!(bodies.len(), VOLLEY_SHOT_COUNT, "one bolt per fan slot");
    // Every bolt is owned by the firing player entity, so a kill attributes
    // back to them (the executor stamps `ProjectileOwner` from the request).
    let owners: Vec<_> = app
        .world_mut()
        .query::<&ambition_projectiles::ProjectileOwner>()
        .iter(app.world())
        .map(|o| o.0)
        .collect();
    assert_eq!(
        owners.len(),
        VOLLEY_SHOT_COUNT,
        "every bolt carries an owner"
    );
    assert!(
        owners.iter().all(|&o| o == player),
        "bolts are owned by the firing player, got {owners:?} (player {player:?})"
    );
    // The bolts fan out — not all the same direction.
    let dirs: Vec<f32> = bodies
        .iter()
        .map(|b| b.body.kin.vel.y.atan2(b.body.kin.vel.x))
        .collect();
    assert!(
        dirs.windows(2).any(|w| (w[0] - w[1]).abs() > 1e-3),
        "the volley spreads across distinct angles"
    );
}

#[test]
fn no_volley_without_attack() {
    let mut app = test_app();
    spawn_primary_player_holding(&mut app, VOLLEY_ID);
    app.update();
    assert_eq!(live_projectile_bodies(&mut app).len(), 0);
}

#[test]
fn volley_origin_uses_extent_along_the_local_aim_axis() {
    let size = ae::Vec2::new(24.0, 40.0);
    let side = volley_origin_local_offset(ae::Vec2::new(1.0, 0.0), size);
    let head = volley_origin_local_offset(ae::Vec2::new(0.0, -1.0), size);
    assert_eq!(side, ae::Vec2::new(20.0, 0.0));
    assert_eq!(head, ae::Vec2::new(0.0, -28.0));
}

#[test]
fn volley_origin_is_c4_equivariant_for_local_aim() {
    let pos = ae::Vec2::new(100.0, 100.0);
    let size = ae::Vec2::new(24.0, 40.0);
    let local_aim = ae::Vec2::new(0.0, -1.0);
    let expected_local = volley_origin_local_offset(local_aim, size);
    for gravity_dir in [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(-1.0, 0.0),
    ] {
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let world_origin = volley_origin_world(pos, size, local_aim, frame);
        let local_origin = frame.to_local(world_origin - pos);
        assert!(
            (local_origin - expected_local).length() < 0.001,
            "volley origin should preserve local geometry under gravity {gravity_dir:?}; got {local_origin:?}"
        );
    }
}

/// ⭐⭐ A SECOND DRIVEN BODY FIRES ITS OWN VOLLEY.
///
/// ⛔⛔ THIS ABILITY READ `ControlledSubject`, WHICH IS ONE ENTITY. On a couch
/// stage the second seat's gauntlet did nothing at all, and with nobody
/// possessing anything (`ControlledSubject(None)`) NEITHER seat fired — the same
/// singular-subject defect `fire_held_ranged_system` was already fixed for.
///
/// The assertion is ATTRIBUTION, not a count: each seat's bolts must be owned by
/// the body that fired them, because ownership is what a kill is credited
/// through.
#[test]
fn two_driven_bodies_each_fire_their_own_volley() {
    use crate::test_support::spawn_seated_body_holding;
    let mut app = test_app();
    app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
    let a = spawn_seated_body_holding(
        &mut app,
        VOLLEY_ID,
        0,
        "seat_a",
        ae::Vec2::new(100.0, 100.0),
    );
    let b = spawn_seated_body_holding(
        &mut app,
        VOLLEY_ID,
        1,
        "seat_b",
        ae::Vec2::new(400.0, 100.0),
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
    assert_eq!(
        owners.iter().filter(|&&o| o == a).count(),
        VOLLEY_SHOT_COUNT,
        "seat a's fan is missing or misattributed: {owners:?}"
    );
    assert_eq!(
        owners.iter().filter(|&&o| o == b).count(),
        VOLLEY_SHOT_COUNT,
        "seat b's fan is missing or misattributed: {owners:?}"
    );
}
