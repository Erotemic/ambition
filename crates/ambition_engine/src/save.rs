//! Save-game data shapes shared by sandbox and future story crates.
//!
//! These types are pure data + `serde`. They don't know about Bevy, file
//! paths, autosave timing, or the LDtk layer — those live in the sandbox
//! crate (`crate::save` over there) so the engine stays free of I/O and
//! presentation concerns.
//!
//! The engine owns the *vocabulary* so reusable mechanics (encounter
//! defeat, switch latch, ability flags) keep one canonical shape across
//! game, sandbox, and editor tooling.

use serde::{Deserialize, Serialize};

/// One persisted encounter (e.g. mob lab) entry. Only the terminal /
/// in-progress states matter for save reconstruction; `Inactive`
/// reconstructs to "fresh attempt available" without needing an entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedEncounter {
    pub id: String,
    pub state: PersistedEncounterState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistedEncounterState {
    /// Default for an encounter that has never been triggered, or one
    /// that was reset via a switch. Not usually written to disk —
    /// missing entries reconstruct to this value.
    #[default]
    Untouched,
    /// Cleared all waves. Surviving terminal state.
    Cleared,
    /// Player died. Resets back to `Untouched` on switch reset; written
    /// so a save mid-attempt restores meaningfully.
    Failed,
}

impl PersistedEncounter {
    pub fn new(id: impl Into<String>, state: PersistedEncounterState) -> Self {
        Self {
            id: id.into(),
            state,
        }
    }
}

/// One latched switch entry. Today the sandbox uses these to track
/// "encounter reset switch outside the room"; future puzzle / door
/// switches reuse the same shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedSwitch {
    pub id: String,
    pub on: bool,
}

impl PersistedSwitch {
    pub fn new(id: impl Into<String>, on: bool) -> Self {
        Self {
            id: id.into(),
            on,
        }
    }
}

/// Top-level sandbox save. Versioned so a future schema change can
/// migrate or refuse to load gracefully.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxSaveData {
    #[serde(default = "default_save_version")]
    pub version: u32,
    #[serde(default)]
    pub encounters: Vec<PersistedEncounter>,
    #[serde(default)]
    pub switches: Vec<PersistedSwitch>,
}

pub const CURRENT_SAVE_VERSION: u32 = 1;

fn default_save_version() -> u32 {
    CURRENT_SAVE_VERSION
}

impl SandboxSaveData {
    pub fn new() -> Self {
        Self {
            version: CURRENT_SAVE_VERSION,
            encounters: Vec::new(),
            switches: Vec::new(),
        }
    }

    /// Look up an encounter's state. Missing entries reconstruct to
    /// `Untouched`, matching the wire format default.
    pub fn encounter(&self, id: &str) -> PersistedEncounterState {
        self.encounters
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.state)
            .unwrap_or_default()
    }

    /// Set an encounter's state. Inserts a new entry if needed; replaces
    /// existing. Encounters that fall back to `Untouched` are removed
    /// from the list to keep the save file compact.
    pub fn set_encounter(&mut self, id: impl Into<String>, state: PersistedEncounterState) {
        let id = id.into();
        if matches!(state, PersistedEncounterState::Untouched) {
            self.encounters.retain(|e| e.id != id);
            return;
        }
        if let Some(existing) = self.encounters.iter_mut().find(|e| e.id == id) {
            existing.state = state;
        } else {
            self.encounters.push(PersistedEncounter { id, state });
        }
    }

    pub fn switch(&self, id: &str) -> bool {
        self.switches
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.on)
            .unwrap_or(false)
    }

    pub fn set_switch(&mut self, id: impl Into<String>, on: bool) {
        let id = id.into();
        if let Some(existing) = self.switches.iter_mut().find(|s| s.id == id) {
            existing.on = on;
        } else {
            self.switches.push(PersistedSwitch { id, on });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_encounter_reads_untouched() {
        let s = SandboxSaveData::default();
        assert_eq!(s.encounter("mob_lab"), PersistedEncounterState::Untouched);
    }

    #[test]
    fn setting_encounter_round_trips() {
        let mut s = SandboxSaveData::new();
        s.set_encounter("mob_lab", PersistedEncounterState::Cleared);
        assert_eq!(s.encounter("mob_lab"), PersistedEncounterState::Cleared);
        // Resetting to untouched removes the entry to keep the save compact.
        s.set_encounter("mob_lab", PersistedEncounterState::Untouched);
        assert!(s.encounters.is_empty());
    }

    #[test]
    fn switch_defaults_to_off() {
        let s = SandboxSaveData::default();
        assert!(!s.switch("reset_switch"));
    }

    #[test]
    fn setting_switch_round_trips() {
        let mut s = SandboxSaveData::new();
        s.set_switch("reset_switch", true);
        assert!(s.switch("reset_switch"));
        s.set_switch("reset_switch", false);
        assert!(!s.switch("reset_switch"));
        assert_eq!(s.switches.len(), 1);
    }

    #[test]
    fn serde_round_trip_preserves_fields() {
        let mut s = SandboxSaveData::new();
        s.set_encounter("mob_lab", PersistedEncounterState::Cleared);
        s.set_encounter("boss_room", PersistedEncounterState::Failed);
        s.set_switch("reset_switch", true);
        let serialized = serde_json::to_string(&s).expect("serialize");
        let restored: SandboxSaveData = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(s, restored);
    }

    #[test]
    fn deserialize_missing_version_uses_current() {
        let json = r#"{"encounters":[],"switches":[]}"#;
        let s: SandboxSaveData = serde_json::from_str(json).expect("parse");
        assert_eq!(s.version, CURRENT_SAVE_VERSION);
    }
}
