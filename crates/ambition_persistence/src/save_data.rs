//! Pure save-game data shapes (`AmbitionGameSaveData`, `PersistedEncounter`,
//! `PersistedSwitch`, ability/quest flags) — the vocabulary the save format
//! is built from.
//!
//! These types are pure data + `serde`: no Bevy, file paths, autosave timing,
//! or LDtk. The Bevy-side disk shim that loads/saves them lives in the sibling
//! `crate::save` module. Keeping the shapes I/O-free gives reusable
//! mechanics (encounter defeat, switch latch, ability flags) one canonical form
//! shared across sandbox and any future story / editor tooling.

use serde::{Deserialize, Serialize};


/// The suffix every `OnRest` death flag ends with.
///
/// It is defined here because the gameplay crate that builds these ids depends
/// on persistence, not the reverse.
pub const DEAD_UNTIL_REST_SUFFIX: &str = "_dead_until_rest";

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

/// Where one runtime occurrence is, as a save file can say it.
///
/// It uses the same vocabulary as the checkpoint horizon: the durable horizon is
/// a serialization of the value that the checkpoint horizon copies.
///
/// It holds no components, velocity, or archetype. A row says where an
/// occurrence is. What it is comes from the authored record, or, for a runtime
/// mint, from [`PersistedMintedItem`]. Component snapshots would tie the save
/// format to ECS layout; that is the job of rollback.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistedWhereabouts {
    /// In somebody's hands. [`PersistedCustody`] records whose. "Somebody has
    /// it" is enough to stop a room minting a second one.
    InCustody,
    /// Lying in `room`, at integer world pixels.
    ///
    /// Integer pixels, for the same reason as [`PersistedCheckpoint`]: a float
    /// removes the `Eq` derive on `AmbitionGameSaveData`, and a NaN would make
    /// the value-comparing autosave rewrite the file every frame. The live
    /// ledger republishes the exact position when the room loads.
    Placed { room: String, x: i32, y: i32 },
    /// Gone for good, and the world is supposed to remember that.
    ///
    /// The live variant has no producer yet, but the format must express it:
    /// otherwise a save silently undoes a terminal disposition.
    /// `a_consumed_occurrence_is_not_resurrected_by_a_load` guards this.
    Consumed,
}

/// One occurrence's whereabouts, keyed by its `SimId` as a string.
///
/// Absence is the default. A save holds a row only for an occurrence that a
/// system had a reason to write. No row means "author it from the record". The
/// save is not a registry of every occurrence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedOccurrence {
    pub id: String,
    pub whereabouts: PersistedWhereabouts,
}

impl PersistedOccurrence {
    pub fn new(id: impl Into<String>, whereabouts: PersistedWhereabouts) -> Self {
        Self {
            id: id.into(),
            whereabouts,
        }
    }
}

/// Which body was carrying which occurrence, both sides by `SimId` string.
///
/// The disk form of
/// `ambition_platformer2d::platformer::lifecycle::CustodyBaseline`. It is
/// separate from [`PersistedOccurrence`] because the two have different owners,
/// and a merged form would let every "was this suppressed?" reader reach a
/// body's inventory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedCustody {
    pub occurrence: String,
    pub custodian: String,
}

impl PersistedCustody {
    pub fn new(occurrence: impl Into<String>, custodian: impl Into<String>) -> Self {
        Self {
            occurrence: occurrence.into(),
            custodian: custodian.into(),
        }
    }
}

/// How to rebuild one instance the SIMULATION minted, which no authored
/// record anywhere can describe.
///
/// ```text
/// identity     occurrence      the occurrence's own SimId
/// provenance   parent+sequence SpawnOrigin::Dynamic — what makes it re-mintable AGAIN
/// definition   held_item       the item spec's authored id — a REFERENCE, not a copy
/// ```
///
/// The provenance lets the next capture find the instance again. Without it,
/// the instance survives one load and is then lost.
///
/// `held_item` is a reference, not a copy of the spec. A copy would override
/// later content edits in every older save.
///
/// There is no position: these rows are held items, and the hand gives the place.
///
/// See the module note on `session::durable_horizon`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedMintedItem {
    pub occurrence: String,
    pub parent: String,
    pub sequence: u64,
    pub held_item: String,
}

/// Where the player resumes: the last checkpoint they touched.
///
/// It holds the room id and the position, so that the player resumes where they
/// stood, not at the room's authored spawn.
///
/// The position is integer world pixels, so the save keeps `Eq`.
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
pub struct AmbitionGameSaveData {
    #[serde(default = "default_save_version")]
    pub(crate) version: u32,
    #[serde(default)]
    pub(crate) encounters: Vec<PersistedEncounter>,
    #[serde(default)]
    pub(crate) switches: Vec<PersistedSwitch>,
    #[serde(default)]
    pub(crate) bosses: Vec<PersistedBossDefeat>,
    #[serde(default)]
    pub(crate) quests: Vec<PersistedQuest>,
    #[serde(default)]
    pub(crate) flags: Vec<PersistedFlag>,
    /// Per-dialogue-id visit counters. `#[serde(default)]` keeps
    /// older saves loadable: missing field → empty Vec.
    #[serde(default)]
    pub(crate) dialog_visits: Vec<PersistedDialogVisit>,
    /// Owned catalog items (the OoT inventory), keyed by `dialog_id`.
    #[serde(default)]
    pub(crate) items: Vec<PersistedItem>,
    /// Player wallet balance.
    #[serde(default)]
    pub(crate) wallet: i32,
    /// Set once the inventory has been persisted at least once, so a restore can
    /// tell a genuinely-saved-but-empty inventory (sold everything) from a fresh
    /// save (keep the starter set).
    #[serde(default)]
    pub(crate) inventory_saved: bool,
    /// The last checkpoint the player touched, if any. `None` is a fresh run.
    #[serde(default)]
    pub(crate) checkpoint: Option<PersistedCheckpoint>,
    /// What became of each runtime occurrence the world remembers anything
    /// about — the durable half of the whereabouts ledger.
    ///
    /// Sparse: only occurrences that were moved, carried, or ended appear.
    /// All others reconstruct from their authored record.
    #[serde(default)]
    pub(crate) occurrences: Vec<PersistedOccurrence>,
    /// Which body was holding which occurrence when this save was written.
    /// Empty hands is a real answer and writes an empty list.
    #[serde(default)]
    pub(crate) custody: Vec<PersistedCustody>,
    /// How to remake the runtime-minted instances that were in a hand. See
    /// [`PersistedMintedItem`].
    #[serde(default)]
    pub(crate) minted_items: Vec<PersistedMintedItem>,
}

/// v4 adds `occurrences`, `custody` and `minted_items` — the durable horizon.
///
/// The bump is on an additive change. With `#[serde(default)]`, a v3 file loads
/// with three empty lists, which is the correct reading. The bump makes a v4
/// file `FromTheFuture` to a v3 build, so that build does not overwrite it.
pub const CURRENT_SAVE_VERSION: u32 = 4;

/// What a file with no `version` field actually is: written by a build from
/// before the field existed, i.e. v1.
pub const PRE_VERSIONING_SAVE_VERSION: u32 = 1;

fn default_save_version() -> u32 {
    PRE_VERSIONING_SAVE_VERSION
}

/// What loading a file concluded about its format.
///
/// It is returned, not logged: for an incompatible file the caller must not
/// write over the bytes, and it needs the verdict to know that.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveCompatibility {
    /// Already at [`CURRENT_SAVE_VERSION`].
    Current,
    /// Upgraded from an older version, which is named so a log line can say it.
    Migrated { from: u32 },
    /// Written by a newer build. The parsed data is usable, but writing over it
    /// destroys what the newer build knew. The player must not lose that save.
    FromTheFuture { found: u32 },
    /// The file names a schema version with no migration path, including
    /// historical `version: 0` files. Callers must preserve the file and
    /// continue from defaults.
    Unsupported { found: u32 },
}

impl SaveCompatibility {
    /// May this build commit its own state over the file it read?
    pub fn is_writable(self) -> bool {
        !matches!(self, Self::FromTheFuture { .. } | Self::Unsupported { .. })
    }
}

/// A fresh save stamped with the current version. `Default` delegates here so a
/// missing/corrupt file (`load_save`) and a reset (`session::reset`) both produce
/// a `CURRENT_SAVE_VERSION` save, not the `u32::default()` (0) a derive would give.
impl Default for AmbitionGameSaveData {
    fn default() -> Self {
        Self::new()
    }
}

impl AmbitionGameSaveData {
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
            occurrences: Vec::new(),
            custody: Vec::new(),
            minted_items: Vec::new(),
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

    /// Set an encounter's state. Inserts a new entry if needed; replaces existing.
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

    /// The owned-item rows (the OoT inventory).
    pub fn items(&self) -> &[PersistedItem] {
        &self.items
    }

    /// The wallet balance.
    pub fn wallet(&self) -> i32 {
        self.wallet
    }

    /// Whether the inventory has been persisted at least once -- the bit that
    /// separates "sold everything" from "fresh save".
    pub fn inventory_saved(&self) -> bool {
        self.inventory_saved
    }

    /// Replace the inventory triple. The three fields are one fact: items
    /// without `inventory_saved` read as a fresh save on the next load.
    pub fn set_inventory(&mut self, items: Vec<PersistedItem>, wallet: i32) {
        self.items = items;
        self.wallet = wallet;
        self.inventory_saved = true;
    }

    /// The last checkpoint the player touched. `None` is a fresh run.
    pub fn checkpoint(&self) -> Option<&PersistedCheckpoint> {
        self.checkpoint.as_ref()
    }

    /// Record the checkpoint the player just touched.
    pub fn set_checkpoint(&mut self, checkpoint: PersistedCheckpoint) {
        self.checkpoint = Some(checkpoint);
    }

    /// The durable whereabouts rows -- sparse by construction.
    pub fn occurrences(&self) -> &[PersistedOccurrence] {
        &self.occurrences
    }

    /// Who was holding what when this save was written.
    pub fn custody(&self) -> &[PersistedCustody] {
        &self.custody
    }

    /// Replace the whereabouts ledger. Both fields are set together because a
    /// custody row without its occurrence row names nothing.
    pub fn set_durable_horizon(
        &mut self,
        occurrences: Vec<PersistedOccurrence>,
        custody: Vec<PersistedCustody>,
    ) {
        self.occurrences = occurrences;
        self.custody = custody;
    }

    /// How to remake the runtime-minted instances that were in a hand.
    pub fn minted_items(&self) -> &[PersistedMintedItem] {
        &self.minted_items
    }

    /// Replace the minted-item recipes.
    pub fn set_minted_items(&mut self, minted_items: Vec<PersistedMintedItem>) {
        self.minted_items = minted_items;
    }

    /// Which durable fact families differ between two saves, by name.
    ///
    /// The exhaustive destructure of both sides is the guard. Do not replace it
    /// with field access: it makes the compiler force a decision for each new
    /// family. `version` is excluded because it is schema metadata.
    pub fn families_that_differ(&self, other: &Self) -> Vec<&'static str> {
        let Self {
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
            occurrences,
            custody,
            minted_items,
        } = self;
        let Self {
            version: _,
            encounters: o_encounters,
            switches: o_switches,
            bosses: o_bosses,
            quests: o_quests,
            flags: o_flags,
            dialog_visits: o_dialog_visits,
            items: o_items,
            wallet: o_wallet,
            inventory_saved: o_inventory_saved,
            checkpoint: o_checkpoint,
            occurrences: o_occurrences,
            custody: o_custody,
            minted_items: o_minted_items,
        } = other;
        let mut differ = Vec::new();
        let mut check = |name: &'static str, same: bool| {
            if !same {
                differ.push(name);
            }
        };
        check("encounters", encounters == o_encounters);
        check("switches", switches == o_switches);
        check("bosses", bosses == o_bosses);
        check("quests", quests == o_quests);
        check("flags", flags == o_flags);
        check("dialog_visits", dialog_visits == o_dialog_visits);
        check("items", items == o_items);
        check("wallet", wallet == o_wallet);
        check("inventory_saved", inventory_saved == o_inventory_saved);
        check("checkpoint", checkpoint == o_checkpoint);
        check("occurrences", occurrences == o_occurrences);
        check("custody", custody == o_custody);
        check("minted_items", minted_items == o_minted_items);
        differ
    }

    /// Every recorded dialogue-visit row. A READER only: counts are advanced
    /// one at a time through [`Self::increment_dialog_visit`].
    pub fn dialog_visits(&self) -> &[PersistedDialogVisit] {
        &self.dialog_visits
    }

    /// Every recorded flag row. A READER only: flags are written one at a time
    /// through [`Self::set_flag`], which is what keeps "who wrote this flag"
    /// answerable. Callers that want one flag should ask [`Self::flag`].
    pub fn flags(&self) -> &[PersistedFlag] {
        &self.flags
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
            // Off is the default, so drop the entry. Same as `set_encounter`.
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
    /// `u32::MAX`). `DialogState::start` calls it once per dialog session, so
    /// `visit_count(id) == 1` means "first visit".
    pub fn increment_dialog_visit(&mut self, id: impl Into<String>) {
        let id = id.into();
        if let Some(existing) = self.dialog_visits.iter_mut().find(|v| v.id == id) {
            existing.count = existing.count.saturating_add(1);
        } else {
            self.dialog_visits
                .push(PersistedDialogVisit { id, count: 1 });
        }
    }

    /// Clear every flag whose id ends with [`DEAD_UNTIL_REST_SUFFIX`] — the rest
    /// mechanic reviving the bodies whose policy is `OnRest`. Returns how many
    /// were dropped.
    ///
    /// This is the rest mechanic for `OnRest`. Without a caller, `OnRest`
    /// behaves the same as `DeadStaysDead`.
    pub fn clear_dead_until_rest_flags(&mut self) -> usize {
        let before = self.flags.len();
        self.flags
            .retain(|f| !f.id.ends_with(DEAD_UNTIL_REST_SUFFIX));
        before - self.flags.len()
    }

    /// Bring a just-deserialized save up to [`CURRENT_SAVE_VERSION`], reporting
    /// what it found.
    ///
    /// Each step goes from `n` to `n + 1`. The v1 → v2 step is empty but kept,
    /// so that the migration mechanism already exists when a real step is needed.
    #[must_use]
    pub fn migrate(&mut self) -> SaveCompatibility {
        if self.version > CURRENT_SAVE_VERSION {
            return SaveCompatibility::FromTheFuture {
                found: self.version,
            };
        }
        if self.version < PRE_VERSIONING_SAVE_VERSION {
            return SaveCompatibility::Unsupported {
                found: self.version,
            };
        }
        if self.version == CURRENT_SAVE_VERSION {
            return SaveCompatibility::Current;
        }
        let from = self.version;
        while self.version < CURRENT_SAVE_VERSION {
            match self.version {
                // v1 → v2: the added fields are all `#[serde(default)]`, so the
                // upgrade is the version stamp.
                1 => {}
                // v2 → v3: additive; `None` is correct for a v2 file.
                2 => {}
                // v3 → v4: additive; three empty lists is correct for a v3 file.
                3 => {}
                // A version bump without a migration step is an incompatibility,
                // not a panic. The caller preserves the bytes and uses defaults.
                other => {
                    return SaveCompatibility::Unsupported { found: other };
                }
            }
            self.version += 1;
        }
        SaveCompatibility::Migrated { from }
    }

    /// Clear all gameplay state while preserving the current schema version.
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
            occurrences,
            custody,
            minted_items,
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
        occurrences.clear();
        custody.clear();
        minted_items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each arm of `families_that_differ` is checked by name. The exhaustive
    /// destructure catches a missing family but not a swapped pair, such as
    /// `check("bosses", quests == o_quests)`.
    #[test]
    fn every_durable_family_is_reported_under_its_own_name() {
        use std::collections::BTreeSet;
        // (name(s) the mutation should produce, how to make it)
        let cases: Vec<(Vec<&str>, fn(&mut AmbitionGameSaveData))> = vec![
            (vec!["encounters"], |d| {
                d.set_encounter("e", PersistedEncounterState::Cleared)
            }),
            (vec!["switches"], |d| d.set_switch("s", true)),
            (vec!["bosses"], |d| {
                d.set_boss("b", PersistedEncounterState::Cleared)
            }),
            (vec!["quests"], |d| {
                d.set_quest("q", PersistedQuestState::InProgress, 1)
            }),
            (vec!["flags"], |d| d.set_flag("f", true)),
            (vec!["dialog_visits"], |d| d.increment_dialog_visit("dv")),
            (vec!["checkpoint"], |d| {
                d.set_checkpoint(PersistedCheckpoint::new("room", 1, 2))
            }),
            (vec!["minted_items"], |d| {
                d.set_minted_items(vec![PersistedMintedItem {
                    occurrence: "o".into(),
                    parent: "p".into(),
                    sequence: 0,
                    held_item: "h".into(),
                }])
            }),
            // These setters write fields that are one fact, so the
            // expectation names every family they touch.
            (vec!["items", "wallet", "inventory_saved"], |d| {
                d.set_inventory(vec![PersistedItem::new("i", 1)], 7)
            }),
            (vec!["occurrences", "custody"], |d| {
                d.set_durable_horizon(
                    vec![PersistedOccurrence::new(
                        "occ",
                        PersistedWhereabouts::InCustody,
                    )],
                    vec![PersistedCustody::new("occ", "slot:0")],
                )
            }),
        ];

        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for (expected, mutate) in cases {
            let base = AmbitionGameSaveData::new();
            let mut changed = base.clone();
            mutate(&mut changed);
            let differ = base.families_that_differ(&changed);
            let mut want = expected.clone();
            want.sort_unstable();
            let mut got = differ.clone();
            got.sort_unstable();
            assert_eq!(
                got, want,
                "mutating {expected:?} must report exactly those families; got {differ:?}"
            );
            // ...and the comparison is symmetric.
            let mut back = changed.families_that_differ(&base);
            back.sort_unstable();
            assert_eq!(
                back, want,
                "the difference must not depend on argument order"
            );
            seen.extend(expected);
        }

        // Anti-vacuity: the cases must cover every family the function reports.
        // `version` is excluded in the function itself.
        let all: BTreeSet<&str> = [
            "encounters",
            "switches",
            "bosses",
            "quests",
            "flags",
            "dialog_visits",
            "items",
            "wallet",
            "inventory_saved",
            "checkpoint",
            "occurrences",
            "custody",
            "minted_items",
        ]
        .into_iter()
        .collect();
        assert_eq!(seen, all, "a durable family has no case in this test");

        // An unchanged pair reports nothing.
        let a = AmbitionGameSaveData::new();
        assert!(a.families_that_differ(&a.clone()).is_empty());
    }

    #[test]
    fn missing_encounter_reads_untouched() {
        let s = AmbitionGameSaveData::default();
        assert_eq!(
            s.encounter("goblin_encounter"),
            PersistedEncounterState::Untouched
        );
    }

    #[test]
    fn setting_encounter_round_trips() {
        let mut s = AmbitionGameSaveData::new();
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
        let s = AmbitionGameSaveData::default();
        assert!(!s.switch("reset_switch"));
    }

    #[test]
    fn setting_switch_round_trips() {
        let mut s = AmbitionGameSaveData::new();
        s.set_switch("reset_switch", true);
        assert!(s.switch("reset_switch"));
        s.set_switch("reset_switch", false);
        assert!(!s.switch("reset_switch"));
        assert_eq!(s.switches.len(), 1);
    }

    #[test]
    fn serde_round_trip_preserves_fields() {
        let mut s = AmbitionGameSaveData::new();
        s.set_encounter("goblin_encounter", PersistedEncounterState::Cleared);
        s.set_encounter("boss_room", PersistedEncounterState::Failed);
        s.set_switch("reset_switch", true);
        let serialized = serde_json::to_string(&s).expect("serialize");
        let restored: AmbitionGameSaveData =
            serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(s, restored);
    }

    /// Every whereabouts variant survives the wire, including `Consumed`,
    /// which has no live producer yet and so no world-side test.
    #[test]
    fn every_whereabouts_variant_round_trips_including_the_terminal_one() {
        let mut s = AmbitionGameSaveData::new();
        s.occurrences = vec![
            PersistedOccurrence::new("placement:carried", PersistedWhereabouts::InCustody),
            PersistedOccurrence::new(
                "placement:dropped",
                PersistedWhereabouts::Placed {
                    room: "portal_bridge".into(),
                    x: -48,
                    y: 96,
                },
            ),
            PersistedOccurrence::new("placement:eaten", PersistedWhereabouts::Consumed),
        ];
        s.custody = vec![PersistedCustody::new("placement:carried", "player:0")];
        s.minted_items = vec![PersistedMintedItem {
            occurrence: "player:0/3".into(),
            parent: "player:0".into(),
            sequence: 3,
            held_item: "javelin".into(),
        }];

        let text = ron::ser::to_string_pretty(&s, ron::ser::PrettyConfig::default())
            .expect("serialize as the writer does");
        let restored: AmbitionGameSaveData = ron::from_str(&text).expect("deserialize");
        assert_eq!(
            restored, s,
            "a whereabouts row that does not survive RON is an object the player \
             left somewhere and will not find there"
        );
    }

    /// A v3 file loads with three empty lists and migrates up.
    #[test]
    fn a_v3_save_migrates_up_with_no_occurrence_rows() {
        let json = r#"{"version":3,"wallet":42,"inventory_saved":true}"#;
        let mut s: AmbitionGameSaveData = serde_json::from_str(json).expect("parse");
        assert_eq!(s.migrate(), SaveCompatibility::Migrated { from: 3 });
        assert_eq!(s.version, CURRENT_SAVE_VERSION);
        assert_eq!(s.wallet, 42, "the migration costs the player nothing");
        assert!(s.occurrences.is_empty());
        assert!(s.custody.is_empty());
        assert!(s.minted_items.is_empty());
    }

    /// Without a version field, the file is treated as v1.
    #[test]
    fn a_file_with_no_version_field_is_the_version_from_before_the_field() {
        let json = r#"{"encounters":[],"switches":[]}"#;
        let s: AmbitionGameSaveData = serde_json::from_str(json).expect("parse");
        assert_eq!(s.version, PRE_VERSIONING_SAVE_VERSION);
    }

    #[test]
    fn a_fresh_save_is_stamped_current_and_needs_no_migration() {
        let mut s = AmbitionGameSaveData::new();
        assert_eq!(s.version, CURRENT_SAVE_VERSION);
        assert_eq!(s.migrate(), SaveCompatibility::Current);
    }

    /// The whole point: an old file becomes a current one, and says where it
    /// came from so the log can too.
    #[test]
    fn an_old_save_migrates_up_to_the_current_version() {
        let json = r#"{"version":1,"encounters":[{"id":"goblin_encounter","state":"Cleared"}],"switches":[]}"#;
        let mut s: AmbitionGameSaveData = serde_json::from_str(json).expect("parse");
        assert_eq!(s.migrate(), SaveCompatibility::Migrated { from: 1 });
        assert_eq!(s.version, CURRENT_SAVE_VERSION);
        // Migrating must not cost the player anything it was carrying.
        assert_eq!(
            s.encounter("goblin_encounter"),
            PersistedEncounterState::Cleared
        );
    }

    /// There has never been a defined v0 schema. Treat `version: 0` as
    /// incompatible, not as v1, and do not panic.
    #[test]
    fn an_unsupported_old_version_is_refused_without_mutating_it() {
        let mut s = AmbitionGameSaveData::new();
        s.version = 0;
        s.set_flag("old_progress", true);

        let verdict = s.migrate();
        assert_eq!(verdict, SaveCompatibility::Unsupported { found: 0 });
        assert!(!verdict.is_writable());
        assert_eq!(
            s.version, 0,
            "refusing a schema must not relabel it current"
        );
        assert!(
            s.flag("old_progress"),
            "classification must not erase parsed data"
        );
    }

    /// A file from a newer build must be reported, not adopted, because the
    /// older build writes back what it adopts.
    #[test]
    fn a_save_from_a_newer_build_is_refused_rather_than_adopted() {
        let mut s = AmbitionGameSaveData::new();
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

    /// A migration is only worth having if it is total.
    #[test]
    fn every_version_in_range_migrates_to_current() {
        for version in PRE_VERSIONING_SAVE_VERSION..=CURRENT_SAVE_VERSION {
            let mut s = AmbitionGameSaveData::new();
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
        let mut s = AmbitionGameSaveData::new();
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
        let mut s = AmbitionGameSaveData::new();
        s.set_quest("first_steps", PersistedQuestState::InProgress, 1);
        assert_eq!(s.quest("first_steps"), (PersistedQuestState::InProgress, 1));
        s.set_quest("first_steps", PersistedQuestState::Completed, 3);
        assert_eq!(s.quest("first_steps"), (PersistedQuestState::Completed, 3));
        s.set_quest("first_steps", PersistedQuestState::NotStarted, 0);
        assert!(s.quests.is_empty());
    }

    #[test]
    fn flag_round_trip_and_off_removes_entry() {
        let mut s = AmbitionGameSaveData::new();
        assert!(!s.flag("seen_intro_cutscene"));
        s.set_flag("seen_intro_cutscene", true);
        assert!(s.flag("seen_intro_cutscene"));
        s.set_flag("seen_intro_cutscene", false);
        assert!(s.flags.is_empty());
    }

    #[test]
    fn deserialize_v1_save_loads_with_empty_new_collections() {
        // A v1-style save (no bosses/quests/flags fields) must still load,
        // because each collection is `#[serde(default)]`.
        let json = r#"{"version":1,"encounters":[{"id":"goblin_encounter","state":"Cleared"}],"switches":[]}"#;
        let s: AmbitionGameSaveData = serde_json::from_str(json).expect("parse");
        assert_eq!(
            s.encounter("goblin_encounter"),
            PersistedEncounterState::Cleared
        );
        assert!(s.bosses.is_empty());
        assert!(s.quests.is_empty());
        assert!(s.flags.is_empty());
    }

    /// A reset leaves NOTHING behind but the schema version.
    #[test]
    fn reset_all_clears_every_collection() {
        let mut s = AmbitionGameSaveData::new();
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
        s.occurrences.push(PersistedOccurrence::new(
            "placement:h",
            PersistedWhereabouts::Placed {
                room: "portal_bridge".into(),
                x: 4,
                y: 5,
            },
        ));
        s.custody
            .push(PersistedCustody::new("placement:h", "player:0"));
        s.minted_items.push(PersistedMintedItem {
            occurrence: "player:0/0".into(),
            parent: "player:0".into(),
            sequence: 0,
            held_item: "javelin".into(),
        });

        s.reset_all();

        assert_eq!(
            s,
            AmbitionGameSaveData::new(),
            "a wholesale reset must leave exactly a fresh save. Anything surviving \
             here is progress a player asked to erase and did not — the original \
             offenders were the wallet, the item list, and the flag that suppresses \
             the starter inventory"
        );
        assert_eq!(s.version, CURRENT_SAVE_VERSION);
    }

    /// No custody or minted-item row names an occurrence the save does not hold.
    ///
    ///
    /// `set_durable_horizon` takes occurrences and custody together because a
    /// custody row without its occurrence row names nothing. The same applies to
    /// `PersistedMintedItem`, whose `occurrence` field is the same key.
    ///
    /// Minted items have their own setter (`set_minted_items`) and writer
    /// (`items/pickup/minted_horizon.rs`), separate from
    /// `session/durable_horizon.rs`. Nothing compares the two, so this test is
    /// the guard. The real repair, if it fails, is to fold minted items into
    /// `set_durable_horizon`.
    #[test]
    fn no_durable_row_names_an_occurrence_the_save_does_not_hold() {
        let mut save = AmbitionGameSaveData::new();
        save.set_durable_horizon(
            vec![PersistedOccurrence::new(
                "placement:carried",
                // `InCustody` does not say whose; `PersistedCustody` does.
                PersistedWhereabouts::InCustody,
            )],
            vec![PersistedCustody::new("placement:carried", "player:0")],
        );
        save.set_minted_items(vec![PersistedMintedItem {
            occurrence: "placement:carried".to_owned(),
            parent: "placement:parent".to_owned(),
            sequence: 1,
            held_item: "torch".to_owned(),
        }]);

        let known: Vec<&str> = save.occurrences().iter().map(|o| o.id.as_str()).collect();
        // Anti-vacuity: an empty horizon has no orphans trivially.
        assert!(
            !known.is_empty() && !save.custody().is_empty() && !save.minted_items().is_empty(),
            "this fixture must hold all three families, or the assertions below are \
             vacuous"
        );
        for row in save.custody() {
            assert!(
                known.contains(&row.occurrence.as_str()),
                "custody row names occurrence {:?}, which the save does not hold",
                row.occurrence
            );
        }
        for row in save.minted_items() {
            assert!(
                known.contains(&row.occurrence.as_str()),
                "minted-item row names occurrence {:?}, which the save does not hold — \
                 `set_minted_items` is a separate authority from `set_durable_horizon` \
                 and nothing reconciles them",
                row.occurrence
            );
        }
    }

    /// Every durable family states whether an admitted ROOM REPLAY retracts it.
    ///
    ///
    /// The replay path has no durable-fact policy of its own. A room replay
    /// changes no durable family by itself; each retraction is a content system.
    /// There is one today: `reset_cut_rope_attempt_on_replay`, which clears a
    /// persisted `cleared` record for cut-rope placements.
    ///
    /// The destructure has no `..`, so a new field fails to compile (E0027) here.
    /// Its author must then decide whether a replay retracts it.
    ///
    /// Current answers:
    /// * `bosses`: retracted for cut-rope placements only. Other `BossSpawn`
    ///   defeats survive a replay (decision 56, open question).
    /// * `encounters`, `switches`, `quests`, `flags`, `dialog_visits`,
    ///   `occurrences`, `custody`, `minted_items`, `items`, `wallet`,
    ///   `checkpoint`, `inventory_saved`: survive. A replay is not a load.
    /// * `version`: schema metadata, not a world fact.
    ///
    /// This is one of three exhaustive destructures over this type
    /// (`families_that_differ`, the clearing path, this one). Do not merge them:
    /// each asks a different question.
    ///
    /// Entity-shaped state needs no answer here, because the rebuild respawns it.
    /// Resource-shaped per-attempt state uses `AttemptScoped`.
    #[test]
    fn every_durable_family_says_whether_a_replay_retracts_it() {
        let data = AmbitionGameSaveData::new();
        // No `..`: this is the whole type, on purpose.
        let AmbitionGameSaveData {
            version,
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
            occurrences,
            custody,
            minted_items,
        } = &data;

        // Anti-vacuity: a fresh save must hold no durable facts.
        assert_eq!(*version, CURRENT_SAVE_VERSION);
        assert!(
            encounters.is_empty()
                && switches.is_empty()
                && bosses.is_empty()
                && quests.is_empty()
                && flags.is_empty()
                && dialog_visits.is_empty()
                && occurrences.is_empty()
                && custody.is_empty()
                && minted_items.is_empty(),
            "a new save carries no durable world facts to retract"
        );
        assert!(
            checkpoint.is_none() && !*inventory_saved && *wallet == 0,
            "and no checkpoint, no saved inventory, no money"
        );
        // `items` is not asserted empty: `new()` seeds a starter inventory.
        let _ = items;
    }
}
