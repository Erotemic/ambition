//! **The LDtk world's rollback schema — registered by the game that HAS one.**
//!
//! ⛔ every other module in this directory is called by
//! `register_engine_rollback_state`, because every other domain is something
//! every platformer has. This one is not: an LDtk world is an authoring-format
//! installation, and until 2026-08-16 `root.ldtk_runtime_index` sat in
//! [`super::actors`] and therefore in the wire format of five games that never
//! install an LDtk world.
//!
//! [`crate::ldtk_world::LdtkWorldPlugin`] is the only caller — the composition
//! statement *"this game has an LDtk world"*. The registration itself is
//! unchanged from its actors-domain form (same name, same kind, same
//! projection), so an LDtk-authored composition's schema dump is byte-identical.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;
use ambition_platformer2d_core::snapshot::checksum_bytes;

const OWNER: &str = "ambition_platformer2d_runtime";

/// Register the LDtk runtime index's rollback row.
pub(crate) fn register(app: &mut App) {
    app.rollback_component_clone_checksum::<ambition_platformer2d_ldtk::LdtkRuntimeIndex>(
        OWNER,
        "root.ldtk_runtime_index",
        "bevy_ggrs clone snapshot + active LDtk area checksum",
        ldtk_runtime_index_checksum,
    );
}

fn ldtk_runtime_index_checksum(index: &ambition_platformer2d_ldtk::LdtkRuntimeIndex) -> u64 {
    checksum_bytes(index.active_area().as_bytes())
}
