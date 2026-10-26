//! Register domain state for rollback.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
const OWNER: &str = env!("CARGO_PKG_NAME");
pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    registrar.rollback_component_clone::<crate::BreakableFeature>(OWNER, "feature.breakable");
    registrar.rollback_component_clone_probed::<crate::StandTimer>(
        OWNER,
        "feature.stand_timer",
        |timer| timer.0.to_bits() as u64,
    );
}
