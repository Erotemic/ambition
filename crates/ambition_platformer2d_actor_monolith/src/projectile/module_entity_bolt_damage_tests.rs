//! A module entity's bolt's END-TO-END damage proof (first written for the
//! native sentry; the sentry is now a procedural module on the module-entity
//! ports, 2026-10-01).
//!
//! It lives HERE because it chains the projectile request through
//! `materialize_projectiles_for_this_tick` → `stamp_new_projectile_allegiance`
//! → `step_projectiles`, and the last two are the KERNEL's. A test that needs
//! two crates belongs where both are visible.
//!
//! The turret comes from the production seam
//! (`ambition_abilities::module_entity::spawn_module_entity`). The request is
//! written as the projectile domain's adapter lowers one a module entity
//! submits: `ProjectileSpawnRequest::open(<the entity>, .., StepThisTick)`.
//! What the module computes (aim, cadence) is held to the native sentry by
//! `ambition_content`'s `sentry_parity_tests`; this test is about the OWNER.
//!
//! ⚠ THE VERDICT UNDER TEST IS `can_hit`: a `HitEvent` naming this victim
//! with this damage is exactly what the faction routing decides.
//! `apply_feature_hit_events` — which turns that into `BodyHealth` — is
//! covered where it lives.

use ambition_abilities::module_entity::{spawn_module_entity, ModuleEntity, Spawner};
use ambition_projectiles::{ProjectileSpawn, ProjectileStart};
use ambition_combat::components::ActorFaction;
use ambition_combat::events::{HitEvent, HitSource};
use ambition_platformer2d_core as ae;
use ambition_projectiles::ProjectileSpawnRequest;
use ambition_vfx::vfx::VfxMessage;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct CapturedHits(Vec<HitEvent>);

const BOLT_DAMAGE: i32 = 2;

/// Each module entity fires one bolt at the target on its first tick, as the
/// projectile adapter lowers a module's request.
fn fire_once(
    entities: Query<(Entity, &ModuleEntity)>,
    mut fired: Local<bool>,
    mut requests: MessageWriter<ProjectileSpawnRequest>,
) {
    if *fired {
        return;
    }
    for (entity, module_entity) in &entities {
        *fired = true;
        requests.write(ProjectileSpawnRequest::open(
            entity,
            ProjectileSpawn {
                origin: module_entity.pos,
                dir: ae::Vec2::X,
                speed: 430.0,
                damage: BOLT_DAMAGE,
                max_lifetime: 1.4,
                half_extent: ae::Vec2::new(7.0, 7.0),
                gravity: 0.0,
                visual_id: String::new(),
                bounces: 0,
                bounce_on_world_contact: false,
                splash_half_extent: 0.0,
                boomerang_return_s: None,
            },
            ProjectileStart::StepThisTick,
        ));
    }
}

fn capture_hits(mut reader: MessageReader<HitEvent>, mut cap: ResMut<CapturedHits>) {
    for e in reader.read() {
        cap.0.push(e.clone());
    }
}

/// ⭐⭐ A DEPLOYED TURRET MUST ACTUALLY DAMAGE THE ENEMY IT SHOOTS.
///
/// ⛔⛔ ASSERTING THAT A BOLT APPEARS IS NOT THIS TEST. The turret fired,
/// the projectile materialized, it flew, it overlapped its target — and it
/// could not damage anything, because a shot's combat side is stamped from
/// its OWNER entity and the owner here is the turret, which carried
/// `Sentry`, `Name` and a session scope and no `ActorFaction` at all.
/// `indiscriminate` is `allegiance.is_none() && owner.is_none()`, so a
/// named owner with no faction is the one combination that can hit nobody.
#[test]
fn a_module_entitys_bolt_damages_the_enemy_it_was_fired_at() {
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.init_resource::<ambition_projectiles::ProjectileVisualCatalog>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "sentry range",
            ae::Vec2::new(2000.0, 800.0),
            ae::Vec2::new(400.0, 400.0),
            Vec::new(),
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
    app.init_resource::<ambition_projectiles::ProjectileSeqCounter>();
    app.init_resource::<CapturedHits>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay::default());
    app.init_resource::<ambition_gameplay_trace::GameplayTraceBuffer>();
    app.add_systems(
        Update,
        (
            fire_once,
            ambition_projectiles::materialize_projectiles_for_this_tick,
            crate::projectile::stamp_new_projectile_allegiance,
            crate::projectile::step_projectiles,
            capture_hits,
        )
            .chain(),
    );

    let enemy_pos = ae::Vec2::new(360.0, 400.0);
    let enemy = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
            ambition_combat::components::FeatureId::new("sentry_target"),
            ambition_combat::components::CenteredAabb::new(
                enemy_pos,
                ae::Vec2::new(16.0, 24.0),
            ),
            ambition_combat::components::ActorDisposition::Hostile,
            ActorFaction::Enemy,
            ambition_characters::actor::BodyCombat {
                hit_flash: 0.0,
                training_dummy: false,
                ..Default::default()
            },
            ambition_characters::actor::BodyHealth::new(
                ambition_characters::actor::Health::new(20),
            ),
        ))
        .id();

    // The turret, deployed by a Player-faction wielder, a short way from its
    // target and armed to fire on the first tick.
    let wielder = app.world_mut().spawn(ActorFaction::Player).id();
    // ⛔ SPAWNED THROUGH THE PRODUCTION SEAM, not by hand: a fixture that
    // assembles its own turret can give it a faction production never
    // grants, and then this test passes about a body that does not ship.
    let side = *app
        .world()
        .get::<ActorFaction>(wielder)
        .expect("the fixture wielder states a side");
    {
        let mut commands = app.world_mut().commands();
        spawn_module_entity(
            &mut commands,
            ModuleEntity {
                kind: "sentry".into(),
                pos: ae::Vec2::new(300.0, 400.0),
                remaining_s: 5.0,
            },
            Spawner {
                scope: ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                side,
                team: None,
                presentation: None,
                id: ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(
                    &ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
                    0,
                ),
            },
        );
    }
    app.world_mut().flush();

    // Long enough for the bolt to cross the 60px gap at its authored speed.
    for _ in 0..90 {
        app.update();
    }

    let hits: Vec<_> = app
        .world()
        .resource::<CapturedHits>()
        .0
        .iter()
        .filter(|e| matches!(e.source, HitSource::Projectile))
        .collect();
    assert!(
        !hits.is_empty(),
        "the turret's bolt reached its target and dealt no damage — a shot \
         whose owner carries no faction stamps no allegiance, and \
         `indiscriminate` is false for a NAMED owner, so `can_hit` is false \
         against every victim in the world"
    );

    assert!(
        hits.iter()
            .all(|e| e.target == ambition_combat::events::HitTarget::Body(enemy)),
        "the bolt must NAME the body it struck, got {:?}",
        hits.iter().map(|e| &e.target).collect::<Vec<_>>(),
    );
    assert!(
        hits.iter().any(|e| e.damage == BOLT_DAMAGE),
        "the bolt lands its authored {BOLT_DAMAGE} damage, got {:?}",
        hits.iter().map(|e| e.damage).collect::<Vec<_>>(),
    );
    // ⚠ THE VERDICT UNDER TEST IS `can_hit`, and it is complete here: a
    // `HitEvent` naming this victim with this damage is exactly what the
    // faction routing decides. `apply_feature_hit_events` — which turns that
    // into `BodyHealth` — is a separate system sitting AT Bevy's
    // system-param ceiling and is covered where it lives; wiring its dozen
    // resources in here would make this a test about fixture assembly.
}
