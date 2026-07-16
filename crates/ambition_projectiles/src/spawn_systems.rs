//! Pool-spawn system: turn a queued player-pool [`SpawnProjectile`] message into a
//! live projectile entity. Pure model work — reads the pool message and spawns
//! the crate's own components; no victim, world, or brain state. The enemy-pool
//! spawn is inlined in the enemy effect stepper (which stays sim-side).

use bevy::prelude::*;

use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, RoomScopedEntity, SessionSpawnScope,
};

use crate::entity::{
    LiveProjectile, PlayerProjectile, ProjectileOwner, ProjectileOwnerId, ProjectileSeqCounter,
};
use crate::spawn_message::{ProjectilePool, SpawnProjectile};
use crate::visual::ProjectileVisualId;

/// Drain queued player-pool spawn messages into live projectile entities.
///
/// A projectile is ROOM- and SESSION-scoped like the rest of a room's spawns: a
/// shot exists in its room's space (a room transition or restore staging sweeps
/// it; the snapshot's own shots come back from blobs), and it must not outlive
/// its session into a successor's world. At the frontend (a session scope with
/// no current session) gameplay spawning sleeps, matching the actor applier.
pub fn apply_player_spawn_projectile_messages(
    mut commands: Commands,
    mut seq: ResMut<ProjectileSeqCounter>,
    mut spawn_projectiles: MessageReader<SpawnProjectile>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    let Some(scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        spawn_projectiles.clear();
        return;
    };
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
            RoomScopedEntity,
            Name::new("Player projectile (sim)"),
        ));
        scope.apply_to(&mut entity);
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
