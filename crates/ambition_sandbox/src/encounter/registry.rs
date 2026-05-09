use std::collections::BTreeMap;

use bevy::prelude::*;

use super::{EncounterPhase, EncounterState};

/// Multi-encounter registry. Keyed by encounter id (matching the
/// `EncounterTrigger.id` field in LDtk). Replaces the older
/// single-encounter `Res<EncounterState>` so the sandbox can carry
/// more than one encounter at once.
#[derive(Resource, Default)]
pub struct EncounterRegistry {
    pub encounters: BTreeMap<String, EncounterState>,
    /// Tracks whether the current LDtk file has been scanned for
    /// encounter triggers yet. Reset by hot reload so an edited LDtk
    /// re-populates the specs.
    pub specs_loaded: bool,
}

impl EncounterRegistry {
    pub fn get(&self, id: &str) -> Option<&EncounterState> {
        self.encounters.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut EncounterState> {
        self.encounters.get_mut(id)
    }

    pub fn ensure(&mut self, id: &str) -> &mut EncounterState {
        self.encounters.entry(id.to_string()).or_default()
    }

    /// True if any encounter is currently locking exits.
    pub fn any_lock_active(&self) -> bool {
        self.encounters.values().any(|e| e.lock_active)
    }

    /// Camera zoom multiplier sourced from the active encounter (if
    /// any). 1.0 if no encounter is in flight. The camera starts
    /// zooming during `Starting` so the ramp finishes before wave 1
    /// spawns.
    pub fn active_camera_zoom(&self) -> f32 {
        for state in self.encounters.values() {
            if matches!(
                state.phase,
                EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
            ) {
                if let Some(spec) = &state.spec {
                    if spec.camera_zoom > 1.0 {
                        return spec.camera_zoom;
                    }
                }
            }
        }
        1.0
    }
}

/// One activation request from a switch interaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchActivation {
    pub id: String,
    pub action: String,
    pub target_encounter: String,
}

impl SwitchActivation {
    /// Parse the `Custom("switch:<id>:<action>:<target>")` payload
    /// produced by `entity_to_runtime` for `Switch` LDtk entities.
    pub fn parse_custom(payload: &str) -> Option<Self> {
        let mut parts = payload.split(':');
        if parts.next()? != "switch" {
            return None;
        }
        let id = parts.next()?.to_string();
        let action = parts.next()?.to_string();
        let target_encounter = parts.next().unwrap_or("").to_string();
        Some(Self {
            id,
            action,
            target_encounter,
        })
    }
}

/// Marker component for the per-encounter seldom_state controller
/// entity. The encounter system spawns one per registered encounter
/// and keeps its sparse-set state component (`EncounterDormant`,
/// `EncounterActive`, `EncounterCleared`, `EncounterFailed`) in sync
/// with the registry's phase. HUD / debug systems can query by state
/// component without touching the resource.
#[derive(Component, Clone, Debug)]
pub struct EncounterController {
    pub encounter_id: String,
}

