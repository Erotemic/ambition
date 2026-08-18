//! Rollback declaration owned by `ambition_platformer2d_ldtk`.
//!
//! This module names this domain's concrete rewindable state while the host
//! supplies the backend through [`RollbackRegistrar`]. It deliberately contains
//! no `bevy_ggrs` dependency and no host/composition logic.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
use ambition_platformer2d_core::snapshot::checksum_bytes;

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register the LDtk runtime index's rollback row.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.rollback_component_clone_checksum::<crate::LdtkRuntimeIndex>(
        OWNER,
        "root.ldtk_runtime_index",
        "active LDtk area checksum",
        ldtk_runtime_index_checksum,
    );
}

fn ldtk_runtime_index_checksum(index: &crate::LdtkRuntimeIndex) -> u64 {
    checksum_bytes(index.active_area().as_bytes())
}
