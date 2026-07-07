//! Pool-spawn system: turn a queued player-pool [`SpawnProjectile`] message into a
//! live projectile entity. Pure model work — reads the pool message and spawns
//! the crate's own components; no victim, world, or brain state. The enemy-pool
//! spawn is inlined in the enemy effect stepper (which stays sim-side).

use bevy::prelude::*;

use crate::entity::{
    LiveProjectile, PlayerProjectile, ProjectileOwner, ProjectileOwnerId, ProjectileSeqCounter,
};
use crate::spawn_message::{ProjectilePool, SpawnProjectile};
use crate::visual_kind::ProjectileVisualKind;

/// Drain queued player-pool spawn messages into live projectile entities.
pub fn apply_player_spawn_projectile_messages(
    mut commands: Commands,
    mut seq: ResMut<ProjectileSeqCounter>,
    mut spawn_projectiles: MessageReader<SpawnProjectile>,
) {
    for msg in spawn_projectiles.read() {
        let ProjectilePool::Player { owner } = msg.pool else {
            continue;
        };
        let body = &msg.projectile.body;
        let mut entity = commands.spawn((
            body.kin,
            body.game,
            ProjectileOwner(owner),
            seq.next(),
            ProjectileOwnerId(msg.projectile.owner_id.clone()),
            LiveProjectile,
            PlayerProjectile,
            Name::new("Player projectile (sim)"),
        ));
        // Named kind rides as its own component (the engine body is generic):
        // combat attribution, trace, and render read it off the entity. Every
        // player shot also carries a visual identity (a kind-less shot — which
        // shouldn't happen for the player — reads as a fireball), so player +
        // enemy shots share ONE kind→art selection path in the render layer.
        let visual = msg
            .kind
            .map(ProjectileVisualKind::from)
            .unwrap_or(ProjectileVisualKind::Fireball);
        entity.insert(visual);
        if let Some(kind) = msg.kind {
            entity.insert(kind);
        }
    }
}
