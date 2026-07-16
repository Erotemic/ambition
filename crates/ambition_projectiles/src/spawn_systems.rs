//! Pool-spawn system: turn a queued player-pool [`SpawnProjectile`] message into a
//! live projectile entity. Pure model work — reads the pool message and spawns
//! the crate's own components; no victim, world, or brain state. The enemy-pool
//! spawn is inlined in the enemy effect stepper (which stays sim-side).

use bevy::prelude::*;

use crate::entity::{
    LiveProjectile, PlayerProjectile, ProjectileOwner, ProjectileOwnerId, ProjectileSeqCounter,
};
use crate::spawn_message::{ProjectilePool, SpawnProjectile};
use crate::visual::ProjectileVisualId;

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
        // The named gameplay kind rides as its own component (the engine body is
        // generic): combat attribution and trace read it off the entity. The
        // OPEN visual id — the key the content art catalog registers under —
        // rides beside it, so player + enemy shots share ONE id→art selection
        // path in the render layer. A kind-less shot (shouldn't happen for the
        // player) reads as the generic fireball look.
        let visual_id = msg
            .kind
            .map(|kind| kind.visual_id().to_string())
            .unwrap_or_else(|| "fireball".to_string());
        entity.insert(ProjectileVisualId(visual_id));
        if let Some(kind) = msg.kind {
            entity.insert(kind);
        }
    }
}
