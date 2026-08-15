//! **Where the occurrence an authored definition minted actually IS.**
//!
//! ⭐ **authored DEFINITION identity is not runtime OCCURRENCE identity.** A room
//! is a set of authored records; building it MINTS one occurrence per record and
//! stamps it with the record's identity ([`SimId::placement`](crate::sim_id::SimId::placement)).
//! Rebuilding the room asks a question the authored record cannot answer on its
//! own: *what happened to the occurrence I minted last time?* Every road that
//! reconstructs a world has to answer it — a carried object, a body that walked
//! somewhere else, a mechanism that was opened, an object that was destroyed for
//! good, a quest item that was moved — and until now the answer was implicit and
//! always the same, because the room boundary destroyed everything it had
//! minted, so "author it again" was right by accident.
//!
//! ⛔ **the accident ended on 2026-08-15.** `InCustodyOf` suspended the RESIDENCY
//! of a carried object so it crosses a room boundary alive. Re-entering the room
//! then ran authored construction again and minted a SECOND occurrence claiming
//! the same `SimId::placement(..)` — two live things with one identity, which is
//! the failure `SimId` exists to make impossible.
//!
//! # THREE HORIZONS, and this file owns exactly one
//!
//! An occurrence's whereabouts is asked at three different times, and the three
//! answers are three different values with three different owners. Conflating
//! any two of them is the defect pattern this repository keeps paying for.
//!
//! 1. ⭐ **CURRENT** — what is true now. Owned HERE, by
//!    [`AuthoredOccurrences`]. Its loaded window is the live world (an entity
//!    with its `ItemCustody`, its position, its components); this ledger is the
//!    part of the current world that the loaded window cannot hold, because the
//!    room it describes is not built. Every row is republished from live state
//!    while that state is loaded — see [`AuthoredOccurrences::rewind_argument`].
//! 2. ⚠ **BASELINE** — what a death/retry restores to: the current whereabouts
//!    as they stood at the last committed checkpoint. **It does not exist**, and
//!    it is a COPY of this ledger, not a second kind of row. See
//!    [`AuthoredOccurrences::baseline_is_a_copy_of_this`] for what would have to
//!    exist and why an item-KIND rule is the wrong shape.
//! 3. ⚠ **DURABLE SAVE** — the same value across process lifetimes. Also does
//!    not exist for occurrences; `AmbitionGameSave` records a
//!    `PersistedCheckpoint` (a room id and a body position) and nothing about
//!    what became of anything the world authored.
//!
//! ⭐ **(2) and (3) are COPIES of (1) at later horizons.** That ordering is
//! forced, not chosen: you cannot commit a baseline of a value you cannot
//! represent, so the representation problem is solved once, here, and the other
//! two horizons are a clone and a serialization of it.
//!
//! ⛔ **it is deliberately NOT a universal instance registry.** It records
//! nothing about occurrences whose whereabouts are the default; an empty ledger
//! is the ordinary state of every room in the game, and the only rows in it are
//! the ones some system had a reason to write. That is also what makes a
//! baseline copy of it cheap and complete: "no row" means "as authored", which
//! is the correct baseline for everything nobody has touched.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::{Query, ResMut, Resource, Vec2, With};

use super::{InCustodyOf, RoomScopedEntity};
use crate::sim_id::SimId;

/// **Where one authored record's occurrence is**, when it is somewhere other
/// than where the record puts it.
///
/// ⚠ **absence is a variant and it is the common one.** No row means "as
/// authored", which is the state of essentially every record essentially
/// always. Nothing here is a status enum for its own sake: each variant exists
/// because reconstruction reaches a different decision from it.
#[derive(Clone, Debug, PartialEq)]
pub enum OccurrenceWhereabouts {
    /// **In a body's hands.** Alive, in no room, crossing boundaries with
    /// whoever carries it. Written by [`project_custody_onto_authored_occurrences`]
    /// from [`InCustodyOf`], which knows nothing about items.
    ///
    /// ⚠ **this row is REPUBLISHED from live state every tick**, so it retracts
    /// by itself when custody ends or when the carrier is destroyed. It is the
    /// one row a rewind can never strand.
    InCustody,
    /// **Lying in `room`, at `at`.** The occurrence was carried somewhere and
    /// put down; `at` is where it came to rest, republished while `room` is
    /// loaded and FROZEN at the value it last held when `room` unloads.
    ///
    /// ⭐ **this is the row that makes relocation durable**, and the only row
    /// whose value outlives the thing it describes.
    Placed { room: String, at: Vec2 },
    /// **Gone for good.** A consumed key, a destroyed mechanism, a body killed
    /// in a way the world is supposed to remember. Distinct from "no row" —
    /// which is also "not alive" — because an ordinary room unload destroys
    /// occurrences by the dozen and every one of them SHOULD come back.
    ///
    /// ⛔ **no producer today, and that is load-bearing for
    /// [`AuthoredOccurrences::rewind_argument`].** Every other row is either
    /// republished from live state or frozen only at a room boundary; this one
    /// would be written mid-frame from an event, so the day it gains a producer
    /// the ledger owes a real rollback registration with a VALUE projection.
    Consumed,
}

/// **What reconstruction must do about one authored record**, in the room it is
/// currently building. A DERIVED value: [`AuthoredOccurrences::outlook_for`]
/// computes it, nothing stores it.
///
/// ⭐ **there are THREE answers, not two.** "Author it" and "do not author it"
/// were enough only while an occurrence could never be anywhere but where its
/// record put it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum OccurrenceDisposition {
    /// **Author it, from the record, as written.** The default, and what every
    /// record without a row gets.
    #[default]
    Authored,
    /// **Author it from the record, but AT `at`.** The occurrence belongs to
    /// this room and is not where the record puts it — it was put down
    /// somewhere else in this room and the room has been unloaded since.
    ///
    /// ⭐ the identity is the record's, so the reconstituted occurrence carries
    /// the same `SimId::placement(..)`: this is the SAME occurrence coming back,
    /// not a copy of it authored at new coordinates.
    Reinstated { at: Vec2 },
    /// **Do not author it.** The occurrence is alive somewhere this room is not,
    /// or it is deliberately gone. Minting a fresh one would put two live things
    /// behind one identity, or resurrect something the world remembers killing.
    Suppressed,
}

impl OccurrenceDisposition {
    /// Whether construction produces an occurrence for this record at all.
    ///
    /// ⚠ TRUE for [`Self::Reinstated`]: a reinstatement is an authoring, with
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

/// **The derived view ONE room's construction consumes.**
///
/// ⭐ **derived, room-scoped, and frozen onto the plan it produced.** A prepared
/// plan is only valid for the world that produced it — a plan prepared while an
/// object was being carried deliberately omits that object, and committing it
/// after the object was put down would leave the room permanently short of a
/// thing it authors. So the view a plan was prepared against travels WITH the
/// plan and a cache compares it rather than guessing.
///
/// ⚠ it holds only the records whose disposition is not the default, so the
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

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// **THE authoritative owner of occurrence whereabouts** — horizon 1 of the
/// three in this module's header.
///
/// ⚠ **a world with no such resource at all means "nothing is remembered"**,
/// which is why every consumer takes it as an `Option` and a composition that
/// never picks anything up carries an empty ledger through its whole life.
///
/// ⛔ **the TYPE NAME is load-bearing and cannot be changed here.**
/// `ambition_platformer2d_runtime::rollback::domains::primitives` names this
/// path in its declaration, and that module is owned by another lane. The name
/// says `AuthoredOccurrences` because that is what it was when it held one
/// derived set; what it holds now is stated by [`OccurrenceWhereabouts`].
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct AuthoredOccurrences {
    rows: BTreeMap<SimId, OccurrenceWhereabouts>,
}

impl AuthoredOccurrences {
    /// **What construction must do about every record of `room`.** The one
    /// derived view, produced from the authoritative rows and nothing else.
    ///
    /// ⭐ **a `Placed` row for a room OTHER than the one being built answers
    /// `Authored`, and that is deliberate, not an oversight.** Exactly one room
    /// is loaded, so an occurrence placed in a different room is not alive: it
    /// was destroyed when that room unloaded, and nothing can reconstitute it
    /// there until construction can read a record whose home room is not the
    /// room being built (see the gate on [`Self::reinstatement_is_room_local`]).
    /// Suppressing here instead would trade a duplication bug for a DELETION
    /// bug: the object would exist in no room at all, forever.
    pub fn outlook_for(&self, room: &str) -> RoomOccurrenceOutlook {
        let rows = self
            .rows
            .iter()
            .filter_map(|(sim_id, whereabouts)| {
                let disposition = match whereabouts {
                    OccurrenceWhereabouts::InCustody | OccurrenceWhereabouts::Consumed => {
                        OccurrenceDisposition::Suppressed
                    }
                    OccurrenceWhereabouts::Placed { room: at_room, at } if at_room == room => {
                        OccurrenceDisposition::Reinstated { at: *at }
                    }
                    OccurrenceWhereabouts::Placed { .. } => return None,
                };
                Some((sim_id.clone(), disposition))
            })
            .collect();
        RoomOccurrenceOutlook { rows }
    }

    /// Read one row. For producers deciding whether they have anything to say.
    pub fn whereabouts(&self, sim_id: &SimId) -> Option<&OccurrenceWhereabouts> {
        self.rows.get(sim_id)
    }

    /// Whether anything is remembered about this occurrence at all — the
    /// question a placement producer asks before it starts tracking one.
    pub fn remembers(&self, sim_id: &SimId) -> bool {
        self.rows.contains_key(sim_id)
    }

    /// The ids currently recorded as carried. Compared before a republish so
    /// change detection stays quiet on the overwhelming majority of ticks.
    pub fn in_custody(&self) -> BTreeSet<SimId> {
        self.rows
            .iter()
            .filter(|(_, whereabouts)| matches!(whereabouts, OccurrenceWhereabouts::InCustody))
            .map(|(sim_id, _)| sim_id.clone())
            .collect()
    }

    /// **Republish the whole custody leg.**
    ///
    /// ⛔ **RETRACT BY RESETTING, NEVER BY REMOVING.** There is no "this one
    /// stopped being carried" call, because a ledger that is edited row by row
    /// drifts from the world it describes the first time a row's retraction has
    /// no event behind it — an entity despawned by something that never heard of
    /// custody, a rollback that rewound past the pickup. The whole leg is
    /// replaced by what is true now, so an id absent from `carried` has had its
    /// custody row dropped by the same write that kept the others.
    ///
    /// ⚠ **it touches ONLY custody rows.** A `Placed` row is not a custody row
    /// that went missing; it is a different fact, written by a different
    /// producer, and a whole-ledger republish here would delete relocation the
    /// instant a hand emptied.
    pub fn republish_custody(&mut self, carried: BTreeSet<SimId>) {
        self.rows
            .retain(|_, whereabouts| !matches!(whereabouts, OccurrenceWhereabouts::InCustody));
        for sim_id in carried {
            self.rows.insert(sim_id, OccurrenceWhereabouts::InCustody);
        }
    }

    /// **State where the occurrences of one room are lying right now.**
    ///
    /// ⭐ **only ids the caller names are touched, and there is no retraction
    /// arm.** A `Placed` row describes a room that may not be loaded, so
    /// "absent from the world" is not evidence of anything — the room is simply
    /// not built. What ends a `Placed` row is the occurrence being picked up
    /// again (custody overwrites it), a reset ([`Self::forget_everything`]), or
    /// the [`OccurrenceWhereabouts::Consumed`] producer that does not exist yet.
    pub fn republish_placements(&mut self, room: &str, placements: BTreeMap<SimId, Vec2>) {
        for (sim_id, at) in placements {
            self.rows.insert(
                sim_id,
                OccurrenceWhereabouts::Placed {
                    room: room.to_string(),
                    at,
                },
            );
        }
    }

    /// **A reset remembers nothing.**
    ///
    /// The sandbox reset destroys the occurrences a whereabouts row is about —
    /// the room AND the hand — and rebuilds the start room from its authored
    /// records alone. A surviving row would put a relocated object back at
    /// coordinates from a world that no longer exists.
    ///
    /// ⭐ **and this is the degenerate case of the BASELINE horizon**: a reset is
    /// "restore the empty baseline". See
    /// [`Self::baseline_is_a_copy_of_this`].
    pub fn forget_everything(&mut self) {
        if !self.rows.is_empty() {
            self.rows.clear();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// **Why this is still declared DERIVED to the rollback registry, and the
    /// exact condition that would make that a lie.**
    ///
    /// Every row is republished from live state on every tick that the state is
    /// live: [`project_custody_onto_authored_occurrences`] rebuilds the custody
    /// leg from `InCustodyOf`, and the placement producer rebuilds a `Placed`
    /// row from the occurrence's own position while its room is loaded. A rewind
    /// therefore restores the world and the next step restores the ledger.
    ///
    /// ⭐ **the one value a rewind cannot recompute is a `Placed` row whose room
    /// is not loaded — and a room stops being loaded only at a room TRANSITION,
    /// which a frame rollback never crosses** (a transition commits from
    /// `commit_confirmed_lifecycle`, at a confirmed frame). So the frozen value
    /// is frozen on the far side of a boundary the rewind window does not reach.
    ///
    /// ⛔⛔ **that argument dies the day [`OccurrenceWhereabouts::Consumed`]
    /// gains a producer**, because that row is written mid-frame from an event
    /// and no live state re-derives it. It owes a registration with a real VALUE
    /// projection (the rows, not a presence probe) at the same moment.
    ///
    /// ⚠ **UNVERIFIED**: no rollback host was run against this file. The claim
    /// above is an argument, not a measurement.
    pub const fn rewind_argument() {}

    /// **The BASELINE horizon is a COPY of this ledger, and that is the whole
    /// design.**
    ///
    /// The maintainer's rule (2026-08-15) is that checkpoint state IS the reset
    /// baseline: ordinary traversal preserves current whereabouts; a death
    /// restores whatever was true at the last committed checkpoint. Written as a
    /// copy of this value that is taken at a checkpoint commit and written back
    /// on death, every line of that rule falls out with **no knowledge of what
    /// kind of thing an occurrence is**:
    ///
    /// * a key picked up after checkpoint C0 has an `InCustody` row that the C0
    ///   copy does not; restoring the copy leaves no row, so the pedestal
    ///   authors it again;
    /// * acquiring it and then committing C1 copies the row, so a later death
    ///   restores the row and the pedestal stays empty;
    /// * ⭐ **a temporary item picked up AFTER C1 reverts too, for the same one
    ///   reason** — its row is not in the C1 copy either. This is the line that
    ///   makes the rule checkpoint-shaped rather than item-shaped, and a
    ///   `KeyItem => always persists` rule satisfies the first two and fails it.
    ///
    /// ⛔ **so do not write an item-kind rule.** A key persists because acquiring
    /// it committed a checkpoint, not because of what it is; a kind rule is a
    /// second authority that disagrees with the checkpoint the moment content
    /// changes.
    ///
    /// ⚠ **WHAT WOULD HAVE TO EXIST, and none of it does:**
    /// 1. a checkpoint COMMIT that is a world event rather than a body position.
    ///    `heal_save_shrine_system` writes `PersistedCheckpoint { room, x, y }`
    ///    and nothing else; there is no moment at which a copy could be taken.
    /// 2. a death/retry road that RESTORES a baseline. Today the only road is
    ///    the sandbox reset, which rebuilds the start room from records and is
    ///    the empty baseline, not a restore.
    /// 3. ⛔⛔ **a body INVENTORY.** An `InCustody` row names a live
    ///    relationship, so it cannot be restored on its own: "the key stays
    ///    acquired" is a claim about a hand that a death emptied. `OwnedItems`
    ///    is a process-global count table with no row per object, so restoring
    ///    an occurrence INTO a body is a fact nothing can currently state. This
    ///    is the boundary — the second line of the maintainer's fixture needs
    ///    it, and inventing it is inventing a save system.
    pub const fn baseline_is_a_copy_of_this() {}

    /// **The gate on relocation: a reinstatement is ROOM-LOCAL.**
    ///
    /// [`Self::outlook_for`] can reinstate an occurrence only in the room whose
    /// authored records are in front of it, because a reinstatement is *"build
    /// this record, at these coordinates"* and construction is a pure function
    /// of ONE `RoomSpec`. An object carried into a room that does not author it
    /// and dropped there has a `Placed` row naming that room, and nothing in
    /// that room's records can produce it.
    ///
    /// ⭐ **closing the gate is one plumbing change, and it is not this file's.**
    /// The room being built needs the records of rooms it does not own — the
    /// caller has them (`RoomConstructionPlan::prepare_from_parts` holds the
    /// whole `RoomSet`) and would pass the foreign requests alongside the room's
    /// own. What must NOT happen first is suppression without it: see the note
    /// on [`Self::outlook_for`] about the deletion bug.
    pub const fn reinstatement_is_room_local() {}
}

/// **Custody is the first thing that gives an occurrence a whereabouts.**
///
/// An occurrence a body is carrying is alive and is not in any room, so the room
/// that authored it must not mint a second one. This reads the residency
/// projection rather than items: what it asks is "is this room-scoped occurrence
/// in somebody's custody", and every answer comes from
/// [`InCustodyOf`](super::InCustodyOf), which knows nothing about inventories
/// either.
///
/// ⚠ **recomputed unconditionally, and compared before it writes.** It carries no
/// "already applied" gate, so it converges after a rewind on the next step; the
/// equality check keeps change detection quiet on the overwhelming majority of
/// ticks, where nothing is being carried at all.
///
/// ⛔ **ORDER IS LOAD-BEARING: the placement producer runs BEFORE this.** On the
/// tick a hand empties, the placement producer turns the outgoing `InCustody`
/// row into a `Placed` row; this system then finds no custody row for that id
/// and has nothing to retract. Run the other way round, the custody retraction
/// erases the only evidence that the object was ever carried, and the placement
/// producer — which deliberately tracks only occurrences the ledger already
/// remembers — never starts.
///
/// ⭐ **the set is a `BTreeSet`, not the query's order.** Bevy's iteration order
/// is an archetype accident and this value reaches a construction plan, so an
/// unordered read here would be a determinism bug that reproduces perfectly on
/// one machine.
pub fn project_custody_onto_authored_occurrences(
    carried: Query<&SimId, (With<InCustodyOf>, With<RoomScopedEntity>)>,
    occurrences: Option<ResMut<AuthoredOccurrences>>,
) {
    let Some(mut occurrences) = occurrences else {
        return;
    };
    let alive: BTreeSet<SimId> = carried.iter().cloned().collect();
    if occurrences.in_custody() != alive {
        occurrences.republish_custody(alive);
    }
}
