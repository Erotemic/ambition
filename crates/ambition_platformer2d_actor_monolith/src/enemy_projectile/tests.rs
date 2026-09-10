use ambition_combat::events::{HitEvent, HitSource};
use ambition_platformer2d_core as ae;
use ambition_vfx::vfx::VfxMessage;
use bevy::prelude::*;

use crate::enemy_projectile::test_support::{live_projectile_bodies, spawn_test_projectile};
use ambition_combat::components::ActorFaction;
use ambition_projectiles::{
    build_in_flight_projectile, ProjectileSeqCounter, ProjectileSpawn, ProjectileSpawnRequest,
    ProjectileStart,
};

#[derive(Resource, Default)]
struct CapturedHits(Vec<HitEvent>);

fn capture_hits(mut reader: MessageReader<HitEvent>, mut cap: ResMut<CapturedHits>) {
    for e in reader.read() {
        cap.0.push(e.clone());
    }
}

fn insert_projectile_authority(app: &mut App) {
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    // The stepper resolves each shot's visual id through the (empty here) content
    // catalog for its detonation-FX pick; init it so the `Res` param validates.
    app.init_resource::<ambition_projectiles::ProjectileVisualCatalog>();
}

/// The faction-aware routing keystone: a Player-faction shot in the
/// single live-projectile road damages the enemy it overlaps and expires on
/// contact — the substrate for the
/// wielded ranged boss attack (`ambition_abilities::ranged::volley`). The enemy-faction path is
/// unchanged (covered by the existing boss-special consumer tests).
#[test]
fn player_faction_shot_damages_an_overlapping_enemy_and_expires() {
    let mut app = App::new();
    insert_projectile_authority(&mut app);
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "phys",
            ae::Vec2::new(800.0, 800.0),
            ae::Vec2::new(400.0, 400.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 780.0),
                ae::Vec2::new(800.0, 20.0),
            )],
        )),
    );
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    app.add_message::<HitEvent>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<crate::avatar::PlayerHealRequested>();
    app.init_resource::<ProjectileSeqCounter>();
    app.init_resource::<CapturedHits>();
    app.init_resource::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>();
    app.init_resource::<ambition_gameplay_trace::GameplayTraceBuffer>();
    app.add_systems(
        Update,
        (crate::projectile::step_projectiles, capture_hits).chain(),
    );

    let enemy_pos = ae::Vec2::new(300.0, 100.0);
    let enemy = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("test_enemy"),
            ambition_combat::components::CenteredAabb::new(enemy_pos, ae::Vec2::new(16.0, 24.0)),
            ambition_combat::components::ActorDisposition::Hostile,
            //  a body with no faction is not something production builds. The
            // fixture had none, because the branch it was written against broadcast
            // a volume and never asked whose side anyone was on. `damage_lands` is
            // the routing rule for every shot now, so an unfactioned body is not a
            // hard target — it is a body the victim query cannot even see.
            ambition_combat::components::ActorFaction::Enemy,
            ambition_characters::actor::BodyCombat {
                hit_flash: 0.0,
                training_dummy: false,
                ..Default::default()
            },
        ))
        .id();
    // A player-faction shot already overlapping the enemy.
    spawn_test_projectile(
        &mut app,
        ProjectileSpawn {
            origin: enemy_pos,
            dir: ae::Vec2::new(1.0, 0.0),
            speed: 200.0,
            damage: 3,
            max_lifetime: 2.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            gravity: 0.0,
            visual_id: String::new(),
            // Straight shot: this ability authors no bounce.
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 0.0,
            boomerang_return_s: None,
        },
        ActorFaction::Player,
    );

    app.update();

    let cap = app.world().resource::<CapturedHits>();
    //  this pair used to assert a claim and its negation — `PlayerProjectile`
    // present, `EnemyProjectile` absent — and the fold into one `Projectile`
    // cause turned that into a contradiction, which is the honest signal that
    // the SOURCE was never what the test cared about.
    //
    // What it actually claims is about reach: the shot hits the enemy and does
    // NOT reach the player. That is a statement about the victim, so it is
    // asserted on the victim.
    let projectile_hits: Vec<_> = cap
        .0
        .iter()
        .filter(|e| matches!(e.source, HitSource::Projectile))
        .collect();
    assert!(
        !projectile_hits.is_empty(),
        "the player-faction shot lands a projectile hit on the enemy"
    );
    // It has: there is one victim loop now, whoever fired, so a player's bolt identifies its victim
    // exactly as an enemy's always did.
    assert!(
        projectile_hits
            .iter()
            .all(|e| e.target == ambition_combat::events::HitTarget::Body(enemy)),
        "the shot must NAME the body it struck, got {:?}",
        projectile_hits
            .iter()
            .map(|e| &e.target)
            .collect::<Vec<_>>()
    );
    assert!(
        app.world()
            .get::<ambition_characters::actor::BodyHealth>(enemy)
            .is_none_or(|health| health.health.current < health.health.max),
        "and the enemy is what it reached"
    );
    assert!(
        live_projectile_bodies(&mut app).is_empty(),
        "the shot expires on contact with the enemy"
    );
}

/// An OWNERLESS shot (orphaned firer, or a truly ownerless volley) is
/// INDISCRIMINATE — it hurts every body it overlaps, even one a faction-owned
/// shot would spare. Pins it against an Enemy actor: an Enemy-OWNED shot would
/// pass a fellow Enemy by (`can_damage(Enemy, Enemy) == false`), but an
/// ownerless one has no ally to spare, so it lands.
#[test]
fn an_ownerless_shot_damages_a_same_faction_actor_indiscriminately() {
    use crate::enemy_projectile::test_support::spawn_ownerless_projectile;
    let mut app = App::new();
    insert_projectile_authority(&mut app);
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "phys",
            ae::Vec2::new(800.0, 800.0),
            ae::Vec2::new(400.0, 400.0),
            vec![],
        )),
    );
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    app.add_message::<HitEvent>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<crate::avatar::PlayerHealRequested>();
    app.init_resource::<ProjectileSeqCounter>();
    app.init_resource::<CapturedHits>();
    app.init_resource::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>();
    app.init_resource::<ambition_gameplay_trace::GameplayTraceBuffer>();
    app.add_systems(
        Update,
        (crate::projectile::step_projectiles, capture_hits).chain(),
    );

    let actor_pos = ae::Vec2::new(300.0, 100.0);
    let enemy = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("enemy_bystander"),
            ambition_combat::components::CenteredAabb::new(actor_pos, ae::Vec2::new(16.0, 24.0)),
            ambition_combat::components::ActorFaction::Enemy,
        ))
        .id();
    // An OWNERLESS shot already overlapping the Enemy actor.
    spawn_ownerless_projectile(
        &mut app,
        ProjectileSpawn {
            origin: actor_pos,
            dir: ae::Vec2::new(1.0, 0.0),
            speed: 200.0,
            damage: 3,
            max_lifetime: 2.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            gravity: 0.0,
            visual_id: String::new(),
            // Straight shot: this ability authors no bounce.
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 0.0,
            boomerang_return_s: None,
        },
    );

    app.update();

    let cap = app.world().resource::<CapturedHits>();
    assert!(
        cap.0
            .iter()
            .any(|e| matches!(e.target, ambition_combat::events::HitTarget::Body(a) if a == enemy)),
        "an ownerless shot hits the Enemy actor a faction-owned Enemy shot would spare"
    );
}

// ── S3e: relational actor-vs-actor projectiles ──────────────────────────

/// Build a headless app wired for `step_projectiles` with the given relations.
fn arena_projectile_app(relations: ambition_combat::targeting::FactionRelations) -> App {
    let mut app = App::new();
    insert_projectile_authority(&mut app);
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "phys",
            ae::Vec2::new(800.0, 800.0),
            ae::Vec2::new(400.0, 400.0),
            vec![],
        )),
    );
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    app.add_message::<HitEvent>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<crate::avatar::PlayerHealRequested>();
    app.init_resource::<ProjectileSeqCounter>();
    app.init_resource::<CapturedHits>();
    app.init_resource::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>();
    app.init_resource::<ambition_gameplay_trace::GameplayTraceBuffer>();
    app.insert_resource(relations);
    app.add_systems(
        Update,
        (crate::projectile::step_projectiles, capture_hits).chain(),
    );
    app
}

fn spawn_boss_actor(app: &mut App, pos: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("arena_robot"),
            ambition_combat::components::CenteredAabb::new(pos, ae::Vec2::new(16.0, 24.0)),
            ambition_combat::components::ActorFaction::Boss,
        ))
        .id()
}

fn spawn_overlapping_enemy_glider(app: &mut App, pos: ae::Vec2) {
    spawn_test_projectile(
        app,
        ProjectileSpawn {
            origin: pos,
            dir: ae::Vec2::new(1.0, 0.0),
            speed: 200.0,
            damage: 3,
            max_lifetime: 2.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            gravity: 0.0,
            visual_id: String::new(),
            // Straight shot: this ability authors no bounce.
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 0.0,
            boomerang_return_s: None,
        },
        ActorFaction::Enemy,
    );
}

/// An Enemy-faction shot (the PCA's glider) damages a Boss-faction body when
/// the relations matrix marks them hostile — the projectile half of the
/// non-player-centric arena. Pre-resolved to that exact actor.
#[test]
fn enemy_glider_damages_a_relationally_hostile_actor() {
    let mut relations = ambition_combat::targeting::FactionRelations::default();
    relations.set_mutual_hostile(
        ambition_combat::components::ActorFaction::Enemy,
        ambition_combat::components::ActorFaction::Boss,
        true,
    );
    let mut app = arena_projectile_app(relations);
    let pos = ae::Vec2::new(300.0, 100.0);
    let boss_actor = spawn_boss_actor(&mut app, pos);
    spawn_overlapping_enemy_glider(&mut app, pos);
    app.update();
    let cap = app.world().resource::<CapturedHits>();
    assert!(
        cap.0
            .iter()
            .any(|e| matches!(e.source, HitSource::Projectile)
                && e.target == ambition_combat::events::HitTarget::Body(boss_actor)),
        "the enemy glider lands a pre-resolved hit on the hostile Boss actor"
    );
}

/// One enemy glider at `pos`, carrying `fired_by_move` and the given damage so
/// its hit is identifiable among several.
fn spawn_stamped_enemy_glider(
    app: &mut App,
    pos: ae::Vec2,
    damage: i32,
    fired_by_move: Option<u32>,
) {
    spawn_stamped_enemy_glider_with_splash(app, pos, damage, fired_by_move, 0.0);
}

/// The same shot with a landing area, so the splash road is reachable.
fn spawn_stamped_enemy_glider_with_splash(
    app: &mut App,
    pos: ae::Vec2,
    damage: i32,
    fired_by_move: Option<u32>,
    splash_half_extent: f32,
) {
    crate::enemy_projectile::test_support::spawn_test_projectile_fired_by(
        app,
        ProjectileSpawn {
            origin: pos,
            dir: ae::Vec2::new(1.0, 0.0),
            speed: 200.0,
            damage,
            max_lifetime: 2.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            gravity: 0.0,
            visual_id: String::new(),
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent,
            boomerang_return_s: None,
        },
        ActorFaction::Enemy,
        fired_by_move,
    );
}

/// ⛔⛤ THE SEAM NOTHING NAMED. `FiredByMoveInstance` ON THE SHOT BECOMES
/// `attacker_move_instance` ON THE HIT.
///
/// ⛔⛔ MEASURED 2026-09-10: before this test that component was written by ONE
/// line (`ambition_projectiles::materialize`), read by ONE line (this file's
/// `step_projectiles`), and NAMED BY NO TEST IN THE TREE. It is not
/// lightly-covered, it is uncovered — and it carries the A12 provenance for BOTH
/// projectile roads, the moveset `Ranged` branch and the boss
/// `Effect -> Special -> technique` branch. A silent break here surfaces as
/// *"cancels credit the wrong move"*, with nothing between the cause and the
/// symptom.
///
/// ⭐⭐ TWO SHOTS WITH DIFFERENT STAMPS, IN ONE WORLD, ON THE SAME TICK. One
/// shot cannot tell a copy from a constant: any hard-coded value passes a
/// single-shot test whose fixture happens to have chosen it. Two force the read
/// to be per-shot, which is the property the damage road actually needs — a
/// fighter has several shots in the air and they belong to different uses.
///
/// ⚠ The shots are told apart by DAMAGE, not by hit order. Hit order is
/// iteration order over a query.
#[test]
fn each_shot_hands_its_own_move_use_to_the_hit_it_lands() {
    let mut app = arena_projectile_app(ambition_combat::targeting::FactionRelations::default());
    let left = ae::Vec2::new(300.0, 100.0);
    let right = ae::Vec2::new(500.0, 100.0);
    spawn_boss_actor(&mut app, left);
    spawn_boss_actor(&mut app, right);
    // Damage 3 was fired by use 11; damage 5 by use 4. Neither number is the
    // other's, and neither is a plausible default.
    spawn_stamped_enemy_glider(&mut app, left, 3, Some(11));
    spawn_stamped_enemy_glider(&mut app, right, 5, Some(4));
    app.update();

    let cap = app.world().resource::<CapturedHits>();
    let landed: Vec<(i32, Option<u32>)> = cap
        .0
        .iter()
        .filter(|e| matches!(e.source, HitSource::Projectile))
        .map(|e| (e.damage, e.attacker_move_instance))
        .collect();
    assert_eq!(
        landed.len(),
        2,
        "the premise: both shots landed, so the stamps below are a claim about \
         two hits that exist -- got {landed:?}"
    );
    assert!(
        landed.contains(&(3, Some(11))),
        "the shot fired by use 11 handed 11 to its hit; got {landed:?}"
    );
    assert!(
        landed.contains(&(5, Some(4))),
        "the shot fired by use 4 handed 4 to its hit; got {landed:?}"
    );
}

/// ⛔⛤ AND THE LANDING AREA IS A SECOND SPEND SITE, NOT THE SAME ONE.
///
/// ⛔⛔ MEASURED 2026-09-10 by poisoning each spend site in turn against the
/// WHOLE 1126-test crate suite: `step_projectiles` hands `fired_by_move` to
/// FIVE places, and before these tests the suite reached NONE of them. Dropping
/// the value at four of the five left every test in the crate green.
///
/// ⇒ A splash is a separate `HitEvent` with a separate field to fill, written
/// by a separate call. It damages whatever is standing in the blast, which can
/// be a body the shot never touched — so it needs the provenance for exactly
/// the same reason the direct hit does, and it can lose it independently.
///
/// ⚠ Told apart from the direct hit by `HitTarget::Volume`, which is what a
/// splash is: an area, not a named recipient.
#[test]
fn the_splash_of_a_landed_shot_names_the_move_use_that_fired_it() {
    let mut app = arena_projectile_app(ambition_combat::targeting::FactionRelations::default());
    let pos = ae::Vec2::new(300.0, 100.0);
    spawn_boss_actor(&mut app, pos);
    spawn_stamped_enemy_glider_with_splash(&mut app, pos, 3, Some(11), 12.0);
    app.update();

    let cap = app.world().resource::<CapturedHits>();
    let splashes: Vec<Option<u32>> = cap
        .0
        .iter()
        .filter(|e| {
            matches!(e.source, HitSource::Projectile)
                && e.target == ambition_combat::events::HitTarget::Volume
        })
        .map(|e| e.attacker_move_instance)
        .collect();
    assert_eq!(
        splashes.len(),
        1,
        "the premise: the shot landed AND its splash was emitted, so the stamp \
         below is a claim about an event that exists"
    );
    assert_eq!(
        splashes[0],
        Some(11),
        "the splash names the same use as the shot that made it"
    );
}

/// ⛔⛤ AND A SHOT THAT HITS NOTHING BUT A WALL IS A THIRD SPEND SITE.
///
/// A bolt that spends itself on world geometry still emits its landing area, and
/// that area still damages whatever is standing in it — so it still owes the
/// question *"which use of which move made this?"*. It is written by a THIRD
/// `emit_landing_splash` call on a branch neither of the other two reaches:
/// `WorldHitOutcome::Expired`, which needs a wall in the room.
///
/// ⚠ THE ROOM THE SHARED FIXTURE BUILDS HAS NO BLOCKS, so this branch was
/// unreachable from every test in the crate. That is the whole reason it went
/// unmeasured: not that anyone judged it unimportant, but that the fixture
/// everyone reused could not get there.
#[test]
fn a_shot_that_expires_on_a_wall_still_names_the_move_use_that_fired_it() {
    let mut app = arena_projectile_app(ambition_combat::targeting::FactionRelations::default());
    // A wall directly in the bolt's path. `insert_session_world_component`
    // REPLACES the fixture's empty room.
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "phys",
            ae::Vec2::new(800.0, 800.0),
            ae::Vec2::new(400.0, 400.0),
            vec![ae::Block::solid(
                "wall",
                ae::Vec2::new(320.0, 60.0),
                ae::Vec2::new(40.0, 120.0),
            )],
        )),
    );
    crate::enemy_projectile::test_support::spawn_test_projectile_fired_by(
        &mut app,
        ProjectileSpawn {
            origin: ae::Vec2::new(300.0, 100.0),
            dir: ae::Vec2::new(1.0, 0.0),
            // Fast enough to reach the wall inside one 1/60s tick.
            speed: 3000.0,
            damage: 3,
            max_lifetime: 2.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            gravity: 0.0,
            visual_id: String::new(),
            // No bounces left, so world contact EXPIRES the shot rather than
            // reflecting it — that is the branch under test.
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 12.0,
            boomerang_return_s: None,
        },
        ActorFaction::Enemy,
        Some(11),
    );
    app.update();

    let cap = app.world().resource::<CapturedHits>();
    let splashes: Vec<Option<u32>> = cap
        .0
        .iter()
        .filter(|e| {
            matches!(e.source, HitSource::Projectile)
                && e.target == ambition_combat::events::HitTarget::Volume
        })
        .map(|e| e.attacker_move_instance)
        .collect();
    assert_eq!(
        splashes.len(),
        1,
        "the premise: the bolt reached the wall and expired there, emitting one \
         landing area -- without the wall this branch never runs and the \
         assertion below would be vacuous"
    );
    assert_eq!(splashes[0], Some(11));
}

/// ⛔⛤ AND A BREAKABLE IS A FOURTH AND FIFTH SPEND SITE. A shot that reaches a
/// crate takes a DIFFERENT branch of `step_projectiles` from a shot that reaches
/// a body, and that branch writes its own `HitEvent` and its own splash.
///
/// ⚠ Both are hit here because a hole in either is the same defect: a breakable
/// broken by move A's bolt, credited to move B.
#[test]
fn a_shot_that_reaches_a_breakable_names_the_move_use_that_fired_it() {
    let mut app = arena_projectile_app(ambition_combat::targeting::FactionRelations::default());
    let pos = ae::Vec2::new(300.0, 100.0);
    app.world_mut().spawn((
        ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
        ambition_combat::components::FeatureId::new("crate_a"),
        ambition_combat::components::CenteredAabb::new(pos, ae::Vec2::new(16.0, 16.0)),
        ambition_combat::components::BreakableFeature::new(
            ambition_interaction::Breakable::new("crate_a", 10),
        ),
    ));
    spawn_stamped_enemy_glider_with_splash(&mut app, pos, 3, Some(11), 12.0);
    app.update();

    let cap = app.world().resource::<CapturedHits>();
    let shots: Vec<(String, Option<u32>)> = cap
        .0
        .iter()
        .filter(|e| matches!(e.source, HitSource::Projectile))
        .map(|e| (format!("{:?}", e.target), e.attacker_move_instance))
        .collect();
    assert!(
        shots.iter().any(|(target, _)| target.starts_with("Feature")),
        "the premise: the shot took the FEATURE branch, not the body branch -- \
         got {shots:?}"
    );
    assert!(
        shots.iter().any(|(target, _)| target == "Volume"),
        "the premise: the feature branch emitted its landing area too -- got \
         {shots:?}"
    );
    assert!(
        shots.iter().all(|(_, instance)| *instance == Some(11)),
        "every event this shot produced names the use that fired it; got {shots:?}"
    );
}

/// ⭐ AND AN UNSTAMPED SHOT HANDS OVER NOTHING. A gun, a thrown bomb and an
/// environmental volley are fired by no move, and `None` is the answer for each
/// of them — not a hole to be filled by whatever the owner happens to be
/// playing when the shot lands.
///
/// ⚠ This arm is what fails if the read is ever replaced by a lookup with a
/// default. It is the only arm a constant-stamping defect cannot satisfy.
#[test]
fn a_shot_no_move_fired_hands_over_no_move_use() {
    let mut app = arena_projectile_app(ambition_combat::targeting::FactionRelations::default());
    let pos = ae::Vec2::new(300.0, 100.0);
    spawn_boss_actor(&mut app, pos);
    spawn_stamped_enemy_glider(&mut app, pos, 3, None);
    app.update();

    let cap = app.world().resource::<CapturedHits>();
    let landed: Vec<Option<u32>> = cap
        .0
        .iter()
        .filter(|e| matches!(e.source, HitSource::Projectile))
        .map(|e| e.attacker_move_instance)
        .collect();
    assert_eq!(landed.len(), 1, "the premise: the shot landed");
    assert_eq!(
        landed[0], None,
        "a shot no move fired names no move use; got {landed:?}"
    );
}

/// Damage is PHYSICAL, not relational: with default relations (no targeting
/// hostility set), an Enemy glider STILL damages a DIFFERENT-faction (Boss)
/// actor it overlaps. Targeting is the relational concern; a shot that LANDS
/// hurts any non-ally. (Friendly fire is off by default, so a same-faction
/// body would be spared.)
#[test]
fn enemy_glider_damages_a_different_faction_actor_physically() {
    let mut app = arena_projectile_app(ambition_combat::targeting::FactionRelations::default());
    let pos = ae::Vec2::new(300.0, 100.0);
    let boss_actor = spawn_boss_actor(&mut app, pos);
    spawn_overlapping_enemy_glider(&mut app, pos);
    app.update();
    let cap = app.world().resource::<CapturedHits>();
    assert!(
        cap.0
            .iter()
            .any(|e| matches!(e.source, HitSource::Projectile)
                && e.target == ambition_combat::events::HitTarget::Body(boss_actor)),
        "a different-faction actor is hit regardless of relations (physical damage)"
    );
}

/// Melee asks `targeting::team_allows_damage` and lands. This loop asked the faction-only
/// `damage_lands` and spared every shot as an ally, so NO projectile from ANY fighter could hit
/// anybody on a crossover grid.
///
///  `StrikeVictim` has carried the victim's `team` the whole time — its own doc says
/// *"Outranks faction for 'may this land'"* — and this loop was the one caller that never asked
/// for it.
#[test]
fn a_seated_fighters_shot_hits_a_same_faction_body_on_another_team() {
    use ambition_combat::targeting::MatchTeam;

    let mut app = arena_projectile_app(ambition_combat::targeting::FactionRelations::default());
    let pos = ae::Vec2::new(300.0, 100.0);

    // The firer and the victim: one faction, two teams — a match.
    let firer = app
        .world_mut()
        .spawn((
            ambition_combat::components::ActorFaction::Player,
            MatchTeam::new("seat 1"),
        ))
        .id();
    let victim = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("seat_two_fighter"),
            ambition_combat::components::CenteredAabb::new(pos, ae::Vec2::new(16.0, 24.0)),
            ambition_combat::components::ActorFaction::Player,
            MatchTeam::new("seat 2"),
            // ⚠ IDENTIFIED, BECAUSE THE TWO BODIES BELOW SIT ON ONE POINT. The
            // resolver's `debug_assert` caught this fixture: two coincident
            // victims with no `SimId` between them are ordered by Bevy query
            // order, and a rollback resimulation does not reproduce it. The
            // outcome here happens not to depend on the order — the teammate is
            // filtered by the team rule — but a fixture that leaves the resolver
            // undecided is one edit away from asserting on query order.
            ambition_platformer2d_shared_tangle::sim_id::SimId::placement("seat_two_fighter"),
        ))
        .id();
    //  THE POISON, in the fixture: a body on the FIRER'S OWN team, overlapping
    // the same shot. Without it a predicate that simply stopped consulting
    // anything would pass this test.
    let teammate = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("same_team_fighter"),
            ambition_combat::components::CenteredAabb::new(pos, ae::Vec2::new(16.0, 24.0)),
            ambition_combat::components::ActorFaction::Player,
            MatchTeam::new("seat 1"),
            ambition_platformer2d_shared_tangle::sim_id::SimId::placement("same_team_fighter"),
        ))
        .id();

    spawn_owned_glider(&mut app, pos, firer);
    app.update();

    let cap = app.world().resource::<CapturedHits>();
    let hit = |who: Entity| {
        cap.0.iter().any(|e| {
            matches!(e.source, HitSource::Projectile)
                && e.target == ambition_combat::events::HitTarget::Body(who)
        })
    };
    assert!(
        hit(victim),
        "a fighter's shot passed through an opponent it shares a faction with — \
         the projectile loop is deciding on factions and cannot see the match"
    );
    assert!(
        !hit(teammate),
        "the shot also hit the firer's own team, so it is not consulting teams — \
         it has stopped consulting anything"
    );
}

/// A SHOT IN FLIGHT DOES NOT CHANGE SIDES WHEN ITS FIRER DIES.
///
/// The four-fighter case the queue names: a fighter fires, loses their final
/// stock, and the ruleset takes the body out of play —
/// `ambition_demo_smash::take_eliminated_fighters_out_of_play` DESPAWNS it, and
/// says why in as many words. The bolt is still in the air.
///
/// It turned on its own team.
///
///  the presentation half of this shot already knew better.
/// `inherit_projectile_presentation_sources` says it outright: *"the bolt is the
/// emitter … it routinely outlives the body that fired it. So the source is
/// STAMPED at spawn rather than looked up at impact."* The combat half was the
/// one that kept asking who was still standing.
///
/// Two shots, two victims, so neither assertion depends on victim iteration
/// order: an indiscriminate shot despawns on the FIRST body it strikes, so a
/// single shot overlapping both bodies would spare the teammate half the time.
#[test]
fn a_shot_outlives_its_firer_without_changing_sides() {
    use ambition_combat::targeting::MatchTeam;

    let mut app = arena_projectile_app(ambition_combat::targeting::FactionRelations::default());

    let firer = app
        .world_mut()
        .spawn((
            ambition_combat::components::ActorFaction::Player,
            MatchTeam::new("seat 1"),
        ))
        .id();
    let teammate_pos = ae::Vec2::new(300.0, 100.0);
    let opponent_pos = ae::Vec2::new(300.0, 300.0);
    let teammate = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("same_team_fighter"),
            ambition_combat::components::CenteredAabb::new(teammate_pos, ae::Vec2::new(16.0, 24.0)),
            ambition_combat::components::ActorFaction::Player,
            MatchTeam::new("seat 1"),
        ))
        .id();
    let opponent = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("seat_two_fighter"),
            ambition_combat::components::CenteredAabb::new(opponent_pos, ae::Vec2::new(16.0, 24.0)),
            ambition_combat::components::ActorFaction::Player,
            MatchTeam::new("seat 2"),
        ))
        .id();

    // Both bolts start well short of their target, so they are genuinely IN
    // FLIGHT — the firer is alive for the first step and dead for the rest.
    spawn_owned_glider(&mut app, teammate_pos - ae::Vec2::new(150.0, 0.0), firer);
    spawn_owned_glider(&mut app, opponent_pos - ae::Vec2::new(150.0, 0.0), firer);
    app.update();
    assert!(
        app.world().resource::<CapturedHits>().0.is_empty(),
        "the fixture is wrong: something was already in range on the firer's last tick"
    );

    // The final stock is spent; the ruleset takes the body out of play.
    app.world_mut().despawn(firer);

    //  `app.update()` is not a tick of sim time — loop on the property (every
    // bolt resolved) with a ceiling. 150 px at 200 px/s is ~45 ticks; the
    // 2 s lifetime retires an unspent bolt at 120.
    let mut live_projectiles = app
        .world_mut()
        .query_filtered::<Entity, With<ambition_projectiles::LiveProjectile>>();
    for _ in 0..240 {
        if live_projectiles.iter(app.world()).next().is_none() {
            break;
        }
        app.update();
    }

    let cap = app.world().resource::<CapturedHits>();
    let hit = |who: Entity| {
        cap.0.iter().any(|e| {
            matches!(e.source, HitSource::Projectile)
                && e.target == ambition_combat::events::HitTarget::Body(who)
        })
    };
    assert!(
        !hit(teammate),
        "the orphaned shot turned on its firer's own team — allegiance evaporated \
         with the body instead of being carried by the bolt"
    );
    assert!(
        hit(opponent),
        "the orphaned shot hit nobody at all, which is the other way to get this \
         wrong: it is still that fighter's attack, aimed at that fighter's foes"
    );
}

/// A SHOT ORPHANED BEFORE ITS FIRST STEP DOES NOT BECOME A HAZARD.
///
/// The sibling of [`a_shot_outlives_its_firer_without_changing_sides`], for the one window that
/// test cannot reach. A fighter who fires and is eliminated inside that tick leaves a shot with
/// a named owner, no stamp, and no way to take one — the owner query wants a non-optional
/// `&ActorFaction` and the body is gone.
///
/// This test is the difference between those two sentences.
///
///  it asserts SAFETY only, deliberately. The shot currently hits nobody,
/// which is the safe direction but not the right answer — the right answer is
/// that attribution is stamped where the entity is BORN, the conclusion
/// `inherit_projectile_presentation_sources` already reached for the
/// presentation half. Asserting "hits the opponent" here would pin the
/// limitation; asserting "hits nobody" would go red the day someone fixes it
/// properly. So this pins only the part that must never regress.
#[test]
fn a_shot_orphaned_before_its_first_step_does_not_turn_on_its_team() {
    use ambition_combat::targeting::MatchTeam;

    let mut app = arena_projectile_app(ambition_combat::targeting::FactionRelations::default());

    let firer = app
        .world_mut()
        .spawn((
            ambition_combat::components::ActorFaction::Player,
            MatchTeam::new("seat 1"),
        ))
        .id();
    let teammate_pos = ae::Vec2::new(300.0, 100.0);
    let teammate = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("same_team_fighter"),
            ambition_combat::components::CenteredAabb::new(teammate_pos, ae::Vec2::new(16.0, 24.0)),
            ambition_combat::components::ActorFaction::Player,
            MatchTeam::new("seat 1"),
        ))
        .id();

    // In flight toward the teammate, and the firer is taken out of play BEFORE any tick runs —
    // so the bolt is stepped for the first time with its owner already gone.
    spawn_owned_glider(&mut app, teammate_pos - ae::Vec2::new(150.0, 0.0), firer);
    app.world_mut().despawn(firer);

    let mut live_projectiles = app
        .world_mut()
        .query_filtered::<Entity, With<ambition_projectiles::LiveProjectile>>();
    for _ in 0..240 {
        if live_projectiles.iter(app.world()).next().is_none() {
            break;
        }
        app.update();
    }

    let cap = app.world().resource::<CapturedHits>();
    let hit_teammate = cap.0.iter().any(|e| {
        matches!(e.source, HitSource::Projectile)
            && e.target == ambition_combat::events::HitTarget::Body(teammate)
    });
    assert!(
        !hit_teammate,
        "a shot orphaned before its stamp was taken hit its firer's own teammate \
         — `indiscriminate` read a failed owner LOOKUP as 'this bolt never had an \
         owner', so a named firer going missing promoted the shot to environmental \
         damage instead of leaving it that fighter's attack"
    );
}

/// A SHOT STAMPED AT BIRTH KEEPS ITS AIM THROUGH ITS FIRER'S ELIMINATION.
///
/// The positive term for
/// [`a_shot_orphaned_before_its_first_step_does_not_turn_on_its_team`], and the
/// reason `stamp_new_projectile_allegiance` exists. That test pins the SAFE
/// behaviour when nothing stamped the bolt; this one pins that the production
/// schedule stamps it, so the safe fallback is never reached.
///
///  the modelled tick is the one that made the second stamp placement
/// necessary. A player bolt materializes at the end of `CombatSet::Materialize`
/// and is stamped there; its firer is eliminated later in the SAME tick, in
/// `CombatSet::Settle`; the bolt first STEPS next tick, with no firer left. So
/// the bolt is deliberately never stepped while its owner lives — which is what
/// `run_system_once` models here, and what a plain `app.update()` cannot: an
/// update would step the bolt with the firer still resident and the stepper's own
/// first-sight stamp would take the fact, hiding whether this system did.
#[test]
fn a_shot_stamped_at_birth_survives_its_firers_elimination() {
    use ambition_combat::targeting::MatchTeam;
    use bevy::ecs::system::RunSystemOnce as _;

    let mut app = arena_projectile_app(ambition_combat::targeting::FactionRelations::default());

    let firer = app
        .world_mut()
        .spawn((
            ambition_combat::components::ActorFaction::Player,
            MatchTeam::new("seat 1"),
        ))
        .id();
    let teammate_pos = ae::Vec2::new(300.0, 100.0);
    let opponent_pos = ae::Vec2::new(300.0, 300.0);
    let teammate = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("same_team_fighter"),
            ambition_combat::components::CenteredAabb::new(teammate_pos, ae::Vec2::new(16.0, 24.0)),
            ambition_combat::components::ActorFaction::Player,
            MatchTeam::new("seat 1"),
        ))
        .id();
    let opponent = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("seat_two_fighter"),
            ambition_combat::components::CenteredAabb::new(opponent_pos, ae::Vec2::new(16.0, 24.0)),
            ambition_combat::components::ActorFaction::Player,
            MatchTeam::new("seat 2"),
        ))
        .id();

    spawn_owned_glider(&mut app, teammate_pos - ae::Vec2::new(150.0, 0.0), firer);
    spawn_owned_glider(&mut app, opponent_pos - ae::Vec2::new(150.0, 0.0), firer);

    // `Materialize`: the bolts exist and take their side. No step yet.
    app.world_mut()
        .run_system_once(crate::projectile::stamp_new_projectile_allegiance)
        .expect("the stamping system runs");

    // `Settle`, same tick: the final stock is spent and the body leaves play.
    app.world_mut().despawn(firer);

    let mut live_projectiles = app
        .world_mut()
        .query_filtered::<Entity, With<ambition_projectiles::LiveProjectile>>();
    for _ in 0..240 {
        if live_projectiles.iter(app.world()).next().is_none() {
            break;
        }
        app.update();
    }

    let cap = app.world().resource::<CapturedHits>();
    let hit = |who: Entity| {
        cap.0.iter().any(|e| {
            matches!(e.source, HitSource::Projectile)
                && e.target == ambition_combat::events::HitTarget::Body(who)
        })
    };
    assert!(
        hit(opponent),
        "a bolt stamped at birth went inert after its firer left — the stamp did \
         not survive the elimination, so the shot stopped being that fighter's attack"
    );
    assert!(
        !hit(teammate),
        "the stamped bolt hit its firer's own teammate, which is the other way to \
         get this wrong: it took a side and then ignored it"
    );
}

/// A shot owned by `firer`, overlapping `pos`. Like
/// [`spawn_overlapping_enemy_glider`] but the OWNER is the caller's, because
/// what is under test is a fact about the owner (its team).
fn spawn_owned_glider(app: &mut App, pos: ae::Vec2, firer: Entity) {
    use ambition_projectiles::{ProjectileOwner, ProjectileSeq};
    let projectile = build_in_flight_projectile(ProjectileSpawn {
        origin: pos,
        dir: ae::Vec2::new(1.0, 0.0),
        speed: 200.0,
        damage: 3,
        max_lifetime: 2.0,
        half_extent: ae::Vec2::new(8.0, 8.0),
        gravity: 0.0,
        visual_id: String::new(),
        bounces: 0,
        bounce_on_world_contact: false,
        splash_half_extent: 0.0,
        boomerang_return_s: None,
    });
    let seq: ProjectileSeq = {
        let mut counter = app
            .world_mut()
            .get_resource_or_insert_with(ProjectileSeqCounter::default);
        counter.next()
    };
    app.world_mut().spawn((
        projectile.body.kin,
        projectile.body.game,
        seq,
        ProjectileOwner(firer),
        ambition_projectiles::LiveProjectile,
        bevy::prelude::Name::new("Owned glider (test)"),
    ));
}

/// Parry-reflect: an enemy shot overlapping a parrying player flips to
/// the player's faction and reverses (+boosts) its velocity, so the same
/// faction-aware routing now sends it back at the enemies — deflect the
/// boss's attack at it.
/// A world with ONE player whose parry window is open, ready to reflect a shot
/// that overlaps them. Returns the app and the player.
///
/// ⭐ Shared by both parry tests so neither can drift into a different world:
/// the faction/velocity acceptance and the provenance one must be describing the
/// SAME reflection, or one of them is answering about a road the other does not
/// take.
fn parry_ready_player_app() -> (App, Entity) {
    use ambition_characters::actor::BodyCombat;
    use ambition_platformer2d_core::BodyKinematics;
    use ambition_platformer2d_core::{BodyBaseSize, BodyOffense, BodyShieldState};
    use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
    let mut app = App::new();
    insert_projectile_authority(&mut app);
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "phys",
            ae::Vec2::new(800.0, 800.0),
            ae::Vec2::new(400.0, 400.0),
            vec![],
        )),
    );
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    app.add_message::<HitEvent>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<crate::avatar::PlayerHealRequested>();
    app.init_resource::<ProjectileSeqCounter>();
    app.init_resource::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>();
    app.init_resource::<ambition_gameplay_trace::GameplayTraceBuffer>();
    app.add_systems(Update, crate::projectile::step_projectiles);

    let player_pos = ae::Vec2::new(200.0, 200.0);
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            ambition_combat::components::ActorFaction::Player,
            BodyKinematics {
                pos: player_pos,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            // Published combat footprint used by victim geometry.
            ae::CenteredAabb::from_center_size(player_pos, ae::Vec2::new(24.0, 40.0)),
            BodyBaseSize {
                base_size: ae::Vec2::new(24.0, 40.0),
            },
            BodyOffense::default(),
            ambition_platformer2d_core::BodyMotionFacts::default(),
            // Parry window OPEN.
            BodyShieldState {
                active: true,
                parry_window_timer: 0.2,
                ..Default::default()
            },
            BodyCombat::default(),
            // Projectile victim queries read the resolved motion frame.
            ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
        ))
        .id();
    (app, player)
}

/// ⛔⛤ THE CROSS-ENTITY WITNESS. A PARRIED BOLT STOPS NAMING THE FIRER'S MOVE,
/// THROUGH THE SHIPPED CALL SITE.
///
/// ⛔⛔ THE DEFECT THIS FAILS ON. `MoveOccurrence` counts PER ENTITY from 0, so
/// the same small integers are live on every body at once. The verdict channel
/// carries `(attacker, instance)`; parry rewrites the attacker — `ProjectileOwner`
/// becomes the player — and used to leave the instance alone. An enemy shot
/// stamped with the FIRER's occurrence 0, reflected, then landing, met the
/// player's own occurrence 0 and credited whatever move the player was playing.
/// ⚠ AND 0-AGAINST-0 IS THE EXPECTED CASE EARLY IN A FIGHT, not a corner.
///
/// ⭐⭐ A SAME-ENTITY FIXTURE CANNOT REACH THIS. Every other occurrence test in
/// the tree puts one body's two uses against each other, where the numbers are
/// genuinely comparable. The bug lives precisely where they are NOT — two
/// counters, one integer range, and a predicate that cannot tell which body a
/// number came from. No predicate can: the only defence is clearing the stamp at
/// the boundary where the pair stops being consistent.
///
/// ⇒ This is the shipped road (`reflect_projectile` inside `step_projectiles`),
/// not the domain function — `projectile::intercept` tests that directly. Both
/// exist because a call site can stop calling.
#[test]
fn a_parried_enemy_shot_stops_naming_the_firers_move_use() {
    let (mut app, _player) = parry_ready_player_app();
    let player_pos = ae::Vec2::new(200.0, 200.0);
    let incoming = ae::Vec2::new(-300.0, 0.0);
    crate::enemy_projectile::test_support::spawn_test_projectile_fired_by(
        &mut app,
        ProjectileSpawn {
            origin: player_pos,
            dir: incoming.normalize(),
            speed: 300.0,
            damage: 2,
            max_lifetime: 2.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            gravity: 0.0,
            visual_id: String::new(),
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 0.0,
            boomerang_return_s: None,
        },
        ActorFaction::Enemy,
        // The FIRER's first use of their move. The player's first use is also 0.
        Some(0),
    );

    // ⚠ THE PREMISE, ASSERTED: the bolt carries the firer's occurrence BEFORE
    // the parry, so its absence afterwards is a claim about the parry and not
    // about a fixture that never stamped it.
    assert_eq!(
        app.world_mut()
            .query::<&ambition_projectiles::FiredByMoveInstance>()
            .iter(app.world())
            .map(|s| s.0)
            .collect::<Vec<_>>(),
        vec![0],
        "the premise: one live bolt, stamped with the firer's use"
    );

    app.update();

    assert_eq!(
        live_projectile_bodies(&mut app).len(),
        1,
        "the premise: the parried bolt stayed in flight, so there is still a \
         shot for the stamp to be absent from"
    );
    assert!(
        app.world_mut()
            .query::<&ambition_projectiles::FiredByMoveInstance>()
            .iter(app.world())
            .next()
            .is_none(),
        "the parried bolt still names the FIRER's use of their move. The player \
         counts occurrences from 0 on their own body, so that stale 0 credits a \
         move that never fired this shot -- and both bodies hold 0 on the first \
         use, which is where a fight starts"
    );
}

#[test]
fn a_parried_enemy_shot_flips_to_player_faction_and_reverses() {
    let (mut app, player) = parry_ready_player_app();
    let player_pos = ae::Vec2::new(200.0, 200.0);
    // An enemy bolt overlapping the player, travelling left (toward where it
    // came from — at the player).
    let incoming = ae::Vec2::new(-300.0, 0.0);
    spawn_test_projectile(
        &mut app,
        ProjectileSpawn {
            origin: player_pos,
            dir: incoming.normalize(),
            speed: 300.0,
            damage: 2,
            max_lifetime: 2.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            gravity: 0.0,
            visual_id: String::new(),
            // Straight shot: this ability authors no bounce.
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 0.0,
            boomerang_return_s: None,
        },
        ActorFaction::Enemy,
    );

    app.update();

    let bodies = live_projectile_bodies(&mut app);
    assert_eq!(bodies.len(), 1, "the parried bolt stays in flight");
    let body = &bodies[0].body;
    // Parry RE-OWNS the bolt to the player (so its firer faction is Player next
    // tick → it routes as the player's own shot, back at the enemies) — it does
    // NOT mutate a faction label.
    let owner = app
        .world_mut()
        .query::<&ambition_projectiles::ProjectileOwner>()
        .iter(app.world())
        .next()
        .map(|o| o.0);
    assert_eq!(owner, Some(player), "parry re-owns the bolt to the player");
    assert!(
        body.kin.vel.x > 0.0,
        "reversed: it now travels back toward the enemy (was -x)"
    );
    assert!(
        body.kin.vel.length() > 300.0,
        "reflected with a speed boost, was 300 now {}",
        body.kin.vel.length()
    );
}

/// Task B: an enemy shot spawned through the executor with a real firing
/// actor carries `ProjectileOwner`, so the hit it lands on the player
/// attributes back to that actor (`HitEvent::attacker`), instead of the
/// historical `None`. Drives the full projectile-request → materializer →
/// `step_projectiles` path so the stamping + the enemy-branch read are both
/// exercised.
#[test]
fn an_owned_enemy_shot_attributes_its_player_hit_to_the_firing_actor() {
    use ambition_characters::actor::BodyCombat;
    use ambition_platformer2d_core::BodyKinematics;
    use ambition_platformer2d_core::{BodyBaseSize, BodyOffense, BodyShieldState};
    use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
    let mut app = App::new();
    insert_projectile_authority(&mut app);
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "phys",
            ae::Vec2::new(800.0, 800.0),
            ae::Vec2::new(400.0, 400.0),
            vec![],
        )),
    );
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    app.add_message::<HitEvent>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<ProjectileSpawnRequest>();
    app.add_message::<crate::avatar::PlayerHealRequested>();
    app.init_resource::<ProjectileSeqCounter>();
    app.init_resource::<CapturedHits>();
    app.init_resource::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>();
    app.init_resource::<ambition_gameplay_trace::GameplayTraceBuffer>();
    app.add_systems(
        Update,
        (
            ambition_projectiles::materialize_projectiles_for_this_tick,
            crate::projectile::step_projectiles,
            capture_hits,
        )
            .chain(),
    );

    // Give the firing stand-in an enemy faction so attribution follows the real enemy path.
    let attacker = app.world_mut().spawn(ActorFaction::Enemy).id();

    // A vulnerable player (no parry / dodge / invuln) at the shot's origin.
    let player_pos = ae::Vec2::new(200.0, 200.0);
    app.world_mut().spawn((
        PlayerEntity,
        BodyKinematics {
            pos: player_pos,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(24.0, 40.0),
            facing: 1.0,
        },
        // Published combat footprint used by victim geometry.
        ae::CenteredAabb::from_center_size(player_pos, ae::Vec2::new(24.0, 40.0)),
        // Match production faction routing so hostility is tested relationally.
        ActorFaction::Player,
        BodyBaseSize {
            base_size: ae::Vec2::new(24.0, 40.0),
        },
        BodyOffense::default(),
        ambition_platformer2d_core::BodyMotionFacts::default(),
        BodyShieldState::default(),
        BodyCombat::default(),
        // Projectile victim queries read the resolved motion frame.
        ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
    ));

    // Fire an enemy-faction shot owned by `attacker`, overlapping the player.
    app.world_mut().write_message(ProjectileSpawnRequest::open(
        attacker,
        ProjectileSpawn {
            origin: player_pos,
            dir: ae::Vec2::new(1.0, 0.0),
            speed: 100.0,
            damage: 2,
            max_lifetime: 2.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            gravity: 0.0,
            visual_id: String::new(),
            // Straight shot: this ability authors no bounce.
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 0.0,
            boomerang_return_s: None,
        },
        ProjectileStart::StepThisTick,
    ));

    app.update();

    let cap = app.world().resource::<CapturedHits>();
    let player_hit = cap
        .0
        .iter()
        .find(|e| matches!(e.source, HitSource::Projectile))
        .expect("the enemy shot lands a projectile hit on the controlled body");
    assert_eq!(
        player_hit.attacker,
        Some(attacker),
        "the hit attributes back to the firing actor, not None"
    );
}

/// A projectile uses the same published victim geometry as melee.
///
/// A published silhouette collides only where its authored rectangles overlap the
/// bolt. The control arm covers the same body/bolt position to prove the miss is
/// geometric rather than a fixture failure.
#[test]
fn a_bolt_misses_the_gap_in_an_authored_silhouette() {
    use ambition_characters::actor::{BodyHealth, Health};

    /// Spawn one enemy bolt against a player publishing `volume`, isolating victim geometry.
    fn arena_publishing(volume: ae::Aabb) -> App {
        let mut app = arena_projectile_app(ambition_combat::targeting::FactionRelations::default());
        let pos = ae::Vec2::new(300.0, 100.0);
        let mut volumes = ambition_combat::components::DamageableVolumes::default();
        volumes.set_single(volume);
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("arena_fighter"),
            ambition_combat::components::CenteredAabb::new(pos, ae::Vec2::new(16.0, 24.0)),
            ambition_combat::components::ActorFaction::Player,
            volumes,
            BodyHealth::new(Health {
                current: 3,
                max: 3,
                invulnerable: Default::default(),
            }),
        ));
        spawn_overlapping_enemy_glider(&mut app, pos);
        app.update();
        app
    }

    // ── Control: the published rectangle covers the body, and the bolt lands ──
    let covering = arena_publishing(ae::Aabb::new(
        ae::Vec2::new(300.0, 100.0),
        ae::Vec2::new(8.0, 12.0),
    ));
    assert!(
        !covering.world().resource::<CapturedHits>().0.is_empty(),
        "a bolt inside an authored silhouette must land — without this the miss \
         below proves only that the fixture never overlapped"
    );

    // ── The gap: the same body, the same bolt, a silhouette that is elsewhere ──
    let gapped = arena_publishing(ae::Aabb::new(
        ae::Vec2::new(300.0, 400.0),
        ae::Vec2::new(8.0, 12.0),
    ));
    assert!(
        gapped.world().resource::<CapturedHits>().0.is_empty(),
        "the bolt overlapped the coarse box and NOT the authored volume, and it \
         still landed — the projectile is testing the box again, which is the \
         half of D23 Jon ruled for"
    );
}

#[test]
fn a_bolt_passes_through_a_body_that_published_no_hurtbox() {
    use ambition_characters::actor::{BodyHealth, Health};

    /// One Enemy-owned bolt already overlapping one Player-faction body, with
    /// the shipped volume publisher running ahead of the stepper exactly as the
    /// production schedule runs it.
    fn arena_with_victim_hp(current: i32) -> (App, Entity) {
        let mut app = arena_projectile_app(ambition_combat::targeting::FactionRelations::default());
        app.add_systems(
            Update,
            crate::features::refresh_body_damageable_volumes
                .before(crate::projectile::step_projectiles),
        );
        let pos = ae::Vec2::new(300.0, 100.0);
        let victim = app
            .world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
                ambition_combat::components::FeatureId::new("arena_fighter"),
                ambition_combat::components::CenteredAabb::new(pos, ae::Vec2::new(16.0, 24.0)),
                ambition_combat::components::ActorFaction::Player,
                // Carrying this is what makes a body a damage target at all.
                ambition_combat::components::DamageableVolumes::default(),
                BodyHealth::new(Health {
                    current,
                    max: 3,
                    invulnerable: Default::default(),
                }),
            ))
            .id();
        spawn_overlapping_enemy_glider(&mut app, pos);
        app.update();
        (app, victim)
    }

    // ── Live control: the publisher publishes the coarse box, the bolt lands ──
    let (live, victim) = arena_with_victim_hp(3);
    assert!(
        live.world()
            .get::<ambition_combat::components::DamageableVolumes>(victim)
            .expect("the shipped publisher ran")
            .published(),
        "the publisher must have spoken for this body, or 'published nothing' is \
         not a distinction this world can make"
    );
    assert!(
        live.world()
            .resource::<CapturedHits>()
            .0
            .iter()
            .any(|e| e.target == ambition_combat::events::HitTarget::Body(victim)),
        "a living body in the bolt's path is struck — otherwise the miss below \
         proves only that the geometry never overlapped"
    );

    // ── Intangible: published, and published NOTHING ──
    let (mut dead, victim) = arena_with_victim_hp(0);
    let published = dead
        .world()
        .get::<ambition_combat::components::DamageableVolumes>(victim)
        .expect("the shipped publisher ran");
    assert!(
        published.published() && published.volumes.is_empty(),
        "the premise: the publisher emptied this body's silhouette, which is a \
         DECISION (intangible), not an absence"
    );
    assert!(
        dead.world().resource::<CapturedHits>().0.is_empty(),
        "a body that published NO hurtbox cannot be hit anywhere — a bolt must \
         pass through it, exactly as a swing does"
    );
    assert_eq!(
        live_projectile_bodies(&mut dead).len(),
        1,
        "and the shot is not absorbed: an intangible body must not eat a bolt \
         that should have flown on to whatever was behind it"
    );
}

/// The spawn executor carries the shot's open `visual_id` forward onto a
/// `ProjectileVisualId` component — the render layer's single art-selection
/// input, set without reading `owner_id`.
#[test]
fn spawn_executor_attaches_visual_id() {
    use ambition_projectiles::ProjectileVisualId;
    let mut app = App::new();
    insert_projectile_authority(&mut app);
    app.add_message::<ProjectileSpawnRequest>();
    app.init_resource::<ProjectileSeqCounter>();
    app.add_systems(
        Update,
        ambition_projectiles::materialize_projectiles_for_this_tick,
    );
    app.world_mut().write_message(ProjectileSpawnRequest::open(
        Entity::PLACEHOLDER,
        ProjectileSpawn {
            origin: ae::Vec2::ZERO,
            dir: ae::Vec2::new(1.0, 0.0),
            speed: 100.0,
            damage: 1,
            max_lifetime: 1.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            gravity: 0.0,
            visual_id: "glider".into(),
            // Straight shot: this ability authors no bounce.
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 0.0,
            boomerang_return_s: None,
        },
        ProjectileStart::StepThisTick,
    ));
    app.update();
    let mut q = app.world_mut().query::<&ProjectileVisualId>();
    let ids: Vec<_> = q.iter(app.world()).map(|v| v.0.clone()).collect();
    assert_eq!(
        ids,
        vec!["glider".to_string()],
        "the glider visual id must ride onto the spawned projectile"
    );
}
