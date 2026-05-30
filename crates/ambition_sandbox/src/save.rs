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

/// One persisted encounter (e.g. goblin encounter) entry. Only the terminal /
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
        Self { id: id.into(), on }
    }
}

/// One persisted boss defeat record.
///
/// The terminal state is the same vocabulary as encounters
/// (`Cleared`/`Failed`) so save UIs can render bosses and encounters
/// uniformly. A "phase reached" snapshot would be a separate type;
/// today we only persist the terminal outcome to keep the schema
/// flat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedBossDefeat {
    pub id: String,
    pub state: PersistedEncounterState,
}

impl PersistedBossDefeat {
    pub fn new(id: impl Into<String>, state: PersistedEncounterState) -> Self {
        Self {
            id: id.into(),
            state,
        }
    }
}

/// Persisted progress for a single quest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedQuest {
    pub id: String,
    pub state: PersistedQuestState,
    /// Index of the active step (0-based). Ignored for `NotStarted` /
    /// `Completed` / `Failed` but kept on the wire so the save can
    /// remember mid-quest progress.
    #[serde(default)]
    pub step: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistedQuestState {
    #[default]
    NotStarted,
    InProgress,
    Completed,
    Failed,
}

impl PersistedQuest {
    pub fn new(id: impl Into<String>, state: PersistedQuestState, step: u8) -> Self {
        Self {
            id: id.into(),
            state,
            step,
        }
    }
}

/// A named on/off world flag. Used for "cutscene_X_seen",
/// "npc_Y_hostile", "tutorial_Z_complete" and other one-shot facts
/// that don't fit the encounter / switch / quest vocabularies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedFlag {
    pub id: String,
    pub on: bool,
}

impl PersistedFlag {
    pub fn new(id: impl Into<String>, on: bool) -> Self {
        Self { id: id.into(), on }
    }
}

/// Per-dialogue visit counter. Incremented every time the sandbox's
/// dialog runner enters the named node. Read by the Yarn binding
/// `visit_count(npc_id)` so authored dialogue can branch on
/// first-time vs. repeat encounters without a per-NPC flag.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedDialogVisit {
    pub id: String,
    pub count: u32,
}

impl PersistedDialogVisit {
    pub fn new(id: impl Into<String>, count: u32) -> Self {
        Self {
            id: id.into(),
            count,
        }
    }
}

/// Top-level sandbox save. Versioned so a future schema change can
/// migrate or refuse to load gracefully.
///
/// Designed to be open-set / extensible: every collection takes
/// `#[serde(default)]` so older saves load against newer schemas with
/// missing fields filling in as empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxSaveData {
    #[serde(default = "default_save_version")]
    pub version: u32,
    #[serde(default)]
    pub encounters: Vec<PersistedEncounter>,
    #[serde(default)]
    pub switches: Vec<PersistedSwitch>,
    #[serde(default)]
    pub bosses: Vec<PersistedBossDefeat>,
    #[serde(default)]
    pub quests: Vec<PersistedQuest>,
    #[serde(default)]
    pub flags: Vec<PersistedFlag>,
    /// Per-dialogue-id visit counters. `#[serde(default)]` keeps
    /// older saves loadable: missing field → empty Vec.
    #[serde(default)]
    pub dialog_visits: Vec<PersistedDialogVisit>,
}

pub const CURRENT_SAVE_VERSION: u32 = 2;

fn default_save_version() -> u32 {
    CURRENT_SAVE_VERSION
}

impl SandboxSaveData {
    pub fn new() -> Self {
        Self {
            version: CURRENT_SAVE_VERSION,
            encounters: Vec::new(),
            switches: Vec::new(),
            bosses: Vec::new(),
            quests: Vec::new(),
            flags: Vec::new(),
            dialog_visits: Vec::new(),
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

    pub fn boss(&self, id: &str) -> PersistedEncounterState {
        self.bosses
            .iter()
            .find(|b| b.id == id)
            .map(|b| b.state)
            .unwrap_or_default()
    }

    /// Set a boss's terminal state. `Untouched` removes the entry to
    /// keep the save file compact, mirroring `set_encounter`.
    pub fn set_boss(&mut self, id: impl Into<String>, state: PersistedEncounterState) {
        let id = id.into();
        if matches!(state, PersistedEncounterState::Untouched) {
            self.bosses.retain(|b| b.id != id);
            return;
        }
        if let Some(existing) = self.bosses.iter_mut().find(|b| b.id == id) {
            existing.state = state;
        } else {
            self.bosses.push(PersistedBossDefeat { id, state });
        }
    }

    pub fn quest(&self, id: &str) -> (PersistedQuestState, u8) {
        self.quests
            .iter()
            .find(|q| q.id == id)
            .map(|q| (q.state, q.step))
            .unwrap_or((PersistedQuestState::NotStarted, 0))
    }

    pub fn set_quest(&mut self, id: impl Into<String>, state: PersistedQuestState, step: u8) {
        let id = id.into();
        if matches!(state, PersistedQuestState::NotStarted) {
            self.quests.retain(|q| q.id != id);
            return;
        }
        if let Some(existing) = self.quests.iter_mut().find(|q| q.id == id) {
            existing.state = state;
            existing.step = step;
        } else {
            self.quests.push(PersistedQuest { id, state, step });
        }
    }

    pub fn flag(&self, id: &str) -> bool {
        self.flags
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.on)
            .unwrap_or(false)
    }

    pub fn set_flag(&mut self, id: impl Into<String>, on: bool) {
        let id = id.into();
        if !on {
            // Off is the default — drop the entry to keep the save
            // compact. Mirrors `set_encounter` with `Untouched`.
            self.flags.retain(|f| f.id != id);
            return;
        }
        if let Some(existing) = self.flags.iter_mut().find(|f| f.id == id) {
            existing.on = on;
        } else {
            self.flags.push(PersistedFlag { id, on });
        }
    }

    /// How many times the named dialogue has been entered. `0` for
    /// never-visited nodes. Used by Yarn's `visit_count(id)` binding
    /// to drive first-vs-repeat dialogue variants.
    pub fn dialog_visit_count(&self, id: &str) -> u32 {
        self.dialog_visits
            .iter()
            .find(|v| v.id == id)
            .map(|v| v.count)
            .unwrap_or(0)
    }

    /// Increment the named dialogue's visit counter (saturating at
    /// `u32::MAX`). Called once per dialog session by the
    /// `DialogState::start` path so `visit_count(id) == 1` reads
    /// "this is the first visit".
    pub fn increment_dialog_visit(&mut self, id: impl Into<String>) {
        let id = id.into();
        if let Some(existing) = self.dialog_visits.iter_mut().find(|v| v.id == id) {
            existing.count = existing.count.saturating_add(1);
        } else {
            self.dialog_visits
                .push(PersistedDialogVisit { id, count: 1 });
        }
    }

    /// Clear every flag whose id ends with `_dead_until_rest`. Used
    /// by the sandbox rest mechanic to revive enemies whose
    /// archetype policy is OnRest. Returns the number of flags
    /// dropped — useful for HUD feedback and tests.
    ///
    /// The suffix is duplicated as a literal here (rather than
    /// imported from the sandbox crate) so the engine save module
    /// stays free of sandbox dependencies; keep the two in sync —
    /// the sandbox side declares it as
    /// `crate::features::ENEMY_DEAD_UNTIL_REST_SUFFIX`.
    pub fn clear_dead_until_rest_flags(&mut self) -> usize {
        let before = self.flags.len();
        self.flags.retain(|f| !f.id.ends_with("_dead_until_rest"));
        before - self.flags.len()
    }

    /// Wholesale clear all gameplay state. Keeps `version` so the
    /// schema remains current. Used by debug "reset save" hooks and
    /// tests.
    pub fn reset_all(&mut self) {
        self.encounters.clear();
        self.switches.clear();
        self.bosses.clear();
        self.quests.clear();
        self.flags.clear();
        self.dialog_visits.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_encounter_reads_untouched() {
        let s = SandboxSaveData::default();
        assert_eq!(
            s.encounter("goblin_encounter"),
            PersistedEncounterState::Untouched
        );
    }

    #[test]
    fn setting_encounter_round_trips() {
        let mut s = SandboxSaveData::new();
        s.set_encounter("goblin_encounter", PersistedEncounterState::Cleared);
        assert_eq!(
            s.encounter("goblin_encounter"),
            PersistedEncounterState::Cleared
        );
        // Resetting to untouched removes the entry to keep the save compact.
        s.set_encounter("goblin_encounter", PersistedEncounterState::Untouched);
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
        s.set_encounter("goblin_encounter", PersistedEncounterState::Cleared);
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

    #[test]
    fn boss_round_trip_and_untouched_removes_entry() {
        let mut s = SandboxSaveData::new();
        s.set_boss("gradient_sentinel", PersistedEncounterState::Cleared);
        assert_eq!(
            s.boss("gradient_sentinel"),
            PersistedEncounterState::Cleared
        );
        s.set_boss("gradient_sentinel", PersistedEncounterState::Untouched);
        assert!(s.bosses.is_empty());
    }

    #[test]
    fn quest_round_trip_and_not_started_removes_entry() {
        let mut s = SandboxSaveData::new();
        s.set_quest("first_steps", PersistedQuestState::InProgress, 1);
        assert_eq!(s.quest("first_steps"), (PersistedQuestState::InProgress, 1));
        s.set_quest("first_steps", PersistedQuestState::Completed, 3);
        assert_eq!(s.quest("first_steps"), (PersistedQuestState::Completed, 3));
        s.set_quest("first_steps", PersistedQuestState::NotStarted, 0);
        assert!(s.quests.is_empty());
    }

    #[test]
    fn flag_round_trip_and_off_removes_entry() {
        let mut s = SandboxSaveData::new();
        assert!(!s.flag("seen_intro_cutscene"));
        s.set_flag("seen_intro_cutscene", true);
        assert!(s.flag("seen_intro_cutscene"));
        s.set_flag("seen_intro_cutscene", false);
        assert!(s.flags.is_empty());
    }

    #[test]
    fn deserialize_v1_save_loads_with_empty_new_collections() {
        // A v1-style save (no bosses/quests/flags fields) must still
        // load — that's the contract of `#[serde(default)]` on each
        // collection. Verifies the v1 → v2 schema migration is
        // backwards-compatible at the wire level.
        let json = r#"{"version":1,"encounters":[{"id":"goblin_encounter","state":"Cleared"}],"switches":[]}"#;
        let s: SandboxSaveData = serde_json::from_str(json).expect("parse");
        assert_eq!(
            s.encounter("goblin_encounter"),
            PersistedEncounterState::Cleared
        );
        assert!(s.bosses.is_empty());
        assert!(s.quests.is_empty());
        assert!(s.flags.is_empty());
    }

    #[test]
    fn reset_all_clears_every_collection() {
        let mut s = SandboxSaveData::new();
        s.set_encounter("a", PersistedEncounterState::Cleared);
        s.set_switch("b", true);
        s.set_boss("c", PersistedEncounterState::Cleared);
        s.set_quest("d", PersistedQuestState::InProgress, 2);
        s.set_flag("e", true);
        s.reset_all();
        assert!(s.encounters.is_empty());
        assert!(s.switches.is_empty());
        assert!(s.bosses.is_empty());
        assert!(s.quests.is_empty());
        assert!(s.flags.is_empty());
        assert_eq!(s.version, CURRENT_SAVE_VERSION);
    }
}
