//! Rollback declarations for state owned by this crate.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register this domain's rollback state.
pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    registrar.rollback_component_cursor::<crate::ActorMotionPath>(
        OWNER,
        "actor.motion_path",
    );
    registrar.rollback_component_clone_probed::<crate::PersonaBaseline>(
        OWNER,
        "actor.persona_baseline",
        |baseline| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            baseline.id.hash(&mut hasher);
            // The STANDING physicals are in the probe for the same reason
            // `ProjectedCharacterKit.granted` is: retraction is driven from them,
            // so a rewind that restores the record without them retracts a
            // replacement to the wrong numbers and nothing reads wrong until a
            // character swap. Mass is hashed by its bit pattern — this is a
            // checksum, not an arithmetic comparison, so `to_bits` is exactly
            // right and `f32`'s missing `Hash` is not an obstacle.
            baseline.displaced.max_health.hash(&mut hasher);
            baseline
                .displaced
                .mass
                .map(|mass| mass.map(f32::to_bits))
                .hash(&mut hasher);
            hasher.finish() ^ baseline.generation.get().rotate_left(32)
        },
    );

}
