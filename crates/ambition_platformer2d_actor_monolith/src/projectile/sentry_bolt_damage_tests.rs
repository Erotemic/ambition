//! The sentry bolt's END-TO-END damage proof, which lives HERE and not with the
//! sentry.
//!
//! ⭐⭐ IT MOVED IN THE ABILITIES CARVE (D33, 2026-09-03) AND THE REASON IS THE
//! POINT. `ambition_abilities` owns `update_sentries` and every other test of it
//! went with the crate. This one does not fit there: it chains
//! `update_sentries` → `materialize_projectiles_for_this_tick` →
//! `stamp_new_projectile_allegiance` → `step_projectiles`, and the last two are
//! the KERNEL's. A test that needs two crates belongs where both are visible,
//! and the carved crate must not grow a dependency on the kernel to keep one
//! fixture — that edge is exactly what the carve removed.
//!
//! ⛔ SO DO NOT "TIDY" IT BACK. Moving it into `ambition_abilities` requires
//! naming `ambition_platformer2d_actor_monolith` from a crate below it, which
//! will not compile; moving the two kernel systems down is a different carve
//! with its own question, not a convenience.
//!
//! ⚠ THE VERDICT UNDER TEST IS `can_hit`, unchanged by the move: a `HitEvent`
//! naming this victim with this damage is exactly what the faction routing
//! decides. `apply_feature_hit_events` — which turns that into `BodyHealth` — is
//! covered where it lives.

use ambition_abilities::ranged::sentry::{deploy_sentry, update_sentries, SENTRY_BOLT_DAMAGE};
use ambition_combat::components::ActorFaction;
use ambition_combat::events::{HitEvent, HitSource};
use ambition_platformer2d_core as ae;
use ambition_projectiles::ProjectileSpawnRequest;
use ambition_vfx::vfx::VfxMessage;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct CapturedHits(Vec<HitEvent>);

fn capture_hits(mut reader: MessageReader<HitEvent>, mut cap: ResMut<CapturedHits>) {
    for e in reader.read() {
        cap.0.push(e.clone());
    }
}

/// ⭐⭐ A DEPLOYED SENTRY MUST ACTUALLY DAMAGE THE ENEMY IT SHOOTS.
///
/// ⛔⛔ ASSERTING THAT A BOLT APPEARS IS NOT THIS TEST. The turret fired,
/// the projectile materialized, it flew, it overlapped its target — and it
/// could not damage anything, because a shot's combat side is stamped from
/// its OWNER entity and the owner here is the turret, which carries
/// `Sentry`, `Name` and a session scope and no `ActorFaction` at all.
/// `indiscriminate` is `allegiance.is_none() && owner.is_none()`, so a
/// named owner with no faction is the one combination that can hit nobody.
#[test]
fn a_sentry_bolt_damages_the_enemy_it_was_fired_at() {
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.init_resource::<ambition_projectiles::ProjectileVisualCatalog>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
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
    app.init_resource::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>();
    app.init_resource::<ambition_gameplay_trace::GameplayTraceBuffer>();
    app.add_systems(
        Update,
        (
            update_sentries,
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
        deploy_sentry(
            &mut commands,
            ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
            ae::Vec2::new(300.0, 400.0),
            side,
            None,
            None,
            None,
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
        "the sentry's bolt reached its target and dealt no damage — a shot \
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
        hits.iter().any(|e| e.damage == SENTRY_BOLT_DAMAGE),
        "the bolt lands its authored {SENTRY_BOLT_DAMAGE} damage, got {:?}",
        hits.iter().map(|e| e.damage).collect::<Vec<_>>(),
    );
    // ⚠ THE VERDICT UNDER TEST IS `can_hit`, and it is complete here: a
    // `HitEvent` naming this victim with this damage is exactly what the
    // faction routing decides. `apply_feature_hit_events` — which turns that
    // into `BodyHealth` — is a separate system sitting AT Bevy's
    // system-param ceiling and is covered where it lives; wiring its dozen
    // resources in here would make this a test about fixture assembly.
}
