//! Register domain state for rollback.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
const OWNER: &str = env!("CARGO_PKG_NAME");
pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    registrar.rollback_component_clone_probed::<crate::RespawnTimer>(
        OWNER,
        "feature.respawn_timer",
        |timer| timer.0.to_bits() as u64,
    );
    registrar.rollback_component_clone_checksum::<crate::WorldTimeSchedule>(
        OWNER,
        "feature.world_time_schedule",
        "when each broken breakable respawns and each collected pickup regrows, on the session clock, and whose horizons own it, kept when their room retires (OW5, Q152, Q151)",
        crate::WorldTimeSchedule::checksum,
    );
}
