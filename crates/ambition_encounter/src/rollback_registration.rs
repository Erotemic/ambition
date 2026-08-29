//! Rollback declaration owned by `ambition_encounter`.
//!
//! This module names this domain's concrete rewindable state while the host
//! supplies the backend through [`RollbackRegistrar`]. It deliberately contains
//! no `bevy_ggrs` dependency and no host/composition logic.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register everything the encounter domain needs rewound.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.require_rollback::<crate::EncounterLifecycle>(OWNER, "entity:encounter_lifecycle");
    registrar.rollback_component_clone::<crate::EncounterMusicRequest>(
        OWNER,
        "root.encounter_music_request",
    );
    registrar.rollback_resource_clone_entity_set::<crate::EncounterRegistry>(
        OWNER,
        "resource.encounter_registry",
        |registry| registry.ids.values().copied().collect(),
    );
    registrar.rollback_resource_map_entities::<crate::EncounterRegistry>(
        OWNER,
        "map.resource.encounter_registry",
    );
    registrar.rollback_component_clone::<crate::Encounter>(OWNER, "encounter.identity");
    registrar.rollback_component_clone::<crate::EncounterObjective>(OWNER, "encounter.objective");
    registrar
        .rollback_component_clone::<crate::EncounterCameraZoom>(OWNER, "encounter.camera_zoom");
    registrar.rollback_component_clone::<crate::EncounterLockWall>(OWNER, "encounter.lock_wall");
    registrar.rollback_component_clone::<crate::EncounterTrack>(OWNER, "encounter.track");
    registrar
        .rollback_component_canonical::<crate::EncounterLifecycle>(OWNER, "encounter.lifecycle");
    registrar.rollback_component_clone_state::<crate::EncounterParticipants>(
        OWNER,
        "encounter.participants",
    );
    registrar
        .rollback_map_entities::<crate::EncounterParticipants>(OWNER, "map.encounter_participants");
    registrar.rollback_component_resolved::<crate::EncounterWaves>(OWNER, "encounter.waves");
    registrar.declare_rollback_derived_resource::<crate::entity::EncounterView>(
        OWNER,
        "derived.encounter_view",
        "presentation-intent read model republished each tick",
    );
    registrar
        .clear_message_on_rollback::<crate::EncounterCommand>(OWNER, "message.encounter_command");
    registrar
        .clear_message_on_rollback::<crate::EncounterEventMsg>(OWNER, "message.encounter_event");
    registrar.clear_message_on_rollback::<crate::EncounterGate>(OWNER, "message.encounter_gate");
    registrar.clear_message_on_rollback::<crate::timeline::EncounterGate>(
        OWNER,
        "message.encounter_gate",
    );
    // ⭐ THE SWITCH STATE, MOVED HERE 2026-08-26 WITH THE TYPES IT DECLARES.
    // This is the fourth thing a type move owes — the type, its consumers, any
    // orphan-rule impls, and THE DECLARATION — and it is the only one nothing
    // catches: a declaration compiles perfectly well in the crate the type
    // left. ⛔ the STABLE NAMES are unchanged, so the wire did not move; only
    // the OWNER string did, from the monolith to the crate that defines them.
    // ⭐ CHECKSUMMED 2026-08-29. This type's own doc names the hazard it was
    // registered for — "a rewind keeps predicted activations and resimulation
    // pushes them again, double-applying an encounter reset" — and
    // `rollback_resource_clone` prevents that (it restores) while installing a
    // PRESENCE-ONLY probe, so the sync test could not SEE the case the author
    // was worried about. A queue is the sharpest form of this: presence cannot
    // distinguish one entry from five.
    registrar.rollback_resource_clone_checksum::<crate::switches::SwitchActivationQueue>(
        OWNER,
        "resource.switch_activation_queue",
        "queued switch activations, in order, by id/action/target",
        crate::switches::SwitchActivationQueue::checksum,
    );
    registrar.rollback_component_clone_probed::<crate::switches::SwitchOn>(
        OWNER,
        "feature.switch_on",
        |on| u64::from(on.0),
    );
    registrar.rollback_component_clone::<crate::switches::SwitchFeature>(OWNER, "feature.switch");
    registrar.clear_message_on_rollback::<crate::switches::SwitchActivated>(
        OWNER,
        "message.switch_activated",
    );
    registrar.declare_rollback_derived_resource::<crate::switches::EncounterSwitchIndex>(
        OWNER,
        "derived.encounter_switch_index",
        "rebuilt from SwitchFeature + SwitchOn components each frame",
    );
}
