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
