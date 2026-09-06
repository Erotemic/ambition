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
    // ⛔⛔ THE SCRIPT'S TRANSIENTS, AND THEY ARE AUTHORITATIVE. Both are attached
    // by an encounter EFFECT mid-fight, so they exist in no boot world and the
    // coverage census — which sweeps the initial world — could never ask about
    // them. Each one steers a body.
    //
    // `CommandedMove` overrides the boss's own control every tick it is present:
    // a rewind that restored the boss's position without restoring where it was
    // being walked to would send it somewhere the resimulation never chose.
    registrar.rollback_component_clone_probed::<crate::encounter_script::CommandedMove>(
        OWNER,
        "encounter.commanded_move",
        |cmd| {
            (cmd.target.x.to_bits() as u64) << 32
                ^ (cmd.target.y.to_bits() as u64)
                ^ (cmd.speed.to_bits() as u64)
        },
    );
    // ⛔⛔ AND THE CODEC IS ONLY HALF OF IT AGAIN. A hazard is spawned MID-MATCH
    // by `EncounterEffect::DropHazard` with a plain `spawn_session_scoped`, so
    // the entity carried no rollback anchor and every registration below was
    // INERT on it: the registry listed them, the coverage sweep counted them as
    // accounted, and nothing restored them. The anchor is the fact that puts the
    // ENTITY in the envelope; the clone is the fact about its bytes.
    registrar
        .require_rollback::<crate::encounter_script::FallingHazard>(OWNER, "entity:falling_hazard");
    // ⛔ AND THIS ONE NAMES AN ENTITY, so the clone is only half of it: a
    // resimulation rebuilds the world's entities and a raw id would point at
    // whoever landed in that slot. `vel_y` and `dropping` are the fall itself.
    registrar.rollback_component_clone_entity_ref::<crate::encounter_script::FallingHazard>(
        OWNER,
        "encounter.falling_hazard",
        |hazard| hazard.target,
    );
    registrar.rollback_map_entities::<crate::encounter_script::FallingHazard>(
        OWNER,
        "map.falling_hazard",
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
