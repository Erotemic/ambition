//! Projectile-vs-world / projectile-vs-actor collision tests.
//! Floor bounce, one-way bounce + passthrough, Hadouken expire,
//! enemy hit detection. Each test builds its own `App` because the
//! shared `min_app()` fixture's `dummy_world` carries a far-side wall
//! that interferes with controlled-collision setups.

use bevy::prelude::*;

use ambition_engine_core as ae;
use crate::projectile::ProjectileKind;

use super::{advance_time, min_app, projectile_test_app, ActorHealth, ActorIdentity};

/// Pre-spawn a fireball directly into the body list and place it
/// just beside an ECS-hostile actor. After one tick the fireball
/// overlaps the actor AABB, queues an ECS damage event, and the
/// follow-up damage drain lowers actor HP and despawns the projectile.
#[test]
fn fireball_damages_enemy_on_intersect() {
    let mut app = min_app();
    app.add_systems(Startup, |mut commands: Commands| {
        crate::features::spawn_encounter_mob(
            &mut commands,
            "projectile_test",
            "test_enemy".into(),
            ambition_characters::actor::EnemyBrain::Custom("medium_striker".into()),
            ae::Vec2::new(400.0, 300.0),
            ae::Vec2::new(28.0, 46.0),
        );
    });
    // Run startup once so the Commands-spawned ECS actor exists before
    // the projectile tick. Encounter-spawned mobs enter the world through
    // Commands at schedule boundaries, so a projectile should not be expected
    // to hit an actor that has only been queued for spawning this same frame.
    app.update();
    // Inject a fireball moving toward the enemy.
    {
        let spec = ProjectileKind::Fireball.spec(
            ae::Vec2::new(395.0, 300.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = crate::projectile::ProjectileBody::from_spec(spec);
        // Override velocity / pos so the next tick definitely
        // overlaps the enemy AABB regardless of arc tuning.
        body.kin.pos = ae::Vec2::new(395.0, 300.0);
        body.kin.vel = ae::Vec2::new(50.0, 0.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body, "");
    }
    advance_time(&mut app, 0.016);
    app.update();

    let (enemy_health, enemy_max) = {
        let world = app.world_mut();
        let mut query = world.query::<(&ActorIdentity, &ActorHealth)>();
        let (_, health) = query
            .iter(world)
            .find(|(identity, _)| identity.id() == "test_enemy")
            .expect("test enemy should be spawned as an ECS actor");
        (health.health.current, health.health.max)
    };
    assert!(
        enemy_health < enemy_max,
        "enemy must lose HP from a projectile hit (was {}, now {})",
        enemy_max,
        enemy_health
    );
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert!(
        bodies.is_empty(),
        "fireball must despawn after hitting an actor"
    );
}

/// Drop a fireball onto a floor block. The first tick should
/// produce a y-axis bounce: vy flips upward, bounces_remaining
/// drops by one, and the projectile must remain in the body list.
#[test]
fn fireball_bounces_off_floor_in_system() {
    // World with a single floor block well below the spawn point.
    let world = ae::World::new(
        "bounce_test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![ae::Block::solid(
            "floor",
            ae::Vec2::new(0.0, 400.0),
            ae::Vec2::new(2000.0, 32.0),
        )],
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);

    // Spawn a fireball just above the floor moving downward.
    let starting_bounces;
    {
        let spec = ProjectileKind::Fireball.spec(
            ae::Vec2::new(500.0, 380.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = crate::projectile::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(500.0, 395.0);
        body.kin.vel = ae::Vec2::new(60.0, 240.0);
        starting_bounces = body.game.bounces_remaining;
        assert!(starting_bounces > 0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body, "");
    }
    advance_time(&mut app, 0.016);
    app.update();
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(bodies.len(), 1, "fireball must survive a floor bounce");
    let body = &bodies[0];
    assert!(
        body.kin.vel.y < 0.0,
        "post-bounce vy must be upward; got {}",
        body.kin.vel.y
    );
    assert_eq!(body.game.bounces_remaining, starting_bounces - 1);
}

/// Same scenario as `fireball_bounces_off_floor_in_system`, but the
/// floor block is a `OneWay` platform. The fireball must still
/// bounce — the player expects skipping fireballs to skip across
/// thin ledges identically to thick floors.
#[test]
fn fireball_bounces_off_one_way_platform_in_system() {
    let world = ae::World::new(
        "one_way_bounce_test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![ae::Block::one_way(
            "ledge",
            ae::Vec2::new(0.0, 400.0),
            ae::Vec2::new(2000.0, 8.0),
        )],
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);

    let starting_bounces;
    {
        let spec = ProjectileKind::Fireball.spec(
            ae::Vec2::new(500.0, 380.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = crate::projectile::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(500.0, 395.0);
        body.kin.vel = ae::Vec2::new(60.0, 240.0);
        starting_bounces = body.game.bounces_remaining;
        assert!(starting_bounces > 0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body, "");
    }
    advance_time(&mut app, 0.016);
    app.update();
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(
        bodies.len(),
        1,
        "fireball must survive a one-way-platform bounce"
    );
    let body = &bodies[0];
    assert!(
        body.kin.vel.y < 0.0,
        "post-bounce vy must be upward; got {}",
        body.kin.vel.y
    );
    assert_eq!(body.game.bounces_remaining, starting_bounces - 1);
}

/// A fireball flying horizontally beneath a thin one-way platform
/// (or rising up into one from below) must NOT be stopped by it —
/// the platform is non-solid from below. Pin the "fireballs pass
/// through one-ways unless they land on top" rule at the system
/// level so a future regression that treats one-ways like solid
/// walls breaks the test.
#[test]
fn fireball_passes_through_one_way_from_below_in_system() {
    let world = ae::World::new(
        "one_way_passthrough_test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![ae::Block::one_way(
            "ledge",
            ae::Vec2::new(0.0, 400.0),
            ae::Vec2::new(2000.0, 8.0),
        )],
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 500.0), 1.0);

    {
        let spec = ProjectileKind::Fireball.spec(
            ae::Vec2::new(500.0, 405.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = crate::projectile::ProjectileBody::from_spec(spec);
        // Centre the body inside the platform's y-range so the
        // contact is unambiguously a side / overlap, not a top
        // landing. Velocity is purely horizontal.
        body.kin.pos = ae::Vec2::new(500.0, 404.0);
        body.kin.vel = ae::Vec2::new(360.0, 0.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body, "");
    }
    advance_time(&mut app, 0.016);
    app.update();
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(
        bodies.len(),
        1,
        "fireball must pass through a one-way platform on side contact"
    );
    let body = &bodies[0];
    assert!(
        body.kin.vel.x > 0.0,
        "horizontal velocity should be unchanged after passthrough; got {}",
        body.kin.vel.x
    );
}

/// Hadouken spawns with `bounces_remaining = 0`. Hitting any solid
/// expires it on the first contact — pinning the "horizontal
/// projectile that disappears on first wall" behavior at the
/// system level (engine test pinned it at the unit level).
#[test]
fn hadouken_expires_on_solid_in_system() {
    let world = ae::World::new(
        "wall_test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![ae::Block::solid(
            "wall",
            ae::Vec2::new(600.0, 0.0),
            ae::Vec2::new(40.0, 800.0),
        )],
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(500.0, 300.0), 1.0);

    {
        let spec = ProjectileKind::Hadouken.spec(
            ae::Vec2::new(580.0, 300.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = crate::projectile::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(595.0, 300.0);
        body.kin.vel = ae::Vec2::new(520.0, 0.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body, "");
    }
    advance_time(&mut app, 0.016);
    app.update();
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert!(
        bodies.is_empty(),
        "Hadouken must expire on first solid hit (no bounces); still alive: {}",
        bodies.len()
    );
}
