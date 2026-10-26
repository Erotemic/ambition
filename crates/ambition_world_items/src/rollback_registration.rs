//! Register physical container state for rollback.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
const OWNER: &str = env!("CARGO_PKG_NAME");
pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    registrar.rollback_component_clone_probed::<crate::containers::FallingChest>(
        OWNER,
        "feature.falling_chest",
        |chest| chest.vel_y.to_bits() as u64,
    );
    registrar.rollback_component_clone::<crate::containers::ChestFeature>(OWNER, "feature.chest");
    registrar.rollback_component_clone::<crate::containers::Opened>(OWNER, "feature.opened");
}
