//! **The encounter domain's rollback schema** (Campaign 2, R3).
//!
//! Encounter lifecycle and its authorities: what a rewind has to put back about a fight in progress.
//!
//! ⚠ **relocation only.** The registrations were extracted mechanically and the
//! schema baseline verifies the result is byte-identical — a retyped call is
//! exactly the mistake that would slip through review and not through the
//! baseline.
//!
//! ⚠ the owner label stays `ambition_runtime` because this module is in it, and
//! must be: `ambition_encounter` sits below the runtime in the crate graph. R1's
//! recorded decision is that this is the right shape for every domain below the
//! runtime; crates above it own their schemas directly.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;

const OWNER: &str = "ambition_runtime";

/// Register everything the encounter domain needs rewound.
pub(in crate::rollback) fn register(app: &mut App) {
    app.require_rollback::<ambition_encounter::EncounterLifecycle>(
        OWNER,
        "entity:encounter_lifecycle",
    );
    app.rollback_component_clone::<ambition_encounter::EncounterMusicRequest>(
        OWNER,
        "root.encounter_music_request",
    );
    app.rollback_resource_clone_entity_set::<ambition_encounter::EncounterRegistry>(
        OWNER,
        "resource.encounter_registry",
        |registry| registry.ids.values().copied().collect(),
    );
    app.rollback_resource_map_entities::<ambition_encounter::EncounterRegistry>(
        OWNER,
        "map.resource.encounter_registry",
    );
    app.rollback_component_clone::<ambition_encounter::Encounter>(OWNER, "encounter.identity");
    app.rollback_component_clone::<ambition_encounter::EncounterObjective>(
        OWNER,
        "encounter.objective",
    );
    app.rollback_component_clone::<ambition_encounter::EncounterCameraZoom>(
        OWNER,
        "encounter.camera_zoom",
    );
    app.rollback_component_clone::<ambition_encounter::EncounterLockWall>(
        OWNER,
        "encounter.lock_wall",
    );
    app.rollback_component_clone::<ambition_encounter::EncounterTrack>(OWNER, "encounter.track");
    app.rollback_component_canonical::<ambition_encounter::EncounterLifecycle>(
        OWNER,
        "encounter.lifecycle",
    );
    app.rollback_component_clone_state::<ambition_encounter::EncounterParticipants>(
        OWNER,
        "encounter.participants",
    );
    app.rollback_map_entities::<ambition_encounter::EncounterParticipants>(
        OWNER,
        "map.encounter_participants",
    );
    app.rollback_component_resolved::<ambition_encounter::EncounterWaves>(OWNER, "encounter.waves");
    app.declare_rollback_derived_resource::<ambition_encounter::entity::EncounterView>(
        OWNER,
        "derived.encounter_view",
        "presentation-intent read model republished each tick",
    );
    app.clear_message_on_rollback::<ambition_encounter::EncounterCommand>(
        OWNER,
        "message.encounter_command",
    );
    app.clear_message_on_rollback::<ambition_encounter::EncounterEventMsg>(
        OWNER,
        "message.encounter_event",
    );
    app.clear_message_on_rollback::<ambition_encounter::EncounterGate>(
        OWNER,
        "message.encounter_gate",
    );
    app.clear_message_on_rollback::<ambition_encounter::timeline::EncounterGate>(
        OWNER,
        "message.encounter_gate",
    );
}
