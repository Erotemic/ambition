//! **What became of the occurrence an authored definition minted.**
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
//! ⚠ **this is DURABLE ROOM STATE, and it is the first of it.** It is not
//! rollback state (see [`AuthoredOccurrences::persisting`]) and it is not
//! residency (residency is [`InCustodyOf`](super::InCustodyOf) and carries no
//! room). Those are three separate concerns and this file touches exactly one.
//!
//! ⛔ **it is deliberately NOT a universal instance registry.** It records
//! nothing about occurrences whose disposition is the default; an empty ledger
//! is the ordinary state of every room in the game, and the only rows in it are
//! the ones some system had a reason to write.

use std::collections::BTreeSet;

use bevy::prelude::{Query, ResMut, Resource, With};

use super::{InCustodyOf, RoomScopedEntity};
use crate::sim_id::SimId;

/// **What reconstruction must do about one authored record**, given what became
/// of the occurrence it minted.
///
/// The variants are the terminal states an occurrence can reach, not a status
/// enum for its own sake: each one exists because reconstruction has to make a
/// different decision.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OccurrenceDisposition {
    /// **Author it.** Nothing durable is remembered about this record, which is
    /// the state of essentially every authored record essentially always: the
    /// room mints a fresh occurrence, exactly as it did before this file
    /// existed. This is the default precisely so that "no memory" and "author
    /// it" are the same thing and no ledger row is needed to say so.
    #[default]
    Authored,
    /// **Do not author it: the occurrence is still ALIVE and is somewhere else.**
    /// Carried out of the room, or otherwise no longer the room's to build.
    /// Minting a second one would put two live things behind one identity.
    Persisting,
    /// **Do not author it: the occurrence is gone for good.** A consumed key, a
    /// destroyed mechanism, a body killed in a way the world is supposed to
    /// remember. Distinct from [`Self::Authored`] — which is also "not alive" —
    /// because an ordinary room unload destroys occurrences by the dozen and
    /// every one of them SHOULD come back.
    ///
    /// ⛔ **no producer today, and that is load-bearing for the paragraph on
    /// [`AuthoredOccurrences::persisting`].** The day something writes this, the
    /// ledger stops being purely derived and owes a rollback registration and a
    /// durable-save representation. It is spelled here so that the question
    /// reconstruction asks is *"what is this record's disposition"* rather than
    /// the hard-coded *"is something with this id alive"* — the second sentence
    /// has no room for this variant at all.
    Consumed,
}

impl OccurrenceDisposition {
    /// The one question reconstruction asks.
    pub fn authors_a_fresh_occurrence(self) -> bool {
        matches!(self, Self::Authored)
    }
}

/// **The dispositions a session remembers**, keyed by the occurrence identity an
/// authored record mints.
///
/// ⚠ **absent means [`OccurrenceDisposition::Authored`]**, and a world with no
/// such resource at all means the same thing — which is why every consumer takes
/// it as an `Option` and a composition that never picks anything up carries an
/// empty ledger through its whole life.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct AuthoredOccurrences {
    persisting: BTreeSet<SimId>,
    consumed: BTreeSet<SimId>,
}

impl AuthoredOccurrences {
    /// What reconstruction must do about this authored record.
    pub fn disposition(&self, sim_id: &SimId) -> OccurrenceDisposition {
        if self.consumed.contains(sim_id) {
            OccurrenceDisposition::Consumed
        } else if self.persisting.contains(sim_id) {
            OccurrenceDisposition::Persisting
        } else {
            OccurrenceDisposition::Authored
        }
    }

    /// Every identity a rebuild of any room must NOT mint, in one set.
    ///
    /// Materialized rather than answered per-record because it is also the
    /// value a prepared plan is compared against: a plan prepared while the
    /// world remembered something different is not promotable into this one.
    pub fn suppressed(&self) -> BTreeSet<SimId> {
        self.persisting.union(&self.consumed).cloned().collect()
    }

    /// The occurrences that are alive elsewhere.
    ///
    /// ⚠ **this leg is DERIVED, every tick, from registered state** — see
    /// [`project_custody_onto_authored_occurrences`]. It holds an answer the
    /// simulation recomputes rather than a fact only it knows, so a rewind that
    /// restores custody restores this on the next step and it is NOT rollback
    /// state. That is true only while [`OccurrenceDisposition::Consumed`] has no
    /// producer; the `consumed` leg accumulates and would not rewind.
    pub fn persisting(&self) -> &BTreeSet<SimId> {
        &self.persisting
    }

    /// Republish the whole derived leg.
    ///
    /// ⛔ **RETRACT BY RESETTING, NEVER BY REMOVING.** There is no "this one
    /// stopped persisting" call, because a ledger that is edited row by row
    /// drifts from the world it describes the first time a row's retraction has
    /// no event behind it — an entity despawned by something that never heard of
    /// custody, a rollback that rewound past the pickup. The whole leg is
    /// replaced by what is true now, so an id absent from `alive` has been reset
    /// to [`OccurrenceDisposition::Authored`] by the same write that kept the
    /// others.
    pub fn republish_persisting(&mut self, alive: BTreeSet<SimId>) {
        self.persisting = alive;
    }
}

/// **Custody is the first thing that gives an occurrence a disposition.**
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
    if occurrences.persisting() != &alive {
        occurrences.republish_persisting(alive);
    }
}
