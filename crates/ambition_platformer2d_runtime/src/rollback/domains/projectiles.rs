//! **The projectile domain's rollback schema** (Campaign 2, R3 — the first
//! domain migrated).
//!
//! Chosen first for being the smallest domain that is genuinely one: 15
//! registrations, no reverse dependency on the runtime, and a state model
//! (a bolt in flight, its owner, its sequence) that nothing else writes.
//!
//! ⚠ **this is a RELOCATION and nothing else.** R3: *"preserve registration
//! order and projections; verify the resulting schema fingerprint is unchanged.
//! Do not strengthen probes or alter snapshot behavior in the same commit as the
//! relocation."* `rollback_schema_baseline` is what verifies it — the schema dump
//! is byte-identical across this move, and it names the exact line if it is not.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;

/// The owner label these registrations carry.
///
/// ⚠ still `ambition_platformer2d_runtime`, deliberately, and it is not a lie: this module IS
/// in `ambition_platformer2d_runtime`, because the dependency direction says it must be until
/// R1 extracts the schema vocabulary. Labelling it `ambition_projectiles` today
/// would claim an ownership the crate graph does not support.
///
/// It is also no longer a wire-format fact — `schema_fingerprint` stopped
/// hashing the owner on 2026-07-31, precisely so this campaign can move
/// registrations without declaring peers incompatible.
const OWNER: &str = "ambition_platformer2d_runtime";

/// Register everything the projectile domain needs rewound.
pub(in crate::rollback) fn register(app: &mut App) {
    // The ANCHOR: a projectile-only entity is a rollback participant in its own
    // right — it carries no body and would otherwise be swept by nothing.
    app.require_rollback::<ambition_projectiles::LiveProjectile>(OWNER, "entity:live_projectile")
        // Resources: the deterministic id source, and the enemy pool's cadence.
        .rollback_resource_canonical::<ambition_projectiles::ProjectileSeqCounter>(
            OWNER,
            "resource.projectile_seq_counter",
        )
        .rollback_resource_canonical::<ambition_projectiles::enemy::EnemyProjectileState>(
            OWNER,
            "resource.enemy_projectile_state",
        )
        // The firing body's own cooldown state.
        .rollback_component_canonical::<ambition_projectiles::PlayerProjectileState>(
            OWNER,
            "player.projectile_state",
        )
        // The bolt itself.
        .rollback_component_canonical::<ambition_projectiles::ProjectileSeq>(OWNER, "projectile.seq")
        .rollback_component_canonical::<ambition_projectiles::ProjectileOwnerId>(
            OWNER,
            "projectile.owner_id",
        )
        .rollback_component_canonical::<ambition_projectiles::ProjectileVisualId>(
            OWNER,
            "projectile.visual_id",
        )
        .rollback_component_canonical::<ambition_projectiles::ProjectileKind>(
            OWNER,
            "projectile.kind",
        )
        .rollback_component_canonical::<ambition_projectiles::LiveProjectile>(
            OWNER,
            "projectile.live_marker",
        )
        .rollback_component_canonical::<ambition_projectiles::PlayerProjectile>(
            OWNER,
            "projectile.player_marker",
        )
        .rollback_component_canonical::<ambition_projectiles::enemy::EnemyProjectile>(
            OWNER,
            "projectile.enemy_marker",
        )
        // ⚠ `projectile.gameplay` stays central: its type is
        // `ambition_platformer2d_shared_tangle::projectile::ProjectileGameplay`, which
        // belongs to the primitives crate rather than this domain. Moving a
        // registration is not the same as moving a TYPE, and a domain adapter
        // that claims another crate's type is the borrowed-authority bug again.
        ;

    // The owner REFERENCE, which is an entity and therefore has to be remapped
    // across a rewind rather than cloned as a number.
    app.rollback_component_clone_entity_ref::<ambition_projectiles::ProjectileOwner>(
        OWNER,
        "component.projectile_owner",
        |owner| owner.0,
    )
    .rollback_map_entities::<ambition_projectiles::ProjectileOwner>(OWNER, "map.projectile_owner")
    // The spawn request buffer: a message written on a tick that gets rewound
    // must not materialize a bolt on the resimulated one.
    //
    // ⚠ the central function registered this TWICE — once as
    // `ambition_projectiles::SpawnProjectile` and once as
    // `ambition_projectiles::spawn_message::SpawnProjectile`, which are the same
    // type through two paths. The registry deduplicates identical descriptors,
    // so it was invisible and harmless; collecting the domain in one place is
    // what made it visible.
    .clear_message_on_rollback::<ambition_projectiles::SpawnProjectile>(
        OWNER,
        "message.spawn_projectile",
    );
}
