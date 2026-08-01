//! **The vfx domain's rollback schema** (Campaign 2, R3).
//!
//! Hitboxes, their lifetimes and hit sets, and the effect-request buffers a rewound tick must not replay. The strike volume a move spawns is a VFX entity here, which is why it lands in this domain rather than combat's.
//!
//! ⚠ **relocation only.** The registrations were extracted mechanically and the
//! schema baseline verifies the result is byte-identical — a retyped call is
//! exactly the mistake that would slip through review and not through the
//! baseline.
//!
//! ⚠ the owner label stays `ambition_platformer2d_runtime` because this module is in it, and
//! must be: `ambition_vfx` sits below the runtime in the crate graph. R1's
//! recorded decision is that this is the right shape for every domain below the
//! runtime; crates above it own their schemas directly.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;

const OWNER: &str = "ambition_platformer2d_runtime";

/// Register everything the vfx domain needs rewound.
pub(in crate::rollback) fn register(app: &mut App) {
    app.require_rollback::<ambition_vfx::Hitbox>(OWNER, "entity:hitbox");
    app.rollback_component_clone_entity_ref::<ambition_vfx::Hitbox>(
        OWNER,
        "combat.hitbox",
        |hitbox| hitbox.owner,
    );
    app.rollback_map_entities::<ambition_vfx::Hitbox>(OWNER, "map.hitbox");
    app.rollback_component_clone_entity_set::<ambition_vfx::HitboxHits>(
        OWNER,
        "combat.hitbox_hits",
        |hits| hits.hit.iter().copied().collect(),
    );
    app.rollback_map_entities::<ambition_vfx::HitboxHits>(OWNER, "map.hitbox_hits");
    app.rollback_component_clone_probed::<ambition_vfx::HitboxLifetime>(
        OWNER,
        "combat.hitbox_lifetime",
        |lifetime| lifetime.remaining_s.to_bits() as u64,
    );
    app.clear_message_on_rollback::<ambition_vfx::EffectRequest>(OWNER, "message.effect_request");
    app.clear_message_on_rollback::<ambition_vfx::vfx::DebrisBurstMessage>(
        OWNER,
        "message.debris_burst",
    );
    app.clear_message_on_rollback::<ambition_vfx::ExplosionRequest>(
        OWNER,
        "message.explosion_request",
    );
    app.clear_message_on_rollback::<ambition_vfx::FireworksRequest>(
        OWNER,
        "message.fireworks_request",
    );
    app.clear_message_on_rollback::<ambition_vfx::VfxMessage>(OWNER, "message.vfx");
}
