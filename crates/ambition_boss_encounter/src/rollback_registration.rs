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
    // The script's transients. Both are attached by an encounter effect
    // mid-fight, so they are in no boot world, and the coverage census (which
    // sweeps the initial world) cannot see them. Each one steers a body.
    //
    // `CommandedMove` overrides the boss's control every tick it is present.
    // A rewind that restored the boss's position but not its walk target
    // would send it where the resimulation never chose.
    registrar.rollback_component_clone_probed::<crate::encounter_script::CommandedMove>(
        OWNER,
        "encounter.commanded_move",
        |cmd| {
            (cmd.target.x.to_bits() as u64) << 32
                ^ (cmd.target.y.to_bits() as u64)
                ^ (cmd.speed.to_bits() as u64)
        },
    );
    // A hazard is spawned mid-match by `EncounterEffect::DropHazard`, so it
    // needs a rollback anchor as well as the codecs; without the anchor the
    // registrations below are inert on it. The anchor puts the entity in the
    // envelope; the clone covers its bytes.
    registrar
        .require_rollback::<crate::encounter_script::FallingHazard>(OWNER, "entity:falling_hazard");
    // This one names an entity, so the clone is not enough: a resimulation
    // rebuilds entities, and a raw id would point at whatever is in that slot.
    // `vel_y` and `dropping` are the fall itself.
    registrar.rollback_component_clone_entity_ref::<crate::encounter_script::FallingHazard>(
        OWNER,
        "encounter.falling_hazard",
        |hazard| hazard.target,
    );
    registrar.rollback_map_entities::<crate::encounter_script::FallingHazard>(
        OWNER,
        "map.falling_hazard",
    );

    // The latch and its message must both be registered.
    // `release_payloads_on_death` emits `PayloadReleased` and then removes this
    // marker, so the marker's absence stops a second emission. The message is
    // cleared on rollback so a resimulation can emit it again, which needs the
    // marker restored too. Otherwise a rewind across the kill frame loses the
    // release.
    registrar.rollback_component_clone::<crate::ReleaseOnDeath>(OWNER, "encounter.release_on_death");
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
