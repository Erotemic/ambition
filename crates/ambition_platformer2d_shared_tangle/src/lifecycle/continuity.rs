//! Tracks where occurrences created from authored definitions currently belong.
//!
//! The live ledger fills the part of current world state that unloaded rooms
//! cannot represent. Checkpoint state is a copy of this ledger; durable saves
//! serialize the same values. No row means the occurrence remains as authored.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use bevy::prelude::{Query, Res, ResMut, Resource, Vec2, With};

use super::{InCustodyOf, RoomScopedEntity};
use crate::sim_id::SimId;

/// Where one authored record's occurrence is, when it is somewhere other
/// than where the record puts it.
///
/// absence is a variant and it is the common one. No row means "as
/// authored", which is the state of essentially every record essentially
/// always. Nothing here is a status enum for its own sake: each variant exists
/// because reconstruction reaches a different decision from it.
#[derive(Clone, Debug, PartialEq)]
pub enum OccurrenceWhereabouts {
    /// In a body's hands. Alive, in no room, crossing boundaries with
    /// whoever carries it. Written by [`project_custody_onto_authored_occurrences`]
    /// from [`InCustodyOf`], which knows nothing about items.
    ///
    /// this row is REPUBLISHED from live state every tick, so it retracts
    /// by itself when custody ends or when the carrier is destroyed. It is the
    /// one row a rewind can never strand.
    InCustody,
    /// Lying in `room`, at `at`. The occurrence was carried somewhere and
    /// put down; `at` is where it came to rest, republished while `room` is
    /// loaded and FROZEN at the value it last held when `room` unloads.
    ///
    /// this is the row that makes relocation durable, and the only row
    /// whose value outlives the thing it describes.
    Placed { room: String, at: Vec2 },
    /// Gone for good. A consumed key, a destroyed mechanism, a body killed
    /// in a way the world is supposed to remember. Distinct from "no row" —
    /// which is also "not alive" — because an ordinary room unload destroys
    /// occurrences by the dozen and every one of them SHOULD come back.
    ///
    /// Written by `record_consumed_pickups` for a collected pickup authored
    /// `Never` (Q154), republished from live state while its room is live, as
    /// the other rows are. The ledger is registered rollback value state (see
    /// [`AuthoredOccurrences::rewind_argument`]), so a rewind takes it back, and
    /// a checkpoint restore replaces it with the pinned ledger.
    Consumed,
}

/// What reconstruction must do about one authored record, in the room it is
/// currently building. A DERIVED value: [`AuthoredOccurrences::outlook_for`]
/// computes it, nothing stores it.
///
/// there are THREE answers, not two. "Author it" and "do not author it"
/// were enough only while an occurrence could never be anywhere but where its
/// record put it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum OccurrenceDisposition {
    /// Author it, from the record, as written. The default, and what every
    /// record without a row gets.
    #[default]
    Authored,
    /// Author it from the record, but AT `at`. The occurrence belongs to
    /// this room and is not where the record puts it — it was put down
    /// somewhere else in this room and the room has been unloaded since.
    ///
    /// the identity is the record's, so the reconstituted occurrence carries
    /// the same `SimId::placement(..)`: this is the SAME occurrence coming back,
    /// not a copy of it authored at new coordinates.
    Reinstated { at: Vec2 },
    /// Do not author it. The occurrence is alive somewhere this room is not,
    /// or it is deliberately gone. Minting a fresh one would put two live things
    /// behind one identity, or resurrect something the world remembers killing.
    Suppressed,
}

impl OccurrenceDisposition {
    /// Whether construction produces an occurrence for this record at all.
    ///
    /// TRUE for [`Self::Reinstated`]: a reinstatement is an authoring, with
    /// the position overridden. Reading this as "unchanged" is how a
    /// reinstatement silently becomes an authoring at the wrong coordinates.
    pub fn authors_a_fresh_occurrence(self) -> bool {
        !matches!(self, Self::Suppressed)
    }

    /// The position this record must be built at, when it is not the record's
    /// own.
    pub fn relocated_to(self) -> Option<Vec2> {
        match self {
            Self::Reinstated { at } => Some(at),
            _ => None,
        }
    }
}

/// The derived view ONE room's construction consumes.
///
/// derived, room-scoped, and frozen onto the plan it produced. A prepared
/// plan is only valid for the world that produced it — a plan prepared while an
/// object was being carried deliberately omits that object, and committing it
/// after the object was put down would leave the room permanently short of a
/// thing it authors. So the view a plan was prepared against travels WITH the
/// plan and a cache compares it rather than guessing.
///
/// it holds only the records whose disposition is not the default, so the
/// ordinary room's view is empty and comparing two of them is comparing two
/// empty maps.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoomOccurrenceOutlook {
    rows: BTreeMap<SimId, OccurrenceDisposition>,
}

impl RoomOccurrenceOutlook {
    /// What construction must do about this record. Absent is
    /// [`OccurrenceDisposition::Authored`].
    pub fn disposition(&self, sim_id: &SimId) -> OccurrenceDisposition {
        self.rows.get(sim_id).copied().unwrap_or_default()
    }

    /// Every identity this build must NOT mint, in one set.
    ///
    /// Materialized because the construction planner takes it as a guard: a
    /// request that reaches the planner for a suppressed identity by some other
    /// route gets a loud `IdentityAlreadyLive` refusal instead of a second live
    /// occurrence.
    pub fn suppressed(&self) -> BTreeSet<SimId> {
        self.rows
            .iter()
            .filter(|(_, disposition)| !disposition.authors_a_fresh_occurrence())
            .map(|(sim_id, _)| sim_id.clone())
            .collect()
    }

    /// Every identity this build OWES the world, and where it owes it.
    ///
    /// the caller must answer this list, not merely consult it. A
    /// reinstatement whose record this room authors is satisfied by relocating
    /// the room's own request; one whose record belongs to ANOTHER room is
    /// satisfied only by going and getting that record. Both are the same
    /// obligation — an occurrence that is lying in this room and has to be here
    /// when the room is built — and a construction road that services the first
    /// and drops the second deletes the object from the world permanently,
    /// because [`AuthoredOccurrences::outlook_for`] has already told its home
    /// room not to author it.
    pub fn reinstatements(&self) -> BTreeMap<SimId, Vec2> {
        self.rows
            .iter()
            .filter_map(|(sim_id, disposition)| {
                disposition.relocated_to().map(|at| (sim_id.clone(), at))
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// THE authoritative owner of occurrence whereabouts — horizon 1 of the
/// three in this module's header.
///
/// a world with no such resource at all means "nothing is remembered",
/// which is why every consumer takes it as an `Option` and a composition that
/// never picks anything up carries an empty ledger through its whole life.
///
/// The rollback declaration now lives with this domain in
/// `crate::rollback_registration`; renaming this type therefore no longer
/// requires a generic runtime census edit. The name still says
/// `AuthoredOccurrences` because that is what it was when it held one derived
/// set; what it holds now is stated by [`OccurrenceWhereabouts`].
///
/// ⭐ THE ROWS ARE SHARED, NOT COPIED (M2 cut C). The rollback snapshot is a
/// clone, taken on every frame, and a ledger with 10,000 dormant rows cost
/// 1.4 ms per clone and as much per checksum. The rows and the custody index
/// are `Arc`s, so a clone copies two pointers. A mutator writes through
/// `Arc::make_mut`, which copies the rows only when a snapshot shares them, and
/// only on a tick that changes a row. The checksum is kept with the `Arc` it
/// was computed from ([`RowsDigest`]).
#[derive(Resource, Clone, Debug, Default)]
pub struct AuthoredOccurrences {
    rows: Arc<BTreeMap<SimId, OccurrenceWhereabouts>>,
    /// The ids whose row is `InCustody`: a DERIVED index of `rows` (FI9).
    /// Each mutator below keeps it, and [`Self::adopt_rows`] builds it again,
    /// so a clone (the rollback snapshot) carries it consistent with its rows.
    /// It lets the custody producer compare and republish the carried set
    /// without a walk over every row: dormant `Placed` rows add no work to a
    /// tick that carries nothing new.
    custody: Arc<BTreeSet<SimId>>,
    /// The ids whose row is `Placed`, by the room the row names: a DERIVED
    /// index of `rows`, kept like `custody`. It lets a reader of one live
    /// room find the rows that place an occurrence there without a walk over
    /// the dormant rows of every other room ([`Self::placed_in`]).
    placed: Arc<BTreeMap<String, BTreeSet<SimId>>>,
}

/// Equal rows, with the same allocation first: a snapshot and the live ledger
/// share their rows while nothing changed, and that comparison costs nothing.
impl PartialEq for AuthoredOccurrences {
    fn eq(&self, other: &Self) -> bool {
        (Arc::ptr_eq(&self.rows, &other.rows) || self.rows == other.rows)
            && (Arc::ptr_eq(&self.custody, &other.custody) || self.custody == other.custody)
    }
}

/// The peer checksum of one shared row set under one domain, kept with the
/// `Arc` it was computed from. DERIVED STORAGE, NOT STATE: it holds a clone
/// of that `Arc`, so the allocation is not freed and cannot be written in
/// place (`Arc::make_mut` copies a shared `Arc`). An `Arc` that is the same
/// allocation therefore holds the same rows, and any other `Arc` is hashed
/// again. The value is the same fold as without the memo, so two peers agree
/// whatever their memos hold.
struct RowsDigest {
    domain: &'static str,
    slot: std::sync::Mutex<Option<(Arc<BTreeMap<SimId, OccurrenceWhereabouts>>, u64)>>,
}

impl RowsDigest {
    const fn new(domain: &'static str) -> Self {
        Self { domain, slot: std::sync::Mutex::new(None) }
    }

    fn of(&self, ledger: &AuthoredOccurrences) -> u64 {
        // No rows costs nothing to hash, and leaves the slot to rows that do.
        if ledger.rows.is_empty() {
            return ledger.digest(self.domain);
        }
        let mut slot = self.slot.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((held, digest)) = slot.as_ref() {
            if Arc::ptr_eq(held, &ledger.rows) {
                return *digest;
            }
        }
        let digest = ledger.digest(self.domain);
        *slot = Some((ledger.rows.clone(), digest));
        digest
    }
}

static LEDGER_DIGEST: RowsDigest = RowsDigest::new("lifecycle.authored_occurrences");
static BASELINE_DIGEST: RowsDigest = RowsDigest::new("lifecycle.occurrence_baseline");

impl AuthoredOccurrences {
    /// What construction must do about every record of `room`. The one
    /// derived view, produced from the authoritative rows and nothing else.
    ///
    /// a `Placed` row is the SAME fact seen from two rooms, and both
    /// rooms must act on it. The room the occurrence is lying in reinstates
    /// it; every other room — including the one whose record MINTED it —
    /// suppresses it. Those two answers are one decision, and the reason they
    /// are computed in one function from one row is that they cannot be allowed
    /// to disagree: suppressing the home room while nothing rebuilds the
    /// occurrence where it lies deletes the object from the world permanently,
    /// and reinstating without suppressing puts two live things behind one
    /// `SimId`.
    ///
    /// so a consumer may not answer half of this. The obligation stated
    /// by [`RoomOccurrenceOutlook::reinstatements`] includes identities the room
    /// being built does not author — the record lives next door — and a road
    /// that services only its own records has taken the suppression and skipped
    /// the reinstatement. That is why the definitions a room may reach for
    /// travel WITH this ledger to construction rather than beside it.
    pub fn outlook_for(&self, room: &str) -> RoomOccurrenceOutlook {
        let rows = self
            .rows
            .iter()
            .map(|(sim_id, whereabouts)| {
                let disposition = match whereabouts {
                    OccurrenceWhereabouts::InCustody | OccurrenceWhereabouts::Consumed => {
                        OccurrenceDisposition::Suppressed
                    }
                    OccurrenceWhereabouts::Placed { room: at_room, at } if at_room == room => {
                        OccurrenceDisposition::Reinstated { at: *at }
                    }
                    // Lying in some OTHER room. Not alive — that room unloaded
                    // and took it with it — but not this room's to author
                    // either: it comes back when the room it is lying in is
                    // built, from the record this room holds.
                    OccurrenceWhereabouts::Placed { .. } => OccurrenceDisposition::Suppressed,
                };
                (sim_id.clone(), disposition)
            })
            .collect();
        RoomOccurrenceOutlook { rows }
    }

    /// Read one row. For producers deciding whether they have anything to say.
    pub fn whereabouts(&self, sim_id: &SimId) -> Option<&OccurrenceWhereabouts> {
        self.rows.get(sim_id)
    }

    /// Every row, in identity order — for the writer that puts this value on
    /// disk.
    ///
    /// an ITERATOR over the sparse rows, deliberately not a "describe every
    /// occurrence" call. The rows are the exceptions; anything absent from this
    /// walk reconstructs from its authored record, and a durable form that
    /// enumerated the world instead would be the universal instance registry this
    /// ledger exists not to be.
    pub fn rows(&self) -> impl Iterator<Item = (&SimId, &OccurrenceWhereabouts)> {
        self.rows.iter()
    }

    /// Replace the whole ledger with what a save file remembered.
    ///
    /// it takes the rows and not a `Self`, so the only way to build a
    /// ledger from outside is to state every row: a `From<AuthoredOccurrences>`
    /// would let a caller clone one authority into another and call it a load.
    pub fn adopt_rows(&mut self, rows: BTreeMap<SimId, OccurrenceWhereabouts>) {
        if *self.rows != rows {
            self.custody = Arc::new(
                rows.iter()
                    .filter(|(_, whereabouts)| matches!(whereabouts, OccurrenceWhereabouts::InCustody))
                    .map(|(sim_id, _)| sim_id.clone())
                    .collect(),
            );
            let mut placed: BTreeMap<String, BTreeSet<SimId>> = BTreeMap::new();
            for (sim_id, whereabouts) in &rows {
                if let OccurrenceWhereabouts::Placed { room, .. } = whereabouts {
                    placed.entry(room.clone()).or_default().insert(sim_id.clone());
                }
            }
            self.placed = Arc::new(placed);
            self.rows = Arc::new(rows);
        }
    }

    /// The ids whose row places them in `room`, from the placement index, so
    /// the cost is that room's rows and not every row.
    pub fn placed_in(&self, room: &str) -> impl Iterator<Item = &SimId> {
        self.placed.get(room).into_iter().flatten()
    }

    /// Keep the placement index for one row that changed from `old` to `new`.
    fn reindex_placed(&mut self, sim_id: &SimId, old: Option<&OccurrenceWhereabouts>, new: Option<&OccurrenceWhereabouts>) {
        let room_of = |row: Option<&OccurrenceWhereabouts>| match row {
            Some(OccurrenceWhereabouts::Placed { room, .. }) => Some(room.clone()),
            _ => None,
        };
        let (old, new) = (room_of(old), room_of(new));
        if old == new {
            return;
        }
        let placed = Arc::make_mut(&mut self.placed);
        if let Some(old) = old {
            if let Some(ids) = placed.get_mut(&old) {
                ids.remove(sim_id);
                if ids.is_empty() {
                    placed.remove(&old);
                }
            }
        }
        if let Some(new) = new {
            placed.entry(new).or_default().insert(sim_id.clone());
        }
    }

    /// Whether anything is remembered about this occurrence at all — the
    /// question a placement producer asks before it starts tracking one.
    pub fn remembers(&self, sim_id: &SimId) -> bool {
        self.rows.contains_key(sim_id)
    }

    /// The ids currently recorded as carried. Compared before a republish so
    /// change detection stays quiet on the overwhelming majority of ticks.
    ///
    /// From the custody index, so the cost is the carried set, not every row.
    pub fn in_custody(&self) -> &BTreeSet<SimId> {
        &self.custody
    }

    /// This ledger with the custody rows of `released` taken off, as it will
    /// be once their custody ends: for a construction prepared before that
    /// happens (see `AwayFromAuthoredRoom`). An id with another row keeps it.
    pub fn with_custody_released(&self, released: &BTreeSet<SimId>) -> Self {
        let mut ledger = self.clone();
        let carried: BTreeSet<SimId> = self.custody.difference(released).cloned().collect();
        if carried.len() != self.custody.len() {
            ledger.republish_custody(carried);
        }
        ledger
    }

    /// This ledger with `held` added to its custody rows: for a construction
    /// that rebuilds a room from an older ledger while those occurrences still
    /// live in a room the construction does not rewind (see
    /// `AwayFromAuthoredRoom`). An id with another row is held too: a live
    /// body is the newer fact.
    pub fn with_custody_held(&self, held: &BTreeSet<SimId>) -> Self {
        let mut ledger = self.clone();
        if !held.is_subset(&self.custody) {
            ledger.republish_custody(self.custody.union(held).cloned().collect());
        }
        ledger
    }

    /// Republish the whole custody leg.
    ///
    /// it touches ONLY custody rows. A `Placed` row is not a custody row
    /// that went missing; it is a different fact, written by a different
    /// producer, and a whole-ledger republish here would delete relocation the
    /// instant a hand emptied.
    pub fn republish_custody(&mut self, carried: BTreeSet<SimId>) {
        // The index names every custody row, so only those rows are visited.
        let rows = Arc::make_mut(&mut self.rows);
        for sim_id in self.custody.iter() {
            rows.remove(sim_id);
        }
        let mut picked_up = Vec::new();
        for sim_id in &carried {
            if let Some(old) = rows.insert(sim_id.clone(), OccurrenceWhereabouts::InCustody) {
                picked_up.push((sim_id.clone(), old));
            }
        }
        self.custody = Arc::new(carried);
        // A carried id that was lying somewhere is no longer placed there.
        for (sim_id, old) in picked_up {
            self.reindex_placed(&sim_id, Some(&old), None);
        }
    }

    /// Admit runtime mints lying in `room` that the ledger has no row for
    /// (OW3).
    ///
    /// A runtime mint has no authored record, so when its room is not live
    /// nothing else can say that it exists. It enters the ledger when it is
    /// minted, not when it is first carried: a boss's dropped gauntlet that
    /// nobody picked up is still lying in the arena when the arena is live
    /// again. An id with a row of any kind is not touched: its row already
    /// says what became of it, and a `Consumed` row is terminal.
    ///
    /// The caller decides what a runtime mint is (its provenance); the
    /// ledger decides only that a mint enters with no earlier row.
    pub fn admit_mints(&mut self, room: &str, mints: BTreeMap<SimId, Vec2>) {
        for (sim_id, at) in mints {
            // Only a new row writes, so shared rows are copied only for one.
            if !self.rows.contains_key(&sim_id) {
                let row = OccurrenceWhereabouts::Placed {
                    room: room.to_string(),
                    at,
                };
                self.reindex_placed(&sim_id, None, Some(&row));
                Arc::make_mut(&mut self.rows).insert(sim_id, row);
            }
        }
    }

    /// State where the occurrences of one room are lying right now, and
    /// REFUSE any id this ledger does not already hold as a live occurrence.
    ///
    /// only ids the caller names are touched, and there is no retraction
    /// arm. A `Placed` row describes a room that may not be loaded, so
    /// "absent from the world" is not evidence of anything — the room is simply
    /// not built. What ends a `Placed` row is the occurrence being picked up
    /// again (custody overwrites it), or a checkpoint restore (which replaces
    /// the whole ledger with its pinned one; a New Game pins an empty one).
    ///
    /// ⛔ **A LIVE OCCURRENCE ENTERS THIS LEDGER THROUGH CUSTODY, OR AS A
    /// RUNTIME MINT THROUGH [`Self::admit_mints`], AND NOWHERE ELSE, and that
    /// rule is enforced HERE because it is the ledger's rule.** An ended one
    /// enters through [`Self::consume`], and only where it has no row.
    /// A placement may be written only for an id whose current row is
    /// `InCustody` (it was in a hand and is being put down) or `Placed` (it is
    /// being republished where it already lies). `None` is refused because an
    /// object nobody ever carried has no relocation to remember — the record
    /// that authored it, or the fact that nothing authored it, is the whole
    /// story. `Consumed` is refused because it is terminal: an ended occurrence
    /// does not come back by being observed lying somewhere.
    ///
    /// this was stated as a comment in the one producer
    /// (`ambition_held_items`, *"that is an invariant, not a filter"*) and
    /// enforced by that producer's own `match`. A rule a caller keeps is a rule
    /// the SECOND caller breaks, and the ledger is the authority on what may be
    /// in it. The producer keeps only the half this cannot decide: whether a
    /// `Placed` row in ANOTHER room is a legitimate relocation or a stale
    /// duplicate, which needs the custody history this method does not see.
    ///
    /// The refusals are returned rather than skipped. A silent veto here
    /// would delete an occurrence from the durable world and look like nothing
    /// happening, so the caller is made to say what it means by them.
    #[must_use = "a refused id is an occurrence the durable world will not remember;                   a caller that drops the refusals has lost it silently"]
    pub fn republish_placements(
        &mut self,
        room: &str,
        placements: BTreeMap<SimId, Vec2>,
    ) -> BTreeSet<SimId> {
        let mut refused = BTreeSet::new();
        for (sim_id, at) in placements {
            let is_live = matches!(
                self.rows.get(&sim_id),
                Some(OccurrenceWhereabouts::InCustody | OccurrenceWhereabouts::Placed { .. })
            );
            if !is_live {
                refused.insert(sim_id);
                continue;
            }
            // A row put down is no longer in custody.
            if self.custody.contains(&sim_id) {
                Arc::make_mut(&mut self.custody).remove(&sim_id);
            }
            let row = OccurrenceWhereabouts::Placed {
                room: room.to_string(),
                at,
            };
            let old = Arc::make_mut(&mut self.rows).insert(sim_id.clone(), row.clone());
            self.reindex_placed(&sim_id, old.as_ref(), Some(&row));
        }
        refused
    }

    /// Remember that these authored occurrences are gone for good. Only an id
    /// with no row is written: a pickup is never carried, so it has no other
    /// row, and a `Consumed` row is terminal. Returns how many rows it wrote.
    pub fn consume(&mut self, sim_ids: impl IntoIterator<Item = SimId>) -> usize {
        let mut written = 0;
        for sim_id in sim_ids {
            if !self.rows.contains_key(&sim_id) {
                Arc::make_mut(&mut self.rows).insert(sim_id, OccurrenceWhereabouts::Consumed);
                written += 1;
            }
        }
        written
    }

    /// Remember that these occurrences, lying in a room, ended there: a bomb
    /// that exploded, a grenade that opened its well. Each `Placed` row
    /// becomes `Consumed`. A row of another kind is not touched: a carried
    /// occurrence did not end where it lay, and a `Consumed` row is already
    /// terminal. Returns the ids whose row it wrote.
    ///
    /// An ended occurrence whose row still placed it would be built again
    /// where it ended, when its room is live again.
    pub fn end(&mut self, sim_ids: impl IntoIterator<Item = SimId>) -> BTreeSet<SimId> {
        let mut ended = BTreeSet::new();
        for sim_id in sim_ids {
            if !matches!(self.rows.get(&sim_id), Some(OccurrenceWhereabouts::Placed { .. })) {
                continue;
            }
            let old = Arc::make_mut(&mut self.rows).insert(sim_id.clone(), OccurrenceWhereabouts::Consumed);
            self.reindex_placed(&sim_id, old.as_ref(), None);
            ended.insert(sim_id);
        }
        ended
    }

    /// Take back the rows of occurrences that never happened: the mints of a
    /// boss defeat that a replay retracted (BOSS-REPLAY-RETRACTION, Q51). Not a
    /// `Consumed` row: a consumed occurrence happened and ended, and a
    /// retracted one did not happen at all, so nothing is left to remember.
    /// Returns the ids that had a row.
    pub fn retract(&mut self, sim_ids: &BTreeSet<SimId>) -> BTreeSet<SimId> {
        let held: BTreeSet<SimId> = sim_ids
            .iter()
            .filter(|sim_id| self.rows.contains_key(*sim_id))
            .cloned()
            .collect();
        if held.is_empty() {
            return held;
        }
        let rows = Arc::make_mut(&mut self.rows);
        let mut removed = Vec::new();
        for sim_id in &held {
            if let Some(old) = rows.remove(sim_id) {
                removed.push((sim_id.clone(), old));
            }
        }
        for (sim_id, old) in removed {
            self.reindex_placed(&sim_id, Some(&old), None);
        }
        if held.iter().any(|sim_id| self.custody.contains(sim_id)) {
            let custody = Arc::make_mut(&mut self.custody);
            for sim_id in &held {
                custody.remove(sim_id);
            }
        }
        held
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// ⛔⛤ THIS CONDITION FIRED, AND THE ANSWER IS NOW THE REGISTRATION.
    ///
    /// It used to read: *"Live custody and placement producers republish their
    /// rows every tick, while room transitions commit beyond the frame-rollback
    /// boundary. If a non-rederived whereabouts state (such as `Consumed`) gains
    /// a producer, this ledger must become registered value state with a
    /// value-sensitive probe."*
    ///
    /// ⇒ It named its own trigger and missed the producer it already had.
    /// [`Self::adopt_rows`] writes rows from a SAVE, and New Game cleared the
    /// ledger from inside the rewinding schedule (until 2026-09-29) — both
    /// authoritative, neither republished.
    /// Measured: one `adopt_rows` call desyncs the sync test within two ticks.
    /// ⇒ So this ledger IS registered value state now
    /// (`rollback_resource_clone_checksum`, with
    /// [`Self::peer_stable_checksum`] as the peer projection), and what is left of
    /// this function is the record of why.
    ///
    /// ⚠ The republish argument was never wrong about the LIVE producers — it
    /// was wrong that they were the only ones.
    pub const fn rewind_argument() {}

    /// THE ONE CANONICAL BYTE ENCODING OF OCCURRENCE ROWS.
    ///
    /// ⛔⛤ **TWO INDEPENDENT ENCODINGS OF THIS STATE EXISTED AND ONE OF THEM WAS
    /// AMBIGUOUS.** The ledger's first peer projection wrote a bare `StateHasher`
    /// fold with no domain and no length prefixes — raw id bytes, then a variant
    /// byte — while [`OccurrenceBaseline::checksum`] was already length-prefixing
    /// the same enum one screen below. Unprefixed concatenation is structurally
    /// ambiguous, not merely untidy: two `InCustody` rows `"a"` and `"b"` write
    /// `a 01 b 01`, and a single row whose id contains those bytes writes the
    /// same stream. `SimId` wraps a `String`, so nothing in the type system
    /// forbids it.
    ///
    /// ⇒ So both projections fold THIS, and a serialization decision about
    /// occurrence rows is made once. A new [`OccurrenceWhereabouts`] variant is a
    /// compile error here rather than a field that quietly stopped being hashed.
    fn encode_rows(&self, out: &mut Vec<u8>) {
        use ambition_platformer2d_core::snapshot::{put_str, put_u64, put_u8, put_vec2};
        put_u64(out, self.rows.len() as u64);
        // `BTreeMap`, so this walk is ordered by identity on every peer. An
        // unordered collection here would report iteration order as divergence.
        for (sim_id, whereabouts) in self.rows.iter() {
            put_str(out, sim_id.as_str());
            match whereabouts {
                OccurrenceWhereabouts::InCustody => put_u8(out, 0),
                OccurrenceWhereabouts::Placed { room, at } => {
                    put_u8(out, 1);
                    put_str(out, room);
                    put_vec2(out, *at);
                }
                OccurrenceWhereabouts::Consumed => put_u8(out, 2),
            }
        }
    }

    /// The ledger's peer checksum projection.
    ///
    /// ⛔⛤ **IT BEGAN AS A DIAGNOSTIC AND IS PEER-MECHANICAL AUTHORITY NOW, WHICH
    /// IS WHY IT IS NO LONGER CALLED `census_projection`.** It existed because
    /// the presence probe this ledger carried could not see a row — a presence
    /// probe on a singleton resource reports `count: 1, xor: 0` however many rows
    /// it holds — and a mid-session load fills the ledger from a SAVE
    /// ([`Self::adopt_rows`]) with nothing to republish it, so the rows survive a
    /// rewind into frames from before the load and reach the hashed save through
    /// `persist_occurrence_horizon_to_save`. Held by
    /// `a_mid_session_load_does_not_reach_back_across_the_rewind`.
    ///
    /// ⭐ **THE DOMAIN IS LOAD-BEARING, NOT HYGIENIC.** `bevy_ggrs` combines every
    /// `ChecksumPart` by XOR and warns in its own source that *"if ... the same
    /// value appears an even number of times, they cancel out (`a ^ a == 0`) and
    /// the desync goes undetected."* [`OccurrenceBaseline::adopt`] copies this
    /// ledger, so EQUAL CONTENTS IS THE STEADY STATE — folding both through one
    /// undomained projection would cancel both entries out of the frame checksum
    /// exactly when they agree, hiding any divergence that moved the two
    /// together. Held by
    /// `the_baseline_and_the_ledger_do_not_cancel_each_other_out`.
    ///
    /// ⚠ A PROJECTION, NOT A `SnapshotState` ENCODING, AND THE DISTINCTION IS
    /// ENFORCED. `bevy_ggrs` stores this value by CLONE, so no encoding is needed
    /// for the snapshot; implementing `SnapshotState` to reach the same fold would
    /// put the type in the encoded set that
    /// `rollback-wire-format-changes-are-declared` watches, for nothing.
    pub fn peer_stable_checksum(&self) -> u64 {
        LEDGER_DIGEST.of(self)
    }

    /// The fold of [`Self::encode_rows`] under `domain`, computed now.
    fn digest(&self, domain: &str) -> u64 {
        use ambition_platformer2d_core::snapshot::PeerDigest;
        let mut bytes = Vec::new();
        self.encode_rows(&mut bytes);
        PeerDigest::in_domain(domain).bytes(&bytes).finish()
    }

    /// Checkpoint baselines copy the entire occurrence ledger. Restoring that copy
    /// makes post-checkpoint custody/placement changes disappear regardless of item
    /// kind. Missing live entities that the baseline says were in custody are
    /// materialized by the custody domain from their authored identity; this ledger
    /// remains a whereabouts record, not a spawn instruction.
    pub const fn baseline_is_a_copy_of_this() {}

    /// A reinstatement is NOT room-local, and the residency it restores is
    /// still not KEYED.
    ///
    /// what is NOT closed is keyed room OWNERSHIP. `RoomScopedEntity` says
    /// an occurrence dies with *a* room, never with *which* room, so residency
    /// still resolves against "whatever room is active". That is exactly right
    /// for today's single-active-room host and is the first thing that breaks
    /// when two participants occupy different rooms at once — at which point
    /// the scope marker owes a room key and this ledger's `Placed { room, .. }`
    /// becomes the thing that names it.
    pub const fn residency_is_reconstructed_not_room_local() {}
}

/// The occurrence domain's share of the reset baseline — horizon 2 of the
/// three, holding exactly what horizon 1 held at the last committed checkpoint.
///
/// a whole-value copy, and that is not laziness. The alternative — record
/// which rows changed since the checkpoint — needs a diff that stays correct
/// across rollback, room streaming and a producer that republishes the whole leg
/// every tick. The ledger is a few dozen rows of two small variants; the copy is
/// the cheap side of that trade by a wide margin.
///
/// ⛔⛤ THIS AND THE LEDGER IT COPIES ARE BOTH REGISTERED VALUE STATE, AND THIS
/// COMMENT USED TO SAY OTHERWISE. It read *"UNLIKE the ledger it copies, this is
/// NOT derived ... every row of `AuthoredOccurrences` is republished from live
/// state, which is what lets it be declared derived"* — and that justification
/// was measured false: [`AuthoredOccurrences::adopt_rows`] writes rows from a
/// SAVE and New Game cleared it from inside the rewinding schedule (until
/// 2026-09-29), neither republished by anything. The ledger is now
/// `rollback_resource_clone_checksum` like this baseline, and both fold
/// [`AuthoredOccurrences::encode_rows`] under their own domain. See
/// [`AuthoredOccurrences::rewind_argument`] for the record of the refuted
/// argument.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct OccurrenceBaseline(AuthoredOccurrences);

impl OccurrenceBaseline {
    /// The remembered ledger. Read by the fixture and by a durable-save writer;
    /// not a place to mutate the baseline from, which is why there is no
    /// `_mut`.
    pub fn remembered(&self) -> &AuthoredOccurrences {
        &self.0
    }

    /// Adopt a ledger as the baseline — the one road that writes this from
    /// outside a [`CheckpointCommitted`](super::CheckpointCommitted).
    ///
    /// it exists for exactly one caller: a durable LOAD. A fresh process
    /// has no checkpoint history at all, so the state the file describes IS its
    /// baseline; leaving the default empty one in place would make the first
    /// death after a load take back everything the save remembered. That is the
    /// same degenerate-case reasoning the sandbox reset uses from the other
    /// side — a host that never commits restores the empty baseline.
    ///
    /// not a general setter. A capture goes through
    /// [`capture_occurrence_baseline`] and reads live state; a road that wrote
    /// the baseline from anywhere else would be a second authority on what a
    /// death restores to.
    pub fn adopt(&mut self, ledger: AuthoredOccurrences) {
        if self.0 != ledger {
            self.0 = ledger;
        }
    }

    /// The desync checksum for this baseline — entity-free, and covering every
    /// field a peer could disagree about.
    ///
    /// ⭐ IT FOLDS THE LEDGER'S OWN ENCODING
    /// ([`AuthoredOccurrences::encode_rows`]) UNDER A DIFFERENT DOMAIN. The rows
    /// are the same shape, so deciding their byte layout twice is how the two
    /// copies drift; the domain is what stops the two `ChecksumPart`s from being
    /// EQUAL, which matters because `bevy_ggrs` XORs them and [`Self::adopt`]
    /// makes equal contents the steady state. See
    /// [`AuthoredOccurrences::peer_stable_checksum`] for the measurement.
    pub fn checksum(&self) -> u64 {
        BASELINE_DIGEST.of(&self.0)
    }
}

/// Record what the world remembers, at the instant a checkpoint commits.
///
/// The absence of a ledger resource entirely is the only case that writes nothing, because
/// there is then nothing in this domain to remember. The baseline is required:
/// `LifecycleCheckpointHorizonPlugin` installs it in the same `build` that
/// schedules this system.
pub fn capture_occurrence_baseline(
    mut commits: bevy::prelude::MessageReader<super::CheckpointCommitted>,
    occurrences: Option<bevy::prelude::Res<AuthoredOccurrences>>,
    mut baseline: ResMut<OccurrenceBaseline>,
) {
    // Drained unconditionally: a commit seen during a load must not be re-read
    // on a later frame and charged to a world that has moved on.
    let committed = commits.read().count() > 0;
    let Some(occurrences) = occurrences else {
        return;
    };
    if !committed {
        return;
    }
    if baseline.0 != *occurrences {
        baseline.0 = occurrences.clone();
    }
}

/// Put the remembered ledger back, on a death.
///
/// this restores the LEDGER and nothing else, on purpose. It does not
/// touch a single occurrence in the world: the rebuild that follows reads the
/// restored ledger and reaches the right answer for every authored record by
/// itself — suppress what the baseline says is elsewhere, reinstate what it says
/// is lying somewhere, author the rest as written. Teaching this system to also
/// fix up live entities would give the world two authorities on the same
/// question, and the whole point of `outlook_for` is that there is one.
///
/// the ONE thing it cannot reach is an occurrence in a hand, because a
/// held occurrence is resident in no room and no rebuild sees it. That leg is
/// `items::pickup::restore_custody_to_checkpoint`, and it belongs to the custody
/// domain because a hand is not room state — including the arm that has to
/// MATERIALIZE an occurrence the world no longer holds an entity for.
/// ⛔⛔ IT RUNS ONLY FROM THE COMMIT, AND READS ONLY WHAT THE COMMIT INSTALLED.
///
/// This once read `ResetToCheckpoint` directly, so a reset the lifecycle slot
/// REFUSED still rolled the ledger back — for a room reconstruction that never
/// happened. A1c/1-2 gave it an admission token to consult; A1c/3b removed the
/// need to consult anything, by moving it into
/// [`CheckpointDomainApply`](super::CheckpointDomainApply), which only the
/// commit executor runs. Nothing it could forget to check remains.
///
/// ⚠ AND IT REDUCES TOWARD THE PINNED POPULATION, not the live baseline. A
/// capture landing between acceptance and commit belongs to the next operation.
pub fn restore_occurrence_baseline(
    inputs: Option<bevy::prelude::Res<super::CheckpointRestoreInputs>>,
    mut occurrences: ResMut<AuthoredOccurrences>,
) {
    let Some(inputs) = inputs else {
        return;
    };
    reduce_occurrences_to_baseline(&inputs.occurrences, &mut occurrences);
}

/// The occurrence domain's reducer: put the remembered ledger back.
///
/// ⭐ SEPARATED FROM ITS TRIGGER so a domain test can hand it a snapshot without
/// standing up a session, a lifecycle slot or a fake reset — and, more
/// importantly, so that doing so is NOT a way around production admission. The
/// system above is the only thing that decides whether this runs for real.
pub fn reduce_occurrences_to_baseline(
    baseline: &OccurrenceBaseline,
    occurrences: &mut AuthoredOccurrences,
) {
    if *occurrences != baseline.0 {
        *occurrences = baseline.0.clone();
    }
}

/// The authored population occurrences that live in a live room other than
/// the room that authored them, which the custody leg holds as carried (Q38).
///
/// The ruling: a respawning population occurrence stays where it is carried
/// while it lives, and its replacement comes from its authored room. So while
/// it lives elsewhere, its home room must not author it again, which is what
/// an `InCustody` row says; and when it dies or its room retires, the row must
/// go, which is what an `InCustody` row does (it is republished from live
/// state every tick). The save keeps no such row, because no hand can be
/// reconstructed for it, so a loaded world builds the occurrence at home.
///
/// A persistent character is not in this set: its whereabouts are durable
/// (`Placed`), written by the body whereabouts producer.
///
/// A crossing's destination is prepared before the crossing commits, and the
/// room the crossing retires takes its away occurrences with it, so that
/// preparation reads the ledger with their custody released
/// ([`AuthoredOccurrences::with_custody_released`]).
///
/// DERIVED, not rollback state: the actor domain writes it each tick from
/// rollback state (the body's live room and its provenance), immediately
/// before its one reader, this projection. Empty or absent in a composition
/// with no actor domain.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq)]
pub struct AwayFromAuthoredRoom(pub BTreeSet<SimId>);

/// Custody is the first thing that gives an occurrence a whereabouts.
///
/// An occurrence a body is carrying is alive and is not in any room, so the room
/// that authored it must not mint a second one. This reads the residency
/// projection rather than items: what it asks is "is this room-scoped occurrence
/// in somebody's custody", and every answer comes from
/// [`InCustodyOf`](super::InCustodyOf), which knows nothing about inventories
/// either.
///
/// recomputed unconditionally, and compared before it writes. It carries no
/// "already applied" gate, so it converges after a rewind on the next step; the
/// equality check keeps change detection quiet on the overwhelming majority of
/// ticks, where nothing is being carried at all.
///
/// Run the other way round, the custody retraction erases the only evidence that the object was
/// ever carried, and the placement producer — which deliberately tracks only occurrences the
/// ledger already remembers — never starts.
///
/// the set is a `BTreeSet`, not the query's order. Bevy's iteration order
/// is an archetype accident and this value reaches a construction plan, so an
/// unordered read here would be a determinism bug that reproduces perfectly on
/// one machine.
/// ⚠ **`InCustodyOf` HAS TWO PRODUCERS AND THEY DIFFER IN DURABILITY, which nothing
/// marks at the marker.** The item domain writes it for held items (which also carry
/// `ItemCustody`); `project_body_custody` writes it for riders, limbs and possessed
/// BODIES, explicitly `Without<GroundItem>`. Both land in the set below, so both get an
/// `InCustody` occurrence row — but `durable_horizon`'s save filter keeps such a row only
/// when the subject also has `ItemCustody`, so the BODY rows are dropped on the way to
/// the file.
///
/// ⇒ That is correct, not a leak: a mount's grip or a possession is session state and the
/// save does not restore a rider onto a mount, so a durable "somebody is holding this"
/// row would be a claim the loader cannot honour. ⭐ It is recorded here because the
/// DURABILITY of a row this function writes depends on WHICH PRODUCER wrote the marker,
/// and that is recoverable today only by reading a filter two crates away.
pub fn project_custody_onto_authored_occurrences(
    carried: Query<&SimId, (With<InCustodyOf>, With<RoomScopedEntity>)>,
    away: Option<Res<AwayFromAuthoredRoom>>,
    // Required: `HeldItemSimulationPlugin` adds this system and the ledger
    // together.
    mut occurrences: ResMut<AuthoredOccurrences>,
) {
    let alive: BTreeSet<SimId> = carried
        .iter()
        .chain(away.iter().flat_map(|away| away.0.iter()))
        .cloned()
        .collect();
    if *occurrences.in_custody() != alive {
        occurrences.republish_custody(alive);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FI9: the custody index is the `InCustody` rows after every mutator. The
    /// custody producer reads the index and not the rows, so an index that
    /// kept an id after its row was put down would keep that occurrence in a
    /// hand forever.
    #[test]
    fn the_custody_index_is_the_custody_rows_after_every_mutation() {
        let filtered = |ledger: &AuthoredOccurrences| -> BTreeSet<SimId> {
            ledger
                .rows()
                .filter(|(_, whereabouts)| matches!(whereabouts, OccurrenceWhereabouts::InCustody))
                .map(|(sim_id, _)| sim_id.clone())
                .collect()
        };
        let (a, b, c) = (SimId::placement("a"), SimId::placement("b"), SimId::placement("c"));
        let mut ledger = AuthoredOccurrences::default();
        let mut steps = 0;
        let placed = |ledger: &AuthoredOccurrences| -> BTreeMap<String, BTreeSet<SimId>> {
            let mut placed: BTreeMap<String, BTreeSet<SimId>> = BTreeMap::new();
            for (sim_id, whereabouts) in ledger.rows() {
                if let OccurrenceWhereabouts::Placed { room, .. } = whereabouts {
                    placed.entry(room.clone()).or_default().insert(sim_id.clone());
                }
            }
            placed
        };
        let mut check = |ledger: &AuthoredOccurrences, step: &str| {
            assert_eq!(*ledger.in_custody(), filtered(ledger), "after {step}");
            assert_eq!(*ledger.placed, placed(ledger), "the placement index after {step}");
            steps += 1;
        };
        ledger.republish_custody([a.clone(), b.clone()].into_iter().collect());
        check(&ledger, "two carried");
        assert!(ledger
            .republish_placements("room", [(a.clone(), Vec2::new(1.0, 2.0))].into_iter().collect())
            .is_empty());
        check(&ledger, "one put down");
        ledger.admit_mints("room", [(c.clone(), Vec2::ZERO), (b.clone(), Vec2::ZERO)].into_iter().collect());
        check(&ledger, "a mint admitted beside a carried id");
        assert!(ledger
            .republish_placements("next door", [(a.clone(), Vec2::new(3.0, 4.0))].into_iter().collect())
            .is_empty());
        check(&ledger, "one moved to another room");
        assert_eq!(ledger.end([a.clone(), b.clone()]), [a.clone()].into_iter().collect());
        check(&ledger, "a placed id ended, a carried one kept");
        ledger.republish_custody([c.clone()].into_iter().collect());
        check(&ledger, "the carried set replaced");
        assert_eq!(ledger.retract(&[c.clone()].into_iter().collect()), [c.clone()].into_iter().collect());
        check(&ledger, "a carried id retracted");
        ledger.adopt_rows(
            [(a.clone(), OccurrenceWhereabouts::InCustody), (b.clone(), OccurrenceWhereabouts::Consumed)]
                .into_iter()
                .collect(),
        );
        check(&ledger, "rows adopted");
        assert_eq!(*ledger.in_custody(), [a].into_iter().collect::<BTreeSet<_>>());
        assert_eq!(steps, 8);
    }

    /// A checkpoint's older ledger, with the occurrences that still live in a
    /// room the restore does not rewind held as carried (Q38). The ledger it
    /// was made from is not changed, the rows it already carried stay
    /// carried, and an id it had placed is now carried: the live body is the
    /// newer fact.
    #[test]
    fn a_ledger_with_custody_held_carries_the_held_ids_and_keeps_its_own() {
        let (kept, placed, away) = (SimId::placement("kept"), SimId::placement("placed"), SimId::placement("away"));
        let mut checkpoint = AuthoredOccurrences::default();
        checkpoint.republish_custody([kept.clone(), placed.clone()].into_iter().collect());
        assert!(checkpoint
            .republish_placements("room", [(placed.clone(), Vec2::new(1.0, 2.0))].into_iter().collect())
            .is_empty());
        let held = checkpoint.with_custody_held(&[placed.clone(), away.clone()].into_iter().collect());
        assert_eq!(*held.in_custody(), [kept.clone(), placed.clone(), away.clone()].into_iter().collect::<BTreeSet<_>>());
        assert_eq!(held.whereabouts(&placed), Some(&OccurrenceWhereabouts::InCustody));
        assert_eq!(*checkpoint.in_custody(), [kept.clone()].into_iter().collect::<BTreeSet<_>>(), "the source ledger changed");
        assert_eq!(
            checkpoint.with_custody_held(&[kept.clone()].into_iter().collect()).rows().count(),
            checkpoint.rows().count(),
            "holding what is already carried changes nothing"
        );
    }

    /// A mint with no row enters where it lies; an id that has a row keeps it.
    /// A row already says what became of the occurrence, and an admission
    /// that wrote over it would move a carried mint back to the floor.
    #[test]
    fn a_mint_enters_where_it_lies_and_a_known_id_keeps_its_row() {
        let new = SimId::placement("boss/drop/weapon");
        let carried = SimId::placement("other/drop/weapon");
        let mut ledger = AuthoredOccurrences::default();
        ledger.republish_custody([carried.clone()].into_iter().collect());
        ledger.admit_mints(
            "arena",
            [(new.clone(), Vec2::new(10.0, 20.0)), (carried.clone(), Vec2::new(30.0, 40.0))]
                .into_iter()
                .collect(),
        );
        assert_eq!(
            ledger.whereabouts(&new),
            Some(&OccurrenceWhereabouts::Placed { room: "arena".into(), at: Vec2::new(10.0, 20.0) }),
        );
        assert_eq!(ledger.whereabouts(&carried), Some(&OccurrenceWhereabouts::InCustody));
    }

    /// ONE ROW, TWO ROOMS, TWO OPPOSITE ANSWERS — and both are asserted.
    ///
    /// An occurrence carried out of the room that authored it and put down next
    /// door is exactly one fact. The room it lies in owes the world that
    /// occurrence; the room whose record minted it owes the world nothing, and
    /// authoring it again would put two live things behind one `SimId`.
    ///
    /// asserting only the suppression is the dangerous half, because a
    /// ledger that suppressed everywhere would pass it and would delete the
    /// object from the world permanently. Both terms are observed here, from
    /// the same row, so neither arm can regress alone.
    #[test]
    fn a_placed_row_reinstates_where_it_lies_and_suppresses_where_it_was_authored() {
        let axe = SimId::placement("blink_run_pickup");
        let mut ledger = AuthoredOccurrences::default();
        // Carried out of the authoring room first: a placement is only ever
        // reachable through custody, and the ledger enforces it.
        ledger.republish_custody([axe.clone()].into_iter().collect());
        assert!(ledger
            .republish_placements(
                "portal_bridge",
                [(axe.clone(), Vec2::new(48.0, 96.0))].into_iter().collect(),
            )
            .is_empty());

        let lying_in = ledger.outlook_for("portal_bridge");
        assert_eq!(
            lying_in.disposition(&axe),
            OccurrenceDisposition::Reinstated {
                at: Vec2::new(48.0, 96.0)
            },
            "the room the occurrence is lying in must rebuild it, where it lies",
        );
        assert_eq!(
            lying_in.reinstatements().get(&axe).copied(),
            Some(Vec2::new(48.0, 96.0)),
            "and it must say so through the obligation list construction reads, \
             which is what carries the position to a record that has none",
        );
        assert!(
            lying_in.suppressed().is_empty(),
            "nothing is suppressed in the room that owes the occurrence"
        );

        let authored_by = ledger.outlook_for("blink_run");
        assert_eq!(
            authored_by.disposition(&axe),
            OccurrenceDisposition::Suppressed,
            "the room whose record minted it must NOT mint a second one: the \
             occurrence exists, next door",
        );
        assert!(authored_by.reinstatements().is_empty());

        // AND A ROOM WITH NO STAKE IN THE ROW IS NOT ASKED TO DO ANYTHING
        // DIFFERENT FROM THE HOME ROOM. Both suppress, because neither is where
        // the occurrence is — an outlook that answered `Authored` for a third
        // room would re-author the record on any road that happened to hold it.
        assert_eq!(
            ledger.outlook_for("somewhere_else").disposition(&axe),
            OccurrenceDisposition::Suppressed,
        );
    }

    /// A record nobody has touched has no row, and no row is `Authored`. This
    /// is the ordinary state of every room in the game, and it is what keeps
    /// the suppression above from being a way to lose an object nobody moved.
    #[test]
    fn an_untouched_record_has_the_default_disposition() {
        let ledger = AuthoredOccurrences::default();
        assert!(ledger.outlook_for("blink_run").is_empty());
        assert_eq!(
            ledger
                .outlook_for("blink_run")
                .disposition(&SimId::placement("blink_run_pickup")),
            OccurrenceDisposition::Authored,
        );
    }

    /// ⛔ THE LEDGER REFUSES AN ID IT NEVER HELD, and says which.
    ///
    /// An occurrence enters through custody and nowhere else. Something that
    /// entered the world already lying on the ground — a death drop, a spawned
    /// reward — has no relocation to remember, and writing a `Placed` row for
    /// it would give the durable world an object no record authored and no hand
    /// ever carried.
    ///
    /// ⚠ THE REFUSAL IS RETURNED, not skipped. A silent veto here deletes an
    /// occurrence from the durable world and looks like nothing happening, so
    /// the caller is made to see it — which is why this asserts the id comes
    /// back and not merely that no row appeared.
    #[test]
    fn a_placement_is_refused_for_an_occurrence_that_was_never_in_custody() {
        let never_carried = SimId::death_drop(&SimId::placement("trex_boss"), "weapon");
        let mut ledger = AuthoredOccurrences::default();

        let refused = ledger.republish_placements(
            "blink_run",
            [(never_carried.clone(), Vec2::new(320.0, 96.0))]
                .into_iter()
                .collect(),
        );

        assert_eq!(
            refused,
            [never_carried.clone()].into_iter().collect::<BTreeSet<_>>(),
            "the refusal names the occurrence the durable world will not remember"
        );
        assert_eq!(
            ledger.whereabouts(&never_carried),
            None,
            "and it wrote no row: a refusal is not a half-write"
        );
    }

    /// A `Consumed` row is TERMINAL, and being seen lying somewhere does not
    /// reopen it.
    ///
    /// this is the arm a `remembers()`-style check would get wrong: the
    /// ledger holds a row for this id, so "do I know about it" answers yes, and
    /// the right question is "is it a LIVE occurrence".
    #[test]
    fn a_consumed_occurrence_is_not_resurrected_by_a_placement() {
        let key = SimId::placement("vault_key");
        let mut ledger = AuthoredOccurrences::default();
        ledger.adopt_rows(
            [(key.clone(), OccurrenceWhereabouts::Consumed)]
                .into_iter()
                .collect(),
        );

        let refused = ledger.republish_placements(
            "blink_run",
            [(key.clone(), Vec2::ZERO)].into_iter().collect(),
        );

        assert!(refused.contains(&key), "an ended occurrence stays ended");
        assert_eq!(
            ledger.whereabouts(&key),
            Some(&OccurrenceWhereabouts::Consumed),
            "and the terminal row is untouched"
        );
    }

    /// The two roads a placement IS allowed to take, so the guard above cannot
    /// pass by refusing everything.
    #[test]
    fn a_carried_occurrence_may_be_put_down_and_republished_where_it_lies() {
        let axe = SimId::placement("blink_run_pickup");
        let mut ledger = AuthoredOccurrences::default();
        ledger.republish_custody([axe.clone()].into_iter().collect());

        let put_down = ledger.republish_placements(
            "portal_bridge",
            [(axe.clone(), Vec2::new(48.0, 96.0))].into_iter().collect(),
        );
        assert!(put_down.is_empty(), "out of a hand and onto the floor");

        // And again the next tick, from the `Placed` row it now holds.
        let still_there = ledger.republish_placements(
            "portal_bridge",
            [(axe.clone(), Vec2::new(50.0, 96.0))].into_iter().collect(),
        );
        assert!(
            still_there.is_empty(),
            "a republish of something already lying there is the common case"
        );
        assert_eq!(
            ledger.whereabouts(&axe),
            Some(&OccurrenceWhereabouts::Placed {
                room: "portal_bridge".to_string(),
                at: Vec2::new(50.0, 96.0),
            }),
        );
    }

    /// One `InCustody` row per id, from raw ids.
    fn ledger_of(ids: &[&str]) -> AuthoredOccurrences {
        let mut ledger = AuthoredOccurrences::default();
        ledger.adopt_rows(
            ids.iter()
                .map(|id| {
                    (
                        SimId::from_snapshot((*id).to_string()),
                        OccurrenceWhereabouts::InCustody,
                    )
                })
                .collect(),
        );
        ledger
    }

    /// The projection the ledger shipped with before it was domain-separated:
    /// raw id bytes, then a variant byte, with no count and no length prefixes.
    fn the_ambiguous_encoding(ledger: &AuthoredOccurrences) -> u64 {
        use ambition_platformer2d_core::snapshot::StateHasher;
        let mut hasher = StateHasher::default();
        for (id, whereabouts) in ledger.rows() {
            hasher.write(id.as_str().as_bytes());
            match whereabouts {
                OccurrenceWhereabouts::InCustody => hasher.write(&[1]),
                OccurrenceWhereabouts::Placed { room, at } => {
                    hasher.write(&[2]);
                    hasher.write(room.as_bytes());
                    hasher.write(&at.x.to_bits().to_le_bytes());
                    hasher.write(&at.y.to_bits().to_le_bytes());
                }
                OccurrenceWhereabouts::Consumed => hasher.write(&[3]),
            }
        }
        hasher.finish()
    }

    /// ⛔⛤ TWO LEDGERS THAT ARE NOT THE SAME WORLD MUST NOT HASH THE SAME, AND
    /// THE FIRST ENCODING MADE THEM.
    ///
    /// Unprefixed concatenation lets a row boundary be re-read as content. Two
    /// `InCustody` rows `"a"` and `"b"` wrote `a 01 b 01`; ONE row whose id is
    /// `"a\x01b"` wrote the same four bytes. `SimId::from_snapshot` takes a
    /// `String`, so nothing forbids the id.
    ///
    /// ⭐ THE OLD ENCODING IS REBUILT HERE AND ASSERTED TO COLLIDE. Without that
    /// half, this test would pass against any encoding at all — including one
    /// that is ambiguous somewhere else — and would say nothing about what was
    /// fixed.
    #[test]
    fn a_row_boundary_cannot_be_re_read_as_part_of_an_id() {
        let two = ledger_of(&["a", "b"]);
        let one = ledger_of(&["a\u{1}b"]);
        assert_eq!(two.rows().count(), 2, "the two-row ledger lost a row");
        assert_eq!(one.rows().count(), 1, "the one-row ledger gained a row");

        // The premise: these two really did collide, so the fix has a subject.
        assert_eq!(
            the_ambiguous_encoding(&two),
            the_ambiguous_encoding(&one),
            "the encoding this test was written against does NOT collide on \
             these inputs, so it is no longer the encoding that shipped and \
             this arm is measuring nothing"
        );

        assert_ne!(
            two.peer_stable_checksum(),
            one.peer_stable_checksum(),
            "a two-row ledger and a one-row ledger hash the same, so a row \
             boundary is still being re-read as id content"
        );
    }

    /// ⛔⛤ THE BASELINE AND THE LEDGER MUST NOT PRODUCE THE SAME CHECKSUM WHEN
    /// THEY AGREE, BECAUSE `bevy_ggrs` XORS THE PARTS.
    ///
    /// Its own source says so: *"if ... the same value appears an even number of
    /// times, they cancel out (`a ^ a == 0`) and the desync goes undetected."*
    /// [`OccurrenceBaseline::adopt`] copies the ledger, so agreement is the
    /// STEADY STATE rather than a corner case — one shared undomained projection
    /// would remove both entries from the frame checksum for as long as they
    /// match, and hide exactly the divergence that moves the two together.
    ///
    /// ⚠ THE ROWS ARE DELIBERATELY IDENTICAL. Any difference in contents would
    /// make the digests differ for the wrong reason and the arm would pass
    /// against a single shared domain.
    #[test]
    fn the_baseline_and_the_ledger_do_not_cancel_each_other_out() {
        let mut ledger = AuthoredOccurrences::default();
        ledger.adopt_rows(
            [(
                SimId::placement("vault_key"),
                OccurrenceWhereabouts::Placed {
                    room: "vault".to_string(),
                    at: Vec2::new(12.0, 34.0),
                },
            )]
            .into_iter()
            .collect(),
        );
        let mut baseline = OccurrenceBaseline::default();
        baseline.adopt(ledger.clone());

        let ledger_part = ledger.peer_stable_checksum();
        let baseline_part = baseline.checksum();
        assert_ne!(
            ledger_part, baseline_part,
            "the baseline and the ledger it copied fold to the same value, so \
             their two ChecksumParts XOR to zero and neither reaches the frame \
             checksum while they agree"
        );
        assert_ne!(
            ledger_part ^ baseline_part,
            0,
            "the two parts cancel under XOR, which is how bevy_ggrs combines them"
        );

        // AND THE ENCODING IS STILL SHARED: a change in the rows must move both.
        let mut moved = ledger.clone();
        moved.adopt_rows(
            [(SimId::placement("vault_key"), OccurrenceWhereabouts::Consumed)]
                .into_iter()
                .collect(),
        );
        let mut moved_baseline = OccurrenceBaseline::default();
        moved_baseline.adopt(moved.clone());
        assert_ne!(
            moved.peer_stable_checksum(),
            ledger_part,
            "changing a row did not move the ledger's checksum"
        );
        assert_ne!(
            moved_baseline.checksum(),
            baseline_part,
            "changing a row did not move the baseline's checksum, so the two are \
             no longer folding the same encoding"
        );
    }

    /// A ledger of `n` dormant `Placed` rows.
    fn dormant_ledger(n: usize) -> AuthoredOccurrences {
        let mut ledger = AuthoredOccurrences::default();
        ledger.adopt_rows(
            (0..n)
                .map(|i| {
                    (
                        SimId::placement(&format!("dormant/{i:05}")),
                        OccurrenceWhereabouts::Placed { room: "elsewhere".into(), at: Vec2::new(i as f32, 0.0) },
                    )
                })
                .collect(),
        );
        ledger
    }

    /// M2 cut C: the kept checksum is the fold it stands for, in each
    /// domain. Each ledger and its baseline checksum equal the fold computed
    /// now, after a row changes, and after the ledger goes back to rows it held.
    #[test]
    fn a_kept_checksum_is_the_fold_of_the_rows_it_was_kept_for() {
        let first = dormant_ledger(3);
        let mut second = first.clone();
        assert!(second
            .republish_placements("elsewhere", [(SimId::placement("dormant/00001"), Vec2::new(9.0, 9.0))].into_iter().collect())
            .is_empty());
        for (step, ledger) in [("first", &first), ("second", &second), ("first again", &first)] {
            assert_eq!(
                ledger.peer_stable_checksum(),
                ledger.digest("lifecycle.authored_occurrences"),
                "the ledger's checksum is not its fold ({step})"
            );
            let mut baseline = OccurrenceBaseline::default();
            baseline.adopt(ledger.clone());
            assert_eq!(
                baseline.checksum(),
                ledger.digest("lifecycle.occurrence_baseline"),
                "the baseline's checksum is not its fold ({step})"
            );
        }
        assert_ne!(first.peer_stable_checksum(), second.peer_stable_checksum(), "a moved row did not move the checksum");
    }

    /// M2 cut C: a clone of a ledger with 10,000 dormant rows, its checksum,
    /// and a comparison with the live ledger cost little more than with none.
    /// The rollback host does all three on every frame. Before the cut, a
    /// clone and a checksum each cost about 1.4 ms at 10,000 rows.
    #[test]
    fn dormant_rows_add_little_to_a_snapshot_of_the_ledger() {
        fn median_snapshot(ledger: &AuthoredOccurrences) -> std::time::Duration {
            let mut times: Vec<_> = (0..101)
                .map(|_| {
                    // AMBITION_REVIEW(determinism): wall clock, in a test. It
                    // measures the cost of a snapshot, and no simulation code
                    // reads it.
                    let start = std::time::Instant::now();
                    let snapshot = std::hint::black_box(ledger.clone());
                    std::hint::black_box(snapshot.peer_stable_checksum());
                    std::hint::black_box(snapshot == *ledger);
                    start.elapsed()
                })
                .collect();
            times.sort();
            times[50]
        }
        let (empty, full) = (dormant_ledger(0), dormant_ledger(10_000));
        let (empty, full) = (median_snapshot(&empty), median_snapshot(&full));
        assert!(
            full < empty * 5 + std::time::Duration::from_micros(20),
            "a snapshot of 10,000 dormant rows took {full:?}, against {empty:?} with none"
        );
    }
}
