//! Rollback declarations for derived world state.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    registrar.declare_rollback_derived_resource::<
        crate::authored_switch_commands::AuthoredSwitchCommands,
    >(
        OWNER,
        "derived.authored_switch_commands",
        "authored switch verbs prepared for the active room; recomputed from the room set and LDtk project",
    );
}
