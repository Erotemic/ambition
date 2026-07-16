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
    LiveProjectile, ProjectileOwner, ProjectileOwnerId, ProjectileSeqCounter, ProjectileVisualId,
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
            // Carry the open visual id the firing site stamped forward onto the
            // entity. The render layer reads this component — not the owner-id
            // string — and resolves it through the content art catalog. An empty
            // id reads as the generic hostile shot. This crate never names one.
            let visual_id = ProjectileVisualId(shot.visual_id.clone());
            let projectile = EnemyProjectileState::build(shot.clone());
            let mut entity = commands.spawn((
                projectile.body.kin,
                projectile.body.game,
                seq.next(),
                ProjectileOwnerId(owner_id),
                visual_id,
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
mod tests;
