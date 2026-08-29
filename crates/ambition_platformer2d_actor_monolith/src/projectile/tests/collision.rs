//! Projectile-vs-world / projectile-vs-actor collision tests.
//! Floor bounce, one-way bounce + passthrough, Hadouken expire,
//! enemy hit detection. Each test builds its own `App` because the
//! shared `min_app()` fixture's `dummy_world` carries a far-side wall
//! that interferes with controlled-collision setups.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_projectiles::ProjectileKind;

use super::{advance_time, min_app, projectile_test_app, BodyHealth};
use ambition_combat::components::ActorIdentity;

/// Pre-spawn a fireball directly into the body list and place it
/// just beside an ECS-hostile actor. After one tick the fireball
/// overlaps the actor AABB, queues an ECS damage event, and the
/// follow-up damage drain lowers actor HP and despawns the projectile.
#[test]
fn fireball_damages_enemy_on_intersect() {
    let mut app = min_app();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    app.add_systems(
        Startup,
        |mut commands: Commands,
         catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>| {
            crate::features::spawn_encounter_mob(
                &mut commands,
                &catalog,
                &Default::default(),
                &crate::character_runtime::fixture_cast(&["fixture_striker"]),
                ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                "projectile_test",
                crate::features::EncounterMobSeed {
                    id: "test_enemy".into(),
                    character: Some("fixture_striker"),
                    brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                        "fixture_striker".into(),
                    ),
                    pos: ae::Vec2::new(400.0, 300.0),
                    size: ae::Vec2::new(28.0, 46.0),
                },
            );
        },
    );
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
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        // Override velocity / pos so the next tick definitely
        // overlaps the enemy AABB regardless of arc tuning.
        body.kin.pos = ae::Vec2::new(395.0, 300.0);
        body.kin.vel = ae::Vec2::new(50.0, 0.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();

    let (enemy_health, enemy_max) = {
        let world = app.world_mut();
        let mut query = world.query::<(&ActorIdentity, &BodyHealth)>();
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
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(500.0, 395.0);
        body.kin.vel = ae::Vec2::new(60.0, 240.0);
        starting_bounces = body.game.bounces_remaining;
        assert!(starting_bounces > 0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
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
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(500.0, 395.0);
        body.kin.vel = ae::Vec2::new(60.0, 240.0);
        starting_bounces = body.game.bounces_remaining;
        assert!(starting_bounces > 0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
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

/// A fireball flying horizontally beneath a thin one-way platform (or rising up into one from
/// below) must NOT be stopped by it — the platform is non-solid from below.
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
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        // Centre the body inside the platform's y-range so the
        // contact is unambiguously a side / overlap, not a top
        // landing. Velocity is purely horizontal.
        body.kin.pos = ae::Vec2::new(500.0, 404.0);
        body.kin.vel = ae::Vec2::new(360.0, 0.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
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
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(595.0, 300.0);
        body.kin.vel = ae::Vec2::new(520.0, 0.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
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

/// ⛔⛔ WHICH OF TWO OVERLAPPING VICTIMS A SHOT HITS WAS QUERY ORDER (D199).
///
/// The victim loop `break`s on the first row that qualifies, and it iterated a
/// Bevy query — archetype order, which is not a promise and which a rollback
/// resimulation does not reproduce. Damage is rollback-authoritative state, so
/// deciding it by iteration order is deterministically wrong.
///
/// ⭐ THE ARM THAT CATCHES IT IS SPAWN ORDER, not a single arrangement: the two
/// bodies are spawned near and far, then far and near, and the SAME one must be
/// hit both times. A test that spawned them once would agree with the bug
/// whenever the archetype happened to list the near body first.
#[test]
fn a_shot_reaching_two_bodies_hits_the_nearer_one_whichever_was_spawned_first() {
    fn hit_victim(near_first: bool) -> String {
        let mut app = min_app();
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
        app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
        // ⛔ BOTH MUST ACTUALLY OVERLAP THE SHOT, or the test proves nothing:
        // a fireball at 395 moving 50px/s covers 0.8px in a tick, so bodies
        // 40px apart are not a choice — only the near one is ever reachable and
        // the loop has nothing to arbitrate. 28px-wide bodies at 400 and 408
        // both contain the endpoint. (Measured: the first version placed them at
        // 400 and 440 and passed with the ordering REMOVED.)
        let order = if near_first {
            [("near", 400.0), ("far", 408.0)]
        } else {
            [("far", 408.0), ("near", 400.0)]
        };
        app.add_systems(
            Startup,
            move |mut commands: Commands,
                  catalog: Res<
                ambition_characters::actor::character_catalog::CharacterCatalog,
            >| {
                for (id, x) in order {
                    crate::features::spawn_encounter_mob(
                        &mut commands,
                        &catalog,
                        &Default::default(),
                        &crate::character_runtime::fixture_cast(&["fixture_striker"]),
                        ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                        "projectile_test",
                        crate::features::EncounterMobSeed {
                            id: id.into(),
                            character: Some("fixture_striker"),
                            brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                                "fixture_striker".into(),
                            ),
                            pos: ae::Vec2::new(x, 300.0),
                            size: ae::Vec2::new(28.0, 46.0),
                        },
                    );
                }
            },
        );
        app.update();
        {
            let spec = ProjectileKind::Fireball.spec(
                ae::Vec2::new(395.0, 300.0),
                ae::Vec2::new(1.0, 0.0),
                1.0,
            );
            let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
            body.kin.pos = ae::Vec2::new(395.0, 300.0);
            body.kin.vel = ae::Vec2::new(50.0, 0.0);
            crate::projectile::tests::spawn_player_projectile(&mut app, body);
        }
        advance_time(&mut app, 0.016);
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<(&ActorIdentity, &BodyHealth)>();
        let hurt: Vec<String> = query
            .iter(world)
            .filter(|(_, health)| health.health.current < health.health.max)
            .map(|(identity, _)| identity.id().to_string())
            .collect();
        assert_eq!(
            hurt.len(),
            1,
            "exactly one body should take the shot: {hurt:?}"
        );
        hurt.into_iter().next().unwrap()
    }

    assert_eq!(hit_victim(true), "near");
    assert_eq!(
        hit_victim(false),
        "near",
        "the far body took the shot because it was spawned first — the victim \
         is being chosen by archetype order, which a rewind does not reproduce"
    );
}

/// D199: a shot must not damage a victim standing BEHIND a wall.
///
/// ⛔ THE ORDER UNDER TEST. `projectile/systems.rs` moves the shot to its new
/// endpoint, then runs the victim loop (overlap → damage → despawn), and only
/// then calls `resolve_world_collision`. So a shot whose endpoint lands on a
/// victim behind blocking geometry can damage them before anything asks whether
/// a wall stopped the travel — and because both tests are ENDPOINT overlap
/// rather than swept, a fast shot crosses a thin wall in one tick.
///
/// This is the cheap regression D199 asks for, and it is the first BEHAVIOURAL
/// check of that ordering: the row's confirmation was a reading of the source.
#[test]
fn a_shot_does_not_damage_a_victim_standing_behind_a_wall() {
    // A wall between the muzzle and the victim. Thin on purpose: the swept
    // question and the ordering question have the same symptom here, and a thin
    // wall is what a fast shot tunnels.
    let world = ae::World::new(
        "wall_between",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![ae::Block::solid(
            "wall",
            ae::Vec2::new(380.0, 260.0),
            ae::Vec2::new(8.0, 120.0),
        )],
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    app.add_systems(
        Startup,
        |mut commands: Commands,
         catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>| {
            crate::features::spawn_encounter_mob(
                &mut commands,
                &catalog,
                &Default::default(),
                &crate::character_runtime::fixture_cast(&["fixture_striker"]),
                ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                "projectile_test",
                crate::features::EncounterMobSeed {
                    id: "walled_enemy".into(),
                    character: Some("fixture_striker"),
                    brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                        "fixture_striker".into(),
                    ),
                    // BEHIND the wall from the shot's point of view.
                    pos: ae::Vec2::new(400.0, 300.0),
                    size: ae::Vec2::new(28.0, 46.0),
                },
            );
        },
    );
    app.update();
    {
        let spec = ProjectileKind::Fireball.spec(
            ae::Vec2::new(360.0, 300.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        // Fast enough that ONE tick carries the endpoint past the wall and onto
        // the victim — which is exactly the case the endpoint test cannot see.
        body.kin.pos = ae::Vec2::new(360.0, 300.0);
        body.kin.vel = ae::Vec2::new(4000.0, 0.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();

    let (health, max) = {
        let world = app.world_mut();
        let mut query = world.query::<(&ActorIdentity, &BodyHealth)>();
        let (_, health) = query
            .iter(world)
            .find(|(identity, _)| identity.id() == "walled_enemy")
            .expect("the walled enemy should be spawned as an ECS actor");
        (health.health.current, health.health.max)
    };
    assert_eq!(
        health, max,
        "a wall stands between the muzzle and the victim, so the victim must take \
         NO damage (was {max}, now {health})"
    );

    // ⚠ AND WHAT THE SHOT ITSELF DID IS A SEPARATE QUESTION — D199's SWEPT half.
    // The damage answer above is settled by a raycast over the whole leg, so it
    // holds at any speed. Whether the shot STOPPED at the wall is decided by
    // `resolve_world_collision`, which is an ENDPOINT test: at 4000 px/s the
    // endpoint is already past a thin wall, so the shot can pass through it while
    // correctly failing to damage anyone behind it. This records which half is
    // which rather than asserting the unfixed one.
    let survivors = crate::projectile::tests::projectile_bodies(&mut app);
    eprintln!(
        "[D199] after one 4000px/s tick through an 8px wall: {} projectile body/bodies remain",
        survivors.len()
    );
}
