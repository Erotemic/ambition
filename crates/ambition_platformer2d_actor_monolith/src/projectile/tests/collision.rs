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
                ambition_encounter::mob_seed::EncounterMobSeed {
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
                        ambition_encounter::mob_seed::EncounterMobSeed {
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
                ambition_encounter::mob_seed::EncounterMobSeed {
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

    // ⭐ AND THE SHOT ITSELF MUST STOP THERE — D199's SWEPT half, a separate
    // question from the damage one above. `resolve_world_collision` is an
    // ENDPOINT test, so at 4000 px/s the endpoint is already past an 8px wall and
    // the shot sailed through it (measured: one body still in flight) while
    // correctly damaging nobody. The caller now pulls a tunnelled shot back to
    // its crossing before resolving, which restores the precondition that test
    // was written for.
    let survivors = crate::projectile::tests::projectile_bodies(&mut app);
    assert!(
        survivors.is_empty(),
        "a shot fast enough to end the tick past a thin wall must still be stopped \
         BY that wall, not tunnel it — {} body/bodies still in flight",
        survivors.len()
    );
}

/// ⛔⛔ **A2b: A CRATE BEHIND A WALL IS BEHIND THE WALL.**
///
/// The boss/breakable branch computed a swept contact and, if it found one,
/// emitted the targeted hit and the splash and `continue`d — so
/// `resolve_world_collision` ran only when NO feature was reached. The ordering
/// was therefore *"feature contact, else world"* rather than *"whichever came
/// first"*, and a crate standing behind an unrelated solid was broken through
/// it. The body branch had asked the ordering question since D199; this branch
/// never had.
///
/// ⭐ THE INVERSE IS THE ANTI-VACUITY FLOOR, and it moves the WALL rather than
/// the crate: same shot, same crate, same speed, so an arm that stopped breaking
/// crates entirely would fail it. A guard that only ever asserts "nothing
/// happened" is satisfied by a projectile road that does nothing.
#[test]
fn a_crate_behind_a_wall_is_not_broken_and_one_in_front_of_it_is() {
    use ambition_combat::components::{BreakableFeature, FeatureName};

    // The shot: box half-extent 12 in x, so its leading edge starts at 372 and
    // travels 64 px in one 0.016 s tick at 4000 px/s.
    fn crate_broken_with_wall_at(wall_x: f32) -> bool {
        let world = ae::World::new(
            "wall_and_crate",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            vec![ae::Block::solid(
                "wall",
                ae::Vec2::new(wall_x, 300.0),
                ae::Vec2::new(6.0, 120.0),
            )],
        );
        let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
        let breakable = app
            .world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
                ambition_combat::components::FeatureId::new("crate"),
                FeatureName::new("crate"),
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement("crate"),
                ambition_combat::components::CenteredAabb::from_center_size(
                    ae::Vec2::new(420.0, 300.0),
                    ae::Vec2::new(12.0, 46.0),
                ),
                BreakableFeature::new(ambition_interaction::Breakable::new("crate", 1)),
            ))
            .id();
        {
            let spec = ProjectileKind::Fireball.spec(
                ae::Vec2::new(360.0, 300.0),
                ae::Vec2::new(1.0, 0.0),
                1.0,
            );
            let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
            body.kin.pos = ae::Vec2::new(360.0, 300.0);
            body.kin.vel = ae::Vec2::new(4000.0, 0.0);
            crate::projectile::tests::spawn_player_projectile(&mut app, body);
        }
        advance_time(&mut app, 0.016);
        app.update();
        app.world()
            .get::<BreakableFeature>(breakable)
            .expect("the crate is still an entity")
            .broken()
    }

    // ⛔ THE FLOOR FIRST: the wall stands PAST the crate (leading edge reaches
    // the crate at 0.66 of the leg and the wall at 0.84), so the crate is
    // reached first and breaks exactly as it always did.
    assert!(
        crate_broken_with_wall_at(432.0),
        "a crate reached BEFORE the wall on the same leg was not broken, so the \
         arm below would be measuring a branch that breaks nothing"
    );
    // ⛔ AND THE DEFECT: the wall now stands between the muzzle and the crate
    // (wall at 0.22 of the leg, crate at 0.66). The shot cannot reach it.
    assert!(
        !crate_broken_with_wall_at(392.0),
        "a crate standing BEHIND a solid wall was broken through it — the \
         boss/breakable branch is emitting its hit without comparing the \
         contact time against the wall's"
    );
}

/// ⛔⛔ **A2b: A CRATE IN FRONT OF A BODY IS IN FRONT OF IT — AND VICE VERSA.**
///
/// Bodies and features were resolved in separate passes: the body loop ran
/// first and, on a hit, `continue`d past the whole projectile step, so the
/// boss/breakable candidate was not even COMPUTED. "Body" therefore beat "boss
/// or breakable" by being the earlier code branch — a crate at 0.2 of the leg
/// lost to a body at 0.8. That is exactly the family knowledge A2 exists to
/// remove: the protocol orders contacts within a projectile by finite time of
/// impact, and a family name is not a time.
///
/// ⭐ BOTH DIRECTIONS, and neither is the floor for the other — a road that
/// always picked the crate would pass one arm and fail the other, and so would
/// one that always picked the body.
#[test]
fn the_earliest_contact_wins_whether_it_is_a_body_or_a_crate() {
    use ambition_combat::components::{BreakableFeature, FeatureName};

    /// Returns `(crate_broken, body_hurt)` for a crate at `crate_x`. The body
    /// stands at 440 and the shot's leading edge starts at 372, covering 64 px.
    fn contact_with_crate_at(crate_x: f32) -> (bool, bool) {
        let world = ae::World::new(
            "open_lane",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        );
        let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
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
                    ambition_encounter::mob_seed::EncounterMobSeed {
                        id: "lane_enemy".into(),
                        character: Some("fixture_striker"),
                        brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                            "fixture_striker".into(),
                        ),
                        pos: ae::Vec2::new(440.0, 300.0),
                        size: ae::Vec2::new(28.0, 46.0),
                    },
                );
            },
        );
        app.update();
        let breakable = app
            .world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
                ambition_combat::components::FeatureId::new("crate"),
                FeatureName::new("crate"),
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement("crate"),
                ambition_combat::components::CenteredAabb::from_center_size(
                    ae::Vec2::new(crate_x, 300.0),
                    ae::Vec2::new(12.0, 46.0),
                ),
                BreakableFeature::new(ambition_interaction::Breakable::new("crate", 1)),
            ))
            .id();
        {
            let spec = ProjectileKind::Fireball.spec(
                ae::Vec2::new(360.0, 300.0),
                ae::Vec2::new(1.0, 0.0),
                1.0,
            );
            let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
            body.kin.pos = ae::Vec2::new(360.0, 300.0);
            body.kin.vel = ae::Vec2::new(4000.0, 0.0);
            crate::projectile::tests::spawn_player_projectile(&mut app, body);
        }
        advance_time(&mut app, 0.016);
        app.update();

        let broken = app
            .world()
            .get::<BreakableFeature>(breakable)
            .expect("the crate is still an entity")
            .broken();
        let hurt = {
            let world = app.world_mut();
            let mut query = world.query::<(&ActorIdentity, &BodyHealth)>();
            let (_, health) = query
                .iter(world)
                .find(|(identity, _)| identity.id() == "lane_enemy")
                .expect("the lane enemy should be spawned as an ECS actor");
            health.health.current < health.health.max
        };
        (broken, hurt)
    }

    // The crate stands at 390 (384..396) and the body's near face at 426, so the
    // crate is reached first and owns the contact.
    let (crate_first_broken, crate_first_hurt) = contact_with_crate_at(390.0);
    assert!(
        crate_first_broken,
        "a crate reached BEFORE the body lost the contact to it — the two \
         families are still being resolved in separate passes"
    );
    assert!(
        !crate_first_hurt,
        "the body behind the crate took the hit as well as the crate"
    );

    // Now the crate stands PAST the body (crate 464..476, body near face 426),
    // so the body is reached first and owns it.
    let (body_first_broken, body_first_hurt) = contact_with_crate_at(470.0);
    assert!(
        body_first_hurt,
        "a body reached BEFORE the crate did not take the hit, so the arm above \
         would be satisfied by a road that always picks the crate"
    );
    assert!(
        !body_first_broken,
        "the crate behind the body was broken as well as the body"
    );
}

/// ⛔⛔ **A2b: A WALL PAST A BIG BODY'S NEAR FACE DOES NOT SAVE IT.**
///
/// The body branch asked its obstruction question by sweeping from the muzzle to
/// the victim's CENTRE — *"is a wall before the victim's middle?"* — which is a
/// different question from the one the protocol asks and wrong in the direction
/// that costs a legitimate hit. On a wide body the shot touches the near face
/// well before the centre; a wall standing between that face and the centre is
/// BEHIND the contact that actually happened, and the old test refused the hit
/// anyway.
///
/// Geometry: the shot's leading edge starts at 372 and covers 128 px in one
/// tick. The body spans 420..540, so contact is at 0.375 of the leg. The wall
/// spans 448..456 — inside the body, past its near face — and is reached at
/// 0.594. The body is first, so the body is hit. Under the centre-sweep the wall
/// sat at 0.63 of a 120 px cast to the centre and refused it.
#[test]
fn a_wall_inside_a_wide_body_past_its_near_face_does_not_stop_the_hit() {
    let world = ae::World::new(
        "wall_inside_body",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![ae::Block::solid(
            "pillar",
            ae::Vec2::new(452.0, 300.0),
            ae::Vec2::new(4.0, 120.0),
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
                ambition_encounter::mob_seed::EncounterMobSeed {
                    id: "wide_enemy".into(),
                    character: Some("fixture_striker"),
                    brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                        "fixture_striker".into(),
                    ),
                    pos: ae::Vec2::new(480.0, 300.0),
                    // WIDE on purpose: the near face and the centre are 60 px
                    // apart, which is the whole distinction under test.
                    size: ae::Vec2::new(120.0, 46.0),
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
        body.kin.pos = ae::Vec2::new(360.0, 300.0);
        body.kin.vel = ae::Vec2::new(8000.0, 0.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();

    let (health, max) = {
        let world = app.world_mut();
        let mut query = world.query::<(&ActorIdentity, &BodyHealth)>();
        let (_, health) = query
            .iter(world)
            .find(|(identity, _)| identity.id() == "wide_enemy")
            .expect("the wide enemy should be spawned as an ECS actor");
        (health.health.current, health.health.max)
    };
    assert!(
        health < max,
        "the shot touched the body's near face at 0.375 of its leg and the wall \
         only at 0.594, so the body must take the hit (still {health} of {max})"
    );
}

/// ⛔⛔ **A2b: A FAST SHOT MUST HIT A THIN BODY IT CROSSES BETWEEN SAMPLES.**
///
/// Target contact was ENDPOINT overlap: the stepper moved the shot to its new
/// position and asked whether the box there reached the victim's published
/// geometry. At 4000 px/s a tick carries the shot 64 px, so a body thinner than
/// that is behind the endpoint before anything looks — the bolt flies through it
/// and nobody is touched. The obstruction test beside it was already swept,
/// which is what made the pair inconsistent: the shot could be stopped by a wall
/// it crossed and not by a body it crossed.
///
/// ⭐ THE SLOW ARM IS THE ANTI-VACUITY FLOOR AND ALSO THE PARITY CHECK. The same
/// victim at a speed whose endpoint lands on it must still be hit — a swept test
/// that reports contact for everything, or one that broke the ordinary case,
/// would pass the fast arm alone.
#[test]
fn a_fast_shot_hits_a_thin_body_it_crosses_within_one_tick() {
    fn victim_hp_after_a_shot_at(speed: f32) -> (i32, i32) {
        // No geometry at all: this row is about the TARGET test, and a wall
        // would let an obstruction answer stand in for a contact answer.
        let world = ae::World::new(
            "open_lane",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        );
        let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
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
                    ambition_encounter::mob_seed::EncounterMobSeed {
                        id: "thin_enemy".into(),
                        character: Some("fixture_striker"),
                        brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                            "fixture_striker".into(),
                        ),
                        pos: ae::Vec2::new(400.0, 300.0),
                        // THIN along the shot's axis: 6 px wide, well inside the
                        // 64 px a fast tick covers.
                        size: ae::Vec2::new(6.0, 46.0),
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
            body.kin.pos = ae::Vec2::new(360.0, 300.0);
            body.kin.vel = ae::Vec2::new(speed, 0.0);
            crate::projectile::tests::spawn_player_projectile(&mut app, body);
        }
        advance_time(&mut app, 0.016);
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<(&ActorIdentity, &BodyHealth)>();
        let (_, health) = query
            .iter(world)
            .find(|(identity, _)| identity.id() == "thin_enemy")
            .expect("the thin enemy should be spawned as an ECS actor");
        (health.health.current, health.health.max)
    }

    // ⛔ THE PREMISE. A speed whose endpoint lands ON the body: the ordinary
    // case, and it must be unchanged.
    let (slow, max) = victim_hp_after_a_shot_at(2500.0);
    assert!(
        slow < max,
        "the ordinary endpoint-overlap case stopped landing, so the fast arm \
         below is measuring a contact test that refuses everything rather than \
         one that sweeps"
    );

    let (fast, max) = victim_hp_after_a_shot_at(4000.0);
    assert!(
        fast < max,
        "a shot crossing a 6 px body at 4000 px/s passed through it untouched \
         (was {max}, still {fast}). One tick carries the box 64 px, so the \
         endpoint is already past the body and an overlap test at the endpoint \
         sees nothing — the contact has to be asked of the whole leg"
    );
}

/// ⛔⛔ **ONE SHOT BREAKS ONE CRATE, and it used to break every crate its
/// contact box overlapped.**
///
/// The feature branch wrote `HitTarget::UnresolvedFeatures`, which hands the
/// applier a VOLUME rather than a victim — and the applier's breakable fold has
/// no `break`: it damages every breakable that volume intersects. So a shot
/// whose box covered two touching crates destroyed both, and for a multi-part
/// boss which part took the hit was a query-order answer a rewind need not
/// reproduce. The swept contact already chose exactly one recipient; the event
/// names it now.
///
/// ⚠ **THE FIRST VERSION OF THIS FIXTURE COULD NOT SEE THE DEFECT, and its
/// poison passing is what said so.** It put the crates AHEAD of the shot: the
/// contact box sits where the shot first touches, with its leading edge on the
/// near crate's near face, so it can never reach a crate FURTHER ON — the
/// broadcast and the named recipient gave the same answer and the test was
/// green either way. The overlap a single contact box can span is BEHIND the
/// contact point, so the two crates have to be inside the shot's own box at the
/// moment it starts. Measured, not reasoned: the poison was run and passed.
#[test]
fn a_direct_shot_breaks_only_one_of_two_crates_inside_its_contact_box() {
    use ambition_combat::components::{BreakableFeature, FeatureName};

    let world = ae::World::new(
        "open_lane",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        Vec::new(),
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    let spawn_crate = |app: &mut bevy::prelude::App, id: &str, x: f32| {
        app.world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
                ambition_combat::components::FeatureId::new(id),
                FeatureName::new(id),
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement(id),
                ambition_combat::components::CenteredAabb::from_center_size(
                    ae::Vec2::new(x, 300.0),
                    ae::Vec2::new(8.0, 46.0),
                ),
                BreakableFeature::new(ambition_interaction::Breakable::new(id, 1)),
            ))
            .id()
    };
    // The shot spawns at 360 with a 24 px-wide box (348..372), so BOTH of these
    // are already inside it on the frame it steps: one contact box, two crates.
    let a = spawn_crate(&mut app, "crate_a", 356.0);
    let b = spawn_crate(&mut app, "crate_b", 366.0);
    {
        let spec =
            ProjectileKind::Fireball.spec(ae::Vec2::new(360.0, 300.0), ae::Vec2::new(1.0, 0.0), 1.0);
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(360.0, 300.0);
        body.kin.vel = ae::Vec2::new(600.0, 0.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();

    let broken = |app: &bevy::prelude::App, entity: bevy::prelude::Entity| {
        app.world()
            .get::<BreakableFeature>(entity)
            .expect("the crate is still an entity")
            .broken()
    };
    let count = [a, b].into_iter().filter(|e| broken(&app, *e)).count();
    // ⛔ THE FLOOR AND THE MEASUREMENT IN ONE NUMBER. Zero means the road damages
    // nothing and the "only one" claim is vacuous; two is the defect.
    assert_eq!(
        count, 1,
        "one shot resolved {count} crate(s) inside a single contact box. Zero \
         means nothing was damaged at all and this fixture proves nothing; two \
         means the event named no recipient, so the applier took its volume and \
         damaged every breakable that volume overlapped — the swept contact had \
         already chosen exactly one"
    );
}

/// ⛔⛔ **THE DIRECT REQUEST IS WRITTEN BEFORE ITS LANDING SPLASH, ON BOTH
/// RECEIVER ROADS.**
///
/// The ordinary body branch wrote the targeted request and then the area one;
/// the boss/breakable branch wrote the splash FIRST. So for a boss or a
/// breakable the area event was applied before the hit that caused it, and
/// wherever the first application changes what the second finds — a hit that
/// grants invulnerability, or one that breaks the target — the splash was
/// credited and the shot refused.
///
/// ⚠ IT ASSERTS THE EMITTED ORDER, deliberately. The consequence needs content
/// that reacts differently to the two orders (an i-frame window, a one-hit
/// break with a reward), and the ORDER is the thing this patch changes; a
/// fixture that could only see the consequence would be green for either order
/// against a target with none.
#[test]
fn the_direct_request_precedes_its_landing_splash_for_a_feature_target() {
    use ambition_combat::components::{BreakableFeature, FeatureName};
    use ambition_combat::events::{HitEvent, HitTarget};

    #[derive(bevy::prelude::Resource, Default)]
    struct EmittedTargets(Vec<HitTarget>);

    fn record(
        mut reader: bevy::prelude::MessageReader<HitEvent>,
        mut seen: bevy::prelude::ResMut<EmittedTargets>,
    ) {
        seen.0.extend(reader.read().map(|event| event.target));
    }

    let world = ae::World::new(
        "open_lane",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        Vec::new(),
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    app.init_resource::<EmittedTargets>();
    app.add_systems(Update, record.after(crate::projectile::step_projectiles));
    app.world_mut().spawn((
        ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
        ambition_combat::components::FeatureId::new("crate"),
        FeatureName::new("crate"),
        ambition_platformer2d_shared_tangle::sim_id::SimId::placement("crate"),
        ambition_combat::components::CenteredAabb::from_center_size(
            ae::Vec2::new(400.0, 300.0),
            ae::Vec2::new(24.0, 46.0),
        ),
        BreakableFeature::new(ambition_interaction::Breakable::new("crate", 1)),
    ));
    {
        let spec = ProjectileKind::Fireball.spec(
            ae::Vec2::new(360.0, 300.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(360.0, 300.0);
        body.kin.vel = ae::Vec2::new(1500.0, 0.0);
        // A landing splash is what makes the pair, and its ORDER, exist at all.
        body.game.splash_half_extent = 40.0;
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();

    let seen = &app.world().resource::<EmittedTargets>().0;
    // ⛔ THE PREMISE. Both events must exist, or "direct came first" is satisfied
    // by a road that emits one of them.
    assert_eq!(
        seen.len(),
        2,
        "expected exactly the direct request and its one landing splash, got {seen:?}"
    );
    assert!(
        matches!(seen[0], HitTarget::Feature(_)),
        "the landing splash was written before the direct request it belongs to, \
         so a target that reacts to the first application — an i-frame window, a \
         one-hit break — resolves the AREA event and refuses the shot that \
         caused it. Order: {seen:?}"
    );
    assert_eq!(seen[1], HitTarget::Volume, "order: {seen:?}");
}

/// ⛔⛔ **A RETURNING SHOT SURVIVES A CRATE EXACTLY AS IT SURVIVES A BODY.**
///
/// Lifetime is the shot's own policy — `game.returns()` — and the feature branch
/// ignored it: every projectile that reached a boss or a breakable was
/// despawned, unconditionally. So the SAME throw came back when it hit a person
/// and vanished when it clipped a crate. The ordinary body branch has asked the
/// policy the whole time; this branch could not, because it had no way to
/// remember whom it had already hit on this leg and a surviving shot overlaps its
/// target for as many ticks as it takes to pass through.
///
/// ⭐ BOTH ARMS, and the ordinary one is the floor: a non-returning shot must
/// still be spent on contact. Without it, "the shot survived" passes for a road
/// that never despawns anything.
#[test]
fn a_returning_shot_survives_the_crate_it_breaks_and_an_ordinary_one_does_not() {
    use ambition_combat::components::{BreakableFeature, FeatureName};

    fn shot_survived_hitting_a_crate(returning: bool) -> bool {
        let world = ae::World::new(
            "open_lane",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        );
        let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("crate"),
            FeatureName::new("crate"),
            ambition_platformer2d_shared_tangle::sim_id::SimId::placement("crate"),
            ambition_combat::components::CenteredAabb::from_center_size(
                ae::Vec2::new(400.0, 300.0),
                ae::Vec2::new(24.0, 46.0),
            ),
            BreakableFeature::new(ambition_interaction::Breakable::new("crate", 1)),
        ));
        {
            let spec = ProjectileKind::Fireball.spec(
                ae::Vec2::new(360.0, 300.0),
                ae::Vec2::new(1.0, 0.0),
                1.0,
            );
            let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
            body.kin.pos = ae::Vec2::new(360.0, 300.0);
            body.kin.vel = ae::Vec2::new(1500.0, 0.0);
            if returning {
                // `returns()` IS a non-zero return acceleration — the boomerang's
                // own definition, not a separate flag to keep in sync.
                body.game.accel = ae::Vec2::new(-6000.0, 0.0);
            }
            crate::projectile::tests::spawn_player_projectile(&mut app, body);
        }
        advance_time(&mut app, 0.016);
        app.update();
        !crate::projectile::tests::projectile_bodies(&mut app).is_empty()
    }

    assert!(
        !shot_survived_hitting_a_crate(false),
        "an ordinary shot outlived the crate it hit, so the returning arm below \
         is measuring a branch that despawns nothing"
    );
    assert!(
        shot_survived_hitting_a_crate(true),
        "a RETURNING shot was despawned by a crate. The same throw survives a \
         body, because that branch asks `game.returns()`; this one despawned \
         whatever it touched, so a boomerang's lifetime depended on which family \
         of thing happened to be in the way"
    );
}

/// ⛔⛔ **A2b: THE BOSS/BREAKABLE BRANCH IS SWEPT TOO, and it is a SECOND road.**
///
/// The ordinary body branch and the feature branch are different code with
/// different geometry sources, and only the first was made swept. A shot
/// crossing a thin crate between two samples had no endpoint that overlapped it,
/// so nothing was touched — and the branch then handed the applier that same
/// endpoint box to re-test, the identical second half. A guard on the body road
/// stays green through all of it.
#[test]
fn a_fast_shot_breaks_a_thin_crate_it_crosses_within_one_tick() {
    use ambition_combat::components::{BreakableFeature, FeatureName};

    fn crate_broken_after_a_shot_at(speed: f32) -> bool {
        let world = ae::World::new(
            "open_lane",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        );
        let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
        let breakable = app
            .world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
                ambition_combat::components::FeatureId::new("thin_crate"),
                FeatureName::new("thin crate"),
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement("thin_crate"),
                // THIN along the shot's axis: 6 px, well inside the 64 px a fast
                // tick covers.
                ambition_combat::components::CenteredAabb::from_center_size(
                    ae::Vec2::new(400.0, 300.0),
                    ae::Vec2::new(6.0, 46.0),
                ),
                BreakableFeature::new(ambition_interaction::Breakable::new("thin_crate", 1)),
            ))
            .id();
        assert!(!app
            .world()
            .get::<BreakableFeature>(breakable)
            .expect("the crate is spawned")
            .broken());
        {
            let spec = ProjectileKind::Fireball.spec(
                ae::Vec2::new(360.0, 300.0),
                ae::Vec2::new(1.0, 0.0),
                1.0,
            );
            let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
            body.kin.pos = ae::Vec2::new(360.0, 300.0);
            body.kin.vel = ae::Vec2::new(speed, 0.0);
            crate::projectile::tests::spawn_player_projectile(&mut app, body);
        }
        advance_time(&mut app, 0.016);
        app.update();
        app.world()
            .get::<BreakableFeature>(breakable)
            .expect("the crate is still an entity")
            .broken()
    }

    // ⛔ THE PREMISE, and the parity check: the endpoint case must be unchanged.
    assert!(
        crate_broken_after_a_shot_at(2500.0),
        "the ordinary endpoint-overlap case stopped breaking the crate, so the \
         fast arm below would be measuring a contact test that refuses \
         everything rather than one that sweeps"
    );
    assert!(
        crate_broken_after_a_shot_at(4000.0),
        "a shot crossing a 6 px crate at 4000 px/s left it intact. One tick \
         carries the box 64 px, so the endpoint is already past it — the feature \
         branch has to ask the whole leg, and the event it writes has to carry \
         the box AT CONTACT or the applier's own re-test refuses what the sweep \
         found"
    );
}

/// ⛔⛔ **A2b: THE VICTIM'S OBSTRUCTION TEST READS THE SHOT'S OWN WORLD-HIT
/// POLICY, and it used to be hard-coded to "solids only".**
///
/// The world branch swept the shot's box against the blocks its `WorldHitPolicy`
/// says stop it; the victim branch cast `raycast_solids(.., include_one_way =
/// false)` at the victim's CENTRE. Two answers to one question — is something in
/// the way — disagreeing about shape AND about policy. An `ExpireOnContact`
/// shot's contract is that any solid, blink-wall or one-way contact ends it, and
/// it damaged a body straight through a one-way anyway.
///
/// ⭐ BOTH ARMS, and the second is the anti-vacuity floor: a `Bouncing` shot
/// crosses a one-way BY DESIGN, so it must still land. Without it, an
/// obstruction test that refuses everything passes the first arm.
/// ⛔⛔ **A ONE-WAY IS DIRECTIONAL, AND OBSTRUCTION HAD TO LEARN THAT FROM THE
/// RESPONSE.**
///
/// The test above fires HORIZONTALLY at a VERTICAL one-way, so its `Bouncing`
/// arm never touches the blocking side at all — a shot travelling sideways is
/// not descending onto anything, and the arm passes whatever the obstruction
/// filter says about approach direction. That left the real split unguarded:
/// `blocks_this_shot` excluded EVERY `OneWay` for a `Bouncing` shot, while
/// `resolve_world_collision` bounces off one the shot is descending onto — as
/// `fireball_bounces_off_one_way_platform_in_system` above has always proved.
/// So a fireball falling onto a platform was ORDERED as though nothing were
/// there, free to damage a body below it, and then physically bounced off it.
///
/// ⭐ BOTH DIRECTIONS, and the arms differ ONLY in the sign of the velocity.
/// The second is the anti-vacuity floor: a fireball crosses a one-way FROM
/// BELOW by design, so it must still land on a body above. Without it, an
/// obstruction filter that simply blocked every one-way passes the first arm
/// and silently breaks the behaviour the platform exists for.
#[test]
fn a_one_way_stops_a_bouncing_shot_descending_onto_it_and_not_one_rising_through() {
    /// Positive `y` is DOWN in this world, as `fireball_bounces_off_one_way_platform_in_system`
    /// establishes: it starts a shot above a ledge with `vel.y > 0` and asserts
    /// the post-bounce `vel.y` is negative.
    fn victim_hp_with_shot_travelling(vy: f32, shot_y: f32, victim_y: f32) -> (i32, i32) {
        let world = ae::World::new(
            "one_way_horizontal",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            // A WIDE, FLAT ledge spanning the shot's column: y 300..308.
            vec![ae::Block::one_way(
                "ledge",
                ae::Vec2::new(200.0, 300.0),
                ae::Vec2::new(400.0, 8.0),
            )],
        );
        let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
        app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
        app.add_systems(
            Startup,
            move |mut commands: Commands,
                  catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>| {
                crate::features::spawn_encounter_mob(
                    &mut commands,
                    &catalog,
                    &Default::default(),
                    &crate::character_runtime::fixture_cast(&["fixture_striker"]),
                    ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                    "projectile_test",
                    ambition_encounter::mob_seed::EncounterMobSeed {
                        id: "shielded_enemy".into(),
                        character: Some("fixture_striker"),
                        brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                            "fixture_striker".into(),
                        ),
                        // ACROSS the ledge from the shot, in both arms.
                        pos: ae::Vec2::new(400.0, victim_y),
                        size: ae::Vec2::new(28.0, 46.0),
                    },
                );
            },
        );
        app.update();
        {
            let spec = ProjectileKind::Fireball.spec(
                ae::Vec2::new(400.0, shot_y),
                ae::Vec2::new(0.0, vy.signum()),
                1.0,
            );
            let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
            body.kin.pos = ae::Vec2::new(400.0, shot_y);
            // The one thing that differs between the arms.
            body.kin.vel = ae::Vec2::new(0.0, vy);
            body.game.world_hit = ambition_projectiles::WorldHitPolicy::Bouncing;
            assert!(
                body.game.bounces_remaining > 0,
                "the directional rule is `descending AND has a bounce left`; a \
                 spent fireball would pass through and prove nothing"
            );
            crate::projectile::tests::spawn_player_projectile(&mut app, body);
        }
        advance_time(&mut app, 0.016);
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<(&ActorIdentity, &BodyHealth)>();
        let (_, health) = query
            .iter(world)
            .find(|(identity, _)| identity.id() == "shielded_enemy")
            .expect("the shielded enemy should be spawned as an ECS actor");
        (health.health.current, health.health.max)
    }

    // DESCENDING onto the ledge, victim below it.
    let (descending, max) = victim_hp_with_shot_travelling(8000.0, 250.0, 350.0);
    assert_eq!(
        descending, max,
        "a `Bouncing` shot DESCENDING onto a one-way damaged a body below it. \
         The response bounces off that platform -- `fireball_bounces_off_one_way_platform_in_system` \
         -- so ordering the shot as if nothing were in the way is the two roads \
         disagreeing about what stopped it (was {max}, now {descending})"
    );

    // RISING through the ledge, victim above it: the design case, unchanged.
    let (rising, max) = victim_hp_with_shot_travelling(-8000.0, 350.0, 250.0);
    assert!(
        rising < max,
        "a `Bouncing` shot RISING through a one-way must still reach a body \
         above it -- a fireball crosses one from below by design. An obstruction \
         filter that blocks every one-way passes the descending arm and breaks \
         the behaviour the platform exists for"
    );
}

#[test]
fn a_one_way_blocks_the_shot_whose_policy_says_it_should_and_no_other() {
    fn victim_hp_after_a_shot_through_a_one_way(
        world_hit: ambition_projectiles::WorldHitPolicy,
    ) -> (i32, i32) {
        let world = ae::World::new(
            "one_way_between",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            vec![ae::Block::one_way(
                "platform",
                ae::Vec2::new(380.0, 260.0),
                ae::Vec2::new(8.0, 120.0),
            )],
        );
        let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
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
                    ambition_encounter::mob_seed::EncounterMobSeed {
                        id: "shielded_enemy".into(),
                        character: Some("fixture_striker"),
                        brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                            "fixture_striker".into(),
                        ),
                        // BEHIND the one-way from the shot's point of view.
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
            body.kin.pos = ae::Vec2::new(360.0, 300.0);
            body.kin.vel = ae::Vec2::new(4000.0, 0.0);
            // The one thing that differs between the arms.
            body.game.world_hit = world_hit;
            crate::projectile::tests::spawn_player_projectile(&mut app, body);
        }
        advance_time(&mut app, 0.016);
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<(&ActorIdentity, &BodyHealth)>();
        let (_, health) = query
            .iter(world)
            .find(|(identity, _)| identity.id() == "shielded_enemy")
            .expect("the shielded enemy should be spawned as an ECS actor");
        (health.health.current, health.health.max)
    }

    let (expiring, max) =
        victim_hp_after_a_shot_through_a_one_way(ambition_projectiles::WorldHitPolicy::ExpireOnContact);
    assert_eq!(
        expiring, max,
        "an `ExpireOnContact` shot damaged a body through a one-way platform. Its \
         own world-hit policy says a one-way ENDS it, and the world branch below \
         agrees — the victim branch was asking a different question with \
         `include_one_way` hard-coded to false (was {max}, now {expiring})"
    );

    let (bouncing, max) =
        victim_hp_after_a_shot_through_a_one_way(ambition_projectiles::WorldHitPolicy::Bouncing);
    assert!(
        bouncing < max,
        "a `Bouncing` shot must still reach a body behind a one-way — a fireball \
         crosses one from below by design, and its policy does not list one-ways \
         as blockers. Refusing it here would mean the obstruction test reads no \
         policy at all, only a different constant, and the arm above proves nothing"
    );
}

/// ⛔⛔ THE SHOT'S SOLID TEST IS ITS CENTRE LINE, AND A SHOT IS A BOX (D199).
///
/// The anti-tunnelling step casts `raycast_solids` along the leg and snaps
/// `kin.pos` to the hit so the END-position box test finds the block. A ray is
/// not the shot, so a box that clips a block's CORNER along its path — while the
/// centre line passes beside it and the endpoint lands clear — is a hit nothing
/// in the road can see.
///
/// The geometry is chosen so all three probes disagree:
///
/// ```text
/// centre line x=500                 never reaches the block's x∈[510,560]
/// endpoint box  (500,272)±(16,12)   y∈[260,284], clear of the block's y∈[330,350]
/// SWEPT box     (500,400)→(500,272) x∈[510,516] × y∈[330,350] — a corner clip
/// ```
#[test]
fn a_shot_clipping_a_block_corner_along_its_path_hits_it() {
    let world = ae::World::new(
        "corner_clip",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![ae::Block::solid(
            "ledge",
            ae::Vec2::new(510.0, 330.0),
            ae::Vec2::new(50.0, 20.0),
        )],
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    {
        let spec = ProjectileKind::Hadouken.spec(
            ae::Vec2::new(500.0, 400.0),
            ae::Vec2::new(0.0, -1.0),
            1.0,
        );
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(500.0, 400.0);
        // Fast enough that one leg carries the box past the ledge entirely.
        body.kin.vel = ae::Vec2::new(0.0, -8000.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert!(
        bodies.is_empty(),
        "the shot's box swept through the ledge's corner and nothing stopped it: \
         {} still in flight at {:?}",
        bodies.len(),
        bodies.iter().map(|b| b.kin.pos).collect::<Vec<_>>()
    );
}

/// ⛔⛔ AND THE CAST IGNORED THE SHOT'S OWN COLLISION POLICY.
///
/// The anti-tunnelling cast passed `include_one_way = false` unconditionally.
/// That is right for a `Bouncing` shot — a fireball crosses a one-way from below
/// by design — and wrong for `ExpireOnContact`, whose whole contract is that ANY
/// solid / blink-wall / one-way contact is expiry. So a fast straight shot flew
/// through a one-way platform its own policy says should have killed it.
#[test]
fn an_expire_on_contact_shot_does_not_tunnel_through_a_one_way() {
    let world = ae::World::new(
        "one_way_tunnel",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![ae::Block::one_way(
            "platform",
            ae::Vec2::new(400.0, 340.0),
            ae::Vec2::new(300.0, 8.0),
        )],
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    {
        let spec = ProjectileKind::Hadouken.spec(
            ae::Vec2::new(500.0, 300.0),
            ae::Vec2::new(0.0, 1.0),
            1.0,
        );
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(500.0, 300.0);
        body.kin.vel = ae::Vec2::new(0.0, 8000.0);
        // The policy this case is about, stated rather than inherited: no
        // authored kind carries it today, and the held-bolt road that does is a
        // different stepper.
        body.game.world_hit = ambition_projectiles::WorldHitPolicy::ExpireOnContact;
        body.game.gravity = 0.0;
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert!(
        bodies.is_empty(),
        "an ExpireOnContact shot crossed a one-way platform in one leg: {:?}",
        bodies.iter().map(|b| b.kin.pos).collect::<Vec<_>>()
    );
}

/// ⛔ THE CONTRAST, so the fix above cannot become "one-ways stop everything".
///
/// A `Bouncing` shot crossing a one-way HORIZONTALLY must still pass through —
/// the authored behaviour `resolve_world_collision` documents, and the reason
/// the cast excluded one-ways in the first place. Green before the swept
/// conversion and green after; that is the point of it.
#[test]
fn a_bouncing_shot_still_crosses_a_one_way_platform_sideways() {
    let world = ae::World::new(
        "one_way_sideways",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![ae::Block::one_way(
            "platform",
            ae::Vec2::new(400.0, 340.0),
            ae::Vec2::new(300.0, 8.0),
        )],
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    {
        let spec = ProjectileKind::Hadouken.spec(
            ae::Vec2::new(420.0, 344.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(420.0, 344.0);
        body.kin.vel = ae::Vec2::new(1200.0, 0.0);
        body.game.gravity = 0.0;
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(
        bodies.len(),
        1,
        "a bouncing shot flying ALONG a one-way must pass through it"
    );
    assert!(
        bodies[0].kin.pos.x > 420.0,
        "it should still be travelling: {:?}",
        bodies[0].kin.pos
    );
}

/// Two bodies at ONE position, one shot: the same body is struck whichever
/// order the archetype lists them in.
///
/// The victim loop breaks on the first qualifying body, ordered by distance
/// from the leg's start and then by the victim's position. Two bodies on one
/// spawn point tie on all of it, and a stable sort over an equal key is query
/// order — which a rollback resimulation does not reproduce. The final
/// tie-break is the victim's `SimId` (S3, 2026-09-02).
///
/// Spawn order IS archetype order for two identical mobs, so running the same
/// shot against `[a, b]` and `[b, a]` asks the question directly.
fn the_body_a_stacked_shot_strikes(
    spawn_order: [&'static str; 2],
    // ⭐ WHICH MOB IS LEFT WITHOUT AN IDENTITY, and `None` for the original
    // arm where both carry one. The unidentified case cannot be expressed by
    // hand-installing `SimId` on both bodies, which is what this fixture used
    // to do unconditionally — so the arm that matters most had no way to exist.
    unidentified: Option<&'static str>,
) -> String {
    let world = ae::World::new(
        "stacked",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![],
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    app.add_systems(
        Startup,
        move |mut commands: Commands,
              catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>| {
            for id in spawn_order {
                crate::features::spawn_encounter_mob(
                    &mut commands,
                    &catalog,
                    &Default::default(),
                    &crate::character_runtime::fixture_cast(&["fixture_striker"]),
                    ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                    "projectile_test",
                    ambition_encounter::mob_seed::EncounterMobSeed {
                        id: id.into(),
                        character: Some("fixture_striker"),
                        brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                            "fixture_striker".into(),
                        ),
                        pos: ae::Vec2::new(400.0, 300.0),
                        size: ae::Vec2::new(28.0, 46.0),
                    },
                );
            }
        },
    );
    app.update();
    // The identities the construction road would have minted for these
    // placements. `spawn_encounter_mob` is the seam BELOW the one that plans
    // `SimId`s, so the fixture supplies them; the claim under test is the
    // resolver's, given two identified bodies.
    {
        let world = app.world_mut();
        let mobs: Vec<(Entity, String)> = {
            let mut q = world.query::<(Entity, &ActorIdentity)>();
            q.iter(world)
                .map(|(entity, identity)| (entity, identity.id().to_string()))
                .collect()
        };
        for (entity, id) in mobs {
            if unidentified == Some(id.as_str()) {
                continue;
            }
            world
                .entity_mut(entity)
                .insert(ambition_platformer2d_shared_tangle::sim_id::SimId::placement(&id));
        }
    }
    {
        let spec = ProjectileKind::Fireball.spec(
            ae::Vec2::new(360.0, 300.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(360.0, 300.0);
        body.kin.vel = ae::Vec2::new(4000.0, 0.0);
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
        "premise: one shot on two stacked bodies damages exactly one of them (hurt: {hurt:?})"
    );
    hurt.into_iter().next().unwrap()
}

#[test]
fn two_stacked_victims_are_struck_in_identity_order_whatever_the_archetype_order() {
    let forward = the_body_a_stacked_shot_strikes(["stack_a", "stack_b"], None);
    let reversed = the_body_a_stacked_shot_strikes(["stack_b", "stack_a"], None);
    assert_eq!(
        forward, reversed,
        "the struck body followed spawn order: {forward} when a was spawned first, \
         {reversed} when b was — a resimulation does not promise either order"
    );
    assert_eq!(
        forward, "stack_a",
        "the tie-break is the victim's SimId, so the lower placement id is struck"
    );
}

/// ⛔⛔ **AN UNIDENTIFIED VICTIM WINS THE TIE, AND THE FIELD'S OWN DOC SAYS IT
/// CANNOT.**
///
/// `StrikeVictim.sim_id` is `Option<&SimId>` and documents the rule exactly: *"a
/// body without one still gets hit, it just cannot win the tie."* The resolver
/// implements that as `a.sim_id.cmp(&b.sim_id)` — and `Option`'s derived
/// ordering puts `None` FIRST, so an unidentified body beats every identified
/// one at a full geometric tie. The comparison says the opposite of the sentence
/// above it.
///
/// ⭐ AND THE EXISTING STACKED FIXTURE CANNOT SEE IT, BY CONSTRUCTION. It
/// hand-installs `SimId` on BOTH bodies, so every victim it produces is
/// identified and the `None` arm of the comparison is never evaluated. The
/// half of a rule that never runs is the half with no test.
///
/// ⚠ BOTH SPAWN ORDERS, because one order alone cannot tell "the rule picked
/// the identified body" from "the query happened to yield it first" — which is
/// the same reason the sibling test runs both.
#[test]
fn an_unidentified_victim_does_not_beat_an_identified_one() {
    let a_first = the_body_a_stacked_shot_strikes(["stack_a", "stack_b"], Some("stack_b"));
    let b_first = the_body_a_stacked_shot_strikes(["stack_b", "stack_a"], Some("stack_b"));
    assert_eq!(
        a_first, b_first,
        "the struck body followed spawn order: {a_first} when a was spawned \
         first, {b_first} when b was. A resimulation does not promise either"
    );
    assert_eq!(
        a_first, "stack_a",
        "`stack_b` carries no `SimId` and was struck anyway, so an unidentified \
         body WON the tie against an identified one. `Option` orders `None` \
         first, which is the reverse of the rule `StrikeVictim.sim_id` \
         documents: a body without an identity still gets hit, it just cannot \
         win the tie"
    );
}

/// AN OPEN PARRY WINDOW REFLECTS A SHOT — the join the two halves never met at.
///
/// ⭐⭐ THIS IS WHAT MAKES A COUNTER STANCE A REFLECTOR FOR FREE.
/// `step_projectiles` gates its parry on `shield.parrying()`, and its own
/// comment says why: *"The SAME catch the melee strike seam resolves, from the
/// other route a strike arrives on: one fact, both roads."* The smash demo's
/// `smash.counter` technique opens exactly that window, so a fighter standing in
/// a counter reflects an incoming projectile without anybody authoring a
/// reflector.
///
/// ⛔⛔ AND NOTHING CHECKED THE JOIN. The existing `parry_tests` call
/// `reflect_parried_shot` DIRECTLY, so they prove the reflection and never the
/// GATE; the stance's own test asserts the window is open and never fires a shot
/// at it. Each half was covered and the middle was not — so gating this road on
/// `active` as well (the exact mistake `parrying()`'s history records) would
/// silently stop every reflector and redden nothing.
#[test]
fn an_open_parry_window_reflects_a_shot_and_takes_it_over() {
    use ambition_platformer2d_core::BodyShieldState;

    let world = ae::World::new(
        "parry_gate_test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        Vec::new(),
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    app.update();

    // The parrier: a body with nothing but a guard, standing where the shot goes.
    let parrier = app
        .world_mut()
        .spawn((
            ambition_combat::components::ActorFaction::Enemy,
            ae::BodyKinematics {
                pos: ae::Vec2::new(500.0, 300.0),
                size: ae::Vec2::new(28.0, 46.0),
                ..Default::default()
            },
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(500.0, 300.0),
                ae::Vec2::new(28.0, 46.0),
            ),
            BodyHealth::restored(ambition_characters::actor::Health::new(100), 0, default()),
            // ⭐ THE WHOLE FIXTURE: an open parry window and no raised shield,
            // which is exactly the state `smash.counter` puts a fighter in.
            BodyShieldState {
                parry_window_timer: 0.20,
                ..Default::default()
            },
        ))
        .id();

    {
        let spec = ProjectileKind::Fireball.spec(
            ae::Vec2::new(470.0, 300.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(495.0, 300.0);
        body.kin.vel = ae::Vec2::new(60.0, 0.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();

    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(
        bodies.len(),
        1,
        "the parried shot did not survive — a reflection returns it, it does not \
         consume it"
    );
    assert!(
        bodies[0].kin.vel.x < 0.0,
        "the shot kept flying forward past an open parry window ({:?}), so the \
         counter stance does not reflect and the gate is not the shield",
        bodies[0].kin.vel
    );

    let owner = app
        .world_mut()
        .query::<&ambition_projectiles::entity::ProjectileOwner>()
        .iter(app.world())
        .map(|o| o.0)
        .next();
    assert_eq!(
        owner,
        Some(parrier),
        "the reflected shot still belongs to whoever fired it, so it is hostile \
         to the body that just parried it"
    );
}

/// ⛔⛔ AND A CLOSED WINDOW DOES NOT REFLECT — the control the pair above does
/// not contain.
///
/// The reflect arm and the absorb arm control each other on the MODE (does a
/// caught shot come back or vanish), and both stand in front of an OPEN window.
/// ⇒ Neither can see a change that made the gate always true: everything would
/// still reflect, everything would still absorb, and a body with no stance at
/// all would silently start catching shots it should have been hit by. A
/// named peer asked for this arm in exactly those words.
///
/// ⭐ ONE FIELD DIFFERENT FROM THE REFLECT ARM — `parry_window_timer: 0.0` — so
/// the two together say the GATE is what decides, not the fixture.
#[test]
fn a_closed_parry_window_lets_the_shot_through() {
    use ambition_platformer2d_core::BodyShieldState;

    let world = ae::World::new(
        "parry_gate_control",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        Vec::new(),
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    app.update();

    app.world_mut().spawn((
        ambition_combat::components::ActorFaction::Enemy,
        ae::BodyKinematics {
            pos: ae::Vec2::new(500.0, 300.0),
            size: ae::Vec2::new(28.0, 46.0),
            ..Default::default()
        },
        ae::CenteredAabb::from_center_size(
            ae::Vec2::new(500.0, 300.0),
            ae::Vec2::new(28.0, 46.0),
        ),
        BodyHealth::restored(ambition_characters::actor::Health::new(100), 0, default()),
        // The ONE difference from the reflect arm: no window is open.
        BodyShieldState {
            parry_window_timer: 0.0,
            ..Default::default()
        },
    ));

    {
        let spec = ProjectileKind::Fireball.spec(
            ae::Vec2::new(470.0, 300.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(495.0, 300.0);
        body.kin.vel = ae::Vec2::new(60.0, 0.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();

    // ⛔ NOT "the shot is gone" — a struck shot and a reflected one are both
    // absent from a naive count. The claim is that nothing came BACK.
    let reversed = crate::projectile::tests::projectile_bodies(&mut app)
        .into_iter()
        .any(|b| b.kin.vel.x < 0.0);
    assert!(
        !reversed,
        "a body with NO parry window open sent the shot back anyway, so the gate \
         is not the window and every fighter reflects for free"
    );
}

/// AN ABSORBING STANCE EATS THE SHOT instead of returning it.
///
/// ⭐⭐ THE OTHER RESPONSE, through the SAME operation. A counter stance already
/// reflects for free — both roads gate on `parrying()` — so this proves
/// `ProjectileInterception::Consume` reaches the world through
/// `intercept_projectile` rather than through a second implementation.
///
/// ⛔ AND IT IS THE PAIR THAT MATTERS. Asserting "the shot is gone" alone would
/// pass for a stance that simply failed to catch anything: a projectile that
/// flew past and expired is also absent. The reflect arm above is the control —
/// same fixture, same shot, one field different, opposite outcomes.
#[test]
fn an_absorbing_parry_consumes_the_shot_rather_than_returning_it() {
    use ambition_platformer2d_core::BodyShieldState;

    let world = ae::World::new(
        "absorb_gate_test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        Vec::new(),
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    app.update();

    app.world_mut().spawn((
        ambition_combat::components::ActorFaction::Enemy,
        ae::BodyKinematics {
            pos: ae::Vec2::new(500.0, 300.0),
            size: ae::Vec2::new(28.0, 46.0),
            ..Default::default()
        },
        ae::CenteredAabb::from_center_size(
            ae::Vec2::new(500.0, 300.0),
            ae::Vec2::new(28.0, 46.0),
        ),
        BodyHealth::restored(ambition_characters::actor::Health::new(100), 0, default()),
        // Both armed: the window that CATCHES and the mode that decides what
        // catching does. A stance with only the mode absorbs nothing.
        BodyShieldState {
            parry_window_timer: 0.20,
            absorb_window_timer: 0.20,
            ..Default::default()
        },
    ));

    {
        let spec = ProjectileKind::Fireball.spec(
            ae::Vec2::new(470.0, 300.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(495.0, 300.0);
        body.kin.vel = ae::Vec2::new(60.0, 0.0);
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();

    assert!(
        crate::projectile::tests::projectile_bodies(&mut app).is_empty(),
        "an absorbing stance returned the shot instead of eating it — so
         `absorb_window_timer` is not reaching the interception, and every
         absorber is a reflector wearing another name"
    );
}

/// ⛔⛔ AN ABSORBED SHOT STOPS. It does not carry on into the body behind.
///
/// `intercept_projectile(.., Consume)` returns `false` to say the projectile did
/// NOT survive, and `step_projectiles` used to discard that answer and `continue`
/// the victim loop. The despawn is a DEFERRED command, so the entity is still
/// live for the rest of the tick: one shot could be swallowed by the nearest
/// absorber, damage a second body, and -- setting neither `struck` nor
/// `reflected` -- fall through to boss/breakable/world resolution as well.
///
/// ⭐ THE EXISTING ABSORBER TEST CANNOT SEE THIS: it has ONE victim, and
/// "the projectile is absent afterwards" is equally true of a shot that was
/// eaten and of a shot that was eaten and then went on hitting things. The
/// second body is the whole instrument.
///
/// ⚠ RUN IN BOTH SPAWN ORDERS, because the victim sweep is ordered by geometry
/// and a test that only ever spawns the absorber first would pass on a fixture
/// that happened to visit it first for the wrong reason.
#[test]
fn a_shot_swallowed_by_an_absorber_never_reaches_the_body_behind_it() {
    use ambition_platformer2d_core::BodyShieldState;

    for absorber_first in [true, false] {
        let world = ae::World::new(
            "absorb_terminal_test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        );
        let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
        app.update();

        // The absorber is NEARER the shot's origin, so the geometry-ordered
        // sweep reaches it first in both spawn orders.
        let spawn_absorber = |app: &mut App| {
            app.world_mut()
                .spawn((
                    ambition_combat::components::ActorFaction::Enemy,
                    ae::BodyKinematics {
                        pos: ae::Vec2::new(500.0, 300.0),
                        size: ae::Vec2::new(28.0, 46.0),
                        ..Default::default()
                    },
                    ae::CenteredAabb::from_center_size(
                        ae::Vec2::new(500.0, 300.0),
                        ae::Vec2::new(28.0, 46.0),
                    ),
                    BodyHealth::restored(
                        ambition_characters::actor::Health::new(100),
                        0,
                        default(),
                    ),
                    BodyShieldState {
                        parry_window_timer: 0.20,
                        absorb_window_timer: 0.20,
                        ..Default::default()
                    },
                ))
                .id()
        };
        // Directly behind, overlapping the same contact region, with no stance
        // of its own: the body that must NOT be hit.
        let spawn_bystander = |app: &mut App| {
            app.world_mut()
                .spawn((
                    ambition_combat::components::ActorFaction::Enemy,
                    ae::BodyKinematics {
                        pos: ae::Vec2::new(508.0, 300.0),
                        size: ae::Vec2::new(28.0, 46.0),
                        ..Default::default()
                    },
                    ae::CenteredAabb::from_center_size(
                        ae::Vec2::new(508.0, 300.0),
                        ae::Vec2::new(28.0, 46.0),
                    ),
                    BodyHealth::restored(
                        ambition_characters::actor::Health::new(100),
                        0,
                        default(),
                    ),
                ))
                .id()
        };

        let (_absorber, bystander) = if absorber_first {
            let a = spawn_absorber(&mut app);
            (a, spawn_bystander(&mut app))
        } else {
            let b = spawn_bystander(&mut app);
            (spawn_absorber(&mut app), b)
        };

        {
            let spec = ProjectileKind::Fireball.spec(
                ae::Vec2::new(470.0, 300.0),
                ae::Vec2::new(1.0, 0.0),
                1.0,
            );
            let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
            body.kin.pos = ae::Vec2::new(495.0, 300.0);
            body.kin.vel = ae::Vec2::new(60.0, 0.0);
            crate::projectile::tests::spawn_player_projectile(&mut app, body);
        }
        advance_time(&mut app, 0.016);
        app.update();

        let order = if absorber_first { "absorber first" } else { "bystander first" };

        let surviving = app
            .world_mut()
            .query::<&ambition_projectiles::ProjectileGameplay>()
            .iter(app.world())
            .count();
        assert_eq!(
            surviving, 0,
            "({order}) the absorbed shot must be gone after the tick"
        );

        // ⛔ THE OBSERVABLE IS THE EMITTED `HitEvent`, NOT `damage_taken()`.
        // This fixture registers `HitEvent` as a message and installs only
        // `apply_feature_hit_events` — nothing applies a BODY hit to health — so
        // `damage_taken()` is 0 here whatever happens, and asserting on it was a
        // test that could not fail. Found by poisoning: reverting the fix left
        // it green, and a control with the absorber removed showed the lone
        // bystander taking 0 damage too.
        let msgs = app
            .world()
            .resource::<bevy::ecs::message::Messages<ambition_combat::events::HitEvent>>();
        let mut cursor = msgs.get_cursor();
        let hits_on_bystander = cursor
            .read(msgs)
            .filter(|hit| hit.target == ambition_combat::events::HitTarget::Body(bystander))
            .count();
        assert_eq!(
            hits_on_bystander, 0,
            "({order}) the shot was SWALLOWED by the absorber in front and still \
             struck the body behind it — a consumed projectile must terminate its \
             whole collision step, not merely skip one victim"
        );

        // ⭐ AND THE ROAD BELOW THE VICTIM LOOP, which is the half a
        // second-body assertion alone would miss. Setting neither `struck` nor
        // `reflected`, an absorbed shot used to fall through to
        // boss/breakable/world resolution and emit the `UnresolvedFeatures` hit
        // as well — a consequence with no victim at all.
        let mut cursor = msgs.get_cursor();
        let unresolved = cursor
            .read(msgs)
            .filter(|hit| {
                hit.target == ambition_combat::events::HitTarget::UnresolvedFeatures
            })
            .count();
        assert_eq!(
            unresolved, 0,
            "({order}) an absorbed shot went on to feature/world resolution"
        );
    }
}

/// ⛔⛔ **THE TIE WENT TO THE TARGET, AND THE COMMENT CLAIMED THAT WAS THE
/// PROTOCOL.**
///
/// `projectile-contact-protocol.md` says an independent blocking surface at
/// exactly the same impact time WINS over an unrelated hurt target, precisely
/// so a tie cannot grant damage through a wall. The road implemented
/// `wall < contact - f32::EPSILON` — the opposite policy, expressed with the
/// epsilon comparator the same document forbids by name.
///
/// ⭐ ASSERTED ON THE COMPARISON ITSELF, not through a fixture. Two swept code
/// paths (`body_sweep` for the wall, `reached_along` for the victim) producing
/// bit-identical `f32` times is not something a behavioural test can promise,
/// and a tie test that never actually ties proves nothing at all.
#[test]
fn an_independent_wall_wins_an_exact_tie_against_a_hurt_target() {
    use crate::projectile::systems::wall_reaches_first;

    // THE CASE THAT FLIPPED. Equal times: the wall takes it.
    assert!(
        wall_reaches_first(Some(0.5), 0.5),
        "a wall at exactly the target's contact time must win — a tie may not \
         grant damage through it"
    );

    // ⭐ AND THE EPSILON SWALLOWED REAL WALLS TOO. `f32::EPSILON` is ~1.19e-7
    // while the gap between adjacent floats at 0.5 is ~5.96e-8, so a wall a
    // FULL REPRESENTABLE STEP earlier still failed `wall < contact - EPSILON`:
    // the old form called a genuinely earlier wall second.
    //
    // ⚠ `next_down`/`next_up`, NOT `0.5 +/- 1e-9`. Written that way first, and
    // both arms were vacuous: 1e-9 is far below the ulp here, so both constants
    // round to exactly 0.5 and the "strictly later" control was silently
    // asserting the tie case with the answer inverted.
    assert!(
        wall_reaches_first(Some(0.5f32.next_down()), 0.5),
        "a wall that genuinely reaches the leg first was ordered behind the \
         target by the epsilon slack"
    );
    assert!(wall_reaches_first(Some(0.1), 0.5), "a plainly earlier wall is first");

    // ⭐ CONTROLS. A rule that answered `true` always would satisfy every
    // assertion above while making every shot stop at nothing.
    assert!(
        !wall_reaches_first(Some(0.5f32.next_up()), 0.5),
        "a wall reached AFTER the target must not block it"
    );
    assert!(!wall_reaches_first(Some(0.9), 0.5), "a plainly later wall is second");
    assert!(
        !wall_reaches_first(None, 0.5),
        "no wall on the leg cannot block anything"
    );
}

/// ⛔⛔ **THE LANDING SPLASH DETONATED WHERE THE TICK ENDED, NOT WHERE THE SHOT
/// HIT.**
///
/// Both damage roads built their direct `HitEvent` from the swept contact and
/// then called `emit_landing_splash(kin.pos, ..)` — the INTEGRATED ENDPOINT.
/// For a fast shot against a thin target that is far past it. The protocol
/// defines the splash as an area attack at the SELECTED IMPACT LOCATION, and
/// this is gameplay rather than presentation: the area `HitEvent` is centred
/// there, so a wrong centre hits a different set of recipients — possibly on
/// the far side of geometry that lies after the contact — and takes the SFX and
/// VFX with it.
///
/// ⭐ THE WITNESS IS THE VOLUME'S CENTRE, WHICH NOTHING CHECKED. The existing
/// direct-before-splash fixture asserts event ORDER and CARDINALITY, and both
/// are satisfied by a splash centred anywhere in the room.
#[test]
fn the_landing_splash_detonates_at_the_contact_not_at_the_tick_endpoint() {
    use ambition_combat::components::{BreakableFeature, FeatureName};
    use ambition_combat::events::{HitEvent, HitTarget};

    #[derive(bevy::prelude::Resource, Default)]
    struct SplashCentres(Vec<ae::Vec2>);

    fn record(
        mut reader: bevy::prelude::MessageReader<HitEvent>,
        mut seen: bevy::prelude::ResMut<SplashCentres>,
    ) {
        seen.0.extend(
            reader
                .read()
                .filter(|event| matches!(event.target, HitTarget::Volume))
                .map(|event| event.volume.center()),
        );
    }

    const TARGET: ae::Vec2 = ae::Vec2::new(400.0, 300.0);
    let world = ae::World::new(
        "open_lane",
        ae::Vec2::new(4000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        Vec::new(),
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    app.init_resource::<SplashCentres>();
    app.add_systems(Update, record.after(crate::projectile::step_projectiles));
    app.world_mut().spawn((
        ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
        ambition_combat::components::FeatureId::new("crate"),
        FeatureName::new("crate"),
        ambition_platformer2d_shared_tangle::sim_id::SimId::placement("crate"),
        // THIN along the shot's axis, so "contacted early" and "ended far past"
        // are far apart.
        ambition_combat::components::CenteredAabb::from_center_size(
            TARGET,
            ae::Vec2::new(8.0, 46.0),
        ),
        BreakableFeature::new(ambition_interaction::Breakable::new("crate", 1)),
    ));
    let endpoint_x;
    {
        let spec = ProjectileKind::Fireball.spec(
            ae::Vec2::new(360.0, 300.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(360.0, 300.0);
        // FAST: one tick carries it far beyond the crate it strikes.
        body.kin.vel = ae::Vec2::new(10000.0, 0.0);
        body.game.splash_half_extent = 40.0;
        endpoint_x = 360.0 + 10000.0 * 0.016;
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();

    let centres = &app.world().resource::<SplashCentres>().0;
    assert_eq!(
        centres.len(),
        1,
        "expected exactly one landing splash; got {centres:?}"
    );
    let centre = centres[0];
    // ⭐ THE GAMEPLAY PROPERTY, not a coordinate: the area attack a contact
    // caused must cover the thing it was caused BY. Centred on the endpoint
    // (~{endpoint_x}) with a 40 px half-extent it does not come near.
    assert!(
        (centre.x - TARGET.x).abs() <= 40.0,
        "the landing splash was centred at {centre:?}, which does not even reach \
         the crate at {TARGET:?} that caused it. The tick endpoint is x={endpoint_x}; \
         a splash centred there is a DIFFERENT area attack, hitting a different \
         set of recipients"
    );
    assert!(
        centre.x < endpoint_x - 40.0,
        "the landing splash is still riding the integrated endpoint (x={endpoint_x}), \
         not the selected contact: got {centre:?}"
    );
}

/// ⛔⛔ **THE SELECTED TOI WITNESS WAS NOT THE ONE THE PHYSICS USED.**
///
/// The pull-back onto the swept contact was skipped whenever the ENDPOINT
/// overlapped any blocking object, and `resolve_world_collision` then ran its
/// own endpoint-overlap search. So a shot could sweep through wall A, finish
/// inside a later wall B, use A's time of impact to refuse every target, and
/// then physically bounce off B — a different surface and, for a bouncing shot,
/// a different collision NORMAL. Ordering and physics disagreed about what
/// stopped the shot.
#[test]
fn a_shot_that_ends_inside_a_later_wall_still_resolves_against_the_one_it_reached_first() {
    // ⚠ LEDGES AND A FALLING SHOT, not walls and a flat one: a fireball only
    // BOUNCES off a support-face landing (`resolve_solid_hit_in_frame`), and a
    // side hit expires. An expiring shot leaves nothing to read a position off,
    // so the surviving-bounce path is the one that can witness WHICH surface
    // resolved it. Near ledge y 400..408, far ledge y 500..508.
    let world = ae::World::new(
        "two_ledges",
        ae::Vec2::new(4000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![
            ae::Block::solid("near", ae::Vec2::new(0.0, 400.0), ae::Vec2::new(2000.0, 8.0)),
            ae::Block::solid("far", ae::Vec2::new(0.0, 500.0), ae::Vec2::new(2000.0, 8.0)),
        ],
    );
    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    {
        let spec = ProjectileKind::Fireball.spec(
            ae::Vec2::new(500.0, 380.0),
            ae::Vec2::new(0.0, 1.0),
            1.0,
        );
        let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
        body.kin.pos = ae::Vec2::new(500.0, 380.0);
        // 0.016 * 7750 = 124 px: from y=380 to y=504, which is INSIDE `far`.
        body.kin.vel = ae::Vec2::new(0.0, 7750.0);
        assert!(body.game.bounces_remaining > 0, "the shot must be able to bounce");
        crate::projectile::tests::spawn_player_projectile(&mut app, body);
    }
    advance_time(&mut app, 0.016);
    app.update();

    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(bodies.len(), 1, "a bouncing fireball survives its first ledge");
    let pos = bodies[0].kin.pos;
    assert!(
        pos.y < 450.0,
        "the shot resolved against the FAR ledge at y=500 after sweeping through \
         the near one at y=400: it ended at {pos:?}. The near ledge is the contact \
         the ordering road used to refuse targets, so the physics must resolve \
         there too — otherwise the selected witness is not authoritative"
    );
}

/// ⛔⛔ **THE RESOLVER'S OWN KIND PRIORITY OVERRULES THE SELECTED WITNESS.**
///
/// `World::first_body_sweep` orders equal-time contacts with a total,
/// deterministic rule (time of impact, then collider centre, then `GeoId`), and
/// `step_projectiles` refuses every target that the winning contact precedes.
/// Then it throws the winner away — it keeps only `(time_of_impact,
/// block.aabb.center())` — and `resolve_world_collision` answers *"what stopped
/// this shot"* A SECOND TIME, by scanning `world.blocks` for an endpoint
/// overlap with SOLIDS BEFORE ONE-WAYS. That priority is not the sweep's order,
/// so the two roads can name different colliders, and `is_support_landing`
/// reads the named collider's geometry: the shot bounces off one and expires on
/// the other. Ordering and physics disagree about what the shot hit.
///
/// ⭐ THE PULL-BACK IS WHY THIS SURVIVED THE EARLIER FIX. Nudging the shot half
/// a pixel toward the SELECTED block's centre separates it from a tied rival in
/// the obvious corner fixture, so the naive test goes green proving the NUDGE
/// works rather than the witness. Here the nudge cannot separate them: the
/// platform's centre lies toward the wall, so the shot ends up strictly
/// overlapping BOTH, and the disagreement is decided by kind priority rather
/// than by vector order.
///
/// ⚠ THE TIE IS ASSERTED, NOT ASSUMED, AND THE GEOMETRY IS DERIVED FROM THE
/// SHOT'S OWN BOX. A tie fixture that never actually ties proves nothing —
/// written first with hand-placed faces, it missed by 3px because the fireball's
/// half-extent is (12, 9) rather than square, and the wall won outright at
/// t=0.42. The faces are now positioned off `kin.size`, so the two contact times
/// are `50 / d` for the same `d` on both axes and tie by construction.
#[test]
fn a_shot_resolves_against_the_collider_the_sweep_selected_not_the_harder_one() {
    const DT: f32 = 0.016;
    const LEG: f32 = 100.0; // Travel on each axis this tick.
    const REACH: f32 = 50.0; // Distance to both faces => both are reached at t=0.5.
    let start = ae::Vec2::new(600.0, 500.0);

    let spec = ProjectileKind::Fireball.spec(start, ae::Vec2::new(-1.0, 1.0), 1.0);
    let mut body = ambition_projectiles::ProjectileBody::from_spec(spec);
    body.kin.pos = start;
    body.kin.vel = ae::Vec2::new(-LEG / DT, LEG / DT);
    // ⭐ ZEROED SO THE TIE IS EXACT. Gravity accelerates only `y`, which would
    // make the two contact times differ in the last bits and hand the tie to
    // whichever axis rounded lower — a fixture whose subject was float noise.
    body.game.gravity = 0.0;
    body.game.accel = ae::Vec2::ZERO;
    assert!(
        body.game.bounces_remaining > 0,
        "the shot must have a bounce budget: with none, `shot_policy_admits` \
         refuses one-ways outright and there is no tie to decide"
    );

    let half = body.kin.size * 0.5;
    // The wall's RIGHT face and the platform's TOP face, each `REACH` px from
    // the shot's box along its own axis.
    let wall_right = start.x - half.x - REACH;
    let platform_top = start.y + half.y + REACH;

    // ⚠ THE WALL IS TALL AND ITS TOP IS FAR ABOVE THE CONTACT, which is what
    // makes the two colliders answer DIFFERENTLY: the platform's top face is a
    // support landing (bounce), while the wall is met halfway down its side and
    // `is_support_landing` refuses it (expire). A short wall would bounce too
    // and the fixture would witness nothing.
    let wall = ae::Block::solid(
        "wall",
        ae::Vec2::new(wall_right - 60.0, 400.0),
        ae::Vec2::new(60.0, 160.0),
    );
    let platform = ae::Block::one_way(
        "platform",
        ae::Vec2::new(300.0, platform_top),
        ae::Vec2::new(400.0, 8.0),
    );
    use ambition_platformer2d_core::AabbExt as _;
    assert!(
        platform.aabb.center().x < wall.aabb.center().x,
        "the sweep breaks an exact tie on centre-x, so the platform must sit \
         left of the wall for the SWEEP to select it"
    );
    let world = ae::World::new(
        "tied_wall_and_platform",
        ae::Vec2::new(4000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![wall, platform],
    );

    // ⭐ THE SWEEP'S ANSWER, ASKED DIRECTLY. This is the half the ordering road
    // uses to refuse targets; the assertions below are about whether the
    // PHYSICS agrees with it.
    let box_at_leg_start = ae::Aabb::new(start, half);
    let selected = ae::cast::body_sweep(
        &world,
        box_at_leg_start,
        body.kin.vel * DT,
        |block: &ae::Block| {
            ambition_projectiles::block_obstructs_shot(
                body.game.world_hit,
                body.game.bounces_remaining,
                body.kin.vel,
                box_at_leg_start,
                ae::Vec2::new(0.0, 1.0),
                block,
            )
        },
    )
    .expect("the shot reaches both colliders within the tick");
    assert_eq!(
        selected.block.name, "platform",
        "the fixture only witnesses the disagreement if the SWEEP picks the \
         one-way: it ties with the wall and wins on centre-x. It picked {:?} at \
         t={} instead, so the geometry drifted and this test is measuring \
         something else",
        selected.block.name, selected.time_of_impact
    );

    let mut app = projectile_test_app(world, ae::Vec2::new(200.0, 200.0), 1.0);
    crate::projectile::tests::spawn_player_projectile(&mut app, body);
    advance_time(&mut app, DT);
    app.update();

    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(
        bodies.len(),
        1,
        "the sweep selected the ONE-WAY PLATFORM, and a bouncing fireball with \
         budget left survives a support-face landing on one. The shot is gone, \
         so the physics resolved against the WALL instead — whose top face is \
         far above the contact, making this a side hit that expires. The \
         ordering road refused targets on the platform's time of impact and the \
         physics answered with a different collider"
    );
    let after = bodies[0].kin;
    assert!(
        after.vel.y < 0.0,
        "a support-face landing reverses local-down velocity; the surviving \
         shot is still travelling downward at {:?}, so it did not bounce off \
         the platform the sweep selected",
        after.vel
    );
}
