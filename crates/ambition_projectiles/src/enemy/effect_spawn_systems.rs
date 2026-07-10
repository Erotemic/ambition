//! Effect-request spawn executor for enemy/boss projectile volleys.
//!
//! Techniques emit `ambition_vfx::Effect::Projectiles` requests. This module
//! materializes those requests as the enemy projectile pool's ECS entities. It
//! is still substrate-only: it stamps shared projectile components, owner ids,
//! visual ids, and deterministic sequence numbers, but it does not resolve
//! victims or inspect actor/player/boss state. The faction-routed hit routing
//! remains in the sim-side stepper that consumes these entities.

use bevy::prelude::*;

use crate::enemy::{EnemyProjectile, EnemyProjectileState};
use crate::{
    LiveProjectile, ProjectileOwner, ProjectileOwnerId, ProjectileSeqCounter, ProjectileVisualKind,
};

/// Materialize enemy-pool projectiles from [`ambition_vfx::Effect::Projectiles`]
/// requests — one projectile entity per shot.
///
/// Scheduled before the unified projectile stepper, this preserves the legacy
/// timing where a boss/enemy projectile spawned by an effect advances on the same
/// frame. Non-projectile effects remain owned by `ambition_vfx::apply_effects`;
/// this executor exists with the projectile substrate so both projectile pools
/// receive the shared [`crate::ProjectileSeq`] ordering stamp in one place.
///
/// The request may name a real firing actor (`req.owner != Entity::PLACEHOLDER`).
/// In that case the spawned entity also carries [`ProjectileOwner`] so downstream
/// sim-side hit routing can attribute the projectile to that actor. Ownerless or
/// enemy-faction shots still keep the opaque owner-id string for self-filtering,
/// rendering, and traces.
pub fn apply_enemy_projectile_effect_requests(
    mut commands: Commands,
    mut seq: ResMut<ProjectileSeqCounter>,
    mut requests: MessageReader<ambition_vfx::EffectRequest>,
) {
    for req in requests.read() {
        let ambition_vfx::Effect::Projectiles { shots } = &req.effect else {
            continue;
        };
        for shot in shots {
            let owner_id = shot.owner_id.clone();
            // Decode the opaque visual tag the firing site stamped into the
            // shot's visual identity. The render layer reads this component —
            // not the owner-id string — to pick the projectile art.
            let visual_kind = ProjectileVisualKind::from_tag(shot.visual_tag);
            let projectile = EnemyProjectileState::build(shot.clone());
            let mut entity = commands.spawn((
                projectile.body.kin,
                projectile.body.game,
                seq.next(),
                ProjectileOwnerId(owner_id),
                visual_kind,
                LiveProjectile,
                EnemyProjectile,
                Name::new("Enemy projectile (sim)"),
            ));
            if req.owner != Entity::PLACEHOLDER {
                entity.insert(ProjectileOwner(req.owner));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core as ae;

    use crate::{ProjectileSeq, ProjectileVisualKind};

    fn spawn_request(owner_id: &str, visual_tag: u16) -> ambition_vfx::EffectRequest {
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
                    visual_tag,
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
            .write_message(spawn_request("pca", ProjectileVisualKind::Glider.to_tag()));
        app.update();

        let mut q = app.world_mut().query_filtered::<
            (&ProjectileOwnerId, &ProjectileVisualKind, &ProjectileSeq),
            (With<EnemyProjectile>, With<LiveProjectile>),
        >();
        let rows: Vec<_> = q
            .iter(app.world())
            .map(|(owner_id, visual_kind, seq)| (owner_id.0.clone(), *visual_kind, *seq))
            .collect();
        assert_eq!(
            rows,
            vec![(
                "pca".to_string(),
                ProjectileVisualKind::Glider,
                ProjectileSeq(0),
            ),],
            "the substrate executor stamps owner id, visual kind, and deterministic sequence"
        );
    }

    #[test]
    fn effect_request_preserves_real_owner_entity_for_later_hit_attribution() {
        let mut app = App::new();
        app.add_message::<ambition_vfx::EffectRequest>();
        app.init_resource::<ProjectileSeqCounter>();
        app.add_systems(Update, apply_enemy_projectile_effect_requests);

        let owner = app.world_mut().spawn_empty().id();
        let mut req = spawn_request("boss_bolt", ProjectileVisualKind::EnemyDefault.to_tag());
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

        app.world_mut().write_message(spawn_request(
            "ownerless",
            ProjectileVisualKind::EnemyDefault.to_tag(),
        ));
        app.update();

        let mut q = app.world_mut().query_filtered::<
            Option<&ProjectileOwner>,
            (With<EnemyProjectile>, With<LiveProjectile>),
        >();
        let owner_present: Vec<_> = q.iter(app.world()).map(|owner| owner.is_some()).collect();
        assert_eq!(
            owner_present,
            vec![false],
            "placeholder effects do not fabricate an owner component"
        );
    }
}
