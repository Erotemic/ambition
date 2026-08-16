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

    /// **Every identity this build OWES the world, and where it owes it.**
    ///
    /// ⭐ **the caller must answer this list, not merely consult it.** A
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
    /// ⭐⭐ **a `Placed` row is the SAME fact seen from two rooms, and both
    /// rooms must act on it.** The room the occurrence is lying in reinstates
    /// it; every other room — including the one whose record MINTED it —
    /// suppresses it. Those two answers are one decision, and the reason they
    /// are computed in one function from one row is that they cannot be allowed
    /// to disagree: suppressing the home room while nothing rebuilds the
    /// occurrence where it lies deletes the object from the world permanently,
    /// and reinstating without suppressing puts two live things behind one
    /// `SimId`.
    ///
    /// ⛔ **so a consumer may not answer half of this.** The obligation stated
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

    /// **This ledger's contribution to the BASELINE horizon is a copy of
    /// itself** — see [`OccurrenceBaseline`], which is that copy.
    ///
    /// ⛔ **and a copy of this is NOT the whole baseline**, which is what this
    /// note used to imply. It is one domain's share of it. The custody leg is
    /// [`super::CustodyBaseline`], owned elsewhere, because "what happened to
    /// this authored occurrence" and "what is this body carrying" are different
    /// questions. [`super::horizon`] states the rule.
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
    /// ⭐ **the three things this note said did not exist now do** (2026-08-15):
    /// [`super::CheckpointCommitted`] is the world event, [`super::ResetToCheckpoint`]
    /// is the death road that restores rather than empties, and the custody leg
    /// is [`super::CustodyBaseline`] — which names the CUSTODIAN, because an
    /// `InCustody` row does not and "the key stays acquired" is a claim about a
    /// particular hand.
    ///
    /// ⚠ **what is still owed**, and it is the second half of that custody leg:
    /// an occurrence whose baseline says a body was carrying it, and which is
    /// NOT live in that body's hand at restore time, cannot be put back —
    /// nothing mints an occurrence directly into custody. Today it cannot
    /// happen, because a held occurrence is resident in no room and therefore
    /// survives every sweep a death crosses. It becomes reachable the moment a
    /// death destroys the carrier.
    pub const fn baseline_is_a_copy_of_this() {}

    /// **A reinstatement is NOT room-local, and the residency it restores is
    /// still not KEYED.**
    ///
    /// ⭐ **the gate this used to describe is closed** (2026-08-15, second leg):
    /// construction reconstructs a room's residency from the world's
    /// definitions plus this ledger, so an occurrence lying in a room that does
    /// not author it comes back there, from the record its home room holds.
    /// `outlook_for` therefore answers `Suppressed` in the home room and
    /// `Reinstated` where the object lies, and those two answers are one row.
    ///
    /// ⚠ **what is NOT closed is keyed room OWNERSHIP.** `RoomScopedEntity` says
    /// an occurrence dies with *a* room, never with *which* room, so residency
    /// still resolves against "whatever room is active". That is exactly right
    /// for today's single-active-room host and is the first thing that breaks
    /// when two participants occupy different rooms at once — at which point
    /// the scope marker owes a room key and this ledger's `Placed { room, .. }`
    /// becomes the thing that names it.
    pub const fn residency_is_reconstructed_not_room_local() {}
}

/// **The occurrence domain's share of the reset baseline** — horizon 2 of the
/// three, holding exactly what horizon 1 held at the last committed checkpoint.
///
/// ⭐ **a whole-value copy, and that is not laziness.** The alternative — record
/// which rows changed since the checkpoint — needs a diff that stays correct
/// across rollback, room streaming and a producer that republishes the whole leg
/// every tick. The ledger is a few dozen rows of two small variants; the copy is
/// the cheap side of that trade by a wide margin.
///
/// ⛔⛔ **UNLIKE the ledger it copies, this is NOT derived and MUST be declared
/// to rollback with a real VALUE projection.** Every row of `AuthoredOccurrences`
/// is republished from live state, which is what lets it be declared derived;
/// nothing republishes a baseline. It is written once at a checkpoint commit —
/// an event inside the rollback window, since a shrine is touched mid-frame —
/// and a rewind past that commit would leave the world holding a baseline from a
/// future that got un-happened. That is the same trap
/// [`AuthoredOccurrences::rewind_argument`] names for
/// [`OccurrenceWhereabouts::Consumed`], reached by a different route.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct OccurrenceBaseline(AuthoredOccurrences);

impl OccurrenceBaseline {
    /// The remembered ledger. Read by the fixture and by a durable-save writer;
    /// ⛔ not a place to mutate the baseline from, which is why there is no
    /// `_mut`.
    pub fn remembered(&self) -> &AuthoredOccurrences {
        &self.0
    }

    /// **The desync checksum for this baseline** — entity-free, and covering
    /// every field a peer could disagree about.
    ///
    /// ⭐ **the projection lives with the value, not with the registration.**
    /// A checksum written beside the registry has to reach through the value's
    /// privacy to do its job, and then silently stops covering whatever a later
    /// field adds. Here the match below is exhaustive, so a new
    /// [`OccurrenceWhereabouts`] variant is a compile error rather than a fact
    /// that quietly stopped being checked.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{
            checksum_bytes, put_str, put_u64, put_u8, put_vec2,
        };
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.0.rows.len() as u64);
        // `BTreeMap`, so this walk is ordered by identity on every peer.
        for (sim_id, whereabouts) in &self.0.rows {
            put_str(&mut bytes, sim_id.as_str());
            match whereabouts {
                OccurrenceWhereabouts::InCustody => put_u8(&mut bytes, 0),
                OccurrenceWhereabouts::Placed { room, at } => {
                    put_u8(&mut bytes, 1);
                    put_str(&mut bytes, room);
                    put_vec2(&mut bytes, *at);
                }
                OccurrenceWhereabouts::Consumed => put_u8(&mut bytes, 2),
            }
        }
        checksum_bytes(&bytes)
    }
}

/// **Record what the world remembers, at the instant a checkpoint commits.**
///
/// ⚠ **it captures even when the ledger is empty, and that matters.** An empty
/// baseline is a real answer — "at this checkpoint, nothing had been moved" —
/// and skipping the write would leave a stale baseline from an earlier
/// checkpoint in place, so a death would restore a world two checkpoints old.
/// The absence of a ledger resource entirely is the only case that writes
/// nothing, because there is then nothing in this domain to remember.
pub fn capture_occurrence_baseline(
    mut commits: bevy::prelude::MessageReader<super::CheckpointCommitted>,
    occurrences: Option<bevy::prelude::Res<AuthoredOccurrences>>,
    baseline: Option<ResMut<OccurrenceBaseline>>,
) {
    // Drained unconditionally: a commit seen during a load must not be re-read
    // on a later frame and charged to a world that has moved on.
    let committed = commits.read().count() > 0;
    let (Some(occurrences), Some(mut baseline)) = (occurrences, baseline) else {
        return;
    };
    if !committed {
        return;
    }
    if baseline.0 != *occurrences {
        baseline.0 = occurrences.clone();
    }
}

/// **Put the remembered ledger back, on a death.**
///
/// ⭐ **this restores the LEDGER and nothing else, on purpose.** It does not
/// touch a single occurrence in the world: the rebuild that follows reads the
/// restored ledger and reaches the right answer for every authored record by
/// itself — suppress what the baseline says is elsewhere, reinstate what it says
/// is lying somewhere, author the rest as written. Teaching this system to also
/// fix up live entities would give the world two authorities on the same
/// question, and the whole point of `outlook_for` is that there is one.
///
/// ⚠ **the ONE thing it cannot reach is an occurrence in a hand**, because a
/// held occurrence is resident in no room and no rebuild sees it. That leg is
/// [`super::retract_custody_to_checkpoint`], and it belongs to the custody
/// domain because a hand is not room state.
pub fn restore_occurrence_baseline(
    mut resets: bevy::prelude::MessageReader<super::ResetToCheckpoint>,
    baseline: Option<bevy::prelude::Res<OccurrenceBaseline>>,
    occurrences: Option<ResMut<AuthoredOccurrences>>,
) {
    let requested = resets.read().count() > 0;
    let (Some(baseline), Some(mut occurrences)) = (baseline, occurrences) else {
        return;
    };
    if !requested {
        return;
    }
    if *occurrences != baseline.0 {
        *occurrences = baseline.0.clone();
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// **ONE ROW, TWO ROOMS, TWO OPPOSITE ANSWERS — and both are asserted.**
    ///
    /// An occurrence carried out of the room that authored it and put down next
    /// door is exactly one fact. The room it lies in owes the world that
    /// occurrence; the room whose record minted it owes the world nothing, and
    /// authoring it again would put two live things behind one `SimId`.
    ///
    /// ⛔ **asserting only the suppression is the dangerous half**, because a
    /// ledger that suppressed everywhere would pass it and would delete the
    /// object from the world permanently. Both terms are observed here, from
    /// the same row, so neither arm can regress alone.
    #[test]
    fn a_placed_row_reinstates_where_it_lies_and_suppresses_where_it_was_authored() {
        let axe = SimId::placement("blink_run_pickup");
        let mut ledger = AuthoredOccurrences::default();
        ledger.republish_placements(
            "portal_bridge",
            [(axe.clone(), Vec2::new(48.0, 96.0))].into_iter().collect(),
        );

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

        // ⭐ AND A ROOM WITH NO STAKE IN THE ROW IS NOT ASKED TO DO ANYTHING
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
}
