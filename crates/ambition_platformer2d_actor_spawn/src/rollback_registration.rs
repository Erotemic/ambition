//! Rollback declarations for state owned by this crate.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register this domain's rollback state.
pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    registrar.rollback_component_clone_probed::<crate::ProjectedCharacterKit>(
        OWNER,
        "actor.projected_character_kit",
        |projected| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            projected.id.hash(&mut hasher);
            projected.granted.hurtboxes.hash(&mut hasher);
            projected.granted.movement_tuning.hash(&mut hasher);
            projected.granted.unmirrored.hash(&mut hasher);
            projected.granted.drawn_quad.hash(&mut hasher);
            projected
                .granted
                .posed_body
                .map(|displaced| {
                    [displaced.base_size, displaced.render_size, displaced.sprite_offset]
                        .map(|v| v.map(|v| (v.x.to_bits(), v.y.to_bits())))
                })
                .hash(&mut hasher);
            hasher.finish() ^ projected.generation.get().rotate_left(32)
        },
    );
    registrar.clear_message_on_rollback::<crate::SpawnActorRequest>(
        OWNER,
        "message.spawn_actor_request",
    );

}
