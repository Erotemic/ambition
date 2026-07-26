//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use ambition_engine_core as ae;

use crate::{ProjectileSeq, ProjectileVisualId};

fn spawn_request(owner_id: &str, visual_id: &str) -> ambition_vfx::EffectRequest {
    ambition_vfx::EffectRequest {
        owner: Entity::PLACEHOLDER,
        effect: ambition_vfx::Effect::Projectiles {
            shots: vec![crate::enemy::EnemyProjectileSpawn {
                origin: ae::Vec2::ZERO,
                dir: ae::Vec2::new(1.0, 0.0),
                speed: 100.0,
                damage: 1,
                max_lifetime: 1.0,
                half_extent: ae::Vec2::new(8.0, 8.0),
                owner_id: owner_id.into(),
                gravity: 0.0,
                visual_id: visual_id.into(),
                // Straight shot: this ability authors no bounce.
                bounces: 0,
                bounce_on_world_contact: false,
            }],
        },
    }
}

#[test]
fn effect_request_spawns_enemy_projectile_entity_with_visual_and_sequence() {
    let mut app = App::new();
    app.add_message::<ambition_vfx::EffectRequest>();
    app.init_resource::<ProjectileSeqCounter>();
    app.add_systems(Update, apply_enemy_projectile_effect_requests);

    app.world_mut()
        .write_message(spawn_request("pca", "glider"));
    app.update();

    let mut q = app.world_mut().query_filtered::<
        (&ProjectileOwnerId, &ProjectileVisualId, &ProjectileSeq),
        (With<EnemyProjectile>, With<LiveProjectile>),
    >();
    let rows: Vec<_> = q
        .iter(app.world())
        .map(|(owner_id, visual_id, seq)| (owner_id.0.clone(), visual_id.0.clone(), *seq))
        .collect();
    assert_eq!(
        rows,
        vec![("pca".to_string(), "glider".to_string(), ProjectileSeq(0)),],
        "the substrate executor stamps owner id, visual id, and deterministic sequence"
    );
}

#[test]
fn effect_request_preserves_real_owner_entity_for_later_hit_attribution() {
    let mut app = App::new();
    app.add_message::<ambition_vfx::EffectRequest>();
    app.init_resource::<ProjectileSeqCounter>();
    app.add_systems(Update, apply_enemy_projectile_effect_requests);

    let owner = app.world_mut().spawn_empty().id();
    let mut req = spawn_request("boss_bolt", "");
    req.owner = owner;
    app.world_mut().write_message(req);
    app.update();

    let mut q = app
        .world_mut()
        .query_filtered::<&ProjectileOwner, (With<EnemyProjectile>, With<LiveProjectile>)>();
    let owners: Vec<_> = q.iter(app.world()).map(|owner| owner.0).collect();
    assert_eq!(
        owners,
        vec![owner],
        "a real effect owner is carried for sim-side hit attribution"
    );
}

#[test]
fn placeholder_owner_remains_ownerless() {
    let mut app = App::new();
    app.add_message::<ambition_vfx::EffectRequest>();
    app.init_resource::<ProjectileSeqCounter>();
    app.add_systems(Update, apply_enemy_projectile_effect_requests);

    app.world_mut()
        .write_message(spawn_request("ownerless", ""));
    app.update();

    let mut q = app
        .world_mut()
        .query_filtered::<Option<&ProjectileOwner>, (With<EnemyProjectile>, With<LiveProjectile>)>(
        );
    let owner_present: Vec<_> = q.iter(app.world()).map(|owner| owner.is_some()).collect();
    assert_eq!(
        owner_present,
        vec![false],
        "placeholder effects do not fabricate an owner component"
    );
}

/// **H1: the bolt carries its firer's voice from the frame it exists.**
///
/// Not "eventually". This executor is followed immediately by `step_projectiles`
/// in the same `Combat` set, so a shot that spawns and hits a wall inside one tick
/// has emitted its impact before any later system runs. The engine's inheritance
/// pass runs EARLIER in that set and therefore could never reach these — it filled
/// the gap for player shots (which first step next frame) and silently missed the
/// whole enemy pool (GPT 5.6, 2026-07-26).
///
/// So the assertion is deliberately made on the same `update` that spawns it, with
/// no second tick: that is the ordering the bug lived in.
#[test]
fn a_shot_carries_its_firers_presentation_source_on_the_frame_it_spawns() {
    let mut app = App::new();
    app.add_message::<ambition_vfx::EffectRequest>();
    app.init_resource::<ProjectileSeqCounter>();
    app.add_systems(Update, apply_enemy_projectile_effect_requests);

    let firer = app
        .world_mut()
        .spawn(ambition_sfx::BodyPresentationSource(
            ambition_sfx::PresentationSourceId::new("sanic_demo"),
        ))
        .id();
    let mut request = spawn_request("sanic", "bolt");
    request.owner = firer;
    app.world_mut().write_message(request);
    app.update();

    let mut q = app
        .world_mut()
        .query_filtered::<&ambition_sfx::BodyPresentationSource, With<EnemyProjectile>>();
    let sources: Vec<String> = q
        .iter(app.world())
        .map(|source| source.id().as_str().to_string())
        .collect();
    assert_eq!(
        sources,
        vec!["sanic_demo".to_string()],
        "a shot that spawns and steps in one tick must already know whose it is"
    );
}

/// An OWNERLESS shot — an environmental volley with no firing body — carries no
/// source, and falls back to the session context. Absent is the honest answer for
/// something nobody fired, and is a different fact from an empty source.
#[test]
fn an_ownerless_shot_carries_no_presentation_source() {
    let mut app = App::new();
    app.add_message::<ambition_vfx::EffectRequest>();
    app.init_resource::<ProjectileSeqCounter>();
    app.add_systems(Update, apply_enemy_projectile_effect_requests);

    app.world_mut()
        .write_message(spawn_request("hazard", "bolt"));
    app.update();

    let mut q = app
        .world_mut()
        .query_filtered::<Option<&ambition_sfx::BodyPresentationSource>, With<EnemyProjectile>>();
    let sources: Vec<bool> = q.iter(app.world()).map(|s| s.is_some()).collect();
    assert_eq!(sources, vec![false]);
}
