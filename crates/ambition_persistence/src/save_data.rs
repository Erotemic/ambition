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
/// It is deliberately the same vocabulary rather than a second one, because the durable horizon
/// is a serialization of the value the checkpoint horizon already copies — not a third
/// description of the same fact.
///
///  no components, no velocity, no archetype. A row says WHERE an
/// occurrence is and nothing about what it is made of; what it IS comes back
/// from the authored record (or, for a runtime mint, from
/// [`PersistedMintedItem`]). Snapshotting components here would weld the save
/// format to ECS layout, which is rollback's job and not this one's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistedWhereabouts {
    /// In somebody's hands.  this does not say WHOSE — that is
    /// [`PersistedCustody`], for the same reason the live ledger keeps the
    /// custodian in a separate domain projection: "somebody has it" is enough to
    /// stop a room minting a second one and not enough to put it back.
    InCustody,
    /// Lying in `room`, at integer world pixels.
    ///
    ///  INTEGER pixels, for [`PersistedCheckpoint`]'s reasons exactly. A
    /// float here would cost `AmbitionGameSaveData`'s `Eq` derive, and a NaN —
    /// which compares unequal to itself — would make the value-comparing
    /// autosave rewrite the file every frame forever. A resting object has no
    /// use for sub-pixel precision, and the live ledger republishes the exact
    /// position from the object itself the moment its room is loaded.
    Placed { room: String, x: i32, y: i32 },
    /// Gone for good, and the world is supposed to remember that.
    ///
    ///  the live variant has no producer yet and this one therefore has no
    /// live writer either — but the format spells it, because a terminal
    /// disposition that the file cannot express is a terminal disposition a save
    /// silently undoes. `a_consumed_occurrence_is_not_resurrected_by_a_load`
    /// drives this variant through a real load.
    Consumed,
}

/// One occurrence's whereabouts, keyed by its `SimId` as a string.
///
///  absence is the common case and is the DEFAULT answer. A save carries a
/// row only for an occurrence some system had a reason to write one for; a record
/// nobody has touched has no row, and no row means "author it from the record".
/// A save that listed every occurrence in the world would be the universal
/// instance registry the design explicitly refuses.
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
/// `ambition_platformer2d::platformer::lifecycle::CustodyBaseline`. Kept
/// separate from [`PersistedOccurrence`] because the two answer different
/// questions with different owners — see that type's own header — and because a
/// save that merged them would let every reader of "was this suppressed?" reach
/// a body's inventory.
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
///  the provenance is not decoration. An instance rebuilt without it cannot
/// say which spawner it descends from, so it would be invisible to the NEXT
/// capture — it would survive exactly one load and then become unrecoverable.
///
///  and `held_item` is a REFERENCE. Copying the resolved spec in would put a
/// second authority for *what a javelin is* inside a save file, and a content edit
/// would then be silently overridden by every save written before it.
///
///  no position, because the rows that reach here are the ones a hand was
/// holding: the hand supplies the place.
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
/// Under a value-comparing autosave that marker commits nothing at all, so the claim was false
/// twice over.
///
/// Room id AND position, not just a room: a room is where you are, a checkpoint
/// is where you STAND, and resuming at the room's authored spawn after resting
/// at a shrine on the far side of it is the difference players notice.
///
/// The position is INTEGER world pixels, and deliberately so.
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
    ///  sparse by construction. Only occurrences somebody moved, carried or
    /// ended appear; everything else reconstructs from its authored record,
    /// which is what keeps a load from resurrecting the world's entire history.
    #[serde(default)]
    pub(crate) occurrences: Vec<PersistedOccurrence>,
    /// Which body was holding which occurrence when this save was written.
    /// Empty hands is a real answer and writes an empty list.
    #[serde(default)]
    pub(crate) custody: Vec<PersistedCustody>,
    /// How to remake the runtime-minted instances that were in a hand. Never
    /// a registry of every mint the session ever made — see
    /// [`PersistedMintedItem`].
    #[serde(default)]
    pub(crate) minted_items: Vec<PersistedMintedItem>,
}

/// v4 adds `occurrences`, `custody` and `minted_items` — the durable horizon.
///
///  the bump is deliberate on an ADDITIVE change, and the reason is not
/// ceremony. `#[serde(default)]` already makes a v3 file load with three empty
/// lists, and empty is the correct reading: a build that did not remember
/// occurrences had nothing to say about them, so every authored record
/// reconstructs from itself, which is exactly the pre-v4 behaviour. What the tag
/// buys is the other direction — a v4 file opened by a v3 build is
/// `FromTheFuture`, so that build plays on a fresh sandbox and does NOT write
/// its occurrence-blind understanding over a file that knows where the player
/// left things.
pub const CURRENT_SAVE_VERSION: u32 = 4;

/// What a file with no `version` field actually is: written by a build from
/// before the field existed, i.e. v1.
pub const PRE_VERSIONING_SAVE_VERSION: u32 = 1;

fn default_save_version() -> u32 {
    PRE_VERSIONING_SAVE_VERSION
}

/// What loading a file concluded about its format.
///
/// Returned rather than logged because the interesting case is not "it worked":
/// an incompatible file means the caller MUST NOT write over the bytes it could
/// not safely interpret, and a caller that cannot see the verdict cannot honour
/// that.
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
    /// The file names a schema version for which this build has no migration
    /// path. This includes historical/accidental `version: 0` files. Parsing the
    /// surrounding RON successfully does not make those bytes safe to adopt or
    /// overwrite, so callers must preserve the file and continue from defaults.
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

    /// Replace the inventory triple. ONE setter for the three fields because
    /// they are one fact: a save that records items without recording that it
    /// recorded them reads as a fresh save on the next load.
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

    /// Replace the whereabouts ledger. ONE setter for both fields because a
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
    /// ⛔ THE DESTRUCTURE IS THE GUARD, and it lives here BECAUSE the fields
    /// are sealed. Do not replace it with `self.field` accesses: the compiler
    /// is what stops a fifteenth family from being added without a decision
    /// about whether a replay may keep it. `version` is excluded by name --
    /// schema metadata, not a fact.
    ///
    /// It moved here from `canonical_reconstitution.rs` when the fields became
    /// private, and got stronger on the way: the old copy destructured only the
    /// BEFORE side and read the after side by field, so it was exhaustive in
    /// one direction. Both sides are destructured now.
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
                // v1 → v2: `bosses`, `quests`, `flags`, `dialog_visits`, `items`,
                // `wallet` and `inventory_saved` were added. Every one is
                // `#[serde(default)]`, so deserialization already produced the
                // right empty value; the upgrade is the version stamp itself.
                1 => {}
                // Additive and `#[serde(default)]`, so a v2 file already deserialized to `None`
                // — which is the correct answer: it was written by a build where touching a
                // shrine saved nothing.
                2 => {}
                // v3 → v4: `occurrences`, `custody` and `minted_items` were
                // added. Additive and `#[serde(default)]`, so a v3 file already
                // deserialized to three empty lists — which is the correct
                // answer, not a lossy one: a build that remembered nothing about
                // occurrences leaves every authored record to reconstruct from
                // itself, exactly as it always did.
                3 => {}
                // A future version bump without its migration step is an
                // incompatibility, not a process-fatal programmer assertion.
                // The disk caller will preserve the original bytes and continue
                // from defaults instead of blocking startup.
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

    /// ⭐⭐ EVERY ARM OF `families_that_differ` IS CHECKED BY ITS OWN NAME, and
    /// the reason is the shape of the function: thirteen hand-written
    /// comparisons over paired bindings (`items` vs `o_items`). A mis-paired
    /// line — `check("bosses", quests == o_quests)` — type-checks, and the
    /// exhaustive destructure that guards against a MISSING family cannot see a
    /// SWAPPED one.
    ///
    /// ⛔ The caller that motivated this only ever asserted two names (`wallet`
    /// and `flags`), so eleven arms were live but unvalidated.
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
            // These setters write a PAIR / TRIPLE on purpose — the fields are
            // one fact — so the expectation names every family they touch.
            (vec!["items", "wallet", "inventory_saved"], |d| {
                d.set_inventory(vec![PersistedItem::new("i", 1)], 7)
            }),
            (vec!["occurrences", "custody"], |d| {
                d.set_durable_horizon(
                    vec![PersistedOccurrence::new("occ", PersistedWhereabouts::InCustody)],
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
            assert_eq!(back, want, "the difference must not depend on argument order");
            seen.extend(expected);
        }

        // ⭐ ANTI-VACUITY: the cases above must cover every family the function
        // reports, or an unchecked arm hides here rather than in the caller.
        // `version` is excluded by name in the function itself.
        let all: BTreeSet<&str> = [
            "encounters", "switches", "bosses", "quests", "flags", "dialog_visits",
            "items", "wallet", "inventory_saved", "checkpoint", "occurrences",
            "custody", "minted_items",
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

    /// EVERY whereabouts variant survives the wire, including the terminal
    /// one.
    ///
    ///  the `Consumed` arm is the one worth writing down. It has no live
    /// producer yet, so no behavioural fixture drives it from the world side —
    /// which is exactly the condition under which a variant quietly stops being
    /// serialized correctly and nobody notices until the producer lands. A
    /// terminal disposition the file cannot express is a terminal disposition a
    /// save silently undoes.
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

    /// A v3 file — the last shape before the durable horizon — loads with three
    /// empty lists and migrates up, which is what "additive" has to mean in
    /// practice rather than in the comment.
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

    /// That was harmless only because nothing read the tag; the moment a migration exists, it is
    /// the difference between upgrading a file and misreading it.
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

    /// `version: 0` existed as an accidental/default value in historical
    /// development saves, but there has never been a defined v0 wire schema.
    /// Treat it as incompatible rather than guessing that it means v1, and most
    /// importantly do not panic just because such a file exists on disk.
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

    /// The case that loses real progress if it is got wrong: a player runs a
    /// newer build, then launches an older one. The older build cannot
    /// understand the file, and must say so rather than quietly adopting it —
    /// because whatever it adopts is what it will write back.
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
        // A v1-style save (no bosses/quests/flags fields) must still
        // load — that's the contract of `#[serde(default)]` on each
        // collection. Verifies the v1 → v2 schema migration is
        // backwards-compatible at the wire level.
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
}
