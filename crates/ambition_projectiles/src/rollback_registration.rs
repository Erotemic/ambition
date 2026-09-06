//! Rollback declaration owned by `ambition_projectiles`.
//!
//! This module names this domain's concrete rewindable state while the host
//! supplies the backend through [`RollbackRegistrar`]. It deliberately contains
//! no `bevy_ggrs` dependency and no host/composition logic.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register everything the projectile domain needs rewound.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    // The ANCHOR: a projectile-only entity is a rollback participant in its own
    // right — it carries no body and would otherwise be swept by nothing.
    registrar.require_rollback::<crate::LiveProjectile>(OWNER, "entity:live_projectile")
        // Resource: the deterministic id source.
        .rollback_resource_canonical::<crate::ProjectileSeqCounter>(
            OWNER,
            "resource.projectile_seq_counter",
        )
        // The firing body's own cooldown state.
        .rollback_component_canonical::<crate::PlayerProjectileState>(
            OWNER,
            "player.projectile_state",
        )
        // The bolt itself.
        .rollback_component_canonical::<crate::ProjectileSeq>(OWNER, "projectile.seq")
        .rollback_component_canonical::<crate::ProjectileVisualId>(
            OWNER,
            "projectile.visual_id",
        )
        .rollback_component_canonical::<crate::ProjectileKind>(
            OWNER,
            "projectile.kind",
        )
        .rollback_component_canonical::<crate::LiveProjectile>(
            OWNER,
            "projectile.live_marker",
        )
        // `projectile.gameplay` is intentionally absent here: its type is
        // owned by `ambition_platformer2d_shared_tangle`, whose own
        // `register_rollback_state` declares it. Moving registration authority
        // means following the TYPE owner rather than claiming a neighbour's row.
        ;

    // The owner REFERENCE, which is an entity and therefore has to be remapped
    // across a rewind rather than cloned as a number.
    registrar.rollback_component_clone_entity_ref::<crate::ProjectileOwner>(
        OWNER,
        "component.projectile_owner",
        |owner| owner.0,
    )
    .rollback_map_entities::<crate::ProjectileOwner>(OWNER, "map.projectile_owner")
    // The authoritative projectile request buffer: a request written on a tick
    // that gets rewound must not materialize on the abandoned future.
    .clear_message_on_rollback::<crate::ProjectileSpawnRequest>(
        OWNER,
        "message.spawn_projectile",
    );
}
