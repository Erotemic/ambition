//! Rollback declaration owned by `ambition_boss_encounter`.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.rollback_component_cursor::<crate::BossEncounter>(OWNER, "boss.encounter");
    registrar.rollback_component_clone::<crate::BossConfig>(OWNER, "boss.config");
    registrar.rollback_component_clone::<crate::BossOverrides>(OWNER, "boss.overrides");
    registrar.rollback_component_clone::<crate::EncounterDef>(OWNER, "encounter.definition");
    registrar.rollback_component_cursor::<crate::sprites::BossAnimFrame>(
        OWNER,
        "component.boss_anim_frame",
    );
    registrar.declare_rollback_derived_component::<crate::EncounterProgress>(
        OWNER,
        "derived.encounter_progress",
        "recomputed from lifecycle and participant health every tick",
    );
    registrar.clear_message_on_rollback::<crate::PayloadReleased>(
        OWNER,
        "message.payload_released",
    );
    // Phase transition is a same-frame simulation handshake. A stale reader
    // cursor would replay presentation/gameplay feedback from an abandoned branch.
    registrar.clear_message_on_rollback::<crate::BossPhaseChanged>(
        OWNER,
        "message.boss_phase_changed",
    );
}
