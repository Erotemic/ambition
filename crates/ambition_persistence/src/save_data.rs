//! Pure save-game data shapes (`SandboxSaveData`, `PersistedEncounter`,
//! `PersistedSwitch`, ability/quest flags) — the vocabulary the save format
//! is built from.
//!
//! These types are pure data + `serde`: no Bevy, file paths, autosave timing,
//! or LDtk. The Bevy-side disk shim that loads/saves them lives in the sibling
//! `crate::save` module. Keeping the shapes I/O-free gives reusable
//! mechanics (encounter defeat, switch latch, ability flags) one canonical form
//! shared across sandbox and any future story / editor tooling.

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

/// One owned catalog item, keyed by its stable lowercase `dialog_id` (not the
/// grid index) so the save survives catalog reordering.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedItem {
    pub id: String,
    pub count: u32,
}

impl PersistedItem {
    pub fn new(id: impl Into<String>, count: u32) -> Self {
        Self {
            id: id.into(),
            count,
        }
    }
}

/// Where the player resumes: the last checkpoint they touched.
///
/// A shrine has always claimed to be a save point — it logged "healed to full +
/// saved" — while writing no checkpoint anywhere, because the save had no field
/// for one and the shrine only called `set_changed()` on a value it never
/// modified (GPT 5.6, 2026-07-27). Under a value-comparing autosave that marker
/// commits nothing at all, so the claim was false twice over.
///
/// Room id AND position, not just a room: a room is where you are, a checkpoint
/// is where you STAND, and resuming at the room's authored spawn after resting
/// at a shrine on the far side of it is the difference players notice.
///
/// The position is INTEGER world pixels, and deliberately so. A checkpoint has no
/// use for sub-pixel precision, and a float here would cost two things that
/// matter more: `SandboxSaveData` could no longer derive `Eq`, and a NaN — which
/// compares unequal to itself — would make the value-comparing autosave rewrite
/// the file on every single frame, forever.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedCheckpoint {
    pub room_id: String,
    pub x: i32,
    pub y: i32,
}

impl PersistedCheckpoint {
    pub fn new(room_id: impl Into<String>, x: i32, y: i32) -> Self {
        Self {
            room_id: room_id.into(),
            x,
            y,
        }
    }
}

/// Top-level sandbox save. Versioned so a future schema change can
/// migrate or refuse to load gracefully.
///
/// Designed to be open-set / extensible: every collection takes
/// `#[serde(default)]` so older saves load against newer schemas with
/// missing fields filling in as empty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Owned catalog items (the OoT inventory), keyed by `dialog_id`.
    #[serde(default)]
    pub items: Vec<PersistedItem>,
    /// Player wallet balance.
    #[serde(default)]
    pub wallet: i32,
    /// Set once the inventory has been persisted at least once, so a restore can
    /// tell a genuinely-saved-but-empty inventory (sold everything) from a fresh
    /// save (keep the starter set).
    #[serde(default)]
    pub inventory_saved: bool,
    /// The last checkpoint the player touched, if any. `None` is a fresh run.
    #[serde(default)]
    pub checkpoint: Option<PersistedCheckpoint>,
}

/// v3 adds `checkpoint`. The bump is what exercises the migration chain on a real
/// schema change rather than on a hypothetical one — every v2 file on disk now
/// takes the v2 → v3 step on load and is written back tagged v3.
pub const CURRENT_SAVE_VERSION: u32 = 3;

/// What a file with no `version` field actually is: written by a build from
/// before the field existed, i.e. v1.
///
/// This used to default to [`CURRENT_SAVE_VERSION`], which made every
/// pre-versioning file CLAIM to be current — the one thing a version tag exists
/// to prevent. Nothing read the tag, so nothing noticed.
pub const PRE_VERSIONING_SAVE_VERSION: u32 = 1;

fn default_save_version() -> u32 {
    PRE_VERSIONING_SAVE_VERSION
}

/// What loading a file concluded about its format.
///
/// Returned rather than logged because the interesting case is not "it worked":
/// [`SaveCompatibility::FromTheFuture`] means the caller MUST NOT write over the
/// file, and a caller that cannot see the verdict cannot honour that.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveCompatibility {
    /// Already at [`CURRENT_SAVE_VERSION`].
    Current,
    /// Upgraded from an older version, which is named so a log line can say it.
    Migrated { from: u32 },
    /// Written by a NEWER build than this one. Not an error to read — the data
    /// that parsed is still there — but writing over it destroys whatever the
    /// newer build knew and this one does not. A player who launches an older
    /// build once should not lose the save they made in the newer one.
    FromTheFuture { found: u32 },
}

impl SaveCompatibility {
    /// May this build commit its own state over the file it read?
    pub fn is_writable(self) -> bool {
        !matches!(self, Self::FromTheFuture { .. })
    }
}

/// A fresh save stamped with the current version. `Default` delegates here so a
/// missing/corrupt file (`load_save`) and a reset (`session::reset`) both produce
/// a `CURRENT_SAVE_VERSION` save, not the `u32::default()` (0) a derive would give.
impl Default for SandboxSaveData {
    fn default() -> Self {
        Self::new()
    }
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
            items: Vec::new(),
            wallet: 0,
            inventory_saved: false,
            checkpoint: None,
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
            // An unrecorded quest is in its default state by definition.
            .unwrap_or_default()
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

    /// Bring a just-deserialized save up to [`CURRENT_SAVE_VERSION`], reporting
    /// what it found.
    ///
    /// The version field has existed since v2 and was WRITTEN and never READ:
    /// no migration, no compatibility check, no consumer anywhere in the
    /// workspace. A tag nothing reads is not a tag, and the cost only arrives
    /// once real player files exist — which is why this landed before release
    /// rather than after.
    ///
    /// Steps run in sequence so each one only has to know how to get from `n` to
    /// `n + 1`. v1 → v2 is deliberately EMPTY and deliberately present: the wire
    /// change was additive (`#[serde(default)]` on the new collections already
    /// fills them), so there is nothing to do — but the step has to exist, or the
    /// first migration that does something real would also be the first one that
    /// has to invent the mechanism, under pressure, with player data at stake.
    #[must_use]
    pub fn migrate(&mut self) -> SaveCompatibility {
        if self.version > CURRENT_SAVE_VERSION {
            return SaveCompatibility::FromTheFuture {
                found: self.version,
            };
        }
        if self.version == CURRENT_SAVE_VERSION {
            return SaveCompatibility::Current;
        }
        let from = self.version;
        while self.version < CURRENT_SAVE_VERSION {
            match self.version {
                // v1 → v2: `bosses`, `quests`, `flags`, `dialog_visits`, `items`,
                // `wallet` and `inventory_saved` were added. Every one is
                // `#[serde(default)]`, so deserialization already produced the
                // right empty value; the upgrade is the version stamp itself.
                1 => {}
                // v2 → v3: `checkpoint` was added. Additive and
                // `#[serde(default)]`, so a v2 file already deserialized to
                // `None` — which is the correct answer: it was written by a build
                // where touching a shrine saved nothing.
                2 => {}
                // Unreachable while the loop bound is CURRENT_SAVE_VERSION, but a
                // future version added without a step must not spin here.
                other => {
                    self.version = CURRENT_SAVE_VERSION;
                    debug_assert!(false, "no migration step from save version {other}");
                    break;
                }
            }
            self.version += 1;
        }
        SaveCompatibility::Migrated { from }
    }

    /// Wholesale clear all gameplay state. Keeps `version` so the schema remains
    /// current.
    ///
    /// **Every field except the version, and that is the point.** This used to
    /// clear six of the nine collections and silently keep `items`, `wallet` and
    /// `inventory_saved` — so a "reset save" would have left the player their
    /// money, their inventory, and the flag saying the inventory had been saved
    /// before (which suppresses the starter set). Nothing in the shipping game
    /// calls this yet, which is exactly why it was worth fixing now: the defect
    /// costs nothing today and is a silently-wrong reset button the day someone
    /// wires one up (GPT 5.6, 2026-07-27).
    ///
    /// A field added to `SandboxSaveData` and not cleared here is the same bug
    /// again; `reset_all_clears_every_collection` is written to fail on that.
    pub fn reset_all(&mut self) {
        let Self {
            // Kept: the schema is still the current schema after a reset.
            version: _,
            encounters,
            switches,
            bosses,
            quests,
            flags,
            dialog_visits,
            items,
            wallet,
            inventory_saved,
            checkpoint,
        } = self;
        encounters.clear();
        switches.clear();
        bosses.clear();
        quests.clear();
        flags.clear();
        dialog_visits.clear();
        items.clear();
        *wallet = 0;
        *inventory_saved = false;
        *checkpoint = None;
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

    /// A file with no `version` field was written before the field existed —
    /// it is v1, and saying so is the entire job of a version tag.
    ///
    /// This test previously asserted the opposite (`uses_current`), which made
    /// every pre-versioning file claim to be the current shape. That was
    /// harmless only because nothing read the tag; the moment a migration exists,
    /// it is the difference between upgrading a file and misreading it.
    #[test]
    fn a_file_with_no_version_field_is_the_version_from_before_the_field() {
        let json = r#"{"encounters":[],"switches":[]}"#;
        let s: SandboxSaveData = serde_json::from_str(json).expect("parse");
        assert_eq!(s.version, PRE_VERSIONING_SAVE_VERSION);
    }

    #[test]
    fn a_fresh_save_is_stamped_current_and_needs_no_migration() {
        let mut s = SandboxSaveData::new();
        assert_eq!(s.version, CURRENT_SAVE_VERSION);
        assert_eq!(s.migrate(), SaveCompatibility::Current);
    }

    /// The whole point: an old file becomes a current one, and says where it
    /// came from so the log can too.
    #[test]
    fn an_old_save_migrates_up_to_the_current_version() {
        let json = r#"{"version":1,"encounters":[{"id":"goblin_encounter","state":"Cleared"}],"switches":[]}"#;
        let mut s: SandboxSaveData = serde_json::from_str(json).expect("parse");
        assert_eq!(s.migrate(), SaveCompatibility::Migrated { from: 1 });
        assert_eq!(s.version, CURRENT_SAVE_VERSION);
        // Migrating must not cost the player anything it was carrying.
        assert_eq!(
            s.encounter("goblin_encounter"),
            PersistedEncounterState::Cleared
        );
    }

    /// The case that loses real progress if it is got wrong: a player runs a
    /// newer build, then launches an older one. The older build cannot
    /// understand the file, and must say so rather than quietly adopting it —
    /// because whatever it adopts is what it will write back.
    #[test]
    fn a_save_from_a_newer_build_is_refused_rather_than_adopted() {
        let mut s = SandboxSaveData::new();
        s.version = CURRENT_SAVE_VERSION + 7;
        assert_eq!(
            s.migrate(),
            SaveCompatibility::FromTheFuture {
                found: CURRENT_SAVE_VERSION + 7
            }
        );
        assert!(!s.migrate().is_writable());
        // And it did NOT quietly stamp itself current on the way past.
        assert_eq!(s.version, CURRENT_SAVE_VERSION + 7);
    }

    /// A migration is only worth having if it is total. Every version from the
    /// first to the current one must arrive at the current one — the loop has a
    /// step for each, and a version added without a step is the failure this
    /// catches.
    #[test]
    fn every_version_in_range_migrates_to_current() {
        for version in PRE_VERSIONING_SAVE_VERSION..=CURRENT_SAVE_VERSION {
            let mut s = SandboxSaveData::new();
            s.version = version;
            let verdict = s.migrate();
            assert_eq!(
                s.version, CURRENT_SAVE_VERSION,
                "version {version} did not reach the current version: {verdict:?}"
            );
        }
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

    /// A reset leaves NOTHING behind but the schema version.
    ///
    /// Written against the whole value rather than field by field: comparing to
    /// a fresh save is what makes a newly-added field fail here the day it is
    /// added, instead of quietly surviving every reset like `wallet` and `items`
    /// did (GPT 5.6, 2026-07-27).
    #[test]
    fn reset_all_clears_every_collection() {
        let mut s = SandboxSaveData::new();
        s.set_encounter("a", PersistedEncounterState::Cleared);
        s.set_switch("b", true);
        s.set_boss("c", PersistedEncounterState::Cleared);
        s.set_quest("d", PersistedQuestState::InProgress, 2);
        s.set_flag("e", true);
        s.dialog_visits.push(PersistedDialogVisit::new("f", 3));
        s.items.push(PersistedItem {
            id: "g".to_string(),
            count: 2,
        });
        s.wallet = 400;
        s.inventory_saved = true;

        s.reset_all();

        assert_eq!(
            s,
            SandboxSaveData::new(),
            "a wholesale reset must leave exactly a fresh save. Anything surviving \
             here is progress a player asked to erase and did not — the original \
             offenders were the wallet, the item list, and the flag that suppresses \
             the starter inventory"
        );
        assert_eq!(s.version, CURRENT_SAVE_VERSION);
    }
}
