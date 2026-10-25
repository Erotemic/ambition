//! Rollback declarations for derived world state.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    crate::rooms::register_rollback_state(registrar);
    registrar
        .declare_rollback_derived_resource::<crate::gated_lock_walls::GatedLockWallCache>(
        OWNER,
        "derived.gated_lock_wall_cache",
        "authored gated walls for the active room; recomputed from the room set and LDtk project",
    );
    registrar
        .declare_rollback_derived_resource::<crate::gated_lock_walls::GatedLockWallVerdicts>(
            OWNER,
            "derived.gated_lock_wall_verdicts",
            "why each authored gated wall stands; rebuilt from the walls' verdicts every gate tick",
        );
    registrar.declare_rollback_derived_resource::<
        crate::authored_room_commands::AuthoredRoomCommands,
    >(
        OWNER,
        "derived.authored_room_commands",
        "each live room's prepared `while_live` line; recomputed from the room set and the command catalog",
    );
}
