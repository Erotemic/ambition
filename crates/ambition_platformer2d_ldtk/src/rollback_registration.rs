//! Rollback declaration owned by `ambition_platformer2d_ldtk`.
//!
//! This module names this domain's concrete rewindable state while the host
//! supplies the backend through [`RollbackRegistrar`]. It deliberately contains
//! no `bevy_ggrs` dependency and no host/composition logic.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
#[cfg(feature = "ldtk_runtime")]
use ambition_platformer2d_core::snapshot::checksum_bytes;

#[cfg(feature = "ldtk_runtime")]
const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register the LDtk runtime index's rollback row.
///
/// The row is gated behind `ldtk_runtime`; the function is not.
/// `LdtkRuntimeIndex` lives in `bevy_runtime`, which exists only with that
/// feature, so without the backend there is nothing to rewind. Gating the
/// function would break every host's call site.
///
/// This forks the rollback schema: a build without `ldtk_runtime` registers one
/// row fewer, so its fingerprint differs. Every shipping composition has the
/// backend, so the recorded baselines are the with-runtime set.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    #[cfg(feature = "ldtk_runtime")]
    registrar.rollback_component_clone_checksum::<crate::LdtkRuntimeIndex>(
        OWNER,
        "root.ldtk_runtime_index",
        "active LDtk area checksum",
        ldtk_runtime_index_checksum,
    );
    #[cfg(not(feature = "ldtk_runtime"))]
    let _ = registrar;
}

#[cfg(feature = "ldtk_runtime")]
fn ldtk_runtime_index_checksum(index: &crate::LdtkRuntimeIndex) -> u64 {
    checksum_bytes(index.active_area().as_bytes())
}
